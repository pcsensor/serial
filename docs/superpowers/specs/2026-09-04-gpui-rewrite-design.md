# GPUI 原生串口调试助手重构设计

## 状态

本设计已于 2026-09-04 获得用户批准，目标分支为 `dev`。

## 目标

在保留现有串口调试助手全部用户功能和交互语义的前提下，移除 Dioxus、Tauri WebView 和现有 UI，实现纯 GPUI 原生桌面应用；同时修复串口生命周期、字节流解码、日志增长、循环发送、持久化和导出等已发现问题，并删除现有 CI workflow。

## 非目标

- 不新增多串口同时连接、网络串口、脚本系统或插件系统。
- 不改变现有四种编码、四种行尾、预设指令、日志导出和串口参数的用户可见含义。
- 不保留第二套 Dioxus/Tauri UI 作为备用入口；重构完成后仓库只维护 GPUI 桌面入口。

## 技术决策

- 使用 `gpui = 0.2.2` 与 `gpui-component = 0.5.1`，二者来自 crates.io 且版本兼容。
- 使用 `gpui-component` 提供 Button、Input、Select、Checkbox、Tabs、Dialog、Scrollbar 等基础控件；应用自行定义颜色、间距、圆角和状态 token。
- 保留 `serialport`、`encoding_rs`、`chrono`、`serde`、`serde_json` 等领域依赖，移除 Tauri、Dioxus、WASM 和 WebView 依赖。
- 使用 `async-channel` 连接串口后台线程和 GPUI 事件循环；UI 更新只通过 GPUI Entity 上下文完成。
- 使用标准配置目录保存预设指令和上次串口配置；保存采用原子替换，避免进程中断留下半写文件。

## 目标目录结构

```text
.
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── src/
│   ├── main.rs                  # GPUI Application 入口
│   ├── app.rs                   # AppView、窗口生命周期、全局事件
│   ├── state.rs                 # 领域状态、日志模型、预设模型
│   ├── serial/
│   │   ├── mod.rs
│   │   ├── encoding.rs          # 编码与 HEX 解析
│   │   ├── manager.rs           # 串口打开、接收、发送、关闭
│   │   └── protocol.rs          # SerialEvent、原始字节组帧
│   ├── export/
│   │   ├── mod.rs
│   │   └── exporter.rs          # TXT/CSV 导出与路径处理
│   └── ui/
│       ├── mod.rs
│       ├── theme.rs             # 浅色扁平主题 token
│       ├── components.rs        # 通用按钮、字段、卡片、状态徽标
│       ├── connection_panel.rs  # 端口和串口参数
│       ├── receive_send.rs      # 收发、日志、循环发送
│       └── command_manager.rs   # 预设指令 CRUD
├── docs/superpowers/specs/
└── .github/                     # 不再包含 CI workflow
```

## UI 设计

界面采用产品化仪表盘风格：背景 `#F5F7FA`，白色卡片，浅灰边框，轻微阴影，圆角 6–10px，柔和绿色作为主色，红色表示错误，琥珀色表示警告。移除渐变、毛玻璃和 Web CSS。

窗口布局保持原有信息架构：顶部标题栏与窗口控制；左侧连接配置卡片；右侧“收发”和“指令管理”两个标签页；底部连接状态与 RX/TX 字节统计。日志使用等宽字体并采用虚拟列表，确保 10,000 条上限内的高频更新可用。

所有控件具备禁用态、焦点态和错误态。端口未连接时发送和循环发送按钮不可用；连接请求期间连接按钮不可用；错误必须显示在应用内状态区或通知中，不依赖开发者控制台。

## 状态和事件

`AppState` 由单个 GPUI Entity 持有，包含：

- `is_connected`、`connection_in_progress`、`port_config`
- `active_tab`
- `messages`、`receive_line_buffer`、`pending_receive_preview`
- `preset_commands`、`preset_commands_loading`、`preset_commands_saving`
- `bytes_received`、`bytes_sent`
- `send_encoding`、`send_line_ending`、`auto_scroll`、`loop_send`、`loop_interval_ms`、`hex_display`
- `status_message`、`last_error`

串口服务向 UI 发送以下事件：

