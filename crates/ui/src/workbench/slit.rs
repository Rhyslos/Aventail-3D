//! The workbench's part in Slit ([decision 0049](../../../../docs/decisions/0049-slit.md)).
//!
//! The controls, chosen by the project lead to work as in Cinema 4D: with
//! Slit in use, click (or press and drag) to put the line's points on the
//! shape, each one snapping to a point, an edge or a polygon under the
//! mouse. The shape is cut as the line grows. The line stays editable until
//! it is finished: drag a point to move it, click the line to add a point
//! there, Ctrl + click a point to remove it, and hold Shift to keep a
//! stretch at 15° steps on screen. Esc or Enter finishes the line, as does
//! picking another tool or mode; Ctrl + Z takes the cut back.
//!
//! The whole line is one undo step: each change takes back the cut before
//! (`Document::abandon`) and records it again with the new line, one
//! `Operation::Slit` per selected object, in its own space.

use super::Workbench;
use crate::components::{self, ViewPointer};
use crate::drawing::SceneDrawing;
use crate::transforming::Tool;
use crate::viewport::{ToolDragEvent, ToolPath};
use glam::{Mat4, Vec2, Vec3};
use scene::{Command, Element, ElementKind, Eye, ObjectId, Operation, SlitLine};
use tools::Camera;

/// Merge keys for slits: kept apart from other merge keys.
const SLIT_KEYS: u64 = 1 << 58;

/// How near (screen points) the mouse must be to a line point to take it.
const POINT_REACH: f32 = 9.0;
/// How near (screen points) the mouse must be to the line to add a point.
const LINE_REACH: f32 = 6.0;
/// Shift keeps a stretch at steps of this many degrees on screen.
const ANGLE_STEP: f32 = 15.0;

/// A slit's line being drawn.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SlitSession {
    key: u64,
    /// The objects cut, and where each is in the world.
    objects: Vec<(ObjectId, Mat4)>,
    /// The line's points, in the world.
    points: Vec<Vec3>,
    /// The camera the line was last changed through: the cut is seen from
    /// it.
    camera: Camera,
    /// Whether it cuts through, as last recorded.
    through: bool,
    /// The point being dragged, and where it was before (`None`: just
    /// added).
    dragging: Option<(usize, Option<Vec3>)>,
    /// How many undo steps there were after the last change: any other
    /// change (an undo, a selection) finishes the line.
    steps: usize,
}

impl SlitSession {
    /// The slit for an object at `world`, in its own space.
    fn line(&self, world: Mat4) -> Option<SlitLine> {
        let inverse = world.inverse();
        if !inverse.is_finite() {
            return None;
        }
        let eye = if self.camera.is_flat() {
            Eye::Along(inverse.transform_vector3(self.camera.forward()))
        } else {
            Eye::At(inverse.transform_point3(self.camera.position))
        };
        Some(SlitLine {
            points: self
                .points
                .iter()
                .map(|p| inverse.transform_point3(*p))
                .collect(),
            eye,
            through: self.through,
        })
    }
}

/// Where `p` (in the world) is on screen through `pointer`'s camera.
fn on_screen(pointer: &ViewPointer, p: Vec3) -> Option<Vec2> {
    pointer.camera.project(p, pointer.size)
}

