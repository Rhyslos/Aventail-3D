//! Connects the viewport renderer (the `render` crate) to the window's
//! wgpu device and to egui, so the panel can show the rendered image.

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use render::{FrameParams, PickRequest, Shape, ViewportRenderer as GpuRenderer, ViewportTarget};
use ui::{PickAsk, PickReply, Picked, VIEW_COUNT, ViewportRenderer, ViewportView};

/// The image one view is drawn into, and the egui texture showing it.
struct SlotImage {
    target: ViewportTarget,
    /// The size last asked for (the target may be clamped smaller).
    requested_size: [u32; 2],
    texture: egui::TextureId,
}

/// Draws the viewport with the same GPU device the window uses: one image
/// per view, so the four-view layout can show them side by side.
pub struct GpuViewport {
    state: RenderState,
    renderer: GpuRenderer,
    images: [Option<SlotImage>; VIEW_COUNT],
    /// Whether the renderer holds the test cube (it starts with it), or
    /// nothing because the cube is hidden or deleted.
    holds_test_cube: bool,
}

impl GpuViewport {
    /// Sets up viewport rendering on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        Self {
            renderer: GpuRenderer::new(&state.device, samples),
            state: state.clone(),
            images: std::array::from_fn(|_| None),
            holds_test_cube: true,
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

    /// Makes sure view `slot` has an image of the requested size,
    /// registered with egui. Returns `None` for a slot outside the four
    /// views.
    fn prepare_image(&mut self, slot: usize, size: [u32; 2]) -> Option<()> {
        let existing = self.images.get(slot)?;
        if existing
            .as_ref()
            .is_some_and(|image| image.requested_size == size)
        {
            return Some(());
        }
        let device = &self.state.device;
        let target = self.renderer.create_target(device, size);
        let mut egui_renderer = self.state.renderer.write();
        let texture = match existing {
            Some(image) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    device,
                    target.display_view(),
                    wgpu::FilterMode::Nearest,
                    image.texture,
                );
                image.texture
            }
            None => egui_renderer.register_native_texture(
                device,
                target.display_view(),
                wgpu::FilterMode::Nearest,
            ),
        };
        drop(egui_renderer);
        *self.images.get_mut(slot)? = Some(SlotImage {
            target,
            requested_size: size,
            texture,
        });
        Some(())
    }
}

impl ViewportRenderer for GpuViewport {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        let params = FrameParams {
            view_projection: view.view_projection,
            camera_position: view.camera_position,
            focus: view.focus,
            sky_top: view.sky_top,
            sky_horizon: view.sky_horizon,
            ground: view.ground,
            fine_grid: view.fine_grid,
            flat_height: view.flat_height,
            grid_plane: view.grid_plane,
        };
        if view.test_cube != self.holds_test_cube {
            let shape = if view.test_cube {
                Shape::test_cube()
            } else {
                Shape::default()
            };
            self.renderer.set_shape(&self.state.device, &shape);
            self.holds_test_cube = view.test_cube;
        }
        const FAILED: &str = "the viewport image could not be created";
        self.prepare_image(view.slot, view.size_px).ok_or(FAILED)?;
        let image = self
            .images
            .get(view.slot)
            .and_then(Option::as_ref)
            .ok_or(FAILED)?;
        // Each render writes its own settings and submits its own work,
        // so drawing several views in one frame keeps them apart.
        self.renderer.render(
            &self.state.device,
            &self.state.queue,
            &image.target,
            &params,
        );
        Ok(image.texture)
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

#[cfg(test)]
mod tests {
    #[test]
    fn framing_knows_the_cube_the_renderer_draws() {
        // The scene's test cube object and the cube the renderer draws
        // must be the same box.
        let scene = scene::TEST_CUBE;
        let side = render::TEST_CUBE_SIZE;
        assert_eq!(scene.size().to_array(), [side; 3]);
        assert_eq!(scene.min.z, 0.0, "standing on the grid");
        assert_eq!(scene.centre().x, 0.0);
        assert_eq!(scene.centre().y, 0.0);
    }
}
