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
/// Selected points: larger, in the accent colour.
const SELECTED_POINT_SIZE: f32 = 9.0;
/// The point under the mouse: larger still, in a lighter accent.
const HOVERED_POINT_SIZE: f32 = 10.0;
/// Selected edges.
const SELECTED_LINE_WIDTH: f32 = 3.0;
/// The edge under the mouse, or the polygon's outline.
const HOVERED_LINE_WIDTH: f32 = 3.5;
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
/// Plain edges in the Shaded with wires look: clear light lines on the
/// grey surface, linear space.
const WIRE_LINE: [f32; 4] = [0.95, 0.95, 0.97, 0.4];
/// Plain edges in the Wireframe look, with no surface behind them.
const WIREFRAME_LINE: [f32; 4] = [0.85, 0.86, 0.9, 0.75];
/// How faint a dense wireframe may fade in the Wireframe look: with no
/// surface, a heavy shape seen from afar would otherwise vanish.
const WIREFRAME_FAINTEST: f32 = 0.3;

/// Hard edges (Display → Hard edges): light cyan, a little thicker than
/// the plain edges, linear space.
const HARD_LINE: [f32; 4] = [0.25, 0.75, 0.95, 1.0];
const HARD_LINE_WIDTH: f32 = 2.5;
/// Normals (Display → Normals): light violet, thin.
const NORMAL_LINE: [f32; 4] = [0.6, 0.5, 1.0, 1.0];
const NORMAL_LINE_WIDTH: f32 = 1.5;
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
            OverlayStyle::Selected => out.line(*a, *b2, SELECTED_LINE_WIDTH, selected),
            OverlayStyle::Hovered => out.line(*a, *b2, HOVERED_LINE_WIDTH, hovered),
            OverlayStyle::Hard => out.line(*a, *b2, HARD_LINE_WIDTH, HARD_LINE),
            OverlayStyle::Normal => out.line(*a, *b2, NORMAL_LINE_WIDTH, NORMAL_LINE),
        }
    }
    for (triangle, style) in &overlay.tints {
        let color = match style {
            OverlayStyle::Hovered => [hovered[0], hovered[1], hovered[2], HOVERED_TINT],
            OverlayStyle::Selected | OverlayStyle::Hard | OverlayStyle::Normal => {
                [r, g, b, SELECTED_TINT]
            }
        };
        out.triangle(*triangle, color);
    }
    for (at, style) in &overlay.points {
        match style {
            OverlayStyle::Selected => out.point(*at, SELECTED_POINT_SIZE, selected),
            OverlayStyle::Hovered => out.point(*at, HOVERED_POINT_SIZE, hovered),
            OverlayStyle::Hard => out.point(*at, SELECTED_POINT_SIZE, HARD_LINE),
            OverlayStyle::Normal => out.point(*at, POINT_SIZE, NORMAL_LINE),
        }
    }
    out
}

