use crate::{
    export::{
        self,
        persistence::{self, SavedSettings},
    },
    serial::{encoding, manager::SerialManager, protocol::SerialEvent},
    state::{self, ActiveTab, AppState, Encoding, MessageDirection, PresetCommand, SendLineEnding},
};
use async_channel::Sender;
use gpui::{
    div, prelude::*, px, Context, ElementId, Entity, IntoElement, Render, SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::{Input, InputState},
    scroll::ScrollableElement,
    Disableable,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub struct AppView {
    state: AppState,
    manager: Arc<Mutex<SerialManager>>,
    events_tx: Sender<SerialEvent>,
    stream_decoder: crate::serial::encoding::StreamDecoder,
    available_ports: Vec<String>,
    port_input: Entity<InputState>,
    send_input: Entity<InputState>,
    preset_name_input: Entity<InputState>,
    preset_content_input: Entity<InputState>,
    loop_interval_input: Entity<InputState>,
    loop_generation: Arc<AtomicU64>,
    editing_preset: Option<usize>,
    last_export: Option<PathBuf>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = persistence::load();
        let state = AppState {
            config: settings.port_config.clone(),
            presets: settings.presets.clone(),
            ..Default::default()
        };
        let (events_tx, events_rx) = async_channel::bounded(256);
        let manager = SerialManager::shared();
        let available_ports = SerialManager::list_ports()
            .into_iter()
            .map(|port| port.port_name)
            .collect();
        let port_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("COM3 / /dev/ttyUSB0")
                .default_value(settings.port_config.port_name)
        });
        let send_input = cx.new(|cx| InputState::new(window, cx).multi_line(true));
        let preset_name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("命令名称")
                .default_value("新命令")
        });
        let preset_content_input = cx.new(|cx| InputState::new(window, cx).placeholder("命令内容"));
        let loop_interval_input = cx.new(|cx| InputState::new(window, cx).default_value("1000"));
        cx.spawn(async move |this, cx| {
            while let Ok(event) = events_rx.recv().await {
                if this
                    .update(cx, |view, cx| {
                        view.handle_serial_event(event);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            state,
            manager,
            events_tx,
            stream_decoder: crate::serial::encoding::StreamDecoder::default(),
            available_ports,
            port_input,
            send_input,
            preset_name_input,
            preset_content_input,
            loop_interval_input,
            loop_generation: Arc::new(AtomicU64::new(0)),
            editing_preset: None,
            last_export: None,
        }
    }

    fn handle_serial_event(&mut self, event: SerialEvent) {
        match event {
            SerialEvent::Data(data) => {
                let text = self
                    .stream_decoder
                    .decode(&data.raw_bytes, &data.encoding)
                    .unwrap_or_else(|_| {
                        encoding::decode(&data.raw_bytes, &Encoding::Hex).unwrap_or_default()
                    });
                let message = state::ReceivedMessage {
                    direction: MessageDirection::Received,
                    timestamp: data.timestamp,
                    data: text,
                    encoding: data.encoding,
                    raw_bytes: data.raw_bytes,
                };
                {
                    self.state.bytes_received += message.raw_bytes.len() as u64;
                    let lines = state::split_received_message_lines(
                        &mut self.state.receive_buffer,
                        message,
                    );
                    for line in lines {
                        state::push_log_bounded(&mut self.state.messages, line);
                    }
                    self.state.receive_preview = self.state.receive_buffer.pending().to_owned();
                    self.state.status = "正在接收".into();
                }
            }
            SerialEvent::Error(error) => {
                self.state.error = Some(error.clone());
                self.state.status = error;
            }
            SerialEvent::Disconnected => {
                if let Ok(mut manager) = self.manager.lock() {
                    let _ = manager.close();
                }
                self.state.connected = false;
                self.state.loop_send = false;
                self.state.status = "设备已断开".into();
            }
        }
    }

    fn sync_config_from_input(&mut self, cx: &Context<Self>) {
        self.state.config.port_name = self
            .port_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .to_owned();
    }

    fn toggle_connection(&mut self, cx: &mut Context<Self>) {
        if self.state.connected {
            let result = self
                .manager
                .lock()
                .map_err(|_| "串口锁已损坏".to_string())
                .and_then(|mut m| m.close());
            self.state.connected = false;
            self.state.loop_send = false;
            self.state.connected = self
                .manager
                .lock()
                .map(|manager| manager.is_open())
                .unwrap_or(false);
            self.state.status = result.map(|_| "已断开".into()).unwrap_or_else(|e| e);
        } else {
            self.sync_config_from_input(cx);
            if self.state.config.port_name.is_empty() {
                self.state.error = Some("请先填写串口名称".into());
                return;
            }
            self.state.connecting = true;
            self.state.error = None;
            let manager = Arc::clone(&self.manager);
            let config = self.state.config.clone();
            let encoding = self.state.send_encoding.clone();
            let events = self.events_tx.clone();
            cx.spawn(async move |this, cx| {
                let result = manager
                    .lock()
                    .map_err(|_| "串口锁已损坏".to_string())
                    .and_then(|mut manager| manager.open(&config, encoding, events));
                let _ = this.update(cx, |view, cx| {
                    view.state.connecting = false;
                    match result {
                        Ok(()) => {
                            view.state.connected = true;
                            view.state.status = format!("已连接 · {}", view.state.config.port_name);
                            let _ = persistence::save(&SavedSettings {
                                port_config: view.state.config.clone(),
                                presets: view.state.presets.clone(),
                            });
                        }
                        Err(error) => {
                            view.state.connected = false;
                            view.state.status = "连接失败".into();
                            view.state.error = Some(error);
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        }
        cx.notify();
    }

    fn send_message(&mut self, cx: &mut Context<Self>) {
        if !self.state.connected {
            self.state.error = Some("请先连接串口".into());
            cx.notify();
            return;
        }
        let content = self.send_input.read(cx).value().to_string();
        if content.is_empty() {
            return;
        }
        let content = state::apply_send_line_ending(
            &content,
            &self.state.send_encoding,
            &self.state.send_line_ending,
        );
        let bytes = match encoding::encode(&content, &self.state.send_encoding) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.state.error = Some(e);
                cx.notify();
                return;
            }
        };
        let manager = Arc::clone(&self.manager);
        let encoding = self.state.send_encoding.clone();
        self.state.status = "发送中…".into();
        cx.spawn(async move |this, cx| {
            let result = manager
                .lock()
                .map_err(|_| "串口锁已损坏".to_string())
                .and_then(|mut manager| manager.send(&bytes));
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(written) => {
                        view.state.bytes_sent += written as u64;
                        state::push_log_bounded(
                            &mut view.state.messages,
                            state::sent_message(
                                &state::current_message_timestamp(),
                                content,
                                encoding,
                                bytes,
                            ),
                        );
                        view.state.status = format!("已发送 {written} 字节");
                    }
                    Err(error) => view.state.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn export_logs(&mut self, format: &str) {
        let Some(dir) = dirs::download_dir().or_else(dirs::document_dir) else {
            self.state.error = Some("无法定位导出目录".into());
            return;
        };
        let path = match export::exporter::normalize_export_path(
            dir.join(format!(
                "serial-log-{}",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            )),
            format,
        ) {
            Ok(path) => path,
            Err(error) => {
                self.state.error = Some(error);
                return;
            }
        };
        let entries = export::exporter::entries_from_messages(&self.state.messages);
        let result = if format == "txt" {
            export::exporter::export_txt(&entries, &path)
        } else {
            export::export_csv(&entries, &path)
        };
        match result {
            Ok(()) => {
                self.last_export = Some(path);
                self.state.status = format!("已导出 {} 条日志", entries.len());
            }
            Err(e) => self.state.error = Some(e),
        }
    }

    fn clear_logs(&mut self) {
        self.state.messages.clear();
        self.state.receive_buffer.clear();
        self.state.receive_preview.clear();
        self.stream_decoder.clear();
        self.state.bytes_received = 0;
        self.state.bytes_sent = 0;
        self.state.status = "日志已清空".into();
    }
    fn cycle_encoding(&mut self) {
        self.state.send_encoding = match self.state.send_encoding {
            Encoding::Ascii => Encoding::Hex,
            Encoding::Hex => Encoding::Utf8,
            Encoding::Utf8 => Encoding::Gbk,
            Encoding::Gbk => Encoding::Ascii,
        };
    }
    fn cycle_baud(&mut self) {
        self.state.config.baud_rate = match self.state.config.baud_rate {
            9_600 => 19_200,
            19_200 => 38_400,
            38_400 => 57_600,
            57_600 => 115_200,
            _ => 9_600,
        };
    }
    fn cycle_data_bits(&mut self) {
        self.state.config.data_bits = match self.state.config.data_bits {
            5 => 6,
            6 => 7,
            7 => 8,
            _ => 5,
        };
    }
    fn cycle_stop_bits(&mut self) {
        self.state.config.stop_bits = if self.state.config.stop_bits == "1" {
            "2"
        } else {
            "1"
        }
        .into();
    }
    fn cycle_parity(&mut self) {
        self.state.config.parity = match self.state.config.parity.as_str() {
            "none" => "odd",
            "odd" => "even",
            _ => "none",
        }
        .into();
    }
    fn cycle_flow_control(&mut self) {
        self.state.config.flow_control = match self.state.config.flow_control.as_str() {
            "none" => "rts_cts",
            "rts_cts" => "xon_xoff",
            _ => "none",
        }
        .into();
    }
    fn refresh_ports(&mut self) {
        self.available_ports = SerialManager::list_ports()
            .into_iter()
            .map(|port| port.port_name)
            .collect();
        self.state.status = format!("已刷新串口列表 · {} 个设备", self.available_ports.len());
    }
    fn add_preset(&mut self, cx: &mut Context<Self>) {
        let name = self.preset_name_input.read(cx).value().to_string();
        let content = self.preset_content_input.read(cx).value().to_string();
        if name.trim().is_empty() || content.is_empty() {
            return;
        }
        if let Some(index) = self.editing_preset.take() {
            if let Some(preset) = self.state.presets.get_mut(index) {
                preset.name = name;
                preset.content = content;
                preset.encoding = self.state.send_encoding.clone();
            }
        } else {
            self.state.presets.push(PresetCommand {
                id: state::new_preset_command_id(),
                name,
                content,
                encoding: self.state.send_encoding.clone(),
            });
        }
        let _ = persistence::save(&SavedSettings {
            port_config: self.state.config.clone(),
            presets: self.state.presets.clone(),
        });
    }
    fn begin_edit(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(preset) = self.state.presets.get(index).cloned() {
            self.editing_preset = Some(index);
            self.send_encoding_for_edit(preset.encoding.clone());
            self.preset_name_input
                .update(cx, |input, cx| input.set_value(preset.name, window, cx));
            self.preset_content_input
                .update(cx, |input, cx| input.set_value(preset.content, window, cx));
        }
    }
    fn send_encoding_for_edit(&mut self, encoding: Encoding) {
        self.state.send_encoding = encoding;
    }
    fn cancel_edit(&mut self) {
        self.editing_preset = None;
    }
    fn delete_preset(&mut self, index: usize) {
        if index < self.state.presets.len() {
            self.state.presets.remove(index);
            let _ = persistence::save(&SavedSettings {
                port_config: self.state.config.clone(),
                presets: self.state.presets.clone(),
            });
        }
    }
    fn send_preset(&mut self, preset: PresetCommand, cx: &mut Context<Self>) {
        if !self.state.connected {
            self.state.error = Some("请先连接串口".into());
            cx.notify();
            return;
        }
        let content = state::apply_send_line_ending(
            &preset.content,
            &preset.encoding,
            &self.state.send_line_ending,
        );
        let bytes = match encoding::encode(&content, &preset.encoding) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.state.error = Some(error);
                cx.notify();
                return;
            }
        };
        let result = self
            .manager
            .lock()
            .map_err(|_| "串口锁已损坏".to_string())
            .and_then(|mut manager| manager.send(&bytes));
        match result {
            Ok(written) => {
                self.state.bytes_sent += written as u64;
                state::push_log_bounded(
                    &mut self.state.messages,
                    state::sent_message(
                        &state::current_message_timestamp(),
                        content,
                        preset.encoding,
                        bytes,
                    ),
                );
                self.state.status = format!("预设已发送 {written} 字节");
            }
            Err(error) => self.state.error = Some(error),
        }
        cx.notify();
    }
    fn toggle_loop_send(&mut self, cx: &mut Context<Self>) {
        if self.state.loop_send {
            self.state.loop_send = false;
            self.loop_generation.fetch_add(1, Ordering::AcqRel);
            self.state.status = "循环发送已停止".into();
            cx.notify();
            return;
        }
        if !self.state.connected {
            self.state.error = Some("请先连接串口".into());
            cx.notify();
            return;
        }
        let content = self.send_input.read(cx).value().to_string();
        if content.is_empty() {
            self.state.error = Some("循环发送内容不能为空".into());
            cx.notify();
            return;
        }
        let interval = self
            .loop_interval_input
            .read(cx)
            .value()
            .to_string()
            .parse::<u64>()
            .unwrap_or(1000);
        let interval = state::normalize_loop_interval_ms(interval);
        self.state.loop_interval_ms = interval;
        let wire_content = state::apply_send_line_ending(
            &content,
            &self.state.send_encoding,
            &self.state.send_line_ending,
        );
        let bytes = match encoding::encode(&wire_content, &self.state.send_encoding) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.state.error = Some(error);
                cx.notify();
                return;
            }
        };
        let generation = self.loop_generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.state.loop_send = true;
        self.state.error = None;
        self.state.status = format!("循环发送中 · 每 {interval} ms");
        let manager = Arc::clone(&self.manager);
        let encoding = self.state.send_encoding.clone();
        cx.spawn(async move |this, cx| loop {
            let should_continue = this
                .update(cx, |view, _| {
                    state::should_continue_loop_send(
                        view.state.connected,
                        view.state.loop_send,
                        generation,
                        view.loop_generation.load(Ordering::Acquire),
                    )
                })
                .unwrap_or(false);
            if !should_continue {
                break;
            }
            let result = manager
                .lock()
                .map_err(|_| "串口锁已损坏".to_string())
                .and_then(|mut manager| manager.send(&bytes));
            match result {
                Ok(written) => {
                    let _ = this.update(cx, |view, cx| {
                        view.state.bytes_sent += written as u64;
                        state::push_log_bounded(
                            &mut view.state.messages,
                            state::sent_message(
                                &state::current_message_timestamp(),
                                wire_content.clone(),
                                encoding.clone(),
                                bytes.clone(),
                            ),
                        );
                        cx.notify();
                    });
                }
                Err(error) => {
                    let _ = this.update(cx, |view, cx| {
                        view.state.loop_send = false;
                        view.state.error = Some(error);
                        cx.notify();
                    });
                    break;
                }
            }
            cx.background_executor()
                .timer(Duration::from_millis(interval))
                .await;
        })
        .detach();
        cx.notify();
    }
    fn use_preset(&mut self, content: String, window: &mut Window, cx: &mut Context<Self>) {
        self.send_input
            .update(cx, |input, cx| input.set_value(content, window, cx));
        self.state.active_tab = ActiveTab::ReceiveSend;
    }
    fn button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        primary: bool,
    ) -> Button {
        let button = Button::new(id).label(label);
        if primary {
            button.primary()
        } else {
            button
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let connected = self.state.connected;
        let available_count = self.available_ports.len();
        let messages = self
            .state
            .messages
            .iter()
            .rev()
            .take(500)
            .cloned()
            .collect::<Vec<_>>();
        let status = self.state.status.clone();
        let error = self.state.error.clone();
        let tab = self.state.active_tab;
        let connect_label = if self.state.connecting {
            "连接中…"
        } else if connected {
            "断开串口"
        } else {
            "连接串口"
        };
        let encoding_label = format!(
            "发送编码：{}",
            match self.state.send_encoding {
                Encoding::Ascii => "ASCII",
                Encoding::Hex => "HEX",
                Encoding::Utf8 => "UTF-8",
                Encoding::Gbk => "GBK",
            }
        );
        let ending_label = format!(
            "行尾：{}",
            match self.state.send_line_ending {
                SendLineEnding::None => "无",
                SendLineEnding::Cr => "CR",
                SendLineEnding::Lf => "LF",
                SendLineEnding::Crlf => "CRLF",
            }
        );
        let receive_tab = self
            .button("tab-receive", "收发终端", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.active_tab = ActiveTab::ReceiveSend;
                cx.notify();
            }));
        let manager_tab = self
            .button("tab-manager", "命令管理", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.active_tab = ActiveTab::CommandManager;
                cx.notify();
            }));
        let connect = self
            .button("connect", connect_label, true)
            .disabled(self.state.connecting)
            .on_click(cx.listener(|this, _, _, cx| this.toggle_connection(cx)));
        let send = self
            .button("send", "发送", true)
            .on_click(cx.listener(|this, _, _, cx| this.send_message(cx)));
        let clear = self
            .button("clear", "清空日志", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.clear_logs();
                cx.notify();
            }));
        let export_csv_button = self
            .button("export-csv", "导出 CSV", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.export_logs("csv");
                cx.notify();
            }));
        let export_txt_button = self
            .button("export-txt", "导出 TXT", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.export_logs("txt");
                cx.notify();
            }));
        let encoding = self
            .button("encoding", encoding_label, false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.cycle_encoding();
                cx.notify();
            }));
        let ending = self
            .button("ending", ending_label, false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.send_line_ending = match this.state.send_line_ending {
                    SendLineEnding::None => SendLineEnding::Cr,
                    SendLineEnding::Cr => SendLineEnding::Lf,
                    SendLineEnding::Lf => SendLineEnding::Crlf,
                    SendLineEnding::Crlf => SendLineEnding::None,
                };
                cx.notify();
            }));
        let loop_button = self
            .button(
                "loop",
                if self.state.loop_send {
                    "停止循环"
                } else {
                    "循环发送"
                },
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| this.toggle_loop_send(cx)));
        let refresh_ports = self
            .button("refresh-ports", "刷新", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.refresh_ports();
                cx.notify();
            }));
        let data_bits = self
            .button(
                "data-bits",
                format!("数据位 {}", self.state.config.data_bits),
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.cycle_data_bits();
                cx.notify();
            }));
        let stop_bits = self
            .button(
                "stop-bits",
                format!("停止位 {}", self.state.config.stop_bits),
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.cycle_stop_bits();
                cx.notify();
            }));
        let parity = self
            .button(
                "parity",
                format!("校验 {}", self.state.config.parity),
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.cycle_parity();
                cx.notify();
            }));
        let flow = self
            .button(
                "flow",
                format!("流控 {}", self.state.config.flow_control),
                false,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.cycle_flow_control();
                cx.notify();
            }));
        let log_children = messages.into_iter().map(|message| {
            let received = message.direction == MessageDirection::Received;
            let direction = if received { "← 收到" } else { "→ 发送" };
            let timestamp = message.timestamp.clone();
            div()
                .flex()
                .gap_3()
                .py_2()
                .border_b_1()
                .border_color(crate::ui::theme::color(crate::ui::theme::BORDER))
                .child(
                    div()
                        .w(px(86.))
                        .text_xs()
                        .text_color(crate::ui::theme::color(crate::ui::theme::MUTED))
                        .child(timestamp),
                )
                .child(
                    div()
                        .w(px(58.))
                        .text_sm()
                        .text_color(crate::ui::theme::color(if received {
                            crate::ui::theme::PRIMARY_DARK
                        } else {
                            crate::ui::theme::MUTED
                        }))
                        .child(direction),
                )
                .child(div().flex_1().font_family("monospace").child(
                    state::format_message_display(&message, self.state.hex_display),
                ))
        });
        let port_input = Input::new(&self.port_input);
        let send_input = Input::new(&self.send_input).appearance(false).h_full();
        let receive_content = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().flex().gap_2().child(receive_tab).child(manager_tab))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Checkbox::new("hex")
                                    .label("HEX 显示")
                                    .checked(self.state.hex_display)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.state.hex_display = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(clear)
                            .child(export_csv_button)
                            .child(export_txt_button),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .bg(crate::ui::theme::color(crate::ui::theme::CARD))
                    .border_1()
                    .border_color(crate::ui::theme::color(crate::ui::theme::BORDER))
                    .rounded_lg()
                    .p_3()
                    .children(log_children),
            )
            .child(if self.state.receive_preview.is_empty() {
                div().h(px(0.))
            } else {
                div()
                    .text_xs()
                    .text_color(crate::ui::theme::color(crate::ui::theme::MUTED))
                    .child(format!(
                        "未完成行：{}",
                        state::visualize_serial_data(&self.state.receive_preview)
                    ))
            })
            .child(
                div()
                    .h(px(150.))
                    .bg(crate::ui::theme::color(crate::ui::theme::CARD))
                    .border_1()
                    .border_color(crate::ui::theme::color(crate::ui::theme::BORDER))
                    .rounded_lg()
                    .p_3()
                    .child(send_input),
            )
            .child(
                div().flex().items_center().justify_between().child(
                    div()
                        .flex()
                        .gap_2()
                        .child(encoding)
                        .child(ending)
                        .child(Input::new(&self.loop_interval_input).w(px(82.)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(crate::ui::theme::color(crate::ui::theme::MUTED))
                                .child("ms"),
                        )
                        .child(
                            Checkbox::new("auto")
                                .label("自动滚动")
                                .checked(self.state.auto_scroll)
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.state.auto_scroll = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(loop_button)
                        .child(send),
                ),
            );
        let presets = self
            .state
            .presets
            .iter()
            .enumerate()
            .map(|(index, preset)| {
                let item = preset.clone();
                let content = item.content.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .border_b_1()
                    .border_color(crate::ui::theme::color(crate::ui::theme::BORDER))
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(item.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(crate::ui::theme::color(crate::ui::theme::MUTED))
                                    .child(content.clone()),
                            ),
                    )
                    .child(self.button(("preset-load", index), "载入", false).on_click(
                        cx.listener(move |this, _, window, cx| {
                            this.use_preset(content.clone(), window, cx)
                        }),
                    ))
                    .child(
                        self.button(("preset-send", index), "发送", true)
                            .disabled(!connected)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.send_preset(item.clone(), cx)
                            })),
                    )
                    .child(self.button(("preset-edit", index), "编辑", false).on_click(
                        cx.listener(move |this, _, window, cx| {
                            this.begin_edit(index, window, cx);
                            cx.notify();
                        }),
                    ))
                    .child(
                        self.button(("preset-delete", index), "删除", false)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_preset(index);
                                cx.notify();
                            })),
                    )
            });
        let save_label = if self.editing_preset.is_some() {
            "更新命令"
        } else {
            "保存命令"
        };
        let cancel_edit = self
            .button("cancel-edit", "取消", false)
            .on_click(cx.listener(|this, _, _, cx| {
                this.cancel_edit();
                cx.notify();
            }));
        let manager_content = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("命令管理"),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(Input::new(&self.preset_name_input))
                    .child(Input::new(&self.preset_content_input).flex_1())
                    .child(
                        self.button("add-preset", save_label, true)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.add_preset(cx);
                                cx.notify();
                            })),
                    )
                    .when(self.editing_preset.is_some(), |this| {
                        this.child(cancel_edit)
                    }),
            )
            .child(div().flex().flex_wrap().gap_2().children(presets));
        let main_content = if tab == ActiveTab::ReceiveSend {
            receive_content
        } else {
            manager_content
        };
        div()
            .size_full()
            .bg(crate::ui::theme::color(crate::ui::theme::BACKGROUND))
            .text_color(crate::ui::theme::color(crate::ui::theme::TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(64.))
                    .px_6()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(34.))
                                    .rounded_lg()
                                    .bg(crate::ui::theme::color(crate::ui::theme::PRIMARY))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(crate::ui::theme::color(crate::ui::theme::CARD))
                                    .child("⌁"),
                            )
                            .child(
                                div()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Serial Debugger"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(crate::ui::theme::color(
                                                crate::ui::theme::MUTED,
                                            ))
                                            .child("GPUI · 串口调试工作台"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().size(px(8.)).rounded_full().bg(
                                        crate::ui::theme::color(if connected {
                                            crate::ui::theme::PRIMARY
                                        } else {
                                            crate::ui::theme::BORDER
                                        }),
                                    ))
                                    .child(status),
                            )
                            .child(connect),
                    ),
            )
            .child(
                div().flex_1().p_5().gap_4().flex().child(
                    div()
                        .w(px(255.))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            crate::ui::components::card()
                                .child(crate::ui::components::label("连接配置"))
                                .child(port_input)
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(crate::ui::theme::color(
                                            crate::ui::theme::MUTED,
                                        ))
                                        .child(format!("发现 {available_count} 个串口设备")),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(crate::ui::components::label(format!(
                                            "波特率：{}",
                                            self.state.config.baud_rate
                                        )))
                                        .child(self.button("baud", "切换", false).on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.cycle_baud();
                                                cx.notify();
                                            }),
                                        )),
                                )
                                .child(div().flex().gap_2().child(data_bits).child(stop_bits))
                                .child(div().flex().gap_2().child(parity).child(flow))
                                .child(refresh_ports)
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(crate::ui::theme::color(
                                            crate::ui::theme::MUTED,
                                        ))
                                        .child("连接前可切换完整串口参数"),
                                )
                                .child(
                                    div()
                                        .mt_3()
                                        .text_xs()
                                        .text_color(crate::ui::theme::color(
                                            crate::ui::theme::MUTED,
                                        ))
                                        .child(format!(
                                            "接收 {} B · 发送 {} B",
                                            self.state.bytes_received, self.state.bytes_sent
                                        )),
                                ),
                        )
                        .child(main_content),
                ),
            )
            .child(
                div()
                    .h(px(32.))
                    .px_6()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(crate::ui::theme::color(crate::ui::theme::MUTED))
                    .child(error.unwrap_or_else(|| "就绪 · 所有数据仅在本地处理".into()))
                    .child(
                        self.last_export
                            .as_ref()
                            .map(|path| format!("最近导出：{}", path.display()))
                            .unwrap_or_default(),
                    ),
            )
    }
}
