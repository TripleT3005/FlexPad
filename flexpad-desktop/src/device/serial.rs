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

pub fn read_line_from_port(port: &mut Box<dyn serialport::SerialPort>, timeout: Duration) -> Option<String> {
    let start_time = std::time::Instant::now();
    let mut line = String::new();
    let mut buffer = [0; 1];

    loop {
        if start_time.elapsed() > timeout {
            break;
        }
        match port.read(&mut buffer) {
            Ok(n) if n > 0 => {
                let ch = buffer[0] as char;
                if ch == '\n' { break; }
                if ch != '\r' { line.push(ch); }
            }
            _ => { std::thread::sleep(Duration::from_millis(5)); }
        }
    }
    if line.is_empty() { None } else { Some(line) }
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
                    && response.contains("DEVICE_INFO:FlexPad") {
                        let parts: Vec<&str> = response.split(',').collect();
                        let device_id = if parts.len() > 1 { parts[1].to_string() } else { "FlexPad-Unknown".to_string() };
                        devices.push(FlexPadDeviceInfo { port_name, device_id });
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

pub fn fetch_config_from_device(port: &mut Box<dyn serialport::SerialPort>) -> Result<String, String> {
    let _ = port.clear(serialport::ClearBuffer::All);
    port.write_all(b"GET_CONFIG\n").map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let timeout = Duration::from_millis(2000);

    while start_time.elapsed() < timeout {
        if let Some(response) = read_line_from_port(port, Duration::from_millis(500))
            && response.starts_with("CONFIG_DATA:") {
                let json_data = response.strip_prefix("CONFIG_DATA:").unwrap().trim();
                return Ok(json_data.to_string());
            }
    }
    Err("Timeout waiting for config response".to_string())
}

pub fn set_config_to_device(port: &mut Box<dyn serialport::SerialPort>, json_config: &str) -> Result<(), String> {
    let _ = port.clear(serialport::ClearBuffer::All);
    let command = format!("SET_CONFIG:{}\n", json_config);
    port.write_all(command.as_bytes()).map_err(|e| e.to_string())?;

    let start_time = std::time::Instant::now();
    let timeout = Duration::from_millis(2000);

    while start_time.elapsed() < timeout {
        if let Some(response) = read_line_from_port(port, Duration::from_millis(500))
            && (response.contains("CONFIG_SAVED") || response.contains("CONFIG_OK") || response.contains("ACK")) {
                return Ok(());
            }
    }
    Err("Timeout waiting for ACK after setting config".to_string())
}