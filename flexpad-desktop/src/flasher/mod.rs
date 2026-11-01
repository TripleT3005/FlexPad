mod image;
mod rom;
mod slip;

pub use image::embedded_image;

use image::FirmwareImage;

use crate::device::{Port, SharedPort, is_flexpad_port, reboot_to_bootloader};
use rom::{FLASH_BLOCK_SIZE, Opcode, Request, Response};
use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

// Constants

const ESPRESSIF_VID: u16 = 0x303A;
const ROM_DOWNLOAD_PID: u16 = 0x0009;
const BAUD_RATE: u32 = 115_200;
const READ_TIMEOUT: Duration = Duration::from_millis(20);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const SYNC_TIMEOUT: Duration = Duration::from_millis(200);
const SYNC_ATTEMPTS: usize = 10;
const SYNC_DRAIN: Duration = Duration::from_millis(100);
const ERASE_TIMEOUT_PER_MEGABYTE: Duration = Duration::from_secs(30);
const PORT_POLL_INTERVAL: Duration = Duration::from_millis(250);
const OPEN_TIMEOUT: Duration = Duration::from_secs(3);
const DOWNLOAD_PORT_TIMEOUT: Duration = Duration::from_secs(10);
const RESTART_TIMEOUT: Duration = Duration::from_secs(10);

const RTC_CNTL_OPTION1_REG: u32 = 0x6000_812C;
const RTC_CNTL_FORCE_DOWNLOAD_BOOT: u32 = 1;
const RTC_CNTL_WDTCONFIG0_REG: u32 = 0x6000_8098;
const RTC_CNTL_WDTCONFIG1_REG: u32 = 0x6000_809C;
const RTC_CNTL_WDTWPROTECT_REG: u32 = 0x6000_80B0;
const RTC_CNTL_WDT_KEY: u32 = 0x50D8_3AA1;
const RTC_WDT_TIMEOUT_CYCLES: u32 = 5000;
const RTC_WDT_CONFIG: u32 = (1 << 31) | (5 << 28) | (1 << 8) | 2;
const GPIO_STRAP_REG: u32 = 0x6000_4038;
const GPIO_STRAP_SPI_BOOT: u32 = 1 << 3;

