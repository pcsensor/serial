use crate::{
    export::{
        self,
        persistence::{self, SavedSettings},
    },
    serial::{encoding, manager::SerialManager, protocol::SerialEvent},
    state::{self, ActiveTab, AppState, Encoding, MessageDirection, PresetCommand},
};
use async_channel::Sender;
use gpui::{prelude::*, Context, Entity, ScrollHandle, Subscription, Window};
use gpui_component::input::InputState;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

mod view;

pub struct AppView {
    state: AppState,
    manager: Arc<Mutex<SerialManager>>,
    events_tx: Sender<SerialEvent>,
    receive_encoding: Encoding,
    preset_encoding: Encoding,
    preview_decoder: crate::serial::encoding::StreamDecoder,
    available_ports: Vec<String>,
    port_input: Entity<InputState>,
    send_input: Entity<InputState>,
    preset_name_input: Entity<InputState>,
    preset_content_input: Entity<InputState>,
    loop_interval_input: Entity<InputState>,
    loop_generation: Arc<AtomicU64>,
    log_scroll_handle: ScrollHandle,
    port_scroll_handle: ScrollHandle,
    preset_scroll_handle: ScrollHandle,
    _port_input_subscription: Subscription,
    editing_preset: Option<String>,
    pending_delete: Option<String>,
    confirm_clear: bool,
    log_focused: bool,
    last_export: Option<PathBuf>,
}

