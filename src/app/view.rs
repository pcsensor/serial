use super::AppView;
use crate::{
    state::{ActiveTab, Encoding, MessageDirection, SendLineEnding},
    ui::{
        components::{self, ButtonTone},
        theme,
    },
};
use gpui::{
    div, prelude::*, px, Context, Div, FontWeight, IntoElement, Render, ScrollHandle,
    StatefulInteractiveElement, Subscription, WeakEntity, Window,
};
use gpui_component::{
    checkbox::Checkbox, input::Input, scroll::ScrollableElement, Disableable, Root, Sizable,
    WindowExt,
};

// Render dialog contents separately so the dialog layer never re-borrows the
// app while its main layout is being rendered. Observe live connection updates.
struct ConnectionDialog {
    owner: WeakEntity<AppView>,
    _subscription: Subscription,
}

impl Render for ConnectionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.owner
            .update(cx, |view, cx| view.render_connection_panel(cx))
            .unwrap_or_else(|_| div())
    }
}

fn encoding_name(encoding: &Encoding) -> &'static str {
    match encoding {
        Encoding::Ascii => "ASCII",
        Encoding::Hex => "HEX",
        Encoding::Utf8 => "UTF-8",
        Encoding::Gbk => "GBK",
    }
}

fn line_ending_name(ending: &SendLineEnding) -> &'static str {
    match ending {
        SendLineEnding::None => "无",
        SendLineEnding::Cr => "CR",
        SendLineEnding::Lf => "LF",
        SendLineEnding::Crlf => "CRLF",
    }
}

fn parity_name(value: &str) -> &'static str {
    match value {
        "odd" => "奇校验",
        "even" => "偶校验",
        _ => "无校验",
    }
}

fn flow_name(value: &str) -> &'static str {
    match value {
        "rts_cts" => "RTS/CTS",
        "xon_xoff" => "XON/XOFF",
        _ => "无流控",
    }
}

fn styled_input(input: Input) -> Input {
    input
        .bg(theme::color(theme::SURFACE))
        .border_2()
        .rounded(px(3.))
}

fn field(label: impl Into<gpui::SharedString>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(components::micro_label(label))
        .child(control)
}

fn log_viewport(handle: &ScrollHandle) -> gpui::Stateful<Div> {
    div()
        .flex_1()
        .min_h(px(0.))
        .id("receive-log")
        .track_scroll(handle)
        .overflow_y_scroll()
        .overflow_x_scroll()
        .vertical_scrollbar(handle)
        .horizontal_scrollbar(handle)
}