/// How far `at` is from the segment from `a` to `b` (on screen), and how far
/// along it the nearest spot is.
fn to_segment(at: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let along = b - a;
    let t = if along.length_squared() > 1e-9 {
        ((at - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (at.distance(a + along * t), t)
}

/// The point on the segment `a`–`b` nearest the ray from `origin` going
/// `way` (unit).
fn nearest_on_segment(a: Vec3, b: Vec3, origin: Vec3, way: Vec3) -> Vec3 {
    let d = b - a;
    let r = a - origin;
    let (dd, dw, ww) = (d.dot(d), d.dot(way), way.dot(way));
    let (dr, wr) = (d.dot(r), way.dot(r));
    let det = dd * ww - dw * dw;
    let t = if det.abs() > 1e-9 {
        ((dw * wr - ww * dr) / det).clamp(0.0, 1.0)
    } else {
        0.0
    };
    a + d * t
}

impl Workbench {
    /// Where a line point at the mouse goes: on a point, an edge or a
    /// polygon of a selected object under it, or (with nothing there, or
    /// `free` asked for) in space, as far away as `depth_like`.
    fn slit_spot(
        &mut self,
        drawing: &SceneDrawing,
        pointer: ViewPointer,
        depth_like: Option<Vec3>,
        free: bool,
    ) -> Vec3 {
        let ray = pointer.camera.ray_through(pointer.at, pointer.size);
        let world = |id: ObjectId| drawing.items.iter().find(|i| i.id == id);
        if !free {
            if let Some((id, Element::Point(v))) =
                self.components.find(ElementKind::Points, drawing, pointer)
                && let Some(item) = world(id)
                && let Some(p) = item.mesh.position(v)
            {
                return item.transform.transform_point3(p);
            }
            if let Some((id, Element::Edge(e))) =
                self.components.find(ElementKind::Edges, drawing, pointer)
                && let Some(item) = world(id)
            {
                let (a, b) = e.ends();
                if let (Some(a), Some(b)) = (item.mesh.position(a), item.mesh.position(b)) {
                    let (a, b) = (
                        item.transform.transform_point3(a),
                        item.transform.transform_point3(b),
                    );
                    return nearest_on_segment(a, b, ray.origin, ray.direction);
                }
            }
        }
        if let Some((id, Element::Polygon(f))) =
            self.components
                .find(ElementKind::Polygons, drawing, pointer)
            && let Some(item) = world(id)
            && let Some(corner) = item.mesh.face(f).and_then(|c| item.mesh.position(c[0]))
        {
            let normal = item
                .transform
                .inverse()
                .transpose()
                .transform_vector3(item.mesh.face_normal(f))
                .normalize_or_zero();
            let on = item.transform.transform_point3(corner);
            let denom = ray.direction.dot(normal);
            if denom.abs() > 1e-6 {
                let t = (on - ray.origin).dot(normal) / denom;
                if t > 0.0 && t.is_finite() {
                    return ray.at(t);
                }
            }
        }
        // In space: as far along the ray as the earlier point, or the middle
        // of what's selected.
        let like = depth_like.or_else(|| {
            drawing.items.iter().find(|i| i.selected).map(|i| {
                i.transform
                    .transform_point3(i.mesh.bounds().map_or(Vec3::ZERO, |b| b.centre()))
            })
        });
        let distance = like
            .map(|p| (p - ray.origin).dot(ray.direction))
            .filter(|d| *d > 1.0 && d.is_finite())
            .unwrap_or(500.0);
        ray.at(distance)
    }

    /// The spot for a point dragged or added at `pointer`, with Shift
    /// keeping the stretch from `from` at steps of 15° on screen.
    fn slit_place(
        &mut self,
        drawing: &SceneDrawing,
        mut pointer: ViewPointer,
        from: Option<Vec3>,
        shift: bool,
    ) -> Vec3 {
        let mut free = false;
        if shift
            && let Some(from) = from
            && let Some(start) = on_screen(&pointer, from)
        {
            let offset = pointer.at - start;
            let length = offset.length();
            if length > 1.0 {
                let step = ANGLE_STEP.to_radians();
                let angle = (offset.y.atan2(offset.x) / step).round() * step;
                pointer.at = start + Vec2::new(angle.cos(), angle.sin()) * length;
                free = true;
            }
        }
        self.slit_spot(drawing, pointer, from, free)
    }

    /// Records the session's line on every object it cuts, instead of what
    /// was recorded before.
    fn record_slit(&mut self, session: &mut SlitSession) {
        self.document.abandon(session.key);
        session.through = self.attributes.slit_through;
        if session.points.len() >= 2 {
            for (id, world) in session.objects.clone() {
                let Some(line) = session.line(world) else {
                    continue;
                };
                let command = Command::Record {
                    id,
                    operation: Operation::Slit { line },
                };
                let label = if session.through {
                    "Slit through"
                } else {
                    "Slit"
                };
                if let Err(why) = self
                    .document
                    .execute_labelled(command, label, Some(session.key))
                {
                    self.note = Some(format!("Couldn't do that: {why}."));
                }
            }
        }
        session.steps = self.document.undo_steps();
    }

    /// Finishes the line being drawn, keeping its cut.
    pub(super) fn finish_slit(&mut self) {
        if let Some(session) = self.slit_session.take() {
            self.note = Some(if session.points.len() >= 2 {
                "Slit finished. Ctrl + Z takes it back.".into()
            } else {
                "Slit: nothing was cut.".into()
            });
        }
        self.viewport.set_tool_path(None);
    }

    /// Shows the line being drawn, and where the next point would go.
    pub(super) fn show_slit(&mut self, drawing: &SceneDrawing) {
        let editing = components::kind_for(self.mode).is_some();
        if self.tool != Tool::Slit || !editing {
            if self.slit_session.is_some() {
                self.finish_slit();
            }
            self.viewport.set_tool_path(None);
            return;
        }
        let points = self
            .slit_session
            .as_ref()
            .map(|s| s.points.clone())
            .unwrap_or_default();
        let dragging = self
            .slit_session
            .as_ref()
            .is_some_and(|s| s.dragging.is_some());
        let cursor = match self.viewport.pointer() {
            Some(pointer) if !dragging => {
                Some(self.slit_spot(drawing, pointer, points.last().copied(), false))
            }
            _ => None,
        };
        self.viewport
            .set_tool_path(Some(ToolPath { points, cursor }));
    }

    /// Carries out this frame's Slit presses, drags and clicks.
    pub(super) fn carry_out_slit(&mut self, drawing: &SceneDrawing, keys: egui::Modifiers) {
        // Anything else that changed the document (an undo, a selection)
        // finishes the line.
        if self
            .slit_session
            .as_ref()
            .is_some_and(|s| s.steps != self.document.undo_steps())
        {
            self.finish_slit();
        }
        // The Cut through switch changed while drawing: cut again.
        if let Some(mut session) = self.slit_session.take() {
            if session.through != self.attributes.slit_through {
                self.record_slit(&mut session);
            }
            self.slit_session = Some(session);
        }
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { pointer } => self.press_slit(drawing, pointer, keys),
                ToolDragEvent::Update { pointer, .. } => {
                    let Some(mut session) = self.slit_session.take() else {
                        continue;
                    };
                    if let Some((i, _)) = session.dragging {
                        let from = i.checked_sub(1).map(|k| session.points[k]);
                        let spot = self.slit_place(drawing, pointer, from, keys.shift);
                        if session.points[i] != spot {
                            session.points[i] = spot;
                            session.camera = pointer.camera;
                            self.record_slit(&mut session);
                        }
                    }
                    self.slit_session = Some(session);
                }
                ToolDragEvent::End => {
                    if let Some(session) = &mut self.slit_session {
                        session.dragging = None;
                    }
                }
                ToolDragEvent::Cancel => {
                    // The point goes back where it was (or away, if new).
                    let Some(mut session) = self.slit_session.take() else {
                        continue;
                    };
                    match session.dragging.take() {
                        Some((i, Some(was))) => session.points[i] = was,
                        Some((i, None)) => {
                            session.points.remove(i);
                        }
                        None => {}
                    }
                    self.record_slit(&mut session);
                    self.slit_session = Some(session);
                }
            }
        }
    }

    /// A press (or click) with Slit: takes a point to drag, removes one
    /// (Ctrl), adds one on the line, or adds one at the end.
    fn press_slit(&mut self, drawing: &SceneDrawing, pointer: ViewPointer, keys: egui::Modifiers) {
        let mut session = match self.slit_session.take() {
            Some(session) => session,
            None => {
                let objects: Vec<(ObjectId, Mat4)> = drawing
                    .items
                    .iter()
                    .filter(|i| i.selected && i.transform.is_finite())
                    .map(|i| (i.id, i.transform))
                    .collect();
                if objects.is_empty() {
                    self.note = Some("Slit: select an object to cut first.".into());
                    return;
                }
                self.sessions += 1;
                SlitSession {
                    key: SLIT_KEYS | self.sessions,
                    objects,
                    points: Vec::new(),
                    camera: pointer.camera,
                    through: self.attributes.slit_through,
                    dragging: None,
                    steps: self.document.undo_steps(),
                }
            }
        };
        let screen: Vec<Option<Vec2>> = session
            .points
            .iter()
            .map(|p| on_screen(&pointer, *p))
            .collect();
        let near_point = screen
            .iter()
            .enumerate()
            .filter_map(|(i, s)| Some((i, (*s)?.distance(pointer.at))))
            .filter(|(_, d)| *d <= POINT_REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        session.camera = pointer.camera;
        if keys.command {
            // Ctrl + click: remove a point.
            if let Some(i) = near_point {
                session.points.remove(i);
                self.record_slit(&mut session);
            }
            self.slit_session = Some(session);
            return;
        }
        if let Some(i) = near_point {
            session.dragging = Some((i, Some(session.points[i])));
            self.slit_session = Some(session);
            return;
        }
        // On the line: a new point there.
        let on_line = screen
            .windows(2)
            .enumerate()
            .filter_map(|(i, pair)| {
                let (a, b) = (pair[0]?, pair[1]?);
                Some((i, to_segment(pointer.at, a, b).0))
            })
            .filter(|(_, d)| *d <= LINE_REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i + 1);
        let (at, from) = match on_line {
            Some(i) => (i, Some(session.points[i - 1])),
            None => (session.points.len(), session.points.last().copied()),
        };
        let spot = self.slit_place(drawing, pointer, from, keys.shift);
        session.points.insert(at, spot);
        session.dragging = Some((at, None));
        if session.points.len() > mesh::slit::MAX_LINE_POINTS {
            session.points.truncate(mesh::slit::MAX_LINE_POINTS);
            session.dragging = None;
        }
        self.record_slit(&mut session);
        self.slit_session = Some(session);
    }
}
