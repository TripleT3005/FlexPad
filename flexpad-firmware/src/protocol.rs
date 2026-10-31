use core::fmt::{self, Write};

// Constants

pub const PRODUCT_NAME: &str = "FlexPad";
pub const MAX_LINE_LENGTH: usize = 1024;

// Line reading

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineTooLong;

pub struct LineReader<const N: usize> {
    buffer: [u8; N],
    length: usize,
    overflowed: bool,
}

impl<const N: usize> LineReader<N> {
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            length: 0,
            overflowed: false,
        }
    }

    pub fn push(&mut self, byte: u8) -> Option<Result<&[u8], LineTooLong>> {
        match byte {
            b'\n' => {
                let length = self.length;
                let overflowed = self.overflowed;
                self.length = 0;
                self.overflowed = false;
                if overflowed {
                    Some(Err(LineTooLong))
                } else {
                    Some(Ok(&self.buffer[..length]))
                }
            }
            b'\r' => None,
            _ if self.overflowed => None,
            _ if self.length == N => {
                self.overflowed = true;
                None
            }
            _ => {
                self.buffer[self.length] = byte;
                self.length += 1;
                None
            }
        }
    }
}

impl<const N: usize> Default for LineReader<N> {
    fn default() -> Self {
        Self::new()
    }
}

// Commands

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command<'a> {
    GetDeviceInfo,
    GetConfig,
    SetConfig(&'a [u8]),
    RebootBootloader,
    Unknown,
}

pub fn parse_command(line: &[u8]) -> Command<'_> {
    let line = line.trim_ascii();
    match line {
        b"GET_DEVICE_INFO" => Command::GetDeviceInfo,
        b"GET_CONFIG" => Command::GetConfig,
        b"REBOOT_BOOTLOADER" => Command::RebootBootloader,
        _ => match line.strip_prefix(b"SET_CONFIG:") {
            Some(payload) => Command::SetConfig(payload),
            None => Command::Unknown,
        },
    }
}

// Responses

pub struct DeviceInfo<'a> {
    pub serial_number: &'a str,
    pub firmware_version: &'a str,
}

pub fn write_response(
    command: Command<'_>,
    info: &DeviceInfo<'_>,
    out: &mut impl Write,
) -> fmt::Result {
    match command {
        Command::GetDeviceInfo => writeln!(
            out,
            "DEVICE_INFO:{},{},{}",
            PRODUCT_NAME, info.serial_number, info.firmware_version
        ),
        Command::GetConfig => out.write_str("CONFIG_DATA:{}\n"),
        Command::SetConfig(_) => out.write_str("CONFIG_ERROR:not_supported\n"),
        Command::RebootBootloader => out.write_str("REBOOTING:bootloader\n"),
        Command::Unknown => out.write_str("ERROR:unknown_command\n"),
    }
}

pub fn write_line_too_long(out: &mut impl Write) -> fmt::Result {
    out.write_str("ERROR:line_too_long\n")
}

pub fn write_serial_number(mac: &[u8; 6], out: &mut impl Write) -> fmt::Result {
    out.write_str(PRODUCT_NAME)?;
    out.write_char('-')?;
    for byte in mac {
        write!(out, "{byte:02X}")?;
    }
    Ok(())
}

// Text buffer

pub struct TextBuffer<const N: usize> {
    buffer: [u8; N],
    length: usize,
}

impl<const N: usize> TextBuffer<N> {
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            length: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.length]
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.length = 0;
    }
}

impl<const N: usize> Default for TextBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Write for TextBuffer<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let bytes = text.as_bytes();
        let end = self.length + bytes.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.buffer[self.length..end].copy_from_slice(bytes);
        self.length = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Line reading

    fn feed<const N: usize>(
        reader: &mut LineReader<N>,
        data: &[u8],
    ) -> Vec<Result<Vec<u8>, LineTooLong>> {
        data.iter()
            .filter_map(|&byte| reader.push(byte).map(|result| result.map(<[u8]>::to_vec)))
            .collect()
    }

    #[test]
    fn line_reader_splits_lines_and_drops_carriage_returns() {
        let mut reader = LineReader::<32>::new();

        let lines = feed(&mut reader, b"GET_CONFIG\r\nGET_DEVICE_INFO\n");

        assert_eq!(
            lines,
            vec![Ok(b"GET_CONFIG".to_vec()), Ok(b"GET_DEVICE_INFO".to_vec())]
        );
    }

    #[test]
    fn line_reader_keeps_partial_line_until_newline() {
        let mut reader = LineReader::<32>::new();

        assert!(feed(&mut reader, b"GET_").is_empty());
        assert_eq!(
            feed(&mut reader, b"CONFIG\n"),
            vec![Ok(b"GET_CONFIG".to_vec())]
        );
    }

    #[test]
    fn line_reader_reports_overflow_once_and_recovers() {
        let mut reader = LineReader::<4>::new();

        let lines = feed(&mut reader, b"TOO_LONG\nOK\n");

        assert_eq!(lines, vec![Err(LineTooLong), Ok(b"OK".to_vec())]);
    }

    // Commands

    #[test]
    fn parse_command_recognizes_known_commands() {
        assert_eq!(parse_command(b"GET_DEVICE_INFO"), Command::GetDeviceInfo);
        assert_eq!(parse_command(b"  GET_CONFIG "), Command::GetConfig);
        assert_eq!(
            parse_command(b"SET_CONFIG:{\"a\":1}"),
            Command::SetConfig(b"{\"a\":1}")
        );
        assert_eq!(
            parse_command(b"REBOOT_BOOTLOADER"),
            Command::RebootBootloader
        );
        assert_eq!(parse_command(b"get_config"), Command::Unknown);
        assert_eq!(parse_command(b""), Command::Unknown);
    }

    // Responses

    fn response(command: Command<'_>) -> String {
        let info = DeviceInfo {
            serial_number: "FlexPad-14C19F46A1DC",
            firmware_version: "0.1.0",
        };
        let mut out = String::new();
        write_response(command, &info, &mut out).unwrap();
        out
    }

    #[test]
    fn device_info_matches_desktop_app_format() {
        let line = response(Command::GetDeviceInfo);

        assert_eq!(line, "DEVICE_INFO:FlexPad,FlexPad-14C19F46A1DC,0.1.0\n");
        assert!(line.contains("DEVICE_INFO:FlexPad"));
        assert_eq!(
            line.trim_end().split(',').nth(1),
            Some("FlexPad-14C19F46A1DC")
        );
    }

    #[test]
    fn other_commands_have_fixed_responses() {
        assert_eq!(response(Command::GetConfig), "CONFIG_DATA:{}\n");
        assert_eq!(
            response(Command::SetConfig(b"{}")),
            "CONFIG_ERROR:not_supported\n"
        );
        assert_eq!(
            response(Command::RebootBootloader),
            "REBOOTING:bootloader\n"
        );
        assert_eq!(response(Command::Unknown), "ERROR:unknown_command\n");
    }

    #[test]
    fn serial_number_uses_uppercase_mac() {
        let mut out = String::new();

        write_serial_number(&[0x14, 0xC1, 0x9F, 0x46, 0xA1, 0xDC], &mut out).unwrap();

        assert_eq!(out, "FlexPad-14C19F46A1DC");
    }

    // Text buffer

    #[test]
    fn text_buffer_rejects_text_that_does_not_fit() {
        let mut buffer = TextBuffer::<8>::new();

        assert!(buffer.write_str("FlexPad").is_ok());
        assert!(buffer.write_str("-1").is_err());
        assert_eq!(buffer.as_str(), "FlexPad");
    }
}
