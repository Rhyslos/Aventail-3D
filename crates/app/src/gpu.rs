//! Connects the viewport renderer (the `render` crate) to the window's
//! wgpu device and to egui, so the panel can show the rendered image.

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use render::{
    EDGE_COLOR, FrameParams, MeshDraw, MeshGeometry, Overlay, PickRequest, Shape,
    ViewportRenderer as GpuRenderer, ViewportTarget,
};
use std::collections::HashMap;
use std::sync::Arc;
use ui::{
    ComponentOverlay, OverlayStyle, PickAsk, PickReply, Picked, SceneDrawing, Shading, VIEW_COUNT,
    ViewportRenderer, ViewportView,
};

/// Every point of an edited object in the Points mode: small light dots,
/// across in pixels.
const POINT_SIZE: f32 = 6.0;
/// Selected points: the largest dots, in the accent colour, so the
/// selection is the clearest thing in the view (decision 0126).
const SELECTED_POINT_SIZE: f32 = 11.0;
/// The point under the mouse: nearly as large, in a lighter accent.
const HOVERED_POINT_SIZE: f32 = 10.0;
/// Selected edges: the thickest lines in the view.
const SELECTED_LINE_WIDTH: f32 = 4.5;
/// The edge under the mouse, or the polygon's outline.
const HOVERED_LINE_WIDTH: f32 = 4.0;
/// With High contrast on: the selection larger still, with a dark rim
/// this many pixels wide around it, so it reads on any colour.
const CONTRAST_GROWTH: f32 = 1.5;
const CONTRAST_RIM: f32 = 3.0;
const CONTRAST_RIM_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.9];
/// Light grey, linear space, for an edited object's points and edges.
const PLAIN_POINT: [f32; 4] = [0.85, 0.85, 0.85, 0.95];
const PLAIN_LINE: [f32; 4] = [0.9, 0.9, 0.9, 0.45];
/// A selected object's edges: the accent, nearly opaque.
const SELECTED_EDGE_ALPHA: f32 = 0.9;
/// How faint a selected object's dense wireframe may fade from afar: a
/// little of the accent stays, so the selection still shows.
const SELECTED_FAINTEST: f32 = 0.25;
/// How see-through the tint on selected and hovered polygons is.
const SELECTED_TINT: f32 = 0.45;
const HOVERED_TINT: f32 = 0.3;
/// With High contrast, selected polygons are tinted more strongly.
const CONTRAST_TINT: f32 = 0.6;
/// Plain edges in the Shaded with wires look: clear light lines on the
/// grey surface, linear space.
const WIRE_LINE: [f32; 4] = [0.95, 0.95, 0.97, 0.4];
/// Plain edges in the Wireframe look, with no surface behind them.
const WIREFRAME_LINE: [f32; 4] = [0.85, 0.86, 0.9, 0.75];
/// How faint a dense wireframe may fade in the Wireframe look: with no
/// surface, a heavy shape seen from afar would otherwise vanish.
const WIREFRAME_FAINTEST: f32 = 0.3;

/// Hard edges (Display → Show hard edges): light cyan, thinner than the
/// selection and a little see-through, so they never outshine it
/// (decision 0126), linear space.
const HARD_LINE: [f32; 4] = [0.25, 0.75, 0.95, 0.7];
const HARD_LINE_WIDTH: f32 = 2.0;
/// Seams (Display → Show seams, and the UV mode): orange, as thick as hard
/// edges, drawn over them where both are, linear space.
const SEAM_LINE: [f32; 4] = [1.0, 0.36, 0.05, 0.85];
/// Edges where seams are preferred, and where none may go (decision
/// 0111): the UV Editor's pale orange and blue.
const SEAM_PREFER_LINE: [f32; 4] = [1.0, 0.58, 0.1, 0.7];
const SEAM_NEVER_LINE: [f32; 4] = [0.07, 0.38, 1.0, 0.85];
/// Normals (Display → Show normals): light violet, thin.
const NORMAL_LINE: [f32; 4] = [0.6, 0.5, 1.0, 1.0];
const NORMAL_LINE_WIDTH: f32 = 1.5;
/// How far towards white the hovered colour is from the accent, so what
/// is under the mouse stands out from what is selected.
const HOVER_LIFT: f32 = 0.45;

