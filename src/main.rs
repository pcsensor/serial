mod app;

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
            |_, cx| cx.new(|_| app::AppView),
        )
        .expect("failed to open the main window");

        cx.activate(true);
    });
}
