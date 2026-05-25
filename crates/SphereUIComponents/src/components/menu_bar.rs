use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, Window,
};

use crate::menu::MenuManifest;
use crate::theme::Colors;

use super::title_bar::{CHROME_TEXT_SIZE, TITLEBAR_HEIGHT};

pub type MenuOpenCb = Arc<dyn Fn(&(String, f32), &mut Window, &mut App) + 'static>;

pub fn menu_bar(open_menu_id: Option<&str>, on_open_menu: MenuOpenCb) -> impl IntoElement {
    let manifest = MenuManifest::load();
    let open_id_owned = open_menu_id.map(|s| s.to_string());

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(1.0))
        .h(px(TITLEBAR_HEIGHT))
        .px(px(4.0))
        .children(manifest.menus.iter().enumerate().map(|(i, menu)| {
            let is_open = open_id_owned.as_deref() == Some(menu.id.as_str());
            let menu_id = menu.id.clone();
            let hover_menu_id = menu.id.clone();
            let cb = on_open_menu.clone();
            let hover_cb = on_open_menu.clone();
            let can_hover_switch = open_id_owned.is_some() && !is_open;

            menu_label_button(
                ("top-menu", i),
                menu.label.clone(),
                is_open,
                can_hover_switch,
                move |hovered, window, cx| {
                    if *hovered {
                        let x: f32 = window.mouse_position().x.into();
                        hover_cb(&(hover_menu_id.clone(), x), window, cx);
                    }
                },
                move |event, window, cx| {
                    let click_x: f32 = event.position.x.into();
                    cb(&(menu_id.clone(), click_x), window, cx);
                },
            )
        }))
}

pub fn menu_label_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<String>,
    active: bool,
    enable_hover_switch: bool,
    on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_mouse_down: impl Fn(&gpui::MouseDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(24.0))
        .px(px(7.0))
        .flex()
        .items_center()
        .rounded_md()
        .text_size(px(CHROME_TEXT_SIZE))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if active {
            Colors::text_primary()
        } else {
            Colors::text_muted()
        })
        .bg(if active {
            Colors::surface_control_hover()
        } else {
            gpui::transparent_black().into()
        })
        .hover(|s| {
            s.bg(Colors::surface_control_hover())
                .text_color(Colors::text_primary())
        })
        .cursor(gpui::CursorStyle::PointingHand)
        .when(enable_hover_switch, |this| this.on_hover(on_hover))
        .on_mouse_down(gpui::MouseButton::Left, on_mouse_down)
        .occlude()
        .child(label.into())
}
