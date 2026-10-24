//! Draw curve ([decision 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)):
//! placing a new curve's points in the viewport.
//!
//! The controls, chosen by the project lead (like the pen tool in
//! drawing programs): click to place a point (a sharp corner); press and
//! drag to pull out its handles for a smooth bend. Enter or a right click
//! finishes the curve, Esc cancels it, Backspace takes the last point
//! back, and clicking the first point again closes the curve into a loop
//! and finishes it. While drawing, the stretch the next click would add
//! follows the mouse.
//!
//! Points go on a construction plane chosen with the first point: the
//! ground (the grid) in the perspective view, and in a flat view the
//! plane facing it through the middle of the world (its grid). With
//! snapping on, a point snaps to a point, edge or polygon under the
//! mouse, else to the grid's lines.
//!
//! Nothing is recorded until the curve is finished: then it is added as
//! one new object, one undo step, its pivot in the middle of its points.
//!
//! Clicking an end of a selected open curve first carries on drawing from
//! that end ([decision 0139](../../../../docs/decisions/0139-spline-editing.md)):
//! the new points join that curve (in front of its first point, from the
//! start end), as one undo step; clicking its other end closes it.

use super::Workbench;
use crate::components::ViewPointer;
use crate::drawing::SceneDrawing;
use crate::objects::DocRequest;
use crate::palette::Mode;
use crate::shape_picker::Shape;
use crate::text::tr;
use crate::transforming::Tool;
use crate::viewport::{ToolDragEvent, ToolPath};
use glam::{Vec2, Vec3};
use scene::{
    Command, Curve, CurveObject, CurveShape, Object, ObjectId, ObjectKind, Place, Transform,
};

/// How near (screen points) the mouse must be to the first point to close
/// the curve on it.
pub(super) const POINT_REACH: f32 = 9.0;
/// How far (screen points) a press may move and still be a click: a sharp
/// point, with no handles.
const CLICK_SLACK: f32 = 4.0;
/// How finely the curve is followed while drawing: pieces per stretch.
const PREVIEW_DETAIL: u32 = 24;

/// A point placed so far, in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placed {
    at: Vec3,
    /// Its out handle (its in handle is the opposite), from the point.
    handle: Vec3,
}

/// A curve being carried on from one of its ends.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Continuing {
    /// The curve.
    id: ObjectId,
    /// From its first point (new points go in front), else its last.
    at_start: bool,
    /// Its other end, in the world: clicking it closes the curve.
    other_end: Vec3,
    /// How many points it has.
    count: usize,
}

/// A curve being drawn.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CurveSession {
    /// The points placed, in the world; when carrying a curve on, the
    /// first is the end it carries on from.
    points: Vec<Placed>,
    /// The curve being carried on, if any.
    continuing: Option<Continuing>,
    /// The plane the points go on: a point on it and its facing.
    plane: (Vec3, Vec3),
    /// Where the press placing the last point began, on screen, while the
    /// button is down.
    pressing: Option<Vec2>,
}

impl CurveSession {
    /// The point that closes the curve when clicked, if the curve would
    /// have enough points for a loop: the first point drawn, or the other
    /// end of the curve carried on.
    fn closing_point(&self) -> Option<Vec3> {
        match self.continuing {
            Some(c) => {
                (self.points.len() >= 2 && c.count + self.points.len() > 3).then_some(c.other_end)
            }
            None => (self.points.len() >= 3).then(|| self.points[0].at),
        }
    }

    /// Whether `pointer` is over the point that closes the curve.
    fn over_closing(&self, pointer: &ViewPointer) -> bool {
        self.closing_point().is_some_and(|at| {
            pointer
                .camera
                .project(at, pointer.size)
                .is_some_and(|s| s.distance(pointer.at) <= POINT_REACH)
        })
    }

    /// The curve drawn so far (closed or not), in the world.
    fn curve(&self, closed: bool) -> Option<Curve> {
        if self.points.is_empty() {
            return None;
        }
        Curve::new(
            self.points.iter().map(|p| {
                let smooth = p.handle.length_squared() > 1e-8;
                (p.at, -p.handle, p.handle, smooth)
            }),
            closed,
        )
        .ok()
    }
}