/// The order lines and points are drawn in: marks first, then what is
/// under the mouse, the selection last so it lies on top of everything
/// (decision 0126).
fn layer(style: OverlayStyle) -> u8 {
    match style {
        OverlayStyle::Normal => 0,
        OverlayStyle::SeamPrefer | OverlayStyle::SeamNever => 1,
        OverlayStyle::Hard => 2,
        OverlayStyle::Seam => 3,
        OverlayStyle::Hovered => 4,
        OverlayStyle::Selected => 5,
    }
}

/// The renderer's overlay for the interface's description of it: selected
/// elements in `accent` (linear colour), the hovered one lighter, the
/// selection drawn last and largest. With `contrast` (Settings → Interface
/// → High contrast), the selection is larger still, with a dark rim.
fn overlay_geometry(overlay: &ComponentOverlay, accent: [f32; 3], contrast: bool) -> Overlay {
    let [r, g, b] = accent;
    let lift = |c: f32| c + (1.0 - c) * HOVER_LIFT;
    let selected = [r, g, b, 1.0];
    let hovered = [lift(r), lift(g), lift(b), 1.0];
    let grow = if contrast { CONTRAST_GROWTH } else { 1.0 };
    let mut out = Overlay::default();
    for (triangle, style) in &overlay.tints {
        let color = match style {
            OverlayStyle::Hovered => [hovered[0], hovered[1], hovered[2], HOVERED_TINT],
            _ if contrast => [r, g, b, CONTRAST_TINT],
            _ => [r, g, b, SELECTED_TINT],
        };
        out.triangle(*triangle, color);
    }
    let mut lines: Vec<&([glam::Vec3; 2], OverlayStyle)> = overlay.lines.iter().collect();
    lines.sort_by_key(|(_, style)| layer(*style));
    for ([a, b2], style) in lines {
        let (width, color) = match style {
            OverlayStyle::Selected => (SELECTED_LINE_WIDTH * grow, selected),
            OverlayStyle::Hovered => (HOVERED_LINE_WIDTH * grow, hovered),
            OverlayStyle::Hard => (HARD_LINE_WIDTH, HARD_LINE),
            OverlayStyle::Seam => (HARD_LINE_WIDTH, SEAM_LINE),
            OverlayStyle::SeamPrefer => (NORMAL_LINE_WIDTH, SEAM_PREFER_LINE),
            OverlayStyle::SeamNever => (HARD_LINE_WIDTH, SEAM_NEVER_LINE),
            OverlayStyle::Normal => (NORMAL_LINE_WIDTH, NORMAL_LINE),
        };
        if contrast && matches!(style, OverlayStyle::Selected | OverlayStyle::Hovered) {
            out.line(*a, *b2, width + CONTRAST_RIM * 2.0, CONTRAST_RIM_COLOR);
        }
        out.line(*a, *b2, width, color);
    }
    let mut points: Vec<&(glam::Vec3, OverlayStyle)> = overlay.points.iter().collect();
    points.sort_by_key(|(_, style)| layer(*style));
    for (at, style) in points {
        let (size, color) = match style {
            OverlayStyle::Selected => (SELECTED_POINT_SIZE * grow, selected),
            OverlayStyle::Hovered => (HOVERED_POINT_SIZE * grow, hovered),
            OverlayStyle::Hard => (POINT_SIZE, HARD_LINE),
            OverlayStyle::Seam => (POINT_SIZE, SEAM_LINE),
            OverlayStyle::SeamPrefer => (POINT_SIZE, SEAM_PREFER_LINE),
            OverlayStyle::SeamNever => (POINT_SIZE, SEAM_NEVER_LINE),
            OverlayStyle::Normal => (POINT_SIZE, NORMAL_LINE),
        };
        if contrast && matches!(style, OverlayStyle::Selected | OverlayStyle::Hovered) {
            out.point(*at, size + CONTRAST_RIM * 2.0, CONTRAST_RIM_COLOR);
        }
        out.point(*at, size, color);
    }
    out
}