// Progress

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashStage {
    Rebooting,
    WaitingForBootloader,
    Connecting,
    Writing { written: usize, total: usize },
    Restarting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashOutcome {
    Restarted,
    NeedsManualReset,
}

// Ports

pub fn is_download_port(port: &serialport::SerialPortInfo) -> bool {
    matches!(
        &port.port_type,
        serialport::SerialPortType::UsbPort(usb)
            if usb.vid == ESPRESSIF_VID && usb.pid == ROM_DOWNLOAD_PID
    )
}

pub fn find_download_port() -> Option<String> {
    serialport::available_ports()
        .ok()?
        .into_iter()
        .find(is_download_port)
        .map(|port| port.port_name)
}

pub fn wait_for_port(
    timeout: Duration,
    matches: impl Fn(&serialport::SerialPortInfo) -> bool,
) -> Option<String> {
    let deadline = Instant::now() + timeout;
    loop {
        let found = serialport::available_ports()
            .ok()
            .and_then(|ports| ports.into_iter().find(|port| matches(port)));
        if let Some(port) = found {
            return Some(port.port_name);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(PORT_POLL_INTERVAL);
    }
}

fn open_download_port(port_name: &str) -> Result<Port, String> {
    let deadline = Instant::now() + OPEN_TIMEOUT;
    loop {
        match serialport::new(port_name, BAUD_RATE)
            .timeout(READ_TIMEOUT)
            .open()
        {
            Ok(port) => return Ok(port),
            Err(e) if Instant::now() >= deadline => {
                return Err(format!("could not open {port_name}: {e}"));
            }
            Err(_) => std::thread::sleep(PORT_POLL_INTERVAL),
        }
    }
}

// Update

#[derive(Debug, Clone)]
pub enum FlashSource {
    Device(SharedPort),
    DownloadPort(String),
}

#[derive(Debug, Clone)]
pub enum FlashEvent {
    Progress(FlashStage),
    Finished(Result<FlashOutcome, String>),
}

pub fn update_firmware(
    source: FlashSource,
    progress: &mut impl FnMut(FlashStage),
) -> Result<FlashOutcome, String> {
    let image = embedded_image().ok_or("this build of the app contains no firmware image")?;
    let port_name = match source {
        FlashSource::Device(port) => {
            progress(FlashStage::Rebooting);
            reboot_to_bootloader(&mut port.lock())?;
            drop(port);
            progress(FlashStage::WaitingForBootloader);
            wait_for_port(DOWNLOAD_PORT_TIMEOUT, is_download_port).ok_or(
                "the FlexPad did not start its bootloader; hold BOOT while plugging it in, \
                 then scan again",
            )?
        }
        FlashSource::DownloadPort(port_name) => port_name,
    };
    let mut port = open_download_port(&port_name)?;
    let outcome = flash_image(&mut port, &image, progress)?;
    drop(port);
    if outcome == FlashOutcome::Restarted
        && wait_for_port(RESTART_TIMEOUT, is_flexpad_port).is_none()
    {
        return Ok(FlashOutcome::NeedsManualReset);
    }
    Ok(outcome)
}

// Flashing

pub fn flash_image(
    port: &mut (impl Read + Write),
    image: &FirmwareImage,
    progress: &mut impl FnMut(FlashStage),
) -> Result<FlashOutcome, String> {
    let mut rom = RomConnection::new(port);
    progress(FlashStage::Connecting);
    rom.sync()?;
    rom.command(&Request::spi_attach(), COMMAND_TIMEOUT)?;
    rom.command(&Request::spi_set_params(image.flash_size), COMMAND_TIMEOUT)?;

    let total = image.written_len();
    let mut written = 0;
    progress(FlashStage::Writing { written, total });
    for segment in image.segments() {
        rom.command(
            &Request::flash_begin(segment.data.len() as u32, segment.offset),
            erase_timeout(segment.data.len()),
        )?;
        for (sequence, block) in segment.data.chunks(FLASH_BLOCK_SIZE).enumerate() {
            rom.command(
                &Request::flash_data(sequence as u32, block),
                COMMAND_TIMEOUT,
            )?;
            written += block.len();
            progress(FlashStage::Writing { written, total });
        }
    }

    progress(FlashStage::Restarting);
    rom.restart()
}

fn erase_timeout(len: usize) -> Duration {
    COMMAND_TIMEOUT + ERASE_TIMEOUT_PER_MEGABYTE.mul_f64(len as f64 / f64::from(1u32 << 20))
}

// ROM connection

struct RomConnection<'a, P> {
    port: &'a mut P,
    decoder: slip::Decoder,
}

impl<'a, P: Read + Write> RomConnection<'a, P> {
    fn new(port: &'a mut P) -> Self {
        Self {
            port,
            decoder: slip::Decoder::default(),
        }
    }

    fn send(&mut self, request: &Request) -> Result<(), String> {
        self.port
            .write_all(&slip::encode(&request.to_bytes()))
            .and_then(|()| self.port.flush())
            .map_err(|e| format!("writing to the device failed: {e}"))
    }

    fn receive(&mut self, opcode: Opcode, timeout: Duration) -> Result<Response, String> {
        let deadline = Instant::now() + timeout;
        let mut buffer = [0u8; 256];
        while Instant::now() < deadline {
            let count = match self.port.read(&mut buffer) {
                Ok(count) => count,
                Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::Interrupted) => 0,
                Err(e) => return Err(format!("reading from the device failed: {e}")),
            };
            if count == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
            for &byte in &buffer[..count] {
                if let Some(response) = self
                    .decoder
                    .push(byte)
                    .and_then(|frame| Response::parse(&frame))
                    && response.opcode == opcode as u8
                {
                    return Ok(response);
                }
            }
        }
        Err(format!("the device did not answer {opcode:?}"))
    }

    fn command(&mut self, request: &Request, timeout: Duration) -> Result<Response, String> {
        self.send(request)?;
        let response = self.receive(request.opcode, timeout)?;
        match response.failure() {
            Some(reason) => Err(format!("{:?} failed: {reason}", request.opcode)),
            None => Ok(response),
        }
    }

    fn sync(&mut self) -> Result<(), String> {
        for _ in 0..SYNC_ATTEMPTS {
            if self.command(&Request::sync(), SYNC_TIMEOUT).is_ok() {
                while self.receive(Opcode::Sync, SYNC_DRAIN).is_ok() {}
                return Ok(());
            }
        }
        Err("the bootloader did not respond".to_string())
    }

    fn read_reg(&mut self, address: u32) -> Result<u32, String> {
        Ok(self
            .command(&Request::read_reg(address), COMMAND_TIMEOUT)?
            .value)
    }

    fn write_reg(&mut self, request: &Request) -> Result<(), String> {
        self.command(request, COMMAND_TIMEOUT).map(|_| ())
    }

    fn restart(&mut self) -> Result<FlashOutcome, String> {
        self.write_reg(&Request::write_reg_masked(
            RTC_CNTL_OPTION1_REG,
            0,
            RTC_CNTL_FORCE_DOWNLOAD_BOOT,
        ))?;
        let strapped_for_download = self.read_reg(GPIO_STRAP_REG)? & GPIO_STRAP_SPI_BOOT == 0;
        let forced_download =
            self.read_reg(RTC_CNTL_OPTION1_REG)? & RTC_CNTL_FORCE_DOWNLOAD_BOOT != 0;
        if strapped_for_download || forced_download {
            return Ok(FlashOutcome::NeedsManualReset);
        }
        self.write_reg(&Request::write_reg(
            RTC_CNTL_WDTWPROTECT_REG,
            RTC_CNTL_WDT_KEY,
        ))?;
        self.write_reg(&Request::write_reg(
            RTC_CNTL_WDTCONFIG1_REG,
            RTC_WDT_TIMEOUT_CYCLES,
        ))?;
        self.write_reg(&Request::write_reg(RTC_CNTL_WDTCONFIG0_REG, RTC_WDT_CONFIG))?;
        let _ = self.write_reg(&Request::write_reg(RTC_CNTL_WDTWPROTECT_REG, 0));
        Ok(FlashOutcome::Restarted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    // Simulated ROM bootloader

    struct SimulatedRom {
        decoder: slip::Decoder,
        output: VecDeque<u8>,
        flash: Vec<u8>,
        registers: std::collections::HashMap<u32, u32>,
        write_offset: Option<u32>,
        next_sequence: u32,
        requests: Vec<u8>,
        synced: bool,
        corrupt_block: Option<u32>,
    }

    impl SimulatedRom {
        fn new() -> Self {
            Self {
                decoder: slip::Decoder::default(),
                output: VecDeque::new(),
                flash: vec![0xAB; 0x42_0000],
                registers: [
                    (GPIO_STRAP_REG, GPIO_STRAP_SPI_BOOT),
                    (RTC_CNTL_OPTION1_REG, RTC_CNTL_FORCE_DOWNLOAD_BOOT),
                ]
                .into(),
                write_offset: None,
                next_sequence: 0,
                requests: Vec::new(),
                synced: false,
                corrupt_block: None,
            }
        }

        fn word(data: &[u8], index: usize) -> u32 {
            u32::from_le_bytes(data[index * 4..index * 4 + 4].try_into().unwrap())
        }

        fn respond(&mut self, opcode: u8, value: u32, status: u8, error: u8) {
            let mut frame = vec![0x01, opcode, 4, 0];
            frame.extend(value.to_le_bytes());
            frame.extend([status, error, 0, 0]);
            self.output.extend(slip::encode(&frame));
        }

        fn handle(&mut self, frame: &[u8]) {
            let opcode = frame[1];
            let checksum = Self::word(&frame[4..8], 0);
            let data = &frame[8..];
            self.requests.push(opcode);
            if !self.synced && opcode != Opcode::Sync as u8 {
                return;
            }
            let (value, status) = match opcode {
                0x08 => {
                    self.synced = true;
                    for _ in 0..7 {
                        self.respond(opcode, 0, 0, 0);
                    }
                    (0, 0)
                }
                0x02 => {
                    let offset = Self::word(data, 3);
                    let size = Self::word(data, 0) as usize;
                    self.flash[offset as usize..][..size.next_multiple_of(0x1000)].fill(0xFF);
                    self.write_offset = Some(offset);
                    self.next_sequence = 0;
                    (0, 0)
                }
                0x03 => {
                    let len = Self::word(data, 0) as usize;
                    let sequence = Self::word(data, 1);
                    let block = &data[16..16 + len];
                    let valid = rom::checksum(block) == checksum
                        && sequence == self.next_sequence
                        && self.corrupt_block != Some(sequence);
                    if valid {
                        let start = self.write_offset.unwrap() as usize + sequence as usize * 0x400;
                        self.flash[start..start + len].copy_from_slice(block);
                        self.next_sequence += 1;
                        (0, 0)
                    } else {
                        (0, 1)
                    }
                }
                0x0A => (
                    self.registers
                        .get(&Self::word(data, 0))
                        .copied()
                        .unwrap_or(0),
                    0,
                ),
                0x09 => {
                    let (address, value, mask) = (
                        Self::word(data, 0),
                        Self::word(data, 1),
                        Self::word(data, 2),
                    );
                    let register = self.registers.entry(address).or_default();
                    *register = (*register & !mask) | (value & mask);
                    (0, 0)
                }
                _ => (0, 0),
            };
            let error = if status == 0 { 0 } else { 0x07 };
            self.respond(opcode, value, status, error);
        }
    }

    impl Read for SimulatedRom {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = buffer.len().min(self.output.len()).min(7);
            for slot in &mut buffer[..count] {
                *slot = self.output.pop_front().unwrap();
            }
            Ok(count)
        }
    }

    impl Write for SimulatedRom {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            for &byte in bytes {
                if let Some(frame) = self.decoder.push(byte) {
                    self.handle(&frame);
                }
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // Flashing

    #[test]
    fn writes_the_image_and_keeps_the_configuration() {
        let data = image::tests::test_image(0x1234);
        let image = FirmwareImage::parse(&data).unwrap();
        let mut rom = SimulatedRom::new();
        let mut stages = Vec::new();

        let outcome = flash_image(&mut rom, &image, &mut |stage| stages.push(stage)).unwrap();

        assert_eq!(outcome, FlashOutcome::Restarted);
        assert_eq!(rom.flash[..0x9000], data[..0x9000]);
        assert_eq!(rom.flash[0x1_0000..data.len()], data[0x1_0000..]);
        assert!(rom.flash[0x9000..0x1_0000].iter().all(|&b| b == 0xAB));
        assert!(rom.flash[0x41_0000..].iter().all(|&b| b == 0xAB));
        assert_eq!(rom.registers[&RTC_CNTL_OPTION1_REG], 0);
        assert_eq!(rom.registers[&RTC_CNTL_WDTCONFIG0_REG], RTC_WDT_CONFIG);
        assert_eq!(rom.registers[&RTC_CNTL_WDTWPROTECT_REG], 0);
        assert!(!rom.requests.contains(&0x04));
        assert_eq!(stages.first(), Some(&FlashStage::Connecting));
        assert_eq!(stages.last(), Some(&FlashStage::Restarting));
        assert!(stages.contains(&FlashStage::Writing {
            written: image.written_len(),
            total: image.written_len(),
        }));
    }

    #[test]
    fn asks_for_a_manual_reset_when_strapped_for_download() {
        let data = image::tests::test_image(16);
        let image = FirmwareImage::parse(&data).unwrap();
        let mut rom = SimulatedRom::new();
        rom.registers.insert(GPIO_STRAP_REG, 0);

        let outcome = flash_image(&mut rom, &image, &mut |_| {}).unwrap();

        assert_eq!(outcome, FlashOutcome::NeedsManualReset);
        assert!(!rom.registers.contains_key(&RTC_CNTL_WDTCONFIG0_REG));
    }

    #[test]
    fn stops_on_a_rejected_block() {
        let data = image::tests::test_image(0x1000);
        let image = FirmwareImage::parse(&data).unwrap();
        let mut rom = SimulatedRom::new();
        rom.corrupt_block = Some(2);

        let error = flash_image(&mut rom, &image, &mut |_| {}).unwrap_err();

        assert_eq!(error, "FlashData failed: checksum mismatch (code 0x07)");
        assert!(!rom.requests.contains(&(Opcode::WriteReg as u8)));
    }

    #[test]
    fn fails_without_a_bootloader() {
        let data = image::tests::test_image(16);
        let image = FirmwareImage::parse(&data).unwrap();
        let mut silent = std::io::Cursor::new(Vec::new());

        assert_eq!(
            flash_image(&mut silent, &image, &mut |_| {}),
            Err("the bootloader did not respond".to_string())
        );
    }

    // Ports

    #[test]
    fn recognises_the_rom_download_port() {
        let port = |pid| serialport::SerialPortInfo {
            port_name: "COM10".to_string(),
            port_type: serialport::SerialPortType::UsbPort(serialport::UsbPortInfo {
                vid: 0x303A,
                pid,
                serial_number: None,
                manufacturer: None,
                product: None,
            }),
        };

        assert!(is_download_port(&port(0x0009)));
        assert!(!is_download_port(&port(0x1001)));
    }
}
