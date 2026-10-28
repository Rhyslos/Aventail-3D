//! The workbench's part in seams ([decision 0098](../../../../docs/decisions/0098-seams.md)):
//! marking and clearing them on the selection, seams from hard edges,
//! suggested seams and selecting them.
//!
//! No keys (the project lead chose menus, decision 0095): Mark seams and
//! Clear seams are in the UV ring of the tool menu (`E`) and the UV menu;
//! Seams from hard edges, Suggest seams and Select seams in the UV menu
//! (Select seams in the Select menu too). Painting seams is the Paint
//! select brush on edges, then Mark seams.

use super::Workbench;
use super::joining::counted;
use crate::palette::Mode;
use crate::text::tr;
use mesh::Anchors;
use scene::{EdgeKey, ElementKind, FaceId, Operation, VertexId};
use std::collections::BTreeSet;

/// The UV menu's name for Seams from hard edges, and what it does.
pub(crate) const SEAMS_FROM_HARD_LABEL: &str = crate::text!("Seams from hard edges");
pub(crate) const SEAMS_FROM_HARD_HELP: &str = crate::text!(
    "Mark every hard edge of the selected objects as a seam \
     (as shown: by angle while Automatic hard edges is on). A normal-map bake needs the UVs \
     split wherever the shading breaks."
);
/// The UV menu's name for Suggest seams, and what it does.
pub(crate) const SUGGEST_SEAMS_LABEL: &str = crate::text!("Suggest seams");
pub(crate) const SUGGEST_SEAMS_HELP: &str = crate::text!(
    "Replace the selected objects' seams with suggested \
     ones: along hard edges and sharp corners, then where each piece needs cutting to lie flat, \
     in hidden places (facing down, in corners)"
);
/// The name for Select seams, and what it does.
pub(crate) const SELECT_SEAMS_LABEL: &str = crate::text!("Select seams");
pub(crate) const SELECT_SEAMS_HELP: &str =
    crate::text!("Select every seam of the edited objects. Shows the Edges mode.");

/// The edges marked (Polygons and UV modes): the border of the selected
/// polygons, where a selected one meets one that isn't (or nothing).
pub(super) fn border_of(mesh: &scene::Mesh, faces: &BTreeSet<FaceId>) -> BTreeSet<EdgeKey> {
    let topology = mesh.topology();
    faces
        .iter()
        .flat_map(|f| topology.face_edges(*f).iter().copied())
        .filter(|e| topology.edge_faces(*e).any(|g| !faces.contains(&g)))
        .collect()
}

pub(super) fn ends(edges: &[EdgeKey]) -> Vec<Vec<VertexId>> {
    edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            vec![a, b]
        })
        .collect()
}

/// The operation marking (or clearing) `edges` on `mesh`.
pub(super) fn seams_op(mesh: &scene::Mesh, edges: Vec<EdgeKey>, seam: bool) -> Operation {
    let places = ends(&edges);
    Operation::SetSeams {
        edges,
        seam,
        anchors: Anchors::record(mesh, ElementKind::Edges, &places),
    }
}

/// Only the edges that would change: inner edges not already so.
fn changing(
    mesh: &scene::Mesh,
    edges: impl IntoIterator<Item = EdgeKey>,
    seam: bool,
) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    edges
        .into_iter()
        .filter(|e| topology.edge_uses(*e).len() == 2 && mesh::uvs::is_seam(mesh, *e) != seam)
        .collect()
}

impl Workbench {
    /// Mark seams (`seam`) or Clear seams: on the selected edges, the
    /// edges between selected points, or, for polygons, the border of the
    /// selection (marking) or every edge of it (clearing).
    pub(super) fn run_seams(&mut self, seam: bool) {
        let name = if seam {
            tr("Mark seams")
        } else {
            tr("Clear seams")
        };
        let mut ops = Vec::new();
        let mut count = 0;
        let mut any = false;
        for (id, selected, mesh) in self.selected_parts() {
            let topology = mesh.topology();
            let edges: BTreeSet<EdgeKey> = match self.mode {
                Mode::Edges => selected.edges().collect(),
                Mode::Points => {
                    let points: BTreeSet<VertexId> = selected.points().collect();
                    topology
                        .edges()
                        .filter(|e| {
                            let (a, b) = e.ends();
                            points.contains(&a) && points.contains(&b)
                        })
                        .collect()
                }
                Mode::Polygons | Mode::Uv => {
                    let faces: BTreeSet<FaceId> = selected.polygons().collect();
                    if seam {
                        border_of(&mesh, &faces)
                    } else {
                        faces
                            .iter()
                            .flat_map(|f| topology.face_edges(*f).iter().copied())
                            .collect()
                    }
                }
                Mode::Model | Mode::Object | Mode::Paint => BTreeSet::new(),
            };
            any |= !edges.is_empty();
            let edges = changing(&mesh, edges, seam);
            if edges.is_empty() {
                continue;
            }
            count += edges.len();
            let live = self.live_unwrap(&mesh, &edges);
            ops.push((id, seams_op(&mesh, edges, seam)));
            ops.extend(live.map(|op| (id, op)));
        }
        if ops.is_empty() {
            self.note = Some(if any {
                if seam {
                    crate::trf!(
                        "{}: those edges are seams already (or open edges, cut already).",
                        name
                    )
                } else {
                    crate::trf!("{}: those edges aren't seams.", name)
                }
            } else {
                crate::trf!(
                    "{}: select some edges first (or polygons, for the edges around them).",
                    name
                )
            });
            return;
        }
        let verb = if seam {
            tr("Mark as seams")
        } else {
            tr("Clear the seams of")
        };
        let label = counted(verb, count, tr("edge"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!(
                "{}. Ctrl + Z takes it back.{}",
                label,
                self.seams_hint()
            ));
        }
    }

