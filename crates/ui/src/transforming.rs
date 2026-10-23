//! Moving, rotating and scaling ([decision 0030](../../../docs/decisions/0030-transform-gizmos.md)):
//! the tools, what they would change, snapping, and turning a gizmo's
//! [`Change`] into commands.
//!
//! The controls, chosen by the project lead:
//!
//! | Action | Control |
//! |---|---|
//! | Move, rotate, scale, all three | `W`, `R`, `S`, `T` (Halberd's keys) |
//! | Back to selecting | `Q` (again: the next select tool) |
//! | Snapping on or off | The palette button; hold `Ctrl` while dragging to flip it for that drag (or only the button, in Settings) |
//! | Ten times slower | Hold `Shift` while dragging |
//! | Cancel a drag | `Esc` or a right click |
//! | Lock X, Y, Z | `X`, `Y`, `Z` (or the palette buttons) |
//! | World or object axes | `L` (or the palette button) |
//!
//! A drag is one undo step: each frame takes back the frame before
//! (`Document::abandon`) and does the whole change from the start again,
//! so nothing piles up and cancelling puts everything back.

use crate::components::kind_for;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::viewport::GridDetail;
use egui::{Key, KeyboardShortcut, Modifiers};
use glam::{Mat4, Quat, Vec2, Vec3};
use mesh::Anchors;
use scene::{Bounds, Command, Document, ElementKind, ObjectId, Operation, Transform, VertexId};
use serde::{Deserialize, Serialize};
use tools::gizmo::{Handle, into_plane, nearest_on_axis, snap};
use tools::{Camera, Change, Gizmo, GizmoTool, PickScene, SnapTargets};

/// The tool in use: what a left drag on the gizmo does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    /// Selecting (no gizmo).
    #[default]
    Select,
    /// The move gizmo.
    Move,
    /// The rotate gizmo.
    Rotate,
    /// The scale gizmo.
    Scale,
    /// All three gizmos at once.
    All,
    /// Extrude: drag anywhere to push the selected polygons or border
    /// edges out (decision 0038).
    Extrude,
    /// Inset: drag anywhere to shrink the selected polygons inward.
    Inset,
    /// Bevel: drag away from the selected edges or points to cut them
    /// away (decision 0040).
    Bevel,
    /// Groove: drag anywhere to cut a thin groove into the shape along the
    /// selected edges, or raise a ridge out of it (decision 0086).
    Groove,
    /// Loop cut: hover an edge to see the loop, click to cut it, drag to
    /// slide it (decision 0047).
    LoopCut,
    /// Slit: click or drag points of a line over the shape, which cuts it
    /// (decision 0049).
    Slit,
    /// Bridge: drag from one open edge to another to join them (decision
    /// 0051).
    Bridge,
    /// Weld onto a target: drag a point onto another to melt it there
    /// (decision 0051).
    Weld,
}

/// The tool keys (Halberd's, decision 0019): move, rotate, scale, all.
pub const TOOL_KEYS: [(Key, Tool); 4] = [
    (Key::W, Tool::Move),
    (Key::R, Tool::Rotate),
    (Key::S, Tool::Scale),
    (Key::T, Tool::All),
];
/// The keys that lock and free the X, Y and Z axes: Shift with the axis's
/// letter ([decision 0047](../../../docs/decisions/0047-loop-cut.md); plain
/// Z frames the selection, and X is kept for Slit).
pub const LOCK_KEYS: [(KeyboardShortcut, usize); 3] = [
    (KeyboardShortcut::new(Modifiers::SHIFT, Key::X), 0),
    (KeyboardShortcut::new(Modifiers::SHIFT, Key::Y), 1),
    (KeyboardShortcut::new(Modifiers::SHIFT, Key::Z), 2),
];
/// The key that switches between world and object axes.
pub const AXES_KEY: Key = Key::L;

