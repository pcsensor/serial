use gpui::{div, prelude::*, Context, Render, Window};

pub struct AppView;

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div().child("Serial Debugger")
    }
}
