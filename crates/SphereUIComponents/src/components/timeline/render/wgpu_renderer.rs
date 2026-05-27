//! Offscreen WGPU arrangement renderer (scaffold).
//!
//! Renders into a private `wgpu::Texture` — **not** a competing window surface.
//! Compositing into GPUI still requires Blade/GPUI texture interop (see
//! `tasks/native/gpui-wgpu-hybrid-renderer.md`).

use super::renderer::{TimelineRenderOutput, TimelineRenderer};
use super::snapshot::TimelineRenderSnapshot;

/// GPU texture produced by an offscreen arrangement pass.
pub struct WgpuOffscreenFrame {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    /// Offscreen color target — keep alive until composited or dropped.
    pub texture: wgpu::Texture,
}

pub struct WgpuTimelineRenderer {
    instance: wgpu::Instance,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    init_error: Option<String>,
}

impl WgpuTimelineRenderer {
    pub fn new() -> Self {
        Self {
            instance: wgpu::Instance::default(),
            device: None,
            queue: None,
            init_error: None,
        }
    }

    pub fn is_available(&mut self) -> bool {
        self.init_error.is_none() && self.ensure_device().is_ok()
    }

    fn ensure_device(&mut self) -> Result<(), String> {
        if self.device.is_some() {
            return Ok(());
        }
        if let Some(err) = &self.init_error {
            return Err(err.clone());
        }
        let adapter = pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|_| "no WGPU adapter".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("futureboard-timeline"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .map_err(|e| e.to_string())?;

        self.device = Some(device);
        self.queue = Some(queue);
        Ok(())
    }

    fn render_offscreen(&mut self, snapshot: &TimelineRenderSnapshot) -> Result<WgpuOffscreenFrame, String> {
        self.ensure_device()?;
        let device = self.device.as_ref().expect("device");
        let queue = self.queue.as_ref().expect("queue");

        let width = snapshot.viewport.width.max(1.0) as u32;
        let height = snapshot.viewport.height.max(1.0) as u32;
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("timeline-offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Timeline arrangement background — matches `Colors::surface_base()` feel.
        let bg = wgpu::Color {
            r: 0.043,
            g: 0.059,
            b: 0.078,
            a: 1.0,
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("timeline-arrangement"),
        });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("timeline-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(bg),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // Scaffold: grid lines, lane fills, clip rects, and waveform chunks will be
            // drawn here via instanced pipelines reading `TimelineRenderSnapshot` only.
        }

        queue.submit(Some(encoder.finish()));

        if std::env::var_os("FUTUREBOARD_GPU_RENDERER_DEBUG").is_some() {
            eprintln!(
                "[gpu-renderer] WgpuTimelineRenderer offscreen {}x{} grid={} clips={} waveform_handles={}",
                width,
                height,
                snapshot.grid_lines.len(),
                snapshot.clips.len(),
                snapshot
                    .clips
                    .iter()
                    .filter(|c| c.waveform.is_some())
                    .count(),
            );
        }

        Ok(WgpuOffscreenFrame {
            width,
            height,
            format,
            texture,
        })
    }
}

impl TimelineRenderer for WgpuTimelineRenderer {
    fn backend_name(&self) -> &'static str {
        "wgpu-offscreen"
    }

    fn render_arrangement(
        &mut self,
        snapshot: &TimelineRenderSnapshot,
    ) -> TimelineRenderOutput {
        let _s = crate::perf::PerfScope::enter("WgpuTimelineRenderer");
        match self.render_offscreen(snapshot) {
            Ok(frame) => TimelineRenderOutput::WgpuOffscreen(frame),
            Err(error) => {
                eprintln!("[gpu-renderer] offscreen render failed: {error}");
                super::gpui_paint::GpuiPaintTimelineRenderer::new().render_arrangement(snapshot)
            }
        }
    }
}
