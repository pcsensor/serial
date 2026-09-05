use gpui::{point, px, rgb, App, BoxShadow, Hsla};

// A restrained neo-brutalist palette: bold enough to give the utility a
// distinct identity, calm enough for long serial-debugging sessions.
pub const CANVAS: u32 = 0xE9EDFF;
pub const SURFACE: u32 = 0xFFFDF7;
pub const SURFACE_ALT: u32 = 0xF5F1E8;
pub const INK: u32 = 0x171717;
pub const MUTED: u32 = 0x65636D;
pub const YELLOW: u32 = 0xFFD84A;
pub const YELLOW_HOVER: u32 = 0xFFC933;
pub const LIME: u32 = 0xA8F06A;
pub const LIME_HOVER: u32 = 0x91DD54;
pub const BLUE: u32 = 0x5B72FF;
pub const BLUE_HOVER: u32 = 0x455EEA;
pub const PINK: u32 = 0xFF7A8A;
pub const PINK_HOVER: u32 = 0xF05D70;
pub const WHITE: u32 = 0xFFFFFF;

pub fn color(value: u32) -> Hsla {
    rgb(value).into()
}

pub fn hard_shadow(offset: f32) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color(INK),
        offset: point(px(offset), px(offset)),
        blur_radius: px(0.),
        spread_radius: px(0.),
    }]
}

pub fn apply_component_theme(cx: &mut App) {
    let theme = gpui_component::Theme::global_mut(cx);
    theme.font_family = ".SystemUIFont".into();
    theme.font_size = px(16.);
    theme.radius = px(3.);
    theme.radius_lg = px(5.);
    theme.shadow = false;

    theme.background = color(SURFACE);
    theme.foreground = color(INK);
    theme.border = color(INK);
    theme.input = color(INK);
    theme.caret = color(BLUE);
    theme.ring = color(BLUE);
    theme.primary = color(YELLOW);
    theme.primary_hover = color(YELLOW_HOVER);
    theme.primary_active = color(YELLOW_HOVER);
    theme.primary_foreground = color(INK);
    theme.secondary = color(SURFACE);
    theme.secondary_hover = color(SURFACE_ALT);
    theme.secondary_active = color(YELLOW);
    theme.secondary_foreground = color(INK);
    theme.muted = color(SURFACE_ALT);
    theme.muted_foreground = color(MUTED);
    theme.selection = color(YELLOW);
    theme.danger = color(PINK);
    theme.danger_hover = color(PINK_HOVER);
    theme.danger_active = color(PINK_HOVER);
    theme.danger_foreground = color(INK);
    theme.success = color(LIME);
    theme.success_hover = color(LIME_HOVER);
    theme.success_active = color(LIME_HOVER);
    theme.success_foreground = color(INK);
    theme.scrollbar = color(CANVAS);
    theme.scrollbar_thumb = color(INK);
    theme.scrollbar_thumb_hover = color(BLUE);
    theme.title_bar = color(SURFACE);
    theme.title_bar_border = color(INK);
}