impl Tool {
    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => tr("Select"),
            Self::Move => tr("Move"),
            Self::Rotate => tr("Rotate"),
            Self::Scale => tr("Scale"),
            Self::All => tr("Transform"),
            Self::Extrude => tr("Extrude"),
            Self::Inset => tr("Inset"),
            Self::Bevel => tr("Bevel"),
            Self::Groove => tr("Groove"),
            Self::LoopCut => tr("Loop cut"),
            Self::Slit => tr("Slit"),
            Self::Bridge => tr("Bridge"),
            Self::Weld => tr("Weld"),
        }
    }

    /// Whether it is a modelling tool set by dragging anywhere in a view.
    pub fn drags_anywhere(self) -> bool {
        matches!(
            self,
            Self::Extrude | Self::Inset | Self::Bevel | Self::Groove
        )
    }

    /// Whether it is a modelling tool used by clicking (and dragging) on
    /// the shape itself: a left click never selects while it is in use.
    pub fn clicks_on_shapes(self) -> bool {
        matches!(self, Self::LoopCut | Self::Slit)
    }

    /// Whether it is a modelling tool used by dragging from one point or
    /// edge to another (Bridge, Weld). A drag that starts on one is the
    /// tool's; a click, or a drag from empty space, selects as usual.
    pub fn drags_between_elements(self) -> bool {
        matches!(self, Self::Bridge | Self::Weld)
    }

    /// Whether it has a count that Ctrl + wheel changes while it is in use
    /// (Bevel's segments, Loop cut's cuts; [decision 0048](../../../docs/decisions/0048-ctrl-wheel-changes-a-tools-count.md)).
    pub fn has_wheel_count(self) -> bool {
        matches!(
            self,
            Self::Bevel | Self::LoopCut | Self::Extrude | Self::Bridge
        )
    }

    /// Whether it has settings in the Attributes panel for its next use.
    pub fn has_settings(self) -> bool {
        self.drags_anywhere() || self.clicks_on_shapes() || self.drags_between_elements()
    }

    /// Its gizmo, if it has one.
    pub fn gizmo(self) -> Option<GizmoTool> {
        match self {
            Self::Select
            | Self::Extrude
            | Self::Inset
            | Self::Bevel
            | Self::Groove
            | Self::LoopCut
            | Self::Slit
            | Self::Bridge
            | Self::Weld => None,
            Self::Move => Some(GizmoTool::Move),
            Self::Rotate => Some(GizmoTool::Rotate),
            Self::Scale => Some(GizmoTool::Scale),
            Self::All => Some(GizmoTool::All),
        }
    }
}

/// How snapping works, chosen in Settings and remembered.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SnapSettings {
    /// Whether snapping is on (the palette button).
    pub on: bool,
    /// Whether holding Ctrl while dragging flips it for that drag. Off:
    /// only the button turns it on and off.
    pub ctrl_flips: bool,
    /// Moves snap to the grid lines shown.
    pub grid: bool,
    /// Moves snap to points.
    pub points: bool,
    /// Moves snap to spots along edges.
    pub edges: bool,
    /// Moves snap to spots on polygons.
    pub polygons: bool,
    /// Turns snap to this many degrees.
    pub angle: f32,
    /// Scaling snaps to steps of this part (0.1 = 10%).
    pub scale: f32,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            on: false,
            ctrl_flips: true,
            grid: true,
            points: true,
            edges: true,
            polygons: true,
            angle: 15.0,
            scale: 0.1,
        }
    }
}

impl SnapSettings {
    /// Whether snapping applies to a drag with Ctrl held (`ctrl`).
    pub fn active(&self, ctrl: bool) -> bool {
        self.on != (ctrl && self.ctrl_flips)
    }

    fn targets(&self) -> SnapTargets {
        SnapTargets {
            points: self.points,
            edges: self.edges,
            polygons: self.polygons,
        }
    }
}

/// How the gizmo's axes point: the world's, or the object's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Axes {
    /// The world's X, Y and Z.
    #[default]
    World,
    /// The (first) object's own axes, turned with it.
    Object,
}

/// One object to move, rotate or scale whole, as it was when the change
/// began.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ObjectStart {
    pub(crate) id: ObjectId,
    pub(crate) name: String,
    /// Where it was in the world.
    pub(crate) world: Mat4,
    /// Where its parent is in the world.
    pub(crate) parent: Mat4,
    pub(crate) transform: Transform,
}

/// One object's points to move, rotate or scale.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PointsStart {
    pub(crate) id: ObjectId,
    /// Where the object is in the world.
    pub(crate) world: Mat4,
    pub(crate) points: Vec<VertexId>,
    /// Where the selected elements are in the shape, so the edit can find
    /// them again if the shape's points are renumbered (decision 0032).
    pub(crate) anchors: Option<Anchors>,
    /// While sliding (Alt + drag on the Move gizmo, decision 0055): the
    /// shape as it was, whose edges the points slide along.
    pub(crate) slide: Option<std::sync::Arc<scene::Mesh>>,
    /// The edit this drag continues (decision 0126): the object's last
    /// edit, when it moved, turned or scaled these same points in the
    /// drag just before, with nothing else done since. Its layer, its
    /// place in the layer, and the edit; the drag changes it rather than
    /// adding another, so drags in a row stay one entry in the edit list.
    pub(crate) follows: Option<(usize, usize, Operation)>,
}