/// How each object's mesh is drawn: where it is, its edges in the accent
/// when selected (plain while its elements are being edited) and clearer
/// in the wire looks, and every point shown in the Points mode.
fn mesh_draws(scene: &SceneDrawing, accent: [f32; 3], shading: Shading) -> Vec<MeshDraw> {
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
        let Some(bounds) = item.shown.bounds() else {
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
    uploaded_overlay: Option<(u64, [f32; 3])>,
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
                .is_some_and(|(mesh, _)| Arc::ptr_eq(mesh, &item.shown));
            // Shading with another hardness (Automatic hard edges switched)
            // is built afresh.
            let same = |old: &Uploaded| old.geometry.hardness() == hardness;
            let heavy = item.shown.face_count() >= BACKGROUND_FROM;
            // A background rebuild of the shape shown now that has finished
            // is swapped in below.
            let ready = waiting
                && self
                    .preparing
                    .get(&key)
                    .is_some_and(|(_, handle)| handle.is_finished());
            match self.meshes.get_mut(&key) {
                Some(old) if !ready && same(old) && Arc::ptr_eq(&old.mesh, &item.shown) => continue,
                Some(old)
                    if !ready
                        && same(old)
                        && !waiting
                        && let Some(moved) = item.shown.moved_since(&old.mesh) =>
                {
                    // Shaded by angle, a move can turn edges sharp or soft.
                    // A light shape is then built afresh at once; a heavy
                    // one follows the move with the shading it had, and is
                    // rebuilt in the background (decision 0066).
                    let follows = old.geometry.can_follow(&old.mesh, &item.shown, &moved);
                    if follows || heavy {
                        let ranges = old.geometry.moved(&item.shown, &moved);
                        self.renderer
                            .update_mesh(queue, key, &old.geometry, &ranges);
                        old.mesh = Arc::clone(&item.shown);
                        let idle = self
                            .preparing
                            .get(&key)
                            .is_none_or(|(_, handle)| handle.is_finished());
                        if !follows && idle {
                            let mesh = Arc::clone(&item.shown);
                            let handle =
                                std::thread::spawn(move || MeshGeometry::shaded(&mesh, hardness));
                            self.preparing
                                .insert(key, (Arc::clone(&item.shown), handle));
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
            let geometry = if item.shown.face_count() < BACKGROUND_FROM {
                Some(MeshGeometry::shaded(&item.shown, hardness))
            } else if waiting && finished {
                self.preparing
                    .remove(&key)
                    .and_then(|(_, handle)| handle.join().ok())
            } else {
                // A newer shape waits until the one being worked out is
                // done, so changes in quick succession don't pile up work.
                if !self.preparing.contains_key(&key) || finished {
                    let mesh = Arc::clone(&item.shown);
                    let handle = std::thread::spawn(move || MeshGeometry::shaded(&mesh, hardness));
                    self.preparing
                        .insert(key, (Arc::clone(&item.shown), handle));
                }
                None
            };
            if let Some(mut geometry) = geometry {
                self.renderer.set_mesh(device, key, &geometry);
                geometry.forget_indices();
                self.meshes.insert(
                    key,
                    Uploaded {
                        mesh: Arc::clone(&item.shown),
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
        let mut draws = mesh_draws(&view.scene, accent, view.shading);
        self.renderer.set_show_faces(view.shading.shows_faces());
        self.renderer.set_show_cavity(view.cavity);
        leave_out_dense(&mut draws, view, &self.meshes);
        self.renderer
            .set_draws(&self.state.device, &self.state.queue, &draws);
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
                (Vec3::X, OverlayStyle::Hovered),
                (Vec3::Y, OverlayStyle::Selected),
            ],
            lines: vec![([Vec3::ZERO, Vec3::Y], OverlayStyle::Selected)],
            tints: vec![([Vec3::ZERO, Vec3::X, Vec3::Y], OverlayStyle::Selected)],
        };
        let geometry = overlay_geometry(&overlay, [1.0, 0.0, 0.0]);
        // Two points and one line; one tinted triangle.
        assert_eq!(geometry.sprites.len(), 3);
        assert_eq!(geometry.tints.len(), 3);
        assert!(
            geometry
                .tints
                .iter()
                .all(|v| v.color == [1.0, 0.0, 0.0, SELECTED_TINT])
        );
        let hovered = &geometry.sprites[1..2];
        assert!(hovered.iter().all(|v| v.width == HOVERED_POINT_SIZE));
        // Lighter than the accent, so it stands out from the selection.
        assert!(
            hovered
                .iter()
                .all(|v| v.color[0] == 1.0 && v.color[1] > 0.3)
        );
        let selected = &geometry.sprites[2..];
        assert!(selected.iter().all(|v| v.width == SELECTED_POINT_SIZE));
        assert!(selected.iter().all(|v| v.color == [1.0, 0.0, 0.0, 1.0]));
        assert!(overlay_geometry(&ComponentOverlay::default(), [1.0; 3]).is_empty());
    }

    fn item(id: u64, selected: bool) -> ui::DrawItem {
        ui::DrawItem {
            id: scene::ObjectId::from_raw(id),
            mesh: Arc::new(mesh::PrimitiveKind::Cube.standard().mesh()),
            shown: Arc::new(mesh::PrimitiveKind::Cube.standard().mesh()),
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
            overlay: Arc::new(ComponentOverlay::default()),
            shading: Shading::default(),
            auto_hard_angle: Some(ui::AUTO_HARD_ANGLE),
            cavity: true,
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
                transform: glam::Mat4::IDENTITY,
                selected,
                colors: Vec::new(),
            }],
            editing: true,
            points: true,
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
