//! Painting with the brush in Paint mode ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)).
//!
//! A stroke begins where the mouse is pressed on a shape: it paints the
//! texture set of the material there, into its top layer (a "Paint" layer
//! is made if the set has none, and the set itself if the material has
//! none). Every frame of the stroke the layer is given to the document
//! through `Command::SetTextureSet`, merged into one undo step, so the
//! views show the paint as it goes on and undo takes the whole stroke.
//!
//! In the UV Editor (decision 0180) the same brush paints flat on the
//! texture: only inside the UV pieces of the texture set's material, on
//! every shown object that uses it.

use super::{LayoutChoice, Workbench};
use crate::attributes::colour_picker::PaletteRequest;
use crate::attributes::{BrushSettings, PaintTool};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::uv_editor::brushing::UvPointer;
use crate::viewport::{BrushEvent, Surface};
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Vec2, Vec3};
use paint::{
    BrushKind, Dab, DepthMap, Layer, PaintSurface, StrokeCanvas, Target, TextureSet, View,
};
use scene::{Mesh, ObjectId};
use std::collections::HashMap;
use std::sync::Arc;

/// Merge keys of paint strokes: apart from every other drag's.
const STROKE_KEYS: u64 = 0x5041_494E_0000_0000;

/// How much [ and ] change the brush's size.
const SIZE_STEP: f32 = 1.15;

/// The paint brush's state on the workbench.
#[derive(Debug, Default)]
pub(super) struct Brushing {
    /// Each object's shape as the brush sees it, made again when the
    /// shape changes.
    surfaces: HashMap<ObjectId, (Arc<Mesh>, Arc<PaintSurface>)>,
    /// The stroke under way.
    session: Option<BrushSession>,
    /// Strokes made so far, for their undo merge keys.
    strokes: u64,
    /// The surface the viewport showed before Paint mode, put back after.
    surface_before: Option<Surface>,
    /// Whether Paint mode was on last frame.
    on: bool,
    /// The layout before Paint mode switched to the Paint one, put back
    /// after (decision 0181).
    layout_before: Option<super::LayoutChoice>,
    /// P was pressed: pick the colour under the mouse (decision 0183).
    pick_now: bool,
    /// A press picked a colour: the drag that follows paints nothing.
    picked_press: bool,
}

/// A stroke under way.
#[derive(Debug)]
struct BrushSession {
    material: String,
    layer: paint::LayerPath,
    set: TextureSet,
    canvas: StrokeCanvas,
    /// Where it is painted: on the model in a 3D view, or flat in the UV
    /// Editor.
    place: StrokePlace,
    /// The mouse's last place, in screen points (in the UV Editor, the
    /// place on the texture times the zoom).
    last: Vec2,
    since: Option<f32>,
    key: u64,
    /// The stroke as painted so far, kept with the layer (decision 0179).
    record: paint::StrokeRecord,
    /// The layer's strokes before this one.
    strokes_before: Vec<Arc<paint::StrokeRecord>>,
    /// Whether it paints the layer's mask (decision 0182).
    on_mask: bool,
    /// The triangles the polygon fill tool has filled in this drag, by
    /// shape (decision 0182).
    filled: std::collections::HashSet<(usize, usize)>,
}

/// Where a stroke is painted. (Only one stroke is under way at a time,
/// so the 3D view's larger variant costs nothing worth boxing.)
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum StrokePlace {
    /// On the model, in a 3D view.
    Model {
        slots: Vec<u32>,
        surface: Arc<PaintSurface>,
        model: Mat4,
        view: View,
        depth: DepthMap,
        camera: tools::Camera,
        size: Vec2,
    },
    /// Flat on the texture, in the UV Editor.
    Flat {
        /// Each shown object's shape using the material, and its slots
        /// that do.
        targets: Vec<(Arc<PaintSurface>, Vec<u32>)>,
        /// Screen points per texture width when the stroke began.
        zoom: f32,
        /// Scene units per texture width where the stroke began, for a
        /// brush measured on the surface.
        units_per_uv: f32,
    },
}

/// What a ray from the mouse hits.
struct Hit {
    id: ObjectId,
    world: Vec3,
    triangle: usize,
    surface: Arc<PaintSurface>,
    model: Mat4,
}

