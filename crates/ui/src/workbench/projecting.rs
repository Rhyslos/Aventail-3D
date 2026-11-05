//! Projections ([decision 0235](../../../../docs/decisions/0235-projections.md)):
//! UV → Project (and the E ring's Unwrap group) lays the selection's UVs
//! on by a flat, box, cylinder or ball shape, placed round the selection
//! and then moved, turned and sized with the usual gizmo (W, R, S, T),
//! the UVs following live; or from the view; or world-aligned for kits.
//!
//! While a projection is placed, it is recorded at once and replaced in
//! the same undo step each time the gizmo changes it. **Enter** or
//! **Done** keeps it; **Esc** or **Cancel** takes it back (the project
//! lead's choice for projections, phase 8 plan). Its shape is drawn over
//! the model in the viewport.

use super::Workbench;
use super::joining::counted;
use super::unwrapping::targets;
use crate::palette::Mode;
use crate::text::tr;
use crate::transforming::Tool;
use crate::viewport::GizmoEvent;
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Quat, Vec3};
use scene::{FaceId, ObjectId, Operation, ProjectOptions, Projection};
use tools::{Change, Gizmo, GizmoTool};

/// The Project submenu's name, and what it holds.
pub(crate) const PROJECT_LABEL: &str = crate::text!("Project");
pub(crate) const PROJECT_HELP: &str = crate::text!(
    "Lay the selection's UVs on from a shape placed round it: flat, a box, a cylinder or a ball, \
     then move, turn and size it with W, R and S. Enter or Done keeps it; Esc cancels."
);
/// Each projection's name and what it suits.
pub(crate) const PLANAR_LABEL: &str = crate::text!("Flat");
pub(crate) const PLANAR_HELP: &str = crate::text!(
    "Project straight on from one side, like a slide projector: for flat or nearly flat parts \
     (a sign, a floor, a decal)."
);
pub(crate) const BOX_LABEL: &str = crate::text!("Box");
pub(crate) const BOX_HELP: &str = crate::text!(
    "Project each polygon from the box side it faces most: for boxy parts (crates, walls, \
     blocks). Seams go where the sides meet."
);
pub(crate) const CYLINDER_LABEL: &str = crate::text!("Cylinder");
pub(crate) const CYLINDER_HELP: &str = crate::text!(
    "Wrap the texture once round, like a label on a can: for pipes, posts, barrels. One seam \
     runs down the back."
);
pub(crate) const SPHERE_LABEL: &str = crate::text!("Ball");
pub(crate) const SPHERE_HELP: &str = crate::text!(
    "Wrap the texture round and from bottom to top, like a globe: for round parts. Squeezed at \
     the top and bottom."
);
pub(crate) const FROM_VIEW_LABEL: &str = crate::text!("Flat, from the view");
pub(crate) const FROM_VIEW_HELP: &str = crate::text!(
    "Project flat straight from where you're looking: the texture lies on the model as you see \
     it now."
);
pub(crate) const WORLD_LABEL: &str = crate::text!("World-aligned");
pub(crate) const WORLD_HELP: &str = crate::text!(
    "Box-project from the world's axes at the project's texel density, so modular kit pieces \
     placed side by side get a texture that runs on across the joins. Recorded at once."
);
/// The buttons shown while a projection is placed.
pub(crate) const DONE_LABEL: &str = crate::text!("Done");
pub(crate) const CANCEL_LABEL: &str = crate::text!("Cancel");

/// A projection being placed.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ProjectSession {
    kind: Projection,
    /// Each object, its shape's place in the world, and its polygons.
    parts: Vec<(ObjectId, Mat4, Vec<FaceId>)>,
    /// The projection's place in the world.
    place: Place,
    /// Its place as a gizmo drag began.
    start: Option<Place>,
    /// The undo step it is recorded in.
    key: u64,
    /// The tool to go back to.
    before: Tool,
}

/// Where a projection sits: the middle of its box, its axes (unit
/// length, at right angles), and its half size along each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Place {
    origin: Vec3,
    axes: [Vec3; 3],
    half: Vec3,
}

impl Place {
    /// From the world to the projection's own space (the box from −1 to
    /// 1).
    fn from_world(&self) -> Mat4 {
        Mat4::from_cols(
            (self.axes[0] * self.half.x).extend(0.0),
            (self.axes[1] * self.half.y).extend(0.0),
            (self.axes[2] * self.half.z).extend(0.0),
            self.origin.extend(1.0),
        )
        .inverse()
    }