impl AppView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (settings, load_error) = match persistence::load_with_error() {
            Ok(settings) => (settings, None),
            Err(error) => (SavedSettings::default(), Some(error)),
        };
        let state = AppState {
            config: settings.port_config.clone(),
            presets: settings.presets.clone(),
            error: load_error,
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
        let send_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入发送内容…")
                .multi_line(true)
        });
        let preset_name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("命令名称")
                .default_value("新命令")
        });
        let preset_content_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("命令内容")
                .multi_line(true)
        });
        let loop_interval_input = cx.new(|cx| InputState::new(window, cx).default_value("1000"));
        let port_input_subscription = cx.subscribe(
            &port_input,
            |view, _, event: &gpui_component::input::InputEvent, cx| {
                if matches!(event, gpui_component::input::InputEvent::Change) {
                    view.sync_config_from_input(cx);
                    view.save_settings();
                    cx.notify();
                }
            },
        );
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
            receive_encoding: Encoding::default(),
            preset_encoding: Encoding::default(),
            preview_decoder: crate::serial::encoding::StreamDecoder::default(),
            available_ports,
            port_input,
            send_input,
            preset_name_input,
            preset_content_input,
            loop_interval_input,
            loop_generation: Arc::new(AtomicU64::new(0)),
            log_scroll_handle: ScrollHandle::new(),
            port_scroll_handle: ScrollHandle::new(),
            preset_scroll_handle: ScrollHandle::new(),
            _port_input_subscription: port_input_subscription,
            editing_preset: None,
            pending_delete: None,
            confirm_clear: false,
            log_focused: false,
            last_export: None,
        }
    }

    fn handle_serial_event(&mut self, event: SerialEvent) {
        match event {
            SerialEvent::Data(data) => {
                self.receive_encoding = data.encoding.clone();
                self.state.bytes_received += data.raw_bytes.len() as u64;
                let lines = state::split_received_data_lines(
                    &mut self.state.receive_buffer,
                    &data.timestamp,
                    &data.raw_bytes,
                    &data.encoding,
                );
                for line in lines {
                    state::push_log_bounded(&mut self.state.messages, line);
                }
                self.preview_decoder.clear();
                self.state.receive_preview = self
                    .preview_decoder
                    .decode(self.state.receive_buffer.pending_bytes(), &data.encoding)
                    .unwrap_or_else(|_| {
                        state::decode_received_bytes_lossy(
                            self.state.receive_buffer.pending_bytes(),
                            &data.encoding,
                        )
                    });
                if self.state.auto_scroll {
                    self.log_scroll_handle.scroll_to_bottom();
                }
                self.state.status = "正在接收".into();
            }
            SerialEvent::Error(error) => {
                self.state.error = Some(error.clone());
                self.state.status = error;
            }
            SerialEvent::Disconnected => {
                if let Ok(mut manager) = self.manager.lock() {
                    let _ = manager.close();
                }
                self.flush_receive_buffer();
                self.state.connected = false;
                self.state.loop_send = false;
                self.loop_generation.fetch_add(1, Ordering::AcqRel);
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
            self.flush_receive_buffer();
            self.state.connected = false;
            self.state.loop_send = false;
            self.loop_generation.fetch_add(1, Ordering::AcqRel);
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
                cx.notify();
                return;
            }
            self.state.connecting = true;
            self.state.error = None;
            let manager = Arc::clone(&self.manager);
            let config = self.state.config.clone();
            let encoding = self.state.send_encoding.clone();
            self.receive_encoding = encoding.clone();
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
                            view.save_settings();
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

    fn export_logs(&mut self, format: &'static str, cx: &mut Context<Self>) {
        self.flush_receive_buffer();
        let entries = export::exporter::entries_from_messages(&self.state.messages);
        let filename = format!(
            "serial-log-{}.{}",
            chrono::Local::now().format("%Y%m%d-%H%M%S"),
            format
        );
        let filter_name = if format == "txt" {
            "文本文件"
        } else {
            "CSV 文件"
        };
        cx.spawn(async move |this, cx| {
            let selected = rfd::AsyncFileDialog::new()
                .set_title("导出串口日志")
                .set_file_name(filename)
                .add_filter(filter_name, &[format])
                .save_file()
                .await;
            let Some(file) = selected else {
                return;
            };
            let path = match export::exporter::normalize_export_path(file.path(), format) {
                Ok(path) => path,
                Err(error) => {
                    let _ = this.update(cx, |view, cx| {
                        view.state.error = Some(error);
                        cx.notify();
                    });
                    return;
                }
            };
            let result = if format == "txt" {
                export::exporter::export_txt(&entries, &path)
            } else {
                export::export_csv(&entries, &path)
            };
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.last_export = Some(path);
                        view.state.status = format!("已导出 {} 条日志", entries.len());
                    }
                    Err(error) => view.state.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        self.state.status = format!("等待选择 {format_name} 保存位置", format_name = filter_name);
    }

    fn clear_logs(&mut self) {
        self.state.messages.clear();
        self.state.receive_buffer.clear();
        self.state.receive_preview.clear();
        self.preview_decoder.clear();
        self.state.bytes_received = 0;
        self.state.bytes_sent = 0;
        self.state.status = "日志已清空".into();
    }

    fn flush_receive_buffer(&mut self) {
        if !self.state.receive_buffer.has_pending() {
            self.state.receive_buffer.clear();
            self.state.receive_preview.clear();
            self.preview_decoder.clear();
            return;
        }
        let raw_bytes = self.state.receive_buffer.take_pending();
        let encoding = self.receive_encoding.clone();
        self.preview_decoder.clear();
        let data = state::decode_received_bytes_lossy(&raw_bytes, &encoding);
        state::push_log_bounded(
            &mut self.state.messages,
            state::ReceivedMessage {
                direction: MessageDirection::Received,
                timestamp: state::current_message_timestamp(),
                data,
                encoding,
                raw_bytes,
            },
        );
        self.state.receive_preview.clear();
    }

    fn save_settings(&mut self) {
        if let Err(error) = persistence::save(&SavedSettings {
            port_config: self.state.config.clone(),
            presets: self.state.presets.clone(),
        }) {
            self.state.error = Some(error);
        }
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
        const BAUD_RATES: &[u32] = &[
            9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600,
        ];
        let index = BAUD_RATES
            .iter()
            .position(|rate| *rate == self.state.config.baud_rate)
            .unwrap_or(0);
        self.state.config.baud_rate = BAUD_RATES[(index + 1) % BAUD_RATES.len()];
        self.save_settings();
    }
    fn cycle_data_bits(&mut self) {
        self.state.config.data_bits = match self.state.config.data_bits {
            5 => 6,
            6 => 7,
            7 => 8,
            _ => 5,
        };
        self.save_settings();
    }
    fn cycle_stop_bits(&mut self) {
        self.state.config.stop_bits = if self.state.config.stop_bits == "1" {
            "2"
        } else {
            "1"
        }
        .into();
        self.save_settings();
    }
    fn cycle_parity(&mut self) {
        self.state.config.parity = match self.state.config.parity.as_str() {
            "none" => "odd",
            "odd" => "even",
            _ => "none",
        }
        .into();
        self.save_settings();
    }
    fn cycle_flow_control(&mut self) {
        self.state.config.flow_control = match self.state.config.flow_control.as_str() {
            "none" => "rts_cts",
            "rts_cts" => "xon_xoff",
            _ => "none",
        }
        .into();
        self.save_settings();
    }
    fn refresh_ports(&mut self) {
        self.available_ports = SerialManager::list_ports()
            .into_iter()
            .map(|port| port.port_name)
            .collect();
        self.state.status = format!("已刷新串口列表 · {} 个设备", self.available_ports.len());
    }
    fn select_port(&mut self, port: String, window: &mut Window, cx: &mut Context<Self>) {
        self.state.config.port_name = port.clone();
        self.port_input
            .update(cx, |input, cx| input.set_value(port, window, cx));
        self.save_settings();
    }
    fn add_preset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.preset_name_input.read(cx).value().to_string();
        let content = self.preset_content_input.read(cx).value().to_string();
        if name.trim().is_empty() || content.is_empty() {
            self.state.error = Some("命令名称和内容都不能为空".into());
            return;
        }
        let was_editing = self.editing_preset.is_some();
        if let Some(id) = self.editing_preset.take() {
            if let Some(preset) = self.state.presets.iter_mut().find(|preset| preset.id == id) {
                preset.name = name.trim().to_owned();
                preset.content = content;
                preset.encoding = self.preset_encoding.clone();
            } else {
                self.state.error = Some("要编辑的命令已不存在".into());
                return;
            }
        } else {
            self.state.presets.push(PresetCommand {
                id: state::new_preset_command_id(),
                name: name.trim().to_owned(),
                content,
                encoding: self.preset_encoding.clone(),
            });
        }
        if self.state.error.as_deref() == Some("命令名称和内容都不能为空") {
            self.state.error = None;
        }
        self.state.status = if was_editing {
            "命令已更新".into()
        } else {
            "命令已保存".into()
        };
        self.save_settings();
        self.preset_name_input
            .update(cx, |input, cx| input.set_value("新命令", window, cx));
        self.preset_content_input
            .update(cx, |input, cx| input.set_value("", window, cx));
    }
    fn begin_edit(&mut self, preset: PresetCommand, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_preset = Some(preset.id);
        self.pending_delete = None;
        self.preset_encoding = preset.encoding.clone();
        self.preset_name_input
            .update(cx, |input, cx| input.set_value(preset.name, window, cx));
        self.preset_content_input
            .update(cx, |input, cx| input.set_value(preset.content, window, cx));
        self.state.status = "正在编辑命令".into();
    }
    fn cycle_preset_encoding(&mut self) {
        self.preset_encoding = match self.preset_encoding {
            Encoding::Ascii => Encoding::Hex,
            Encoding::Hex => Encoding::Utf8,
            Encoding::Utf8 => Encoding::Gbk,
            Encoding::Gbk => Encoding::Ascii,
        };
    }
    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_preset = None;
        self.preset_name_input
            .update(cx, |input, cx| input.set_value("新命令", window, cx));
        self.preset_content_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.state.status = "已取消编辑".into();
    }
    fn confirm_or_request_delete(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_delete.as_deref() == Some(id.as_str()) {
            self.state.presets.retain(|preset| preset.id != id);
            if self.editing_preset.as_deref() == Some(id.as_str()) {
                self.editing_preset = None;
                self.preset_name_input
                    .update(cx, |input, cx| input.set_value("新命令", window, cx));
                self.preset_content_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
            }
            self.pending_delete = None;
            self.state.status = "命令已删除".into();
            self.save_settings();
        } else {
            self.pending_delete = Some(id);
            self.state.status = "再次点击“确认删除”完成操作".into();
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
        let manager = Arc::clone(&self.manager);
        let encoding = preset.encoding;
        self.state.status = "预设发送中…".into();
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
                        view.state.status = format!("预设已发送 {written} 字节");
                    }
                    Err(error) => view.state.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
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
            let next_interval = this
                .update(cx, |view, cx| {
                    let value = view
                        .loop_interval_input
                        .read(cx)
                        .value()
                        .to_string()
                        .parse::<u64>()
                        .unwrap_or(view.state.loop_interval_ms);
                    let value = state::normalize_loop_interval_ms(value);
                    view.state.loop_interval_ms = value;
                    value
                })
                .unwrap_or(interval);
            cx.background_executor()
                .timer(Duration::from_millis(next_interval))
                .await;
        })
        .detach();
        cx.notify();
    }
    fn use_preset(&mut self, preset: PresetCommand, window: &mut Window, cx: &mut Context<Self>) {
        let name = preset.name.clone();
        self.send_input
            .update(cx, |input, cx| input.set_value(preset.content, window, cx));
        self.state.send_encoding = preset.encoding;
        self.state.active_tab = ActiveTab::ReceiveSend;
        self.pending_delete = None;
        self.state.status = format!("已载入命令 · {name}");
    }
}