/// How each object's mesh is drawn: where it is, its edges in the accent
/// when selected (plain while its elements are being edited) and clearer
/// in the wire looks, and every point shown in the Points mode.
#[cfg(test)]
fn mesh_draws(scene: &SceneDrawing, accent: [f32; 3], shading: Shading) -> Vec<MeshDraw> {
    mesh_draws_showing(scene, accent, shading, ui::Surface::Plain, &HashMap::new())
}

/// The same, with surfaces showing `surface` (decision 0102); each
/// object's texel density, for the stretch view, from its uploaded
/// geometry in `densities` (by key).
fn mesh_draws_showing(
    scene: &SceneDrawing,
    accent: [f32; 3],
    shading: Shading,
    surface: ui::Surface,
    densities: &HashMap<u64, f32>,
) -> Vec<MeshDraw> {
    let [r, g, b] = accent;
    let (plain, edited_line, least) = match shading {
        Shading::Shaded => (EDGE_COLOR, PLAIN_LINE, 0.0),
        Shading::ShadedWithWires => (WIRE_LINE, PLAIN_LINE, 0.0),
        Shading::Wireframe => (
            WIREFRAME_LINE,
            [PLAIN_LINE[0], PLAIN_LINE[1], PLAIN_LINE[2], 0.8],
            WIREFRAME_FAINTEST,
        ),
    };
    scene
        .items
        .iter()
        .filter(|item| item.transform.is_finite())
        .map(|item| {
            let edited = item.selected && scene.editing;
            MeshDraw {
                key: item.id.get(),
                model: item.transform,
                edges: true,
                edge_color: if edited {
                    edited_line
                } else if item.selected {
                    [r, g, b, SELECTED_EDGE_ALPHA]
                } else {
                    plain
                },
                points: (edited && scene.points).then_some((PLAIN_POINT, POINT_SIZE)),
                faintest: if item.selected && !edited {
                    SELECTED_FAINTEST.max(least)
                } else {
                    least
                },
                palette: render::palette(&item.colors),
                surface: match surface {
                    ui::Surface::Plain => render::SurfaceDraw::Plain,
                    ui::Surface::Checker => render::SurfaceDraw::Checker {
                        squares: ui::checker_squares(scene.texture_size),
                    },
                    ui::Surface::Stretch => render::SurfaceDraw::Stretch {
                        density: densities.get(&item.id.get()).copied().unwrap_or(0.0),
                    },
                    ui::Surface::NumberedChecker => render::SurfaceDraw::NumberedChecker,
                },
            }
        })
        .collect()
}

/// From this many polygons a new shape's GPU geometry is worked out on
/// another thread, so the window keeps working
/// ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
const BACKGROUND_FROM: usize = 50_000;

/// How many pixels apart an object's edges appear in `view`, around its
/// middle: its average edge length seen from there. `None` when it can't
/// be told (the middle is behind the camera).
fn edge_spacing(view: &ViewportView, middle: glam::Vec3, length: f32) -> Option<f32> {
    let towards = (middle - view.camera_position).normalize_or_zero();
    let mut across = towards.cross(glam::Vec3::Z);
    if across.length() < 1e-3 {
        across = towards.cross(glam::Vec3::X);
    }
    let [w, h] = view.size_px.map(|v| v.max(1) as f32);
    let screen = |p: glam::Vec3| {
        let clip = view.view_projection * p.extend(1.0);
        (clip.w > 1e-5).then(|| glam::Vec2::new(clip.x / clip.w * w, clip.y / clip.w * h) * 0.5)
    };
    Some(screen(middle)?.distance(screen(middle + across.normalize_or_zero() * length)?))
}