/// A drag of points' undo label, from its verb and count ("Move 1
/// point", "Scale 8 points"): made by `Target::label`, read back
/// by [`follows`].
const ONE_POINT_DRAG: &str = crate::text!("{} {} point");
/// A drag of more than one point's undo label; see [`ONE_POINT_DRAG`].
const POINTS_DRAG: &str = crate::text!("{} {} points");

/// Whether `label` is a drag of points moved, turned or scaled: the label
/// [`Target::label`] gives them, with any count, in the language in use.
fn is_points_drag(label: &str) -> bool {
    [tr("Move"), tr("Rotate"), tr("Scale")].iter().any(|verb| {
        [tr(ONE_POINT_DRAG), tr(POINTS_DRAG)]
            .iter()
            .any(|template| {
                let count = '\u{1}';
                crate::text::fill(template, &[verb, &count])
                    .split_once(count)
                    .is_some_and(|(start, end)| label.starts_with(start) && label.ends_with(end))
            })
    })
}

/// The edit a drag of `points` on `id` continues, if any (decision 0126):
/// the object's last edit is a move, turn or scale of exactly these
/// points, and the last undo step was that drag.
fn follows(
    document: &scene::Document,
    id: ObjectId,
    points: &[VertexId],
) -> Option<(usize, usize, Operation)> {
    if !is_points_drag(&document.undo_label()?) {
        return None;
    }
    let layers = document.scene().object(id)?.stack.layers();
    let layer = layers.len().checked_sub(1)?;
    let entries = &layers[layer].entries;
    let index = entries.len().checked_sub(1)?;
    let entry = &entries[index];
    if !entry.enabled {
        return None;
    }
    let same = |ids: &[VertexId]| {
        let mut ids = ids.to_vec();
        ids.sort_unstable();
        ids.dedup();
        ids == points
    };
    match &entry.operation {
        Operation::MovePoints { points: p, .. } | Operation::TransformPoints { points: p, .. }
            if same(p) =>
        {
            Some((layer, index, entry.operation.clone()))
        }
        _ => None,
    }
}

/// `before` and then `after` (both on the same points) as one edit: two
/// moves add up; anything else becomes one matrix (decision 0126).
pub(crate) fn combined(before: &Operation, after: Operation) -> Operation {
    let matrix = |op: &Operation| match op {
        Operation::MovePoints { offset, .. } => Some(Mat4::from_translation(*offset)),
        Operation::TransformPoints { matrix, .. } => Some(*matrix),
        _ => None,
    };
    match (before, &after) {
        (
            Operation::MovePoints {
                points,
                offset: a,
                anchors,
            },
            Operation::MovePoints { offset: b, .. },
        ) => Operation::MovePoints {
            points: points.clone(),
            offset: *a + *b,
            anchors: anchors.clone(),
        },
        (
            Operation::MovePoints {
                points, anchors, ..
            }
            | Operation::TransformPoints {
                points, anchors, ..
            },
            _,
        ) => match (matrix(before), matrix(&after)) {
            (Some(first), Some(second)) => Operation::TransformPoints {
                points: points.clone(),
                matrix: second * first,
                anchors: anchors.clone(),
            },
            _ => after,
        },
        _ => after,
    }
}

/// What a change works on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Targets {
    /// Whole objects (the Model mode).
    Objects(Vec<ObjectStart>),
    /// Points of the selected objects (the Points, Edges and Polygons
    /// modes): the selected points, or the points of the selected edges
    /// or polygons.
    Points(Vec<PointsStart>, ElementKind),
}

/// What the gizmo would change, and where it sits.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Target {
    pub(crate) pivot: Vec3,
    pub(crate) axes: [Vec3; 3],
    /// The box around what would change, in the world.
    pub(crate) bounds: Bounds,
    pub(crate) targets: Targets,
    /// Whether objects' pivots move instead of the objects (the Object
    /// mode, decision 0068): the shapes stay where they are.
    pub(crate) pivots: bool,
}

