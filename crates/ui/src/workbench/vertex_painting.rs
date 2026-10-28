//! Vertex colour painting ([decision 0205](../../../../docs/decisions/0205-vertex-colour-painting.md)):
//! with Attributes → Paint on: Vertex colours, the brush paints the
//! colours of polygon corners near it on the model, and the polygon fill
//! tool fills polygons' corners. A stroke is one recorded edit
//! (`Operation::PaintColours`), recorded at its first dab and replaced as
//! it grows, so the model shows the colours while you paint and the
//! stroke is one undo step.

use super::Workbench;
use crate::attributes::PaintTool;
use crate::drawing::SceneDrawing;
use crate::text::tr;
use glam::{Mat4, Vec2, Vec3};
use paint::BrushKind;
use scene::{Command, FaceId, Mesh, ObjectId, Operation, VertexId};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A vertex colour stroke under way.
#[derive(Debug, Clone)]
pub(super) struct VertexSession {
    id: ObjectId,
    /// The shape as recorded before the stroke (its edits applied).
    mesh: Arc<Mesh>,
    /// Where the object is drawn in the world.
    model: Mat4,
    /// Each corner painted so far, and its colour now.
    painted: BTreeMap<(FaceId, VertexId), [u8; 4]>,
    /// The undo step's merge key.
    key: u64,
    /// Where its edit was recorded (layer, index), once it was.
    recorded: Option<(usize, usize)>,
    /// Whether it has dabs not yet sent to the document.
    dirty: bool,
    /// The last place dabbed, on screen.
    last: Vec2,
}

impl Workbench {
    /// Starts a vertex colour stroke (or fill) at `at` in a view of
    /// `size` seen by `camera`.
    pub(super) fn begin_vertex_stroke(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) {
        self.brushing.vertex = None;
        let Some(hit) = self.brush_hit(drawing, camera, size, at) else {
            self.note =
                Some(tr("Paint on a shape: press the left button over one, and drag.").to_string());
            return;
        };
        let Some(evaluated) = self.document.scene().evaluated(hit.id) else {
            return;
        };
        let key = self.new_step_key();
        self.brushing.vertex = Some(VertexSession {
            id: hit.id,
            mesh: evaluated.mesh,
            model: hit.model,
            painted: BTreeMap::new(),
            key,
            recorded: None,
            dirty: false,
            last: at,
        });
        if self.attributes.brush.tool == PaintTool::Fill {
            self.vertex_fill(hit.triangle, &hit.surface);
        } else {
            self.vertex_dab(camera, size, hit.world);
        }
        self.send_vertex_stroke();
    }

    /// Carries a vertex colour stroke on to `at`, dabbing along the way.
    pub(super) fn continue_vertex_stroke(
        &mut self,
        drawing: &SceneDrawing,
        camera: &tools::Camera,
        size: Vec2,
        at: Vec2,
    ) {
        if self.attributes.brush.tool == PaintTool::Fill {
            return;
        }
        let Some(session) = &self.brushing.vertex else {
            return;
        };
        let (from, id) = (session.last, session.id);
        let step = (self.attributes.brush.brush.radius
            * self.attributes.brush.brush.spacing.max(0.05))
        .max(1.0);
        let distance = from.distance(at);
        let count = ((distance / step).ceil() as usize).clamp(1, 200);
        for i in 1..=count {
            let p = from.lerp(at, i as f32 / count as f32);
            if let Some(hit) = self.brush_hit(drawing, camera, size, p)
                && hit.id == id
            {
                self.vertex_dab(camera, size, hit.world);
            }
        }
        if let Some(session) = &mut self.brushing.vertex {
            session.last = at;
        }
        self.send_vertex_stroke();
    }

