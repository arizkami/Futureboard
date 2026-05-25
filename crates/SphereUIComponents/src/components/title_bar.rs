use gpui::{div, px, svg, Div, InteractiveElement, ParentElement, Rgba, Styled, WindowControlArea};

use crate::theme::Colors;

pub const TITLEBAR_HEIGHT: f32 = 32.0;
pub const STATUSBAR_HEIGHT: f32 = 22.0;
pub const CHROME_ICON_BUTTON_SIZE: f32 = 26.0;
pub const WINDOW_CONTROL_WIDTH: f32 = 34.0;
pub const CHROME_PAD_X: f32 = 6.0;
pub const CHROME_TEXT_SIZE: f32 = 10.5;
pub const CHROME_TITLE_SIZE: f32 = 11.5;

pub fn section_separator() -> impl gpui::IntoElement {
    div()
        .w(px(1.0))
        .h(px(18.0))
        .mx(px(3.0))
        .bg(Colors::panel_border())
}

pub fn chrome_button(
    icon_path: Option<&'static str>,
    fallback_text: &'static str,
    active: bool,
    color: Rgba,
) -> Div {
    let bg = if active {
        Colors::accent_muted()
    } else {
        gpui::transparent_black().into()
    };

    let mut button = div()
        .w(px(CHROME_ICON_BUTTON_SIZE))
        .h(px(CHROME_ICON_BUTTON_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(bg)
        .text_color(color)
        .hover(|style| {
            style
                .bg(Colors::surface_control_hover())
                .text_color(Colors::text_primary())
        });

    if let Some(path) = icon_path {
        button = button.child(svg().path(path).w(px(13.0)).h(px(13.0)).text_color(color));
    } else {
        button = button.child(fallback_text);
    }

    button
}

pub fn window_control_button(
    area: WindowControlArea,
    icon_path: &'static str,
    fallback_text: &'static str,
) -> Div {
    chrome_button(Some(icon_path), fallback_text, false, Colors::text_muted())
        .w(px(WINDOW_CONTROL_WIDTH))
        .h(px(TITLEBAR_HEIGHT))
        .rounded_none()
        .window_control_area(area)
        .occlude()
}

pub fn draggable_spacer() -> Div {
    div()
        .flex_1()
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx| {
            window.start_window_move();
        })
}

pub fn status_item(text: impl Into<String>, strong: bool) -> impl gpui::IntoElement {
    div()
        .h(px(18.0))
        .flex()
        .items_center()
        .px(px(6.0))
        .rounded_sm()
        .text_size(px(10.0))
        .font_weight(if strong {
            gpui::FontWeight::MEDIUM
        } else {
            gpui::FontWeight::NORMAL
        })
        .text_color(if strong {
            Colors::statusbar_text()
        } else {
            Colors::statusbar_text_muted()
        })
        .child(text.into())
}