impl Workbench {
    /// Paint mode on or off, each frame before the viewport is drawn: the
    /// brush shown in the views, its settings in Attributes, and the
    /// Painted surface while painting.
    pub(super) fn update_paint_mode(&mut self, drawing: &SceneDrawing) {
        let on = self.mode == Mode::Paint;
        if on && !self.brushing.on {
            self.brushing.surface_before = Some(self.viewport.options().surface);
            self.viewport.options_mut().surface = Surface::Painted;
            // The Paint layout (decision 0181), and back after.
            if self.layout_choice != LayoutChoice::Paint {
                self.brushing.layout_before = Some(self.layout_choice);
                self.swap_layout(LayoutChoice::Paint);
            }
        }
        if !on && self.brushing.on {
            if let Some(before) = self.brushing.surface_before.take()
                && self.viewport.options().surface == Surface::Painted
            {
                self.viewport.options_mut().surface = before;
            }
            self.brushing.session = None;
            let before = self.brushing.layout_before.take();
            if self.layout_choice == LayoutChoice::Paint {
                let back = before.unwrap_or(match self.mode {
                    Mode::Uv => LayoutChoice::UvEdit,
                    _ => LayoutChoice::Standard,
                });
                self.swap_layout(back);
            }
        }
        self.brushing.on = on;
        self.attributes.painting = on;
        if !on {
            self.viewport.set_paint_brush(None);
            self.uv_editor.set_paint_brush(None);
            return;
        }
        // What is under the mouse: the size on screen of a brush measured
        // on the surface, and what it would paint.
        let hover = self.viewport.pointer().and_then(|p| {
            let hit = self.brush_hit(drawing, &p.camera, p.size, p.at)?;
            Some((p, hit))
        });
        let settings = self.attributes.brush;
        let mut radius = settings.brush.radius;
        self.attributes.brush_target = Default::default();
        if let Some((pointer, hit)) = &hover {
            let per_point = pointer
                .camera
                .world_per_pixel(hit.world, pointer.size)
                .max(1e-6);
            if settings.on_surface {
                radius = settings.surface_radius / per_point;
            }
            let triangle = hit.surface.triangles()[hit.triangle];
            self.attributes.brush_target =
                self.target_of(hit.id, &triangle, hit.model, radius * per_point);
        }
        self.viewport
            .set_paint_brush(Some((radius.max(0.5), settings.brush.hardness)));
        // The UV Editor's brush: its size on the texture, and what it
        // would paint when the mouse is there instead.
        let mut flat_radius = settings.brush.radius;
        if let Some(pointer) = self.uv_editor.brush_hover()
            && let Some((id, triangle, units_per_uv)) = self.flat_hit(drawing, pointer.uv)
        {
            let units_per_point = units_per_uv / pointer.zoom.max(1e-6);
            if settings.on_surface {
                flat_radius = settings.surface_radius / units_per_point.max(1e-9);
            }
            if hover.is_none() {
                let model = drawn_transform(drawing, id);
                self.attributes.brush_target =
                    self.target_of(id, &triangle, model, flat_radius * units_per_point);
            }
        }
        self.uv_editor
            .set_paint_brush(Some((flat_radius.max(0.5), settings.brush.hardness)));
    }

    /// What the brush would paint on `triangle` of object `id` (placed by
    /// `model`), `radius` scene units across, for Attributes.
    fn target_of(
        &self,
        id: ObjectId,
        triangle: &paint::surface::PaintTriangle,
        model: Mat4,
        radius: f32,
    ) -> crate::attributes::BrushTarget {
        let material = self.material_at(id, triangle.slot);
        let set = self.document.scene().paint().set(&material).cloned();
        let size = set
            .as_ref()
            .map_or(self.document.scene().texture().size, |s| s.size);
        let layer = set
            .as_ref()
            .and_then(|s| {
                let path = self.paint_target(&material, s)?;
                s.layer(&path).filter(|l| l.image().is_some())
            })
            .map_or_else(|| tr("Paint").to_string(), |l| l.name.clone());
        crate::attributes::BrushTarget {
            layer,
            material,
            texel: texel_width(triangle, model, size),
            radius: Some(radius),
        }
    }

