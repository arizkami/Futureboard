//! Plain-Rust facade over the C ABI editor-window helpers in
//! `sphere_plugin_editor_*` (defined in `vst3backend`). The N-API wrapper
//! in `editor_window.rs` is feature-gated and unavailable to the native
//! `futureboard_native` binary; this module ships the same surface as
//! `Result<u64, String>` so both targets can drive the IPlugView
//! lifecycle.
//!
//! Hard rules (per `SKILL.md` §13–14):
//! - These calls must not run on the audio thread.
//! - Every `open_plugin_editor_window` must pair with `close_plugin_editor_window`.
//! - Bad plugin → `Err(...)`, never panic.
//! - `attach_vst3_editor_view` is best-effort; failure leaves the host
//!   window open so the caller can render a GPUI fallback.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_ulonglong};

#[repr(C)]
struct SpherePluginHostString {
    data: *const c_char,
    len: u64,
}

extern "C" {
    fn sphere_plugin_editor_open_window(
        window_id: *const c_char,
        title: *const c_char,
        subtitle: *const c_char,
        width: c_int,
        height: c_int,
    ) -> c_ulonglong;
    fn sphere_plugin_editor_get_attach_handle(handle: c_ulonglong) -> c_ulonglong;
    fn sphere_plugin_editor_attach_vst3_view(
        handle: c_ulonglong,
        plugin_path: *const c_char,
        class_id: *const c_char,
    ) -> c_int;
    fn sphere_plugin_editor_close_window(handle: c_ulonglong);
    fn sphere_plugin_editor_focus_window(handle: c_ulonglong);
    fn sphere_plugin_editor_resize_window(handle: c_ulonglong, width: c_int, height: c_int);
    fn sphere_plugin_editor_drain_param_events_json() -> SpherePluginHostString;
    fn sphere_plugin_host_free_string(value: SpherePluginHostString);
}

/// Options accepted by the native editor window. `width`/`height` default
/// to a conservative 560×380 if unset.
#[derive(Debug, Clone, Default)]
pub struct NativeEditorWindowOptions {
    pub window_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub plugin_path: Option<String>,
    pub class_id: Option<String>,
    /// Set to "VST3" to also call `attach_vst3_editor_view` after open.
    pub format: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NativeEditorParamEvent {
    pub window_id: String,
    pub param_id: f64,
    pub value: f64,
}

#[derive(serde::Deserialize)]
struct ParamEventRaw {
    #[serde(rename = "windowId")]
    window_id: String,
    #[serde(rename = "paramId")]
    param_id: f64,
    value: f64,
}

fn to_cstring(label: &str, value: String) -> Result<CString, String> {
    CString::new(value).map_err(|e| format!("{label}: {e}"))
}

/// Open a native plugin editor window. Returns the opaque host handle
/// (non-zero on success) which subsequent `close`/`focus`/`resize`/
/// `attach` calls must reference.
pub fn open_plugin_editor_window(options: NativeEditorWindowOptions) -> Result<u64, String> {
    let window_id = to_cstring("window_id", options.window_id)?;
    let title = to_cstring("title", options.title)?;
    let subtitle_text = options
        .subtitle
        .clone()
        .unwrap_or_else(|| "Native plugin editor window".to_string());
    let subtitle = to_cstring("subtitle", subtitle_text)?;
    let handle = unsafe {
        sphere_plugin_editor_open_window(
            window_id.as_ptr(),
            title.as_ptr(),
            subtitle.as_ptr(),
            options.width.unwrap_or(560) as c_int,
            options.height.unwrap_or(380) as c_int,
        )
    };
    if handle == 0 {
        return Err("plugin editor window failed to open".to_string());
    }
    if options
        .format
        .as_deref()
        .map(|f| f.eq_ignore_ascii_case("VST3"))
        .unwrap_or(false)
    {
        if let (Some(plugin_path), Some(class_id)) = (options.plugin_path, options.class_id) {
            let _ = attach_vst3_editor_view(handle, plugin_path, class_id);
        }
    }
    Ok(handle)
}

pub fn get_plugin_editor_attach_handle(handle: u64) -> u64 {
    if handle == 0 {
        return 0;
    }
    unsafe { sphere_plugin_editor_get_attach_handle(handle as c_ulonglong) }
}

pub fn attach_vst3_editor_view(
    handle: u64,
    plugin_path: String,
    class_id: String,
) -> Result<bool, String> {
    if handle == 0 {
        return Ok(false);
    }
    let plugin_path = to_cstring("plugin_path", plugin_path)?;
    let class_id = to_cstring("class_id", class_id)?;
    let ok = unsafe {
        sphere_plugin_editor_attach_vst3_view(
            handle as c_ulonglong,
            plugin_path.as_ptr(),
            class_id.as_ptr(),
        )
    };
    Ok(ok != 0)
}

pub fn close_plugin_editor_window(handle: u64) {
    if handle == 0 {
        return;
    }
    unsafe { sphere_plugin_editor_close_window(handle as c_ulonglong) };
}

pub fn focus_plugin_editor_window(handle: u64) {
    if handle == 0 {
        return;
    }
    unsafe { sphere_plugin_editor_focus_window(handle as c_ulonglong) };
}

pub fn resize_plugin_editor_window(handle: u64, width: u32, height: u32) {
    if handle == 0 {
        return;
    }
    unsafe {
        sphere_plugin_editor_resize_window(handle as c_ulonglong, width as c_int, height as c_int)
    };
}

/// Drain any pending parameter-change events emitted by the native
/// editor view. Callers should poll this on the UI thread at ~30 Hz.
pub fn drain_plugin_editor_param_events() -> Result<Vec<NativeEditorParamEvent>, String> {
    let native = unsafe { sphere_plugin_editor_drain_param_events_json() };
    if native.data.is_null() {
        return Ok(Vec::new());
    }
    let json = unsafe { CStr::from_ptr(native.data) }
        .to_string_lossy()
        .into_owned();
    unsafe { sphere_plugin_host_free_string(native) };
    let parsed: Vec<ParamEventRaw> =
        serde_json::from_str(&json).map_err(|e| format!("param event json: {e}"))?;
    Ok(parsed
        .into_iter()
        .map(|p| NativeEditorParamEvent {
            window_id: p.window_id,
            param_id: p.param_id,
            value: p.value,
        })
        .collect())
}

/// FNV-1a stable id for path-keyed window ids. Matches the helper used
/// by the N-API wrapper.
pub fn stable_id(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
