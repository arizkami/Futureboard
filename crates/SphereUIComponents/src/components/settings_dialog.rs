use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, svg, App, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window,
};

use crate::assets;
use crate::components::text_input::{
    text_field_with_callbacks, TextInputCallbacks, TextInputState,
};
use crate::components::slider::slider;
use crate::components::controls::{
    fb_button, fb_field_label, fb_form_row, fb_section_label, fb_segmented_button,
    fb_stepper_button, FbButtonKind,
};
use crate::theme::Colors;
use crate::settings::SettingsSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Hardware,
    Appearance,
    Editing,
    Recording,
    Playback,
    Plugins,
    FilesFolders,
    Shortcuts,
    Accessibility,
    CloudAccount,
    Advanced,
    About,
}

impl SettingsTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Hardware => "Hardware",
            Self::Appearance => "Appearance",
            Self::Editing => "Editing",
            Self::Recording => "Recording",
            Self::Playback => "Playback",
            Self::Plugins => "Plugins",
            Self::FilesFolders => "Files & Folders",
            Self::Shortcuts => "Shortcuts",
            Self::Accessibility => "Accessibility",
            Self::CloudAccount => "Cloud & Account",
            Self::Advanced => "Advanced",
            Self::About => "About",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::General => assets::ICON_FILE_PATH,
            Self::Hardware => assets::ICON_MIC_PATH,
            Self::Appearance => assets::ICON_SLIDERS_HORIZONTAL_PATH,
            Self::Editing => assets::ICON_PENCIL_PATH,
            Self::Recording => assets::ICON_CIRCLE_PATH,
            Self::Playback => assets::ICON_PLAY_PATH,
            Self::Plugins => assets::ICON_CPU_PATH,
            Self::FilesFolders => assets::ICON_FOLDER_PATH,
            Self::Shortcuts => assets::ICON_LINK_PATH,
            Self::Accessibility => assets::ICON_BUG_PATH,
            Self::CloudAccount => assets::ICON_SHARE_PATH,
            Self::Advanced => assets::ICON_CLOCK_PATH,
            Self::About => assets::ICON_CIRCLE_DOT_PATH,
        }
    }

    pub fn all() -> [Self; 13] {
        [
            Self::General,
            Self::Hardware,
            Self::Appearance,
            Self::Editing,
            Self::Recording,
            Self::Playback,
            Self::Plugins,
            Self::FilesFolders,
            Self::Shortcuts,
            Self::Accessibility,
            Self::CloudAccount,
            Self::Advanced,
            Self::About,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct SettingsDialogState {
    pub is_open: bool,
    pub active_tab: SettingsTab,
    pub search_query: String,
}

impl SettingsDialogState {
    pub fn closed() -> Self {
        Self {
            is_open: false,
            active_tab: SettingsTab::General,
            search_query: String::new(),
        }
    }

    pub fn open() -> Self {
        Self {
            is_open: true,
            active_tab: SettingsTab::General,
            search_query: String::new(),
        }
    }
}

pub type UpdateSettingFn = Arc<dyn Fn(&mut SettingsSchema) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct SettingsDialogCallbacks {
    pub on_close: Arc<dyn Fn(&(), &mut Window, &mut App) + 'static>,
    pub on_select_tab: Arc<dyn Fn(&SettingsTab, &mut Window, &mut App) + 'static>,
    pub on_update_setting: Arc<dyn Fn(UpdateSettingFn, &mut Window, &mut App) + 'static>,
}

fn icon(path: &'static str, size: f32, color: gpui::Rgba) -> impl IntoElement {
    svg().path(path).w(px(size)).h(px(size)).text_color(color)
}

pub fn fb_checkbox(
    id: impl Into<gpui::ElementId>,
    checked: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(12.0))
        .h(px(12.0))
        .rounded_sm()
        .border(px(1.0))
        .border_color(Colors::border_default())
        .bg(if checked {
            Colors::accent_primary()
        } else {
            Colors::surface_input()
        })
        .cursor(gpui::CursorStyle::PointingHand)
        .on_click(on_click)
        .children(if checked {
            Some(
                svg()
                    .path(assets::ICON_CHECK_PATH)
                    .w(px(8.0))
                    .h(px(8.0))
                    .text_color(Colors::text_inverse()),
            )
        } else {
            None
        })
}

fn settings_header(title: &'static str, icon_path: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .pb(px(8.0))
        .border_b(px(1.0))
        .border_color(Colors::divider())
        .child(icon(icon_path, 12.0, Colors::accent_primary()))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(Colors::text_primary())
                .child(title),
        )
}

