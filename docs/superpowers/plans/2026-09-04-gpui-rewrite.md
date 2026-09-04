# GPUI 原生串口调试助手重构实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 `dev` 分支把现有 Dioxus/Tauri 串口调试器重构为纯 GPUI 原生桌面应用，保留全部功能并修复已发现的可靠性问题，同时删除 CI。

**架构：** 将串口、编码、组帧、导出和持久化抽为不依赖 UI 的 Rust 领域层；GPUI Entity 持有应用状态，通过 `async-channel` 接收串口线程事件。UI 使用 GPUI 0.2.2 和 gpui-component 0.5.1 的原生控件与统一浅色主题。

**技术栈：** Rust stable、GPUI 0.2.2、gpui-component 0.5.1、serialport 4.9、encoding_rs 0.8、async-channel 2、chrono 0.4、serde/serde_json。

---

## 文件结构锁定

将创建或修改：

- `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`：单包 GPUI 工程和固定工具链。
- `src/main.rs`、`src/app.rs`、`src/state.rs`：GPUI 入口、窗口根视图、应用状态。
- `src/serial/{mod.rs,encoding.rs,manager.rs,protocol.rs}`：串口和原始字节组帧。
- `src/export/{mod.rs,exporter.rs}`：TXT/CSV 导出与路径校验。
- `src/ui/{mod.rs,theme.rs,components.rs,connection_panel.rs,receive_send.rs,command_manager.rs}`：全部 GPUI UI。
- `README.md`、`.gitignore`：重写运行说明并忽略视觉伴侣产物。

将删除：

- `src-tauri/`、`Dioxus.toml`、旧 `src/app.rs` 及旧 `src/components/`、`src/services/`。
- `.github/workflows/build-desktop.yml`。

---

### 任务 1：建立 GPUI 编译基线

**文件：** `Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`src/main.rs`、`src/app.rs`。

- [ ] **步骤 1：替换根 manifest 为单包 GPUI 依赖**

将 package 名改为 `serial-debugger`，移除 Dioxus/WASM/web-sys/gloo 依赖，添加：

```toml
[package]
name = "serial-debugger"
version = "0.1.12"
edition = "2021"

