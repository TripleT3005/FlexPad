use std::hash::{Hash, Hasher};
use std::io::{ErrorKind, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use iced::futures::Stream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::serial::Port;

// Constants

pub const READ_TIMEOUT: Duration = Duration::from_millis(20);
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(2000);
const MAX_LINE_LENGTH: usize = 20 * 1024;
const READ_BUFFER_SIZE: usize = 1024;

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

// Events

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinEdge {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDirection {
    Right,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Pin(u8, PinEdge),
    Encoder(u8, TurnDirection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEvent {
    Input(InputEvent),
    Disconnected,
}

pub fn parse_event(line: &str) -> Option<InputEvent> {
    let mut parts = line.strip_prefix("EVENT:")?.split(':');
    let event = match (parts.next()?, parts.next()?.parse().ok()?, parts.next()?) {
        ("PIN", pin, "PRESSED") => InputEvent::Pin(pin, PinEdge::Pressed),
        ("PIN", pin, "RELEASED") => InputEvent::Pin(pin, PinEdge::Released),
        ("ENCODER", pin, "RIGHT") => InputEvent::Encoder(pin, TurnDirection::Right),
        ("ENCODER", pin, "LEFT") => InputEvent::Encoder(pin, TurnDirection::Left),
        _ => return None,
    };
    parts.next().is_none().then_some(event)
}

// Line splitting

#[derive(Debug, Default)]
pub struct LineSplitter {
    buffer: Vec<u8>,
}

impl LineSplitter {
    pub fn push(&mut self, bytes: &[u8], mut on_line: impl FnMut(String)) {
        for &byte in bytes {
            match byte {
                b'\n' => {
                    if !self.buffer.is_empty() {
                        on_line(String::from_utf8_lossy(&self.buffer).into_owned());
                        self.buffer.clear();
                    }
                }
                b'\r' => {}
                _ if self.buffer.len() < MAX_LINE_LENGTH => self.buffer.push(byte),
                _ => {}
            }
        }
    }
}

// Routing

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    Event(InputEvent),
    Response(String),
    Ignored,
}

pub fn route_line(line: String) -> Incoming {
    if line.starts_with("EVENT:") {
        parse_event(&line).map_or(Incoming::Ignored, Incoming::Event)
    } else {
        Incoming::Response(line)
    }
}

// Link

pub trait Link: Read + Write {
    fn is_open(&self) -> bool;
}

impl Link for Port {
    fn is_open(&self) -> bool {
        self.bytes_to_read().is_ok()
    }
}

// Reader

fn run_reader(
    mut link: impl Link,
    outgoing: Receiver<Vec<u8>>,
    responses: Sender<String>,
    events: UnboundedSender<DeviceEvent>,
) {
    let mut splitter = LineSplitter::default();
    let mut buffer = [0; READ_BUFFER_SIZE];
    loop {
        loop {
            match outgoing.try_recv() {
                Ok(bytes) => {
                    if link.write_all(&bytes).and_then(|()| link.flush()).is_err() {
                        let _ = events.send(DeviceEvent::Disconnected);
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let open = match link.read(&mut buffer) {
            Ok(0) => link.is_open(),
            Ok(count) => {
                splitter.push(&buffer[..count], |line| match route_line(line) {
                    Incoming::Event(event) => {
                        let _ = events.send(DeviceEvent::Input(event));
                    }
                    Incoming::Response(response) => {
                        let _ = responses.send(response);
                    }
                    Incoming::Ignored => {}
                });
                true
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::TimedOut | ErrorKind::WouldBlock | ErrorKind::Interrupted
                ) =>
            {
                link.is_open()
            }
            Err(_) => false,
        };
        if !open {
            let _ = events.send(DeviceEvent::Disconnected);
            return;
        }
    }
}

// Connection

struct Client {
    outgoing: Sender<Vec<u8>>,
    responses: Receiver<String>,
}

struct Shared {
    id: u64,
    client: Mutex<Client>,
    events: Mutex<Option<UnboundedReceiver<DeviceEvent>>>,
}

#[derive(Clone)]
pub struct Connection(Arc<Shared>);

impl Connection {
    pub fn open(link: impl Link + Send + 'static) -> Self {
        let (outgoing_sender, outgoing_receiver) = mpsc::channel();
        let (response_sender, response_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = unbounded_channel();
        std::thread::spawn(move || {
            run_reader(link, outgoing_receiver, response_sender, event_sender);
        });
        Self(Arc::new(Shared {
            id: NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed),
            client: Mutex::new(Client {
                outgoing: outgoing_sender,
                responses: response_receiver,
            }),
            events: Mutex::new(Some(event_receiver)),
        }))
    }

    pub fn take_events(&self) -> Option<UnboundedReceiver<DeviceEvent>> {
        self.0
            .events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    pub fn request<T>(
        &self,
        command: &str,
        accept: impl FnMut(&str) -> Option<Result<T, String>>,
    ) -> Result<T, String> {
        self.request_with_timeout(command, RESPONSE_TIMEOUT, accept)
    }

    fn request_with_timeout<T>(
        &self,
        command: &str,
        timeout: Duration,
        mut accept: impl FnMut(&str) -> Option<Result<T, String>>,
    ) -> Result<T, String> {
        let client = self.0.client.lock().unwrap_or_else(PoisonError::into_inner);
        while client.responses.try_recv().is_ok() {}

        let mut line = command.as_bytes().to_vec();
        line.push(b'\n');
        client
            .outgoing
            .send(line)
            .map_err(|_| "The device was disconnected".to_string())?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match client.responses.recv_timeout(remaining) {
                Ok(response) => {
                    if let Some(reason) = response.strip_prefix("ERROR:") {
                        return Err(format!("The device reported an error ({reason})"));
                    }
                    if let Some(result) = accept(&response) {
                        return result;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err("The device did not answer in time".to_string());
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("The device was disconnected".to_string());
                }
            }
        }
    }
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Hash for Connection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Connection({})", self.0.id)
    }
}

pub fn device_events(connection: &Connection) -> impl Stream<Item = DeviceEvent> + use<> {
    iced::futures::stream::unfold(connection.take_events(), |receiver| async move {
        let mut receiver = receiver?;
        let event = receiver.recv().await?;
        Some((event, Some(receiver)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    // Fake link

    #[derive(Clone, Default)]
    struct FakeLink(Arc<Mutex<FakeState>>);

    #[derive(Default)]
    struct FakeState {
        incoming: VecDeque<u8>,
        written: Vec<u8>,
        answers: Vec<(String, String)>,
        unplugged: bool,
    }

    impl FakeLink {
        fn feed(&self, data: &str) {
            self.0.lock().unwrap().incoming.extend(data.as_bytes());
        }

        fn answer(&self, command: &str, response: &str) {
            self.0
                .lock()
                .unwrap()
                .answers
                .push((format!("{command}\n"), response.to_string()));
        }

        fn unplug(&self) {
            self.0.lock().unwrap().unplugged = true;
        }

        fn written(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().written.clone()).unwrap()
        }
    }

    impl Read for FakeLink {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut state = self.0.lock().unwrap();
            if state.incoming.is_empty() {
                drop(state);
                std::thread::sleep(Duration::from_millis(2));
                return Err(ErrorKind::TimedOut.into());
            }
            let count = buf.len().min(state.incoming.len());
            for (slot, byte) in buf.iter_mut().zip(state.incoming.drain(..count)) {
                *slot = byte;
            }
            Ok(count)
        }
    }

    impl Write for FakeLink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut state = self.0.lock().unwrap();
            state.written.extend_from_slice(buf);
            let command = String::from_utf8_lossy(buf).into_owned();
            if let Some(index) = state.answers.iter().position(|(c, _)| *c == command) {
                let (_, response) = state.answers.remove(index);
                state.incoming.extend(response.as_bytes());
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Link for FakeLink {
        fn is_open(&self) -> bool {
            !self.0.lock().unwrap().unplugged
        }
    }

    fn accept_config(line: &str) -> Option<Result<String, String>> {
        line.strip_prefix("CONFIG_DATA:")
            .map(|json| Ok(json.to_string()))
    }

    fn next_event(receiver: &mut UnboundedReceiver<DeviceEvent>) -> DeviceEvent {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(event) = receiver.try_recv() {
                return event;
            }
            assert!(Instant::now() < deadline, "no event arrived");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // Event parsing

    #[test]
    fn parses_pin_and_encoder_events() {
        assert_eq!(
            parse_event("EVENT:PIN:4:PRESSED"),
            Some(InputEvent::Pin(4, PinEdge::Pressed))
        );
        assert_eq!(
            parse_event("EVENT:PIN:12:RELEASED"),
            Some(InputEvent::Pin(12, PinEdge::Released))
        );
        assert_eq!(
            parse_event("EVENT:ENCODER:15:RIGHT"),
            Some(InputEvent::Encoder(15, TurnDirection::Right))
        );
        assert_eq!(
            parse_event("EVENT:ENCODER:15:LEFT"),
            Some(InputEvent::Encoder(15, TurnDirection::Left))
        );
    }

    #[test]
    fn rejects_malformed_events() {
        for line in [
            "EVENT:PIN:4",
            "EVENT:PIN:x:PRESSED",
            "EVENT:PIN:400:PRESSED",
            "EVENT:PIN:4:HELD",
            "EVENT:ENCODER:15:UP",
            "EVENT:KNOB:15:LEFT",
            "EVENT:PIN:4:PRESSED:extra",
            "PIN:4:PRESSED",
        ] {
            assert_eq!(parse_event(line), None, "{line}");
        }
    }

    // Line splitting and routing

    #[test]
    fn splits_lines_across_chunks() {
        let mut splitter = LineSplitter::default();
        let mut lines = Vec::new();

        splitter.push(b"EVENT:PIN:4:PRE", |line| lines.push(line));
        splitter.push(b"SSED\r\nCONFIG_DATA:{\"a\":\"\xC3", |line| {
            lines.push(line)
        });
        splitter.push(b"\xA4\"}\n\n", |line| lines.push(line));

        assert_eq!(lines, ["EVENT:PIN:4:PRESSED", "CONFIG_DATA:{\"a\":\"ä\"}"]);
    }

    #[test]
    fn caps_overlong_lines() {
        let mut splitter = LineSplitter::default();
        let mut lines = Vec::new();

        splitter.push(&vec![b'x'; MAX_LINE_LENGTH + 10], |line| lines.push(line));
        splitter.push(b"\nOK\n", |line| lines.push(line));

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), MAX_LINE_LENGTH);
        assert_eq!(lines[1], "OK");
    }

    #[test]
    fn routes_events_and_responses() {
        assert_eq!(
            route_line("EVENT:ENCODER:15:LEFT".to_string()),
            Incoming::Event(InputEvent::Encoder(15, TurnDirection::Left))
        );
        assert_eq!(
            route_line("EVENT:SOMETHING:NEW".to_string()),
            Incoming::Ignored
        );
        assert_eq!(
            route_line("MACRO_QUEUED".to_string()),
            Incoming::Response("MACRO_QUEUED".to_string())
        );
    }

    // Connection

    #[test]
    fn responses_reach_the_request_while_events_go_to_the_stream() {
        let link = FakeLink::default();
        link.answer(
            "GET_CONFIG",
            "EVENT:PIN:4:PRESSED\nCONFIG_DATA:{}\nEVENT:PIN:4:RELEASED\n",
        );
        let connection = Connection::open(link.clone());
        let mut events = connection.take_events().unwrap();

        assert_eq!(
            connection.request("GET_CONFIG", accept_config),
            Ok("{}".to_string())
        );
        assert_eq!(
            next_event(&mut events),
            DeviceEvent::Input(InputEvent::Pin(4, PinEdge::Pressed))
        );
        assert_eq!(
            next_event(&mut events),
            DeviceEvent::Input(InputEvent::Pin(4, PinEdge::Released))
        );
        assert_eq!(link.written(), "GET_CONFIG\n");
        assert!(connection.take_events().is_none());
    }

    #[test]
    fn stale_responses_are_dropped_before_a_request() {
        let link = FakeLink::default();
        link.feed("CONFIG_DATA:{\"old\":1}\n");
        let connection = Connection::open(link.clone());
        std::thread::sleep(Duration::from_millis(50));
        link.answer("GET_CONFIG", "CONFIG_DATA:{\"new\":2}\n");

        assert_eq!(
            connection.request("GET_CONFIG", accept_config),
            Ok("{\"new\":2}".to_string())
        );
    }

    #[test]
    fn unrelated_lines_are_skipped_and_errors_end_the_request() {
        let link = FakeLink::default();
        link.answer("GET_CONFIG", "CONFIG_SAVED\nCONFIG_DATA:{}\n");
        link.answer("NOPE", "ERROR:unknown_command\n");
        let connection = Connection::open(link);

        assert_eq!(
            connection.request("GET_CONFIG", accept_config),
            Ok("{}".to_string())
        );
        assert_eq!(
            connection.request("NOPE", accept_config),
            Err("The device reported an error (unknown_command)".to_string())
        );
    }

    #[test]
    fn a_missing_answer_times_out() {
        let connection = Connection::open(FakeLink::default());

        assert_eq!(
            connection.request_with_timeout("GET_CONFIG", Duration::from_millis(30), accept_config),
            Err("The device did not answer in time".to_string())
        );
    }

    #[test]
    fn unplugging_reports_a_disconnect_and_fails_requests() {
        let link = FakeLink::default();
        let connection = Connection::open(link.clone());
        let mut events = connection.take_events().unwrap();

        link.unplug();

        assert_eq!(next_event(&mut events), DeviceEvent::Disconnected);
        assert_eq!(
            connection.request("GET_CONFIG", accept_config),
            Err("The device was disconnected".to_string())
        );
    }

    #[test]
    fn connections_are_told_apart_by_id() {
        let first = Connection::open(FakeLink::default());
        let second = Connection::open(FakeLink::default());

        assert_eq!(first, first.clone());
        assert_ne!(first, second);
    }
}
