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
use scene::{Command, Curve, CurveObject, CurveShape, Object, ObjectKind, Place, Transform};

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

/// A curve being drawn.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CurveSession {
    points: Vec<Placed>,
    /// The plane the points go on: a point on it and its facing.
    plane: (Vec3, Vec3),
    /// Where the press placing the last point began, on screen, while the
    /// button is down.
    pressing: Option<Vec2>,
}

impl CurveSession {
    /// The curve drawn so far (closed or not), in the world.
    fn curve(&self, closed: bool) -> Option<Curve> {
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
        // On the first point, with enough points for a loop: close it.
        if let Some(session) = &self.curve_session
            && session.points.len() >= 3
            && pointer
                .camera
                .project(session.points[0].at, pointer.size)
                .is_some_and(|s| s.distance(pointer.at) <= POINT_REACH)
        {
            self.close_curve();
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
            session.points.pop();
            session.pressing = None;
            let count = session.points.len();
            self.note = Some(crate::trf!("Draw curve: {} points.", count));
        }
    }

    /// Closes the curve into a loop and finishes it.
    fn close_curve(&mut self) {
        if let Some(session) = self.curve_session.take() {
            self.add_curve(&session, true);
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
            if session.points.len() >= 2 {
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

    /// Shows the curve being drawn, and the stretch the next click would
    /// add.
    pub(super) fn show_curve(&mut self, drawing: &SceneDrawing) {
        if self.tool != Tool::DrawCurve {
            return;
        }
        let pointer = self.viewport.pointer();
        let Some(session) = self.curve_session.clone() else {
            // Nothing yet: a ring where the first point would go.
            let cursor = pointer.and_then(|p| {
                let plane = Self::construction_plane(&p);
                self.curve_spot(drawing, p, plane, false)
            });
            self.viewport.set_tool_path(Some(ToolPath {
                cursor,
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
            let over_first = session.points.len() >= 3
                && pointer
                    .camera
                    .project(session.points[0].at, pointer.size)
                    .is_some_and(|s| s.distance(pointer.at) <= POINT_REACH);
            let next = if over_first {
                Some(session.points[0])
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
