//! GPUI paint fallback — existing quad/div grid path driven by snapshots.

use std::sync::Arc;

use gpui::{canvas, fill, point, px, size, Bounds, IntoElement, Pixels, Styled};

use super::renderer::{TimelineRenderOutput, TimelineRenderer};
use super::snapshot::TimelineRenderSnapshot;
use crate::components::timeline::timeline_state::GridLineLevel;
use crate::theme::Colors;

/// Renders the arrangement grid via GPUI `canvas` + `paint_quad`.
pub struct GpuiPaintTimelineRenderer;

impl GpuiPaintTimelineRenderer {
    pub fn new() -> Self {
        Self
    }

    fn paint_grid(snapshot: &TimelineRenderSnapshot, bounds: Bounds<Pixels>, window: &mut gpui::Window) {
        let grid_height = snapshot.viewport.height;
        let grid_width = snapshot.viewport.width;
        let paint_bounds = Bounds::new(bounds.origin, size(px(grid_width), px(grid_height)));
        window.paint_layer(paint_bounds, |window| {
            for shade in &snapshot.bar_shades {
                let bar_bounds = local_bounds(bounds, shade.x, 0.0, shade.width, grid_height);
                window.paint_quad(fill(
                    bar_bounds,
                    Colors::with_alpha(Colors::text_primary(), 0.022),
                ));
            }
            for line in &snapshot.grid_lines {
                let alpha = match line.level {
                    GridLineLevel::Bar => 0.14,
                    GridLineLevel::Beat => 0.062,
                    GridLineLevel::Sub => 0.026,
                };
                let line_bounds = local_bounds(bounds, line.x, 0.0, 1.0, grid_height);
                window.paint_quad(fill(
                    line_bounds,
                    Colors::with_alpha(Colors::text_primary(), alpha),
                ));
            }
        });
    }
}

impl TimelineRenderer for GpuiPaintTimelineRenderer {
    fn backend_name(&self) -> &'static str {
        "gpui-paint"
    }

    fn render_arrangement(
        &mut self,
        snapshot: &TimelineRenderSnapshot,
    ) -> TimelineRenderOutput {
        let _s = crate::perf::PerfScope::enter("GpuiPaintTimelineRenderer");
        crate::perf::count("grid_lines", snapshot.grid_lines.len() as u64);
        crate::perf::count("visible_clips", snapshot.clips.len() as u64);

        let snapshot = Arc::new(snapshot.clone());
        let element = canvas(
            |_bounds, _window, _cx| {},
            move |bounds, (), window, _cx| {
                GpuiPaintTimelineRenderer::paint_grid(snapshot.as_ref(), bounds, window);
            },
        )
        .absolute()
        .inset_0()
        .into_any_element();

        TimelineRenderOutput::Gpui(element)
    }
}

fn local_bounds(parent: Bounds<Pixels>, x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(
        parent.origin + point(px(x), px(y)),
        size(px(width.max(0.0)), px(height.max(0.0))),
    )
}

/// Dev-only: log snapshot stats when WGPU path runs in parallel with GPUI display.
pub fn log_snapshot_stats(snapshot: &TimelineRenderSnapshot, backend: &str) {
    if std::env::var_os("FUTUREBOARD_GPU_RENDERER_DEBUG").is_some() {
        eprintln!(
            "[gpu-renderer] snapshot backend={backend} grid={} clips={} lanes={} tracks={}..{}",
            snapshot.grid_lines.len(),
            snapshot.clips.len(),
            snapshot.lanes.len(),
            snapshot.visible_tracks.start_index,
            snapshot.visible_tracks.end_index,
        );
    }
}
