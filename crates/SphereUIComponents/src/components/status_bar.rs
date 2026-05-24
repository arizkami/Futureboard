use crate::theme::Colors;
use gpui::{div, px, IntoElement, ParentElement, Styled};

pub fn status_bar(left: impl Into<String>, right: impl Into<String>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(22.0))
        .px(px(10.0))
        .bg(Colors::surface_panel())
        .border_t(px(1.0))
        .border_color(Colors::border_subtle())
        .child(
            div()
                .flex_1()
                .text_color(Colors::text_muted())
                .text_size(px(10.5))
                .child(left.into()),
        )
        .child(
            div()
                .text_color(Colors::text_muted())
                .text_size(px(10.5))
                .child(right.into()),
        )
}