    /// "From the texture" (decision 0205): every corner of the selected
    /// objects takes the colour of its material's painted texture at its
    /// UV (over the material's colour), as one undo step.
    pub(super) fn vertex_colours_from_texture(&mut self) {
        if !std::mem::take(&mut self.attributes.vertex_from_texture) {
            return;
        }
        let key = self.new_step_key();
        let mut done = 0;
        let ids: Vec<ObjectId> = self.document.selection();
        for id in ids {
            let scene = self.document.scene();
            let Some(evaluated) = scene.evaluated(id) else {
                continue;
            };
            let mesh = evaluated.mesh;
            let Some(uvs) = mesh::uvs::Uvs::of(&mesh) else {
                continue;
            };
            let list = scene.material_list(id);
            let (starts, _) = mesh.face_layout();
            let mut mixed: std::collections::HashMap<String, Option<paint::Image>> =
                std::collections::HashMap::new();
            let mut corners = Vec::new();
            for (face, points) in mesh.faces() {
                let slot = mesh::material::slot_of(&mesh, face) as usize;
                let material = list.get(slot).or(list.first());
                let name =
                    material.map_or_else(|| scene::Material::standard().name, |m| m.name.clone());
                let base = material.map_or([0.8; 3], |m| m.color);
                let image = mixed.entry(name.clone()).or_insert_with(|| {
                    let set = scene.paint().set(&name)?;
                    Some(paint::Composite::of(set)?.image().clone())
                });
                let start = starts[face.get() as usize] as usize;
                for (i, point) in points.iter().enumerate() {
                    let uv = uvs.at(start + i);
                    let p = image.as_ref().map_or([0; 4], |img| sample(img, uv));
                    let a = f32::from(p[3]) / 255.0;
                    let mut c = [255u8; 4];
                    for k in 0..3 {
                        let v = base[k] * 255.0 * (1.0 - a) + f32::from(p[k]) * a;
                        c[k] = v.round().clamp(0.0, 255.0) as u8;
                    }
                    corners.push((face, *point, c));
                }
            }
            if corners.is_empty() {
                continue;
            }
            let command = Command::Record {
                id,
                operation: Operation::PaintColours { corners },
            };
            match self.document.execute_labelled(
                command,
                tr("Vertex colours from the texture"),
                Some(key),
            ) {
                Ok(_) => done += 1,
                Err(why) => {
                    self.note = Some(crate::trf!("Couldn't paint the vertex colours: {}.", why));
                }
            }
        }
        if done == 0 && self.note.is_none() {
            self.note = Some(
                tr("Select the painted objects to turn their texture into vertex colours.")
                    .to_string(),
            );
        }
    }

    /// Ends a vertex colour stroke.
    pub(super) fn end_vertex_stroke(&mut self) {
        self.send_vertex_stroke();
        self.brushing.vertex = None;
    }

    /// Paints the corners within the brush of `world`: each mixed toward
    /// the brush colour (white for the eraser) by the brush's falloff and
    /// flow, facing the camera unless painting through.
    fn vertex_dab(&mut self, camera: &tools::Camera, size: Vec2, world: Vec3) {
        let settings = self.attributes.brush;
        let pressure = self.pen().map(|p| p.pressure);
        let per_point = camera.world_per_pixel(world, size).max(1e-6);
        let radius_world = if settings.on_surface {
            settings.surface_radius
        } else {
            settings.brush.radius * per_point
        } * settings.size_pressure.share(pressure);
        let flow = settings.brush.flow * settings.flow_pressure.share(pressure);
        let target = match settings.brush.kind {
            BrushKind::Erase => [255; 4],
            _ => settings.brush.color,
        };
        let hardness = settings.brush.hardness.clamp(0.0, 1.0);
        let through = settings.brush.through;
        let Some(session) = &mut self.brushing.vertex else {
            return;
        };
        let inverse = session.model.inverse();
        let scale = session
            .model
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element()
            .max(1e-6);
        let centre = inverse.transform_point3(world);
        let radius = (radius_world / scale).max(1e-4);
        let to_camera = inverse.transform_point3(camera.position);
        for (face, list) in session.mesh.faces() {
            let normal = session.mesh.face_normal(face);
            for point in list {
                let Some(p) = session.mesh.position(*point) else {
                    continue;
                };
                let d = p.distance(centre) / radius;
                if d > 1.0 {
                    continue;
                }
                if !through && normal.dot(to_camera - p) <= 0.0 {
                    continue;
                }
                // Full strength inside the hard core, fading to the rim.
                let falloff = if d <= hardness {
                    1.0
                } else {
                    1.0 - (d - hardness) / (1.0 - hardness).max(1e-6)
                };
                let amount = (falloff * flow).clamp(0.0, 1.0);
                if amount <= 0.0 {
                    continue;
                }
                let current = session
                    .painted
                    .get(&(face, *point))
                    .copied()
                    .unwrap_or_else(|| {
                        mesh::colours::colour_at(&session.mesh, face, *point).unwrap_or([255; 4])
                    });
                let mixed = mix(current, target, amount);
                if mixed != current {
                    session.painted.insert((face, *point), mixed);
                    session.dirty = true;
                }
            }
        }
    }