    /// The brush's events from the views, after they are drawn.
    pub(super) fn carry_out_brush(&mut self, drawing: &SceneDrawing) {
        if std::mem::take(&mut self.brushing.pick_now) {
            let picked = match (self.viewport.pointer(), self.uv_editor.brush_hover()) {
                (Some(p), _) => self.pick_on_model(drawing, &p.camera, p.size, p.at),
                (None, Some(p)) => self.pick_flat(drawing, p.uv),
                (None, None) => false,
            };
            if !picked {
                self.note = Some(
                    tr("Point at a shape (or a UV piece) and press P to take its colour.")
                        .to_string(),
                );
            }
        }
        for event in self.viewport.take_brush_events() {
            match event {
                BrushEvent::Begin(p) if self.attributes.picking => {
                    self.attributes.picking = false;
                    self.brushing.picked_press = true;
                    self.pick_on_model(drawing, &p.camera, p.size, p.at);
                }
                BrushEvent::Begin(p) => self.begin_stroke(drawing, &p.camera, p.size, p.at),
                BrushEvent::Move(_) if self.brushing.picked_press => {}
                BrushEvent::Move(p) => self.continue_stroke(p.at),
                BrushEvent::End => {
                    self.brushing.session = None;
                    self.brushing.picked_press = false;
                }
                BrushEvent::Adjust {
                    size,
                    hardness,
                    flow,
                } => self.adjust_brush(size, hardness, flow),
            }
        }
        for event in self.uv_editor.take_brush_events() {
            match event {
                BrushEvent::Begin(p) if self.attributes.picking => {
                    self.attributes.picking = false;
                    self.brushing.picked_press = true;
                    self.pick_flat(drawing, p.uv);
                }
                BrushEvent::Begin(p) => self.begin_flat_stroke(drawing, p),
                BrushEvent::Move(_) if self.brushing.picked_press => {}
                BrushEvent::Move(p) => self.continue_flat_stroke(p),
                BrushEvent::End => {
                    self.brushing.session = None;
                    self.brushing.picked_press = false;
                }
                BrushEvent::Adjust {
                    size,
                    hardness,
                    flow,
                } => self.adjust_brush(size, hardness, flow),
            }
        }
    }

    /// A Ctrl + drag's change to the brush: its size times `size`, its
    /// hardness and flow plus the others.
    fn adjust_brush(&mut self, size: f32, hardness: f32, flow: f32) {
        let settings = &mut self.attributes.brush;
        if settings.on_surface {
            settings.surface_radius = (settings.surface_radius * size).clamp(
                crate::attributes::BRUSH_SURFACE_RADIUS.0,
                crate::attributes::BRUSH_SURFACE_RADIUS.1,
            );
        } else {
            settings.brush.radius *= size;
        }
        settings.brush.hardness += hardness;
        settings.brush.flow += flow;
        settings.brush = settings.brush.clamped();
    }

    /// Paint mode's keys: 1 paint, 2 erase and 4 polygon fill (3, 5 and
    /// 6 are the tools still to come), P picks a colour (decision 0183), [ and ] the size, Esc back to the Model mode.
    pub(super) fn paint_keys(&mut self, ui: &Ui) {
        if self.mode != Mode::Paint || ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |ui: &Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(ui, Key::Num1) {
            self.attributes.brush.tool = PaintTool::Brush;
            self.attributes.brush.brush.kind = BrushKind::Paint;
        }
        if pressed(ui, Key::Num2) {
            self.attributes.brush.tool = PaintTool::Brush;
            self.attributes.brush.brush.kind = BrushKind::Erase;
        }
        if pressed(ui, Key::Num4) {
            self.attributes.brush.tool = PaintTool::Fill;
        }
        if pressed(ui, Key::P) {
            self.brushing.pick_now = true;
        }
        for key in [Key::Num3, Key::Num5, Key::Num6] {
            if pressed(ui, key) {
                self.note = Some(
                    tr(
                        "Projection, smudge and clone come in later steps of the texture \
                         painter. 1 paints, 2 erases, 4 fills polygons.",
                    )
                    .to_string(),
                );
            }
        }
        let settings = &mut self.attributes.brush;
        for (key, factor) in [
            (Key::OpenBracket, 1.0 / SIZE_STEP),
            (Key::CloseBracket, SIZE_STEP),
        ] {
            if pressed(ui, key) {
                if settings.on_surface {
                    settings.surface_radius = (settings.surface_radius * factor).clamp(
                        crate::attributes::BRUSH_SURFACE_RADIUS.0,
                        crate::attributes::BRUSH_SURFACE_RADIUS.1,
                    );
                } else {
                    settings.brush.radius *= factor;
                    settings.brush = settings.brush.clamped();
                }
            }
        }
        if self.brushing.session.is_none() && pressed(ui, Key::Escape) {
            self.mode = Mode::Model;
        }
    }

