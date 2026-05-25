#[cfg(feature = "gpu-renderer")]
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::IntoElement;
#[cfg(feature = "gpu-renderer")]
use gpui::{canvas, fill, point, px, size, Bounds, Pixels, Styled};

#[cfg(not(feature = "gpu-renderer"))]
use crate::components::timeline::timeline_grid::timeline_grid;
#[cfg(feature = "gpu-renderer")]
use crate::components::timeline::timeline_state::GridLineLevel;
use crate::components::timeline::timeline_state::TimelineState;

#[cfg(feature = "gpu-renderer")]
static GPU_RENDERER_PROBE_LOGGED: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "gpu-renderer"))]
pub fn timeline_surface(
    state: &TimelineState,
    grid_width: f32,
    grid_height: f32,
) -> impl IntoElement {
    timeline_grid(state, grid_width, grid_height)
}

#[cfg(feature = "gpu-renderer")]
pub fn timeline_surface(
    state: &TimelineState,
    grid_width: f32,
    grid_height: f32,
) -> impl IntoElement {
    let _s = crate::perf::PerfScope::enter("TimelineSurface");
    let lines = state.get_arrangement_grid_lines(grid_width);
    crate::perf::count("grid_lines", lines.len() as u64);

    let ppb = state.viewport.pixels_per_second * state.seconds_per_beat();
    let bpb = state.beats_per_bar();
    let bar_w = bpb * ppb;
    let scroll_x = state.viewport.scroll_x;
    let mut bar_fills = Vec::new();
    if bar_w >= 2.0 {
        let start_beat = scroll_x / ppb;
        let first_bar = (start_beat / bpb).floor() as i32;
        let last_bar = ((scroll_x + grid_width) / bar_w).ceil() as i32;
        for bar in first_bar..=last_bar {
            if bar % 2 == 0 {
                bar_fills.push((bar as f32 * bar_w - scroll_x).round());
            }
        }
    }

    canvas(
        move |_bounds, window, _cx| {
            probe_gpu_surface_support_once(window);
        },
        move |bounds, (), window, _cx| {
            let paint_bounds = Bounds::new(bounds.origin, size(px(grid_width), px(grid_height)));
            window.paint_layer(paint_bounds, |window| {
                for x in bar_fills {
                    let bar_bounds = local_bounds(bounds, x, 0.0, bar_w.round(), grid_height);
                    window.paint_quad(fill(
                        bar_bounds,
                        gpui::Rgba {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.022,
                        },
                    ));
                }

                for line in lines {
                    let alpha = match line.level {
                        GridLineLevel::Bar => 0.14,
                        GridLineLevel::Beat => 0.062,
                        GridLineLevel::Sub => 0.026,
                    };
                    let line_bounds = local_bounds(bounds, line.x, 0.0, 1.0, grid_height);
                    window.paint_quad(fill(
                        line_bounds,
                        gpui::Rgba {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: alpha,
                        },
                    ));
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

#[cfg(feature = "gpu-renderer")]
fn local_bounds(parent: Bounds<Pixels>, x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(
        parent.origin + point(px(x), px(y)),
        size(px(width.max(0.0)), px(height.max(0.0))),
    )
}

#[cfg(feature = "gpu-renderer")]
fn probe_gpu_surface_support_once(window: &mut gpui::Window) {
    if GPU_RENDERER_PROBE_LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }

    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

    let has_window_handle = window.window_handle().is_ok();
    let has_display_handle = window.display_handle().is_ok();
    let _wgpu_instance = wgpu::Instance::default();
    eprintln!(
        "[gpu-renderer] WGPU instance initialized. GPUI raw handles: window={} display={}. \
         Bounded WGPU child-surface compositing is not exposed by GPUI 0.2.2; \
         TimelineSurface is using the clipped GPUI paint fallback.",
        has_window_handle, has_display_handle
    );
}