impl Target {
    /// What the gizmo would change now, in `mode`: `None` when nothing
    /// is selected that can move.
    pub(crate) fn of(
        document: &Document,
        drawing: &SceneDrawing,
        mode: Mode,
        axes: Axes,
    ) -> Option<Self> {
        let scene = document.scene();
        match kind_for(mode) {
            None => {
                let selection = document.selection();
                // Objects inside another selected one move with it.
                let top: Vec<ObjectId> = selection
                    .iter()
                    .copied()
                    .filter(|id| {
                        !selection
                            .iter()
                            .any(|o| o != id && scene.is_inside(*id, *o))
                    })
                    .collect();
                let starts: Vec<ObjectStart> = top
                    .iter()
                    .filter_map(|id| {
                        let object = scene.object(*id)?;
                        Some(ObjectStart {
                            id: *id,
                            name: object.name.clone(),
                            world: scene.world_matrix(*id),
                            parent: scene
                                .parent(*id)
                                .map_or(Mat4::IDENTITY, |p| scene.world_matrix(p)),
                            transform: object.transform,
                        })
                    })
                    .collect();
                let first = starts.first()?;
                let pivot = starts
                    .iter()
                    .map(|s| s.world.w_axis.truncate())
                    .sum::<Vec3>()
                    / starts.len() as f32;
                let bounds = scene
                    .bounds_of(top.iter().copied())
                    .or_else(|| Bounds::new(pivot, pivot))?;
                Some(Self {
                    pivot,
                    axes: axes_of(axes, first.world),
                    bounds,
                    targets: Targets::Objects(starts),
                    pivots: mode == Mode::Object,
                })
            }
            Some(kind) => {
                let mut starts = Vec::new();
                let mut world_points = Vec::new();
                for item in drawing.items.iter().filter(|i| i.selected) {
                    // A shape squashed flat (scaled to 0) can't have its
                    // points moved: the change can't be worked out in its
                    // own space.
                    if item.transform.determinant().abs() < 1e-12 {
                        continue;
                    }
                    let Some(selected) = document.elements(item.id) else {
                        continue;
                    };
                    let topology = item.mesh.topology();
                    let valid = selected.valid_in(&item.mesh, topology);
                    let elements: Vec<Vec<VertexId>> = match kind {
                        ElementKind::Points => valid.points().map(|v| vec![v]).collect(),
                        ElementKind::Edges => valid
                            .edges()
                            .map(|e| {
                                let (a, b) = e.ends();
                                vec![a, b]
                            })
                            .collect(),
                        ElementKind::Polygons => valid
                            .polygons()
                            .filter_map(|f| Some(item.mesh.face(f)?.to_vec()))
                            .collect(),
                    };
                    let anchors = Anchors::record(&item.mesh, kind, &elements);
                    let mut points: Vec<VertexId> = match kind {
                        ElementKind::Points => valid.points().collect(),
                        ElementKind::Edges => valid
                            .edges()
                            .flat_map(|e| {
                                let (a, b) = e.ends();
                                [a, b]
                            })
                            .collect(),
                        ElementKind::Polygons => valid
                            .polygons()
                            .flat_map(|f| item.mesh.face(f).unwrap_or_default().to_vec())
                            .collect(),
                    };
                    points.sort_unstable();
                    points.dedup();
                    if points.is_empty() {
                        continue;
                    }
                    world_points.extend(
                        points
                            .iter()
                            .filter_map(|v| item.mesh.position(*v))
                            .map(|p| item.transform.transform_point3(p)),
                    );
                    let follows = follows(document, item.id, &points);
                    starts.push(PointsStart {
                        id: item.id,
                        world: item.transform,
                        points,
                        anchors,
                        slide: None,
                        follows,
                    });
                }
                let first = starts.first()?;
                let bounds = Bounds::around(world_points)?;
                Some(Self {
                    pivot: bounds.centre(),
                    axes: axes_of(axes, first.world),
                    bounds,
                    targets: Targets::Points(starts, kind),
                    pivots: false,
                })
            }
        }
    }

    /// The same target, sliding: a move slides the points along their
    /// edges instead (decision 0055). Objects are unchanged.
    #[must_use]
    pub(crate) fn sliding(mut self, drawing: &SceneDrawing) -> Self {
        if let Targets::Points(starts, _) = &mut self.targets {
            for start in starts {
                start.slide = drawing
                    .items
                    .iter()
                    .find(|i| i.id == start.id)
                    .map(|i| std::sync::Arc::clone(&i.mesh));
            }
        }
        self
    }

