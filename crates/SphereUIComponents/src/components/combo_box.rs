use std::sync::Arc;

use gpui::{
    div, px, svg, App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, Window,
};

use crate::assets;
use crate::theme::Colors;

#[derive(Clone, Copy)]
pub struct ComboBoxOption<T: Copy + PartialEq + 'static> {
    pub label: &'static str,
    pub value: T,
}

pub fn combo_box_trigger(
    id: impl Into<gpui::ElementId>,
    label: impl Into<String>,
    open: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(30.0))
        .w_full()
        .min_w(px(0.0))
        .rounded_md()
        .border(px(1.0))
        .border_color(if open {
            Colors::border_focus()
        } else {
            Colors::border_subtle()
        })
        .bg(if open {
            Colors::surface_card()
        } else {
            Colors::surface_input()
        })
        .px(px(9.0))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .cursor(gpui::CursorStyle::PointingHand)
        .hover(|s| {
            s.bg(Colors::surface_control_hover())
                .border_color(Colors::border_strong())
        })
        .on_click(on_click)
        .child(
            div()
                .min_w(px(0.0))
                .flex_1()
                .truncate()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(Colors::text_primary())
                .child(label.into()),
        )
        .child(
            svg()
                .path(assets::ICON_CHEVRON_DOWN_PATH)
                .w(px(11.0))
                .h(px(11.0))
                .text_color(Colors::text_faint()),
        )
}

pub fn combo_box_menu<T: Copy + PartialEq + 'static>(
    id: impl Into<gpui::ElementId>,
    left: f32,
    top: f32,
    width: f32,
    selected: T,
    options: &'static [ComboBoxOption<T>],
    on_select: Arc<dyn Fn(T, &mut Window, &mut App) + 'static>,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(width))
        .rounded_md()
        .border(px(1.0))
        .border_color(Colors::border_subtle())
        .bg(Colors::surface_card())
        .shadow(vec![gpui::BoxShadow {
            color: gpui::rgba(0x00000080).into(),
            offset: gpui::point(px(0.0), px(10.0)),
            blur_radius: px(28.0),
            spread_radius: px(0.0),
        }])
        .p(px(4.0))
        .id(id)
        .occlude()
        .children(options.iter().enumerate().map(|(index, option)| {
            let active = option.value == selected;
            let value = option.value;
            let on_select = on_select.clone();
            div()
                .id(("combo-box-option", index))
                .h(px(25.0))
                .w_full()
                .rounded_md()
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .bg(if active {
                    Colors::accent_muted()
                } else {
                    gpui::transparent_black().into()
                })
                .text_size(px(10.5))
                .font_weight(if active {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(if active {
                    Colors::text_primary()
                } else {
                    Colors::text_secondary()
                })
                .cursor(gpui::CursorStyle::PointingHand)
                .hover(|s| s.bg(Colors::surface_control_hover()))
                .on_click(move |_, window, cx| on_select(value, window, cx))
                .child(option.label)
                .children(active.then(|| {
                    svg()
                        .path(assets::ICON_CHECK_PATH)
                        .w(px(11.0))
                        .h(px(11.0))
                        .text_color(Colors::accent_primary())
                }))
        }))
}
