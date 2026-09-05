use super::theme;
use gpui::{div, prelude::*, px, App, Div, ElementId, FontWeight, SharedString};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariants};

#[derive(Clone, Copy)]
pub enum ButtonTone {
    Primary,
    Secondary,
    Accent,
    Dark,
    Danger,
}

pub fn surface() -> Div {
    div()
        .bg(theme::color(theme::SURFACE))
        .border_2()
        .border_color(theme::color(theme::INK))
        .rounded(px(4.))
        .shadow(theme::hard_shadow(4.))
}

pub fn inset_surface() -> Div {
    div()
        .bg(theme::color(theme::SURFACE_ALT))
        .border_2()
        .border_color(theme::color(theme::INK))
        .rounded(px(3.))
}

pub fn micro_label(text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .text_color(theme::color(theme::MUTED))
        .child(text.into())
}

pub fn section_kicker(
    index: impl Into<SharedString>,
    title: impl Into<SharedString>,
    accent: u32,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .px_2()
                .py_1()
                .bg(theme::color(accent))
                .border_2()
                .border_color(theme::color(theme::INK))
                .rounded(px(2.))
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .child(index.into()),
        )
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .child(title.into()),
        )
}

pub fn badge(text: impl Into<SharedString>, background: u32) -> Div {
    div()
        .px_2()
        .py_1()
        .bg(theme::color(background))
        .border_1()
        .border_color(theme::color(theme::INK))
        .rounded(px(2.))
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .child(text.into())
}

pub fn neo_button(
    cx: &App,
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    tone: ButtonTone,
) -> Button {
    let (background, hover, active, foreground) = match tone {
        ButtonTone::Primary => (
            theme::YELLOW,
            theme::YELLOW_HOVER,
            theme::YELLOW_HOVER,
            theme::INK,
        ),
        ButtonTone::Secondary => (
            theme::SURFACE,
            theme::SURFACE_ALT,
            theme::YELLOW,
            theme::INK,
        ),
        ButtonTone::Accent => (
            theme::BLUE,
            theme::BLUE_HOVER,
            theme::BLUE_HOVER,
            theme::WHITE,
        ),
        ButtonTone::Dark => (theme::INK, 0x333333, 0x000000, theme::WHITE),
        ButtonTone::Danger => (
            theme::PINK,
            theme::PINK_HOVER,
            theme::PINK_HOVER,
            theme::INK,
        ),
    };
    let variant = ButtonCustomVariant::new(cx)
        .color(theme::color(background))
        .foreground(theme::color(foreground))
        .border(theme::color(theme::INK))
        .hover(theme::color(hover))
        .active(theme::color(active));

    let button = Button::new(id)
        .child(
            div()
                .text_color(theme::color(foreground))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.into()),
        )
        .custom(variant)
        .rounded(ButtonRounded::Small)
        .h(px(44.))
        .px_3()
        .border_2()
        .border_color(theme::color(theme::INK))
        .font_weight(FontWeight::SEMIBOLD);

    if matches!(tone, ButtonTone::Secondary) {
        button.shadow_none()
    } else {
        button.shadow(theme::hard_shadow(2.))
    }
}