    /// Whether this target slides its points.
    pub(crate) fn slides(&self) -> bool {
        matches!(&self.targets, Targets::Points(starts, _) if starts.iter().any(|s| s.slide.is_some()))
    }

    /// The gizmo for `tool` at this target.
    pub(crate) fn gizmo(&self, tool: GizmoTool) -> Gizmo {
        Gizmo {
            origin: self.pivot,
            axes: self.axes,
            tool,
        }
    }

    /// The change as a matrix in the world, around the pivot.
    pub(crate) fn matrix(&self, change: &Change) -> Mat4 {
        let to = Mat4::from_translation(self.pivot);
        let from = Mat4::from_translation(-self.pivot);
        match *change {
            Change::Translate(d) => Mat4::from_translation(d),
            Change::Rotate { axis, angle } => {
                to * Mat4::from_quat(Quat::from_axis_angle(axis, angle)) * from
            }
            Change::Scale(f) => {
                let basis = Mat4::from_cols(
                    self.axes[0].extend(0.0),
                    self.axes[1].extend(0.0),
                    self.axes[2].extend(0.0),
                    glam::Vec4::W,
                );
                to * basis * Mat4::from_scale(f) * basis.transpose() * from
            }
        }
    }

    /// The commands that make `change`, from where everything started.
    pub(crate) fn commands(&self, change: &Change) -> Vec<Command> {
        let m = self.matrix(change);
        match &self.targets {
            Targets::Objects(_) if self.pivots && matches!(change, Change::Scale(_)) => {
                // A pivot has no size: scaling it does nothing.
                Vec::new()
            }
            Targets::Objects(starts) => {
                let changes = starts
                    .iter()
                    .filter_map(|s| {
                        let local = s.parent.inverse() * m * s.world;
                        let mut t = Transform::from_matrix(local)?;
                        if let Change::Translate(_) = change {
                            // A move changes only where it is: keep the
                            // turn and size exactly as they were.
                            t.rotation = s.transform.rotation;
                            t.scale = s.transform.scale;
                        }
                        Some((s.id, t))
                    })
                    .collect();
                vec![self.objects_command(changes)]
            }
            Targets::Points(starts, _) => starts
                .iter()
                .flat_map(|s| {
                    let inverse = s.world.inverse();
                    if let (Some(shape), Change::Translate(d)) = (&s.slide, change) {
                        let slides = mesh::slide::slides_for(
                            shape,
                            &s.points,
                            inverse.transform_vector3(*d),
                        );
                        // The points' places, then their neighbours'.
                        let places: Vec<Vec<VertexId>> = slides
                            .iter()
                            .map(|sl| vec![sl.point])
                            .chain(slides.iter().map(|sl| vec![sl.towards]))
                            .collect();
                        let anchors = Anchors::record(shape, ElementKind::Points, &places);
                        return vec![Command::Record {
                            id: s.id,
                            operation: Operation::SlidePoints { slides, anchors },
                        }];
                    }
                    let operation = match change {
                        Change::Translate(d) => Operation::MovePoints {
                            points: s.points.clone(),
                            offset: inverse.transform_vector3(*d),
                            anchors: s.anchors.clone(),
                        },
                        _ => Operation::TransformPoints {
                            points: s.points.clone(),
                            matrix: inverse * m * s.world,
                            anchors: s.anchors.clone(),
                        },
                    };
                    match &s.follows {
                        // The drag just before moved these points: change
                        // that edit instead of adding another.
                        Some((layer, index, before)) => vec![Command::ReplaceOperation {
                            id: s.id,
                            layer: *layer,
                            index: *index,
                            operation: combined(before, operation),
                        }],
                        None => vec![Command::Record {
                            id: s.id,
                            operation,
                        }],
                    }
                })
                .collect(),
        }
    }

    /// The command that places objects (or, in the Object mode, their
    /// pivots) as `changes` say.
    pub(crate) fn objects_command(&self, changes: Vec<(ObjectId, Transform)>) -> Command {
        if self.pivots {
            Command::MovePivots(changes)
        } else {
            Command::SetTransforms(changes)
        }
    }

