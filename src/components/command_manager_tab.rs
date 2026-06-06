use crate::services::api;
use crate::services::store::*;
use dioxus::prelude::*;

#[component]
pub fn CommandManagerTab() -> Element {
    let mut state: AppState = use_context();
    let mut editing = use_signal(|| false);
    let mut edit_name = use_signal(String::new);
    let mut edit_content = use_signal(String::new);
    let mut edit_encoding = use_signal(|| Encoding::Ascii);
    let mut edit_id = use_signal(|| Option::<String>::None);

    use_effect(move || {
        if !claim_once(&mut state.preset_commands_loaded.write()) {
            return;
        }

        spawn(async move {
            match api::load_preset_commands().await {
                Ok(cmds) => {
                    *state.preset_commands.write() = cmds;
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("[command_manager] 加载预设指令失败: {}", e).into(),
                    );
                }
            }
        });
    });

    let save = move |_| {
        let name = (*edit_name.read()).clone();
        let content = (*edit_content.read()).clone();
        let encoding = (*edit_encoding.read()).clone();
        let id = (*edit_id.read()).clone();

        if name.is_empty() || content.is_empty() {
            return;
        }

        let cmds = {
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
                    let id = new_preset_command_id();
                    commands.push(PresetCommand {
                        id,
                        name,
                        content,
                        encoding,
                    });
                }
            }
            commands.clone()
        };
        *editing.write() = false;

        spawn(async move {
            if let Err(e) = api::save_preset_commands(&cmds).await {
                web_sys::console::error_1(
                    &format!("[command_manager] 保存预设指令失败: {}", e).into(),
                );
            }
        });
    };

    let delete = move |id: String| {
        state.preset_commands.write().retain(|c| c.id != id);
        let cmds = state.preset_commands.read().clone();
        spawn(async move {
            if let Err(e) = api::save_preset_commands(&cmds).await {
                web_sys::console::error_1(
                    &format!("[command_manager] 删除后保存预设指令失败: {}", e).into(),
                );
            }
        });
    };

    let send_preset = move |cmd: PresetCommand| {
        let line_ending = (*state.send_line_ending.read()).clone();
        let wire_content = apply_send_line_ending(&cmd.content, &cmd.encoding, &line_ending);
        spawn(async move {
            match api::send_serial_data(wire_content.clone(), cmd.encoding.clone()).await {
                Ok(n) => {
                    *state.bytes_sent.write() += n as u64;
                    let timestamp = current_message_timestamp();
                    state.received_messages.write().push(sent_message(
                        &timestamp,
                        wire_content,
                        cmd.encoding,
                    ));
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("[command_manager] 发送预设指令失败: {}", e).into(),
                    );
                }
            }
        });
    };

    let commands = state.preset_commands.read().clone();
    let is_editing = *editing.read();
    let line_ending = state.send_line_ending.read().clone();

    rsx! {
        div {
            style: "display:flex;flex-direction:column;flex:1;overflow:hidden;gap:8px;",

            div {
                style: "display:flex;align-items:center;gap:8px;",
                div {
                    style: "font-size:14px;font-weight:600;color:#e5e7eb;",
                    "指令管理"
                }
                div { style: "flex:1;" }
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
            }

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
                                let sp = send_preset;
                                move |_| sp(c.clone())
                            },
                            "发送"
                        }
                        button {
                            class: "btn btn-secondary",
                            style: "padding:3px 8px;font-size:11px;",
                            onclick: {
                                let c = cmd.clone();
                                move |_| {
                                    *edit_name.write() = c.name.clone();
                                    *edit_content.write() = c.content.clone();
                                    *edit_encoding.write() = c.encoding.clone();
                                    *edit_id.write() = Some(c.id.clone());
                                    *editing.write() = true;
                                }
                            },
                            "编辑"
                        }
                        button {
                            class: "btn btn-danger",
                            style: "padding:3px 8px;font-size:11px;",
                            onclick: {
                                let id = cmd.id.clone();
                                let mut d = delete;
                                move |_| d(id.clone())
                            },
                            "删除"
                        }
                    }
                }
            }

            if !is_editing {
                button {
                    class: "btn btn-primary",
                    style: "width:100%;padding:10px;",
                    onclick: move |_| {
                        *edit_name.write() = String::new();
                        *edit_content.write() = String::new();
                        *edit_encoding.write() = Encoding::Ascii;
                        *edit_id.write() = None;
                        *editing.write() = true;
                    },
                    "+ 新增指令"
                }
            }

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
