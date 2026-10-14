//! The workbench's part in clean-up: Delete, Dissolve, Fill hole, Flip
//! normals and Fix facing ([decision 0052](../../../../docs/decisions/0052-clean-up.md)).
//!
//! The controls, chosen by the project lead:
//!
//! - In the Points, Edges and Polygons modes, **Delete** deletes the
//!   selected elements (leaving a hole) and **Backspace** dissolves them
//!   (the surface stays closed). In the Model mode both keys still delete
//!   the selected objects.
//! - Fill hole, Flip normals and Fix facing have no keys: they are in the
//!   tool menu (`E`) and the Mesh menu, and Select flipped and Select
//!   similar facing are in the Select menu.
//!
//! Each is recorded per object as an `Operation` with anchors, one undo
//! step in all.

use super::Workbench;
use super::joining::counted;
use crate::palette::Mode;
use mesh::Anchors;
use scene::{EdgeKey, ElementKind, FaceId, Operation, VertexId};
use std::collections::BTreeSet;

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
        for (id, selected, mesh) in self.selected_parts() {
            let c = chosen(mode, &selected, &mesh);
            if c.len() == 0 {
                continue;
            }
            if dissolve && !dissolves(&mesh, &c) {
                refused += c.len();
                continue;
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
        if self.record_each(&label, ops) {
            self.note = Some(format!("{label}. Ctrl + Z takes it back."));
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

/// Whether dissolving `c` on `mesh` would change anything (tried on a
/// copy), so a refused dissolve isn't recorded as an empty undo step.
fn dissolves(mesh: &scene::Mesh, c: &Chosen) -> bool {
    let mut trial = mesh.clone();
    let outcome = if !c.faces.is_empty() {
        mesh::edit::dissolve_faces(&mut trial, &c.faces)
    } else if !c.edges.is_empty() {
        mesh::edit::dissolve_edges(&mut trial, &c.edges)
    } else {
        mesh::edit::dissolve_points(&mut trial, &c.points)
    };
    outcome.matched > 0
}