[dependencies]
gpui = "0.2.2"
gpui-component = "0.5.1"
async-channel = "2"
chrono = { version = "0.4", features = ["serde"] }
encoding_rs = "0.8"
dirs = "6"
csv = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serialport = "4.9"
uuid = { version = "1", features = ["v4", "serde"] }
```

- [ ] **步骤 2：写最小 GPUI 入口并验证编译**

`src/main.rs` 使用 `Application::new().run`，初始化 `gpui_component::init(cx)`，打开 1100×750 窗口并挂载 `AppView`。`AppView` 暂时只渲染标题文本。

运行：`cargo check --locked`。

预期：命令成功，确认 Windows 原生 GPUI 依赖可解析。

- [ ] **步骤 3：固定工具链并提交基线**

创建 `rust-toolchain.toml`，声明 `channel = "stable"`、`components = ["rustfmt", "clippy"]`、`targets = ["wasm32-unknown-unknown"]` 不适用于纯 GPUI，故只声明当前桌面构建所需的 stable 组件；提交 `feat: 建立 GPUI 原生应用基线`。

---

### 任务 2：迁移领域模型与编码组帧

**文件：** `src/state.rs`、`src/serial/{mod.rs,encoding.rs,protocol.rs}`。

- [ ] **步骤 1：迁移并补充编码测试**

迁移 ASCII/HEX/UTF-8/GBK 编解码测试，并新增：ASCII 非 ASCII 输入拒绝、GBK/UTF-8 非法序列保留原始字节、HEX 空白和奇数长度错误。

- [ ] **步骤 2：实现原始字节组帧测试并确认先失败**

添加测试：CRLF 跨 chunk 只产生一行、UTF-8 三字节字符跨 chunk、无行尾 preview、flush pending、每行保留原始 bytes。先运行 `cargo test serial::protocol`，确认新行为在实现前失败。

- [ ] **步骤 3：实现 `ReceiveLineBuffer` 和 `SerialEvent`**

定义 `SerialEvent::{Data, Error, Disconnected}`；缓冲区按 `\r`/`\n` 字节拆分，输出 `(timestamp, raw_bytes)`，并提供 `pending_bytes()`、`flush()`、`clear()`。完整帧再调用编码模块解码。

- [ ] **步骤 4：实现统一日志状态函数**

`AppState` 使用普通 Rust struct，包含连接状态、配置、日志、统计、预设、错误和循环 generation；`push_message` 对所有发送/接收路径实施 10,000 条上限。预设 ID 使用 UUID。

- [ ] **步骤 5：运行领域层测试并提交**

运行：`cargo test serial state`。

预期：编码、组帧、日志上限和 ID 测试全部通过；提交 `feat: 迁移串口领域模型和原始字节组帧`。

---

### 任务 3：重构 SerialManager 可靠性

**文件：** `src/serial/manager.rs`、`src/serial/protocol.rs`。

- [ ] **步骤 1：添加失败回滚、断线和写入 deadline 测试**

覆盖 reader 初始化失败时 manager 仍关闭、读错误产生 `Error + Disconnected`、发送 `WouldBlock` 在 deadline 后返回、close 能 join 接收线程。

- [ ] **步骤 2：实现纯 Rust 事件通道**

`SerialManager` 不再引用 `tauri::AppHandle`；`open` 校验配置后创建原生端口、reader、`async_channel::Sender<SerialEvent>`，全部成功后才写入字段。接收线程发送原始 bytes 和时间戳。

- [ ] **步骤 3：实现有限重试和 Drop 清理**

发送循环使用 `Instant` deadline 和 stop flag；`WouldBlock` 睡眠 1ms 后重试，超时返回中文错误。`close` 与 `Drop` 复用同一停止/join/释放路径。

- [ ] **步骤 4：运行串口测试并提交**

运行：`cargo test serial::manager --all-targets`。

预期：Windows 可运行的测试全部通过；提交 `fix: 加固串口生命周期和读写异常`。

---

### 任务 4：迁移导出与配置持久化

**文件：** `src/export/{mod.rs,exporter.rs}`、`src/state.rs`。

- [ ] **步骤 1：补 CSV/TXT 边界测试**

测试逗号、双引号、换行、公式前缀、空扩展名和格式不匹配路径；TXT 必须包含编码列信息。

- [ ] **步骤 2：实现缓冲导出**

使用 `BufWriter` 写 TXT/CSV；CSV 所有字段统一 RFC 4180 转义并保留 BOM；路径无扩展名时追加目标扩展名，其他扩展名返回错误。

- [ ] **步骤 3：实现原子 JSON 配置存储**

在 `dirs::config_dir()/serial-debugger/` 保存 `preset-commands.json` 和 `port-config.json`。写入临时文件后 rename；加载失败保留默认值并将错误传到 UI。

- [ ] **步骤 4：运行导出/持久化测试并提交**

运行：`cargo test export state`。

预期：全部通过；提交 `feat: 迁移日志导出和配置持久化`。

---

### 任务 5：实现 GPUI 主题和通用控件

**文件：** `src/ui/{mod.rs,theme.rs,components.rs}`。

- [ ] **步骤 1：定义浅色 token**

集中定义背景、卡片、边框、主色、错误色、警告色、文本色、间距、圆角和字号；禁止组件内重复硬编码主题值。

- [ ] **步骤 2：实现通用组件**

封装 `card`、`status_badge`、`field_label`、`action_button`、`empty_state`、`error_notice`，使用 gpui-component 的基础控件并保证禁用/焦点/错误状态。

- [ ] **步骤 3：运行 UI 编译验证并提交**

运行：`cargo check --locked`。

预期：GPUI 组件类型统一且无重复 GPUI 依赖；提交 `feat: 添加 GPUI 浅色扁平主题`。

---

### 任务 6：实现连接面板和窗口生命周期

**文件：** `src/app.rs`、`src/ui/connection_panel.rs`。

- [ ] **步骤 1：添加连接状态转移测试**

覆盖未连接、连接中、已连接、失败回滚、断线和重新连接；发送按钮在未连接/连接中必须禁用。

- [ ] **步骤 2：实现 GPUI 根视图**

`AppView` 持有 `AppState`、`SerialManager`、事件 receiver 和后台 task；窗口关闭前停止循环、关闭串口并等待资源退出。

- [ ] **步骤 3：实现连接配置 UI**

保留端口刷新、端口选择、波特率、数据位、停止位、校验、流控和打开/关闭按钮；错误写入状态区，不能只写控制台。

- [ ] **步骤 4：运行 GPUI 启动验证并提交**

运行：`cargo run`，确认 Windows 原生窗口能打开、字段可交互、连接状态可更新；提交 `feat: 实现 GPUI 连接配置面板`。

---

### 任务 7：实现收发页和循环发送

**文件：** `src/ui/receive_send.rs`、`src/app.rs`、`src/state.rs`。

- [ ] **步骤 1：添加收发和循环 generation 测试**

覆盖发送编码/行尾、发送日志 raw bytes、接收 preview、断开 flush、自动滚动开关、HEX 显示、循环停止后旧任务不能继续发送。

- [ ] **步骤 2：实现虚拟日志列表**

用 GPUI/组件库的虚拟列表渲染最多 10,000 条消息；RX/TX 使用不同颜色；HEX 模式优先显示 raw bytes；日志清空时同时清除 preview 和计数。

- [ ] **步骤 3：实现发送和循环发送**

发送按钮调用 `SerialManager::send`；循环任务持有 generation，动态读取间隔，连接断开/发送错误/停止操作立即使 generation 失效。

- [ ] **步骤 4：实现导出对话框和状态提示**

保留 TXT/CSV 选择、保存路径、导出错误和成功提示；导出前 flush pending receive。

- [ ] **步骤 5：运行收发测试并提交**

运行：`cargo test receive_send state`，再运行 `cargo run` 手测收发页；提交 `feat: 实现 GPUI 收发和循环发送`。

---

### 任务 8：实现预设指令管理页

**文件：** `src/ui/command_manager.rs`、`src/state.rs`。

- [ ] **步骤 1：添加预设 CRUD 和保存队列测试**

覆盖新增、编辑、删除、UUID 唯一、非法空字段、保存失败提示和快速连续保存不被旧快照覆盖。

- [ ] **步骤 2：实现列表和编辑表单**

保留预设名称、内容、编码、发送、编辑、删除、新增、保存、取消和行尾选择；未连接时发送禁用。

- [ ] **步骤 3：实现串行持久化任务**

保存请求按版本号顺序执行；失败保留内存值并显示重试入口；首次加载失败允许重新加载。

- [ ] **步骤 4：运行预设测试并提交**

运行：`cargo test command_manager state`；提交 `feat: 实现 GPUI 预设指令管理`。

---

### 任务 9：移除旧框架与 CI、更新文档

**文件：** 删除 `src-tauri/`、`Dioxus.toml`、旧 Dioxus `src/components/` 与 `src/services/`；删除 `.github/workflows/build-desktop.yml`；修改 `README.md`。

- [ ] **步骤 1：确认新入口不再依赖旧框架**

运行：`rg -n "tauri|dioxus|wasm-bindgen|web_sys|__TAURI__|\.github/workflows" Cargo.toml src README.md .github`。

预期：只剩 README 的迁移说明，不存在编译依赖和旧入口。

- [ ] **步骤 2：删除旧入口、配置、资源和 CI**

删除范围必须与文件结构锁定一致；保留 `docs/superpowers/` 设计与计划文档。

- [ ] **步骤 3：重写 README**

说明 GPUI 依赖、Windows/macOS/Linux 系统要求、`cargo run`、`cargo build --release`、功能清单、测试命令和当前不提供 CI 的事实。

- [ ] **步骤 4：检查工作区并提交**

运行：`git diff --check`、`git status --short`。

预期：只有计划内文件；提交 `refactor: 移除 Dioxus Tauri 和 CI`。

---

### 任务 10：完整验证和最终审查

**文件：** 视验证结果修改对应实现/测试/README。

- [ ] **步骤 1：运行完整静态检查**

运行：`cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets --locked`。

- [ ] **步骤 2：运行 Windows 原生 smoke test**

运行：`cargo run`；验证窗口、连接配置、标签切换、输入、预设编辑、关闭清理。可用虚拟串口验证收发、断线、重连和背压。

- [ ] **步骤 3：核对需求清单和旧框架清除**

逐项核对规格验收标准；使用 `rg` 确认仓库无 Tauri/Dioxus/WASM/CI workflow 残留；确认 `dev` 分支干净且提交历史包含各阶段提交。

- [ ] **步骤 4：提交最终修复并报告证据**

只在命令输出支持结论后报告测试、构建和功能状态；若某平台无法在当前 Windows 环境验证，明确标记为未验证。
