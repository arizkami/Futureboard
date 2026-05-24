mod app;
mod audio_state;
mod window;

use sphere_ui_components::embedded_assets::EmbeddedAssets;

fn main() {
    // Instantiate the native audio facade up front so we surface any
    // construction errors before the GPUI shell takes over. The handle is
    // dropped at end of `main` for now — the rest of the shell does not
    // depend on a running stream yet.
    match audio_state::NativeAudioState::new() {
        Ok(state) => {
            eprintln!(
                "[audio] sphere-direct-audio-engine v{} ready (backend={:?}, sr={}, buf={})",
                state.version(),
                state.config.backend,
                state.config.sample_rate,
                state.config.buffer_size,
            );
        }
        Err(e) => {
            eprintln!("[audio] failed to build NativeAudioState: {e}");
        }
    }

    gpui::Application::new()
        .with_assets(EmbeddedAssets::new())
        .run(app::setup);
}

