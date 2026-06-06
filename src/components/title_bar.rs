use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    let minimize = move |_| async move {
        let _: Result<(), String> =
            crate::services::api::tauri_invoke_no_args("plugin:window|minimize").await;
    };

    let close = move |_| async move {
        let _: Result<(), String> =
            crate::services::api::tauri_invoke_no_args("plugin:window|close").await;
    };

    rsx! {
        div {
            style: "display:flex;align-items:center;justify-content:space-between;padding:8px 16px;background:rgba(0,0,0,0.2);",
            "data-tauri-drag-region": "true",
            div {
                style: "display:flex;align-items:center;gap:8px;",
                "data-tauri-drag-region": "true",
                span {
                    style: "font-size:14px;font-weight:600;background:linear-gradient(135deg,#667eea,#764ba2);-webkit-background-clip:text;-webkit-text-fill-color:transparent;",
                    "⚡ 串口调试助手"
                }
            }
            div {
                style: "display:flex;gap:8px;",
                button {
                    class: "btn btn-secondary",
                    style: "padding:4px 10px;font-size:12px;",
                    onclick: minimize,
                    "—"
                }
                button {
                    class: "btn btn-danger",
                    style: "padding:4px 10px;font-size:12px;",
                    onclick: close,
                    "✕"
                }
            }
        }
    }
}
