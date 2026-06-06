# 串口调试助手实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 构建基于 Rust + Dioxus + Tauri v2 的跨平台串口调试助手，支持多编码收发、预设指令管理、数据导出，采用现代化毛玻璃透明 UI。

**架构：** Dioxus 编译为 WASM 运行在 Tauri v2 WebView 中作为前端，Rust 原生代码作为后端处理串口通信。前后端通过 Tauri IPC（invoke/event）通信。窗口使用系统级毛玻璃效果（macOS vibrancy / Windows Mica）。

**技术栈：** Tauri v2, Dioxus 0.7, serialport 4.9, window-vibrancy 0.7, tauri-plugin-store, tauri-plugin-dialog, serde, encoding_rs

---

## 文件结构

```
serial-debugger/
├── Cargo.toml                          # workspace 根配置
├── Dioxus.toml                         # Dioxus CLI 配置
├── src-tauri/
│   ├── Cargo.toml                      # 后端依赖
│   ├── tauri.conf.json                 # Tauri 配置（透明窗口、无装饰等）
│   ├── capabilities/
│   │   └── default.json                # Tauri 权限声明
│   ├── icons/                          # 应用图标
│   └── src/
│       ├── main.rs                     # Tauri 入口，注册插件和命令
│       ├── serial/
│       │   ├── mod.rs                  # 串口模块导出
│       │   ├── manager.rs              # SerialManager：串口生命周期管理
│       │   ├── commands.rs             # Tauri commands：list_ports, open_port 等
│       │   └── encoding.rs             # 编码/解码：ASCII, HEX, GBK, UTF-8
│       └── export/
│           ├── mod.rs                  # 导出模块导出
│           └── exporter.rs             # TXT/CSV 导出逻辑
├── src/
│   ├── main.rs                         # Dioxus 入口
│   ├── app.rs                          # 根组件：整体布局
│   ├── components/
│   │   ├── mod.rs                      # 组件模块导出
│   │   ├── title_bar.rs                # 自定义标题栏（拖拽、最小化、关闭）
│   │   ├── status_bar.rs               # 底部状态栏
│   │   ├── tab_nav.rs                  # 标签页导航
│   │   ├── serial_panel.rs             # 串口配置面板（左侧）
│   │   ├── receive_send_tab.rs         # 收发标签页
│   │   └── command_manager_tab.rs      # 指令管理标签页
│   ├── services/
│   │   ├── mod.rs                      # 服务模块导出
│   │   ├── api.rs                      # Tauri IPC 封装（invoke + listen）
│   │   └── store.rs                    # 全局状态管理（Signals）
│   └── assets/
│       └── main.css                    # 全局样式（毛玻璃、渐变、透明）
```

---

## 任务 1：项目脚手架

**文件：**
- 创建：`Cargo.toml`（workspace）
- 创建：`Dioxus.toml`
- 创建：`src-tauri/Cargo.toml`
- 创建：`src-tauri/tauri.conf.json`
- 创建：`src-tauri/capabilities/default.json`
- 创建：`src-tauri/src/main.rs`（最小可运行）
- 创建：`src/main.rs`（最小 Dioxus 应用）
- 创建：`src/app.rs`（占位根组件）

- [ ] **步骤 1：安装工具链**

```bash
cargo install create-tauri-app --locked
cargo install dioxus-cli --locked
```

- [ ] **步骤 2：创建项目目录和 workspace Cargo.toml**

```bash
mkdir serial-debugger && cd serial-debugger
```

```toml
# Cargo.toml（项目根目录）
[workspace]
members = ["src-tauri"]
resolver = "2"
```

- [ ] **步骤 3：创建 Dioxus.toml**

```toml
# Dioxus.toml
[application]
name = "serial-debugger"

[web.app]
title = "串口调试助手"

[web.watcher]
reload_html = true

[web.resource.dev]
```

- [ ] **步骤 4：创建 src-tauri/Cargo.toml**

```toml
# src-tauri/Cargo.toml
[package]
name = "serial-debugger"
version = "0.1.0"
edition = "2021"

[lib]
name = "serial_debugger_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["macos-private-apis"] }
tauri-plugin-opener = "2"
tauri-plugin-store = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serialport = "4.9"
window-vibrancy = "0.7"
encoding_rs = "0.8"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
```

- [ ] **步骤 5：创建 src-tauri/build.rs**

```rust
// src-tauri/build.rs
fn main() {
    tauri_build::build()
}
```

- [ ] **步骤 6：创建 src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-cli/schema.json",
  "productName": "串口调试助手",
  "version": "0.1.0",
  "identifier": "com.serial-debugger.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:8080",
    "beforeDevCommand": "dx serve --port 8080 --interactive false",
    "beforeBuildCommand": "dx build --release"
  },
  "app": {
    "windows": [
      {
        "title": "串口调试助手",
        "width": 1100,
        "height": 750,
        "minWidth": 900,
        "minHeight": 600,
        "decorations": false,
        "transparent": true,
        "resizable": true
      }
    ],
    "macOSPrivateApis": true,
    "security": {
      "csp": null
    }
  }
}
```

- [ ] **步骤 7：创建 src-tauri/capabilities/default.json**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "默认权限",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-close",
    "core:window:allow-minimize",
    "core:window:allow-start-dragging",
    "opener:default",
    "store:default",
    "dialog:default"
  ]
}
```

- [ ] **步骤 8：创建最小 src-tauri/src/main.rs**

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
```

同时创建 `src-tauri/src/lib.rs`：

```rust
// src-tauri/src/lib.rs
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
```

- [ ] **步骤 9：创建前端 Cargo.toml（项目根目录）**

在根 `Cargo.toml` 中添加前端 package：

```toml
# Cargo.toml（更新）
[workspace]
members = ["src-tauri"]
resolver = "2"

[package]
name = "serial-debugger-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus = { version = "0.7", features = ["web"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["Window", "console"] }
gloo-timers = { version = "0.3", features = ["futures"] }
chrono = "0.4"
```

- [ ] **步骤 10：创建最小 src/main.rs 和 src/app.rs**

```rust
// src/main.rs
use dioxus::prelude::*;

mod app;

fn main() {
    dioxus::launch(app::App);
}
```

```rust
// src/app.rs
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    rsx! {
        div {
            h1 { "串口调试助手" }
            p { "加载中..." }
        }
    }
}
```

- [ ] **步骤 11：创建空目录结构**

```bash
mkdir -p src-tauri/src/serial
mkdir -p src-tauri/src/export
mkdir -p src-tauri/icons
mkdir -p src/components
mkdir -p src/services
mkdir -p src/assets
touch src-tauri/src/serial/mod.rs
touch src-tauri/src/export/mod.rs
touch src/components/mod.rs
touch src/services/mod.rs
touch src/assets/main.css
```

- [ ] **步骤 12：验证项目可编译运行**

```bash
cargo tauri dev
```

预期：应用窗口打开，显示 "串口调试助手" 和 "加载中..." 文本。

- [ ] **步骤 13：Commit**

```bash
git init
git add .
git commit -m "feat: 初始化 Tauri v2 + Dioxus 项目脚手架"
```

---

## 任务 2：编码/解码模块

**文件：**
- 创建：`src-tauri/src/serial/encoding.rs`
- 测试：`src-tauri/src/serial/encoding.rs`（内联 `#[cfg(test)]` 模块）

