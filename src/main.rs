mod app;
mod export;
mod serial;
mod state;
mod ui;

use gpui::{prelude::*, px, size, App, Application, Bounds, WindowBounds, WindowOptions};

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        ui::theme::apply_component_theme(cx);

        let bounds = Bounds::centered(None, size(px(1240.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.0), px(660.0))),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| app::AppView::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            },
        )
        .expect("failed to open the main window");

        cx.activate(true);
    });
}