    /// Takes the colour shown on the shape under `at` (points, in a view
    /// of `size` seen through `camera`) as the brush's. Whether there was
    /// one.
    fn pick_on_model(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) -> bool {
        let Some(hit) = self.brush_hit(drawing, camera, size, at) else {
            return false;
        };
        let triangle = hit.surface.triangles()[hit.triangle];
        let local = hit.model.inverse().transform_point3(hit.world);
        let uv = uv_at(&triangle, local);
        self.take_colour(hit.id, triangle.slot, uv)
    }

    /// Takes the colour shown at `uv` in the UV Editor as the brush's.
    fn pick_flat(&mut self, drawing: &SceneDrawing, uv: Vec2) -> bool {
        let Some((id, triangle, _)) = self.flat_hit(drawing, uv) else {
            return false;
        };
        self.take_colour(id, triangle.slot, uv)
    }

    /// Takes the colour object `id`'s material in `slot` shows at `uv`:
    /// its painted layers mixed over its own colour, averaged over 3 × 3
    /// texels (decision 0183).
    fn take_colour(&mut self, id: ObjectId, slot: u32, uv: Vec2) -> bool {
        let material = self.material_at(id, slot);
        let scene = self.document.scene();
        let list = scene.material_list(id);
        let base = list
            .get(slot as usize)
            .or(list.first())
            .map_or([0.5; 3], |m| m.color)
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round());
        let mut sum = [0.0f32; 3];
        let mut count = 0.0;
        if let Some(set) = scene.paint().set(&material) {
            let size = set.size as f32;
            let x = (uv.x * size).floor() as i64;
            let y = ((1.0 - uv.y) * size).floor() as i64;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (px, py) = (x + dx, y + dy);
                    if px < 0 || py < 0 || px >= set.size as i64 || py >= set.size as i64 {
                        continue;
                    }
                    let p = paint::composite::pixel_at(set, px as u32, py as u32);
                    let a = p[3] as f32 / 255.0;
                    for c in 0..3 {
                        sum[c] += p[c] as f32 * a + base[c] * (1.0 - a);
                    }
                    count += 1.0;
                }
            }
        }
        let colour: [u8; 3] = if count > 0.0 {
            sum.map(|v| (v / count).round().clamp(0.0, 255.0) as u8)
        } else {
            base.map(|v| v as u8)
        };
        self.attributes.brush.brush.color = [colour[0], colour[1], colour[2], 255];
        self.note = Some(crate::trf!(
            "Picked {} from {}.",
            paint::palette::hex(colour),
            material
        ));
        true
    }

    /// Carries out what the colour picker asked of the project's palettes
    /// (decision 0183).
    pub(super) fn carry_out_palette_requests(&mut self, actions: &mut Vec<super::WorkbenchAction>) {
        for request in std::mem::take(&mut self.attributes.palette_requests) {
            match request {
                PaletteRequest::Set(palettes, label) => {
                    let command = scene::Command::SetPalettes(palettes);
                    if let Err(why) = self.document.execute_labelled(command, tr(label), None) {
                        self.note = Some(crate::trf!("Couldn't change the palettes: {}.", why));
                    }
                }
                PaletteRequest::Import => {
                    actions.push(super::WorkbenchAction::ImportPalette { path: None });
                }
                PaletteRequest::Export(index) => {
                    actions.push(super::WorkbenchAction::ExportPalette { index, path: None });
                }
            }
        }
    }

    /// The layer the brush paints on in `set` (of `material`): the one
    /// chosen in the Paint layers panel, or else the topmost paint layer.
    /// `None` when neither is there (one is made on painting).
    fn paint_target(&self, material: &str, set: &TextureSet) -> Option<paint::LayerPath> {
        match self.paint_layers.active(material, set) {
            Some(path) => Some(path),
            None => set.top_paint_layer(),
        }
    }

    /// The name of the material in slot `slot` of object `id` (the first
    /// for a slot past the list's end, as the export does).
    fn material_at(&self, id: ObjectId, slot: u32) -> String {
        let list = self.document.scene().material_list(id);
        list.get(slot as usize)
            .or(list.first())
            .map(|m| m.name.clone())
            .unwrap_or_default()
    }

    /// Object `id`'s shape as the brush sees it.
    fn brush_surface(&mut self, id: ObjectId, mesh: &Arc<Mesh>) -> Arc<PaintSurface> {
        match self.brushing.surfaces.get(&id) {
            Some((made_from, surface)) if Arc::ptr_eq(made_from, mesh) => Arc::clone(surface),
            _ => {
                let surface = Arc::new(PaintSurface::of(mesh));
                self.brushing
                    .surfaces
                    .insert(id, (Arc::clone(mesh), Arc::clone(&surface)));
                surface
            }
        }
    }

    /// The nearest shape under the mouse at `at` (points, in a view of
    /// `size` seen through `camera`), with UVs to paint through.
    fn brush_hit(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) -> Option<Hit> {
        let ray = camera.ray_through(at, size);
        let mut best: Option<(f32, Hit)> = None;
        for item in &drawing.items {
            if item.outline || item.cage.is_some() || item.drawn.face_count() == 0 {
                continue;
            }
            let inverse = item.transform.inverse();
            if !inverse.is_finite() {
                continue;
            }
            let surface = self.brush_surface(item.id, &item.drawn);
            if !surface.has_uvs() {
                continue;
            }
            let origin = inverse.transform_point3(ray.origin);
            let direction = inverse.transform_vector3(ray.direction);
            let Some((t, triangle)) = surface.hit(origin, direction) else {
                continue;
            };
            let world = item.transform.transform_point3(origin + direction * t);
            let distance = world.distance(ray.origin);
            if best.as_ref().is_none_or(|(d, _)| distance < *d) {
                best = Some((
                    distance,
                    Hit {
                        id: item.id,
                        world,
                        triangle,
                        surface,
                        model: item.transform,
                    },
                ));
            }
        }
        best.map(|(_, hit)| hit)
    }

    /// Starts a stroke at `at`, on the shape there.
    fn begin_stroke(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) {
        self.brushing.session = None;
        let Some(hit) = self.brush_hit(drawing, camera, size, at) else {
            self.note = Some(
                tr("Paint on a shape with UVs: press the left button over one, and drag.")
                    .to_string(),
            );
            return;
        };
        let slot = hit.surface.triangles()[hit.triangle].slot;
        let material = self.material_at(hit.id, slot);
        let slots = self.slots_of(hit.id, &material);
        let view = View {
            view_projection: camera.view_projection(size),
            size,
            eye: camera.position,
            forward: camera.forward(),
            flat: camera.is_flat(),
        };
        let depth = DepthMap::of(&hit.surface, hit.model, &view);
        let place = StrokePlace::Model {
            slots,
            surface: hit.surface,
            model: hit.model,
            view,
            depth,
            camera: *camera,
            size,
        };
        if self.start_session(material, place, at) {
            self.stroke_to(at);
        }
    }

    /// Starts a stroke in the UV Editor at `pointer`, on the piece there.
    fn begin_flat_stroke(&mut self, drawing: &SceneDrawing, pointer: UvPointer) {
        self.brushing.session = None;
        let Some((id, triangle, units_per_uv)) = self.flat_hit(drawing, pointer.uv) else {
            self.note = Some(
                tr("Paint inside a UV piece: press the left button over one, and drag.")
                    .to_string(),
            );
            return;
        };
        let material = self.material_at(id, triangle.slot);
        let mut targets = Vec::new();
        for shown in crate::uv_editor::shown_objects(&self.document) {
            let Some(mesh) = drawn_mesh(drawing, shown) else {
                continue;
            };
            let slots = self.slots_of(shown, &material);
            if !slots.is_empty() {
                let surface = self.brush_surface(shown, &mesh);
                targets.push((surface, slots));
            }
        }
        let zoom = pointer.zoom.max(1e-6);
        let at = pointer.uv * zoom;
        let place = StrokePlace::Flat {
            targets,
            zoom,
            units_per_uv,
        };
        if self.start_session(material, place, at) {
            self.stroke_to(at);
        }
    }

    /// The slots of object `id` holding `material`.
    fn slots_of(&self, id: ObjectId, material: &str) -> Vec<u32> {
        let list = self.document.scene().material_list(id);
        (0..list.len().max(1) as u32)
            .filter(|s| self.material_at(id, *s) == material)
            .collect()
    }

    /// The shown object with a UV piece at `uv` in the UV Editor: its id,
    /// the triangle there and how many scene units a texture width covers
    /// there.
    fn flat_hit(
        &mut self,
        drawing: &SceneDrawing,
        uv: Vec2,
    ) -> Option<(ObjectId, paint::surface::PaintTriangle, f32)> {
        for id in crate::uv_editor::shown_objects(&self.document) {
            let Some(mesh) = drawn_mesh(drawing, id) else {
                continue;
            };
            let surface = self.brush_surface(id, &mesh);
            let Some(triangle) = paint::flat::triangle_at(&surface, uv) else {
                continue;
            };
            let model = drawn_transform(drawing, id);
            let t = surface.triangles()[triangle];
            let world = t.points.map(|p| model.transform_point3(p));
            let area = (world[1] - world[0]).cross(world[2] - world[0]).length();
            let flat = (t.uvs[1] - t.uvs[0]).perp_dot(t.uvs[2] - t.uvs[0]).abs();
            let units_per_uv = if flat > 1e-12 && area > 1e-12 {
                (area / flat).sqrt()
            } else {
                1.0
            };
            return Some((id, t, units_per_uv));
        }
        None
    }

    /// Opens a stroke on `material`'s texture set (made if missing, with a
    /// paint layer on top if it has none), painted at `place`, starting at
    /// `at`. False when there is nothing to paint on.
    fn start_session(&mut self, material: String, place: StrokePlace, at: Vec2) -> bool {
        let scene = self.document.scene();
        let mut set = match scene.paint().set(&material) {
            Some(set) => (**set).clone(),
            None => match TextureSet::new(scene.texture().size) {
                Some(set) => set,
                None => return false,
            },
        };
        // The chosen layer's mask when it is chosen (decision 0182); the
        // chosen layer when it's a paint layer; else a new one, above the
        // chosen layer or on top (decision 0181).
        let target = self.paint_target(&material, &set);
        let on_mask = self.paint_layers.editing_mask()
            && target
                .as_ref()
                .is_some_and(|p| set.layer(p).is_some_and(|l| l.mask.is_some()));
        let paintable = |l: &Layer| on_mask || l.image().is_some();
        let layer = match target.filter(|p| set.layer(p).is_some_and(paintable)) {
            Some(path) => path,
            None => {
                let above = self.paint_layers.active(&material, &set);
                match set.add_paint_layer(tr("Paint"), above.as_deref()) {
                    Some(path) => path,
                    None => return false,
                }
            }
        };
        self.paint_layers.show_material(&material);
        self.paint_layers.choose(&material, layer.clone(), on_mask);
        let Some(chosen) = set.layer(&layer) else {
            return false;
        };
        let (image, strokes_before) = match (&chosen.mask, on_mask) {
            (Some(mask), true) => (mask.image.clone(), mask.strokes.clone()),
            _ => match chosen.image() {
                Some(image) => (image.clone(), chosen.strokes.clone()),
                None => return false,
            },
        };
        let brush = self.attributes.brush.brush;
        // The colour painted with, shown as Before in the picker.
        self.attributes.colour.before = [brush.color[0], brush.color[1], brush.color[2]];
        let record = paint::StrokeRecord {
            kind: brush.kind,
            color: brush.color,
            hardness: brush.hardness,
            flow: brush.flow,
            through: brush.through,
            dabs: Vec::new(),
            fills: Vec::new(),
        };
        self.brushing.strokes += 1;
        self.brushing.session = Some(BrushSession {
            material,
            layer,
            set,
            canvas: StrokeCanvas::new(image),
            place,
            last: at,
            since: None,
            key: STROKE_KEYS | self.brushing.strokes,
            record,
            strokes_before,
            on_mask,
            filled: Default::default(),
        });
        true
    }

    /// Carries the stroke on to `at`.
    fn continue_stroke(&mut self, at: Vec2) {
        if self.brushing.session.is_some() {
            self.stroke_to(at);
        }
    }

    /// Carries a UV Editor stroke on to `pointer`.
    fn continue_flat_stroke(&mut self, pointer: UvPointer) {
        if let Some(session) = &self.brushing.session
            && let StrokePlace::Flat { zoom, .. } = session.place
        {
            self.stroke_to(pointer.uv * zoom);
        }
    }

    /// Dabs from the stroke's last place to `at`, then gives the layer to
    /// the document.
    fn stroke_to(&mut self, at: Vec2) {
        let settings = self.attributes.brush;
        let Some(session) = self.brushing.session.as_mut() else {
            return;
        };
        let spots =
            paint::stroke::dabs_along(session.last, at, settings.brush.step(), &mut session.since);
        session.last = at;
        let painted = match (&session.place, settings.tool) {
            (_, PaintTool::Fill) => fill_at(session, &settings, at),
            (StrokePlace::Model { .. }, PaintTool::Brush) => model_dabs(session, &settings, &spots),
            (StrokePlace::Flat { .. }, PaintTool::Brush) => flat_dabs(session, &settings, &spots),
        };
        if !painted {
            return;
        }
        if let Some(layer) = session.set.layer_mut(&session.layer) {
            let mut strokes = session.strokes_before.clone();
            strokes.push(Arc::new(session.record.clone()));
            match (&mut layer.mask, session.on_mask) {
                (Some(mask), true) => {
                    mask.image = session.canvas.image().clone();
                    mask.strokes = strokes;
                }
                _ => {
                    if let Some(image) = layer.image_mut() {
                        *image = session.canvas.image().clone();
                    }
                    layer.strokes = strokes;
                }
            }
        }
        let label = match settings.brush.kind {
            BrushKind::Paint => tr("Paint stroke"),
            BrushKind::Erase => tr("Erase stroke"),
        };
        let command = scene::Command::SetTextureSet {
            material: session.material.clone(),
            set: Some(session.set.clone()),
        };
        let key = session.key;
        if let Err(why) = self.document.execute_labelled(command, label, Some(key)) {
            self.note = Some(crate::trf!("Couldn't paint: {}.", why));
            self.brushing.session = None;
        }
    }
}

