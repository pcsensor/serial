use super::protocol::{ReceivedData, SerialEvent};
use crate::state::{Encoding, PortConfig};
use async_channel::Sender;
use serialport::{self, ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const READ_TIMEOUT: Duration = Duration::from_millis(100);
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

pub struct SerialManager {
    port: Option<Box<dyn SerialPort>>,
    receiver: Option<JoinHandle<()>>,
    stop: Option<Arc<AtomicBool>>,
}

impl Default for SerialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: None,
            receiver: None,
            stop: None,
        }
    }
    pub fn shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }
    pub fn list_ports() -> Vec<serialport::SerialPortInfo> {
        serialport::available_ports().unwrap_or_default()
    }

    pub fn open(
        &mut self,
        config: &PortConfig,
        encoding: Encoding,
        events: Sender<SerialEvent>,
    ) -> Result<(), String> {
        if self.port.is_some() {
            return Err("串口已打开，请先关闭".into());
        }
        let data_bits = match config.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            8 => DataBits::Eight,
            _ => return Err("无效的数据位".into()),
        };
        let stop_bits = match config.stop_bits.as_str() {
            "1" => StopBits::One,
            "2" => StopBits::Two,
            _ => return Err("无效的停止位".into()),
        };
        let parity = match config.parity.as_str() {
            "none" => Parity::None,
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            _ => return Err("无效的校验位".into()),
        };
        let flow = match config.flow_control.as_str() {
            "none" => FlowControl::None,
            "rts_cts" => FlowControl::Hardware,
            "xon_xoff" => FlowControl::Software,
            _ => return Err("无效的流控方式".into()),
        };
        let mut port = serialport::new(&config.port_name, config.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(flow)
            .timeout(READ_TIMEOUT)
            .open()
            .map_err(|e| format!("打开串口失败: {e}"))?;
        let mut reader = port.try_clone().map_err(|e| format!("克隆串口失败: {e}"))?;
        reader
            .set_timeout(READ_TIMEOUT)
            .map_err(|e| format!("设置接收超时失败: {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread_events = events.clone();
        let handle = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while !thread_stop.load(Ordering::Acquire) {
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let _ = thread_events.send_blocking(SerialEvent::Data(ReceivedData {
                            timestamp: crate::state::current_message_timestamp(),
                            raw_bytes: buffer[..n].to_vec(),
                            encoding: encoding.clone(),
                        }));
                    }
                    Ok(_) => {}
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::TimedOut
                                | std::io::ErrorKind::WouldBlock
                                | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(e) => {
                        let _ = thread_events
                            .send_blocking(SerialEvent::Error(format!("读取串口失败: {e}")));
                        let _ = thread_events.send_blocking(SerialEvent::Disconnected);
                        break;
                    }
                }
            }
        });
        let _ = port.write_data_terminal_ready(true);
        // Only publish a fully initialized connection. If any setup above
        // fails, the local port is dropped and state remains disconnected.
        self.port = Some(port);
        self.receiver = Some(handle);
        self.stop = Some(stop);
        Ok(())
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<usize, String> {
        let port = self.port.as_mut().ok_or("串口未打开")?;
        let deadline = Instant::now() + WRITE_TIMEOUT;
        let mut written = 0;
        while written < bytes.len() {
            if Instant::now() >= deadline {
                return Err("发送超时：串口写入缓冲区已满".into());
            }
            match port.write(&bytes[written..]) {
                Ok(0) => return Err("发送失败：写入 0 字节".into()),
                Ok(n) => written += n,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::Interrupted
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    thread::sleep(Duration::from_millis(2))
                }
                Err(e) => return Err(format!("发送失败: {e}")),
            }
        }
        Ok(written)
    }

    pub fn close(&mut self) -> Result<(), String> {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Release);
        }
        if let Some(handle) = self.receiver.take() {
            handle.join().map_err(|_| "接收线程关闭失败".to_string())?;
        }
        if let Some(port) = self.port.take() {
            let _ = port.clear(ClearBuffer::All);
        }
        Ok(())
    }
    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }
}

impl Drop for SerialManager {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_serial_options_before_open() {
        let (tx, _) = async_channel::unbounded();
        let mut manager = SerialManager::new();
        let config = PortConfig {
            data_bits: 9,
            ..Default::default()
        };
        assert!(manager.open(&config, Encoding::Ascii, tx).is_err());
        assert!(!manager.is_open());
    }
    #[test]
    fn send_requires_open_connection() {
        let mut manager = SerialManager::new();
        assert!(manager.send(b"x").is_err());
    }
}
