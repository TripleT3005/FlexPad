use std::io::{Read, Write};
use std::time::Duration;

use super::connection::{Connection, READ_TIMEOUT};

const ESPRESSIF_VID: u16 = 0x303A;
const SERIAL_PREFIX: &str = "FlexPad-";
const PORT_SETTLE_MS: u64 = 100;

pub type Port = Box<dyn serialport::SerialPort>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexPadDeviceInfo {
    pub port_name: String,
    pub device_id: String,
    pub firmware_version: Option<String>,
}

impl std::fmt::Display for FlexPadDeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.device_id, self.port_name)
    }
}

pub fn read_line_from_port(port: &mut impl Read, timeout: Duration) -> Option<String> {
    let start_time = std::time::Instant::now();
    let mut line = Vec::new();
    let mut buffer = [0; 1];

    while start_time.elapsed() <= timeout {
        match port.read(&mut buffer) {
            Ok(n) if n > 0 => match buffer[0] {
                b'\n' => break,
                b'\r' => {}
                byte => line.push(byte),
            },
            _ => {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    if line.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&line).into_owned())
    }
}

pub fn is_flexpad_port(port: &serialport::SerialPortInfo) -> bool {
    let serialport::SerialPortType::UsbPort(usb) = &port.port_type else {
        return false;
    };
    usb.vid == ESPRESSIF_VID
        && usb.serial_number.as_deref().is_some_and(|serial| {
            serial
                .get(..SERIAL_PREFIX.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SERIAL_PREFIX))
        })
}

pub fn scan_flexpad_devices() -> Vec<FlexPadDeviceInfo> {
    let mut devices = Vec::new();
    let ports = match serialport::available_ports() {
        Ok(p) => p,
        Err(_) => return devices,
    };

    for p in ports.into_iter().filter(is_flexpad_port) {
        let port_name = p.port_name;
        if let Ok(mut port) = serialport::new(&port_name, 115200)
            .timeout(Duration::from_millis(200))
            .open()
        {
            let _ = port.write_data_terminal_ready(true);
            std::thread::sleep(Duration::from_millis(PORT_SETTLE_MS));
            let _ = port.clear(serialport::ClearBuffer::All);

            if port.write_all(b"GET_DEVICE_INFO\n").is_ok()
                && let Some(response) = read_line_from_port(&mut port, Duration::from_millis(600))
                && response.contains("DEVICE_INFO:FlexPad")
            {
                let parts: Vec<&str> = response.split(',').collect();
                let device_id = parts
                    .get(1)
                    .map_or_else(|| "FlexPad-Unknown".to_string(), |id| id.to_string());
                let firmware_version = parts.get(2).map(|version| version.trim().to_string());
                devices.push(FlexPadDeviceInfo {
                    port_name,
                    device_id,
                    firmware_version,
                });
            }
        }
    }
    devices
}

pub fn connect_to_port(port_name: &str) -> Result<Connection, String> {
    let mut port = serialport::new(port_name, 115200)
        .timeout(READ_TIMEOUT)
        .open()
        .map_err(|e| e.to_string())?;
    let _ = port.write_data_terminal_ready(true);
    std::thread::sleep(Duration::from_millis(1000));
    let _ = port.clear(serialport::ClearBuffer::All);
    Ok(Connection::open(port))
}

pub fn fetch_config_from_device(connection: &Connection) -> Result<String, String> {
    connection.request("GET_CONFIG", |response| {
        response
            .strip_prefix("CONFIG_DATA:")
            .map(|json| Ok(json.trim().to_string()))
    })
}

