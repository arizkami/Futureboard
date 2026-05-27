//! Hybrid timeline rendering: immutable snapshots + pluggable backends.
//!
//! - [`TimelineViewport`] — scroll/zoom bounds for the arrangement region
//! - [`TimelineRenderSnapshot`] — read-only frame description (built on UI thread)
//! - [`TimelineRenderer`] — GPUI paint or offscreen WGPU draw
//!
//! Normal UI (menus, dialogs, headers, lanes as interactive GPUI elements) stays
//! in GPUI; dense paint (grid, future clip/waveform batches) routes here.

pub mod gpui_paint;
pub mod renderer;
pub mod snapshot;
pub mod viewport;
#[cfg(feature = "gpu-renderer")]
pub mod wgpu_renderer;

pub use gpui_paint::GpuiPaintTimelineRenderer;
pub use renderer::{
    create_timeline_renderer, create_timeline_renderer_with_fallback, TimelineRenderOutput,
    TimelineRenderer, TimelineRendererBackend,
};
pub use snapshot::{
    BarShadeSnapshot, GridLineSnapshot, PlayheadSnapshot, RenderClipSnapshot, RenderLaneSnapshot,
    RenderClipKind, SelectionSnapshot, SnapshotBuildOptions, TimelineRenderSnapshot,
    VisibleBeatRange, VisibleTrackRange, WaveformChunkHandle, WaveformReadyKind,
};
pub use viewport::TimelineViewport;
#[cfg(feature = "gpu-renderer")]
pub use wgpu_renderer::{WgpuOffscreenFrame, WgpuTimelineRenderer};
