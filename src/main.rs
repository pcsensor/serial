mod app;
mod export;
mod serial;
mod state;
mod ui;

use gpui::{prelude::*, px, size, App, Application, Bounds, WindowBounds, WindowOptions};

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1100.0), px(750.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| app::AppView::new(window, cx)),
        )
        .expect("failed to open the main window");

        cx.activate(true);
    });
}
