use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_float};

use serde_json::Value;

#[repr(C)]
struct SphereDauxVst3Processor {
    _private: [u8; 0],
}

extern "C" {
    fn sphere_daux_vst3_create(
        plugin_path: *const c_char,
        class_id: *const c_char,
        sample_rate: c_double,
    ) -> *mut SphereDauxVst3Processor;
    fn sphere_daux_vst3_destroy(processor: *mut SphereDauxVst3Processor);
    fn sphere_daux_vst3_process_stereo_sample(
        processor: *mut SphereDauxVst3Processor,
        in_l: c_float,
        in_r: c_float,
        out_l: *mut c_float,
        out_r: *mut c_float,
    ) -> i32;
    fn sphere_daux_vst3_process_count(processor: *mut SphereDauxVst3Processor) -> u64;
    fn sphere_daux_vst3_last_input_peak(processor: *mut SphereDauxVst3Processor) -> c_double;
    fn sphere_daux_vst3_last_output_peak(processor: *mut SphereDauxVst3Processor) -> c_double;
    fn sphere_daux_vst3_last_difference_peak(processor: *mut SphereDauxVst3Processor) -> c_double;
    /// Enqueue a normalized (0..1) VST3 parameter change.
    /// Delivered to IAudioProcessor via inputParameterChanges on the next process call.
    fn sphere_daux_vst3_set_param(
        processor: *mut SphereDauxVst3Processor,
        param_id: u32,
        value: c_double,
    );
}

#[derive(Debug)]
pub struct Vst3RuntimeProcessor {
    raw: *mut SphereDauxVst3Processor,
    plugin_path: String,
    class_id: String,
    sample_rate: u32,
}

unsafe impl Send for Vst3RuntimeProcessor {}

impl Vst3RuntimeProcessor {
    pub fn from_params(
        params: &std::collections::HashMap<String, Value>,
        sample_rate: u32,
    ) -> Option<Self> {
        let plugin_path = params.get("path").and_then(Value::as_str)?.trim();
        if plugin_path.is_empty() {
            return None;
        }
        let class_id = params
            .get("classId")
            .or_else(|| params.get("class_id"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        Self::new(plugin_path, class_id, sample_rate)
    }

    pub fn new(plugin_path: &str, class_id: &str, sample_rate: u32) -> Option<Self> {
        let path = CString::new(plugin_path).ok()?;
        let class_id_c = CString::new(class_id).ok()?;
        let raw = unsafe {
            sphere_daux_vst3_create(
                path.as_ptr(),
                class_id_c.as_ptr(),
                sample_rate.max(1) as c_double,
            )
        };
        if raw.is_null() {
            return None;
        }
        Some(Self {
            raw,
            plugin_path: plugin_path.to_string(),
            class_id: class_id.to_string(),
            sample_rate: sample_rate.max(1),
        })
    }

    #[inline]
    pub fn process_stereo_sample(&mut self, l: f32, r: f32) -> Option<(f32, f32)> {
        if self.raw.is_null() {
            return None;
        }
        let mut out_l = 0.0f32;
        let mut out_r = 0.0f32;
        let ok = unsafe {
            sphere_daux_vst3_process_stereo_sample(self.raw, l, r, &mut out_l, &mut out_r)
        };
        if ok == 0 {
            None
        } else {
            Some((out_l, out_r))
        }
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        !self.raw.is_null()
    }

    #[inline]
    pub fn process_count(&self) -> u64 {
        if self.raw.is_null() {
            0
        } else {
            unsafe { sphere_daux_vst3_process_count(self.raw) }
        }
    }

    #[inline]
    pub fn last_input_peak(&self) -> f64 {
        if self.raw.is_null() {
            0.0
        } else {
            unsafe { sphere_daux_vst3_last_input_peak(self.raw) as f64 }
        }
    }

    #[inline]
    pub fn last_output_peak(&self) -> f64 {
        if self.raw.is_null() {
            0.0
        } else {
            unsafe { sphere_daux_vst3_last_output_peak(self.raw) as f64 }
        }
    }

    #[inline]
    pub fn last_difference_peak(&self) -> f64 {
        if self.raw.is_null() {
            0.0
        } else {
            unsafe { sphere_daux_vst3_last_difference_peak(self.raw) as f64 }
        }
    }

    /// Enqueue a normalized (0..1) parameter change for the given VST3 ParamID.
    ///
    /// The change is delivered to `IAudioProcessor` via `inputParameterChanges`
    /// on the next `process_stereo_sample` call.  Safe to call from the audio
    /// thread (inside command-drain) or from any other thread.
    ///
    /// `param_id` — the integer `Steinberg::Vst::ParamID` as exposed by the plugin.
    /// `value`    — normalized value in `[0.0, 1.0]`.
    #[inline]
    pub fn set_param(&mut self, param_id: u32, value: f64) {
        if self.raw.is_null() {
            return;
        }
        unsafe { sphere_daux_vst3_set_param(self.raw, param_id, value as c_double) }
    }
}

impl Clone for Vst3RuntimeProcessor {
    fn clone(&self) -> Self {
        Self::new(&self.plugin_path, &self.class_id, self.sample_rate).unwrap_or(Self {
            raw: std::ptr::null_mut(),
            plugin_path: self.plugin_path.clone(),
            class_id: self.class_id.clone(),
            sample_rate: self.sample_rate,
        })
    }
}

impl Drop for Vst3RuntimeProcessor {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        unsafe { sphere_daux_vst3_destroy(self.raw) };
        self.raw = std::ptr::null_mut();
    }
}
