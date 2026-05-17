//! DAUx WASAPI Exclusive backend — Windows only.
//!
//! Uses raw Win32 WASAPI COM APIs to open a device in exclusive mode with
//! event-driven buffer filling and MMCSS "Pro Audio" thread priority.
//!
//! # Thread model
//!
//! A dedicated audio thread is spawned.  The thread:
//!   1. Calls `CoInitializeEx(COINIT_MULTITHREADED)` for COM.
//!   2. Sets MMCSS "Pro Audio" priority via `AvSetMmThreadCharacteristicsW`.
//!   3. Opens WASAPI device in exclusive, event-driven mode.
//!   4. Runs `WaitForSingleObject(buffer_event)` render loop until `stop_flag`.
//!   5. Calls `CoUninitialize` on exit.
//!
//! If exclusive mode is denied by the device, falls back to WASAPI Shared.

#![allow(non_snake_case, clippy::too_many_arguments)]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IAudioClient, IAudioRenderClient, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_SHAREMODE_EXCLUSIVE, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_NOPERSIST, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

use crate::backend::DauxDeviceConfig;
use crate::backend::render::{drain_commands, fill_output_f32, LocalAudioState};
use crate::command::EngineCommand;
use crate::engine::SharedState;
use crate::error::SphereAudioError;
use crate::runtime::RuntimeProject;

// ── Raw extern for MMCSS (avrt.lib) ──────────────────────────────────────────

#[link(name = "avrt")]
extern "system" {
    fn AvSetMmThreadCharacteristicsW(task_name: *const u16, task_index: *mut u32) -> isize;
    fn AvRevertMmThreadCharacteristics(handle: isize) -> i32;
}

// ─────────────────────────────────────────────────────────────────────────────

/// Handle to a running WASAPI Exclusive stream.
/// Drop to signal the audio thread to stop.
pub struct WasapiExclusiveHandle {
    pub cmd_tx: Sender<EngineCommand>,
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub device_name: String,
    stop_flag: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for WasapiExclusiveHandle {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

pub fn open(
    config: &DauxDeviceConfig,
    shared: Arc<SharedState>,
    initial_runtime: RuntimeProject,
    glitch_counter: Arc<AtomicU64>,
) -> Result<WasapiExclusiveHandle, SphereAudioError> {
    let output_device_id = config.output_device_id.clone();
    let requested_sr = config.sample_rate;
    let buf_frames = config.buffer_size.unwrap_or(if config.safe_mode { 512 } else { 256 });

    let (tx, rx) = bounded::<EngineCommand>(512);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop_flag);
    let glitch2 = Arc::clone(&glitch_counter);

    // Info channel: audio thread reports (sample_rate, buffer_size, device_name) back.
    let (info_tx, info_rx) = std::sync::mpsc::channel::<Result<(u32, u32, String), String>>();

    let t = thread::Builder::new()
        .name("daux-wasapi-excl".into())
        .spawn(move || {
            unsafe {
                wasapi_thread(
                    output_device_id, requested_sr, buf_frames,
                    rx, shared, initial_runtime, glitch2, stop2, info_tx,
                );
            }
        })
        .map_err(|e| SphereAudioError::StreamOpenFailed(e.to_string()))?;

    let (sample_rate, buffer_size, device_name) = info_rx
        .recv_timeout(std::time::Duration::from_secs(8))
        .map_err(|_| SphereAudioError::StreamOpenFailed("WASAPI thread init timeout".into()))
        .and_then(|r| r.map_err(SphereAudioError::StreamOpenFailed))?;

    Ok(WasapiExclusiveHandle {
        cmd_tx: tx,
        sample_rate,
        buffer_size,
        device_name,
        stop_flag,
        thread: Some(t),
    })
}

// ─────────────────────────────────────────────────────────────────────────────

unsafe fn wasapi_thread(
    device_id: Option<String>,
    requested_sr: Option<u32>,
    buf_frames: u32,
    cmd_rx: Receiver<EngineCommand>,
    shared: Arc<SharedState>,
    initial_runtime: RuntimeProject,
    glitch_counter: Arc<AtomicU64>,
    stop_flag: Arc<AtomicBool>,
    info_tx: std::sync::mpsc::Sender<Result<(u32, u32, String), String>>,
) {
    // ── COM init ──────────────────────────────────────────────────────────────
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

    // ── MMCSS ─────────────────────────────────────────────────────────────────
    let task: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
    let mut task_idx = 0u32;
    let mmcss_h = AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut task_idx);
    if mmcss_h != 0 {
        eprintln!("[DAUx WASAPI Excl] MMCSS 'Pro Audio' set (index={task_idx})");
        shared.mmcss_active.store(true, Ordering::Relaxed);
    } else {
        eprintln!("[DAUx WASAPI Excl] MMCSS set failed (non-fatal)");
    }

