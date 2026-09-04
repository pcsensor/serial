use gpui::{rgb, Hsla};

pub const BACKGROUND: u32 = 0xF5F7FA;
pub const CARD: u32 = 0xFFFFFF;
pub const BORDER: u32 = 0xE4E7EC;
pub const TEXT: u32 = 0x182230;
pub const MUTED: u32 = 0x667085;
pub const PRIMARY: u32 = 0x4CAF88;
pub const PRIMARY_DARK: u32 = 0x2F8F6A;
pub fn color(value: u32) -> Hsla {
    rgb(value).into()
}
