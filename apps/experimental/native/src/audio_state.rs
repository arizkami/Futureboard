//! Minimal native audio engine integration.
//!
//! At this stage the native shell does not actually drive playback — it
//! only proves that the Rust facade in `sphere_direct_audio_engine::native`
//! can be instantiated and queried without touching the NAPI surface.
//!
//! The `NativeAudioState` is owned by `app::setup` and currently kept
//! offline (no `start()` call) so we do not open the OS audio device
//! before the rest of the playback path is wired up.

// The crate is published with `[lib] name = "DAUx"` so the N-API output
// is `DAUx.node`. Rust consumers import its symbols through that same
// name — alias it locally for readability.
use DAUx::{
    AudioBackend, AudioEngine, EngineConfig, EngineStats, SphereAudioError,
};

pub struct NativeAudioState {
    pub config: EngineConfig,
    pub engine: AudioEngine,
}

impl NativeAudioState {
    /// Build the default native audio state. Does not open the device —
    /// call `start_if_offline` once the rest of the playback pipeline is
    /// in place.
    pub fn new() -> Result<Self, SphereAudioError> {
        let config = EngineConfig {
            backend: AudioBackend::Auto,
            ..AudioEngine::default_config()
        };
        let engine = AudioEngine::new(config.clone())?;
        Ok(Self { config, engine })
    }

    /// Polling snapshot — wired into the status-bar / inspector later.
    pub fn stats(&self) -> EngineStats {
        self.engine.stats()
    }

    /// Engine semver — useful for the About box.
    pub fn version(&self) -> String {
        self.engine.version()
    }
}