pub fn set_config_to_device(connection: &Connection, json_config: &str) -> Result<(), String> {
    connection.request(&format!("SET_CONFIG:{json_config}"), |response| {
        if response == "CONFIG_SAVED" {
            return Some(Ok(()));
        }
        response
            .strip_prefix("CONFIG_ERROR:")
            .map(|reason| Err(format!("The device rejected the configuration ({reason})")))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroTestResult {
    Queued,
    Busy,
    Rejected(String),
}

pub fn parse_macro_response(response: &str) -> Option<MacroTestResult> {
    match response {
        "MACRO_QUEUED" => Some(MacroTestResult::Queued),
        "MACRO_ERROR:busy" => Some(MacroTestResult::Busy),
        _ => response
            .strip_prefix("MACRO_ERROR:")
            .map(|reason| MacroTestResult::Rejected(reason.to_string())),
    }
}

pub fn run_macro_on_device(
    connection: &Connection,
    function_json: &str,
) -> Result<MacroTestResult, String> {
    connection.request(&format!("RUN_MACRO:{function_json}"), |response| {
        parse_macro_response(response).map(Ok)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const TIMEOUT: Duration = Duration::from_millis(20);

    // Port filter

    fn usb_port(vid: u16, serial_number: Option<&str>) -> serialport::SerialPortInfo {
        serialport::SerialPortInfo {
            port_name: "COM9".to_string(),
            port_type: serialport::SerialPortType::UsbPort(serialport::UsbPortInfo {
                vid,
                pid: 0x1001,
                serial_number: serial_number.map(str::to_string),
                manufacturer: None,
                product: None,
            }),
        }
    }

    #[test]
    fn accepts_flexpad_serial_numbers_in_any_case() {
        assert!(is_flexpad_port(&usb_port(
            0x303A,
            Some("FLEXPAD-0123456789AB")
        )));
        assert!(is_flexpad_port(&usb_port(
            0x303A,
            Some("FlexPad-0123456789AB")
        )));
    }

    #[test]
    fn rejects_other_devices() {
        assert!(!is_flexpad_port(&usb_port(
            0x303A,
            Some("14:C1:9F:46:A1:DC")
        )));
        assert!(!is_flexpad_port(&usb_port(0x303A, None)));
        assert!(!is_flexpad_port(&usb_port(0x2341, Some("FlexPad-123"))));
        assert!(!is_flexpad_port(&usb_port(0x303A, Some("Flex"))));
        assert!(!is_flexpad_port(&serialport::SerialPortInfo {
            port_name: "COM1".to_string(),
            port_type: serialport::SerialPortType::PciPort,
        }));
    }

    // Macro responses

    #[test]
    fn parses_macro_responses() {
        assert_eq!(
            parse_macro_response("MACRO_QUEUED"),
            Some(MacroTestResult::Queued)
        );
        assert_eq!(
            parse_macro_response("MACRO_ERROR:busy"),
            Some(MacroTestResult::Busy)
        );
        assert_eq!(
            parse_macro_response("MACRO_ERROR:unknown_key:hyper"),
            Some(MacroTestResult::Rejected("unknown_key:hyper".to_string()))
        );
        assert_eq!(parse_macro_response("CONFIG_SAVED"), None);
    }

    // Line reading

    #[test]
    fn reads_utf8_lines() {
        let mut port =
            Cursor::new("CONFIG_DATA:{\"profile_name\":\"Größe äöü 😀\"}\r\nNEXT\n".as_bytes());

        assert_eq!(
            read_line_from_port(&mut port, TIMEOUT).as_deref(),
            Some("CONFIG_DATA:{\"profile_name\":\"Größe äöü 😀\"}")
        );
        assert_eq!(
            read_line_from_port(&mut port, TIMEOUT).as_deref(),
            Some("NEXT")
        );
    }

    #[test]
    fn returns_none_without_data() {
        let mut port = Cursor::new(Vec::new());

        assert_eq!(read_line_from_port(&mut port, TIMEOUT), None);
    }

    #[test]
    fn replaces_invalid_bytes_instead_of_failing() {
        let mut port = Cursor::new(vec![b'O', 0xFF, b'K', b'\n']);

        assert_eq!(
            read_line_from_port(&mut port, TIMEOUT).as_deref(),
            Some("O\u{FFFD}K")
        );
    }

    #[test]
    fn returns_partial_line_after_timeout() {
        let mut port = Cursor::new(b"DEVICE_INFO:Flex".to_vec());

        assert_eq!(
            read_line_from_port(&mut port, TIMEOUT).as_deref(),
            Some("DEVICE_INFO:Flex")
        );
    }
}
