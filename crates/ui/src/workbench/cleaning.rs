//! The workbench's part in clean-up: Delete, Dissolve, Fill hole, Flip
//! normals and Fix facing ([decision 0052](../../../../docs/decisions/0052-clean-up.md)).
//!
//! The controls, chosen by the project lead:
//!
//! - In the Points, Edges and Polygons modes, **Delete** deletes the
//!   selected elements (leaving a hole) and **Backspace** dissolves them
//!   (the surface stays closed). In the Model mode both keys still delete
//!   the selected objects.
//! - **J** fills holes. Flip normals and Fix facing have no keys: they
//!   are in the tool menu (`E`) and the Mesh menu, and Select flipped and
//!   Select similar facing are in the Select menu.
//! - Dissolving across a sharp corner (sharper than 30°) asks first, as
//!   the merged polygon would be bent ([decision 0057](../../../../docs/decisions/0057-after-phase-4-testing.md)).
//!
//! Each is recorded per object as an `Operation` with anchors, one undo
//! step in all.

use super::Workbench;
use super::joining::counted;
use crate::palette::Mode;
use crate::theme;
use egui::{Key, Modifiers, RichText};
use mesh::Anchors;
use scene::{EdgeKey, ElementKind, FaceId, ObjectId, Operation, VertexId};
use std::collections::BTreeSet;

/// The question asked before dissolving across a sharp corner
/// ([decision 0057](../../../../docs/decisions/0057-after-phase-4-testing.md)):
/// what would be dissolved, waiting for an answer.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DissolveQuestion {
    /// The undo step's name: "Dissolve 1 edge".
    label: String,
    /// The operations to record if the answer is yes.
    ops: Vec<(ObjectId, Operation)>,
    /// The "I know what I'm doing, proceed" tickbox, which unlocks the
    /// Dissolve button.
    sure: bool,
    /// The "Don't warn me again" tickbox.
    dont_warn: bool,
}

/// The question window's title.
pub(crate) const DISSOLVE_QUESTION_TITLE: &str = "Dissolve across a sharp corner?";

/// How far apart, in degrees, two polygons' facing may be for Select
/// similar facing to count them alike.
pub(crate) const SIMILAR_FACING_ANGLE: f32 = 30.0;

/// The selected elements of one object in the current mode, with the
/// anchors that find them again.
struct Chosen {
    points: Vec<VertexId>,
    edges: Vec<EdgeKey>,
    faces: Vec<FaceId>,
    anchors: Option<Anchors>,
}

impl Chosen {
    fn len(&self) -> usize {
        self.points.len() + self.edges.len() + self.faces.len()
    }
}

/// The mode's selected elements of `selected` on `mesh`.
fn chosen(mode: Mode, selected: &scene::ElementSelection, mesh: &scene::Mesh) -> Chosen {
    let mut out = Chosen {
        points: Vec::new(),
        edges: Vec::new(),
        faces: Vec::new(),
        anchors: None,
    };
    match mode {
        Mode::Points => {
            out.points = selected.points().collect();
            let places: Vec<Vec<VertexId>> = out.points.iter().map(|v| vec![*v]).collect();
            out.anchors = Anchors::record(mesh, ElementKind::Points, &places);
        }
        Mode::Edges => {
            out.edges = selected.edges().collect();
            let places: Vec<Vec<VertexId>> = out
                .edges
                .iter()
                .map(|e| {
                    let (a, b) = e.ends();
                    vec![a, b]
                })
                .collect();
            out.anchors = Anchors::record(mesh, ElementKind::Edges, &places);
        }
        Mode::Polygons => {
            out.faces = selected.polygons().collect();
            let places: Vec<Vec<VertexId>> = out
                .faces
                .iter()
                .filter_map(|f| mesh.face(*f).map(<[VertexId]>::to_vec))
                .collect();
            out.anchors = Anchors::record(mesh, ElementKind::Polygons, &places);
        }
        _ => {}
    }
    out
}

/// The name of the current mode's elements, for messages.
fn thing(mode: Mode) -> &'static str {
    match mode {
        Mode::Points => "point",
        Mode::Edges => "edge",
        _ => "polygon",
    }
}

impl Workbench {
    /// Delete (in an element mode): deletes the selected points, edges or
    /// polygons, leaving a hole.
    pub(super) fn run_delete_elements(&mut self) {
        self.remove_elements(false);
    }

    /// Backspace (in an element mode): dissolves the selected points,
    /// edges or polygons, keeping the surface closed.
    pub(super) fn run_dissolve(&mut self) {
        self.remove_elements(true);
    }