    /// The place after the gizmo's `change` from here.
    fn changed(&self, change: &Change) -> Self {
        match *change {
            Change::Translate(by) => Self {
                origin: self.origin + by,
                ..*self
            },
            Change::Rotate { axis, angle } => {
                let turn = Quat::from_axis_angle(axis, angle);
                Self {
                    axes: self.axes.map(|a| (turn * a).normalize_or_zero()),
                    ..*self
                }
            }
            Change::Scale(by) => Self {
                half: (self.half * by.abs()).max(Vec3::splat(MIN_HALF)),
                ..*self
            },
        }
    }

    /// Lines drawing the projection's shape in the world.
    fn lines(&self, kind: Projection) -> Vec<[Vec3; 2]> {
        let at = |x: f32, y: f32, z: f32| {
            self.origin
                + self.axes[0] * (x * self.half.x)
                + self.axes[1] * (y * self.half.y)
                + self.axes[2] * (z * self.half.z)
        };
        let mut out = Vec::new();
        let circle = |out: &mut Vec<[Vec3; 2]>, z: f32, r: f32| {
            let n = 32;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let b = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
                out.push([
                    at(r * a.sin(), -r * a.cos(), z),
                    at(r * b.sin(), -r * b.cos(), z),
                ]);
            }
        };
        match kind {
            Projection::Planar => {
                let c = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
                for i in 0..4 {
                    let (a, b) = (c[i], c[(i + 1) % 4]);
                    out.push([at(a.0, a.1, 0.0), at(b.0, b.1, 0.0)]);
                }
                // Which way it looks, and which way is up in the texture.
                out.push([at(0.0, 0.0, 0.0), at(0.0, 0.0, 0.5)]);
                out.push([at(0.0, 1.0, 0.0), at(0.0, 1.2, 0.0)]);
            }
            Projection::Box => {
                for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    out.push([at(x, y, -1.0), at(x, y, 1.0)]);
                }
                for z in [-1.0, 1.0] {
                    let c = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
                    for i in 0..4 {
                        let (a, b) = (c[i], c[(i + 1) % 4]);
                        out.push([at(a.0, a.1, z), at(b.0, b.1, z)]);
                    }
                }
            }
            Projection::Cylinder => {
                circle(&mut out, -1.0, 1.0);
                circle(&mut out, 1.0, 1.0);
                // The seam line at the back, and three more down the side.
                for (x, y) in [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)] {
                    out.push([at(x, y, -1.0), at(x, y, 1.0)]);
                }
            }
            Projection::Sphere => {
                circle(&mut out, 0.0, 1.0);
                // Two upright rings, through the back (the seam) and the
                // sides.
                let n = 32;
                for side in [0.0f32, std::f32::consts::FRAC_PI_2] {
                    for i in 0..n {
                        let a = i as f32 / n as f32 * std::f32::consts::TAU;
                        let b = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
                        let p = |t: f32| at(t.cos() * side.sin(), -t.cos() * side.cos(), t.sin());
                        out.push([p(a), p(b)]);
                    }
                }
            }
        }
        out
    }
}

/// The smallest half size a projection can be sized down to.
const MIN_HALF: f32 = 1e-3;