    /// The undo step's name: "Move Cube", "Rotate 2 objects", "Scale 8
    /// points"; for pivots, "Move Cube's pivot", "Rotate 2 pivots".
    pub(crate) fn label(&self, change: &Change) -> String {
        let verb = match change {
            Change::Translate(_) if self.slides() => tr("Slide"),
            Change::Translate(_) => tr("Move"),
            Change::Rotate { .. } => tr("Rotate"),
            Change::Scale(_) => tr("Scale"),
        };
        match &self.targets {
            Targets::Objects(starts) => match (starts.as_slice(), self.pivots) {
                ([one], false) => crate::trf!("{} {}", verb, one.name),
                (many, false) => crate::trf!("{} {} objects", verb, many.len()),
                ([one], true) => crate::trf!("{} {}'s pivot", verb, one.name),
                (many, true) => crate::trf!("{} {} pivots", verb, many.len()),
            },
            Targets::Points(starts, _) => {
                let n: usize = starts.iter().map(|s| s.points.len()).sum();
                let template = if n == 1 { ONE_POINT_DRAG } else { POINTS_DRAG };
                crate::text::fill(tr(template), &[&verb, &n])
            }
        }
    }

    /// Whether `object` (its position in the pick scene's list, via
    /// `drawing`) or one of its points is what moves: such things are
    /// never snapped to.
    fn moves_object(&self, drawing: &SceneDrawing, object: usize, document: &Document) -> bool {
        let Some(item) = drawing.items.get(object) else {
            return false;
        };
        match &self.targets {
            // Moving pivots moves no shape: a pivot can snap to its own
            // shape's points (a corner, say).
            Targets::Objects(_) if self.pivots => false,
            Targets::Objects(starts) => starts
                .iter()
                .any(|s| s.id == item.id || document.scene().is_inside(item.id, s.id)),
            Targets::Points(..) => false,
        }
    }

    fn moves_point(&self, drawing: &SceneDrawing, object: usize, point: VertexId) -> bool {
        let (Some(item), Targets::Points(starts, _)) = (drawing.items.get(object), &self.targets)
        else {
            return false;
        };
        starts
            .iter()
            .any(|s| s.id == item.id && s.points.binary_search(&point).is_ok())
    }
}

/// The gizmo's axes for `axes`, given the first object's world matrix.
fn axes_of(axes: Axes, world: Mat4) -> [Vec3; 3] {
    const WORLD: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
    if axes == Axes::World {
        return WORLD;
    }
    let x = world.x_axis.truncate().normalize_or(Vec3::X);
    let y = world.y_axis.truncate();
    let y = (y - x * y.dot(x)).normalize_or(Vec3::Y);
    let z = x.cross(y).normalize_or(Vec3::Z);
    if x.is_finite() && y.is_finite() && z.is_finite() {
        [x, y, z]
    } else {
        WORLD
    }
}

/// Everything snapping needs for one frame of a drag.
pub(crate) struct Snapping<'a> {
    pub(crate) settings: SnapSettings,
    pub(crate) grid: GridDetail,
    pub(crate) scene: &'a PickScene,
    pub(crate) drawing: &'a SceneDrawing,
    pub(crate) document: &'a Document,
    pub(crate) camera: Camera,
    pub(crate) size: Vec2,
    pub(crate) mouse: Vec2,
    pub(crate) locked: [bool; 3],
}

/// The grid step moves snap to: the lines shown.
pub(crate) fn grid_step(grid: GridDetail) -> f32 {
    grid.fine_spacing().unwrap_or(100.0)
}

