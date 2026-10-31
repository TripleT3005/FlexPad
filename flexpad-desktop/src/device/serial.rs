use std::io::{Read, Write};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexPadDeviceInfo {
    pub port_name: String,
    pub device_id: String,
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

pub fn scan_flexpad_devices() -> Vec<FlexPadDeviceInfo> {
    let mut devices = Vec::new();
    let ports = match serialport::available_ports() {
        Ok(p) => p,
        Err(_) => return devices,
    };

    for p in ports {
        let port_name = p.port_name;
        if let Ok(mut port) = serialport::new(&port_name, 115200)
            .timeout(Duration::from_millis(200))
            .open()
        {
            let _ = port.write_data_terminal_ready(true);
            std::thread::sleep(Duration::from_millis(1500));
            let _ = port.clear(serialport::ClearBuffer::All);

            if port.write_all(b"GET_DEVICE_INFO\n").is_ok()
                && let Some(response) = read_line_from_port(&mut port, Duration::from_millis(600))
                && response.contains("DEVICE_INFO:FlexPad")
            {
                let parts: Vec<&str> = response.split(',').collect();
                let device_id = if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    "FlexPad-Unknown".to_string()
                };
                devices.push(FlexPadDeviceInfo {
                    port_name,
                    device_id,
                });
            }
        }
    }
    devices
}

pub fn connect_to_port(port_name: &str) -> Result<Box<dyn serialport::SerialPort>, String> {
    let mut port = serialport::new(port_name, 115200)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|e| e.to_string())?;
    let _ = port.write_data_terminal_ready(true);
    std::thread::sleep(Duration::from_millis(1000));
    let _ = port.clear(serialport::ClearBuffer::All);
    Ok(port)
}

pub fn fetch_config_from_device(
    port: &mut Box<dyn serialport::SerialPort>,
) -> Result<String, String> {
    let _ = port.clear(serialport::ClearBuffer::All);
    port.write_all(b"GET_CONFIG\n").map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let timeout = Duration::from_millis(2000);

    while start_time.elapsed() < timeout {
        if let Some(response) = read_line_from_port(port, Duration::from_millis(500))
            && response.starts_with("CONFIG_DATA:")
        {
            let json_data = response.strip_prefix("CONFIG_DATA:").unwrap().trim();
            return Ok(json_data.to_string());
        }
    }
    Err("Timeout waiting for config response".to_string())
}

pub fn set_config_to_device(
    port: &mut Box<dyn serialport::SerialPort>,
    json_config: &str,
) -> Result<(), String> {
    let _ = port.clear(serialport::ClearBuffer::All);
    let command = format!("SET_CONFIG:{}\n", json_config);
    port.write_all(command.as_bytes())
        .map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let timeout = Duration::from_millis(2000);

    while start_time.elapsed() < timeout {
        let Some(response) = read_line_from_port(port, Duration::from_millis(500)) else {
            continue;
        };
        if response.contains("CONFIG_SAVED")
            || response.contains("CONFIG_OK")
            || response.contains("ACK")
        {
            return Ok(());
        }
        if let Some(reason) = response.strip_prefix("CONFIG_ERROR:") {
            return Err(format!("The device rejected the configuration ({reason})"));
        }
    }
    Err("Timeout waiting for ACK after setting config".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // Line reading

    const TIMEOUT: Duration = Duration::from_millis(20);

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