    /// Where to see seams, when they aren't shown in the viewport now,
    /// and whether the UVs followed.
    fn seams_hint(&self) -> &'static str {
        match (
            self.mode == Mode::Uv || self.viewport.options().show_seams,
            self.uv_settings.live_unwrap,
        ) {
            (true, true) => tr(" The pieces around them are unwrapped again."),
            (true, false) => tr(" Unwrap to lay the pieces flat along them."),
            (false, true) => tr(
                " The pieces around them are unwrapped again. Seams show in orange in the UV \
                 Editor and the UV mode, or with Display → Seams.",
            ),
            (false, false) => tr(
                " Seams show in orange in the UV Editor and the UV mode, or with Display → \
                 Seams.",
            ),
        }
    }

    /// The edges of object `id`'s `mesh` shading hard, as shown: by its
    /// own smoothing (decision 0126; an instance's original's).
    pub(super) fn shown_hard_edges(&self, id: scene::ObjectId, mesh: &scene::Mesh) -> Vec<EdgeKey> {
        mesh::hardness::shading_hard_edges(mesh, self.smoothing_of(id))
    }

    /// Object `id`'s smoothing (an instance's is its original's).
    pub(crate) fn smoothing_of(&self, id: scene::ObjectId) -> mesh::Hardness {
        let scene = self.document.scene();
        let own = match scene.object(id).map(|o| &o.kind) {
            Some(scene::ObjectKind::Instance(source)) => *source,
            _ => id,
        };
        scene
            .object(own)
            .map_or(mesh::Hardness::Marked, |o| o.hardness)
    }

    /// Seams from hard edges: marks the selected objects' hard edges as
    /// seams.
    pub(crate) fn run_seams_from_hard_edges(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects first.",
                tr(SEAMS_FROM_HARD_LABEL)
            ));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh) in self.own_shapes() {
            // Never on an edge marked "never" (decision 0111).
            let hard = self
                .shown_hard_edges(id, &mesh)
                .into_iter()
                .filter(|e| mesh::uvs::seam_preference(&mesh, *e) != scene::SeamPreference::Never);
            let edges = changing(&mesh, hard, true);
            if edges.is_empty() {
                continue;
            }
            count += edges.len();
            let live = self.live_unwrap(&mesh, &edges);
            ops.push((id, seams_op(&mesh, edges, true)));
            ops.extend(live.map(|op| (id, op)));
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: every hard edge is a seam already.",
                tr(SEAMS_FROM_HARD_LABEL)
            ));
            return;
        }
        let label = counted(tr("Mark as seams"), count, tr("hard edge"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!(
                "{}. Ctrl + Z takes it back.{}",
                label,
                self.seams_hint()
            ));
        }
    }

    /// Suggest seams: replaces the selected objects' seams with
    /// `uv::seams::suggest`'s, as one undo step.
    pub(crate) fn run_suggest_seams(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects first.",
                tr(SUGGEST_SEAMS_LABEL)
            ));
            return;
        }
        let mut ops = Vec::new();
        let mut total = 0;
        let mut changed = 0;
        for (id, mesh) in self.own_shapes() {
            let hard = self.shown_hard_edges(id, &mesh);
            let suggested: BTreeSet<EdgeKey> =
                uv::seams::suggest_with(&mesh, &hard, self.suggest_options())
                    .into_iter()
                    .collect();
            total += suggested.len();
            let now: BTreeSet<EdgeKey> = mesh::uvs::seams(&mesh).into_iter().collect();
            // Edges of locked pieces stay as they are (decision 0112).
            let suggested: BTreeSet<EdgeKey> = match mesh::uvs::locked(&mesh) {
                Some(lock) => {
                    let topology = mesh.topology();
                    let touches_lock = |e: &EdgeKey| {
                        topology
                            .edge_faces(*e)
                            .any(|f| lock.get(f.get() as usize).copied().unwrap_or(false))
                    };
                    suggested
                        .into_iter()
                        .filter(|e| !touches_lock(e))
                        .chain(now.iter().copied().filter(|e| touches_lock(e)))
                        .collect()
                }
                None => suggested,
            };
            let clear: Vec<EdgeKey> = changing(&mesh, now.difference(&suggested).copied(), false);
            let mark: Vec<EdgeKey> = changing(&mesh, suggested.iter().copied(), true);
            changed += clear.len() + mark.len();
            let both: Vec<EdgeKey> = clear.iter().chain(&mark).copied().collect();
            let live = self.live_unwrap(&mesh, &both);
            if !clear.is_empty() {
                ops.push((id, seams_op(&mesh, clear, false)));
            }
            if !mark.is_empty() {
                ops.push((id, seams_op(&mesh, mark, true)));
            }
            ops.extend(live.map(|op| (id, op)));
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: the seams are already where they'd be suggested.",
                tr(SUGGEST_SEAMS_LABEL)
            ));
            return;
        }
        if self.record_each(tr(SUGGEST_SEAMS_LABEL), ops) {
            self.note = Some(crate::trf!(
                "{} suggested ({} changed). Ctrl + Z takes it back.{}",
                counted("", total, tr("seam")).trim_start(),
                changed,
                self.seams_hint()
            ));
        }
    }

    /// Select seams, from the menus: shows the Edges mode and selects
    /// them.
    pub(crate) fn run_select_seams(&mut self) {
        self.mode = Mode::Edges;
        self.select_requests
            .push(super::selection::SelectRequest::Seams);
    }
}
