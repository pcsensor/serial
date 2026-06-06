# 串口调试助手

基于 Tauri 2 + Dioxus 0.7 的跨平台串口调试工具，使用 Rust 全栈开发。

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue)
![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB)
![Dioxus](https://img.shields.io/badge/Dioxus-0.7-FFAA00)
![Rust](https://img.shields.io/badge/Rust-2021-orange)

## 功能特性

- 串口枚举、打开 / 关闭、状态监测
- 多编码收发：ASCII、HEX、UTF-8、GBK
- 实时接收显示，时间戳 + 原始字节
- 命令管理：保存常用指令、单次发送、循环发送
- 数据导出：TXT、CSV
- macOS / Windows / Linux 原生窗口毛玻璃效果（Vibrancy / Mica / Acrylic / Blur）
- 自定义无边框窗口与主题

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面外壳 | [Tauri 2](https://tauri.app/) |
| 前端 UI | [Dioxus 0.7](https://dioxuslabs.com/)（WebView 渲染） |
| 串口驱动 | [serialport](https://crates.io/crates/serialport) 4.x |
| 编码转换 | [encoding_rs](https://crates.io/crates/encoding_rs)（GBK 支持） |
| 窗口效果 | [window-vibrancy](https://crates.io/crates/window-vibrancy) |

## 项目结构

```
.
├── src/                   # 前端 (Dioxus, 编译为 wasm)
│   ├── app.rs             # 根组件
│   ├── components/        # UI 组件
│   │   ├── serial_panel.rs       # 串口配置面板
│   │   ├── receive_send_tab.rs   # 接收 / 发送 Tab
│   │   ├── command_manager_tab.rs# 命令管理 Tab
│   │   ├── tab_nav.rs / title_bar.rs / status_bar.rs
│   ├── services/
│   │   ├── api.rs         # Tauri invoke 封装
│   │   └── store.rs       # 全局状态
│   └── assets/main.css
├── src-tauri/             # 后端 (Tauri / Rust)
│   ├── src/
│   │   ├── main.rs / lib.rs
│   │   ├── serial/
│   │   │   ├── manager.rs    # 串口管理 (跨平台)
│   │   │   ├── commands.rs   # Tauri 命令
│   │   │   └── encoding.rs   # 编码转换
│   │   └── export/exporter.rs # 日志导出
│   ├── Cargo.toml
│   └── tauri.conf.json
└── Cargo.toml             # workspace 根
```

## 环境要求

- Rust **stable** (edition 2021)
- [Tauri CLI](https://tauri.app/start/prerequisites/)：`cargo install tauri-cli --version "^2.0"`
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started/)：`cargo install dioxus-cli --version "^0.7"`
- 各平台原生依赖：
  - **macOS**：Xcode Command Line Tools
  - **Windows**：Microsoft Edge WebView2（Win10 1809+ 自带）+ MSVC 工具链
  - **Linux**：`libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev`、`libssl-dev`、`pkg-config`

## 开发

```bash
# 启动开发服务器（自动热重载）
cargo tauri dev
```

`cargo tauri dev` 会先通过 `dx serve` 启动 Dioxus 前端（`http://localhost:8080`），再启动 Tauri 主进程。

## 构建发行版

```bash
cargo tauri build
```

产物输出位置：

- macOS: `src-tauri/target/release/bundle/{dmg,macos}/`
- Windows: `src-tauri/target/release/bundle/{msi,nsis}/`
- Linux: `src-tauri/target/release/bundle/{deb,rpm,appimage}/`

## 测试

```bash
cd src-tauri
cargo test
```

后端覆盖了串口管理器关闭流程和编码模块（ASCII / HEX / UTF-8 / GBK）。

## 跨平台说明

串口底层使用 cfg 切换原生类型：

| 平台 | 原生句柄 | 备注 |
| --- | --- | --- |
| macOS | `serialport::TTYPort` | 接收线程额外设置 `O_NONBLOCK`，规避部分驱动 `poll()` 假阳性导致 `read()` 永久阻塞 |
| Linux | `serialport::TTYPort` | 默认阻塞 + 100ms 超时 |
| Windows | `serialport::COMPort` | 默认阻塞 + 100ms 超时 |

macOS 专属逻辑严格隔离在 `#[cfg(target_os = "macos")]` 块内，不影响其他平台。

## 串口接收问题排障

如果遇到串口连接后无响应、断开卡死等问题，参考 commit 历史：

- `fix: 串口连接卡死` — `O_NONBLOCK` + `WouldBlock` 兜底
- `fix: 保存指令卡死` — 配置写盘流程

## 许可

待定。

## 致谢

- [Tauri](https://github.com/tauri-apps/tauri)
- [Dioxus](https://github.com/DioxusLabs/dioxus)
- [serialport-rs](https://github.com/serialport/serialport-rs)