/// Paints `spots` (screen points in the stroke's view) on the model.
/// Whether any dab landed.
fn model_dabs(session: &mut BrushSession, settings: &BrushSettings, spots: &[Vec2]) -> bool {
    let StrokePlace::Model {
        slots,
        surface,
        model,
        view,
        depth,
        camera,
        size,
    } = &session.place
    else {
        return false;
    };
    let inverse = model.inverse();
    let mut brush = settings.brush;
    let mut painted = false;
    for &spot in spots {
        let ray = camera.ray_through(spot, *size);
        let origin = inverse.transform_point3(ray.origin);
        let direction = inverse.transform_vector3(ray.direction);
        let Some((t, _)) = surface.hit(origin, direction) else {
            continue;
        };
        let world = model.transform_point3(origin + direction * t);
        let per_point = camera.world_per_pixel(world, *size).max(1e-6);
        let radius = if settings.on_surface {
            settings.surface_radius / per_point
        } else {
            settings.brush.radius
        };
        brush.radius = radius;
        let target = Target {
            surface,
            slots,
            model: *model,
            view,
            depth: Some(depth),
        };
        paint::paint_dab(
            &mut session.canvas,
            &target,
            &brush,
            &Dab {
                screen: spot,
                world,
                radius,
                world_radius: radius * per_point,
            },
        );
        let scale = model
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element()
            .max(1e-6);
        session.record.dabs.push(paint::DabRecord {
            at: origin + direction * t,
            toward: inverse
                .transform_vector3(view.toward_camera(world))
                .normalize_or_zero(),
            radius: radius * per_point / scale,
        });
        painted = true;
    }
    painted
}

