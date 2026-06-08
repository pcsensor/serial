use crate::services::api;
use crate::services::store::*;
use dioxus::prelude::*;

const RECEIVE_LOG_CONTAINER_ID: &str = "receive-log-container";

#[component]
pub fn ReceiveSendTab() -> Element {
    let mut state: AppState = use_context();
    let mut send_text = use_signal(String::new);

    use_effect(move || {
        if !claim_serial_data_listener_registration(
            &mut state.serial_data_listener_registered.write(),
        ) {
            return;
        }

        api::tauri_listen(
            "serial-data",
            move |event| match serde_wasm_bindgen::from_value::<serde_json::Value>(event)
                .map_err(|e| e.to_string())
                .and_then(parse_received_message_event)
            {
                Ok(payload) => {
                    let len = payload.raw_bytes.len() as u64;
                    let lines = {
                        let mut buffer = state.receive_line_buffer.write();
                        split_received_message_lines(&mut buffer, payload)
                    };
                    if !lines.is_empty() {
                        state.received_messages.write().extend(lines);
                    }
                    *state.bytes_received.write() += len;
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("[receive_send] serial-data 解析失败: {}", e).into(),
                    );
                }
            },
        );
    });

    use_effect(move || {
        let message_count = state.received_messages.read().len();
        let auto_scroll = *state.auto_scroll.read();
        if should_scroll_receive_log(auto_scroll, message_count) {
            spawn(async move {
                gloo_timers::future::sleep(std::time::Duration::from_millis(0)).await;
                scroll_receive_log_to_bottom(RECEIVE_LOG_CONTAINER_ID);
            });
        }
    });

    let send = move |_| {
        let content = (*send_text.read()).clone();
        let encoding = (*state.send_encoding.read()).clone();
        let line_ending = (*state.send_line_ending.read()).clone();
        let wire_content = apply_send_line_ending(&content, &encoding, &line_ending);
        spawn(async move {
            match api::send_serial_data(wire_content.clone(), encoding.clone()).await {
                Ok(n) => {
                    *state.bytes_sent.write() += n as u64;
                    let timestamp = current_message_timestamp();
                    state.received_messages.write().push(sent_message(
                        &timestamp,
                        wire_content,
                        encoding,
                    ));
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("[send] 发送失败: {}", e).into());
                }
            }
        });
    };

    let clear = move |_| {
        state.received_messages.write().clear();
        state.receive_line_buffer.write().clear();
        *state.bytes_received.write() = 0;
        *state.bytes_sent.write() = 0;
    };

    let export = move |_| {
        let entries: Vec<LogEntry> = state
            .received_messages
            .read()
            .iter()
            .map(|m| LogEntry {
                timestamp: m.timestamp.clone(),
                direction: message_direction_label(&m.direction).to_string(),
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
                    &Args {
                        entries,
                        path: p,
                        format: format.to_string(),
                    },
                )
                .await;
            }
        });
    };

    let messages = state.received_messages.read().clone();
    let encoding = state.send_encoding.read().clone();
    let line_ending = state.send_line_ending.read().clone();
    let auto_scroll = *state.auto_scroll.read();
    let hex_display = *state.hex_display.read();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;flex:1;overflow:hidden;gap:8px;",

            div {
                class: "glass-card",
                style: "flex:1;display:flex;flex-direction:column;overflow:hidden;padding:10px;",
                div {
                    id: RECEIVE_LOG_CONTAINER_ID,
                    style: "flex:1;overflow-y:auto;font-family:'Cascadia Code','Fira Code',monospace;font-size:12px;line-height:1.6;",
                    for (index, msg) in messages.iter().enumerate() {
                        {
                            let direction = message_direction_label(&msg.direction);
                            let data = format_message_display(msg, hex_display);
                            let direction_color = if msg.direction == MessageDirection::Sent {
                                "color:#22c55e;margin:0 6px;"
                            } else {
                                "color:#888;margin:0 6px;"
                            };
                            rsx! {
                        div {
                            key: "{received_message_render_key(index, msg)}",
                            style: "padding:2px 0;border-bottom:1px solid rgba(255,255,255,0.03);",
                            span { style: "color:#667eea;", "[{msg.timestamp}]" }
                            span { style: "{direction_color}", "{direction}:" }
                            span { "{data}" }
                        }
                            }
                        }
                    }
                }
                div {
                    style: "display:flex;align-items:center;gap:8px;margin-top:8px;",
                    label {
                        style: "display:flex;align-items:center;gap:4px;font-size:11px;color:#888;",
                        input {
                            r#type: "checkbox",
                            checked: auto_scroll,
                            onchange: move |e| {
                                *state.auto_scroll.write() = e.checked();
                            },
                        }
                        "自动滚动"
                    }
                    label {
                        style: "display:flex;align-items:center;gap:4px;font-size:11px;color:#888;",
                        input {
                            r#type: "checkbox",
                            checked: hex_display,
                            onchange: move |e| {
                                *state.hex_display.write() = e.checked();
                            },
                        }
                        "HEX显示"
                    }
                    div { style: "flex:1;" }
                    button { class: "btn btn-secondary", onclick: clear, "清空" }
                    button { class: "btn btn-secondary", onclick: export, "导出" }
                }
            }

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
                    select {
                        class: "glass-select",
                        value: "{send_line_ending_value(&line_ending)}",
                        onchange: move |e| {
                            *state.send_line_ending.write() = parse_send_line_ending_value(&e.value());
                        },
                        option { value: "none", "NONE" }
                        option { value: "cr", "CR" }
                        option { value: "lf", "LF" }
                        option { value: "crlf", "CRLF" }
                    }
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
                    div { style: "flex:1;" }
                    button {
                        class: "btn btn-primary",
                        onclick: send,
                        disabled: !*state.is_connected.read(),
                        "发送"
                    }
                    button {
                        class: if *state.loop_send.read() { "btn btn-danger" } else { "btn btn-secondary" },
                        onclick: {
                            move |_| {
                                let is_loop = *state.loop_send.read();
                                *state.loop_send.write() = !is_loop;
                                if !is_loop {
                                    let content = (*send_text.read()).clone();
                                    let encoding_val = (*state.send_encoding.read()).clone();
                                    let line_ending_val = (*state.send_line_ending.read()).clone();
                                    let wire_content = apply_send_line_ending(
                                        &content,
                                        &encoding_val,
                                        &line_ending_val,
                                    );
                                    let interval = normalize_loop_interval_ms(*state.loop_interval_ms.read());
                                    let mut st = state;
                                    spawn(async move {
                                        loop {
                                            if !should_continue_loop_send(
                                                *st.is_connected.read(),
                                                *st.loop_send.read(),
                                            ) {
                                                *st.loop_send.write() = false;
                                                break;
                                            }
                                            match api::send_serial_data(
                                                wire_content.clone(),
                                                encoding_val.clone(),
                                            ).await {
                                                Ok(n) => {
                                                    *st.bytes_sent.write() += n as u64;
                                                    let timestamp = current_message_timestamp();
                                                    st.received_messages.write().push(sent_message(
                                                        &timestamp,
                                                        wire_content.clone(),
                                                        encoding_val.clone(),
                                                    ));
                                                }
                                                Err(e) => {
                                                    web_sys::console::error_1(
                                                        &format!("[loop_send] 发送失败: {}", e).into(),
                                                    );
                                                    *st.loop_send.write() = false;
                                                    break;
                                                }
                                            }
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

#[cfg(target_arch = "wasm32")]
fn scroll_receive_log_to_bottom(element_id: &str) {
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(document) = js_sys::Reflect::get(&window, &JsValue::from_str("document")) else {
        return;
    };
    let Some(get_element_by_id) =
        js_sys::Reflect::get(&document, &JsValue::from_str("getElementById"))
            .ok()
            .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
    else {
        return;
    };

    let args = js_sys::Array::new();
    args.push(&JsValue::from_str(element_id));
    let Ok(element) = get_element_by_id.apply(&document, &args) else {
        return;
    };
    if element.is_null() || element.is_undefined() {
        return;
    }
    let Ok(scroll_height) = js_sys::Reflect::get(&element, &JsValue::from_str("scrollHeight"))
    else {
        return;
    };
    let _ = js_sys::Reflect::set(&element, &JsValue::from_str("scrollTop"), &scroll_height);
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_receive_log_to_bottom(_element_id: &str) {}
