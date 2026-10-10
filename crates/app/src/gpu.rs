//! Connects the viewport renderer (the `render` crate) to the window's
//! wgpu device and to egui, so the panel can show the rendered image.

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use render::{FrameParams, PickRequest, ViewportRenderer as GpuRenderer, ViewportTarget};
use ui::{PickAsk, PickReply, Picked, ViewportRenderer, ViewportView};

/// Draws the viewport with the same GPU device the window uses.
pub struct GpuViewport {
    state: RenderState,
    renderer: GpuRenderer,
    target: Option<ViewportTarget>,
    /// The size last asked for (the target may be clamped smaller).
    requested_size: [u32; 2],
    texture: Option<egui::TextureId>,
}

impl GpuViewport {
    /// Sets up viewport rendering on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        Self {
            renderer: GpuRenderer::new(&state.device, samples),
            state: state.clone(),
            target: None,
            requested_size: [0, 0],
            texture: None,
        }
    }

    /// A short description of the GPU, for the status bar.
    pub fn describe(&self) -> String {
        let info = self.state.adapter.get_info();
        format!(
            "Graphics: {} ({:?}, {}× smoothing)",
            info.name,
            info.backend,
            self.renderer.sample_count()
        )
    }

    /// Makes sure a target of the requested size exists and is registered
    /// with egui. Returns the egui texture showing it.
    fn prepare_target(&mut self, size: [u32; 2]) -> Option<egui::TextureId> {
        if let (Some(texture), Some(_)) = (self.texture, &self.target)
            && self.requested_size == size
        {
            return Some(texture);
        }
        let device = &self.state.device;
        let target = self.renderer.create_target(device, size);
        let mut egui_renderer = self.state.renderer.write();
        let id = match self.texture {
            Some(id) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    device,
                    target.display_view(),
                    wgpu::FilterMode::Nearest,
                    id,
                );
                id
            }
            None => egui_renderer.register_native_texture(
                device,
                target.display_view(),
                wgpu::FilterMode::Nearest,
            ),
        };
        self.texture = Some(id);
        self.target = Some(target);
        self.requested_size = size;
        Some(id)
    }
}

impl ViewportRenderer for GpuViewport {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        let texture = self
            .prepare_target(view.size_px)
            .ok_or("the viewport image could not be created")?;
        let target = self
            .target
            .as_ref()
            .ok_or("the viewport image could not be created")?;
        let params = FrameParams {
            view_projection: view.view_projection,
            camera_position: view.camera_position,
            focus: view.focus,
            sky_top: view.sky_top,
            sky_horizon: view.sky_horizon,
            ground: view.ground,
            fine_grid: view.fine_grid,
        };
        self.renderer
            .render(&self.state.device, &self.state.queue, target, &params);
        Ok(texture)
    }

    fn pick(&mut self, ask: Option<PickAsk>) -> PickReply {
        let device = &self.state.device;
        let answer = self.renderer.poll_pick(device).map(|r| Picked {
            pixel: r.pixel,
            point: r.point,
        });
        if let Some(ask) = ask
            && !self.renderer.is_picking()
        {
            self.renderer.request_pick(
                device,
                &self.state.queue,
                PickRequest {
                    view_projection: ask.view_projection,
                    pixel: ask.pixel,
                    size: ask.size_px,
                },
            );
        }
        PickReply {
            answer,
            waiting: self.renderer.is_picking(),
        }
    }
}
