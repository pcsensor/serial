use crate::services::api;
use crate::services::store::*;
use dioxus::prelude::*;

#[derive(Debug, Clone, serde::Deserialize)]
struct PortInfo {
    port_name: String,
    port_type: String,
}

#[component]
pub fn SerialPanel() -> Element {
    let mut state: AppState = use_context();
    let mut ports = use_signal(Vec::<PortInfo>::new);
    let mut status_message = use_signal(String::new);

    let toggle_connection = move |_| {
        let is_connected = *state.is_connected.read();
        let config = (*state.port_config.read()).clone();
        let encoding = (*state.send_encoding.read()).clone();
        if !can_toggle_connection(
            is_connected,
            &config.port_name,
            *state.connection_in_progress.read(),
        ) {
            return;
        }
        *state.connection_in_progress.write() = true;
        spawn(async move {
            if is_connected {
                *state.loop_send.write() = false;
                let result: Result<(), String> = api::tauri_invoke_no_args("close_port").await;
                if result.is_ok() {
                    *state.is_connected.write() = false;
                    state.receive_line_buffer.write().clear();
                    status_message.write().clear();
                } else if let Err(e) = result {
                    *status_message.write() = format!("关闭失败: {}", e);
                }
            } else {
                #[derive(serde::Serialize)]
                struct Args {
                    config: PortConfig,
                    encoding: Encoding,
                }
                let result: Result<(), String> =
                    api::tauri_invoke("open_port", &Args { config, encoding }).await;
                if result.is_ok() {
                    *state.is_connected.write() = true;
                    status_message.write().clear();
                } else if let Err(e) = result {
                    *status_message.write() = format!("打开失败: {}", e);
                }
            }
            *state.connection_in_progress.write() = false;
        });
    };

    use_effect(move || {
        spawn(async move {
            web_sys::console::log_1(&"[serial_panel] 开始调用 list_ports...".into());
            let result: Result<Vec<PortInfo>, String> =
                api::tauri_invoke_no_args("list_ports").await;
            match result {
                Ok(p) => {
                    web_sys::console::log_1(
                        &format!("[serial_panel] 找到 {} 个串口", p.len()).into(),
                    );
                    if p.is_empty() {
                        *status_message.write() = "未发现串口，请连接设备后刷新".to_string();
                    } else {
                        status_message.write().clear();
                    }
                    *ports.write() = p;
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("[serial_panel] list_ports 失败: {}", e).into(),
                    );
                    *status_message.write() = format!("获取串口失败: {}", e);
                }
            }
        });
    });

    let is_connected = *state.is_connected.read();
    let is_connection_in_progress = *state.connection_in_progress.read();
    let config = state.port_config.read().clone();
    let ports_list = ports.read().clone();
    let selected_port = ports_list
        .iter()
        .find(|p| p.port_name == config.port_name)
        .cloned();
    let status = status_message.read().clone();
    let selected_port_meta = if let Some(port) = selected_port.as_ref() {
        format!("类型: {}", compact_port_type(&port.port_type))
    } else if ports_list.is_empty() {
        "未发现可用串口".to_string()
    } else {
        "请选择一个串口设备".to_string()
    };

    rsx! {
        div {
            class: "glass-card serial-card",
            div {
                class: "serial-card-header",
                span { class: "serial-title", "串口配置" }
                span { class: "serial-count", "{ports_list.len()} 个端口" }
            }

            label_item { "端口" }
            div { class: "serial-port-row",
                select {
                    class: "glass-select",
                    value: "{config.port_name}",
                    onchange: move |e| {
                        state.port_config.write().port_name = e.value();
                    },
                    option { value: "", "选择端口" }
                    for p in ports_list.iter() {
                        option { value: "{p.port_name}", "{p.port_name}" }
                    }
                }
                button {
                    class: "btn btn-secondary icon-button",
                    title: "刷新串口列表",
                    onclick: move |_| {
                        spawn(async move {
                            let result: Result<Vec<PortInfo>, String> =
                                api::tauri_invoke_no_args("list_ports").await;
                            match result {
                                Ok(p) => {
                                    web_sys::console::log_1(&format!("[refresh] 找到 {} 个串口", p.len()).into());
                                    if p.is_empty() {
                                        *status_message.write() = "未发现串口，请连接设备后刷新".to_string();
                                    } else {
                                        status_message.write().clear();
                                    }
                                    *ports.write() = p;
                                }
                                Err(e) => {
                                    web_sys::console::error_1(&format!("[refresh] list_ports 失败: {}", e).into());
                                    *status_message.write() = format!("获取串口失败: {}", e);
                                }
                            }
                        });
                    },
                    "↻"
                }
            }
            div {
                class: "serial-port-meta",
                "{selected_port_meta}"
            }

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

            label_item { "停止位" }
            select {
                class: "glass-select",
                value: "{config.stop_bits}",
                onchange: move |e| {
                    state.port_config.write().stop_bits = e.value();
                },
                option { value: "1", "1" }
                option { value: "2", "2" }
            }

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
            }

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

            if !status.is_empty() {
                div { class: "serial-status-message", "{status}" }
            }

            div { style: "margin-top:auto;",
                button {
                    class: if is_connected { "btn btn-danger" } else { "btn btn-primary" },
                    style: "width:100%;padding:10px;",
                    onclick: toggle_connection,
                    disabled: !can_toggle_connection(
                        is_connected,
                        &config.port_name,
                        is_connection_in_progress,
                    ),
                    if is_connection_in_progress {
                        "处理中..."
                    } else if is_connected {
                        "断开连接"
                    } else {
                        "打开串口"
                    }
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

fn compact_port_type(port_type: &str) -> &'static str {
    if port_type.contains("UsbPort") {
        "USB 设备"
    } else if port_type.contains("BluetoothPort") {
        "蓝牙设备"
    } else if port_type.contains("PciPort") {
        "PCI 设备"
    } else {
        "未知类型"
    }
}

#[cfg(test)]
mod tests {
    use super::compact_port_type;

    #[test]
    fn compacts_serial_port_debug_type_for_display() {
        assert_eq!(
            compact_port_type("UsbPort(UsbPortInfo { vid: 1027, pid: 24577 })"),
            "USB 设备"
        );
        assert_eq!(compact_port_type("BluetoothPort"), "蓝牙设备");
        assert_eq!(compact_port_type("PciPort"), "PCI 设备");
        assert_eq!(compact_port_type("Unknown"), "未知类型");
    }
}
