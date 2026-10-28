//! Painting with the brush in Paint mode ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)).
//!
//! A stroke begins where the mouse is pressed on a shape: it paints the
//! texture set of the material there, into its top layer (a "Paint" layer
//! is made if the set has none, and the set itself if the material has
//! none). Every frame of the stroke the layer is given to the document
//! through `Command::SetTextureSet`, merged into one undo step, so the
//! views show the paint as it goes on and undo takes the whole stroke.

use super::Workbench;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::viewport::{BrushEvent, Surface};
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Vec2, Vec3};
use paint::{BrushKind, Dab, DepthMap, Image, Layer, PaintSurface, StrokeCanvas, Target, TextureSet, View};
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
}

/// A stroke under way.
#[derive(Debug)]
struct BrushSession {
    id: ObjectId,
    material: String,
    layer: usize,
    slots: Vec<u32>,
    set: TextureSet,
    canvas: StrokeCanvas,
    surface: Arc<PaintSurface>,
    model: Mat4,
    view: View,
    depth: DepthMap,
    camera: tools::Camera,
    size: Vec2,
    last: Vec2,
    since: Option<f32>,
    key: u64,
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
        }
        if !on && self.brushing.on {
            if let Some(before) = self.brushing.surface_before.take()
                && self.viewport.options().surface == Surface::Painted
            {
                self.viewport.options_mut().surface = before;
            }
            self.brushing.session = None;
        }
        self.brushing.on = on;
        self.attributes.painting = on;
        if !on {
            self.viewport.set_paint_brush(None);
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
            let per_point = pointer.camera.world_per_pixel(hit.world, pointer.size).max(1e-6);
            if settings.on_surface {
                radius = settings.surface_radius / per_point;
            }
            let triangle = &hit.surface.triangles()[hit.triangle];
            let material = self.material_at(hit.id, triangle.slot);
            let set = self.document.scene().paint().set(&material).cloned();
            let size = set
                .as_ref()
                .map_or(self.document.scene().texture().size, |s| s.size);
            self.attributes.brush_target = crate::attributes::BrushTarget {
                layer: set
                    .as_ref()
                    .and_then(|s| s.layers.last())
                    .map_or_else(|| tr("Paint").to_string(), |l| l.name.clone()),
                material,
                texel: texel_width(triangle, hit.model, size),
                radius: Some(radius * per_point),
            };
        }
        if let Some(session) = &self.brushing.session
            && settings.on_surface
        {
            let per_point = session.camera.world_per_pixel(session.view.eye, session.size);
            let _ = per_point;
        }
        self.viewport
            .set_paint_brush(Some((radius.max(0.5), settings.brush.hardness)));
    }

    /// The brush's events from the views, after they are drawn.
    pub(super) fn carry_out_brush(&mut self, drawing: &SceneDrawing) {
        for event in self.viewport.take_brush_events() {
            match event {
                BrushEvent::Begin(p) => self.begin_stroke(drawing, &p.camera, p.size, p.at),
                BrushEvent::Move(p) => self.continue_stroke(p.at),
                BrushEvent::End => self.brushing.session = None,
                BrushEvent::Adjust {
                    size,
                    hardness,
                    flow,
                } => {
                    let settings = &mut self.attributes.brush;
                    if settings.on_surface {
                        settings.surface_radius = (settings.surface_radius * size).clamp(
                            crate::attributes::SURFACE_RADIUS.0,
                            crate::attributes::SURFACE_RADIUS.1,
                        );
                    } else {
                        settings.brush.radius *= size;
                    }
                    settings.brush.hardness += hardness;
                    settings.brush.flow += flow;
                    settings.brush = settings.brush.clamped();
                }
            }
        }
    }

    /// Paint mode's keys: 1 paint and 2 erase (3 to 6 are the tools still
    /// to come), [ and ] the size, Esc back to the Model mode.
    pub(super) fn paint_keys(&mut self, ui: &Ui) {
        if self.mode != Mode::Paint || ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |ui: &Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(ui, Key::Num1) {
            self.attributes.brush.brush.kind = BrushKind::Paint;
        }
        if pressed(ui, Key::Num2) {
            self.attributes.brush.brush.kind = BrushKind::Erase;
        }
        for key in [Key::Num3, Key::Num4, Key::Num5, Key::Num6] {
            if pressed(ui, key) {
                self.note = Some(tr(
                    "Projection, polygon fill, smudge and clone come in later steps of the \
                     texture painter. 1 paints, 2 erases.",
                )
                .to_string());
            }
        }
        let settings = &mut self.attributes.brush;
        for (key, factor) in [(Key::OpenBracket, 1.0 / SIZE_STEP), (Key::CloseBracket, SIZE_STEP)] {
            if pressed(ui, key) {
                if settings.on_surface {
                    settings.surface_radius = (settings.surface_radius * factor).clamp(
                        crate::attributes::SURFACE_RADIUS.0,
                        crate::attributes::SURFACE_RADIUS.1,
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
    fn begin_stroke(&mut self, drawing: &SceneDrawing, camera: &tools::Camera, size: Vec2, at: Vec2) {
        self.brushing.session = None;
        let Some(hit) = self.brush_hit(drawing, camera, size, at) else {
            self.note = Some(
                tr("Paint on a shape with UVs: press the left button over one, and drag.")
                    .to_string(),
            );
            return;
        };
        let scene = self.document.scene();
        let slot = hit.surface.triangles()[hit.triangle].slot;
        let material = self.material_at(hit.id, slot);
        let list = scene.material_list(hit.id);
        let slots: Vec<u32> = (0..list.len().max(1) as u32)
            .filter(|s| self.material_at(hit.id, *s) == material)
            .collect();
        let mut set = match scene.paint().set(&material) {
            Some(set) => (**set).clone(),
            None => match TextureSet::new(scene.texture().size) {
                Some(set) => set,
                None => return,
            },
        };
        if set.layers.last().and_then(Layer::image).is_none() {
            let Some(image) = Image::new(set.size) else { return };
            let name = set.free_name(tr("Paint"));
            set.layers.push(Layer::paint(name, image));
        }
        let layer = set.layers.len() - 1;
        let Some(image) = set.layers[layer].image().cloned() else {
            return;
        };
        let view = View {
            view_projection: camera.view_projection(size),
            size,
            eye: camera.position,
            forward: camera.forward(),
            flat: camera.is_flat(),
        };
        let depth = DepthMap::of(&hit.surface, hit.model, &view);
        self.brushing.strokes += 1;
        self.brushing.session = Some(BrushSession {
            id: hit.id,
            material,
            layer,
            slots,
            set,
            canvas: StrokeCanvas::new(image),
            surface: hit.surface,
            model: hit.model,
            view,
            depth,
            camera: *camera,
            size,
            last: at,
            since: None,
            key: STROKE_KEYS | self.brushing.strokes,
        });
        self.stroke_to(at);
    }

    /// Carries the stroke on to `at`.
    fn continue_stroke(&mut self, at: Vec2) {
        if self.brushing.session.is_some() {
            self.stroke_to(at);
        }
    }

    /// Dabs from the stroke's last place to `at`, then gives the layer to
    /// the document.
    fn stroke_to(&mut self, at: Vec2) {
        let settings = self.attributes.brush;
        let Some(session) = self.brushing.session.as_mut() else {
            return;
        };
        let inverse = session.model.inverse();
        let mut brush = settings.brush;
        let step_radius = settings.brush.radius;
        brush.radius = step_radius;
        let spots = paint::stroke::dabs_along(
            session.last,
            at,
            settings.brush.step(),
            &mut session.since,
        );
        session.last = at;
        let mut painted = false;
        for spot in spots {
            let ray = session.camera.ray_through(spot, session.size);
            let origin = inverse.transform_point3(ray.origin);
            let direction = inverse.transform_vector3(ray.direction);
            let Some((t, _)) = session.surface.hit(origin, direction) else {
                continue;
            };
            let world = session.model.transform_point3(origin + direction * t);
            let per_point = session
                .camera
                .world_per_pixel(world, session.size)
                .max(1e-6);
            let radius = if settings.on_surface {
                settings.surface_radius / per_point
            } else {
                settings.brush.radius
            };
            brush.radius = radius;
            let target = Target {
                surface: &session.surface,
                slots: &session.slots,
                model: session.model,
                view: &session.view,
                depth: Some(&session.depth),
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
            painted = true;
        }
        if !painted {
            return;
        }
        if let Some(layer) = session.set.layers.get_mut(session.layer)
            && let Some(image) = layer.image_mut()
        {
            *image = session.canvas.image().clone();
        }
        let label = match settings.brush.kind {
            BrushKind::Paint => tr("Paint stroke"),
            BrushKind::Erase => tr("Erase stroke"),
        };
        let command = scene::Command::SetTextureSet {
            material: session.material.clone(),
            set: Some(session.set.clone()),
        };
        let (key, id) = (session.key, session.id);
        if let Err(why) = self.document.execute_labelled(command, label, Some(key)) {
            self.note = Some(crate::trf!("Couldn't paint: {}.", why));
            self.brushing.session = None;
        }
        let _ = id;
    }
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