impl AppView {
    fn render_header(&mut self, cx: &mut Context<Self>) -> Div {
        let terminal_tone = if self.state.active_tab == ActiveTab::ReceiveSend {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        };
        let commands_tone = if self.state.active_tab == ActiveTab::CommandManager {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        };
        let terminal_tab = components::neo_button(cx, "tab-terminal", "数据终端", terminal_tone)
            .h(px(34.))
            .shadow_none()
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.active_tab = ActiveTab::ReceiveSend;
                this.pending_delete = None;
                cx.notify();
            }));
        let commands_tab = components::neo_button(cx, "tab-commands", "命令库", commands_tone)
            .h(px(34.))
            .shadow_none()
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.active_tab = ActiveTab::CommandManager;
                this.confirm_clear = false;
                cx.notify();
            }));
        let (connection_label, connection_color) = if self.state.connecting {
            ("正在连接", theme::YELLOW)
        } else if self.state.connected {
            ("已连接", theme::LIME)
        } else {
            ("未连接", theme::PINK)
        };
        let connection_button =
            components::neo_button(cx, "open-connection", "连接台", ButtonTone::Secondary)
                .h(px(34.))
                .on_click(cx.listener(|_, _, window, cx| {
                    if window.has_active_dialog(cx) {
                        return;
                    }
                    let owner = cx.entity();
                    let panel = cx.new(|cx| ConnectionDialog {
                        owner: owner.downgrade(),
                        _subscription: cx.observe(&owner, |_, _, cx| cx.notify()),
                    });
                    window.open_dialog(cx, move |dialog, _, _| {
                        dialog
                            .title("连接台")
                            .w(px(640.))
                            .close_button(false)
                            .border_2()
                            .border_color(theme::color(theme::INK))
                            .rounded(px(4.))
                            .child(panel.clone())
                    });
                }));

        div()
            .h(px(56.))
            .flex_shrink_0()
            .px_5()
            .bg(theme::color(theme::SURFACE))
            .border_b_2()
            .border_color(theme::color(theme::INK))
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
                            .bg(theme::color(theme::INK))
                            .border_2()
                            .border_color(theme::color(theme::INK))
                            .rounded(px(3.))
                            .shadow(theme::hard_shadow(3.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme::color(theme::YELLOW))
                            .font_weight(FontWeight::BOLD)
                            .child("S/"),
                    )
                    .child(
                        div()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD)
                                    .child("串口工作台"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(theme::color(theme::MUTED))
                                    .child("SERIAL DEBUGGER · 0.1.12"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(terminal_tab)
                    .child(commands_tab),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(px(10.))
                            .rounded_full()
                            .bg(theme::color(connection_color))
                            .border_1()
                            .border_color(theme::color(theme::INK)),
                    )
                    .child(components::badge(connection_label, connection_color))
                    .child(connection_button),
            )
    }

    fn render_connection_panel(&mut self, cx: &mut Context<Self>) -> Div {
        let controls_locked = self.state.connected || self.state.connecting;
        let connection_label = if self.state.connecting {
            "连接中…"
        } else if self.state.connected {
            "断开串口"
        } else {
            "建立连接"
        };
        let connection_tone = if self.state.connected {
            ButtonTone::Danger
        } else {
            ButtonTone::Primary
        };
        let connect = components::neo_button(cx, "connect", connection_label, connection_tone)
            .h(px(36.))
            .disabled(self.state.connecting)
            .on_click(cx.listener(|this, _, _, cx| this.toggle_connection(cx)));
        let close = components::neo_button(cx, "close-connection", "关闭", ButtonTone::Secondary)
            .h(px(36.))
            .on_click(|_, window, cx| window.close_dialog(cx));
        let refresh =
            components::neo_button(cx, "refresh-ports", "刷新设备", ButtonTone::Secondary)
                .h(px(36.))
                .disabled(self.state.connecting)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.refresh_ports();
                    cx.notify();
                }));
        let baud = components::neo_button(
            cx,
            "baud",
            format!("波特率 {} ↻", self.state.config.baud_rate),
            ButtonTone::Secondary,
        )
        .h(px(36.))
        .disabled(controls_locked)
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_baud();
            cx.notify();
        }));
        let data_bits = components::neo_button(
            cx,
            "data-bits",
            format!("{} bit ↻", self.state.config.data_bits),
            ButtonTone::Secondary,
        )
        .disabled(controls_locked)
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_data_bits();
            cx.notify();
        }));
        let stop_bits = components::neo_button(
            cx,
            "stop-bits",
            format!("{} stop ↻", self.state.config.stop_bits),
            ButtonTone::Secondary,
        )
        .disabled(controls_locked)
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_stop_bits();
            cx.notify();
        }));
        let parity = components::neo_button(
            cx,
            "parity",
            format!("{} ↻", parity_name(&self.state.config.parity)),
            ButtonTone::Secondary,
        )
        .disabled(controls_locked)
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_parity();
            cx.notify();
        }));
        let flow = components::neo_button(
            cx,
            "flow",
            format!("{} ↻", flow_name(&self.state.config.flow_control)),
            ButtonTone::Secondary,
        )
        .disabled(controls_locked)
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_flow_control();
            cx.notify();
        }));
        let port_input = styled_input(Input::new(&self.port_input))
            .h(px(36.))
            .w(px(220.))
            .disabled(controls_locked);
        let port_buttons = self
            .available_ports
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, port)| {
                components::neo_button(cx, ("port", index), port.clone(), ButtonTone::Secondary)
                    .compact()
                    .h(px(32.))
                    .px_2()
                    .shadow_none()
                    .disabled(controls_locked)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_port(port.clone(), window, cx);
                        cx.notify();
                    }))
            })
            .collect::<Vec<_>>();
        let has_quick_ports = !port_buttons.is_empty();
        let device_summary = if self.available_ports.is_empty() {
            components::badge("未发现设备 · 可手动输入", theme::PINK)
        } else {
            components::badge(
                format!("发现 {} 个设备", self.available_ports.len()),
                theme::LIME,
            )
        };
        let lock_note = if controls_locked {
            components::badge("当前会话参数已锁定", theme::YELLOW)
        } else {
            components::badge("参数可编辑", theme::SURFACE_ALT)
        };
        let parity_code = match self.state.config.parity.as_str() {
            "odd" => "O",
            "even" => "E",
            _ => "N",
        };
        let port_name = if self.state.config.port_name.is_empty() {
            "未选择端口".to_owned()
        } else {
            self.state.config.port_name.clone()
        };
        let config_summary = format!(
            "{} · {} · {}{}{}",
            port_name,
            self.state.config.baud_rate,
            self.state.config.data_bits,
            parity_code,
            self.state.config.stop_bits
        );

        let mut advanced = div()
            .mt_3()
            .pt_3()
            .border_t_2()
            .border_color(theme::color(theme::INK))
            .child(
                div()
                    .mb_2()
                    .flex()
                    .gap_2()
                    .child(device_summary)
                    .child(lock_note),
            )
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_3()
                    .flex_wrap()
                    .child(field("数据位", data_bits))
                    .child(field("停止位", stop_bits))
                    .child(field("校验", parity))
                    .child(field("流控", flow))
                    .child(
                        div()
                            .pb_2()
                            .text_xs()
                            .text_color(theme::color(theme::MUTED))
                            .child("带 ↻ 的参数点击即可切换"),
                    ),
            );
        if has_quick_ports {
            advanced = advanced.child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(components::micro_label("快速选择"))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .h(px(44.))
                            .id("port-quick-list")
                            .track_scroll(&self.port_scroll_handle)
                            .overflow_x_scroll()
                            .horizontal_scrollbar(&self.port_scroll_handle)
                            .child(
                                div()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .children(port_buttons),
                            ),
                    ),
            );
        }

        div()
            .w_full()
            .min_w(px(0.))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_2()
                    .flex_wrap()
                    .child(field("串口设备", port_input))
                    .child(refresh)
                    .child(baud),
            )
            .child(advanced)
            .when_some(self.state.error.clone(), |this, error| {
                this.child(
                    div()
                        .mt_3()
                        .p_2()
                        .bg(theme::color(theme::PINK))
                        .text_sm()
                        .child(error),
                )
            })
            .child(
                div()
                    .mt_3()
                    .pt_3()
                    .border_t_1()
                    .border_color(theme::color(theme::INK))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_xs()
                            .font_family("monospace")
                            .child(config_summary),
                    )
                    .child(close)
                    .child(connect),
            )
    }

    fn render_terminal(&mut self, cx: &mut Context<Self>) -> Div {
        let connected = self.state.connected;
        let encoding_button = components::neo_button(
            cx,
            "encoding",
            format!("编码 · {} ↻", encoding_name(&self.state.send_encoding)),
            ButtonTone::Accent,
        )
        .h(px(34.))
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_encoding();
            cx.notify();
        }));
        let ending_button = components::neo_button(
            cx,
            "ending",
            format!(
                "行尾 · {} ↻",
                line_ending_name(&self.state.send_line_ending)
            ),
            ButtonTone::Secondary,
        )
        .h(px(34.))
        .on_click(cx.listener(|this, _, _, cx| {
            this.state.send_line_ending = match this.state.send_line_ending {
                SendLineEnding::None => SendLineEnding::Cr,
                SendLineEnding::Cr => SendLineEnding::Lf,
                SendLineEnding::Lf => SendLineEnding::Crlf,
                SendLineEnding::Crlf => SendLineEnding::None,
            };
            cx.notify();
        }));
        let loop_button = components::neo_button(
            cx,
            "loop-send",
            if self.state.loop_send {
                "停止循环"
            } else {
                "循环发送"
            },
            if self.state.loop_send {
                ButtonTone::Danger
            } else {
                ButtonTone::Dark
            },
        )
        .h(px(34.))
        .disabled(!connected || self.state.connecting)
        .on_click(cx.listener(|this, _, _, cx| this.toggle_loop_send(cx)));
        let send_button = components::neo_button(cx, "send", "立即发送 →", ButtonTone::Primary)
            .h(px(34.))
            .px_5()
            .disabled(!connected || self.state.connecting)
            .on_click(cx.listener(|this, _, _, cx| this.send_message(cx)));
        let clear_label = if self.confirm_clear {
            "确认清空"
        } else {
            "清空日志"
        };
        let clear_button = components::neo_button(cx, "clear", clear_label, ButtonTone::Danger)
            .h(px(36.))
            .on_click(cx.listener(|this, _, _, cx| {
                if this.confirm_clear {
                    this.clear_logs();
                    this.confirm_clear = false;
                } else {
                    this.confirm_clear = true;
                    this.state.status = "再次点击“确认清空”完成操作".into();
                }
                cx.notify();
            }));
        let cancel_clear =
            components::neo_button(cx, "cancel-clear", "取消", ButtonTone::Secondary)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.confirm_clear = false;
                    this.state.status = "已取消清空日志".into();
                    cx.notify();
                }))
                .h(px(36.));
        let export_csv =
            components::neo_button(cx, "export-csv", "导出 CSV", ButtonTone::Secondary)
                .h(px(36.))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.export_logs("csv", cx);
                    cx.notify();
                }));
        let export_txt =
            components::neo_button(cx, "export-txt", "导出 TXT", ButtonTone::Secondary)
                .h(px(36.))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.export_logs("txt", cx);
                    cx.notify();
                }));

        let focus_button = components::neo_button(
            cx,
            "focus-log",
            if self.log_focused {
                "退出专注"
            } else {
                "专注日志"
            },
            ButtonTone::Secondary,
        )
        .h(px(32.))
        .on_click(cx.listener(|this, _, _, cx| {
            this.log_focused = !this.log_focused;
            cx.notify();
        }));

        let mut toolbar = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(
                Checkbox::new("hex-display")
                    .label("HEX 显示")
                    .checked(self.state.hex_display)
                    .on_click(cx.listener(|this, checked, _, cx| {
                        this.state.hex_display = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Checkbox::new("auto-scroll")
                    .label("自动跟随")
                    .checked(self.state.auto_scroll)
                    .on_click(cx.listener(|this, checked, _, cx| {
                        this.state.auto_scroll = *checked;
                        cx.notify();
                    })),
            )
            .child(export_csv)
            .child(export_txt)
            .child(clear_button)
            .child(focus_button);
        if self.confirm_clear {
            toolbar = toolbar.child(cancel_clear);
        }

        let messages = self
            .state
            .messages
            .iter()
            .rev()
            .take(500)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        let is_empty = messages.is_empty();
        let log_rows = messages
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                let received = message.direction == MessageDirection::Received;
                let direction = if received {
                    "RX / 收到"
                } else {
                    "TX / 发出"
                };
                let direction_color = if received { theme::LIME } else { theme::BLUE };
                div()
                    .min_h(px(30.))
                    .px_3()
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_3()
                    .border_b_1()
                    .border_color(theme::color(theme::INK))
                    .when(index % 2 == 1, |this| {
                        this.bg(theme::color(theme::SURFACE_ALT))
                    })
                    .child(
                        div()
                            .w(px(104.))
                            .flex_shrink_0()
                            .font_family("monospace")
                            .text_xs()
                            .text_color(theme::color(theme::MUTED))
                            .child(message.timestamp.clone()),
                    )
                    .child(
                        div()
                            .w(px(86.))
                            .flex_shrink_0()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(
                                div()
                                    .px_1()
                                    .bg(theme::color(direction_color))
                                    .text_color(theme::color(theme::INK))
                                    .child(direction),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .font_family("monospace")
                            .text_sm()
                            .whitespace_normal()
                            .child(crate::state::format_message_display(
                                &message,
                                self.state.hex_display,
                            )),
                    )
            })
            .collect::<Vec<_>>();
        let log_body = if is_empty {
            log_viewport(&self.log_scroll_handle).child(
                div()
                    .size_full()
                    .min_h(px(170.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("等待第一条数据"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme::color(theme::MUTED))
                            .child("连接设备后，收发记录会按时间顺序出现在这里。"),
                    )
                    .child(components::badge("最多显示最近 500 条", theme::YELLOW)),
            )
        } else {
            log_viewport(&self.log_scroll_handle).children(log_rows)
        };
        let preview = if self.state.receive_preview.is_empty() {
            div().h(px(0.))
        } else {
            div()
                .h(px(34.))
                .flex_shrink_0()
                .px_3()
                .py_2()
                .bg(theme::color(theme::YELLOW))
                .border_t_2()
                .border_color(theme::color(theme::INK))
                .font_family("monospace")
                .text_xs()
                .truncate()
                .child(format!(
                    "未完成行 / {}",
                    crate::state::visualize_serial_data(&self.state.receive_preview)
                ))
        };
        let receive_encoding = if connected {
            format!("RX {}（连接时）", encoding_name(&self.receive_encoding))
        } else {
            "RX 等待连接".into()
        };

        let log_panel = components::surface()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .p_2()
                    .flex_shrink_0()
                    .border_b_2()
                    .border_color(theme::color(theme::INK))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(components::micro_label("实时日志"))
                            .child(div().font_family("monospace").text_xs().child(format!(
                                "RX {} B · TX {} B",
                                self.state.bytes_received, self.state.bytes_sent
                            ))),
                    )
                    .child(toolbar),
            )
            .child(
                div()
                    .h(px(28.))
                    .flex_shrink_0()
                    .px_3()
                    .bg(theme::color(theme::INK))
                    .text_color(theme::color(theme::WHITE))
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .child(div().w(px(104.)).child("时间"))
                    .child(div().w(px(86.)).child("方向"))
                    .child(div().flex_1().child("数据负载"))
                    .child(receive_encoding),
            )
            .child(log_body)
            .child(preview);

        let send_input = Input::new(&self.send_input).appearance(false).size_full();
        let interval_input = styled_input(Input::new(&self.loop_interval_input))
            .h(px(34.))
            .w(px(92.));
        let composer = components::surface()
            .p_2()
            .flex_shrink_0()
            .child(
                components::inset_surface()
                    .h(px(56.))
                    .p_2()
                    .child(send_input),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .child(encoding_button)
                            .child(ending_button)
                            .child(interval_input)
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme::color(theme::MUTED))
                                    .child("ms"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(loop_button)
                            .child(send_button),
                    ),
            );

        div()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .child(log_panel)
            .when(!self.log_focused, |this| this.child(composer))
    }

    fn render_commands(&mut self, cx: &mut Context<Self>) -> Div {
        let is_editing = self.editing_preset.is_some();
        let save_label = if is_editing {
            "更新命令"
        } else {
            "保存命令"
        };
        let save_button =
            components::neo_button(cx, "save-preset", save_label, ButtonTone::Primary).on_click(
                cx.listener(|this, _, window, cx| {
                    this.add_preset(window, cx);
                    cx.notify();
                }),
            );
        let cancel_edit =
            components::neo_button(cx, "cancel-edit", "取消编辑", ButtonTone::Secondary).on_click(
                cx.listener(|this, _, window, cx| {
                    this.cancel_edit(window, cx);
                    cx.notify();
                }),
            );
        let preset_encoding = components::neo_button(
            cx,
            "preset-encoding",
            format!("{} ↻", encoding_name(&self.preset_encoding)),
            ButtonTone::Accent,
        )
        .on_click(cx.listener(|this, _, _, cx| {
            this.cycle_preset_encoding();
            cx.notify();
        }));
        let editor = components::surface()
            .p_3()
            .flex_shrink_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_wrap()
                    .child(components::section_kicker(
                        "03",
                        if is_editing {
                            "编辑命令"
                        } else {
                            "新建命令"
                        },
                        theme::YELLOW,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(components::badge(
                                format!(
                                    "直接发送行尾 · {}",
                                    line_ending_name(&self.state.send_line_ending)
                                ),
                                theme::SURFACE_ALT,
                            ))
                            .when(is_editing, |this| {
                                this.child(components::badge("EDITING", theme::PINK))
                            }),
                    ),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_end()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        field(
                            "命令名称",
                            styled_input(Input::new(&self.preset_name_input))
                                .large()
                                .w_full(),
                        )
                        .w(px(190.))
                        .flex_shrink_0(),
                    )
                    .child(
                        field(
                            "命令内容",
                            styled_input(Input::new(&self.preset_content_input))
                                .large()
                                .w_full()
                                .h(px(76.)),
                        )
                        .flex_1()
                        .min_w(px(280.)),
                    )
                    .child(field("命令编码", preset_encoding).flex_shrink_0())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .pb_1()
                            .child(save_button)
                            .when(is_editing, |this| this.child(cancel_edit)),
                    ),
            );

        let is_empty = self.state.presets.is_empty();
        let preset_rows = self
            .state
            .presets
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, preset)| {
                let is_current = self.editing_preset.as_deref() == Some(preset.id.as_str());
                let is_pending_delete = self.pending_delete.as_deref() == Some(preset.id.as_str());
                let load_preset = preset.clone();
                let send_preset = preset.clone();
                let edit_preset = preset.clone();
                let delete_id = preset.id.clone();
                let load_button = components::neo_button(
                    cx,
                    ("preset-load", index),
                    "载入发送台",
                    ButtonTone::Secondary,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.use_preset(load_preset.clone(), window, cx);
                    cx.notify();
                }));
                let direct_send = components::neo_button(
                    cx,
                    ("preset-send", index),
                    "直接发送",
                    ButtonTone::Primary,
                )
                .disabled(!self.state.connected)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.send_preset(send_preset.clone(), cx)),
                );
                let edit_button = components::neo_button(
                    cx,
                    ("preset-edit", index),
                    "编辑",
                    if is_current {
                        ButtonTone::Accent
                    } else {
                        ButtonTone::Secondary
                    },
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.begin_edit(edit_preset.clone(), window, cx);
                    cx.notify();
                }));
                let delete_button = components::neo_button(
                    cx,
                    ("preset-delete", index),
                    if is_pending_delete {
                        "确认删除"
                    } else {
                        "删除"
                    },
                    ButtonTone::Danger,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.confirm_or_request_delete(delete_id.clone(), window, cx);
                    cx.notify();
                }));
                let cancel_delete = components::neo_button(
                    cx,
                    ("preset-delete-cancel", index),
                    "取消",
                    ButtonTone::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.pending_delete = None;
                    this.state.status = "已取消删除命令".into();
                    cx.notify();
                }));

                components::inset_surface()
                    .p_3()
                    .when(is_current, |this| this.bg(theme::color(theme::YELLOW)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .flex_wrap()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(components::badge(
                                        format!("{:02}", index + 1),
                                        theme::BLUE,
                                    ))
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::BOLD)
                                            .child(preset.name.clone()),
                                    )
                                    .child(components::badge(
                                        encoding_name(&preset.encoding),
                                        theme::YELLOW,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .flex_wrap()
                                    .child(load_button)
                                    .child(direct_send)
                                    .child(edit_button)
                                    .child(delete_button)
                                    .when(is_pending_delete, |this| this.child(cancel_delete)),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .pt_3()
                            .border_t_1()
                            .border_color(theme::color(theme::INK))
                            .font_family("monospace")
                            .text_sm()
                            .max_h(px(64.))
                            .overflow_hidden()
                            .whitespace_normal()
                            .text_color(theme::color(theme::MUTED))
                            .child(preset.content),
                    )
            })
            .collect::<Vec<_>>();
        let list_body = if is_empty {
            div()
                .flex_1()
                .min_h(px(0.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("还没有快捷命令"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme::color(theme::MUTED))
                        .child("在上方创建第一条命令，常用操作会更快。"),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h(px(0.))
                .id("preset-list")
                .track_scroll(&self.preset_scroll_handle)
                .overflow_y_scroll()
                .vertical_scrollbar(&self.preset_scroll_handle)
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .children(preset_rows)
                .into_any_element()
        };
        let list = components::surface()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_4()
                    .py_3()
                    .bg(theme::color(theme::INK))
                    .text_color(theme::color(theme::WHITE))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_weight(FontWeight::BOLD).child("已保存命令"))
                    .child(components::badge(
                        format!("{} 条", self.state.presets.len()),
                        theme::YELLOW,
                    )),
            )
            .child(list_body);

        div()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(components::section_kicker("02", "命令库", theme::LIME))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme::color(theme::MUTED))
                            .child("创建、编辑并快速发送常用串口命令"),
                    ),
            )
            .child(editor)
            .child(list)
    }

    fn render_footer(&mut self, cx: &mut Context<Self>) -> Div {
        let has_error = self.state.error.is_some();
        let primary_text = self
            .state
            .error
            .as_ref()
            .map(|error| format!("注意 / {error}"))
            .unwrap_or_else(|| format!("活动 / {}", self.state.status));
        let secondary_text = self
            .last_export
            .as_ref()
            .map(|path| format!("最近导出 / {}", path.display()))
            .unwrap_or_else(|| "本地处理 · 数据不会离开此设备".into());
        let dismiss_error =
            components::neo_button(cx, "dismiss-error", "关闭提示", ButtonTone::Secondary)
                .h(px(28.))
                .px_2()
                .shadow_none()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.error = None;
                    cx.notify();
                }));
        let mut trailing = div()
            .flex()
            .items_center()
            .gap_3()
            .child(div().max_w(px(480.)).truncate().child(secondary_text));
        if has_error {
            trailing = trailing.child(dismiss_error);
        }

        div()
            .h(px(32.))
            .flex_shrink_0()
            .px_5()
            .bg(theme::color(if has_error {
                theme::PINK
            } else {
                theme::INK
            }))
            .text_color(theme::color(if has_error {
                theme::INK
            } else {
                theme::WHITE
            }))
            .border_t_2()
            .border_color(theme::color(theme::INK))
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .child(div().flex_1().min_w(px(0.)).truncate().child(primary_text))
            .child(trailing)
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.render_header(cx);
        let content = match self.state.active_tab {
            ActiveTab::ReceiveSend => self.render_terminal(cx),
            ActiveTab::CommandManager => self.render_commands(cx),
        };
        let footer = self.render_footer(cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);

        div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(theme::color(theme::CANVAS))
            .text_color(theme::color(theme::INK))
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .min_w(px(0.))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(content),
            )
            .child(footer)
            .children(dialog_layer)
    }
}