- [ ] **步骤 1：编写编码模块测试**

在 `src-tauri/src/serial/encoding.rs` 中先写测试：

```rust
// src-tauri/src/serial/encoding.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Ascii,
    Hex,
    Utf8,
    Gbk,
}

pub fn encode(input: &str, encoding: &Encoding) -> Result<Vec<u8>, String> {
    match encoding {
        Encoding::Ascii => Ok(input.as_bytes().to_vec()),
        Encoding::Utf8 => Ok(input.as_bytes().to_vec()),
        Encoding::Hex => parse_hex(input),
        Encoding::Gbk => encode_gbk(input),
    }
}

pub fn decode(data: &[u8], encoding: &Encoding) -> Result<String, String> {
    match encoding {
        Encoding::Ascii => Ok(data.iter().map(|b| *b as char).collect()),
        Encoding::Utf8 => String::from_utf8(data.to_vec()).map_err(|e| e.to_string()),
        Encoding::Hex => Ok(format_hex(data)),
        Encoding::Gbk => decode_gbk(data),
    }
}

fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err("HEX 字符串长度必须为偶数".to_string());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| format!("无效的 HEX 字符: {}", e))
        })
        .collect()
}

fn format_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn encode_gbk(input: &str) -> Result<Vec<u8>, String> {
    let (encoded, _, had_errors) = encoding_rs::GBK.encode(input);
    if had_errors {
        Err("GBK 编码失败：包含不支持的字符".to_string())
    } else {
        Ok(encoded.into_owned())
    }
}

fn decode_gbk(data: &[u8]) -> Result<String, String> {
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(data);
    if had_errors {
        Err("GBK 解码失败：无效的字节序列".to_string())
    } else {
        Ok(decoded.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_encode_decode() {
        let data = encode("Hello", &Encoding::Ascii).unwrap();
        assert_eq!(data, vec![72, 101, 108, 108, 111]);
        let text = decode(&data, &Encoding::Ascii).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_hex_encode_decode() {
        let data = encode("48 65 6C 6C 6F", &Encoding::Hex).unwrap();
        assert_eq!(data, vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
        let text = decode(&data, &Encoding::Hex).unwrap();
        assert_eq!(text, "48 65 6C 6C 6F");
    }

    #[test]
    fn test_hex_without_spaces() {
        let data = encode("48656C6C6F", &Encoding::Hex).unwrap();
        assert_eq!(data, vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
    }

    #[test]
    fn test_hex_odd_length_error() {
        let result = encode("486", &Encoding::Hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_utf8_encode_decode() {
        let data = encode("你好世界", &Encoding::Utf8).unwrap();
        let text = decode(&data, &Encoding::Utf8).unwrap();
        assert_eq!(text, "你好世界");
    }

    #[test]
    fn test_gbk_encode_decode() {
        let data = encode("你好", &Encoding::Gbk).unwrap();
        let text = decode(&data, &Encoding::Gbk).unwrap();
        assert_eq!(text, "你好");
    }

    #[test]
    fn test_hex_format() {
        assert_eq!(format_hex(&[0x00, 0xFF, 0x0A]), "00 FF 0A");
    }
}
```

- [ ] **步骤 2：运行测试验证通过**

```bash
cargo test --lib serial::encoding -- --nocapture
```

预期：所有 7 个测试 PASS。

- [ ] **步骤 3：更新 serial/mod.rs**

