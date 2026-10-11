//! Connects the viewport renderer (the `render` crate) to the window's
//! wgpu device and to egui, so the panel can show the rendered image.

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use render::{
    FrameParams, MeshInstance, Overlay, PickRequest, Shape, ViewportRenderer as GpuRenderer,
    ViewportTarget,
};
use ui::{
    ComponentOverlay, OverlayStyle, PickAsk, PickReply, Picked, VIEW_COUNT, ViewportRenderer,
    ViewportView,
};

/// Plain points in the Points mode: small light dots, across in pixels.
const POINT_SIZE: f32 = 6.0;
/// Selected points: larger, in the accent colour.
const SELECTED_POINT_SIZE: f32 = 9.0;
/// The point under the mouse: larger still, in a lighter accent.
const HOVERED_POINT_SIZE: f32 = 10.0;
/// Edges of the edited objects, to see their structure.
const LINE_WIDTH: f32 = 1.25;
/// Selected edges.
const SELECTED_LINE_WIDTH: f32 = 3.0;
/// The edge under the mouse, or the polygon's outline.
const HOVERED_LINE_WIDTH: f32 = 3.5;
/// Light grey, linear space, for plain points and edges.
const PLAIN_POINT: [f32; 4] = [0.85, 0.85, 0.85, 0.95];
const PLAIN_LINE: [f32; 4] = [0.9, 0.9, 0.9, 0.45];
/// How see-through the tint on selected and hovered polygons is.
const SELECTED_TINT: f32 = 0.45;
const HOVERED_TINT: f32 = 0.3;
/// How far towards white the hovered colour is from the accent, so what
/// is under the mouse stands out from what is selected.
const HOVER_LIFT: f32 = 0.45;

/// The renderer's overlay for the interface's description of it: selected
/// elements in `accent` (linear colour), the hovered one lighter.
fn overlay_geometry(overlay: &ComponentOverlay, accent: [f32; 3]) -> Overlay {
    let [r, g, b] = accent;
    let lift = |c: f32| c + (1.0 - c) * HOVER_LIFT;
    let selected = [r, g, b, 1.0];
    let hovered = [lift(r), lift(g), lift(b), 1.0];
    let mut out = Overlay::default();
    for ([a, b2], style) in &overlay.lines {
        match style {
            OverlayStyle::Plain => out.line(*a, *b2, LINE_WIDTH, PLAIN_LINE),
            OverlayStyle::Selected => out.line(*a, *b2, SELECTED_LINE_WIDTH, selected),
            OverlayStyle::Hovered => out.line(*a, *b2, HOVERED_LINE_WIDTH, hovered),
        }
    }
    for (triangle, style) in &overlay.tints {
        let color = match style {
            OverlayStyle::Hovered => [hovered[0], hovered[1], hovered[2], HOVERED_TINT],
            OverlayStyle::Selected | OverlayStyle::Plain => [r, g, b, SELECTED_TINT],
        };
        out.triangle(*triangle, color);
    }
    for (at, style) in &overlay.points {
        match style {
            OverlayStyle::Plain => out.point(*at, POINT_SIZE, PLAIN_POINT),
            OverlayStyle::Selected => out.point(*at, SELECTED_POINT_SIZE, selected),
            OverlayStyle::Hovered => out.point(*at, HOVERED_POINT_SIZE, hovered),
        }
    }
    out
}

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
    /// The version of the scene drawing, and the selection colour, the
    /// renderer's shape was built from, if any.
    uploaded: Option<(u64, [f32; 3])>,
    /// The same for the overlay.
    uploaded_overlay: Option<(u64, [f32; 3])>,
}

impl GpuViewport {
    /// Sets up viewport rendering on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        Self {
            renderer: GpuRenderer::new(&state.device, samples),
            state: state.clone(),
            images: std::array::from_fn(|_| None),
            uploaded: None,
            uploaded_overlay: None,
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
            fine_spacing: view.grid_detail.fine_spacing(),
            flat_height: view.flat_height,
            grid_plane: view.grid_plane,
        };
        let wanted = (view.scene.version, view.selection_color);
        if self.uploaded != Some(wanted) {
            let instances: Vec<MeshInstance<'_>> = view
                .scene
                .items
                .iter()
                .map(|item| MeshInstance {
                    mesh: &item.mesh,
                    transform: item.transform,
                    // Objects being edited show their elements instead of
                    // an outline.
                    selected: item.selected && !view.scene.editing,
                })
                .collect();
            let [r, g, b] = view.selection_color.map(render::srgb_to_linear);
            let shape = Shape::from_meshes(&instances, [r, g, b, 0.9]);
            self.renderer.set_shape(&self.state.device, &shape);
            self.uploaded = Some(wanted);
        }
        let wanted = (view.overlay.version, view.selection_color);
        if self.uploaded_overlay != Some(wanted) {
            let accent = view.selection_color.map(render::srgb_to_linear);
            let overlay = overlay_geometry(&view.overlay, accent);
            self.renderer.set_overlay(&self.state.device, &overlay);
            self.uploaded_overlay = Some(wanted);
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
    use super::*;
    use glam::Vec3;

    #[test]
    fn the_overlay_turns_into_points_lines_and_tints() {
        let overlay = ComponentOverlay {
            version: 1,
            points: vec![
                (Vec3::ZERO, OverlayStyle::Plain),
                (Vec3::X, OverlayStyle::Hovered),
                (Vec3::Y, OverlayStyle::Selected),
            ],
            lines: vec![([Vec3::ZERO, Vec3::Y], OverlayStyle::Plain)],
            tints: vec![([Vec3::ZERO, Vec3::X, Vec3::Y], OverlayStyle::Selected)],
        };
        let geometry = overlay_geometry(&overlay, [1.0, 0.0, 0.0]);
        // Three points and one line, six corners each; one tinted triangle.
        assert_eq!(geometry.sprites.len(), 24);
        assert_eq!(geometry.tints.len(), 3);
        assert!(
            geometry
                .tints
                .iter()
                .all(|v| v.color == [1.0, 0.0, 0.0, SELECTED_TINT])
        );
        let hovered = &geometry.sprites[12..18];
        assert!(hovered.iter().all(|v| v.width == HOVERED_POINT_SIZE));
        // Lighter than the accent, so it stands out from the selection.
        assert!(
            hovered
                .iter()
                .all(|v| v.color[0] == 1.0 && v.color[1] > 0.3)
        );
        let selected = &geometry.sprites[18..];
        assert!(selected.iter().all(|v| v.width == SELECTED_POINT_SIZE));
        assert!(selected.iter().all(|v| v.color == [1.0, 0.0, 0.0, 1.0]));
        assert!(overlay_geometry(&ComponentOverlay::default(), [1.0; 3]).is_empty());
    }
}