impl Target {
    /// `change`, snapped (decision 0030): moves to the point, edge or
    /// polygon under the mouse, else to the grid; turns to whole steps of
    /// the snap angle; scaling to whole steps of the snap part.
    pub(crate) fn snapped(&self, change: Change, handle: Handle, snapping: &Snapping) -> Change {
        let s = snapping.settings;
        match change {
            Change::Translate(delta) => {
                let whole =
                    |object: usize| self.moves_object(snapping.drawing, object, snapping.document);
                let moving = |object: usize, point: VertexId| {
                    self.moves_point(snapping.drawing, object, point)
                };
                let found = snapping.scene.snap_target(
                    &snapping.camera,
                    snapping.size,
                    snapping.mouse,
                    s.targets(),
                    &whole,
                    &moving,
                );
                if let Some(target) = found {
                    let to = match handle {
                        Handle::Axis(i) => nearest_on_axis(self.pivot, self.axes[i], target),
                        Handle::Plane(i) => into_plane(self.pivot, self.axes[i], target),
                        _ => target,
                    };
                    let mut moved = to - self.pivot;
                    for (i, locked) in snapping.locked.iter().enumerate() {
                        if *locked {
                            moved -= self.axes[i] * moved.dot(self.axes[i]);
                        }
                    }
                    return Change::Translate(moved);
                }
                if !s.grid {
                    return change;
                }
                let step = grid_step(snapping.grid);
                let world = self.axes == [Vec3::X, Vec3::Y, Vec3::Z];
                let mut moved = Vec3::ZERO;
                for axis in &self.axes {
                    let along = delta.dot(*axis);
                    if along.abs() < 1e-6 {
                        continue;
                    }
                    let snapped = if world {
                        // Land on the grid lines themselves.
                        let start = self.pivot.dot(*axis);
                        snap(start + along, step) - start
                    } else {
                        snap(along, step)
                    };
                    moved += *axis * snapped;
                }
                Change::Translate(moved)
            }
            Change::Rotate { axis, angle } => Change::Rotate {
                axis,
                angle: snap(angle, s.angle.to_radians()),
            },
            Change::Scale(f) => Change::Scale(Vec3::from_array(f.to_array().map(|v| {
                if v == 1.0 {
                    v
                } else {
                    1.0 + snap(v - 1.0, s.scale)
                }
            }))),
        }
    }
}

/// A change in progress: what it works on, as it was when it began, and
/// the merge key that keeps it one undo step.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Session {
    pub(crate) key: u64,
    pub(crate) target: Target,
}

/// Merge keys for transform changes: kept apart from other merge keys.
const SESSION_KEYS: u64 = 1 << 61;

impl Session {
    /// A change of `target`, numbered `number`.
    pub(crate) fn new(target: Target, number: u64) -> Self {
        Self {
            key: SESSION_KEYS | number,
            target,
        }
    }

    /// Makes the scene show `change` (instead of the change shown before).
    /// Returns the undo step's name, or why it couldn't be done.
    pub(crate) fn show(&self, document: &mut Document, change: &Change) -> Result<String, String> {
        let label = self.target.label(change);
        let commands = if is_nothing(change) {
            Vec::new()
        } else {
            self.target.commands(change)
        };
        self.show_commands(document, commands, &label)?;
        Ok(label)
    }

    /// Makes the scene show what `commands` do (instead of what was shown
    /// before), as one undo step called `label`.
    pub(crate) fn show_commands(
        &self,
        document: &mut Document,
        commands: Vec<Command>,
        label: &str,
    ) -> Result<(), String> {
        document.abandon(self.key);
        for command in commands {
            document
                .execute_labelled(command, label, Some(self.key))
                .map_err(|why| why.to_string())?;
        }
        Ok(())
    }

    /// Puts everything back as it was when the change began.
    pub(crate) fn cancel(&self, document: &mut Document) {
        document.abandon(self.key);
    }
}

/// Whether a change changes nothing.
pub(crate) fn is_nothing(change: &Change) -> bool {
    match change {
        Change::Translate(d) => d.length_squared() < 1e-12,
        Change::Rotate { angle, .. } => angle.abs() < 1e-7,
        Change::Scale(f) => f.abs_diff_eq(Vec3::ONE, 1e-7),
    }
}

/// A change in words for the status bar: "Move 12.5 cm along X", "Rotate
/// 45°", "Scale 150%".
pub(crate) fn describe(change: &Change, target: &Target, units: crate::units::Units) -> String {
    match change {
        Change::Translate(d) => {
            let parts: Vec<String> = ["X", "Y", "Z"]
                .iter()
                .zip(target.axes)
                .map(|(name, axis)| crate::trf!("{} {}", name, units.format(d.dot(axis))))
                .collect();
            if target.slides() {
                crate::trf!("Sliding along the edges (the drag: {})", parts.join(", "))
            } else {
                crate::trf!("Moving: {}", parts.join(", "))
            }
        }
        Change::Rotate { angle, .. } => crate::trf!("Rotating: {:.1}°", angle.to_degrees()),
        Change::Scale(f) => {
            let parts: Vec<String> = f
                .to_array()
                .iter()
                .map(|v| format!("{:.0}%", v * 100.0))
                .collect();
            crate::trf!("Scaling: {}", parts.join(" × "))
        }
    }
}

#[cfg(test)]
mod tests;
