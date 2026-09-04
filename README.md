# 串口调试助手

一个纯 Rust / GPUI 的跨平台串口调试工具。UI 使用 GPUI 原生渲染，搭配 `gpui-component`，采用现代化浅色扁平卡片设计，不依赖 WebView、Dioxus 或 Tauri。

## 功能

- 枚举并连接串口，支持波特率、数据位、停止位、校验位与流控配置
- ASCII、HEX、UTF-8、GBK 编码收发，支持 CR / LF / CRLF 行尾
- 实时接收日志、发送回显、HEX 显示、自动滚动与统计字节数
- 循环发送（可配置毫秒间隔，使用代际令牌安全停止）
- 命令预设的保存、加载与 JSON 持久化
- TXT / CSV 导出（CSV 使用标准 quoting，并带 UTF-8 BOM，兼容 Excel）
- 串口断开和读写失败会同步反映到 UI 状态

## 技术栈

| 层 | 技术 |
| --- | --- |
| 原生窗口与渲染 | [GPUI 0.2.2](https://github.com/zed-industries/zed/tree/main/crates/gpui) |
| 原生控件 | [gpui-component 0.5.1](https://github.com/longbridge/gpui-component) |
| 串口驱动 | [serialport 4.x](https://crates.io/crates/serialport) |
| 编码转换 | [encoding_rs](https://crates.io/crates/encoding_rs) |
| 配置与导出 | serde、serde_json、csv、dirs、rfd（系统保存对话框） |

## 项目结构

```
src/
├── main.rs                 # GPUI 应用入口
├── app.rs                  # 根视图、交互和生命周期
├── state.rs                # 可测试的领域状态与日志规则
├── serial/
│   ├── manager.rs           # 串口打开、接收线程、限时发送、清理
│   ├── encoding.rs          # 编解码与增量 UTF-8 解码
│   └── protocol.rs          # 串口事件模型
├── export/
│   ├── exporter.rs          # TXT / CSV 导出
│   └── persistence.rs       # 原子化配置与预设持久化
└── ui/
    ├── theme.rs             # 浅色扁平主题色
    └── components.rs        # 通用卡片和标签
```

## 环境要求

- Rust stable（edition 2021）
- Windows 使用 MSVC 工具链；Linux / macOS 安装 GPUI 所需系统图形依赖

## 开发与构建

```bash
cargo run
cargo check --locked
cargo test --all-targets --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
```

应用配置保存在系统配置目录的 `serial-debugger/settings.json`；点击导出时可在系统保存对话框中选择目标路径。

## 分支

当前重构在 `dev` 分支进行。仓库已移除旧的 Dioxus/Tauri 工程、WebView 资源和 CI workflow，后续发布由本地构建流程负责。
