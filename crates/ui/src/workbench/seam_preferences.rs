//! The workbench's part in seam preferences and stretch budgets
//! ([decision 0111](../../../../docs/decisions/0111-seam-preferences-and-stretch-budget.md)).
//!
//! No keys: the UV menu holds Prefer seams here, Never seams here and
//! Clear seam preference (on the selected edges, the edges between
//! selected points, or the selected polygons' edges), and the Stretch
//! budget submenu (for the pieces holding the selection). The budget for
//! every other piece is in UV → Seam suggestions.

use super::Workbench;
use super::joining::counted;
use super::seams::{border_of, ends};
use crate::palette::Mode;
use crate::text::tr;
use mesh::Anchors;
use scene::{EdgeKey, ElementKind, FaceId, Operation, SeamPreference, VertexId};
use std::collections::BTreeSet;

/// The UV menu's names for the three preference commands, and what they
/// do.
pub(crate) const PREFER_LABEL: &str = crate::text!("Prefer seams here");
pub(crate) const PREFER_HELP: &str = crate::text!(
    "When Suggest seams or Unwrap needs a cut, put it on the \
     selected edges first (or around the selected polygons). Drawn dashed yellow."
);
pub(crate) const NEVER_LABEL: &str = crate::text!("Never seams here");
pub(crate) const NEVER_HELP: &str = crate::text!(
    "Suggest seams, Unwrap and Seams from hard edges never put a \
     seam on the selected edges (or anywhere on the selected polygons), unless a piece couldn't \
     lie flat otherwise. Drawn blue."
);
pub(crate) const CLEAR_PREFERENCE_LABEL: &str = crate::text!("Clear seam preference");
pub(crate) const CLEAR_PREFERENCE_HELP: &str =
    crate::text!("Let the automatic tools choose freely again on the selected edges (or polygons)");

/// The UV menu's name for the stretch budget submenu, and its parts.
pub(crate) const BUDGET_LABEL: &str = crate::text!("Stretch budget");
pub(crate) const BUDGET_VALUE_LABEL: &str = crate::text!("Budget");
pub(crate) const SET_BUDGET_LABEL: &str = crate::text!("Give the selection this budget");
pub(crate) const CLEAR_BUDGET_LABEL: &str = crate::text!("Clear the selection's budget");

impl Workbench {
    /// Gives the selected edges (or polygons' edges) `preference`, as one
    /// undo step.
    pub(crate) fn run_seam_preference(&mut self, preference: SeamPreference) {
        let name = match preference {
            SeamPreference::Prefer => tr(PREFER_LABEL),
            SeamPreference::Never => tr(NEVER_LABEL),
            SeamPreference::Free => tr(CLEAR_PREFERENCE_LABEL),
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
                    if preference == SeamPreference::Prefer {
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
            let edges: Vec<EdgeKey> = edges
                .into_iter()
                .filter(|e| {
                    topology.edge_uses(*e).len() == 2
                        && mesh::uvs::seam_preference(&mesh, *e) != preference
                })
                .collect();
            if edges.is_empty() {
                continue;
            }
            count += edges.len();
            let places = ends(&edges);
            ops.push((
                id,
                Operation::SetSeamPreference {
                    edges,
                    preference,
                    anchors: Anchors::record(&mesh, ElementKind::Edges, &places),
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(if any {
                crate::trf!(
                    "{}: those edges are like that already (or open edges, cut already).",
                    name
                )
            } else {
                crate::trf!("{}: select some edges first (or polygons).", name)
            });
            return;
        }
        let verb = match preference {
            SeamPreference::Prefer => tr("Prefer seams on"),
            SeamPreference::Never => tr("Allow no seams on"),
            SeamPreference::Free => tr("Clear the seam preference of"),
        };
        let label = counted(verb, count, tr("edge"));
        if self.record_each(&label, ops) {
            let why = match preference {
                SeamPreference::Prefer => {
                    tr(" Suggest seams and Unwrap cut there first when they need to.")
                }
                SeamPreference::Never => tr(" Suggest seams and Unwrap won't cut there."),
                SeamPreference::Free => "",
            };
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.{}", label, why));
        }
    }

    /// Gives the pieces holding the selection (whole objects with nothing
    /// selected inside them) the stretch budget `budget`, or clears theirs,
    /// as one undo step.
    pub(crate) fn run_stretch_budget(&mut self, budget: Option<f32>) {
        let shapes = self.own_shapes();
        if shapes.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects first.",
                tr(BUDGET_LABEL)
            ));
            return;
        }
        let budget =
            budget.map(|b| b.clamp(*scene::BUDGET_RANGE.start(), *scene::BUDGET_RANGE.end()));
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh) in shapes {
            let chosen = super::unwrapping::targets(&mesh, self.document.elements(id), self.mode);
            // The whole pieces holding them, so a piece has one budget.
            let cuts = uv::unwrap::cuts_of(&mesh);
            let wanted: BTreeSet<FaceId> = chosen.into_iter().collect();
            let faces: Vec<FaceId> = uv::seams::pieces(&mesh, &cuts)
                .into_iter()
                .filter(|p| p.iter().any(|f| wanted.contains(f)))
                .flatten()
                .collect();
            if faces.is_empty() {
                continue;
            }
            count += faces.len();
            let anchors = super::unwrapping::anchors(&mesh, &faces);
            ops.push((
                id,
                Operation::SetStretchBudget {
                    faces,
                    budget,
                    anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: the selected objects have no polygons.",
                tr(BUDGET_LABEL)
            ));
            return;
        }
        let verb = if budget.is_some() {
            tr("Set the stretch budget of")
        } else {
            tr("Clear the stretch budget of")
        };
        let label = counted(verb, count, tr("polygon"));
        if self.record_each(&label, ops) {
            let why = match budget {
                Some(b) => crate::trf!(
                    " Suggest seams and Unwrap cut these pieces until no polygon is more than \
                     {:.2}× off.",
                    b
                ),
                None => String::new(),
            };
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.{}", label, why));
        }
    }

    /// The UV menu's Stretch budget submenu.
    pub(crate) fn budget_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(tr(BUDGET_VALUE_LABEL));
            ui.add(
                egui::DragValue::new(&mut self.uv_settings.piece_budget)
                    .range(scene::BUDGET_RANGE)
                    .speed(0.01)
                    .max_decimals(2)
                    .suffix("×"),
            )
            .on_hover_text(tr(
                "The most any polygon of the piece may be off once laid flat: stretched this \
                 many times more one way than the other, or this many times bigger or smaller \
                 than the rest. 1.5× is strict; 2× (red in the Stretch view) is loose.",
            ));
        });
        if ui
            .button(tr(SET_BUDGET_LABEL))
            .on_hover_text(tr(
                "The pieces holding the selection (or whole objects) keep this budget: Suggest \
                 seams and Unwrap cut them further until they are within it",
            ))
            .clicked()
        {
            let budget = self.uv_settings.piece_budget;
            self.run_stretch_budget(Some(budget));
            ui.close();
        }
        if ui
            .button(tr(CLEAR_BUDGET_LABEL))
            .on_hover_text(tr(
                "The pieces holding the selection follow the budget in UV → Seam suggestions again",
            ))
            .clicked()
        {
            self.run_stretch_budget(None);
            ui.close();
        }
    }
}
