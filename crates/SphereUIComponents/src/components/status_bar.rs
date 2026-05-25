use crate::components::title_bar::{status_item, STATUSBAR_HEIGHT};
use crate::theme::Colors;
use gpui::{div, px, IntoElement, ParentElement, Styled};

pub fn status_bar(left: impl Into<String>, right: impl Into<String>) -> impl IntoElement {
    let left = left.into();
    let right = right.into();
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .h(px(STATUSBAR_HEIGHT))
        .px(px(6.0))
        .gap(px(8.0))
        .bg(Colors::surface_titlebar())
        .border_t(px(1.0))
        .border_color(Colors::border_subtle())
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .child(status_item(left, true)),
        )
        .child(
            div()
                .flex_none()
                .overflow_hidden()
                .child(status_item(right, false)),
        )
}