/// Paints `spots` (places on the texture times the zoom) flat on the
/// texture. Whether any dab landed.
fn flat_dabs(session: &mut BrushSession, settings: &BrushSettings, spots: &[Vec2]) -> bool {
    let StrokePlace::Flat {
        targets,
        zoom,
        units_per_uv,
    } = &session.place
    else {
        return false;
    };
    // The brush's radius in texture widths.
    let radius = if settings.on_surface {
        settings.surface_radius / units_per_uv.max(1e-6)
    } else {
        settings.brush.radius / zoom
    };
    let size = session.canvas.image().size() as f32;
    let mut brush = settings.brush;
    brush.radius = radius * size;
    for &spot in spots {
        let uv = spot / *zoom;
        let centre = Vec2::new(uv.x * size, (1.0 - uv.y) * size);
        for (surface, slots) in targets {
            paint::flat::paint_flat_dab(
                &mut session.canvas,
                surface,
                slots,
                &brush,
                centre,
                radius * size,
            );
        }
        session.record.dabs.push(paint::DabRecord::flat(uv, radius));
    }
    !spots.is_empty()
}

/// The polygon fill tool at `at` (screen points in the stroke's view, or
/// the place on the texture times the zoom): fills the polygon, UV piece
/// or material there, once per drag (decision 0182). Whether it filled
/// anything new.
fn fill_at(session: &mut BrushSession, settings: &BrushSettings, at: Vec2) -> bool {
    let mut brush = settings.brush;
    brush.hardness = 1.0;
    brush.flow = 1.0;
    // The shapes and the triangle under `at` on each.
    let mut hits: Vec<(usize, Arc<PaintSurface>, Vec<u32>, usize)> = Vec::new();
    match &session.place {
        StrokePlace::Model {
            slots,
            surface,
            model,
            camera,
            size,
            ..
        } => {
            let inverse = model.inverse();
            let ray = camera.ray_through(at, *size);
            let origin = inverse.transform_point3(ray.origin);
            let direction = inverse.transform_vector3(ray.direction);
            if let Some((_, triangle)) = surface.hit(origin, direction) {
                hits.push((0, Arc::clone(surface), slots.clone(), triangle));
            }
        }
        StrokePlace::Flat { targets, zoom, .. } => {
            let uv = at / *zoom;
            for (i, (surface, slots)) in targets.iter().enumerate() {
                if let Some(triangle) = paint::flat::triangle_at(surface, uv)
                    && slots.contains(&surface.triangles()[triangle].slot)
                {
                    hits.push((i, Arc::clone(surface), slots.clone(), triangle));
                }
            }
        }
    }
    let mut filled = false;
    for (shape, surface, slots, triangle) in hits {
        if session.filled.contains(&(shape, triangle)) {
            continue;
        }
        let area = paint::fill::fill_area(&surface, triangle, settings.fill_by, &slots);
        paint::fill::fill_triangles(&mut session.canvas, &surface, &area, &brush);
        session.filled.extend(area.iter().map(|t| (shape, *t)));
        session.record.fills.push(paint::FillRecord {
            face: surface.triangles()[triangle].face,
            by: settings.fill_by,
        });
        filled = true;
    }
    filled
}

