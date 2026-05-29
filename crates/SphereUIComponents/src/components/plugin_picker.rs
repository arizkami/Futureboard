//! PluginPicker — DAW-style insert plugin browser overlay.
//!
//! Layout: header / search / body (sidebar + virtualized list) / footer with
//! Add button. The overlay reads from the cached plug-in index passed in by
//! [`crate::layout::StudioLayout`]; it never scans, instantiates, or reads
//! plug-in binaries. Picking a plug-in routes through `apply_picked_insert`
//! which adds the insert slot and flips the engine project dirty flag — the
//! picker itself does **no** audio engine work.

use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, svg, uniform_list, App, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window,
};

use crate::assets;
use crate::components::controls::{fb_button, FbButtonKind};
use crate::components::text_input::{text_field_with_callbacks, TextInputCallbacks, TextInputState};
use crate::theme::Colors;
use sphere_plugin_host::{PluginFormat, PluginKind, PluginStatus, RegistryPlugin};

type VoidCb = Arc<dyn Fn(&(), &mut Window, &mut App) + 'static>;
type StringCb = Arc<dyn Fn(&String, &mut Window, &mut App) + 'static>;
type FilterCb = Arc<dyn Fn(&PickerFilter, &mut Window, &mut App) + 'static>;

/// Special plugin id used to insert the documented stub effect when the
/// registry has no insert-capable plugin. Keeps the project round-trip
/// exercisable on a clean dev box. Mirrors the Phase 2a fallback id.
pub const STUB_PLUGIN_ID: &str = "futureboard.stub.gain";

/// Sentinel category meaning "no category filter". Preserved for legacy call
/// sites that still pass plain category strings via [`PluginPickerCallbacks`].
pub const CATEGORY_ALL: &str = "All";

const ROW_HEIGHT: f32 = 36.0;
const SIDEBAR_WIDTH: f32 = 168.0;
const MODAL_WIDTH: f32 = 740.0;
const MODAL_HEIGHT: f32 = 520.0;

/// Sidebar filter rail. Matches a typical DAW plug-in browser: library
/// groupings + format chips + a small dynamic vendors/categories tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerFilter {
    All,
    Favorites,
    Instruments,
    Effects,
    Format(PluginFormat),
    Vendor(String),
    Category(String),
}

impl PickerFilter {
    fn matches(&self, plugin: &RegistryPlugin) -> bool {
        match self {
            PickerFilter::All => true,
            // Favorites are not persisted yet — show nothing rather than lie.
            PickerFilter::Favorites => false,
            PickerFilter::Instruments => plugin.kind == PluginKind::Instrument,
            PickerFilter::Effects => plugin.kind == PluginKind::Effect,
            PickerFilter::Format(fmt) => plugin.format == *fmt,
            PickerFilter::Vendor(v) => plugin.vendor.eq_ignore_ascii_case(v),
            PickerFilter::Category(c) => plugin.display_category().eq_ignore_ascii_case(c),
        }
    }

