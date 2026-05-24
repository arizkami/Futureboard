//! Browser sidebar — left dock of the studio shell.
//!
//! The panel renders a real filesystem-backed TreeView. Root sections mirror
//! the WebUI Browser categories, while expanded folders are read lazily from
//! disk through `file_browser.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    div, px, svg, App, AppContext, Empty, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window,
};

use crate::assets;
use crate::components::file_browser::{BrowserNodeKind, BrowserVisibleNode, FileBrowserState};
use crate::theme::Colors;

pub const SIDEBAR_WIDTH: f32 = 272.0;
const TREE_ROW_HEIGHT: f32 = 26.0;
const TREE_INDENT: f32 = 14.0;

pub type ActivateFileCb = Arc<dyn Fn(&PathBuf, &mut Window, &mut App) + 'static>;
pub type SelectEntryCb = Arc<dyn Fn(&PathBuf, &mut Window, &mut App) + 'static>;
pub type ToggleNodeCb = Arc<dyn Fn(&(String, Option<PathBuf>), &mut Window, &mut App) + 'static>;

#[derive(Clone, Debug)]
pub struct BrowserDragItem {
    pub path: PathBuf,
    pub label: String,
}

pub struct BrowserDragPreview {
    label: String,
}

impl Render for BrowserDragPreview {
    fn render(&mut self, _w: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(5.0))
            .rounded_md()
            .border(px(1.0))
            .border_color(Colors::border_subtle())
            .bg(Colors::surface_raised())
            .shadow_lg()
            .child(
                svg()
                    .path(assets::ICON_FILE_PATH)
                    .w(px(12.0))
                    .h(px(12.0))
                    .text_color(Colors::status_success()),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(Colors::text_primary())
                    .child(self.label.clone()),
            )
    }
}

pub fn sidebar(
    state: &FileBrowserState,
    on_toggle: ToggleNodeCb,
    on_select: SelectEntryCb,
    on_activate_file: ActivateFileCb,
) -> impl IntoElement {
    let header = div()
        .px(px(10.0))
        .py(px(8.0))
        .border_b(px(1.0))
        .border_color(Colors::border_subtle())
        .child(
            div()
                .text_color(Colors::text_primary())
                .text_xs()
                .font_weight(gpui::FontWeight::BOLD)
                .child("Browser"),
        );

    let selected_label = state
        .selected
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "No file selected".to_string());

    let path_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(5.0))
        .border_b(px(1.0))
        .border_color(Colors::border_subtle())
        .child(
            div()
                .text_size(px(9.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(Colors::text_faint())
                .child("SEL"),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .truncate()
                .text_size(px(10.0))
                .text_color(Colors::text_muted())
                .child(truncate_path(&selected_label, 42)),
        );

    let nodes = state.visible_nodes();
    let rows: Vec<gpui::AnyElement> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            tree_row(
                i,
                node,
                on_toggle.clone(),
                on_select.clone(),
                on_activate_file.clone(),
            )
            .into_any_element()
        })
        .collect();

    let listing = div()
        .flex_1()
        .min_h_0()
        .id("browser-tree")
        .overflow_y_scroll()
        .flex_col()
        .px(px(4.0))
        .py(px(4.0))
        .children(rows);

    let error_banner = state.error.as_ref().map(|e| {
        div()
            .px(px(8.0))
            .py(px(4.0))
            .text_size(px(9.0))
            .text_color(Colors::status_error())
            .child(format!("Error: {}", e))
    });

    div()
        .flex()
        .flex_col()
        .w(px(SIDEBAR_WIDTH))
        .h_full()
        .bg(Colors::surface_panel())
        .border_r(px(1.0))
        .border_color(Colors::border_subtle())
        .child(header)
        .child(path_row)
        .children(error_banner)
        .child(listing)
}

