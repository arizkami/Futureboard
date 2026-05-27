use gpui::{px, size, Bounds, Point, WindowBounds, WindowOptions};

use sphere_ui_components::platform_chrome;

pub fn studio_window_options() -> WindowOptions {
    let mut options = platform_chrome::studio_window_options();
    options.window_bounds = Some(WindowBounds::Windowed(Bounds {
        origin: Point::default(),
        size: size(px(1400.0), px(900.0)),
    }));
    options
}