/// Leaves out what would be too dense to see in `view`: every point of a
/// heavy shape seen from afar, and its edges unless it is selected (a
/// selected one keeps a faint tint). The shaders fade them out anyway;
/// leaving them out saves the graphics card drawing millions of
/// invisible dots and lines.
fn leave_out_dense(draws: &mut [MeshDraw], view: &ViewportView, meshes: &HashMap<u64, Uploaded>) {
    for (draw, item) in draws
        .iter_mut()
        .zip(view.scene.items.iter().filter(|i| i.transform.is_finite()))
    {
        let Some(uploaded) = meshes.get(&draw.key) else {
            continue;
        };
        let Some(bounds) = item.drawn.bounds() else {
            continue;
        };
        let scale = (draw.model.x_axis.truncate().length()
            + draw.model.y_axis.truncate().length()
            + draw.model.z_axis.truncate().length())
            / 3.0;
        let middle = draw.model.transform_point3(bounds.centre());
        let length = uploaded.geometry.edge_length * scale;
        let Some(spacing) = edge_spacing(view, middle, length) else {
            continue;
        };
        if spacing < POINTS_FADED_BELOW {
            draw.points = None;
        }
        if spacing < EDGES_FADED_BELOW && draw.faintest == 0.0 {
            draw.edges = false;
        }
    }
}

/// Below this many pixels apart, the shader has faded every point out.
const POINTS_FADED_BELOW: f32 = 2.0;
/// Below this many pixels apart, the shader has faded plain edges out.
const EDGES_FADED_BELOW: f32 = 1.5;

/// An object's mesh as last uploaded.
struct Uploaded {
    mesh: Arc<mesh::Mesh>,
    geometry: MeshGeometry,
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
    /// The version of the scene drawing, the selection colour and the
    /// shading's hardness the renderer's placements were made for, if any.
    uploaded: Option<(u64, [f32; 3], mesh::Hardness)>,
    /// Each object's mesh on the GPU.
    meshes: HashMap<u64, Uploaded>,
    /// Heavy shapes' geometry being worked out in the background.
    preparing: HashMap<u64, (Arc<mesh::Mesh>, std::thread::JoinHandle<MeshGeometry>)>,
    /// The same for the overlay.
    uploaded_overlay: Option<(u64, [f32; 3], bool)>,
    /// The colours the numbered checker image on the GPU was made in, if
    /// it has been made (decision 0107).
    grid_for: Option<[[u8; 3]; 2]>,
}

impl GpuViewport {
    /// Sets up viewport rendering on the window's GPU device.
    pub fn new(state: &RenderState) -> Self {
        let samples = render::best_sample_count(&state.adapter);
        let mut renderer = GpuRenderer::new(&state.device, samples);
        // Objects are drawn one by one from their meshes; the renderer's
        // own single shape (its test cube) isn't used.
        renderer.set_shape(&state.device, &Shape::default());
        Self {
            renderer,
            state: state.clone(),
            images: std::array::from_fn(|_| None),
            uploaded: None,
            meshes: HashMap::new(),
            preparing: HashMap::new(),
            uploaded_overlay: None,
            grid_for: None,
        }
    }