/// Where the ray from `pointer` meets the plane through `on` facing
/// `normal`, if in front of the camera.
fn on_plane(pointer: &ViewPointer, (on, normal): (Vec3, Vec3)) -> Option<Vec3> {
    let ray = pointer.camera.ray_through(pointer.at, pointer.size);
    let facing = ray.direction.dot(normal);
    if facing.abs() < 1e-4 {
        return None;
    }
    let t = (on - ray.origin).dot(normal) / facing;
    (t > 0.0 && t.is_finite()).then(|| ray.at(t))
}

impl Workbench {
    /// Carries out a choice from the Create menu, a shape list or command
    /// search: a ready-made shape or curve, or Draw curve.
    pub(crate) fn create_shape(&mut self, shape: Shape, requests: &mut Vec<DocRequest>) {
        match shape {
            Shape::Primitive(kind) => requests.push(super::create(kind)),
            Shape::Curve(kind) => requests.push(DocRequest::Run(Command::Create(
                ObjectKind::Curve(CurveObject::line(CurveShape::Ready(kind.standard()))),
            ))),
            Shape::DrawCurve => self.start_drawing_curve(),
        }
    }

    /// Picks up Draw curve: in the Model mode (curves are objects), with
    /// no curve begun yet.
    pub(crate) fn start_drawing_curve(&mut self) {
        if crate::components::kind_for(self.mode).is_some() {
            self.mode = Mode::Model;
        }
        self.set_tool(Tool::DrawCurve);
    }

    /// The construction plane for the view `pointer` is in: the ground in
    /// the perspective view, else the plane facing the view through the
    /// middle of the world (where its grid is).
    fn construction_plane(pointer: &ViewPointer) -> (Vec3, Vec3) {
        if pointer.camera.is_flat() {
            (Vec3::ZERO, -pointer.camera.forward())
        } else {
            (Vec3::ZERO, Vec3::Z)
        }
    }

    /// Where a point placed at `pointer` goes: on `plane`, snapped when
    /// snapping is on (Ctrl flips it), or `None` if the plane can't be
    /// reached from this view.
    fn curve_spot(
        &mut self,
        drawing: &SceneDrawing,
        pointer: ViewPointer,
        plane: (Vec3, Vec3),
        ctrl: bool,
    ) -> Option<Vec3> {
        let options = self.viewport.options();
        let snap = options.snap;
        let grid = options.grid_detail;
        if snap.active(ctrl) {
            let found = self.components.pick_scene(drawing).snap_target(
                &pointer.camera,
                pointer.size,
                pointer.at,
                snap.targets(),
                &|_| false,
                &|_, _| false,
            );
            if found.is_some() {
                return found;
            }
            if snap.grid {
                let step = crate::transforming::grid_step(grid);
                return on_plane(&pointer, plane).map(|p| (p / step).round() * step);
            }
        }
        on_plane(&pointer, plane)
    }

    /// Takes the frame's presses, drags and clicks with Draw curve.
    pub(super) fn carry_out_curve(&mut self, drawing: &SceneDrawing, keys: egui::Modifiers) {
        if self.viewport.take_finish_click() {
            self.finish_curve_and_put_away();
            return;
        }
        for event in self.viewport.take_tool_events() {
            match event {
                ToolDragEvent::Begin { pointer } => self.press_curve(drawing, pointer, keys),
                ToolDragEvent::Update { pointer, .. } => self.pull_curve(pointer),
                ToolDragEvent::End => {
                    if let Some(session) = &mut self.curve_session {
                        session.pressing = None;
                    }
                }
                ToolDragEvent::Cancel => {
                    // A right click mid-drag: the point being placed goes.
                    if let Some(session) = &mut self.curve_session
                        && session.pressing.take().is_some()
                    {
                        session.points.pop();
                    }
                }
            }
        }
    }