    /// Fills the corners of the polygons the fill tool reaches from
    /// `triangle` of `surface` with the brush colour.
    fn vertex_fill(&mut self, triangle: usize, surface: &paint::PaintSurface) {
        let settings = self.attributes.brush;
        let target = match settings.brush.kind {
            BrushKind::Erase => [255; 4],
            _ => settings.brush.color,
        };
        let Some(session) = &mut self.brushing.vertex else {
            return;
        };
        let slot = surface.triangles().get(triangle).map_or(0, |t| t.slot);
        let triangles = paint::fill::fill_area(surface, triangle, settings.fill_by, &[slot]);
        let faces: std::collections::BTreeSet<u32> = triangles
            .iter()
            .filter_map(|t| surface.triangles().get(*t).map(|t| t.face))
            .collect();
        for face in faces {
            let face = FaceId::from_raw(face);
            let Some(list) = session.mesh.face(face) else {
                continue;
            };
            for point in list {
                session.painted.insert((face, *point), target);
            }
        }
        session.dirty = true;
    }

    /// Sends the stroke's colours to the document: recorded at first,
    /// replaced after, as one undo step.
    fn send_vertex_stroke(&mut self) {
        let Some(session) = &mut self.brushing.vertex else {
            return;
        };
        if !session.dirty || session.painted.is_empty() {
            return;
        }
        session.dirty = false;
        let operation = Operation::PaintColours {
            corners: session
                .painted
                .iter()
                .map(|((f, v), c)| (*f, *v, *c))
                .collect(),
        };
        let (id, key) = (session.id, session.key);
        let command = match session.recorded {
            Some((layer, index)) => Command::ReplaceOperation {
                id,
                layer,
                index,
                operation,
            },
            None => Command::Record { id, operation },
        };
        if let Err(why) =
            self.document
                .execute_labelled(command, tr("Paint vertex colours"), Some(key))
        {
            self.note = Some(crate::trf!("Couldn't paint the vertex colours: {}.", why));
            self.brushing.vertex = None;
            return;
        }
        // Where it was recorded, to replace it as the stroke grows.
        let place = self.document.scene().object(id).and_then(|o| {
            let layers = o.stack.layers();
            let layer = layers.len().checked_sub(1)?;
            let index = layers[layer].entries.len().checked_sub(1)?;
            Some((layer, index))
        });
        if let Some(session) = &mut self.brushing.vertex
            && session.recorded.is_none()
        {
            session.recorded = place;
        }
    }
}

/// The texel of `image` at `uv` (where the texture repeats).
fn sample(image: &paint::Image, uv: Vec2) -> [u8; 4] {
    let size = image.size();
    let s = size as f32;
    let x = (uv.x.rem_euclid(1.0) * s).floor().clamp(0.0, s - 1.0) as u32;
    let y = ((1.0 - uv.y.rem_euclid(1.0)) * s)
        .floor()
        .clamp(0.0, s - 1.0) as u32;
    image.pixel(x, y)
}

/// `a` moved toward `b` by `t` (0 to 1), channel by channel.
fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = f32::from(a[c]) + (f32::from(b[c]) - f32::from(a[c])) * t;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}