    /// Brings the objects' meshes on the GPU up to date with `scene`: a
    /// mesh that is the same isn't touched, one whose points only moved is
    /// updated around them, anything else is uploaded whole. A heavy new
    /// shape is worked out on another thread; until it is ready its
    /// object shows its previous shape, if it had one.
    fn upload_meshes(&mut self, scene: &SceneDrawing, hardness: mesh::Hardness) {
        let device = &self.state.device;
        let queue = &self.state.queue;
        let mut keys = Vec::with_capacity(scene.items.len());
        for item in &scene.items {
            let key = item.id.get();
            keys.push(key);
            let waiting = self
                .preparing
                .get(&key)
                .is_some_and(|(mesh, _)| Arc::ptr_eq(mesh, &item.drawn));
            // Shading with another hardness (Automatic hard edges switched)
            // is built afresh.
            let same = |old: &Uploaded| old.geometry.hardness() == hardness;
            let heavy = item.drawn.face_count() >= BACKGROUND_FROM;
            // A background rebuild of the shape shown now that has finished
            // is swapped in below.
            let ready = waiting
                && self
                    .preparing
                    .get(&key)
                    .is_some_and(|(_, handle)| handle.is_finished());
            match self.meshes.get_mut(&key) {
                Some(old) if !ready && same(old) && Arc::ptr_eq(&old.mesh, &item.drawn) => continue,
                Some(old)
                    if !ready
                        && same(old)
                        && !waiting
                        && let Some(moved) = item.drawn.moved_since(&old.mesh) =>
                {
                    // Shaded by angle, a move can turn edges sharp or soft.
                    // A light shape is then built afresh at once; a heavy
                    // one follows the move with the shading it had, and is
                    // rebuilt in the background (decision 0066).
                    let follows = old.geometry.can_follow(&old.mesh, &item.drawn, &moved);
                    if follows || heavy {
                        let ranges = old.geometry.moved(&item.drawn, &moved);
                        self.renderer
                            .update_mesh(queue, key, &old.geometry, &ranges);
                        old.mesh = Arc::clone(&item.drawn);
                        let idle = self
                            .preparing
                            .get(&key)
                            .is_none_or(|(_, handle)| handle.is_finished());
                        if !follows && idle {
                            let mesh = Arc::clone(&item.drawn);
                            let handle =
                                std::thread::spawn(move || MeshGeometry::shaded(&mesh, hardness));
                            self.preparing
                                .insert(key, (Arc::clone(&item.drawn), handle));
                        }
                        continue;
                    }
                }
                _ => {}
            }
            let finished = self
                .preparing
                .get(&key)
                .is_some_and(|(_, handle)| handle.is_finished());
            let geometry = if item.drawn.face_count() < BACKGROUND_FROM {
                Some(MeshGeometry::shaded(&item.drawn, hardness))
            } else if waiting && finished {
                self.preparing
                    .remove(&key)
                    .and_then(|(_, handle)| handle.join().ok())
            } else {
                // A newer shape waits until the one being worked out is
                // done, so changes in quick succession don't pile up work.
                if !self.preparing.contains_key(&key) || finished {
                    let mesh = Arc::clone(&item.drawn);
                    let handle = std::thread::spawn(move || MeshGeometry::shaded(&mesh, hardness));
                    self.preparing
                        .insert(key, (Arc::clone(&item.drawn), handle));
                }
                None
            };
            if let Some(mut geometry) = geometry {
                self.renderer.set_mesh(device, key, &geometry);
                geometry.forget_indices();
                self.meshes.insert(
                    key,
                    Uploaded {
                        mesh: Arc::clone(&item.drawn),
                        geometry,
                    },
                );
            }
        }
        self.meshes.retain(|key, _| keys.contains(key));
        self.preparing.retain(|key, _| keys.contains(key));
        self.renderer.keep_meshes(&keys);
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
        let hardness = view
            .auto_hard_angle
            .map_or(mesh::Hardness::Marked, mesh::Hardness::ByAngle);
        let wanted = (view.scene.version, view.selection_color, hardness);
        if self.uploaded != Some(wanted) || !self.preparing.is_empty() {
            self.upload_meshes(&view.scene, hardness);
            self.uploaded = Some(wanted);
        }
        // Placed for each view, as what is too dense to see depends on
        // how close the view is.
        let accent = view.selection_color.map(render::srgb_to_linear);
        let densities: HashMap<u64, f32> = self
            .meshes
            .iter()
            .map(|(key, uploaded)| (*key, uploaded.geometry.uv_density))
            .collect();
        // The numbered checker is made when first shown, and again when
        // the accent colour changes.
        if view.surface == ui::Surface::NumberedChecker && self.grid_for != Some(view.grid_colors) {
            let image = ui::grid_image(view.grid_colors);
            self.renderer.set_grid_image(
                &self.state.device,
                &self.state.queue,
                ui::GRID_IMAGE_SIZE as u32,
                &image,
            );
            self.grid_for = Some(view.grid_colors);
        }
        let mut draws =
            mesh_draws_showing(&view.scene, accent, view.shading, view.surface, &densities);
        self.renderer.set_show_faces(view.shading.shows_faces());
        self.renderer.set_show_cavity(view.cavity);
        leave_out_dense(&mut draws, view, &self.meshes);
        self.renderer
            .set_draws(&self.state.device, &self.state.queue, &draws);
        let wanted = (
            view.overlay.version,
            view.selection_color,
            view.high_contrast,
        );
        if self.uploaded_overlay != Some(wanted) {
            let accent = view.selection_color.map(render::srgb_to_linear);
            let overlay = overlay_geometry(&view.overlay, accent, view.high_contrast);
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

    fn preparing(&self) -> bool {
        !self.preparing.is_empty()
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
                (Vec3::Y, OverlayStyle::Selected),
                (Vec3::X, OverlayStyle::Hovered),
            ],
            lines: vec![([Vec3::ZERO, Vec3::Y], OverlayStyle::Selected)],
            tints: vec![([Vec3::ZERO, Vec3::X, Vec3::Y], OverlayStyle::Selected)],
        };
        let geometry = overlay_geometry(&overlay, [1.0, 0.0, 0.0], false);
        // One line and two points; one tinted triangle.
        assert_eq!(geometry.sprites.len(), 3);
        assert_eq!(geometry.tints.len(), 3);
        assert!(
            geometry
                .tints
                .iter()
                .all(|v| v.color == [1.0, 0.0, 0.0, SELECTED_TINT])
        );
        // The hovered point comes before the selected one: the selection
        // is drawn last, on top.
        let hovered = &geometry.sprites[1];
        assert_eq!(hovered.width, HOVERED_POINT_SIZE);
        // Lighter than the accent, so it stands out from the selection.
        assert!(hovered.color[0] == 1.0 && hovered.color[1] > 0.3);
        let selected = &geometry.sprites[2];
        assert_eq!(selected.width, SELECTED_POINT_SIZE);
        assert_eq!(selected.color, [1.0, 0.0, 0.0, 1.0]);
        assert!(overlay_geometry(&ComponentOverlay::default(), [1.0; 3], false).is_empty());
    }

