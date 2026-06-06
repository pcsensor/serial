use dioxus::prelude::*;

use crate::components;
use crate::services::store::{ActiveTab, AppState};

static MAIN_CSS: Asset = asset!("/src/assets/main.css");

#[component]
pub fn App() -> Element {
    let state = use_context_provider(AppState::init);

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        div { class: "app-container",
            components::title_bar::TitleBar {}
            div { class: "main-content",
                div { class: "sidebar",
                    components::serial_panel::SerialPanel {}
                }
                div { class: "content-area",
                    components::tab_nav::TabNav {}
                    match *state.active_tab.read() {
                        ActiveTab::ReceiveSend => rsx! {
                            components::receive_send_tab::ReceiveSendTab {}
                        },
                        ActiveTab::CommandManager => rsx! {
                            components::command_manager_tab::CommandManagerTab {}
                        },
                    }
                }
            }
            components::status_bar::StatusBar {}
        }
    }
}
