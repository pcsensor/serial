use serde::{Deserialize, Serialize};
use serialport::{self, FlowControl as SPFlowControl, Parity, SerialPortInfo, StopBits};
use std::io::{Read, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::encoding::{self, Encoding};

#[cfg(unix)]
type NativePort = serialport::TTYPort;
#[cfg(windows)]
type NativePort = serialport::COMPort;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: String,
    pub parity: String,
    pub flow_control: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedData {
    pub timestamp: String,
    pub data: String,
    pub encoding: Encoding,
    pub raw_bytes: Vec<u8>,
}

pub struct SerialManager {
    port: Option<NativePort>,
    receiver_handle: Option<JoinHandle<()>>,
    stop_flag: Option<Arc<AtomicBool>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: None,
            receiver_handle: None,
            stop_flag: None,
        }
    }

    pub fn list_ports() -> Vec<SerialPortInfo> {
        serialport::available_ports().unwrap_or_default()
    }

    pub fn open(&mut self, config: &PortConfig) -> Result<(), String> {
        if self.port.is_some() {
            return Err("串口已打开，请先关闭".to_string());
        }

        let data_bits = match config.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            8 => serialport::DataBits::Eight,
            _ => return Err("无效的数据位".to_string()),
        };

        let stop_bits = match config.stop_bits.as_str() {
            "1" => StopBits::One,
            "2" => StopBits::Two,
            _ => return Err("无效的停止位".to_string()),
        };

        let parity = match config.parity.as_str() {
            "none" => Parity::None,
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            _ => return Err("无效的校验位".to_string()),
        };

        let flow_control = match config.flow_control.as_str() {
            "none" => SPFlowControl::None,
            "rts_cts" => SPFlowControl::Hardware,
            "xon_xoff" => SPFlowControl::Software,
            _ => return Err("无效的流控方式".to_string()),
        };

        let builder = serialport::new(&config.port_name, config.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(flow_control)
            .timeout(Duration::from_millis(100));

        let port = NativePort::open(&builder).map_err(|e| format!("打开串口失败: {}", e))?;

        self.port = Some(port);

        Ok(())
    }

    pub fn start_receiving(
        &mut self,
        app_handle: AppHandle,
        encoding: Encoding,
    ) -> Result<(), String> {
        let port = self.port.as_ref().ok_or("串口未打开")?;
        let mut reader = port
            .try_clone_native()
            .map_err(|e| format!("克隆串口失败: {}", e))?;
        // macOS 串口驱动 poll() 会假阳性返回可读，随后 read() 永久阻塞。
        // 设置 O_NONBLOCK 让 read() 在无数据时返回 WouldBlock 而非阻塞。
        // Linux 不需要（行为正确），Windows 没有 fcntl/O_NONBLOCK。
        #[cfg(target_os = "macos")]
        unsafe {
            use std::os::unix::io::AsRawFd;
            let reader_fd = reader.as_raw_fd();
            let flags = libc::fcntl(reader_fd, libc::F_GETFL, 0);
            libc::fcntl(reader_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.stop_flag = Some(Arc::clone(&stop_flag));

        let handle = thread::spawn(move || {
            let mut buffer = vec![0u8; 1024];
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let data = &buffer[..n];
                        let decoded = encoding::decode(data, &encoding)
                            .unwrap_or_else(|_| format!("{:02X?}", data));
                        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                        let received = ReceivedData {
                            timestamp,
                            data: decoded,
                            encoding: encoding.clone(),
                            raw_bytes: data.to_vec(),
                        };
                        let _ = app_handle.emit("serial-data", &received);
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        let _ = app_handle.emit("serial-error", format!("读取错误: {}", e));
                        break;
                    }
                }
            }
        });

        self.receiver_handle = Some(handle);
        Ok(())
    }

    pub fn send(&mut self, data: &[u8]) -> Result<usize, String> {
        let port = self.port.as_mut().ok_or("串口未打开")?;
        port.write(data).map_err(|e| format!("发送失败: {}", e))
    }

    pub fn close(&mut self) -> Result<(), String> {
        if let Some(stop_flag) = self.stop_flag.take() {
            stop_flag.store(true, Ordering::Relaxed);
        }
        self.port = None;
        if let Some(handle) = self.receiver_handle.take() {
            handle
                .join()
                .map_err(|_| "接收线程关闭失败".to_string())?;
        }
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_sets_stop_flag_and_joins_thread() {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let thread_stop_flag = Arc::clone(&stop_flag);
        let thread_finished = Arc::clone(&finished);
        let handle = thread::spawn(move || {
            while !thread_stop_flag.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(1));
            }
            thread_finished.store(true, Ordering::Relaxed);
        });
        let mut manager = SerialManager {
            port: None,
            receiver_handle: Some(handle),
            stop_flag: Some(stop_flag),
        };

        manager.close().unwrap();

        assert!(finished.load(Ordering::Relaxed));
        assert!(manager.receiver_handle.is_none());
        assert!(manager.stop_flag.is_none());
    }
}