    #[test]
    fn the_selection_outshines_hard_edges_and_seams_and_lies_on_top() {
        // The review's finding: a selected edge looked thinner and duller
        // than the hard edges and seams around it.
        let overlay = ComponentOverlay {
            version: 1,
            points: Vec::new(),
            lines: vec![
                ([Vec3::ZERO, Vec3::X], OverlayStyle::Selected),
                ([Vec3::ZERO, Vec3::Y], OverlayStyle::Hard),
                ([Vec3::ZERO, Vec3::Z], OverlayStyle::Seam),
                ([Vec3::X, Vec3::Y], OverlayStyle::Hovered),
            ],
            tints: Vec::new(),
        };
        let geometry = overlay_geometry(&overlay, [0.2, 0.5, 1.0], false);
        let widths: Vec<f32> = geometry.sprites.iter().map(|s| s.width).collect();
        let last = geometry.sprites.last().unwrap();
        assert_eq!(
            last.width, SELECTED_LINE_WIDTH,
            "the selection is drawn last"
        );
        assert!(widths.iter().all(|w| *w <= SELECTED_LINE_WIDTH));
        const { assert!(SELECTED_LINE_WIDTH > HARD_LINE_WIDTH * 2.0) };
        // Marks are see-through.
        const { assert!(HARD_LINE[3] < 1.0 && SEAM_LINE[3] < 1.0) };
    }