/// The shape drawn for object `id`, if it has polygons.
fn drawn_mesh(drawing: &SceneDrawing, id: ObjectId) -> Option<Arc<Mesh>> {
    drawing
        .items
        .iter()
        .find(|i| i.id == id && !i.outline && i.cage.is_none() && i.drawn.face_count() > 0)
        .map(|i| Arc::clone(&i.drawn))
}

/// Where object `id` is drawn in the world.
fn drawn_transform(drawing: &SceneDrawing, id: ObjectId) -> Mat4 {
    drawing
        .items
        .iter()
        .find(|i| i.id == id && !i.outline && i.cage.is_none())
        .map_or(Mat4::IDENTITY, |i| i.transform)
}

/// The UV at `point` (in the shape's own space, on `triangle`'s plane).
fn uv_at(triangle: &paint::surface::PaintTriangle, point: Vec3) -> Vec2 {
    let [a, b, c] = triangle.points;
    let (v0, v1, v2) = (b - a, c - a, point - a);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return triangle.uvs[0];
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;
    triangle.uvs[0] * u + triangle.uvs[1] * v + triangle.uvs[2] * w
}

/// How wide one texture pixel is on `triangle` (placed by `model`) in a
/// texture `size` pixels across, in centimetres.
fn texel_width(triangle: &paint::surface::PaintTriangle, model: Mat4, size: u32) -> Option<f32> {
    let world = triangle.points.map(|p| model.transform_point3(p));
    let mut most = 0.0f32;
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let length = world[a].distance(world[b]);
        let texels = (triangle.uvs[a] - triangle.uvs[b]).length() * size as f32;
        if length > 1e-6 {
            most = most.max(texels / length);
        }
    }
    (most > 0.0).then(|| 1.0 / most)
}

#[cfg(test)]
mod uv_at_tests {
    use super::*;

    #[test]
    fn the_uv_at_a_point_follows_the_triangle() {
        let surface = PaintSurface::of(&mesh::PrimitiveKind::Cube.standard().mesh());
        for t in surface.triangles() {
            let middle = (t.points[0] + t.points[1] + t.points[2]) / 3.0;
            let uv = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
            assert!(uv_at(t, middle).distance(uv) < 1e-4);
            assert!(uv_at(t, t.points[1]).distance(t.uvs[1]) < 1e-4);
        }
    }
}