pub fn settings_dialog(
    state: &SettingsDialogState,
    schema: &SettingsSchema,
    search_input: &TextInputState,
    search_focused: bool,
    search_callbacks: TextInputCallbacks,
    callbacks: SettingsDialogCallbacks,
    available_inputs: &[String],
    available_outputs: &[String],
    available_backends: &[String],
) -> impl IntoElement {
    let close_backdrop = callbacks.on_close.clone();
    let close_button = callbacks.on_close.clone();

    let query = state.search_query.trim().to_lowercase();
    let is_match = |label: &str, keywords: &[&str]| {
        if query.is_empty() {
            return true;
        }
        let q = query.as_str();
        label.to_lowercase().contains(q) || keywords.iter().any(|k| k.to_lowercase().contains(q))
    };

    // Sidebar Category Tabs
    let sidebar_items = SettingsTab::all().into_iter().enumerate().map(|(index, tab)| {
        let active = state.active_tab == tab && query.is_empty();
        let cb = callbacks.on_select_tab.clone();
        
        let has_matching_settings = if query.is_empty() {
            true
        } else {
            match tab {
                SettingsTab::General => {
                    is_match("Language", &["language", "english"]) ||
                    is_match("Show start screen", &["start", "screen", "wizard"]) ||
                    is_match("Check updates", &["updates", "check"]) ||
                    is_match("Tempo", &["tempo", "bpm"]) ||
                    is_match("Sample Rate", &["sample", "rate"]) ||
                    is_match("Buffer Size", &["buffer", "size"]) ||
                    is_match("Autosave", &["autosave", "backup", "minutes"]) ||
                    is_match("Notifications", &["warnings", "alerts"])
                }
                SettingsTab::Hardware => {
                    is_match("Audio Driver", &["driver", "backend", "wasapi", "shared"]) ||
                    is_match("Input Device", &["input", "mic", "microphone"]) ||
                    is_match("Output Device", &["output", "speakers"]) ||
                    is_match("MIDI Enabled Inputs", &["midi", "inputs", "keyboard"]) ||
                    is_match("Sync", &["clock", "sync", "ltc"])
                }
                SettingsTab::Appearance => {
                    is_match("Theme", &["theme", "variant", "fleet"]) ||
                    is_match("UI Scale", &["scale", "size"]) ||
                    is_match("Arrangement Grid", &["grid", "intensity", "opacity"]) ||
                    is_match("Piano Roll Guides", &["piano", "roll", "guides", "keys"]) ||
                    is_match("Mixer Meter", &["mixer", "decay", "peak", "hold"])
                }
                SettingsTab::Editing => {
                    is_match("Mouse Zoom", &["mouse", "zoom", "sensitivity", "natural"]) ||
                    is_match("Snap to Grid", &["snap", "grid", "default"]) ||
                    is_match("Undo History", &["undo", "redo", "history", "max"])
                }
                SettingsTab::Recording => {
                    is_match("Audio Recording Format", &["format", "bit", "depth", "wav"]) ||
                    is_match("Metronome Click", &["metronome", "click", "sound", "volume"])
                }
                SettingsTab::Playback => {
                    is_match("Transport Playback", &["spacebar", "transport", "stop", "start"])
                }
                SettingsTab::Plugins => {
                    is_match("VST3 CLAP Formats", &["vst3", "clap", "plugins"]) ||
                    is_match("Paths Directories", &["paths", "directories", "folders"]) ||
                    is_match("Plugin Scanning", &["scan", "background"])
                }
                _ => false,
            }
        };

        let is_visible = query.is_empty() || has_matching_settings;

        if is_visible {
            div()
                .id(("settings-tab", index))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .h(px(26.0))
                .px(px(8.0))
                .rounded_md()
                .bg(if active {
                    Colors::surface_hover()
                } else {
                    gpui::transparent_black().into()
                })
                .text_size(px(10.5))
                .font_weight(if active {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::MEDIUM
                })
                .text_color(if active {
                    Colors::text_primary()
                } else if !query.is_empty() {
                    Colors::accent_primary()
                } else {
                    Colors::text_secondary()
                })
                .cursor(gpui::CursorStyle::PointingHand)
                .hover(|s| s.bg(Colors::surface_control_hover()))
                .on_click(move |_, window, cx| cb(&tab, window, cx))
                .child(icon(tab.icon(), 12.0, if active { Colors::accent_primary() } else { Colors::text_faint() }))
                .child(tab.label())
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }).collect::<Vec<_>>();

    // Right Side Content Views Builder
    let mut sections = Vec::new();

    // General Panel
    if (state.active_tab == SettingsTab::General && query.is_empty()) || (!query.is_empty() && (
        is_match("Language", &["language", "english"]) ||
        is_match("Show start screen", &["start", "screen", "wizard"]) ||
        is_match("Check updates", &["updates", "check"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("General > Application", assets::ICON_FILE_PATH))
                .child(fb_form_row(
                    "Language",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.general.language.clone();
                            let up = on_update.clone();
                            fb_segmented_button("lang-en", "English", val == "en", move |_, w, cx| {
                                up(Arc::new(|s| s.general.language = "en".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.general.language.clone();
                            let up = on_update.clone();
                            fb_segmented_button("lang-fr", "French", val == "fr", move |_, w, cx| {
                                up(Arc::new(|s| s.general.language = "fr".to_string()), w, cx);
                            })
                        })
                ))
                .child(fb_form_row(
                    "Start Wizard",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.general.show_start_screen;
                            let up = on_update.clone();
                            fb_checkbox("show-start-screen", val, move |_, w, cx| {
                                up(Arc::new(move |s| s.general.show_start_screen = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Show welcome wizard project templates on launch"),
                        )
                ))
                .child(fb_form_row(
                    "Update Check",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.general.check_updates;
                            let up = on_update.clone();
                            fb_checkbox("check-updates", val, move |_, w, cx| {
                                up(Arc::new(move |s| s.general.check_updates = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Automatically check for software updates"),
                        )
                ))
                .into_any_element()
        );
    }

    // General Panel > Autosave & Notifications
    if (state.active_tab == SettingsTab::General && query.is_empty()) || (!query.is_empty() && (
        is_match("Autosave", &["autosave", "backup", "minutes"]) ||
        is_match("Notifications", &["warnings", "alerts", "notifications"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("General > Autosave & Backup", assets::ICON_FILE_PATH))
                .child(fb_form_row(
                    "Autosave",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.general.autosave.enabled;
                            let up = on_update.clone();
                            fb_checkbox("autosave-enabled", val, move |_, w, cx| {
                                up(Arc::new(move |s| s.general.autosave.enabled = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Automatically save projects periodically"),
                        )
                ))
                .child(fb_form_row(
                    "Interval",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child({
                            let val = schema.general.autosave.interval_minutes;
                            let up = on_update.clone();
                            fb_stepper_button("autosave-interval-dec", "-", move |_, w, cx| {
                                up(Arc::new(move |s| s.general.autosave.interval_minutes = val.saturating_sub(1).max(1)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .w(px(40.0))
                                .h(px(28.0))
                                .rounded_md()
                                .border(px(1.0))
                                .border_color(Colors::border_subtle())
                                .bg(Colors::surface_input())
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .text_color(Colors::text_primary())
                                .child(schema.general.autosave.interval_minutes.to_string())
                        )
                        .child({
                            let val = schema.general.autosave.interval_minutes;
                            let up = on_update.clone();
                            fb_stepper_button("autosave-interval-inc", "+", move |_, w, cx| {
                                up(Arc::new(move |s| s.general.autosave.interval_minutes = (val + 1).min(120)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("minutes")
                        )
                ))
                .child(fb_form_row(
                    "Max Backups",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child({
                            let val = schema.general.autosave.max_backups;
                            let up = on_update.clone();
                            fb_stepper_button("autosave-backups-dec", "-", move |_, w, cx| {
                                up(Arc::new(move |s| s.general.autosave.max_backups = val.saturating_sub(1).max(1)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .w(px(40.0))
                                .h(px(28.0))
                                .rounded_md()
                                .border(px(1.0))
                                .border_color(Colors::border_subtle())
                                .bg(Colors::surface_input())
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .text_color(Colors::text_primary())
                                .child(schema.general.autosave.max_backups.to_string())
                        )
                        .child({
                            let val = schema.general.autosave.max_backups;
                            let up = on_update.clone();
                            fb_stepper_button("autosave-backups-inc", "+", move |_, w, cx| {
                                up(Arc::new(move |s| s.general.autosave.max_backups = (val + 1).min(99)), w, cx);
                            })
                        })
                ))
                .into_any_element()
        );

        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("General > Notifications", assets::ICON_FILE_PATH))
                .child(fb_form_row(
                    "Warnings",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.general.notifications.enable_warnings;
                            let up = on_update.clone();
                            fb_checkbox("notif-warnings-enabled", val, move |_, w, cx| {
                                up(Arc::new(move |s| s.general.notifications.enable_warnings = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Show warnings on critical errors or file conflicts"),
                        )
                ))
                .child(fb_form_row(
                    "System Notifications",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.general.notifications.enable_system_notifications;
                            let up = on_update.clone();
                            fb_checkbox("notif-system-enabled", val, move |_, w, cx| {
                                up(Arc::new(move |s| s.general.notifications.enable_system_notifications = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Send OS system notifications on export finished"),
                        )
                ))
                .into_any_element()
        );
    }

    // Project Defaults Defaults (within General Tab as per specs)
    if (state.active_tab == SettingsTab::General && query.is_empty()) || (!query.is_empty() && (
        is_match("Tempo", &["tempo", "bpm"]) ||
        is_match("Sample Rate", &["sample", "rate"]) ||
        is_match("Buffer Size", &["buffer", "size"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("General > Project Defaults", assets::ICON_FILE_PATH))
                .child(fb_form_row(
                    "Default Tempo",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child({
                            let up = on_update.clone();
                            let tempo = schema.general.project_defaults.tempo;
                            fb_stepper_button("tempo-dec", "-", move |_, w, cx| {
                                up(Arc::new(move |s| s.general.project_defaults.tempo = (tempo - 1.0).max(20.0)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .w(px(52.0))
                                .h(px(28.0))
                                .rounded_md()
                                .border(px(1.0))
                                .border_color(Colors::border_subtle())
                                .bg(Colors::surface_input())
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .text_color(Colors::text_primary())
                                .child(format!("{:.0}", schema.general.project_defaults.tempo))
                        )
                        .child({
                            let up = on_update.clone();
                            let tempo = schema.general.project_defaults.tempo;
                            fb_stepper_button("tempo-inc", "+", move |_, w, cx| {
                                up(Arc::new(move |s| s.general.project_defaults.tempo = (tempo + 1.0).min(999.0)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("bpm")
                        )
                ))
                .child(fb_form_row(
                    "Sample Rate",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.general.project_defaults.sample_rate;
                            let up = on_update.clone();
                            fb_segmented_button("sr-44100", "44.1k", val == 44100, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.sample_rate = 44100), w, cx);
                            })
                        })
                        .child({
                            let val = schema.general.project_defaults.sample_rate;
                            let up = on_update.clone();
                            fb_segmented_button("sr-48000", "48k", val == 48000, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.sample_rate = 48000), w, cx);
                            })
                        })
                        .child({
                            let val = schema.general.project_defaults.sample_rate;
                            let up = on_update.clone();
                            fb_segmented_button("sr-96000", "96k", val == 96000, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.sample_rate = 96000), w, cx);
                            })
                        })
                ))
                .child(fb_form_row(
                    "Buffer Size",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.general.project_defaults.buffer_size;
                            let up = on_update.clone();
                            fb_segmented_button("buf-64", "64", val == 64, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.buffer_size = 64), w, cx);
                            })
                        })
                        .child({
                            let val = schema.general.project_defaults.buffer_size;
                            let up = on_update.clone();
                            fb_segmented_button("buf-128", "128", val == 128, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.buffer_size = 128), w, cx);
                            })
                        })
                        .child({
                            let val = schema.general.project_defaults.buffer_size;
                            let up = on_update.clone();
                            fb_segmented_button("buf-256", "256", val == 256, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.buffer_size = 256), w, cx);
                            })
                        })
                        .child({
                            let val = schema.general.project_defaults.buffer_size;
                            let up = on_update.clone();
                            fb_segmented_button("buf-512", "512", val == 512, move |_, w, cx| {
                                up(Arc::new(|s| s.general.project_defaults.buffer_size = 512), w, cx);
                            })
                        })
                ))
                .into_any_element()
        );
    }

    // Hardware Panel (Audio Driver device etc.)
    if (state.active_tab == SettingsTab::Hardware && query.is_empty()) || (!query.is_empty() && (
        is_match("Audio Driver", &["driver", "backend", "wasapi"]) ||
        is_match("Input Device", &["input", "microphone"]) ||
        is_match("Output Device", &["output", "speakers"]) ||
        is_match("MIDI Enabled Inputs", &["midi", "inputs", "outputs", "port", "keyboard"]) ||
        is_match("Sync Clock", &["sync", "clock", "source", "ltc"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        
        let mut driver_buttons = div().flex().flex_row().gap(px(4.0));
        for (i, backend) in available_backends.iter().enumerate() {
            let active = schema.hardware.audio.driver_type == *backend;
            let backend_name = backend.clone();
            let up = on_update.clone();
            driver_buttons = driver_buttons.child(
                fb_segmented_button(
                    ("driver-backend", i),
                    backend.as_str(),
                    active,
                    move |_, w, cx| {
                        let backend_clone = backend_name.clone();
                        up(Arc::new(move |s| s.hardware.audio.driver_type = backend_clone.clone()), w, cx);
                    }
                )
            );
        }

        let mut input_buttons = div().flex().flex_row().gap(px(4.0));
        for (i, input_dev) in available_inputs.iter().enumerate() {
            let active = schema.hardware.audio.device_in == *input_dev;
            let input_name = input_dev.clone();
            let up = on_update.clone();
            input_buttons = input_buttons.child(
                fb_segmented_button(
                    ("audio-input-device", i),
                    input_dev.as_str(),
                    active,
                    move |_, w, cx| {
                        let input_clone = input_name.clone();
                        up(Arc::new(move |s| s.hardware.audio.device_in = input_clone.clone()), w, cx);
                    }
                )
            );
        }

        let mut output_buttons = div().flex().flex_row().gap(px(4.0));
        for (i, output_dev) in available_outputs.iter().enumerate() {
            let active = schema.hardware.audio.device_out == *output_dev;
            let output_name = output_dev.clone();
            let up = on_update.clone();
            output_buttons = output_buttons.child(
                fb_segmented_button(
                    ("audio-output-device", i),
                    output_dev.as_str(),
                    active,
                    move |_, w, cx| {
                        let output_clone = output_name.clone();
                        up(Arc::new(move |s| s.hardware.audio.device_out = output_clone.clone()), w, cx);
                    }
                )
            );
        }

        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("Hardware > Audio", assets::ICON_MIC_PATH))
                .child(fb_form_row("Audio Driver", driver_buttons))
                .child(fb_form_row("Input Device", input_buttons))
                .child(fb_form_row("Output Device", output_buttons))
                .into_any_element()
        );

        // MIDI Section
        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Hardware > MIDI", assets::ICON_MIC_PATH))
                .child(fb_form_row(
                    "MIDI Inputs",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child({
                            let enabled = schema.hardware.midi.enabled_inputs.contains(&"Keyboard Controller".to_string());
                            let up_in = up.clone();
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(fb_checkbox("midi-keyboard-ctrl", enabled, move |_, w, cx| {
                                    up_in(Arc::new(move |s| {
                                        let list = &mut s.hardware.midi.enabled_inputs;
                                        if enabled {
                                            list.retain(|x| x != "Keyboard Controller");
                                        } else if !list.contains(&"Keyboard Controller".to_string()) {
                                            list.push("Keyboard Controller".to_string());
                                        }
                                    }), w, cx);
                                }))
                                .child(div().text_size(px(10.5)).text_color(Colors::text_primary()).child("Keyboard Controller"))
                        })
                        .child({
                            let enabled = schema.hardware.midi.enabled_inputs.contains(&"Midi Device 2".to_string());
                            let up_in = up.clone();
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(fb_checkbox("midi-device-2", enabled, move |_, w, cx| {
                                    up_in(Arc::new(move |s| {
                                        let list = &mut s.hardware.midi.enabled_inputs;
                                        if enabled {
                                            list.retain(|x| x != "Midi Device 2");
                                        } else if !list.contains(&"Midi Device 2".to_string()) {
                                            list.push("Midi Device 2".to_string());
                                        }
                                    }), w, cx);
                                }))
                                .child(div().text_size(px(10.5)).text_color(Colors::text_primary()).child("Midi Device 2"))
                        })
                ))
                .child(fb_form_row(
                    "MIDI Outputs",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child({
                            let enabled = schema.hardware.midi.enabled_outputs.contains(&"Synth Out".to_string());
                            let up_out = up.clone();
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(fb_checkbox("midi-synth-out", enabled, move |_, w, cx| {
                                    up_out(Arc::new(move |s| {
                                        let list = &mut s.hardware.midi.enabled_outputs;
                                        if enabled {
                                            list.retain(|x| x != "Synth Out");
                                        } else if !list.contains(&"Synth Out".to_string()) {
                                            list.push("Synth Out".to_string());
                                        }
                                    }), w, cx);
                                }))
                                .child(div().text_size(px(10.5)).text_color(Colors::text_primary()).child("Synth Out"))
                        })
                ))
                .child(fb_form_row(
                    "MIDI Clock Sync",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.hardware.midi.clock_sync;
                            let up_sync = up.clone();
                            fb_checkbox("midi-clock-sync", val, move |_, w, cx| {
                                up_sync(Arc::new(move |s| s.hardware.midi.clock_sync = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Send MIDI clock to output devices"),
                        )
                ))
                .into_any_element()
        );

        // Sync Section
        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Hardware > Sync", assets::ICON_CLOCK_PATH))
                .child(fb_form_row(
                    "Clock Source",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.hardware.sync.clock_source.clone();
                            let up_clk = up.clone();
                            fb_segmented_button("sync-internal", "Internal", val == "Internal", move |_, w, cx| {
                                up_clk(Arc::new(|s| s.hardware.sync.clock_source = "Internal".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.hardware.sync.clock_source.clone();
                            let up_clk = up.clone();
                            fb_segmented_button("sync-midi", "MIDI", val == "MIDI", move |_, w, cx| {
                                up_clk(Arc::new(|s| s.hardware.sync.clock_source = "MIDI".to_string()), w, cx);
                            })
                        })
                ))
                .child(fb_form_row(
                    "LTC Reader",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.hardware.sync.ltc_enabled;
                            let up_ltc = up.clone();
                            fb_checkbox("sync-ltc-enabled", val, move |_, w, cx| {
                                up_ltc(Arc::new(move |s| s.hardware.sync.ltc_enabled = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Enable linear timecode (LTC) reader on input"),
                        )
                ))
                .into_any_element()
        );
    }

    // Appearance Panel (Theme, sliders)
    if (state.active_tab == SettingsTab::Appearance && query.is_empty()) || (!query.is_empty() && (
        is_match("Theme", &["theme", "fleet", "dark"]) ||
        is_match("UI Scale", &["scale", "size"]) ||
        is_match("Arrangement Grid", &["grid", "intensity", "opacity"]) ||
        is_match("Piano Roll Guides", &["piano", "roll", "guides", "keys"]) ||
        is_match("Mixer Meter", &["mixer", "decay", "peak", "hold"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("Appearance > Theme & UI", assets::ICON_SLIDERS_HORIZONTAL_PATH))
                .child(fb_form_row(
                    "Theme Preset",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.appearance.theme.clone();
                            let up = on_update.clone();
                            fb_segmented_button("theme-fleet", "Fleet Dark", val == "Fleet Dark", move |_, w, cx| {
                                up(Arc::new(|s| s.appearance.theme = "Fleet Dark".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.appearance.theme.clone();
                            let up = on_update.clone();
                            fb_segmented_button("theme-ableton", "Ableton Dark", val == "Ableton Dark", move |_, w, cx| {
                                up(Arc::new(|s| s.appearance.theme = "Ableton Dark".to_string()), w, cx);
                            })
                        })
                ))
                .child(fb_form_row(
                    "UI Scale",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(slider(
                            "ui-scale-slider",
                            (schema.appearance.ui_scale - 0.5) / 2.0, // map [0.5, 2.5] to [0, 1]
                            Colors::accent_primary(),
                            {
                                let up = on_update.clone();
                                move |val, w, cx| {
                                    let actual_val = 0.5 + val * 2.0;
                                    up(Arc::new(move |s| s.appearance.ui_scale = actual_val), w, cx);
                                }
                            }
                        ))
                        .child(
                            div()
                                .w(px(32.0))
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(format!("{:.1}x", schema.appearance.ui_scale))
                        )
                ))
                .child(fb_form_row(
                    "Grid Intensity",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(slider(
                            "grid-intensity-slider",
                            schema.appearance.arrangement.grid_line_intensity,
                            Colors::accent_primary(),
                            {
                                let up = on_update.clone();
                                move |val, w, cx| {
                                    let intensity = *val;
                                    up(Arc::new(move |s| s.appearance.arrangement.grid_line_intensity = intensity), w, cx);
                                }
                            }
                        ))
                        .child(
                            div()
                                .w(px(32.0))
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(format!("{:.0}%", schema.appearance.arrangement.grid_line_intensity * 100.0))
                        )
                ))
                .into_any_element()
        );

        // Piano Roll Section
        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Appearance > Piano Roll", assets::ICON_PENCIL_PATH))
                .child(fb_form_row(
                    "Key Guides",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.appearance.piano_roll.show_key_guides;
                            let up_guides = up.clone();
                            fb_checkbox("appearance-key-guides", val, move |_, w, cx| {
                                up_guides(Arc::new(move |s| s.appearance.piano_roll.show_key_guides = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Show piano key guides in background"),
                        )
                ))
                .into_any_element()
        );

        // Mixer Section
        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Appearance > Mixer", assets::ICON_SLIDERS_HORIZONTAL_PATH))
                .child(fb_form_row(
                    "Meter Decay",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(slider(
                            "mixer-decay-slider",
                            (schema.appearance.mixer.meter_decay_db_per_sec - 12.0) / 36.0, // map [12, 48] to [0, 1]
                            Colors::accent_primary(),
                            {
                                let up_decay = up.clone();
                                move |val, w, cx| {
                                    let actual_val = 12.0 + val * 36.0;
                                    up_decay(Arc::new(move |s| s.appearance.mixer.meter_decay_db_per_sec = actual_val), w, cx);
                                }
                            }
                        ))
                        .child(
                            div()
                                .w(px(52.0))
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(format!("{:.1} dB/s", schema.appearance.mixer.meter_decay_db_per_sec))
                        )
                ))
                .child(fb_form_row(
                    "Peak Hold",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(slider(
                            "mixer-peak-slider",
                            (schema.appearance.mixer.peak_hold_seconds - 0.5) / 4.5, // map [0.5, 5.0] to [0, 1]
                            Colors::accent_primary(),
                            {
                                let up_peak = up.clone();
                                move |val, w, cx| {
                                    let actual_val = 0.5 + val * 4.5;
                                    up_peak(Arc::new(move |s| s.appearance.mixer.peak_hold_seconds = actual_val), w, cx);
                                }
                            }
                        ))
                        .child(
                            div()
                                .w(px(52.0))
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(format!("{:.1} s", schema.appearance.mixer.peak_hold_seconds))
                        )
                ))
                .into_any_element()
        );
    }

    // Editing Panel (Mouse, snap, undo history)
    if (state.active_tab == SettingsTab::Editing && query.is_empty()) || (!query.is_empty() && (
        is_match("Mouse Zoom", &["mouse", "zoom", "sensitivity", "natural"]) ||
        is_match("Snap to Grid", &["snap", "grid", "default"]) ||
        is_match("Undo History", &["undo", "redo", "history", "max"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("Editing > Mouse & Navigation", assets::ICON_PENCIL_PATH))
                .child(fb_form_row(
                    "Zoom Sensitivity",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(slider(
                            "zoom-sensitivity-slider",
                            (schema.editing.mouse.zoom_sensitivity - 0.2) / 1.8, // map [0.2, 2.0] to [0, 1]
                            Colors::accent_primary(),
                            {
                                let up = on_update.clone();
                                move |val, w, cx| {
                                    let actual_val = 0.2 + val * 1.8;
                                    up(Arc::new(move |s| s.editing.mouse.zoom_sensitivity = actual_val), w, cx);
                                }
                            }
                        ))
                        .child(
                            div()
                                .w(px(32.0))
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(format!("{:.1}x", schema.editing.mouse.zoom_sensitivity))
                        )
                ))
                .child(fb_form_row(
                    "Natural Scroll",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.editing.mouse.natural_scroll;
                            let up = on_update.clone();
                            fb_checkbox("editing-natural-scroll", val, move |_, w, cx| {
                                up(Arc::new(move |s| s.editing.mouse.natural_scroll = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Invert trackpad/mousewheel scroll direction"),
                        )
                ))
                .into_any_element()
        );

        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Editing > Grid & Snap", assets::ICON_SLIDERS_HORIZONTAL_PATH))
                .child(fb_form_row(
                    "Snap to Grid",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.editing.snap.snap_to_grid;
                            let up_snap = up.clone();
                            fb_checkbox("editing-snap-grid", val, move |_, w, cx| {
                                up_snap(Arc::new(move |s| s.editing.snap.snap_to_grid = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Snap clips/notes to current grid lines"),
                        )
                ))
                .child(fb_form_row(
                    "Default Snap",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.editing.snap.default_snap_value.clone();
                            let up_val = up.clone();
                            fb_segmented_button("snap-1-4", "1/4", val == "1/4", move |_, w, cx| {
                                up_val(Arc::new(|s| s.editing.snap.default_snap_value = "1/4".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.editing.snap.default_snap_value.clone();
                            let up_val = up.clone();
                            fb_segmented_button("snap-1-8", "1/8", val == "1/8", move |_, w, cx| {
                                up_val(Arc::new(|s| s.editing.snap.default_snap_value = "1/8".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.editing.snap.default_snap_value.clone();
                            let up_val = up.clone();
                            fb_segmented_button("snap-1-16", "1/16", val == "1/16", move |_, w, cx| {
                                up_val(Arc::new(|s| s.editing.snap.default_snap_value = "1/16".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.editing.snap.default_snap_value.clone();
                            let up_val = up.clone();
                            fb_segmented_button("snap-1-32", "1/32", val == "1/32", move |_, w, cx| {
                                up_val(Arc::new(|s| s.editing.snap.default_snap_value = "1/32".to_string()), w, cx);
                            })
                        })
                ))
                .into_any_element()
        );

        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Editing > History", assets::ICON_CLOCK_PATH))
                .child(fb_form_row(
                    "Max Undo Steps",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child({
                            let val = schema.editing.history.max_undo_steps;
                            let up_steps = up.clone();
                            fb_stepper_button("undo-steps-dec", "-", move |_, w, cx| {
                                up_steps(Arc::new(move |s| s.editing.history.max_undo_steps = val.saturating_sub(5).max(10)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .w(px(40.0))
                                .h(px(28.0))
                                .rounded_md()
                                .border(px(1.0))
                                .border_color(Colors::border_subtle())
                                .bg(Colors::surface_input())
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .text_color(Colors::text_primary())
                                .child(schema.editing.history.max_undo_steps.to_string())
                        )
                        .child({
                            let val = schema.editing.history.max_undo_steps;
                            let up_steps = up.clone();
                            fb_stepper_button("undo-steps-inc", "+", move |_, w, cx| {
                                up_steps(Arc::new(move |s| s.editing.history.max_undo_steps = (val + 5).min(500)), w, cx);
                            })
                        })
                ))
                .into_any_element()
        );
    }

    // Recording Panel (Audio recording format, Metronome)
    if (state.active_tab == SettingsTab::Recording && query.is_empty()) || (!query.is_empty() && (
        is_match("Audio Recording Format", &["format", "bit", "depth", "wav"]) ||
        is_match("Metronome Click", &["metronome", "click", "sound", "volume"])
    )) {
        let on_update = callbacks.on_update_setting.clone();

        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("Recording > Audio Format", assets::ICON_CIRCLE_PATH))
                .child(fb_form_row(
                    "Format Type",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.recording.audio.format.clone();
                            let up = on_update.clone();
                            fb_segmented_button("rec-format-wav", "WAV", val == "wav", move |_, w, cx| {
                                up(Arc::new(|s| s.recording.audio.format = "wav".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.recording.audio.format.clone();
                            let up = on_update.clone();
                            fb_segmented_button("rec-format-aiff", "AIFF", val == "aiff", move |_, w, cx| {
                                up(Arc::new(|s| s.recording.audio.format = "aiff".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.recording.audio.format.clone();
                            let up = on_update.clone();
                            fb_segmented_button("rec-format-flac", "FLAC", val == "flac", move |_, w, cx| {
                                up(Arc::new(|s| s.recording.audio.format = "flac".to_string()), w, cx);
                            })
                        })
                ))
                .child(fb_form_row(
                    "Bit Depth",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.recording.audio.bit_depth;
                            let up = on_update.clone();
                            fb_segmented_button("rec-depth-16", "16-bit", val == 16, move |_, w, cx| {
                                up(Arc::new(|s| s.recording.audio.bit_depth = 16), w, cx);
                            })
                        })
                        .child({
                            let val = schema.recording.audio.bit_depth;
                            let up = on_update.clone();
                            fb_segmented_button("rec-depth-24", "24-bit", val == 24, move |_, w, cx| {
                                up(Arc::new(|s| s.recording.audio.bit_depth = 24), w, cx);
                            })
                        })
                        .child({
                            let val = schema.recording.audio.bit_depth;
                            let up = on_update.clone();
                            fb_segmented_button("rec-depth-32", "32-bit float", val == 32, move |_, w, cx| {
                                up(Arc::new(|s| s.recording.audio.bit_depth = 32), w, cx);
                            })
                        })
                ))
                .into_any_element()
        );

        let up = on_update.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .mt(px(12.0))
                .child(settings_header("Recording > Metronome Click", assets::ICON_CIRCLE_PATH))
                .child(fb_form_row(
                    "Enable Click",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let val = schema.recording.metronome.enabled;
                            let up_met = up.clone();
                            fb_checkbox("rec-metronome-enabled", val, move |_, w, cx| {
                                up_met(Arc::new(move |s| s.recording.metronome.enabled = !val), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Hear metronome click during recording & playback"),
                        )
                ))
                .child(fb_form_row(
                    "Click Volume",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(slider(
                            "metronome-volume-slider",
                            schema.recording.metronome.volume,
                            Colors::accent_primary(),
                            {
                                let up_vol = up.clone();
                                move |val, w, cx| {
                                    let volume = *val;
                                    up_vol(Arc::new(move |s| s.recording.metronome.volume = volume), w, cx);
                                }
                            }
                        ))
                        .child(
                            div()
                                .w(px(32.0))
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(format!("{:.0}%", schema.recording.metronome.volume * 100.0))
                        )
                ))
                .child(fb_form_row(
                    "Click Sound",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child({
                            let val = schema.recording.metronome.sound_type.clone();
                            let up_snd = up.clone();
                            fb_segmented_button("met-sound-wood", "Woodblock", val == "Woodblock", move |_, w, cx| {
                                up_snd(Arc::new(|s| s.recording.metronome.sound_type = "Woodblock".to_string()), w, cx);
                            })
                        })
                        .child({
                            let val = schema.recording.metronome.sound_type.clone();
                            let up_snd = up.clone();
                            fb_segmented_button("met-sound-beep", "Beep", val == "Beep", move |_, w, cx| {
                                up_snd(Arc::new(|s| s.recording.metronome.sound_type = "Beep".to_string()), w, cx);
                            })
                        })
                ))
                .child(fb_form_row(
                    "Count-in Bars",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child({
                            let val = schema.recording.metronome.count_in_bars;
                            let up_cnt = up.clone();
                            fb_stepper_button("met-count-dec", "-", move |_, w, cx| {
                                up_cnt(Arc::new(move |s| s.recording.metronome.count_in_bars = val.saturating_sub(1).max(0)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .w(px(40.0))
                                .h(px(28.0))
                                .rounded_md()
                                .border(px(1.0))
                                .border_color(Colors::border_subtle())
                                .bg(Colors::surface_input())
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.0))
                                .text_color(Colors::text_primary())
                                .child(schema.recording.metronome.count_in_bars.to_string())
                        )
                        .child({
                            let val = schema.recording.metronome.count_in_bars;
                            let up_cnt = up.clone();
                            fb_stepper_button("met-count-inc", "+", move |_, w, cx| {
                                up_cnt(Arc::new(move |s| s.recording.metronome.count_in_bars = (val + 1).min(4)), w, cx);
                            })
                        })
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("bars")
                        )
                ))
                .into_any_element()
        );
    }

    // Playback Panel (Transport options)
    if (state.active_tab == SettingsTab::Playback && query.is_empty()) || (!query.is_empty() && (
        is_match("Transport Playback", &["spacebar", "transport", "stop", "start"])
    )) {
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("Playback > Transport", assets::ICON_PLAY_PATH))
                .child(fb_form_row(
                    "Spacebar Action",
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .child(fb_segmented_button("space-play-pause", "Play / Pause", true, |_e, _w, _cx| {}))
                        .child(fb_segmented_button("space-play-stop", "Play / Stop (Soon)", false, |_e, _w, _cx| {}))
                ))
                .child(fb_form_row(
                    "Return to Start",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(fb_checkbox("return-on-stop", true, |_e, _w, _cx| {}))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child("Return playhead to start position on Stop"),
                        )
                ))
                .into_any_element()
        );
    }

    // Plugins Panel (vst directories list etc.)
    if (state.active_tab == SettingsTab::Plugins && query.is_empty()) || (!query.is_empty() && (
        is_match("VST3 CLAP Formats", &["vst3", "clap", "plugins"]) ||
        is_match("Paths Directories", &["paths", "directories", "folders"])
    )) {
        let on_update = callbacks.on_update_setting.clone();
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("Plugins > Formats & Folders", assets::ICON_CPU_PATH))
                .child(fb_form_row(
                    "Formats",
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(16.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child({
                                    let val = schema.plugins.vst3.enabled;
                                    let up = on_update.clone();
                                    fb_checkbox("vst3-enabled", val, move |_, w, cx| {
                                        up(Arc::new(move |s| s.plugins.vst3.enabled = !val), w, cx);
                                    })
                                })
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(Colors::text_primary())
                                        .child("Enable VST3"),
                                )
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child({
                                    let val = schema.plugins.clap.enabled;
                                    let up = on_update.clone();
                                    fb_checkbox("clap-enabled", val, move |_, w, cx| {
                                        up(Arc::new(move |s| s.plugins.clap.enabled = !val), w, cx);
                                    })
                                })
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(Colors::text_primary())
                                        .child("Enable CLAP"),
                                )
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child({
                                    let val = schema.plugins.scan.background_scan;
                                    let up = on_update.clone();
                                    fb_checkbox("scan-background-scan", val, move |_, w, cx| {
                                        up(Arc::new(move |s| s.plugins.scan.background_scan = !val), w, cx);
                                    })
                                })
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(Colors::text_primary())
                                        .child("Background Scan"),
                                )
                        )
                ))
                .child(fb_form_row(
                    "VST3 Folders",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .children(schema.plugins.vst3.paths.iter().map(|path| {
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(path.clone())
                        }))
                ))
                .child(fb_form_row(
                    "CLAP Folders",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .children(schema.plugins.clap.paths.iter().map(|path| {
                            div()
                                .text_size(px(10.0))
                                .text_color(Colors::text_muted())
                                .child(path.clone())
                        }))
                ))
                .child(fb_form_row(
                    "Actions",
                    fb_button("trigger-plugins-scan", "Scan Plugins Now", FbButtonKind::Primary, true, |_e, _w, _cx| {
                        eprintln!("[plugins] manual scan triggered from settings dialog");
                    })
                ))
                .into_any_element()
        );
    }

    // About Panel
    if (state.active_tab == SettingsTab::About && query.is_empty()) || (!query.is_empty() && (
        is_match("Version About", &["version", "credits", "about"])
    )) {
        sections.push(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(settings_header("About Futureboard Studio", assets::ICON_CIRCLE_DOT_PATH))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(Colors::text_primary())
                        .child("Futureboard Studio / Mochi DAW v0.1.0")
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(Colors::text_muted())
                        .child("Built with GPUI, Rust, and C++ VST3 SDK.")
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(Colors::text_faint())
                        .child("© 2026 Futureboard Studio team. All rights reserved.")
                )
                .into_any_element()
        );
    }

    // Fill placeholder sections for other panels if not matches
    if sections.is_empty() {
        sections.push(
            div()
                .px(px(12.0))
                .py(px(24.0))
                .text_align(gpui::TextAlign::Center)
                .text_size(px(11.0))
                .text_color(Colors::text_faint())
                .child(if query.is_empty() {
                    format!("The {} panel is not fully wired in Native yet.", state.active_tab.label())
                } else {
                    format!("No settings match \"{}\"", query)
                })
                .into_any_element()
        );
    }

    // Overlay shell
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0()
        .flex()
        .items_start()
        .justify_center()
        .pt(px(56.0))
        .px(px(18.0))
        .pb(px(32.0))
        .id("settings-modal-overlay")
        .bg(gpui::transparent_black())
        .occlude()
        .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
            close_backdrop(&(), window, cx);
        })
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(640.0))
                .max_w(px(640.0))
                .h(px(520.0))
                .max_h(px(520.0))
                .overflow_hidden()
                .rounded_xl()
                .border(px(1.0))
                .border_color(Colors::border_default())
                .bg(Colors::surface_window())
                .shadow(vec![gpui::BoxShadow {
                    color: Colors::surface_overlay().into(),
                    offset: gpui::point(px(0.0), px(16.0)),
                    blur_radius: px(40.0),
                    spread_radius: px(0.0),
                }])
                .on_mouse_down(gpui::MouseButton::Left, |_, _window, cx| {
                    cx.stop_propagation();
                })
                // Title Bar
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .h(px(40.0))
                        .px(px(16.0))
                        .border_b(px(1.0))
                        .border_color(Colors::divider())
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(icon(assets::ICON_SLIDERS_HORIZONTAL_PATH, 13.0, Colors::accent_primary()))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(Colors::text_primary())
                                        .child("Preferences"),
                                ),
                        )
                        // Search bar inside title bar
                        .child(
                            div()
                                .w(px(220.0))
                                .child(text_field_with_callbacks(
                                    search_input,
                                    search_focused,
                                    search_callbacks,
                                ))
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded_md()
                                .id("settings-close")
                                .cursor(gpui::CursorStyle::PointingHand)
                                .hover(|s| s.bg(Colors::surface_control_hover()))
                                .on_click(move |_, window, cx| close_button(&(), window, cx))
                                .child(icon(assets::ICON_X_PATH, 13.0, Colors::text_faint())),
                        ),
                )
                // Two-column layout
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_1()
                        .min_h_0()
                        // Left Sidebar: Tabs List
                        .child(
                            div()
                                .id("settings-sidebar-scroll")
                                .w(px(160.0))
                                .flex_shrink_0()
                                .border_r(px(1.0))
                                .border_color(Colors::divider())
                                .bg(Colors::surface_panel_alt())
                                .overflow_y_scroll()
                                .p(px(8.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .children(sidebar_items)
                        )
                        // Right Content Panel
                        .child(
                            div()
                                .id("settings-content-scroll")
                                .flex_1()
                                .bg(Colors::surface_panel())
                                .overflow_y_scroll()
                                .p(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(16.0))
                                .children(sections)
                        )
                )
        )
}