    #[test]
    fn high_contrast_makes_the_selection_larger_with_a_dark_rim() {
        let overlay = ComponentOverlay {
            version: 1,
            points: vec![(Vec3::Y, OverlayStyle::Selected)],
            lines: vec![
                ([Vec3::ZERO, Vec3::X], OverlayStyle::Selected),
                ([Vec3::ZERO, Vec3::Y], OverlayStyle::Hard),
            ],
            tints: vec![([Vec3::ZERO, Vec3::X, Vec3::Y], OverlayStyle::Selected)],
        };
        let plain = overlay_geometry(&overlay, [1.0, 0.0, 0.0], false);
        let contrast = overlay_geometry(&overlay, [1.0, 0.0, 0.0], true);
        // A rim behind each selected line and point.
        assert_eq!(contrast.sprites.len(), plain.sprites.len() + 2);
        let rims: Vec<_> = contrast
            .sprites
            .iter()
            .filter(|s| s.color == CONTRAST_RIM_COLOR)
            .collect();
        assert_eq!(rims.len(), 2);
        let line = contrast
            .sprites
            .iter()
            .find(|s| s.color == [1.0, 0.0, 0.0, 1.0] && s.a != s.b)
            .unwrap();
        assert!(line.width > SELECTED_LINE_WIDTH);
        assert!(contrast.tints.iter().all(|v| v.color[3] == CONTRAST_TINT));
        // Hard edges stay as they are.
        assert!(contrast.sprites.iter().any(|s| s.width == HARD_LINE_WIDTH));
    }

    fn item(id: u64, selected: bool) -> ui::DrawItem {
        ui::DrawItem {
            id: scene::ObjectId::from_raw(id),
            mesh: Arc::new(mesh::PrimitiveKind::Cube.standard().mesh()),
            shown: Arc::new(mesh::PrimitiveKind::Cube.standard().mesh()),
            drawn: Arc::new(mesh::PrimitiveKind::Cube.standard().mesh()),
            transform: glam::Mat4::from_translation(Vec3::X * id as f32),
            selected,
            colors: Vec::new(),
        }
    }

    #[test]
    fn objects_are_drawn_where_they_are_with_their_edges_coloured() {
        let scene = |editing, points| SceneDrawing {
            version: 1,
            items: vec![item(1, true), item(2, false)],
            editing,
            points,
            mirrors: Vec::new(),
            texture_size: 2048,
        };
        let draws = mesh_draws(&scene(false, false), [1.0, 0.0, 0.0], Shading::Shaded);
        assert_eq!(draws.len(), 2);
        assert_eq!(draws[0].key, 1);
        assert_eq!(draws[0].model.w_axis.x, 1.0);
        assert_eq!(draws[0].edge_color, [1.0, 0.0, 0.0, SELECTED_EDGE_ALPHA]);
        assert_eq!(draws[1].edge_color, EDGE_COLOR);
        assert!(draws.iter().all(|d| d.points.is_none()));
        // Edited: plain edges, and points in the Points mode.
        let draws = mesh_draws(&scene(true, true), [1.0, 0.0, 0.0], Shading::Shaded);
        assert_eq!(draws[0].edge_color, PLAIN_LINE);
        assert_eq!(draws[0].points, Some((PLAIN_POINT, POINT_SIZE)));
        assert_eq!(draws[1].points, None, "only the edited object");
        // A broken placement isn't drawn.
        let mut broken = scene(false, false);
        broken.items[1].transform = glam::Mat4::from_scale(Vec3::NAN);
        assert_eq!(mesh_draws(&broken, [1.0; 3], Shading::Shaded).len(), 1);
    }