    // ── Device ────────────────────────────────────────────────────────────────
    let enumerator: IMMDeviceEnumerator =
        match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            Ok(e) => e,
            Err(e) => {
                let _ = info_tx.send(Err(format!("CoCreateInstance(IMMDeviceEnumerator): {e}")));
                cleanup_mmcss(mmcss_h);
                CoUninitialize();
                return;
            }
        };

    let device: IMMDevice = match resolve_device(&enumerator, device_id.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            let _ = info_tx.send(Err(e));
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    };

    let device_name = get_device_friendly_name(&device);
    eprintln!("[DAUx WASAPI Excl] Opening: {device_name}");

    // ── IAudioClient ──────────────────────────────────────────────────────────
    let client: IAudioClient = match device.Activate(CLSCTX_ALL, None) {
        Ok(c) => c,
        Err(e) => {
            let _ = info_tx.send(Err(format!("Activate(IAudioClient): {e}")));
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    };

    // Get device's mix format (native format for exclusive mode negotiation).
    let mix_fmt = match client.GetMixFormat() {
        Ok(p) => p,
        Err(e) => {
            let _ = info_tx.send(Err(format!("GetMixFormat: {e}")));
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    };

    let native_sr = (*mix_fmt).nSamplesPerSec;
    let device_ch = (*mix_fmt).nChannels as usize;
    let sample_rate = requested_sr.unwrap_or(native_sr);

    // ── Try exclusive then fallback shared ────────────────────────────────────
    // Exclusive HNS duration (buffer_frames / sr * 10_000_000 for 100ns units).
    let hns = (buf_frames as i64 * 10_000_000i64) / native_sr as i64;
    let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_NOPERSIST;

    let exclusive_ok = client
        .IsFormatSupported(AUDCLNT_SHAREMODE_EXCLUSIVE, mix_fmt, None)
        .is_ok();

    let (share_mode, mode_str) = if exclusive_ok {
        (AUDCLNT_SHAREMODE_EXCLUSIVE, "exclusive")
    } else {
        eprintln!("[DAUx WASAPI Excl] Exclusive not supported — using Shared fallback");
        (AUDCLNT_SHAREMODE_SHARED, "shared-fallback")
    };

    // Initialize.
    let init_result = client.Initialize(share_mode, flags, hns, hns, mix_fmt, None);
    if let Err(e) = init_result {
        // Second attempt: default period duration.
        if let Err(e2) = client.Initialize(share_mode, flags, 0, 0, mix_fmt, None) {
            let _ = info_tx.send(Err(format!("IAudioClient::Initialize ({mode_str}): {e} / fallback: {e2}")));
            windows::Win32::System::Com::CoTaskMemFree(Some(mix_fmt as *const _ as *const _));
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    }
    windows::Win32::System::Com::CoTaskMemFree(Some(mix_fmt as *const _ as *const _));

    let actual_buf = match client.GetBufferSize() {
        Ok(f) => f,
        Err(e) => {
            let _ = info_tx.send(Err(format!("GetBufferSize: {e}")));
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    };

    // ── Buffer ready event ────────────────────────────────────────────────────
    let buf_event: HANDLE = match CreateEventW(None, false, false, None) {
        Ok(h) => h,
        Err(e) => {
            let _ = info_tx.send(Err(format!("CreateEventW: {e}")));
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    };

    if let Err(e) = client.SetEventHandle(buf_event) {
        let _ = info_tx.send(Err(format!("SetEventHandle: {e}")));
        let _ = CloseHandle(buf_event);
        cleanup_mmcss(mmcss_h);
        CoUninitialize();
        return;
    }

    // ── IAudioRenderClient ────────────────────────────────────────────────────
    let render: IAudioRenderClient = match client.GetService() {
        Ok(s) => s,
        Err(e) => {
            let _ = info_tx.send(Err(format!("GetService(IAudioRenderClient): {e}")));
            let _ = CloseHandle(buf_event);
            cleanup_mmcss(mmcss_h);
            CoUninitialize();
            return;
        }
    };

    if let Err(e) = client.Start() {
        let _ = info_tx.send(Err(format!("IAudioClient::Start: {e}")));
        let _ = CloseHandle(buf_event);
        cleanup_mmcss(mmcss_h);
        CoUninitialize();
        return;
    }

    shared.sample_rate.store(sample_rate, Ordering::Relaxed);
    let _ = info_tx.send(Ok((sample_rate, actual_buf, device_name.clone())));
    eprintln!(
        "[DAUx WASAPI Excl] Stream: device='{}' mode={mode_str} sr={sample_rate} buf={actual_buf}fr ch={device_ch}",
        device_name
    );

    // ── Runtime ───────────────────────────────────────────────────────────────
    let mut runtime = initial_runtime;
    runtime.sample_rate = sample_rate;
    let mut local = LocalAudioState::new(sample_rate as f64);
    let mut scratch = vec![0.0f32; actual_buf as usize * device_ch];

    // ── Render loop ───────────────────────────────────────────────────────────
    loop {
        if stop_flag.load(Ordering::Relaxed) { break; }

        let wait = WaitForSingleObject(buf_event, 2000);
        if wait != WAIT_OBJECT_0 {
            eprintln!("[DAUx WASAPI Excl] WaitForSingleObject timeout/err — stopping");
            glitch_counter.fetch_add(1, Ordering::Relaxed);
            break;
        }
        if stop_flag.load(Ordering::Relaxed) { break; }

        drain_commands(&cmd_rx, &mut runtime, &shared, &mut local, sample_rate);

        // How many frames can we write?
        let padding = client.GetCurrentPadding().unwrap_or(actual_buf);
        let frames = actual_buf.saturating_sub(padding);
        if frames == 0 { continue; }

        let buf_ptr = match render.GetBuffer(frames) {
            Ok(p) => p,
            Err(_) => { glitch_counter.fetch_add(1, Ordering::Relaxed); continue; }
        };

        let total = frames as usize * device_ch;
        if scratch.len() < total { scratch.resize(total, 0.0f32); }
        let s = &mut scratch[..total];
        for x in s.iter_mut() { *x = 0.0; }
        fill_output_f32(s, device_ch, &mut runtime, &shared, &mut local);

        // Copy f32 samples to WASAPI buffer (device should be float format).
        let out: &mut [f32] = std::slice::from_raw_parts_mut(buf_ptr as *mut f32, total);
        out.copy_from_slice(s);

        if let Err(e) = render.ReleaseBuffer(frames, 0) {
            eprintln!("[DAUx WASAPI Excl] ReleaseBuffer: {e}");
            glitch_counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    let _ = client.Stop();
    let _ = CloseHandle(buf_event);
    eprintln!("[DAUx WASAPI Excl] Stopped: {device_name}");

    cleanup_mmcss(mmcss_h);
    shared.mmcss_active.store(false, Ordering::Relaxed);
    CoUninitialize();
}

// ─────────────────────────────────────────────────────────────────────────────

unsafe fn resolve_device(
    enumerator: &IMMDeviceEnumerator,
    name: Option<&str>,
) -> Result<IMMDevice, String> {
    match name {
        None => enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .map_err(|e| format!("GetDefaultAudioEndpoint: {e}")),
        Some(wanted) => {
            use windows::Win32::Media::Audio::IMMDeviceCollection;
            let coll: IMMDeviceCollection = enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|e| format!("EnumAudioEndpoints: {e}"))?;
            let count = coll.GetCount().map_err(|e| format!("GetCount: {e}"))?;
            for i in 0..count {
                let dev = coll.Item(i).map_err(|e| format!("Item({i}): {e}"))?;
                if get_device_friendly_name(&dev) == wanted {
                    return Ok(dev);
                }
            }
            // Not found — use default.
            eprintln!("[DAUx WASAPI Excl] Device '{wanted}' not found, using default");
            enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint (fallback): {e}"))
        }
    }
}

unsafe fn get_device_friendly_name(device: &IMMDevice) -> String {
    // Attempt to read friendly name via IPropertyStore.
    // On failure, return a placeholder rather than panicking.
    use windows::Win32::Devices::Properties::DEVPKEY_Device_FriendlyName;
    use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY};
    use windows::Win32::System::Com::STGM_READ;

    let store: IPropertyStore = match device.OpenPropertyStore(STGM_READ) {
        Ok(s) => s,
        Err(_) => return "Unknown Device".into(),
    };

    // DEVPROPKEY and PROPERTYKEY are layout-compatible (GUID + u32); cast directly.
    let key = &DEVPKEY_Device_FriendlyName as *const _ as *const PROPERTYKEY;
    let prop = match store.GetValue(key) {
        Ok(p) => p,
        Err(_) => return "Unknown Device".into(),
    };

    // windows 0.58 wraps PROPVARIANT opaquely.  Read the raw bytes manually.
    // Memory layout on 64-bit: [vt: u16][pad: u16×3][data: *mut u16] (total 16 bytes).
    // VT_LPWSTR = 31.
    #[repr(C)]
    struct RawPropVariant {
        vt: u16,
        _pad: [u16; 3],
        pwsz: *mut u16,
    }
    let raw = &prop as *const _ as *const RawPropVariant;
    if (*raw).vt == 31 {
        let ptr = (*raw).pwsz;
        if !ptr.is_null() {
            let mut len = 0usize;
            while *ptr.add(len) != 0 { len += 1; }
            let slice = std::slice::from_raw_parts(ptr, len);
            let s = String::from_utf16_lossy(slice).to_string();
            windows::Win32::System::Com::CoTaskMemFree(Some(ptr as *const _));
            return s;
        }
    }
    "Unknown Device".into()
}

unsafe fn cleanup_mmcss(handle: isize) {
    if handle != 0 {
        AvRevertMmThreadCharacteristics(handle);
    }
}