impl Workbench {
    /// UV → Project's entries (and the E ring's Project).
    pub(super) fn project_entries(&mut self, ui: &mut Ui) {
        for (label, help, kind) in [
            (tr(PLANAR_LABEL), tr(PLANAR_HELP), Projection::Planar),
            (tr(BOX_LABEL), tr(BOX_HELP), Projection::Box),
            (tr(CYLINDER_LABEL), tr(CYLINDER_HELP), Projection::Cylinder),
            (tr(SPHERE_LABEL), tr(SPHERE_HELP), Projection::Sphere),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                self.start_projection(kind, false);
                ui.close();
            }
        }
        ui.separator();
        if ui
            .button(tr(FROM_VIEW_LABEL))
            .on_hover_text(tr(FROM_VIEW_HELP))
            .clicked()
        {
            self.start_projection(Projection::Planar, true);
            ui.close();
        }
        if ui
            .button(tr(WORLD_LABEL))
            .on_hover_text(tr(WORLD_HELP))
            .clicked()
        {
            self.run_world_aligned();
            ui.close();
        }
    }

    /// The selected objects' polygons to project: the selection's in an
    /// element mode, otherwise every polygon; with their shapes' places.
    fn projection_parts(&self) -> Vec<(ObjectId, Mat4, std::sync::Arc<scene::Mesh>, Vec<FaceId>)> {
        let scene = self.document.scene();
        self.own_shapes()
            .into_iter()
            .map(|(id, mesh)| {
                let faces = targets(&mesh, self.document.elements(id), self.mode);
                (id, scene.shape_matrix(id), mesh, faces)
            })
            .filter(|(_, _, _, faces)| !faces.is_empty())
            .collect()
    }

    /// Whether a projection is being placed.
    pub(crate) fn projecting(&self) -> bool {
        self.project_session.is_some()
    }

    /// Places a `kind` projection round the selection (`from_view`: flat,
    /// looking the way the view does) and records it, ready for the
    /// gizmo.
    pub(crate) fn start_projection(&mut self, kind: Projection, from_view: bool) {
        self.finish_projection();
        let parts = self.projection_parts();
        if parts.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects (or polygons) to project onto first.",
                tr(PROJECT_LABEL)
            ));
            return;
        }
        // Where the polygons are in the world.
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        let axes = if from_view {
            let camera = self.viewport.camera();
            [camera.right(), camera.up(), -camera.forward()]
        } else {
            [Vec3::X, Vec3::Y, Vec3::Z]
        };
        for (_, matrix, mesh, faces) in &parts {
            for f in faces {
                for v in mesh.face(*f).unwrap_or(&[]) {
                    let p = matrix.transform_point3(mesh.position(*v).unwrap_or(Vec3::ZERO));
                    let local = Vec3::new(p.dot(axes[0]), p.dot(axes[1]), p.dot(axes[2]));
                    lo = lo.min(local);
                    hi = hi.max(local);
                }
            }
        }
        if !lo.is_finite() || !hi.is_finite() {
            return;
        }
        let middle = (lo + hi) / 2.0;
        let size = (hi - lo) / 2.0;
        let mut axes = axes;
        // A flat projection (not from the view) looks along the
        // selection's thinnest side, its square across the other two.
        if kind == Projection::Planar && !from_view {
            axes = match size.min_position() {
                0 => [Vec3::Y, Vec3::Z, Vec3::X],
                1 => [Vec3::X, Vec3::Z, -Vec3::Y],
                _ => [Vec3::X, Vec3::Y, Vec3::Z],
            };
        }
        let world_middle = if from_view {
            let camera = self.viewport.camera();
            camera.right() * middle.x + camera.up() * middle.y - camera.forward() * middle.z
        } else {
            middle
        };
        // The texture stays square: one size on the square's two axes
        // (and the box's three, the round shapes' radius), a little
        // bigger than the selection.
        let along = |a: Vec3| Vec3::new(a.dot(Vec3::X), a.dot(Vec3::Y), a.dot(Vec3::Z));
        let sized = |a: Vec3| {
            let d = along(a);
            (d.x * size.x).abs().max((d.y * size.y).abs()).max((d.z * size.z).abs())
        };
        let half = if from_view {
            let s = size.x.max(size.y).max(MIN_HALF) * 1.02;
            Vec3::new(s, s, size.z.max(MIN_HALF) * 1.02)
        } else {
            let across = sized(axes[0]).max(sized(axes[1])).max(MIN_HALF) * 1.02;
            let deep = sized(axes[2]).max(MIN_HALF) * 1.02;
            match kind {
                Projection::Box | Projection::Sphere => {
                    let s = across.max(deep);
                    Vec3::splat(s)
                }
                _ => Vec3::new(across, across, deep),
            }
        };
        let before = self.tool;
        let key = self.new_step_key();
        self.project_session = Some(ProjectSession {
            kind,
            parts: parts
                .into_iter()
                .map(|(id, matrix, _, faces)| (id, matrix, faces))
                .collect(),
            place: Place {
                origin: world_middle,
                axes,
                half,
            },
            start: None,
            key,
            before,
        });
        if self.tool.gizmo().is_none() {
            self.tool = Tool::Move;
        }
        if self.record_projection() {
            self.note = Some(tr(
                "Projecting: move, turn and size the projection with W, R and S (or T) and watch \
                 the UVs follow. Enter or Done keeps it; Esc or Cancel takes it back.",
            )
            .into());
        }
    }

    /// Records the projection where it is now, replacing what it
    /// recorded before in the same undo step.
    fn record_projection(&mut self) -> bool {
        let Some(session) = &self.project_session else {
            return false;
        };
        let to = session.place.from_world();
        let kind = session.kind;
        let key = session.key;
        let count: usize = session.parts.iter().map(|(_, _, f)| f.len()).sum();
        let ops: Vec<(ObjectId, Operation)> = session
            .parts
            .iter()
            .filter_map(|(id, matrix, faces)| {
                let mesh = self.document.scene().evaluated(*id)?.mesh;
                Some((
                    *id,
                    Operation::ProjectUvs {
                        faces: faces.clone(),
                        options: ProjectOptions {
                            kind,
                            placement: to * *matrix,
                            world: None,
                        },
                        anchors: super::unwrapping::anchors(&mesh, faces),
                    },
                ))
            })
            .collect();
        self.document.abandon(key);
        let label = counted(tr("Project UVs onto"), count, tr("polygon"));
        self.record_each_in(&label, ops, key)
    }

    /// Keeps the projection (Enter, Done).
    pub(crate) fn finish_projection(&mut self) {
        if let Some(session) = self.project_session.take() {
            self.tool = session.before;
            self.components.set_tool_lines(Vec::new());
            self.note = Some(tr("Projection kept. Ctrl + Z takes it back.").into());
        }
    }

    /// Takes the projection back (Esc, Cancel).
    pub(crate) fn cancel_projection(&mut self) {
        if let Some(session) = self.project_session.take() {
            self.document.abandon(session.key);
            self.tool = session.before;
            self.components.set_tool_lines(Vec::new());
            self.note = Some(tr("Projection cancelled: the UVs are as they were.").into());
        }
    }

    /// While projecting: Enter keeps it, Esc cancels it. Leaving the
    /// mode it was started in, or undoing it, keeps what is recorded.
    /// Returns whether a key was taken.
    pub(super) fn projecting_keys(&mut self, ui: &Ui) -> bool {
        if self.project_session.is_none() || ui.ctx().memory(|m| m.focused().is_some()) {
            return false;
        }
        // Undone from outside (Ctrl + Z): nothing left to place.
        let key = self.project_session.as_ref().map(|s| s.key);
        if self.document.last_merge_key() != key {
            self.project_session = None;
            self.components.set_tool_lines(Vec::new());
            return false;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            self.finish_projection();
            return true;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.cancel_projection();
            return true;
        }
        false
    }

    /// The gizmo on the projection, while placing one (`None`: not
    /// projecting).
    pub(super) fn projection_gizmo(&self) -> Option<Gizmo> {
        let session = self.project_session.as_ref()?;
        Some(Gizmo {
            origin: session.place.origin,
            axes: session.place.axes,
            tool: self.tool.gizmo().unwrap_or(GizmoTool::Move),
        })
    }

    /// Shows the projection's shape over the model.
    pub(super) fn show_projection(&mut self) {
        if let Some(session) = &self.project_session {
            let lines = session.place.lines(session.kind);
            self.components.set_tool_lines(lines);
        }
    }

    /// Carries out the gizmo's events on the projection. Returns whether
    /// it took them (while projecting).
    pub(super) fn projection_gizmo_events(&mut self, events: &[GizmoEvent]) -> bool {
        if self.project_session.is_none() {
            return false;
        }
        for event in events {
            let Some(session) = &mut self.project_session else {
                break;
            };
            match event {
                GizmoEvent::Begin { .. } => session.start = Some(session.place),
                GizmoEvent::Update { change, .. } => {
                    let start = *session.start.get_or_insert(session.place);
                    let next = start.changed(change);
                    if next != session.place {
                        session.place = next;
                        self.record_projection();
                    }
                }
                GizmoEvent::End => session.start = None,
                GizmoEvent::Cancel => {
                    if let Some(start) = session.start.take() {
                        session.place = start;
                        self.record_projection();
                    }
                }
            }
        }
        true
    }

    /// World-aligned: box-projects the selection from the world's axes at
    /// the project's texel density, as one undo step.
    pub(crate) fn run_world_aligned(&mut self) {
        self.finish_projection();
        let parts = self.projection_parts();
        if parts.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects (or polygons) first.",
                tr(WORLD_LABEL)
            ));
            return;
        }
        let scale = self.document.scene().texture().uv_per_unit();
        let mut count = 0;
        let ops: Vec<(ObjectId, Operation)> = parts
            .into_iter()
            .map(|(id, matrix, mesh, faces)| {
                count += faces.len();
                let anchors = super::unwrapping::anchors(&mesh, &faces);
                (
                    id,
                    Operation::ProjectUvs {
                        faces,
                        options: ProjectOptions {
                            kind: Projection::Box,
                            placement: matrix,
                            world: Some(scale),
                        },
                        anchors,
                    },
                )
            })
            .collect();
        let label = counted(tr("World-align the UVs of"), count, tr("polygon"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!(
                "{} at {:.0} px/m. Ctrl + Z takes it back. Moving the objects later doesn't move \
                 the texture: world-align again after placing them.",
                label,
                self.document.scene().texture().pixels_per_metre(scale)
            ));
        }
    }

    /// The Done and Cancel buttons over the viewport while projecting.
    pub(super) fn projection_buttons(&mut self, ui: &Ui) {
        if self.project_session.is_none() || self.mode == Mode::Paint {
            return;
        }
        let rect = ui.ctx().content_rect();
        egui::Area::new(egui::Id::new("projection-buttons"))
            .fixed_pos(egui::pos2(rect.center().x - 80.0, rect.min.y + 64.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr(PROJECT_LABEL));
                        if ui.button(tr(DONE_LABEL)).clicked() {
                            self.finish_projection();
                        }
                        if ui.button(tr(CANCEL_LABEL)).clicked() {
                            self.cancel_projection();
                        }
                    });
                });
            });
    }
}