    /// A press: closes the curve on its first point, or places a new one.
    fn press_curve(&mut self, drawing: &SceneDrawing, pointer: ViewPointer, keys: egui::Modifiers) {
        // On the first point (or the carried-on curve's other end), with
        // enough points for a loop: close it.
        if let Some(session) = &self.curve_session
            && session.over_closing(&pointer)
        {
            self.close_curve();
            return;
        }
        // The first click on an end of a selected open curve carries it on.
        if self.curve_session.is_none()
            && let Some((continuing, end)) = self.curve_end_under(&pointer)
        {
            let plane = (end.at, Self::construction_plane(&pointer).1);
            self.curve_session = Some(CurveSession {
                points: vec![end],
                continuing: Some(continuing),
                plane,
                pressing: None,
            });
            self.note = Some(
                tr(
                    "Draw curve: carrying the curve on from this end. Click to add points; \
                     Enter or a right click finishes, Esc cancels.",
                )
                .into(),
            );
            return;
        }
        let plane = self
            .curve_session
            .as_ref()
            .map_or_else(|| Self::construction_plane(&pointer), |s| s.plane);
        let Some(at) = self.curve_spot(drawing, pointer, plane, keys.command) else {
            self.note = Some(
                tr("Draw curve: this view looks along the curve's plane; draw in another view.")
                    .into(),
            );
            return;
        };
        let session = self.curve_session.get_or_insert(CurveSession {
            points: Vec::new(),
            continuing: None,
            plane,
            pressing: None,
        });
        if session.points.len() >= mesh::curve::MAX_CURVE_POINTS {
            self.note = Some(tr("Draw curve: a curve can have at most 4096 points.").into());
            return;
        }
        session.points.push(Placed {
            at,
            handle: Vec3::ZERO,
        });
        session.pressing = Some(pointer.at);
        let count = session.points.len();
        self.note = Some(crate::trf!(
            "Draw curve: {} points. Drag for a smooth bend; Enter or a right click finishes, \
             Esc cancels, Backspace takes the last point back.",
            count
        ));
    }

    /// A drag while placing a point: pulls its handles out (a smooth bend).
    fn pull_curve(&mut self, pointer: ViewPointer) {
        let Some(session) = &self.curve_session else {
            return;
        };
        let (Some(start), Some(last)) = (session.pressing, session.points.last().copied()) else {
            return;
        };
        let plane = (last.at, session.plane.1);
        // Handles follow the mouse freely: snapping is for points.
        let pulled = if start.distance(pointer.at) <= CLICK_SLACK {
            None
        } else {
            on_plane(&pointer, plane)
        };
        if let Some(session) = &mut self.curve_session
            && let Some(point) = session.points.last_mut()
        {
            point.handle = pulled.map_or(Vec3::ZERO, |p| p - point.at);
        }
    }

    /// Backspace: takes the last point back.
    pub(super) fn take_back_curve_point(&mut self) {
        if let Some(session) = &mut self.curve_session {
            // The end a curve is carried on from stays.
            if session.continuing.is_none() || session.points.len() > 1 {
                session.points.pop();
            }
            session.pressing = None;
            let count = session.points.len();
            self.note = Some(crate::trf!("Draw curve: {} points.", count));
        }
    }

    /// Closes the curve into a loop and finishes it.
    fn close_curve(&mut self) {
        if let Some(session) = self.curve_session.take() {
            match session.continuing {
                Some(continuing) => self.carry_on_curve(&session, continuing, true),
                None => self.add_curve(&session, true),
            }
        }
        self.put_curve_tool_away();
    }

    /// Finishes the curve being drawn and goes back to selecting, keeping
    /// what finishing said in the status bar.
    pub(super) fn finish_curve_and_put_away(&mut self) {
        self.finish_curve();
        self.put_curve_tool_away();
    }

    /// Back to the Select tool, keeping the status bar's note.
    fn put_curve_tool_away(&mut self) {
        let note = self.note.take();
        self.set_tool(Tool::Select);
        if note.is_some() {
            self.note = note;
        }
    }

    /// Finishes the curve being drawn (Enter, a right click, another tool
    /// or mode), adding it if it has two points or more.
    pub(super) fn finish_curve(&mut self) {
        if let Some(session) = self.curve_session.take() {
            if let Some(continuing) = session.continuing {
                if session.points.len() >= 2 {
                    self.carry_on_curve(&session, continuing, false);
                } else {
                    self.note = Some(tr("Draw curve: no points were added.").into());
                }
            } else if session.points.len() >= 2 {
                self.add_curve(&session, false);
            } else {
                self.note =
                    Some(tr("Draw curve: a curve needs two points; nothing was made.").into());
            }
        }
        self.viewport.set_tool_path(None);
    }