    #[allow(dead_code)]
    fn label(&self) -> String {
        match self {
            PickerFilter::All => "All".to_string(),
            PickerFilter::Favorites => "Favorites".to_string(),
            PickerFilter::Instruments => "Instruments".to_string(),
            PickerFilter::Effects => "Effects".to_string(),
            PickerFilter::Format(fmt) => fmt.label().to_string(),
            PickerFilter::Vendor(v) => v.clone(),
            PickerFilter::Category(c) => c.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginPickerState {
    pub is_open: bool,
    /// Track that receives the new insert when a plugin is committed.
    pub track_id: String,
    /// Active sidebar filter.
    pub filter: PickerFilter,
    /// Current search query (kept in sync with the search input).
    pub query: String,
    /// Currently highlighted plug-in row (single click). `None` until the user
    /// touches the list. Double-click / Enter / Add commits this id.
    pub selected_id: Option<String>,
}

impl PluginPickerState {
    pub fn closed() -> Self {
        Self {
            is_open: false,
            track_id: String::new(),
            filter: PickerFilter::All,
            query: String::new(),
            selected_id: None,
        }
    }

    pub fn open_for(track_id: &str) -> Self {
        Self {
            is_open: true,
            track_id: track_id.to_string(),
            filter: PickerFilter::All,
            query: String::new(),
            selected_id: None,
        }
    }
}

#[derive(Clone)]
pub struct PluginPickerCallbacks {
    /// Dismiss without inserting.
    pub on_close: VoidCb,
    /// Single-click row — highlights but does not commit.
    pub on_select: StringCb,
    /// Commit a plug-in by id (`Add` button, double-click, Enter).
    pub on_pick: StringCb,
    /// Sidebar filter change.
    pub on_select_filter: FilterCb,
    /// Re-run the cached SQLite load (used by the Retry button).
    pub on_retry_load: VoidCb,
    /// Open the Plugin Manager external window.
    pub on_open_plugin_manager: VoidCb,
    /// Drop the SQLite catalog and trigger a fresh scan.
    pub on_rebuild_database: VoidCb,
}

/// Loading / error state for the cached catalog, distinct from `plugins.len()`.
/// This drives the picker's body — Loading shows skeleton rows, MissingDatabase
/// / Error show actionable messages, Ready hands off to the virtualized list.
#[derive(Debug, Clone)]
pub enum CatalogStatus {
    /// Background SQLite load still in flight.
    Loading,
    /// Loaded successfully (may still be empty if the user never scanned).
    Ready,
    /// `index.dat` does not exist on disk yet — invite the user to scan.
    MissingDatabase,
    /// SQLite open/read failed; the picker shows the error with Retry +
    /// Open Plugin Manager + Rebuild Database buttons.
    Error(String),
}

/// Compatibility alias for older call sites.
#[allow(dead_code)]
pub type PluginPickerLoadState = CatalogStatus;

fn icon(path: &'static str, size: f32, color: gpui::Rgba) -> impl IntoElement {
    svg().path(path).w(px(size)).h(px(size)).text_color(color)
}

/// Precomputed lowercased haystack (name + vendor + category + format) for
/// substring search. Built once per render and passed to the index filter.
fn searchable_text(plugin: &RegistryPlugin) -> String {
    let category = plugin.display_category();
    format!(
        "{} {} {} {}",
        plugin.name, plugin.vendor, category, plugin.format.label()
    )
    .to_lowercase()
}

/// Apply the active sidebar filter + query against precomputed haystacks.
/// Returns indices into `plugins` so callers hand only the filtered view to
/// [`uniform_list`].
fn apply_filter_indices(
    plugins: &[RegistryPlugin],
    haystacks: &[String],
    filter: &PickerFilter,
    query: &str,
) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    let mut out = Vec::with_capacity(plugins.len().min(64));
    for (idx, plugin) in plugins.iter().enumerate() {
        if !filter.matches(plugin) {
            continue;
        }
        if !q.is_empty() && !haystacks[idx].contains(&q) {
            continue;
        }
        out.push(idx);
    }
    out
}

/// Unique sorted vendor list from the catalog. Capped to keep the sidebar
/// tail readable — most users want format/kind filters anyway.
fn vendor_list(plugins: &[RegistryPlugin], limit: usize) -> Vec<String> {
    let mut vendors: Vec<String> = plugins
        .iter()
        .map(|p| p.vendor.clone())
        .filter(|v| !v.is_empty())
        .collect();
    vendors.sort();
    vendors.dedup();
    vendors.truncate(limit);
    vendors
}

fn category_list(plugins: &[RegistryPlugin], limit: usize) -> Vec<String> {
    let mut cats: Vec<String> = plugins
        .iter()
        .map(|p| p.display_category())
        .filter(|c| !c.is_empty())
        .collect();
    cats.sort();
    cats.dedup();
    cats.truncate(limit);
    cats
}

fn count_with<F: Fn(&RegistryPlugin) -> bool>(plugins: &[RegistryPlugin], pred: F) -> usize {
    plugins.iter().filter(|p| pred(p)).count()
}

fn format_badge(fmt: PluginFormat) -> impl IntoElement {
    let (fg, bg, border) = match fmt {
        PluginFormat::Vst3 => (
            Colors::accent_primary(),
            Colors::accent_muted(),
            Colors::border_accent(),
        ),
        PluginFormat::Clap => (
            Colors::status_success(),
            gpui::rgba(0x6FCF9720),
            Colors::status_success(),
        ),
        PluginFormat::Au => (
            Colors::status_warning(),
            gpui::rgba(0xE5C07B18),
            Colors::status_warning(),
        ),
        _ => (
            Colors::text_faint(),
            Colors::surface_input(),
            Colors::border_subtle(),
        ),
    };
    div()
        .px(px(5.0))
        .py(px(1.0))
        .rounded_sm()
        .border(px(1.0))
        .border_color(border)
        .bg(bg)
        .text_size(px(9.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(fg)
        .child(fmt.label())
}

fn status_badge(label: &'static str, tone_warn: bool) -> impl IntoElement {
    let (fg, bg) = if tone_warn {
        (Colors::status_warning(), gpui::rgba(0xE5C07B14))
    } else {
        (Colors::text_faint(), Colors::surface_input())
    };
    div()
        .px(px(5.0))
        .py(px(1.0))
        .rounded_sm()
        .border(px(1.0))
        .border_color(Colors::border_subtle())
        .bg(bg)
        .text_size(px(9.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(fg)
        .child(label)
}

fn plugin_row(
    index: usize,
    plugin: &RegistryPlugin,
    selected: bool,
    on_select: StringCb,
    on_pick: StringCb,
) -> impl IntoElement {
    let id_select = plugin.id.clone();
    let id_pick = plugin.id.clone();
    let name = plugin.name.clone();
    let vendor = plugin.vendor.clone();
    let category = plugin.display_category();
    let fmt = plugin.format;
    let kind_icon = match plugin.kind {
        PluginKind::Instrument => assets::ICON_MUSIC_PATH,
        PluginKind::Effect => assets::ICON_SLIDERS_HORIZONTAL_PATH,
    };
    let kind_color = match plugin.kind {
        PluginKind::Instrument => Colors::accent_primary(),
        PluginKind::Effect => Colors::status_success(),
    };
    let metadata_only = plugin.status == PluginStatus::MissingPreset;

    div()
        .id(("plugin-picker-row", index))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .h(px(ROW_HEIGHT))
        .px(px(10.0))
        .border_b(px(1.0))
        .border_color(Colors::divider())
        .when(selected, |el| el.bg(Colors::accent_muted()))
        .when(!selected, |el| el.hover(|s| s.bg(Colors::surface_hover())))
        .cursor(gpui::CursorStyle::PointingHand)
        .on_click(move |event, window, cx| {
            if event.click_count() >= 2 {
                on_pick(&id_pick, window, cx);
            } else {
                on_select(&id_select, window, cx);
            }
        })
        .child(icon(kind_icon, 12.0, kind_color))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(Colors::text_primary())
                .truncate()
                .child(name),
        )
        .child(
            div()
                .w(px(140.0))
                .text_size(px(10.5))
                .text_color(Colors::text_dim())
                .truncate()
                .child(vendor),
        )
        .child(
            div()
                .w(px(110.0))
                .text_size(px(10.5))
                .text_color(Colors::text_dim())
                .truncate()
                .child(category),
        )
        .child(format_badge(fmt))
        .when(metadata_only, |el| {
            el.child(status_badge("Metadata", true))
        })
}

fn sidebar_item(
    id: impl Into<gpui::ElementId>,
    label: String,
    count: Option<usize>,
    active: bool,
    cb: FilterCb,
    value: PickerFilter,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .w_full()
        .px(px(8.0))
        .py(px(4.0))
        .rounded_md()
        .when(active, |el| el.bg(Colors::accent_muted()))
        .when(!active, |el| {
            el.hover(|s| s.bg(Colors::surface_control_hover()))
        })
        .cursor(gpui::CursorStyle::PointingHand)
        .on_click(move |_, window, cx| cb(&value, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(11.0))
                .text_color(if active {
                    Colors::accent_primary()
                } else {
                    Colors::text_dim()
                })
                .truncate()
                .child(label),
        )
        .when_some(count, |el, n| {
            el.child(
                div()
                    .text_size(px(10.0))
                    .text_color(if active {
                        Colors::accent_primary()
                    } else {
                        Colors::text_faint()
                    })
                    .child(format!("{n}")),
            )
        })
}

fn sidebar_section_label(label: &'static str) -> impl IntoElement {
    div()
        .px(px(10.0))
        .pt(px(8.0))
        .pb(px(2.0))
        .text_size(px(9.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(Colors::text_faint())
        .child(label)
}

/// Full message + hint for empty/error states.
fn message_view(title: String, hint: Option<String>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .py(px(40.0))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(Colors::text_secondary())
                .child(title),
        )
        .when_some(hint, |el, h| {
            el.child(
                div()
                    .text_size(px(10.5))
                    .text_color(Colors::text_faint())
                    .child(h),
            )
        })
}

/// Single placeholder row — uses theme tokens only. Skeletons are static (no
/// per-frame animation) so the picker stays cheap while loading; the user sees
/// instant layout instead of a blank panel.
fn skeleton_row(index: usize) -> impl IntoElement {
    let block = |w: f32, alpha: f32| {
        div()
            .h(px(10.0))
            .w(px(w))
            .rounded_sm()
            .bg(Colors::with_alpha(Colors::text_primary(), alpha))
    };
    let alpha = 0.06 + ((index % 3) as f32) * 0.015;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .h(px(ROW_HEIGHT))
        .px(px(10.0))
        .border_b(px(1.0))
        .border_color(Colors::divider())
        .child(
            div()
                .w(px(12.0))
                .h(px(12.0))
                .rounded_sm()
                .bg(Colors::with_alpha(Colors::text_primary(), alpha)),
        )
        .child(div().flex_1().min_w(px(0.0)).child(block(140.0 + (index % 4) as f32 * 20.0, alpha)))
        .child(div().w(px(140.0)).child(block(110.0, alpha)))
        .child(div().w(px(110.0)).child(block(80.0, alpha)))
        .child(div().w(px(54.0)).child(block(36.0, alpha)))
}

fn skeleton_body() -> impl IntoElement {
    let mut col = div().flex().flex_col();
    for i in 0..14 {
        col = col.child(skeleton_row(i));
    }
    col
}

/// Action button row shown under the `Error` state body.
fn recovery_actions(
    on_retry: VoidCb,
    on_open_manager: VoidCb,
    on_rebuild: VoidCb,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap(px(8.0))
        .pt(px(12.0))
        .child(fb_button(
            "plugin-picker-retry",
            "Retry",
            FbButtonKind::Primary,
            true,
            move |_, window, cx| on_retry(&(), window, cx),
        ))
        .child(fb_button(
            "plugin-picker-open-mgr",
            "Open Plugin Manager",
            FbButtonKind::Default,
            true,
            move |_, window, cx| on_open_manager(&(), window, cx),
        ))
        .child(fb_button(
            "plugin-picker-rebuild",
            "Rebuild Database",
            FbButtonKind::Default,
            true,
            move |_, window, cx| on_rebuild(&(), window, cx),
        ))
}

/// Full-screen modal overlay. Render last so it sits above the mixer.
#[allow(clippy::too_many_arguments)]
pub fn plugin_picker_overlay(
    state: &PluginPickerState,
    plugins: &[RegistryPlugin],
    catalog_status: CatalogStatus,
    search_input: &TextInputState,
    search_focused: bool,
    search_callbacks: TextInputCallbacks,
    callbacks: PluginPickerCallbacks,
) -> impl IntoElement {
    let close_backdrop = callbacks.on_close.clone();
    let close_button = callbacks.on_close.clone();
    let on_pick_add = callbacks.on_pick.clone();
    let on_pick_stub = callbacks.on_pick.clone();
    let filter_cb = callbacks.on_select_filter.clone();

    let debug = std::env::var_os("FUTUREBOARD_PLUGIN_PICKER_DEBUG").is_some();
    let render_started = std::time::Instant::now();
    let is_loading = matches!(catalog_status, CatalogStatus::Loading);

    // ---- Precompute haystacks + filter. O(n) per render but bounded by the
    // catalog size; cheaper than a per-row format!()+to_lowercase().
    let haystacks: Vec<String> = plugins.iter().map(searchable_text).collect();
    let filtered_indices = apply_filter_indices(plugins, &haystacks, &state.filter, &state.query);
    let total = plugins.len();
    let visible_count = filtered_indices.len();

    // Snapshot only the rows the list actually needs — uniform_list will read
    // these by index but only materialize the visible range.
    let visible_plugins: Arc<Vec<RegistryPlugin>> = Arc::new(
        filtered_indices
            .iter()
            .map(|&i| plugins[i].clone())
            .collect(),
    );

    let vendors = vendor_list(plugins, 32);
    let categories = category_list(plugins, 32);

    let instrument_count = count_with(plugins, |p| p.kind == PluginKind::Instrument);
    let effect_count = count_with(plugins, |p| p.kind == PluginKind::Effect);
    let vst3_count = count_with(plugins, |p| p.format == PluginFormat::Vst3);
    let clap_count = count_with(plugins, |p| p.format == PluginFormat::Clap);
    let au_count = count_with(plugins, |p| p.format == PluginFormat::Au);

    let selected = state
        .selected_id
        .as_deref()
        .and_then(|id| plugins.iter().find(|p| p.id == id));

    if debug {
        let reason = if visible_count == 0 {
            match &catalog_status {
                CatalogStatus::MissingDatabase => "no database",
                CatalogStatus::Loading => "catalog loading",
                CatalogStatus::Error(_) => "load error",
                CatalogStatus::Ready if total == 0 => "catalog empty",
                CatalogStatus::Ready if !state.query.is_empty() => "query eliminated all",
                CatalogStatus::Ready => "filter eliminated all",
            }
        } else {
            "ok"
        };
        eprintln!(
            "[plugin-picker] render state={:?} total={} visible={} query=\"{}\" filter={:?} vendors={} categories={} reason={} render_ms={}",
            catalog_status,
            total,
            visible_count,
            state.query,
            state.filter,
            vendors.len(),
            categories.len(),
            reason,
            render_started.elapsed().as_millis(),
        );
    }

    // ---- Sidebar
    let sidebar = {
        let mut col = div().flex().flex_col().w(px(SIDEBAR_WIDTH)).py(px(4.0));

        col = col.child(sidebar_section_label("Library"));
        col = col.child(div().px(px(4.0)).child(sidebar_item(
            "pp-filter-all",
            "All".to_string(),
            Some(total),
            state.filter == PickerFilter::All,
            filter_cb.clone(),
            PickerFilter::All,
        )));
        col = col.child(div().px(px(4.0)).child(sidebar_item(
            "pp-filter-fav",
            "Favorites".to_string(),
            None,
            state.filter == PickerFilter::Favorites,
            filter_cb.clone(),
            PickerFilter::Favorites,
        )));

        col = col.child(sidebar_section_label("Kind"));
        col = col.child(div().px(px(4.0)).child(sidebar_item(
            "pp-filter-inst",
            "Instruments".to_string(),
            Some(instrument_count),
            state.filter == PickerFilter::Instruments,
            filter_cb.clone(),
            PickerFilter::Instruments,
        )));
        col = col.child(div().px(px(4.0)).child(sidebar_item(
            "pp-filter-fx",
            "Effects".to_string(),
            Some(effect_count),
            state.filter == PickerFilter::Effects,
            filter_cb.clone(),
            PickerFilter::Effects,
        )));

        col = col.child(sidebar_section_label("Format"));
        col = col.child(div().px(px(4.0)).child(sidebar_item(
            "pp-filter-vst3",
            "VST3".to_string(),
            Some(vst3_count),
            state.filter == PickerFilter::Format(PluginFormat::Vst3),
            filter_cb.clone(),
            PickerFilter::Format(PluginFormat::Vst3),
        )));
        col = col.child(div().px(px(4.0)).child(sidebar_item(
            "pp-filter-clap",
            "CLAP".to_string(),
            Some(clap_count),
            state.filter == PickerFilter::Format(PluginFormat::Clap),
            filter_cb.clone(),
            PickerFilter::Format(PluginFormat::Clap),
        )));
        if au_count > 0 || cfg!(target_os = "macos") {
            col = col.child(div().px(px(4.0)).child(sidebar_item(
                "pp-filter-au",
                "AU".to_string(),
                Some(au_count),
                state.filter == PickerFilter::Format(PluginFormat::Au),
                filter_cb.clone(),
                PickerFilter::Format(PluginFormat::Au),
            )));
        }

        if !vendors.is_empty() {
            col = col.child(sidebar_section_label("Vendors"));
            for (i, v) in vendors.iter().enumerate() {
                let active = matches!(&state.filter, PickerFilter::Vendor(name) if name.eq_ignore_ascii_case(v));
                col = col.child(div().px(px(4.0)).child(sidebar_item(
                    ("pp-filter-vendor", i),
                    v.clone(),
                    None,
                    active,
                    filter_cb.clone(),
                    PickerFilter::Vendor(v.clone()),
                )));
            }
        }

        if !categories.is_empty() {
            col = col.child(sidebar_section_label("Categories"));
            for (i, c) in categories.iter().enumerate() {
                let active = matches!(&state.filter, PickerFilter::Category(name) if name.eq_ignore_ascii_case(c));
                col = col.child(div().px(px(4.0)).child(sidebar_item(
                    ("pp-filter-cat", i),
                    c.clone(),
                    None,
                    active,
                    filter_cb.clone(),
                    PickerFilter::Category(c.clone()),
                )));
            }
        }

        div()
            .flex()
            .flex_col()
            .w(px(SIDEBAR_WIDTH))
            .min_w(px(SIDEBAR_WIDTH))
            .border_r(px(1.0))
            .border_color(Colors::divider())
            .bg(Colors::surface_panel_alt())
            .child(
                div()
                    .id("plugin-picker-sidebar-scroll")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(col),
            )
    };

    // ---- Main list area
    let list_body: gpui::AnyElement = if is_loading {
        // Skeleton instead of a blank panel while SQLite read runs on the
        // background executor. The shell already painted in <100 ms; this is
        // visual continuity, not real data.
        skeleton_body().into_any_element()
    } else if visible_count > 0 {
        let on_select_cb = callbacks.on_select.clone();
        let on_pick_cb = callbacks.on_pick.clone();
        let rows = visible_plugins.clone();
        let selected_id = state.selected_id.clone();
        uniform_list(
            "plugin-picker-list",
            visible_count,
            move |range, _window, _cx| {
                let on_select = on_select_cb.clone();
                let on_pick = on_pick_cb.clone();
                let rows = rows.clone();
                let selected_id = selected_id.clone();
                range
                    .map(|i| {
                        let p = &rows[i];
                        let is_sel = selected_id.as_deref() == Some(p.id.as_str());
                        plugin_row(i, p, is_sel, on_select.clone(), on_pick.clone())
                            .into_any_element()
                    })
                    .collect::<Vec<_>>()
            },
        )
        .size_full()
        .into_any_element()
    } else {
        // Distinct empty/error states — no silent blank panel.
        match &catalog_status {
            CatalogStatus::Loading => skeleton_body().into_any_element(),
            CatalogStatus::MissingDatabase => message_view(
                "No plugin database found.".to_string(),
                Some("Open Plugin Manager and click Scan Now.".to_string()),
            )
            .into_any_element(),
            CatalogStatus::Error(err) => div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .py(px(28.0))
                .child(message_view(
                    "Failed to load plugin database.".to_string(),
                    Some(err.clone()),
                ))
                .child(recovery_actions(
                    callbacks.on_retry_load.clone(),
                    callbacks.on_open_plugin_manager.clone(),
                    callbacks.on_rebuild_database.clone(),
                ))
                .into_any_element(),
            CatalogStatus::Ready if total == 0 => message_view(
                "No plugins found.".to_string(),
                Some("Scan plugins in Plugin Manager.".to_string()),
            )
            .into_any_element(),
            CatalogStatus::Ready if !state.query.is_empty() => message_view(
                "No plugins match this search.".to_string(),
                None,
            )
            .into_any_element(),
            CatalogStatus::Ready => message_view(
                "No plugins in this filter.".to_string(),
                Some("Pick a different sidebar entry or insert the stub.".to_string()),
            )
            .into_any_element(),
        }
    };

    let list_section = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.0))
        .child(
            // Column headers
            div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(26.0))
                .px(px(10.0))
                .border_b(px(1.0))
                .border_color(Colors::divider())
                .bg(Colors::surface_input())
                .gap(px(8.0))
                .text_size(px(9.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(Colors::text_faint())
                .child(div().w(px(12.0)))
                .child(div().flex_1().min_w(px(0.0)).child("Plug-in"))
                .child(div().w(px(140.0)).child("Vendor"))
                .child(div().w(px(110.0)).child("Category"))
                .child(div().w(px(54.0)).child("Format")),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(0.0))
                .child(list_body),
        );

    // ---- Footer
    let footer_label = if let Some(p) = selected {
        format!("{} · {} · {}", p.name, p.vendor, p.format.label())
    } else if is_loading {
        "Loading plugin index…".to_string()
    } else if visible_count == 0 {
        match &catalog_status {
            CatalogStatus::Loading => "Loading plugin index…".to_string(),
            CatalogStatus::MissingDatabase => "Open Plugin Manager → Scan Now".to_string(),
            CatalogStatus::Error(_) => "Database error".to_string(),
            CatalogStatus::Ready if total == 0 => "Catalog is empty".to_string(),
            CatalogStatus::Ready => "Adjust filter or search".to_string(),
        }
    } else {
        format!("{visible_count} of {total} plug-in(s)")
    };

    let can_add = selected.is_some();
    let selected_id_for_add = state.selected_id.clone();

    let footer = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .h(px(40.0))
        .px(px(12.0))
        .border_t(px(1.0))
        .border_color(Colors::divider())
        .bg(Colors::surface_panel_alt())
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(10.5))
                .text_color(Colors::text_dim())
                .truncate()
                .child(footer_label),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(fb_button(
                    "plugin-picker-stub",
                    "Insert Stub",
                    FbButtonKind::Default,
                    true,
                    move |_, window, cx| on_pick_stub(&STUB_PLUGIN_ID.to_string(), window, cx),
                ))
                .child(fb_button(
                    "plugin-picker-add",
                    "Add",
                    FbButtonKind::Primary,
                    can_add,
                    move |_, window, cx| {
                        if let Some(id) = selected_id_for_add.clone() {
                            on_pick_add(&id, window, cx);
                        }
                    },
                )),
        );

    div()
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0()
        .flex()
        .items_start()
        .justify_center()
        .pt(px(64.0))
        .px(px(18.0))
        .pb(px(32.0))
        .id("plugin-picker-overlay")
        .bg(gpui::transparent_black())
        .occlude()
        .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            close_backdrop(&(), window, cx);
        })
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(MODAL_WIDTH))
                .max_w(px(MODAL_WIDTH))
                .h(px(MODAL_HEIGHT))
                .max_h(px(MODAL_HEIGHT))
                .overflow_hidden()
                .rounded_xl()
                .border(px(1.0))
                .border_color(Colors::border_default())
                .bg(Colors::surface_window())
                .shadow_xl()
                .on_mouse_down(gpui::MouseButton::Left, |_, _window, cx| {
                    cx.stop_propagation();
                })
                // Titlebar
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .h(px(36.0))
                        .px(px(14.0))
                        .border_b(px(1.0))
                        .border_color(Colors::divider())
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(icon(assets::ICON_CPU_PATH, 13.0, Colors::accent_primary()))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(Colors::text_primary())
                                        .child("Add Insert"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(22.0))
                                .h(px(22.0))
                                .rounded_md()
                                .id("plugin-picker-close")
                                .cursor(gpui::CursorStyle::PointingHand)
                                .hover(|s| s.bg(Colors::surface_control_hover()))
                                .on_click(move |_, window, cx| close_button(&(), window, cx))
                                .child(icon(assets::ICON_X_PATH, 12.0, Colors::text_faint())),
                        ),
                )
                // Search
                .child(
                    div()
                        .border_b(px(1.0))
                        .border_color(Colors::divider())
                        .px(px(10.0))
                        .py(px(7.0))
                        .child(text_field_with_callbacks(
                            search_input,
                            search_focused,
                            search_callbacks,
                        )),
                )
                // Body: sidebar + list
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_1()
                        .min_h(px(0.0))
                        .child(sidebar)
                        .child(list_section),
                )
                // Footer
                .child(footer),
        )
}
