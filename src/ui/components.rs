use super::theme;
use gpui::{div, prelude::*};

pub fn card() -> gpui::Div {
    div()
        .bg(theme::color(theme::CARD))
        .border_1()
        .border_color(theme::color(theme::BORDER))
        .rounded_lg()
        .p_4()
}
pub fn label(text: impl Into<gpui::SharedString>) -> gpui::Div {
    div()
        .text_sm()
        .text_color(theme::color(theme::MUTED))
        .child(text.into())
}