```rust
// src-tauri/src/serial/mod.rs
pub mod encoding;
```

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/serial/
git commit -m "feat: 添加编码/解码模块（ASCII/HEX/UTF-8/GBK）"
```

---

## 任务 3：串口管理器

**文件：**
- 创建：`src-tauri/src/serial/manager.rs`

- [ ] **步骤 1：实现 SerialManager**

```rust
// src-tauri/src/serial/manager.rs
use serialport::{self, SerialPort, SerialPortInfo, Parity, StopBits, FlowControl as SPFlowControl};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::encoding::{self, Encoding};

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
    port: Option<Box<dyn SerialPort>>,
    receiver_handle: Option<JoinHandle<()>>,
    stop_flag: Arc<Mutex<bool>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: None,
            receiver_handle: None,
            stop_flag: Arc::new(Mutex::new(false)),
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
            "1.5" => StopBits::OnePointFive,
            "2" => StopBits::Two,
            _ => return Err("无效的停止位".to_string()),
        };

        let parity = match config.parity.as_str() {
            "none" => Parity::None,
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            "mark" => Parity::Mark,
            "space" => Parity::Space,
            _ => return Err("无效的校验位".to_string()),
        };

        let flow_control = match config.flow_control.as_str() {
            "none" => SPFlowControl::None,
            "rts_cts" => SPFlowControl::RtsCts,
            "xon_xoff" => SPFlowControl::XonXoff,
            _ => return Err("无效的流控方式".to_string()),
        };

        let port = serialport::new(&config.port_name, config.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(flow_control)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| format!("打开串口失败: {}", e))?;

        self.port = Some(port);
        *self.stop_flag.lock().unwrap() = false;

        Ok(())
    }

    pub fn start_receiving(
        &mut self,
        app_handle: AppHandle,
        encoding: Encoding,
    ) -> Result<(), String> {
        let port = self.port.as_ref().ok_or("串口未打开")?;
        let mut reader = port.try_clone().map_err(|e| format!("克隆串口失败: {}", e))?;
        let stop_flag = Arc::clone(&self.stop_flag);

        let handle = thread::spawn(move || {
            let mut buffer = vec![0u8; 1024];
            loop {
                if *stop_flag.lock().unwrap() {
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
        *self.stop_flag.lock().unwrap() = true;
        if let Some(handle) = self.receiver_handle.take() {
            let _ = handle.join();
        }
        self.port = None;
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }
}
```

- [ ] **步骤 2：更新 serial/mod.rs**

```rust
// src-tauri/src/serial/mod.rs
pub mod encoding;
pub mod manager;
```

- [ ] **步骤 3：验证编译通过**

```bash
cargo check -p serial-debugger
```

预期：编译无错误。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/src/serial/manager.rs src-tauri/src/serial/mod.rs
git commit -m "feat: 添加串口管理器（打开/关闭/收发/接收线程）"
```

---

## 任务 4：Tauri Commands

**文件：**
- 创建：`src-tauri/src/serial/commands.rs`
- 修改：`src-tauri/src/main.rs`

- [ ] **步骤 1：实现 Tauri commands**

```rust
// src-tauri/src/serial/commands.rs
use std::sync::Mutex;
use serialport::SerialPortInfo;
use tauri::{AppHandle, State};
use serde::{Deserialize, Serialize};

use super::encoding::Encoding;
use super::manager::{PortConfig, SerialManager};

pub struct SerialState(pub Mutex<SerialManager>);

#[derive(Serialize)]
pub struct PortInfo {
    pub port_name: String,
    pub port_type: String,
}

#[tauri::command]
pub fn list_ports() -> Vec<PortInfo> {
    SerialManager::list_ports()
        .into_iter()
        .map(|p| PortInfo {
            port_name: p.port_name,
            port_type: format!("{:?}", p.port_type),
        })
        .collect()
}

#[tauri::command]
pub fn open_port(
    state: State<SerialState>,
    app: AppHandle,
    config: PortConfig,
    encoding: Encoding,
) -> Result<(), String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.open(&config)?;
    manager.start_receiving(app, encoding)?;
    Ok(())
}

#[tauri::command]
pub fn close_port(state: State<SerialState>) -> Result<(), String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.close()
}

#[derive(Deserialize)]
pub struct SendRequest {
    pub content: String,
    pub encoding: Encoding,
}

#[tauri::command]
pub fn send_data(
    state: State<SerialState>,
    request: SendRequest,
) -> Result<usize, String> {
    let data = super::encoding::encode(&request.content, &request.encoding)?;
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.send(&data)
}

#[tauri::command]
pub fn is_port_open(state: State<SerialState>) -> bool {
    let manager = state.0.lock().unwrap();
    manager.is_open()
}
```

- [ ] **步骤 2：更新 main.rs 注册 commands**

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod serial;
mod export;

use serial::commands::SerialState;
use serial::manager::SerialManager;
use std::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(SerialState(Mutex::new(SerialManager::new())))
        .invoke_handler(tauri::generate_handler![
            serial::commands::list_ports,
            serial::commands::open_port,
            serial::commands::close_port,
            serial::commands::send_data,
            serial::commands::is_port_open,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
```

同时更新 `src-tauri/src/lib.rs`：

```rust
// src-tauri/src/lib.rs
mod serial;
mod export;

use serial::commands::SerialState;
use serial::manager::SerialManager;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(SerialState(Mutex::new(SerialManager::new())))
        .invoke_handler(tauri::generate_handler![
            serial::commands::list_ports,
            serial::commands::open_port,
            serial::commands::close_port,
            serial::commands::send_data,
            serial::commands::is_port_open,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
```

- [ ] **步骤 3：验证编译通过**

```bash
cargo check -p serial-debugger
```

预期：编译无错误。

- [ ] **步骤 4：Commit**

```bash
git add src-tauri/
git commit -m "feat: 注册 Tauri commands（list_ports/open/close/send）"
```

---

## 任务 5：数据导出模块

**文件：**
- 创建：`src-tauri/src/export/exporter.rs`
- 创建：`src-tauri/src/export/mod.rs`
- 修改：`src-tauri/src/main.rs`

- [ ] **步骤 1：实现导出逻辑**

```rust
// src-tauri/src/export/exporter.rs
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: String,
    pub encoding: String,
    pub data: String,
}

pub fn export_txt(entries: &[LogEntry], path: &str) -> Result<(), String> {
    let content: String = entries
        .iter()
        .map(|e| format!("[{}] {}: {}", e.timestamp, e.direction, e.data))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(Path::new(path), content).map_err(|e| format!("写入文件失败: {}", e))
}

pub fn export_csv(entries: &[LogEntry], path: &str) -> Result<(), String> {
    let mut content = String::from("时间,方向,编码,数据\n");
    for e in entries {
        let escaped_data = e.data.replace('"', "\"\"");
        content.push_str(&format!(
            "{},{},{},\"{}\"\n",
            e.timestamp, e.direction, e.encoding, escaped_data
        ));
    }
    fs::write(Path::new(path), content).map_err(|e| format!("写入文件失败: {}", e))
}

#[tauri::command]
pub fn export_data(
    entries: Vec<LogEntry>,
    path: String,
    format: String,
) -> Result<(), String> {
    match format.as_str() {
        "txt" => export_txt(&entries, &path),
        "csv" => export_csv(&entries, &path),
        _ => Err("不支持的导出格式".to_string()),
    }
}

#[tauri::command]
pub async fn save_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("文本文件", &["txt"])
        .add_filter("CSV 文件", &["csv"])
        .save_file(move |path| {
            let _ = tx.send(path.map(|p| p.to_string()));
        });
    rx.recv().map_err(|e| e.to_string())
}
```

- [ ] **步骤 2：更新 export/mod.rs**

```rust
// src-tauri/src/export/mod.rs
pub mod exporter;
```

- [ ] **步骤 3：在 main.rs 注册导出 commands**

在 `invoke_handler` 中添加：

```rust
.invoke_handler(tauri::generate_handler![
    serial::commands::list_ports,
    serial::commands::open_port,
    serial::commands::close_port,
    serial::commands::send_data,
    serial::commands::is_port_open,
    export::exporter::export_data,
    export::exporter::save_dialog,
])
```

- [ ] **步骤 4：验证编译通过**

```bash
cargo check -p serial-debugger
```

- [ ] **步骤 5：Commit**

```bash
git add src-tauri/
git commit -m "feat: 添加数据导出模块（TXT/CSV）"
```

---

## 任务 6：毛玻璃效果与窗口配置

**文件：**
- 修改：`src-tauri/src/main.rs`
- 修改：`src-tauri/src/lib.rs`

- [ ] **步骤 1：在 setup 中应用毛玻璃效果**

更新 `main.rs`：

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod serial;
mod export;

use serial::commands::SerialState;
use serial::manager::SerialManager;
use std::sync::Mutex;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(SerialState(Mutex::new(SerialManager::new())))
        .invoke_handler(tauri::generate_handler![
            serial::commands::list_ports,
            serial::commands::open_port,
            serial::commands::close_port,
            serial::commands::send_data,
            serial::commands::is_port_open,
            export::exporter::export_data,
            export::exporter::save_dialog,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            window_vibrancy::apply_vibrancy(
                &window,
                window_vibrancy::NSVisualEffectMaterial::Sidebar,
                None,
                None,
            )
            .expect("应用 vibrancy 失败");

            #[cfg(target_os = "windows")]
            {
                window_vibrancy::apply_mica(&window, None)
                    .or_else(|_| window_vibrancy::apply_acrylic(&window, None))
                    .expect("应用毛玻璃效果失败");
            }

            #[cfg(target_os = "linux")]
            {
                let _ = window_vibrancy::apply_blur(&window, None);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
```

同样更新 `lib.rs` 中的 `run()` 函数，添加相同的 `.setup()` 逻辑。

- [ ] **步骤 2：验证编译通过**

```bash
cargo check -p serial-debugger
```

- [ ] **步骤 3：Commit**

```bash
git add src-tauri/
git commit -m "feat: 添加系统级毛玻璃效果（vibrancy/Mica/Acrylic）"
```

---

## 任务 7：前端 Tauri API 封装与全局状态

**文件：**
- 创建：`src/services/api.rs`
- 创建：`src/services/store.rs`
- 创建：`src/services/mod.rs`

- [ ] **步骤 1：实现 Tauri IPC 封装**

```rust
// src/services/api.rs
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &js_sys::Function) -> JsValue;
}

pub async fn tauri_invoke<T: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &T,
) -> Result<R, String> {
    let args_js = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let result = invoke(cmd, args_js).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

pub async fn tauri_invoke_no_args<R: for<'de> Deserialize<'de>>(
    cmd: &str,
) -> Result<R, String> {
    let args_js = JsValue::NULL;
    let result = invoke(cmd, args_js).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

pub fn tauri_listen<F>(event: &str, callback: F)
where
    F: 'static + Fn(JsValue),
{
    let closure = Closure::wrap(Box::new(callback) as Box<dyn Fn(JsValue)>);
    let _ = listen(event, closure.as_ref().unchecked_ref());
    closure.forget();
}
```

- [ ] **步骤 2：添加 serde-wasm-bindgen 依赖**

在根 `Cargo.toml` 的 `[dependencies]` 中添加：

```toml
serde-wasm-bindgen = "0.6"
```

- [ ] **步骤 3：实现全局状态管理**

```rust
// src/services/store.rs
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: String,
    pub parity: String,
    pub flow_control: String,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: "1".to_string(),
            parity: "none".to_string(),
            flow_control: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Ascii,
    Hex,
    Utf8,
    Gbk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedMessage {
    pub timestamp: String,
    pub data: String,
    pub encoding: Encoding,
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: String,
    pub encoding: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCommand {
    pub id: String,
    pub name: String,
    pub content: String,
    pub encoding: Encoding,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTab {
    ReceiveSend,
    CommandManager,
}

#[derive(Clone)]
pub struct AppState {
    pub is_connected: Signal<bool>,
    pub port_config: Signal<PortConfig>,
    pub active_tab: Signal<ActiveTab>,
    pub received_messages: Signal<Vec<ReceivedMessage>>,
    pub log_entries: Signal<Vec<LogEntry>>,
    pub preset_commands: Signal<Vec<PresetCommand>>,
    pub bytes_received: Signal<u64>,
    pub bytes_sent: Signal<u64>,
    pub send_encoding: Signal<Encoding>,
    pub auto_scroll: Signal<bool>,
    pub loop_send: Signal<bool>,
    pub loop_interval_ms: Signal<u64>,
}

impl AppState {
    pub fn init() -> Self {
        Self {
            is_connected: Signal::new(false),
            port_config: Signal::new(PortConfig::default()),
            active_tab: Signal::new(ActiveTab::ReceiveSend),
            received_messages: Signal::new(Vec::new()),
            log_entries: Signal::new(Vec::new()),
            preset_commands: Signal::new(Vec::new()),
            bytes_received: Signal::new(0),
            bytes_sent: Signal::new(0),
            send_encoding: Signal::new(Encoding::Ascii),
            auto_scroll: Signal::new(true),
            loop_send: Signal::new(false),
            loop_interval_ms: Signal::new(1000),
        }
    }
}
```

- [ ] **步骤 4：更新 services/mod.rs**

```rust
// src/services/mod.rs
pub mod api;
pub mod store;
```

- [ ] **步骤 5：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 6：Commit**

```bash
git add src/services/ Cargo.toml
git commit -m "feat: 前端 Tauri API 封装与全局状态管理"
```

---

## 任务 8：全局样式与根组件

**文件：**
- 创建：`src/assets/main.css`
- 修改：`src/app.rs`
- 修改：`src/main.rs`

- [ ] **步骤 1：编写全局样式**

```css
/* src/assets/main.css */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

html, body {
    background: transparent;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC",
        "Microsoft YaHei", sans-serif;
    color: #e0e0e0;
    overflow: hidden;
    height: 100%;
}

#main {
    height: 100%;
    display: flex;
    flex-direction: column;
}

.app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: linear-gradient(135deg, rgba(30, 30, 60, 0.85), rgba(20, 20, 40, 0.9));
    border-radius: 12px;
    overflow: hidden;
}

.glass-card {
    background: rgba(255, 255, 255, 0.06);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
}

.glass-input {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    color: #e0e0e0;
    padding: 6px 10px;
    font-size: 13px;
    outline: none;
    transition: border-color 0.2s;
}

.glass-input:focus {
    border-color: rgba(100, 140, 255, 0.6);
}

.glass-select {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 6px;
    color: #e0e0e0;
    padding: 6px 10px;
    font-size: 13px;
    outline: none;
    cursor: pointer;
}

.glass-select option {
    background: #2a2a3e;
    color: #e0e0e0;
}

.btn {
    padding: 6px 16px;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.2s;
    font-weight: 500;
}

.btn-primary {
    background: linear-gradient(135deg, #667eea, #764ba2);
    color: white;
}

.btn-primary:hover {
    background: linear-gradient(135deg, #7b8ff5, #8a5db8);
    box-shadow: 0 4px 15px rgba(102, 126, 234, 0.3);
}

.btn-danger {
    background: linear-gradient(135deg, #f5576c, #ff6b6b);
    color: white;
}

.btn-danger:hover {
    background: linear-gradient(135deg, #ff6b7f, #ff8585);
    box-shadow: 0 4px 15px rgba(245, 87, 108, 0.3);
}

.btn-secondary {
    background: rgba(255, 255, 255, 0.1);
    color: #ccc;
    border: 1px solid rgba(255, 255, 255, 0.15);
}

.btn-secondary:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
}

.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
    gap: 12px;
    padding: 0 12px 12px;
}

.sidebar {
    width: 220px;
    flex-shrink: 0;
}

.content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

/* 滚动条 */
::-webkit-scrollbar {
    width: 6px;
}

::-webkit-scrollbar-track {
    background: transparent;
}

::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.25);
}
```

- [ ] **步骤 2：更新 app.rs 根组件**

```rust
// src/app.rs
use dioxus::prelude::*;

mod components;
mod services;

use services::store::{AppState, ActiveTab};

#[component]
pub fn App() -> Element {
    let state = AppState::init();
    use_context_provider(|| state.clone());

    rsx! {
        div { class: "app-container",
            components::title_bar::TitleBar {}
            div { class: "main-content",
                div { class: "sidebar",
                    components::serial_panel::SerialPanel {}
                }
                div { class: "content-area",
                    components::tab_nav::TabNav {}
                    match *state.active_tab.read() {
                        ActiveTab::ReceiveSend => rsx! {
                            components::receive_send_tab::ReceiveSendTab {}
                        },
                        ActiveTab::CommandManager => rsx! {
                            components::command_manager_tab::CommandManagerTab {}
                        },
                    }
                }
            }
            components::status_bar::StatusBar {}
        }
    }
}
```

- [ ] **步骤 3：更新 main.rs 引入 CSS**

```rust
// src/main.rs
use dioxus::prelude::*;

mod app;

fn main() {
    dioxus::launch(app::App);
}
```

在 `Dioxus.toml` 中配置 CSS 引入，或在 `app.rs` 中使用 `document::Stylesheet`。

- [ ] **步骤 4：创建组件占位文件**

为每个组件创建最小占位实现，确保编译通过：

```rust
// src/components/mod.rs
pub mod title_bar;
pub mod status_bar;
pub mod tab_nav;
pub mod serial_panel;
pub mod receive_send_tab;
pub mod command_manager_tab;
```

每个组件文件创建最小结构，例如：

```rust
// src/components/title_bar.rs（占位）
use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    rsx! { div { "标题栏" } }
}
```

其余组件同理。

- [ ] **步骤 5：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 6：Commit**

```bash
git add src/
git commit -m "feat: 全局样式、根组件布局与组件占位"
```

---

## 任务 9：自定义标题栏

**文件：**
- 修改：`src/components/title_bar.rs`

- [ ] **步骤 1：实现标题栏组件**

```rust
// src/components/title_bar.rs
use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    let minimize = move |_| async move {
        let _: Result<(), String> = crate::services::api::tauri_invoke_no_args("plugin:window|minimize").await;
    };

    let close = move |_| async move {
        let _: Result<(), String> = crate::services::api::tauri_invoke_no_args("plugin:window|close").await;
    };

    rsx! {
        div {
            style: "display:flex;align-items:center;justify-content:space-between;padding:8px 16px;background:rgba(0,0,0,0.2);",
            "data-tauri-drag-region": "true",
            div {
                style: "display:flex;align-items:center;gap:8px;",
                "data-tauri-drag-region": "true",
                span {
                    style: "font-size:14px;font-weight:600;background:linear-gradient(135deg,#667eea,#764ba2);-webkit-background-clip:text;-webkit-text-fill-color:transparent;",
                    "⚡ 串口调试助手"
                }
            }
            div {
                style: "display:flex;gap:8px;",
                button {
                    class: "btn btn-secondary",
                    style: "padding:4px 10px;font-size:12px;",
                    onclick: minimize,
                    "—"
                }
                button {
                    class: "btn btn-danger",
                    style: "padding:4px 10px;font-size:12px;",
                    onclick: close,
                    "✕"
                }
            }
        }
    }
}
```

- [ ] **步骤 2：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/title_bar.rs
git commit -m "feat: 自定义标题栏（拖拽、最小化、关闭）"
```

---

## 任务 10：串口配置面板

**文件：**
- 修改：`src/components/serial_panel.rs`

- [ ] **步骤 1：实现串口配置面板**

```rust
// src/components/serial_panel.rs
use dioxus::prelude::*;
use crate::services::store::*;
use crate::services::api;

#[derive(Debug, Clone, serde::Deserialize)]
struct PortInfo {
    port_name: String,
    port_type: String,
}

#[component]
pub fn SerialPanel() -> Element {
    let state: AppState = use_context();
    let ports = use_signal(Vec::<PortInfo>::new);
    let loading = use_signal(|| false);

    let refresh_ports = move |_| {
        spawn(async move {
            let result: Result<Vec<PortInfo>, String> =
                api::tauri_invoke_no_args("list_ports").await;
            if let Ok(p) = result {
                *ports.write() = p;
            }
        });
    };

    let toggle_connection = move |_| {
        let is_connected = *state.is_connected.read();
        let config = (*state.port_config.read()).clone();
        let encoding = (*state.send_encoding.read()).clone();
        spawn(async move {
            if is_connected {
                let result: Result<(), String> =
                    api::tauri_invoke_no_args("close_port").await;
                if result.is_ok() {
                    *state.is_connected.write() = false;
                }
            } else {
                #[derive(serde::Serialize)]
                struct Args {
                    config: PortConfig,
                    encoding: Encoding,
                }
                let result: Result<(), String> = api::tauri_invoke(
                    "open_port",
                    &Args { config, encoding },
                ).await;
                if result.is_ok() {
                    *state.is_connected.write() = true;
                }
            }
        });
    };

    use_effect(move || {
        refresh_ports(());
    });

    let is_connected = *state.is_connected.read();
    let config = state.port_config.read().clone();

    rsx! {
        div {
            class: "glass-card",
            style: "padding:14px;display:flex;flex-direction:column;gap:10px;height:100%;",
            div {
                style: "font-size:13px;font-weight:600;margin-bottom:4px;color:#a0a8d0;",
                "串口配置"
            }

            // 端口选择
            label_item { "端口" }
            div { style: "display:flex;gap:4px;",
                select {
                    class: "glass-select",
                    style: "flex:1;",
                    value: "{config.port_name}",
                    onchange: move |e| {
                        state.port_config.write().port_name = e.value();
                    },
                    option { value: "", "选择端口" }
                    for p in ports.read().iter() {
                        option { value: "{p.port_name}", "{p.port_name}" }
                    }
                }
                button {
                    class: "btn btn-secondary",
                    style: "padding:4px 8px;font-size:11px;",
                    onclick: refresh_ports,
                    "↻"
                }
            }

            // 波特率
            label_item { "波特率" }
            select {
                class: "glass-select",
                value: "{config.baud_rate}",
                onchange: move |e| {
                    if let Ok(v) = e.value().parse() {
                        state.port_config.write().baud_rate = v;
                    }
                },
                for rate in [9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600] {
                    option { value: "{rate}", "{rate}" }
                }
            }

            // 数据位
            label_item { "数据位" }
            select {
                class: "glass-select",
                value: "{config.data_bits}",
                onchange: move |e| {
                    if let Ok(v) = e.value().parse() {
                        state.port_config.write().data_bits = v;
                    }
                },
                for bits in [5, 6, 7, 8] {
                    option { value: "{bits}", "{bits}" }
                }
            }

            // 停止位
            label_item { "停止位" }
            select {
                class: "glass-select",
                value: "{config.stop_bits}",
                onchange: move |e| {
                    state.port_config.write().stop_bits = e.value();
                },
                option { value: "1", "1" }
                option { value: "1.5", "1.5" }
                option { value: "2", "2" }
            }

            // 校验位
            label_item { "校验位" }
            select {
                class: "glass-select",
                value: "{config.parity}",
                onchange: move |e| {
                    state.port_config.write().parity = e.value();
                },
                option { value: "none", "None" }
                option { value: "odd", "Odd" }
                option { value: "even", "Even" }
                option { value: "mark", "Mark" }
                option { value: "space", "Space" }
            }

            // 流控
            label_item { "流控" }
            select {
                class: "glass-select",
                value: "{config.flow_control}",
                onchange: move |e| {
                    state.port_config.write().flow_control = e.value();
                },
                option { value: "none", "None" }
                option { value: "rts_cts", "RTS/CTS" }
                option { value: "xon_xoff", "XON/XOFF" }
            }

            // 连接按钮
            div { style: "margin-top:auto;",
                button {
                    class: if is_connected { "btn btn-danger" } else { "btn btn-primary" },
                    style: "width:100%;padding:10px;",
                    onclick: toggle_connection,
                    if is_connected { "断开连接" } else { "打开串口" }
                }
            }
        }
    }
}

#[component]
fn label_item(children: Element) -> Element {
    rsx! {
        div {
            style: "font-size:11px;color:#888;margin-bottom:2px;",
            {children}
        }
    }
}
```

- [ ] **步骤 2：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/serial_panel.rs
git commit -m "feat: 串口配置面板（端口/波特率/数据位/停止位/校验位/流控）"
```

---

## 任务 11：标签页导航与状态栏

**文件：**
- 修改：`src/components/tab_nav.rs`
- 修改：`src/components/status_bar.rs`

- [ ] **步骤 1：实现标签页导航**

```rust
// src/components/tab_nav.rs
use dioxus::prelude::*;
use crate::services::store::*;

#[component]
pub fn TabNav() -> Element {
    let state: AppState = use_context();
    let active = state.active_tab.read().clone();

    rsx! {
        div {
            style: "display:flex;gap:4px;padding:8px 0;",
            tab_button {
                label: "收发",
                is_active: active == ActiveTab::ReceiveSend,
                onclick: move |_| {
                    *state.active_tab.write() = ActiveTab::ReceiveSend;
                }
            }
            tab_button {
                label: "指令管理",
                is_active: active == ActiveTab::CommandManager,
                onclick: move |_| {
                    *state.active_tab.write() = ActiveTab::CommandManager;
                }
            }
        }
    }
}

#[component]
fn tab_button(label: String, is_active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let bg = if is_active {
        "background:linear-gradient(135deg,rgba(102,126,234,0.3),rgba(118,75,162,0.3));border-color:rgba(102,126,234,0.5);"
    } else {
        "background:rgba(255,255,255,0.05);border-color:rgba(255,255,255,0.1);"
    };

    rsx! {
        button {
            class: "btn",
            style: "padding:6px 20px;font-size:13px;border:1px solid;border-radius:8px;{bg}",
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}
```

- [ ] **步骤 2：实现状态栏**

```rust
// src/components/status_bar.rs
use dioxus::prelude::*;
use crate::services::store::AppState;

#[component]
pub fn StatusBar() -> Element {
    let state: AppState = use_context();
    let is_connected = *state.is_connected.read();
    let bytes_rx = *state.bytes_received.read();
    let bytes_tx = *state.bytes_sent.read();

    let status_text = if is_connected { "已连接" } else { "未连接" };
    let status_color = if is_connected { "#4ade80" } else { "#f87171" };

    rsx! {
        div {
            style: "display:flex;justify-content:space-between;padding:6px 16px;background:rgba(0,0,0,0.25);font-size:11px;color:#888;",
            div {
                style: "display:flex;align-items:center;gap:6px;",
                span {
                    style: "width:8px;height:8px;border-radius:50%;background:{status_color};display:inline-block;",
                }
                span { "{status_text}" }
            }
            div {
                style: "display:flex;gap:16px;",
                span { "接收: {bytes_rx} 字节" }
                span { "发送: {bytes_tx} 字节" }
            }
        }
    }
}
```

- [ ] **步骤 3：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 4：Commit**

```bash
git add src/components/tab_nav.rs src/components/status_bar.rs
git commit -m "feat: 标签页导航与底部状态栏"
```

---

## 任务 12：收发标签页

**文件：**
- 修改：`src/components/receive_send_tab.rs`

- [ ] **步骤 1：实现收发标签页**

```rust
// src/components/receive_send_tab.rs
use dioxus::prelude::*;
use crate::services::store::*;
use crate::services::api;
use serde::Deserialize;

#[component]
pub fn ReceiveSendTab() -> Element {
    let state: AppState = use_context();
    let send_text = use_signal(String::new);

    // 监听串口数据事件
    use_effect(move || {
        api::tauri_listen("serial-data", move |event| {
            if let Ok(payload) = serde_wasm_bindgen::from_value::<ReceivedMessage>(event) {
                let len = payload.raw_bytes.len() as u64;
                state.received_messages.write().push(payload);
                *state.bytes_received.write() += len;
            }
        });
    });

    let send = move |_| {
        let content = (*send_text.read()).clone();
        let encoding = (*state.send_encoding.read()).clone();
        let state_clone = state.clone();
        spawn(async move {
            #[derive(serde::Serialize)]
            struct Args {
                request: SendRequest,
            }
            #[derive(serde::Serialize)]
            struct SendRequest {
                content: String,
                encoding: Encoding,
            }
            let result: Result<usize, String> = api::tauri_invoke(
                "send_data",
                &Args {
                    request: SendRequest { content, encoding },
                },
            ).await;
            if let Ok(n) = result {
                *state_clone.bytes_sent.write() += n as u64;
            }
        });
    };

    let clear = move |_| {
        state.received_messages.write().clear();
        *state.bytes_received.write() = 0;
    };

    let export = move |_| {
        let entries: Vec<LogEntry> = state
            .received_messages
            .read()
            .iter()
            .map(|m| LogEntry {
                timestamp: m.timestamp.clone(),
                direction: "接收".to_string(),
                encoding: format!("{:?}", m.encoding),
                data: m.data.clone(),
            })
            .collect();
        spawn(async move {
            let path: Result<Option<String>, String> =
                api::tauri_invoke_no_args("save_dialog").await;
            if let Ok(Some(p)) = path {
                let format = if p.ends_with(".csv") { "csv" } else { "txt" };
                #[derive(serde::Serialize)]
                struct Args {
                    entries: Vec<LogEntry>,
                    path: String,
                    format: String,
                }
                let _: Result<(), String> = api::tauri_invoke(
                    "export_data",
                    &Args { entries, path: p, format: format.to_string() },
                ).await;
            }
        });
    };

    let messages = state.received_messages.read().clone();
    let encoding = state.send_encoding.read().clone();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;flex:1;overflow:hidden;gap:8px;",

            // 接收区
            div {
                class: "glass-card",
                style: "flex:1;display:flex;flex-direction:column;overflow:hidden;padding:10px;",
                div {
                    style: "flex:1;overflow-y:auto;font-family:'Cascadia Code','Fira Code',monospace;font-size:12px;line-height:1.6;",
                    for msg in messages.iter() {
                        div {
                            style: "padding:2px 0;border-bottom:1px solid rgba(255,255,255,0.03);",
                            span { style: "color:#667eea;", "[{msg.timestamp}]" }
                            span { style: "color:#888;margin:0 6px;", "收到:" }
                            span { "{msg.data}" }
                        }
                    }
                }
                div {
                    style: "display:flex;gap:6px;margin-top:8px;",
                    button { class: "btn btn-secondary", onclick: clear, "清空" }
                    button { class: "btn btn-secondary", onclick: export, "导出 ▼" }
                }
            }

            // 发送区
            div {
                class: "glass-card",
                style: "padding:10px;display:flex;flex-direction:column;gap:8px;",
                textarea {
                    class: "glass-input",
                    style: "width:100%;height:60px;resize:none;font-family:'Cascadia Code','Fira Code',monospace;font-size:12px;",
                    placeholder: "输入要发送的数据...",
                    value: "{send_text}",
                    oninput: move |e| {
                        *send_text.write() = e.value();
                    },
                }
                div {
                    style: "display:flex;align-items:center;gap:8px;",
                    select {
                        class: "glass-select",
                        value: format!("{:?}", encoding).to_lowercase(),
                        onchange: move |e| {
                            *state.send_encoding.write() = match e.value().as_str() {
                                "hex" => Encoding::Hex,
                                "utf8" => Encoding::Utf8,
                                "gbk" => Encoding::Gbk,
                                _ => Encoding::Ascii,
                            };
                        },
                        option { value: "ascii", "ASCII" }
                        option { value: "hex", "HEX" }
                        option { value: "utf8", "UTF-8" }
                        option { value: "gbk", "GBK" }
                    }
                    div { style: "flex:1;" }
                    label {
                        style: "display:flex;align-items:center;gap:4px;font-size:11px;color:#888;",
                        input {
                            r#type: "number",
                            class: "glass-input",
                            style: "width:70px;",
                            value: "{*state.loop_interval_ms.read()}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse() {
                                    *state.loop_interval_ms.write() = v;
                                }
                            },
                        }
                        "ms"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: send,
                        disabled: !*state.is_connected.read(),
                        "发送"
                    }
                    button {
                        class: if *state.loop_send.read() { "btn btn-danger" } else { "btn btn-secondary" },
                        onclick: {
                            let state_clone = state.clone();
                            move |_| {
                                let is_loop = *state_clone.loop_send.read();
                                *state_clone.loop_send.write() = !is_loop;
                                if !is_loop {
                                    let content = (*send_text.read()).clone();
                                    let encoding = (*state_clone.send_encoding.read()).clone();
                                    let interval = *state_clone.loop_interval_ms.read();
                                    let st = state_clone.clone();
                                    spawn(async move {
                                        loop {
                                            if !*st.loop_send.read() { break; }
                                            #[derive(serde::Serialize)]
                                            struct Args { request: SendRequest }
                                            #[derive(serde::Serialize)]
                                            struct SendRequest { content: String, encoding: Encoding }
                                            let _: Result<usize, String> = api::tauri_invoke(
                                                "send_data",
                                                &Args { request: SendRequest {
                                                    content: content.clone(),
                                                    encoding: encoding.clone(),
                                                }},
                                            ).await;
                                            gloo_timers::future::sleep(
                                                std::time::Duration::from_millis(interval)
                                            ).await;
                                        }
                                    });
                                }
                            }
                        },
                        disabled: !*state.is_connected.read(),
                        if *state.loop_send.read() { "停止" } else { "循环" }
                    }
                }
            }
        }
    }
}
```

- [ ] **步骤 2：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/receive_send_tab.rs
git commit -m "feat: 收发标签页（接收显示/发送/导出/编码选择）"
```

---

## 任务 13：指令管理标签页

**文件：**
- 修改：`src/components/command_manager_tab.rs`

- [ ] **步骤 1：实现指令管理组件**

```rust
// src/components/command_manager_tab.rs
use dioxus::prelude::*;
use crate::services::store::*;

#[component]
pub fn CommandManagerTab() -> Element {
    let state: AppState = use_context();
    let editing = use_signal(|| false);
    let edit_name = use_signal(String::new);
    let edit_content = use_signal(String::new);
    let edit_encoding = use_signal(|| Encoding::Ascii);
    let edit_id = use_signal(|| Option::<String>::None);

    // 启动时从 store 加载预设指令
    use_effect(move || {
        spawn(async move {
            #[derive(serde::Serialize)]
            struct GetArgs { key: String }
            let result: Result<Option<Vec<PresetCommand>>, String> = api::tauri_invoke(
                "plugin:store|get",
                &GetArgs { key: "preset_commands".to_string() },
            ).await;
            if let Ok(Some(cmds)) = result {
                *state.preset_commands.write() = cmds;
            }
        });
    });

    // 预设指令变化时保存到 store
    let commands_for_save = state.preset_commands.read().clone();
    use_effect(use_reactive!(|commands_for_save| {
        spawn(async move {
            #[derive(serde::Serialize)]
            struct SetArgs { key: String, value: serde_json::Value }
            let value = serde_json::to_value(&commands_for_save).unwrap_or_default();
            let _: Result<(), String> = api::tauri_invoke(
                "plugin:store|set",
                &SetArgs { key: "preset_commands".to_string(), value },
            ).await;
            let _: Result<(), String> = api::tauri_invoke_no_args("plugin:store|save").await;
        });
    }));

    let start_add = move |_| {
        *edit_name.write() = String::new();
        *edit_content.write() = String::new();
        *edit_encoding.write() = Encoding::Ascii;
        *edit_id.write() = None;
        *editing.write() = true;
    };

    let start_edit = move |cmd: PresetCommand| {
        *edit_name.write() = cmd.name;
        *edit_content.write() = cmd.content;
        *edit_encoding.write() = cmd.encoding;
        *edit_id.write() = Some(cmd.id);
        *editing.write() = true;
    };

    let save = move |_| {
        let name = (*edit_name.read()).clone();
        let content = (*edit_content.read()).clone();
        let encoding = (*edit_encoding.read()).clone();
        let id = (*edit_id.read()).clone();

        if name.is_empty() || content.is_empty() {
            return;
        }

        let mut commands = state.preset_commands.write();
        match id {
            Some(existing_id) => {
                if let Some(cmd) = commands.iter_mut().find(|c| c.id == existing_id) {
                    cmd.name = name;
                    cmd.content = content;
                    cmd.encoding = encoding;
                }
            }
            None => {
                commands.push(PresetCommand {
                    id: uuid_simple(),
                    name,
                    content,
                    encoding,
                });
            }
        }
        *editing.write() = false;
    };

    let delete = move |id: String| {
        state.preset_commands.write().retain(|c| c.id != id);
    };

    let send_preset = move |cmd: PresetCommand| {
        let state_clone = state.clone();
        spawn(async move {
            #[derive(serde::Serialize)]
            struct Args {
                request: SendRequest,
            }
            #[derive(serde::Serialize)]
            struct SendRequest {
                content: String,
                encoding: Encoding,
            }
            let result: Result<usize, String> = crate::services::api::tauri_invoke(
                "send_data",
                &Args {
                    request: SendRequest {
                        content: cmd.content,
                        encoding: cmd.encoding,
                    },
                },
            ).await;
            if let Ok(n) = result {
                *state_clone.bytes_sent.write() += n as u64;
            }
        });
    };

    let commands = state.preset_commands.read().clone();
    let is_editing = *editing.read();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;flex:1;overflow:hidden;gap:8px;",

            // 指令列表
            div {
                class: "glass-card",
                style: "flex:1;overflow-y:auto;padding:10px;",
                if commands.is_empty() {
                    div {
                        style: "text-align:center;color:#666;padding:40px 0;font-size:13px;",
                        "暂无预设指令，点击下方按钮添加"
                    }
                }
                for cmd in commands.iter() {
                    div {
                        key: "{cmd.id}",
                        style: "display:flex;align-items:center;gap:8px;padding:8px;border-bottom:1px solid rgba(255,255,255,0.05);",
                        div {
                            style: "flex:1;",
                            div { style: "font-size:13px;font-weight:500;", "{cmd.name}" }
                            div { style: "font-size:11px;color:#888;font-family:monospace;", "{cmd.content}" }
                        }
                        span {
                            style: "font-size:10px;padding:2px 6px;background:rgba(102,126,234,0.2);border-radius:4px;color:#667eea;",
                            "{format_encoding(&cmd.encoding)}"
                        }
                        button {
                            class: "btn btn-primary",
                            style: "padding:3px 8px;font-size:11px;",
                            disabled: !*state.is_connected.read(),
                            onclick: {
                                let c = cmd.clone();
                                move |_| send_preset(c.clone())
                            },
                            "发送"
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "padding:3px 8px;font-size:11px;",
                            onclick: {
                                let c = cmd.clone();
                                move |_| start_edit(c.clone())
                            },
                            "编辑"
                        }
                        button {
                            class: "btn btn-danger",
                            style: "padding:3px 8px;font-size:11px;",
                            onclick: {
                                let id = cmd.id.clone();
                                move |_| delete(id.clone())
                            },
                            "删除"
                        }
                    }
                }
            }

            // 新增按钮
            if !is_editing {
                button {
                    class: "btn btn-primary",
                    style: "width:100%;padding:10px;",
                    onclick: start_add,
                    "+ 新增指令"
                }
            }

            // 编辑/新增表单
            if is_editing {
                div {
                    class: "glass-card",
                    style: "padding:12px;display:flex;flex-direction:column;gap:8px;",
                    div { style: "font-size:12px;color:#a0a8d0;font-weight:600;",
                        if edit_id.read().is_some() { "编辑指令" } else { "新增指令" }
                    }
                    input {
                        class: "glass-input",
                        placeholder: "指令名称",
                        value: "{edit_name}",
                        oninput: move |e| { *edit_name.write() = e.value(); },
                    }
                    input {
                        class: "glass-input",
                        placeholder: "指令内容",
                        value: "{edit_content}",
                        oninput: move |e| { *edit_content.write() = e.value(); },
                    }
                    select {
                        class: "glass-select",
                        value: format!("{:?}", *edit_encoding.read()).to_lowercase(),
                        onchange: move |e| {
                            *edit_encoding.write() = match e.value().as_str() {
                                "hex" => Encoding::Hex,
                                "utf8" => Encoding::Utf8,
                                "gbk" => Encoding::Gbk,
                                _ => Encoding::Ascii,
                            };
                        },
                        option { value: "ascii", "ASCII" }
                        option { value: "hex", "HEX" }
                        option { value: "utf8", "UTF-8" }
                        option { value: "gbk", "GBK" }
                    }
                    div {
                        style: "display:flex;gap:6px;",
                        button { class: "btn btn-primary", onclick: save, "保存" }
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| { *editing.write() = false; },
                            "取消"
                        }
                    }
                }
            }
        }
    }
}

fn format_encoding(enc: &Encoding) -> &'static str {
    match enc {
        Encoding::Ascii => "ASCII",
        Encoding::Hex => "HEX",
        Encoding::Utf8 => "UTF-8",
        Encoding::Gbk => "GBK",
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{:x}", t)
}
```

- [ ] **步骤 2：验证编译通过**

```bash
cargo check -p serial-debugger-ui
```

- [ ] **步骤 3：Commit**

```bash
git add src/components/command_manager_tab.rs
git commit -m "feat: 指令管理标签页（增删改查/发送预设指令）"
```

---

## 任务 14：集成与验证

**文件：**
- 修改：`src-tauri/src/main.rs`（最终检查）
- 修改：`src/app.rs`（最终检查）

- [ ] **步骤 1：完整编译验证**

```bash
cargo tauri build --debug
```

预期：编译成功，生成可执行文件。

- [ ] **步骤 2：运行应用进行手动测试**

```bash
cargo tauri dev
```

验证清单：
- [ ] 窗口显示毛玻璃/透明效果
- [ ] 标题栏可拖拽移动窗口
- [ ] 最小化/关闭按钮正常工作
- [ ] 串口列表正确显示
- [ ] 串口参数可配置
- [ ] 打开/关闭串口正常
- [ ] 接收数据带时间戳显示
- [ ] 发送数据（ASCII/HEX/UTF-8/GBK）正常
- [ ] 标签页切换正常
- [ ] 指令管理增删改查正常
- [ ] 导出 TXT/CSV 文件正常
- [ ] 状态栏显示连接状态和字节统计

- [ ] **步骤 3：修复发现的问题**

根据手动测试结果修复任何编译错误或运行时 bug。

- [ ] **步骤 4：最终 Commit**

```bash
git add .
git commit -m "feat: 串口调试助手 v0.1.0 完成"
```

---

## 依赖版本汇总

| Crate | 版本 | 用途 |
|-------|------|------|
| tauri | 2 | 应用框架 |
| tauri-plugin-store | 2 | 持久化存储 |
| tauri-plugin-dialog | 2 | 文件对话框 |
| tauri-plugin-opener | 2 | 打开外部链接 |
| serialport | 4.9 | 串口通信 |
| window-vibrancy | 0.7 | 毛玻璃效果 |
| encoding_rs | 0.8 | GBK 编码支持 |
| serde | 1 | 序列化 |
| serde_json | 1 | JSON 处理 |
| chrono | 0.4 | 时间戳 |
| uuid | 1 | UUID 生成 |
| dioxus | 0.7 | 前端框架 |
| wasm-bindgen | 0.2 | WASM 绑定 |
| wasm-bindgen-futures | 0.4 | WASM 异步 |
| serde-wasm-bindgen | 0.6 | WASM 序列化 |
| js-sys | 0.3 | JS 类型 |
| web-sys | 0.3 | Web API |