    fn remove_elements(&mut self, dissolve: bool) {
        let mode = self.mode;
        let verb = if dissolve { "Dissolve" } else { "Delete" };
        let mut ops = Vec::new();
        let mut count = 0;
        let mut refused = 0;
        // Whether dissolving would merge polygons meeting at a sharp
        // corner into one bent polygon.
        let mut sharp = false;
        for (id, selected, mesh) in self.selected_parts() {
            let c = chosen(mode, &selected, &mesh);
            if c.len() == 0 {
                continue;
            }
            if dissolve {
                match dissolved(&mesh, &c) {
                    None => {
                        refused += c.len();
                        continue;
                    }
                    Some(after) => sharp |= bends(&mesh, &after),
                }
            }
            count += c.len();
            let (points, edges, faces, anchors) = (c.points, c.edges, c.faces, c.anchors);
            let operation = if dissolve {
                Operation::Dissolve {
                    points,
                    edges,
                    faces,
                    anchors,
                }
            } else {
                Operation::DeleteElements {
                    points,
                    edges,
                    faces,
                    anchors,
                }
            };
            ops.push((id, operation));
        }
        if ops.is_empty() {
            self.note = Some(if refused > 0 {
                "Dissolve: nothing here can go without opening a hole. Points and edges on \
                 an open border stay, and so do polygons that wouldn't merge into one \
                 (use Delete for those)."
                    .into()
            } else {
                format!("{verb}: select some {}s first.", thing(mode))
            });
            return;
        }
        let label = counted(verb, count, thing(mode));
        if sharp && !self.viewport.options().dissolve_without_warning {
            self.dissolve_question = Some(DissolveQuestion {
                label,
                ops,
                sure: false,
                dont_warn: false,
            });
            self.note = Some(
                "Dissolve: this would merge polygons across a sharp corner. See the question."
                    .into(),
            );
            return;
        }
        if self.record_each(&label, ops) {
            self.note = Some(format!("{label}. Ctrl + Z takes it back."));
        }
    }