fn tree_row(
    index: usize,
    node: &BrowserVisibleNode,
    on_toggle: ToggleNodeCb,
    on_select: SelectEntryCb,
    on_activate_file: ActivateFileCb,
) -> impl IntoElement {
    let id = node.id.clone();
    let path = node.path.clone();
    let path_for_select = node.path.clone();
    let path_for_activate = node.path.clone();
    let path_for_toggle = node.path.clone();
    let path_for_disclosure = node.path.clone();
    let label = node.label.clone();
    let expandable = node.expandable;
    let expanded = node.expanded;
    let selected = node.selected;
    let is_section = node.kind == BrowserNodeKind::Section;
    let is_folder = node.kind == BrowserNodeKind::Folder || node.kind == BrowserNodeKind::Section;
    let is_file = node.kind == BrowserNodeKind::File;
    let is_audio = node.is_audio();
    let is_midi = node.is_midi();
    let depth = node.depth as f32;

    let bg = if selected {
        Colors::accent_soft()
    } else {
        gpui::transparent_black().into()
    };

    let text_color = if selected {
        Colors::text_primary()
    } else if is_section {
        Colors::text_secondary()
    } else if is_audio || is_midi || is_folder {
        Colors::text_muted()
    } else {
        Colors::text_faint()
    };

    let icon_path = if is_folder {
        assets::ICON_FOLDER_PATH
    } else {
        assets::ICON_FILE_PATH
    };

    let icon_color = if selected {
        Colors::accent_primary()
    } else if is_section {
        Colors::accent_primary()
    } else if is_folder {
        Colors::text_muted()
    } else if is_audio {
        Colors::status_success()
    } else if is_midi {
        Colors::status_warning()
    } else {
        Colors::text_faint()
    };

    let disclosure_id = id.clone();
    let disclosure_toggle = on_toggle.clone();
    let mut disclosure = div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(12.0))
        .h_full()
        .rounded_sm()
        .id(("browser-disclosure", index))
        .child(disclosure_icon(expandable, expanded));
    if expandable {
        disclosure = disclosure
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(|s| s.bg(Colors::surface_hover()))
            .on_click(move |_, w, cx| {
                disclosure_toggle(&(disclosure_id.clone(), path_for_disclosure.clone()), w, cx);
            });
    }

    let mut row = div()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .h(px(TREE_ROW_HEIGHT))
        .w_full()
        .gap(px(4.0))
        .pl(px(6.0 + depth * TREE_INDENT))
        .pr(px(6.0))
        .rounded_sm()
        .bg(bg)
        .id(("browser-tree-row", index))
        .cursor(gpui::CursorStyle::PointingHand)
        .hover(|s| s.bg(Colors::surface_hover()))
        .child(if selected {
            div()
                .absolute()
                .left(px(0.0))
                .top(px(4.0))
                .bottom(px(4.0))
                .w(px(2.0))
                .rounded_full()
                .bg(Colors::accent_primary())
                .into_any_element()
        } else {
            Empty.into_any_element()
        })
        .child(disclosure)
        .child(
            svg()
                .path(icon_path)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(icon_color),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .truncate()
                .text_size(px(11.0))
                .font_weight(if is_section {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(text_color)
                .child(label.clone()),
        )
        .children(node.error.as_ref().map(|_| {
            div()
                .text_size(px(9.0))
                .text_color(Colors::status_error())
                .child("unavailable")
        }));

    row = row.on_mouse_down(gpui::MouseButton::Left, move |_, w, cx| {
        if let Some(path) = path_for_select.as_ref() {
            on_select(path, w, cx);
        }
    });

    row = row.on_click(move |event, w, cx| {
        if expandable {
            if event.click_count() >= 2 || is_section {
                on_toggle(&(id.clone(), path_for_toggle.clone()), w, cx);
            }
        } else if is_file && event.click_count() >= 2 && (is_audio || is_midi) {
            if let Some(path) = path_for_activate.as_ref() {
                on_activate_file(path, w, cx);
            }
        }
    });

    if is_audio {
        let drag_label = label.clone();
        if let Some(path) = path {
            row = row.on_drag(
                BrowserDragItem {
                    path,
                    label: drag_label,
                },
                |drag, _offset, _window, cx| {
                    cx.new(|_| BrowserDragPreview {
                        label: drag.label.clone(),
                    })
                },
            );
        }
    }

    row
}

fn disclosure_icon(expandable: bool, expanded: bool) -> impl IntoElement {
    if expandable {
        let icon_path = if expanded {
            assets::ICON_CHEVRON_DOWN_PATH
        } else {
            assets::ICON_CHEVRON_RIGHT_PATH
        };
        svg()
            .path(icon_path)
            .w(px(10.0))
            .h(px(10.0))
            .text_color(Colors::text_faint())
            .into_any_element()
    } else {
        div().w(px(10.0)).h(px(10.0)).into_any_element()
    }
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let tail = &s[s.len().saturating_sub(max - 1)..];
        format!("...{}", tail)
    }
}
