use crate::services::store::*;
use dioxus::prelude::*;

#[component]
pub fn TabNav() -> Element {
    let mut state: AppState = use_context();
    let active = state.active_tab.read().clone();

    rsx! {
        div {
            style: "display:flex;gap:4px;padding:8px 0;",
            tab_button {
                label: "收发",
                is_active: active == ActiveTab::ReceiveSend,
                onclick: move |_| {
                    *state.active_tab.write() = ActiveTab::ReceiveSend;
                }
            }
            tab_button {
                label: "指令管理",
                is_active: active == ActiveTab::CommandManager,
                onclick: move |_| {
                    *state.active_tab.write() = ActiveTab::CommandManager;
                }
            }
        }
    }
}

#[component]
fn tab_button(label: String, is_active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let bg = if is_active {
        "background:linear-gradient(135deg,rgba(102,126,234,0.3),rgba(118,75,162,0.3));border-color:rgba(102,126,234,0.5);"
    } else {
        "background:rgba(255,255,255,0.05);border-color:rgba(255,255,255,0.1);"
    };

    rsx! {
        button {
            class: "btn",
            style: "padding:6px 20px;font-size:13px;border:1px solid;border-radius:8px;{bg}",
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}