    #[test]
    fn the_wire_looks_draw_edges_clearly() {
        let scene = SceneDrawing {
            version: 1,
            items: vec![item(1, true), item(2, false)],
            editing: false,
            points: false,
            mirrors: Vec::new(),
            texture_size: 2048,
        };
        let wires = mesh_draws(&scene, [1.0, 0.0, 0.0], Shading::ShadedWithWires);
        assert_eq!(wires[1].edge_color, WIRE_LINE);
        const { assert!(WIRE_LINE[3] > EDGE_COLOR[3] * 2.0, "clearly stronger") };
        assert_eq!(wires[0].edge_color, [1.0, 0.0, 0.0, SELECTED_EDGE_ALPHA]);
        let wireframe = mesh_draws(&scene, [1.0, 0.0, 0.0], Shading::Wireframe);
        assert_eq!(wireframe[1].edge_color, WIREFRAME_LINE);
        // With no surface, even a heavy shape keeps a faint outline.
        assert!(wireframe.iter().all(|d| d.faintest >= WIREFRAME_FAINTEST));
        assert!(!Shading::Wireframe.shows_faces() && Shading::ShadedWithWires.shows_faces());
    }

    /// A view of `scene` from `distance` centimetres in front of the
    /// origin, 800 × 600 pixels.
    fn view_from(distance: f32, scene: SceneDrawing) -> ViewportView {
        let camera =
            tools::Camera::looking_at(Vec3::new(0.0, -distance, 100.0), Vec3::new(0.0, 0.0, 100.0));
        ViewportView {
            slot: 0,
            size_px: [800, 600],
            view_projection: camera.view_projection(glam::Vec2::new(800.0, 600.0)),
            camera_position: camera.position,
            focus: Vec3::ZERO,
            flat_height: None,
            grid_plane: render::GROUND_PLANE,
            sky_top: [0.0; 3],
            sky_horizon: [0.0; 3],
            ground: [0.0; 3],
            grid_detail: ui::GridDetail::default(),
            scene: Arc::new(scene),
            selection_color: [1.0; 3],
            grid_colors: [[200, 60, 60], [60, 60, 200]],
            overlay: Arc::new(ComponentOverlay::default()),
            shading: Shading::default(),
            auto_hard_angle: Some(ui::AUTO_HARD_ANGLE),
            cavity: true,
            surface: ui::Surface::Plain,
            high_contrast: false,
        }
    }

    #[test]
    fn dense_points_and_edges_are_left_out_from_afar() {
        let ball = Arc::new(mesh::samples::heavy_ball(40_000, 100.0));
        let scene = |selected| SceneDrawing {
            version: 1,
            items: vec![ui::DrawItem {
                id: scene::ObjectId::from_raw(1),
                mesh: Arc::clone(&ball),
                shown: Arc::clone(&ball),
                drawn: Arc::clone(&ball),
                transform: glam::Mat4::IDENTITY,
                selected,
                colors: Vec::new(),
            }],
            editing: true,
            points: true,
            mirrors: Vec::new(),
            texture_size: 2048,
        };
        let mut meshes = HashMap::new();
        meshes.insert(
            1,
            Uploaded {
                mesh: Arc::clone(&ball),
                geometry: MeshGeometry::new(&ball),
            },
        );
        let draws = |distance, selected| {
            let view = view_from(distance, scene(selected));
            let mut draws = mesh_draws(&view.scene, [1.0, 0.0, 0.0], Shading::Shaded);
            leave_out_dense(&mut draws, &view, &meshes);
            draws[0]
        };
        // From afar, the edited ball's points and edges are too dense.
        let far = draws(2_000.0, true);
        assert!(far.points.is_none() && !far.edges);
        // Close up, they show.
        let near = draws(150.0, true);
        assert!(near.points.is_some() && near.edges);
        // A selected ball being looked at (not edited) keeps its edges
        // for the faint tint.
        let view = view_from(
            2_000.0,
            SceneDrawing {
                editing: false,
                ..scene(true)
            },
        );
        let mut tinted = mesh_draws(&view.scene, [1.0, 0.0, 0.0], Shading::Shaded);
        leave_out_dense(&mut tinted, &view, &meshes);
        assert!(tinted[0].edges && tinted[0].faintest > 0.0);
        // The spacing grows as the view comes closer.
        let spacing =
            |d| edge_spacing(&view_from(d, scene(false)), Vec3::new(0.0, 0.0, 100.0), 1.0).unwrap();
        assert!(spacing(200.0) > spacing(2_000.0) * 5.0);
    }
}
