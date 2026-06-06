use crate::services::store::AppState;
use dioxus::prelude::*;

#[component]
pub fn StatusBar() -> Element {
    let state: AppState = use_context();
    let is_connected = *state.is_connected.read();
    let bytes_rx = *state.bytes_received.read();
    let bytes_tx = *state.bytes_sent.read();

    let status_text = if is_connected {
        "已连接"
    } else {
        "未连接"
    };
    let status_color = if is_connected { "#4ade80" } else { "#f87171" };

    rsx! {
        div {
            style: "display:flex;justify-content:space-between;padding:6px 16px;background:rgba(0,0,0,0.25);font-size:11px;color:#888;",
            div {
                style: "display:flex;align-items:center;gap:6px;",
                span {
                    style: "width:8px;height:8px;border-radius:50%;background:{status_color};display:inline-block;",
                }
                span { "{status_text}" }
            }
            div {
                style: "display:flex;gap:16px;",
                span { "接收: {bytes_rx} 字节" }
                span { "发送: {bytes_tx} 字节" }
            }
        }
    }
}