    /// Shows the question asked before dissolving across a sharp corner,
    /// if one is waiting, and carries out the answer.
    pub(super) fn dissolve_question_window(&mut self, ctx: &egui::Context) {
        let Some(question) = self.dissolve_question.as_mut() else {
            return;
        };
        let mut answer = None;
        egui::Modal::new(egui::Id::new("dissolve_question")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(
                RichText::new(DISSOLVE_QUESTION_TITLE)
                    .strong()
                    .color(theme::TEXT_STRONG),
            );
            ui.add_space(4.0);
            ui.label(
                "Dissolving here merges polygons that meet at a sharp corner into one bent \
                 polygon. A bent polygon isn't flat, so it shades badly and can look torn or \
                 creased.",
            );
            ui.add_space(4.0);
            ui.label(
                "To take the polygons away and leave a hole, use Delete instead. To keep the \
                 surface but lose a corner, Weld or Bevel are usually what you want.",
            );
            ui.add_space(8.0);
            ui.checkbox(&mut question.sure, "I know what I'm doing, proceed");
            ui.checkbox(&mut question.dont_warn, "Don't warn me again")
                .on_hover_text("You can switch the warning back on in Settings → Interface.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(question.sure, egui::Button::new("Dissolve"))
                    .on_disabled_hover_text("Tick \"I know what I'm doing, proceed\" first.")
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    answer = Some(false);
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Esc: cancel").small().color(theme::TEXT_WEAK));
        });
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            answer = Some(false);
        }
        let Some(answer) = answer else {
            return;
        };
        let Some(question) = self.dissolve_question.take() else {
            return;
        };
        if !answer {
            self.note = Some("Dissolve cancelled: nothing changed.".into());
            return;
        }
        if question.dont_warn {
            self.viewport.options_mut().dissolve_without_warning = true;
        }
        if self.record_each(&question.label, question.ops) {
            self.note = Some(format!("{}. Ctrl + Z takes it back.", question.label));
        }
    }

    /// Fill hole: closes the holes touching the selection, or every hole
    /// of the selected objects when nothing on them is selected.
    pub(super) fn run_fill_hole(&mut self) {
        let mode = self.mode;
        let mut ops = Vec::new();
        let mut missed = false;
        for (id, selected, mesh) in self.selected_parts() {
            if mesh.topology().border_edges().is_empty() {
                continue;
            }
            let c = chosen(mode, &selected, &mesh);
            let points: BTreeSet<VertexId> = c
                .points
                .iter()
                .copied()
                .chain(c.edges.iter().flat_map(|e| {
                    let (a, b) = e.ends();
                    [a, b]
                }))
                .chain(
                    c.faces
                        .iter()
                        .filter_map(|f| mesh.face(*f))
                        .flat_map(|corners| corners.iter().copied()),
                )
                .collect();
            // Tried on a copy, so no empty undo step is recorded. With
            // nothing selected, every hole is filled.
            let mut trial = (*mesh).clone();
            let wanted: Vec<VertexId> = points.iter().copied().collect();
            if mesh::edit::fill_holes(&mut trial, &wanted).matched == 0 {
                missed = true;
                continue;
            }
            let points: Vec<VertexId> = points.into_iter().collect();
            let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
            let anchors = Anchors::record(&mesh, ElementKind::Points, &places);
            ops.push((id, Operation::FillHoles { points, anchors }));
        }
        if ops.is_empty() {
            self.note = Some(if missed {
                "Fill hole: no hole touches the selection. Select an edge on its border, or \
                 select nothing to fill every hole."
                    .into()
            } else {
                "Fill hole: the selected objects have no holes.".into()
            });
            return;
        }
        let label = if ops.iter().all(|(_, op)| match op {
            Operation::FillHoles { points, .. } => points.is_empty(),
            _ => false,
        }) {
            "Fill every hole"
        } else {
            "Fill hole"
        };
        if self.record_each(label, ops) {
            self.note = Some(format!("{label}. Ctrl + Z takes it back."));
        }
    }

    /// Flip normals: turns the selected polygons inside out.
    pub(super) fn run_flip(&mut self) {
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, selected, mesh) in self.selected_parts() {
            let c = chosen(Mode::Polygons, &selected, &mesh);
            if c.faces.is_empty() {
                continue;
            }
            count += c.faces.len();
            ops.push((
                id,
                Operation::FlipPolygons {
                    faces: c.faces,
                    anchors: c.anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some("Flip normals: select some polygons first.".into());
            return;
        }
        let label = counted("Flip", count, "polygon");
        if self.record_each(&label, ops) {
            self.note = Some(format!("{label}. Ctrl + Z takes it back."));
        }
    }

    /// Fix facing: turns every polygon of the selected objects that faces
    /// the wrong way round.
    pub(super) fn run_fix_facing(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some("Fix facing: select the objects to fix first.".into());
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, _, mesh) in self.selected_parts() {
            let wrong = mesh::cleanup::faces_facing_wrong(&mesh).len();
            if wrong > 0 {
                count += wrong;
                ops.push((id, Operation::FixFacing));
            }
        }
        if ops.is_empty() {
            self.note = Some("Fix facing: every polygon already faces the right way.".into());
            return;
        }
        if self.record_each("Fix facing", ops) {
            self.note = Some(format!(
                "Turned {} the right way round. Ctrl + Z takes it back.",
                counted("", count, "polygon").trim_start()
            ));
        }
    }
}

/// `mesh` with `c` dissolved (tried on a copy), or `None` when nothing
/// would change, so a refused dissolve isn't recorded as an empty undo
/// step.
fn dissolved(mesh: &scene::Mesh, c: &Chosen) -> Option<scene::Mesh> {
    let mut trial = mesh.clone();
    let outcome = if !c.faces.is_empty() {
        mesh::edit::dissolve_faces(&mut trial, &c.faces)
    } else if !c.edges.is_empty() {
        mesh::edit::dissolve_edges(&mut trial, &c.edges)
    } else {
        mesh::edit::dissolve_points(&mut trial, &c.points)
    };
    (outcome.matched > 0).then_some(trial)
}

/// Whether going from `before` to `after` took away an edge whose two
/// polygons met more sharply than the Hard edges by angle start (30°):
/// the merged polygon is then bent, not flat.
pub(crate) fn bends(before: &scene::Mesh, after: &scene::Mesh) -> bool {
    let cos = mesh::hardness::DEFAULT_ANGLE.to_radians().cos();
    let (old, new) = (before.topology(), after.topology());
    old.edges().filter(|e| !new.has_edge(*e)).any(|e| {
        let [one, two] = old.edge_uses(e) else {
            return false;
        };
        let (n1, n2) = (before.face_normal(one.face), before.face_normal(two.face));
        n1.length_squared() > 0.5 && n2.length_squared() > 0.5 && n1.dot(n2) < cos - 1e-6
    })
}