    /// Cancels the curve being drawn (Esc), and puts the tool away.
    pub(super) fn cancel_curve(&mut self) {
        self.curve_session = None;
        self.viewport.set_tool_path(None);
        self.note = Some(tr("Draw curve cancelled: nothing was made.").into());
        self.put_curve_tool_away();
    }

    /// Adds the session's curve as a new object, selected, with its pivot
    /// in the middle of its points: one undo step.
    fn add_curve(&mut self, session: &CurveSession, closed: bool) {
        let Some(world) = session.curve(closed) else {
            return;
        };
        let middle = mesh::Bounds::around(session.points.iter().map(|p| p.at))
            .map_or(Vec3::ZERO, |b| b.centre());
        let Ok(own) = world.transformed(glam::Mat4::from_translation(-middle)) else {
            return;
        };
        let mut object = Object::new(ObjectKind::Curve(CurveObject::line(CurveShape::Points(
            own,
        ))));
        object.name = self.document.scene().unused_name(tr("Curve"));
        object.transform = Transform {
            position: middle,
            ..Transform::default()
        };
        let command = Command::Insert {
            object,
            place: Place {
                parent: None,
                index: 0,
            },
        };
        match self
            .document
            .execute_labelled(command, tr("Draw curve"), None)
        {
            Ok(()) => {
                self.note = Some(crate::trf!(
                    "Curve made with {} points{}. Give it a sweep or a lathe in the Attributes \
                     panel; press 1 to edit its points.",
                    session.points.len(),
                    if closed { tr(", closed") } else { "" }
                ));
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't do that: {}.", why)),
        }
    }

    /// The ends of the selected open curves, in the world.
    fn selected_curve_ends(&self) -> Vec<Vec3> {
        let scene = self.document.scene();
        self.selected_curves()
            .into_iter()
            .filter_map(|id| {
                let curve = scene.object(id)?.kind.curve()?.shape.curve();
                if curve.closed() {
                    return None;
                }
                let matrix = scene.shape_matrix(id);
                let first = curve.points().first()?.position;
                let last = curve.points().last()?.position;
                Some([
                    matrix.transform_point3(first),
                    matrix.transform_point3(last),
                ])
            })
            .flatten()
            .collect()
    }

    /// The end of a selected open curve under `pointer`, if any: what
    /// carrying it on needs, and the end as a placed point (its handle
    /// pointing outward).
    fn curve_end_under(&self, pointer: &ViewPointer) -> Option<(Continuing, Placed)> {
        let scene = self.document.scene();
        let mut best: Option<(f32, Continuing, Placed)> = None;
        for id in self.selected_curves() {
            let Some(curve) = scene
                .object(id)
                .and_then(|o| o.kind.curve())
                .map(|c| c.shape.curve())
            else {
                continue;
            };
            let points = curve.points();
            let (Some(first), Some(last)) = (points.first(), points.last()) else {
                continue;
            };
            if curve.closed() || points.len() < 2 {
                continue;
            }
            let matrix = scene.shape_matrix(id);
            for (at_start, end, other) in [(true, first, last), (false, last, first)] {
                let at = matrix.transform_point3(end.position);
                let Some(on_screen) = pointer.camera.project(at, pointer.size) else {
                    continue;
                };
                let d = on_screen.distance(pointer.at);
                if d > POINT_REACH || best.as_ref().is_some_and(|b| b.0 <= d) {
                    continue;
                }
                let outward = if at_start {
                    end.handle_in
                } else {
                    end.handle_out
                };
                best = Some((
                    d,
                    Continuing {
                        id,
                        at_start,
                        other_end: matrix.transform_point3(other.position),
                        count: points.len(),
                    },
                    Placed {
                        at,
                        handle: matrix.transform_vector3(outward),
                    },
                ));
            }
        }
        best.map(|(_, c, p)| (c, p))
    }

    /// Adds the points drawn after the end of the curve being carried on,
    /// as one undo step; `close` closes it on its other end too.
    fn carry_on_curve(&mut self, session: &CurveSession, continuing: Continuing, close: bool) {
        let scene = self.document.scene();
        let into = scene.shape_matrix(continuing.id).inverse();
        if !into.is_finite() {
            return;
        }
        let points: Vec<_> = session
            .points
            .iter()
            .skip(1)
            .map(|p| {
                let handle = into.transform_vector3(p.handle);
                let kind = if handle.length_squared() > 1e-8 {
                    scene::PointKind::Smooth
                } else {
                    scene::PointKind::Sharp
                };
                (into.transform_point3(p.at), -handle, handle, kind)
            })
            .collect();
        let added = points.len();
        let key = self.new_step_key();
        let label = tr("Carry on curve");
        let mut commands = vec![Command::EditCurve {
            id: continuing.id,
            edit: scene::CurveEdit::Extend {
                at_start: continuing.at_start,
                points,
            },
        }];
        if close {
            commands.push(Command::EditCurve {
                id: continuing.id,
                edit: scene::CurveEdit::SetClosed(true),
            });
        }
        for command in commands {
            if let Err(why) = self.document.execute_labelled(command, label, Some(key)) {
                self.note = Some(crate::trf!("Couldn't do that: {}.", why));
                return;
            }
        }
        self.note = Some(crate::trf!(
            "Curve carried on by {} point{}{}. Ctrl + Z takes it back.",
            added,
            if added == 1 { "" } else { "s" },
            if close { tr(", closed") } else { "" }
        ));
    }

    /// Shows the curve being drawn, and the stretch the next click would
    /// add.
    pub(super) fn show_curve(&mut self, drawing: &SceneDrawing) {
        if self.tool != Tool::DrawCurve {
            return;
        }
        let pointer = self.viewport.pointer();
        let Some(session) = self.curve_session.clone() else {
            // Nothing yet: a ring where the first point would go, and the
            // ends of selected open curves, which can be carried on.
            let over_end = pointer
                .as_ref()
                .and_then(|p| self.curve_end_under(p))
                .map(|(_, end)| end.at);
            let cursor = over_end.or_else(|| {
                pointer.and_then(|p| {
                    let plane = Self::construction_plane(&p);
                    self.curve_spot(drawing, p, plane, false)
                })
            });
            let ends = self.selected_curve_ends();
            self.viewport.set_tool_path(Some(ToolPath {
                cursor,
                dots: (!ends.is_empty()).then_some(ends),
                ..ToolPath::default()
            }));
            return;
        };
        let line = session
            .curve(false)
            .map(|c| c.polyline(PREVIEW_DETAIL))
            .unwrap_or_default();
        let dots: Vec<Vec3> = session.points.iter().map(|p| p.at).collect();
        let handles = session
            .points
            .iter()
            .filter(|p| p.handle.length_squared() > 1e-8)
            .flat_map(|p| [[p.at, p.at - p.handle], [p.at, p.at + p.handle]])
            .collect();
        // The stretch from the last point to the mouse (or, over the first
        // point, back to it), unless a point is being pulled.
        let mut cursor = None;
        let mut preview = Vec::new();
        if session.pressing.is_none()
            && let (Some(pointer), Some(last)) = (pointer, session.points.last().copied())
        {
            let closing = session
                .closing_point()
                .filter(|_| session.over_closing(&pointer));
            let next = if let Some(at) = closing {
                Some(Placed {
                    at,
                    handle: Vec3::ZERO,
                })
            } else {
                self.curve_spot(drawing, pointer, session.plane, false)
                    .map(|at| Placed {
                        at,
                        handle: Vec3::ZERO,
                    })
            };
            if let Some(next) = next {
                let span = mesh::curve::Span {
                    start: last.at,
                    pull_start: last.at + last.handle,
                    pull_end: next.at - next.handle,
                    end: next.at,
                };
                preview = (0..=PREVIEW_DETAIL)
                    .map(|k| span.at(k as f32 / PREVIEW_DETAIL as f32))
                    .collect();
                cursor = Some(next.at);
            }
        }
        self.viewport.set_tool_path(Some(ToolPath {
            points: line,
            cursor,
            dots: Some(dots),
            handles,
            preview,
        }));
    }
}
