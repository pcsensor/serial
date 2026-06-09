use serde::{Deserialize, Serialize};
use serialport::{
    self, ClearBuffer, FlowControl as SPFlowControl, Parity, SerialPort, SerialPortInfo, StopBits,
};
use std::io::{Read, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::encoding::{self, Encoding};

const READ_POLL_TIMEOUT: Duration = Duration::from_millis(100);
const WRITE_TIMEOUT: Duration = Duration::from_millis(100);

#[cfg(unix)]
type NativePort = serialport::TTYPort;
#[cfg(windows)]
type NativePort = serialport::COMPort;

#[cfg(unix)]
fn configure_nonblocking(port: &NativePort) -> Result<(), String> {
    use std::os::unix::io::AsRawFd;

    let fd = port.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL, 0) };
    if flags < 0 {
        return Err(format!(
            "读取串口文件状态失败: {}",
            std::io::Error::last_os_error()
        ));
    }

    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(format!(
            "设置串口非阻塞模式失败: {}",
            std::io::Error::last_os_error()
        ));
    }

    Ok(())
}

fn assert_terminal_ready(port: &mut NativePort) {
    let _ = port.write_data_terminal_ready(true);
}

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
            .timeout(WRITE_TIMEOUT);

        let mut port = NativePort::open(&builder).map_err(|e| format!("打开串口失败: {}", e))?;

        #[cfg(unix)]
        // serialport opens POSIX TTYs as blocking fds; keep them nonblocking so
        // driver/poll edge cases cannot stall Tauri commands while the manager lock is held.
        configure_nonblocking(&port)?;

        assert_terminal_ready(&mut port);

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
        reader
            .set_timeout(READ_POLL_TIMEOUT)
            .map_err(|e| format!("设置接收超时失败: {}", e))?;
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.stop_flag = Some(Arc::clone(&stop_flag));

        let handle = thread::spawn(move || {
            let mut buffer = vec![0u8; 1024];
            loop {
                if stop_flag.load(Ordering::Acquire) {
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

        let mut written = 0;

        while written < data.len() {
            match port.write(&data[written..]) {
                Ok(0) => {
                    return Err("发送失败: 写入 0 字节".to_string());
                }
                Ok(n) => {
                    written += n;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                // 非阻塞模式下发送缓冲区满时返回 WouldBlock，短暂等待后重试
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => {
                    return Err(format!("发送失败: {}", e));
                }
            }
        }

        Ok(written)
    }

    pub fn close(&mut self) -> Result<(), String> {
        if let Some(stop_flag) = self.stop_flag.take() {
            stop_flag.store(true, Ordering::Release);
        }
        // 先 join 线程，确保接收线程退出后再关闭 port，避免
        // reader（try_clone_native 克隆的 fd）在 port 关闭后
        // 收到 EIO 并向已无窗口的 app_handle 发送错误事件。
        if let Some(handle) = self.receiver_handle.take() {
            handle.join().map_err(|_| "接收线程关闭失败".to_string())?;
        }
        if let Some(port) = self.port.as_mut() {
            let _ = port.clear(ClearBuffer::All);
        }
        self.port = None;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn fd_is_nonblocking(fd: std::os::unix::io::RawFd) -> bool {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL, 0) };
        flags & libc::O_NONBLOCK != 0
    }

    /// 打开 PTY 设备的测试辅助。
    ///
    /// macOS 上两个 ioctl 对 PTY 不支持，会返回 ENOTTY：
    /// 1. `TIOCEXCL` — serialport 默认 exclusive=true 时调用，PTY 不支持独占锁。
    ///    修复：`.exclusive(false)`
    /// 2. `IOSSIOSPEED` — macOS 专用的非标准波特率设置，只有真实串口才支持。
    ///    serialport 源码注释原文："attempting to set the baud rate on a pseudo terminal
    ///    via this ioctl call will fail with the ENOTTY error."
    ///    修复：`baud_rate=0`，serialport 内部跳过该 ioctl（见 termios.rs set_termios）。
    ///
    /// 生产代码打开真实串口时两者均受支持，不受影响。
    #[cfg(unix)]
    fn open_pty_for_test(manager: &mut SerialManager, config: &PortConfig) -> Result<(), String> {
        let data_bits = match config.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            8 => serialport::DataBits::Eight,
            _ => return Err("无效的数据位".to_string()),
        };
        let stop_bits = match config.stop_bits.as_str() {
            "1" => serialport::StopBits::One,
            "2" => serialport::StopBits::Two,
            _ => return Err("无效的停止位".to_string()),
        };
        let parity = match config.parity.as_str() {
            "none" => serialport::Parity::None,
            "odd"  => serialport::Parity::Odd,
            "even" => serialport::Parity::Even,
            _ => return Err("无效的校验位".to_string()),
        };
        let flow_control = match config.flow_control.as_str() {
            "none"     => serialport::FlowControl::None,
            "rts_cts"  => serialport::FlowControl::Hardware,
            "xon_xoff" => serialport::FlowControl::Software,
            _ => return Err("无效的流控方式".to_string()),
        };

        let builder = serialport::new(&config.port_name, config.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(flow_control)
            .timeout(WRITE_TIMEOUT)
            // macOS PTY 不支持 TIOCEXCL（独占 ioctl），需关闭 exclusive
            .exclusive(false)
            // macOS PTY 不支持 IOSSIOSPEED（非标准波特率 ioctl）。
            // baud_rate=0 告知 serialport 跳过该 ioctl（见 posix/termios.rs:set_termios）。
            .baud_rate(0);

        let mut port = NativePort::open(&builder)
            .map_err(|e| format!("打开串口失败: {}", e))?;

        configure_nonblocking(&port)?;
        assert_terminal_ready(&mut port);
        manager.port = Some(port);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn open_configures_unix_tty_as_nonblocking() {
        use serialport::SerialPort;
        use std::os::unix::io::AsRawFd;

        let (_master, _slave) = serialport::TTYPort::pair().unwrap();
        let port_name = _slave.name().unwrap();

        let mut manager = SerialManager::new();
        open_pty_for_test(&mut manager, &PortConfig {
            port_name,
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: "1".to_string(),
            parity: "none".to_string(),
            flow_control: "none".to_string(),
        }).unwrap();

        let fd = manager.port.as_ref().unwrap().as_raw_fd();
        assert!(fd_is_nonblocking(fd));
    }

    #[cfg(unix)]
    #[test]
    fn send_waits_for_temporarily_full_unix_output_queue() {
        use std::io::{Read, Write};

        let (mut master, _slave) = serialport::TTYPort::pair().unwrap();
        let port_name = _slave.name().unwrap();

        let mut manager = SerialManager::new();
        open_pty_for_test(&mut manager, &PortConfig {
            port_name,
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: "1".to_string(),
            parity: "none".to_string(),
            flow_control: "none".to_string(),
        }).unwrap();

        let filler = vec![0x55; 4096];
        let mut saturated = false;
        for _ in 0..256 {
            match manager.port.as_mut().unwrap().write(&filler) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    saturated = true;
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    saturated = true;
                    break;
                }
                Err(e) => panic!("unexpected write error while filling pty: {e}"),
            }
        }
        assert!(saturated, "pty output queue did not fill during test setup");

        let (release_master, wait_for_send) = std::sync::mpsc::channel();
        let drain_handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            let mut buffer = vec![0u8; 8192];
            let _ = master.read(&mut buffer);
            let _ = wait_for_send.recv_timeout(Duration::from_secs(1));
        });

        let sent = manager.send(b"x");
        let _ = release_master.send(());

        drain_handle.join().unwrap();
        assert_eq!(sent.unwrap(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn open_keeps_write_timeout_low_for_responsive_sends() {
        use serialport::SerialPort;

        let (_master, _slave) = serialport::TTYPort::pair().unwrap();
        let port_name = _slave.name().unwrap();

        let mut manager = SerialManager::new();
        open_pty_for_test(&mut manager, &PortConfig {
            port_name,
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: "1".to_string(),
            parity: "none".to_string(),
            flow_control: "none".to_string(),
        }).unwrap();

        assert_eq!(
            manager.port.as_ref().unwrap().timeout(),
            Duration::from_millis(100)
        );
    }

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