```rust
enum SerialEvent {
    Data { bytes: Vec<u8>, timestamp: DateTime<Local> },
    Error(String),
    Disconnected { reason: String },
}
```

接收事件以原始字节为事实来源。按 CR/LF 字节组帧后，再用连接时选择的编码解码；这样 UTF-8 和 GBK 字符可以跨 read 边界正确解析，并且每条日志保留真实 `raw_bytes`。

没有行尾的数据会更新为“接收中”预览，不再静默消失；断开和导出前会 flush 尾部缓冲为正式日志。清空操作会同时清除正式日志和预览缓冲。

## 串口生命周期和可靠性

打开流程必须完成以下步骤后才提交连接状态：参数校验、原生端口打开、reader 克隆、接收通道建立、接收线程启动。任一步失败都关闭已打开的资源并向 UI 返回错误。

发送操作使用写入 deadline 和取消检查。`WouldBlock`、`Interrupted` 会有限重试；超过 deadline 返回错误，不得无限占用串口锁。关闭操作设置取消标志、等待接收线程退出、清空缓冲并释放端口。`SerialManager` 的 Drop 路径执行无错误传播版本的相同清理。

接收线程发生非超时读取错误时发送 `Error` 和 `Disconnected`，服务状态变为关闭，UI 同步变为“已断开”，用户可以直接重新连接。

循环发送只允许一个任务运行。每次启动产生递增 generation；停止、断开或发送失败都会使旧 generation 失效，旧任务醒来后不得继续发送。日志写入、普通发送、预设发送和循环发送统一经过 10,000 条上限函数。

## 编码和导出

- ASCII 发送拒绝非 ASCII 字符，并对接收的非 ASCII 字节使用明确的替代显示策略。
- HEX 忽略空白、要求偶数长度、输出大写并以空格分隔。
- UTF-8/GBK 在完整帧上解码；非法序列显示错误并保留原始字节。
- 发送日志保存实际发送字节，HEX 显示不再依赖重新编码字符串。
- TXT 包含时间、方向、编码和数据。
- CSV 的所有字段统一进行 RFC 4180 风格转义，并保留 UTF-8 BOM；导出使用 `BufWriter`。
- 导出格式与扩展名必须匹配；无扩展名时自动添加正确扩展名。

## 持久化

预设命令和上次串口配置保存在用户配置目录。预设新增、编辑和删除通过串行保存队列提交，保存失败时保留内存状态但显示错误并允许重试；不会被较旧的异步快照覆盖。预设 ID 使用 UUID。

## 工程迁移

- 删除 `.github/workflows/build-desktop.yml`，仓库不再运行 CI。
- 删除 `Dioxus.toml`、`src-tauri/` 以及现有 Web UI 入口和资源。
- 根 `Cargo.toml` 改为单一 GPUI package，锁定 GPUI 兼容版本。
- 添加 `rust-toolchain.toml`，声明 stable 工具链及 Windows、macOS、Linux 目标。
- README 改写为 GPUI 的安装、开发、运行、打包和平台依赖说明。

## 测试策略

- 保留并迁移现有编码、HEX、行尾、日志、预设和导出单元测试。
- 增加跨 chunk UTF-8/GBK、无换行预览、断开 flush、接收线程错误、打开回滚、发送 deadline、循环 generation、统一日志上限、UUID 唯一性和 CSV 特殊字段测试。
- 使用 Unix PTY 测试发送背压和关闭；Windows 至少验证 COM 构建和 UI 启动。
- GPUI UI 测试覆盖连接按钮状态、错误通知、标签页切换、输入与预设编辑的核心状态转移。

## 验收标准

1. `cargo test --all-targets --locked` 通过。
2. `cargo fmt --check` 和 `cargo clippy --all-targets -- -D warnings` 通过。
3. `cargo run` 在 Windows 打开 GPUI 原生窗口，无 WebView、Dioxus 或 Tauri 运行时。
4. 端口枚举、配置、连接、收发、循环、预设、导出、标题栏和状态栏逐项可用。
5. 设备拔出、重新连接、背压、跨 chunk 多字节编码均不会卡死、假连接或丢失原始字节。
6. 仓库中不存在 CI workflow，工作区仅包含 `dev` 分支上的预期变更。
