//! The workbench's part in hard and soft edges
//! ([decision 0053](../../../../docs/decisions/0053-hard-and-soft-edges.md)).
//!
//! No keys, as the project lead chose: Hard edges and Soft edges are in
//! the tool menu (`E`) and the Mesh menu, for the selected edges (Edges
//! mode) or the edges of the selected polygons (Polygons mode); Hard
//! edges by angle and Select hard edges are in the Mesh and Select menus.

use super::Workbench;
use super::joining::counted;
use crate::palette::Mode;
use crate::text::tr;
use mesh::Anchors;
use scene::{Command, EdgeKey, ElementKind, Operation, VertexId};
use std::collections::BTreeSet;

impl Workbench {
    /// Hard edges (`hard`) or Soft edges: marks the selected edges, or
    /// the selected polygons' edges.
    pub(super) fn run_hardness(&mut self, hard: bool) {
        let name = if hard {
            "Mark hard edges"
        } else {
            "Mark soft edges"
        };
        let polygons = self.mode == Mode::Polygons;
        let mut ops = Vec::new();
        let mut count = 0;
        let mut already = 0;
        for (id, selected, mesh) in self.selected_parts() {
            let edges: BTreeSet<EdgeKey> = if polygons {
                selected
                    .polygons()
                    .flat_map(|f| mesh.topology().face_edges(f).iter().copied())
                    .collect()
            } else {
                selected.edges().collect()
            };
            // Only the edges that change, so nothing empty is recorded.
            let edges: Vec<EdgeKey> = edges
                .into_iter()
                .filter(|e| {
                    let (a, b) = e.ends();
                    let differs = mesh.is_hard(a, b) != hard;
                    already += usize::from(!differs);
                    differs
                })
                .collect();
            if edges.is_empty() {
                continue;
            }
            count += edges.len();
            let ends: Vec<Vec<VertexId>> = edges
                .iter()
                .map(|e| {
                    let (a, b) = e.ends();
                    vec![a, b]
                })
                .collect();
            ops.push((
                id,
                Operation::SetHardEdges {
                    edges,
                    hard,
                    anchors: Anchors::record(&mesh, ElementKind::Edges, &ends),
                },
            ));
        }
        if ops.is_empty() {
            let (what, whose) = if polygons {
                ("polygons", "selected polygons' edges")
            } else {
                ("edges", "selected edges")
            };
            let state = if hard { "hard" } else { "soft" };
            self.note = Some(if already > 0 {
                format!("{name}: the {whose} are already {state}.")
            } else {
                format!("{name}: select some {what} first.")
            });
            return;
        }
        let label = counted(if hard { "Harden" } else { "Soften" }, count, "edge");
        let key = self.new_step_key();
        let ids: Vec<scene::ObjectId> = ops.iter().map(|(id, _)| *id).collect();
        let switched = self.switch_to_marks(&ids, key, &label);
        if self.record_each_in(&label, ops, key) {
            self.note = Some(if switched > 0 {
                format!(
                    "{label}. {} smoothed by angle, so it now uses its marked edges: its look \
                     was kept as marks, and yours show. Ctrl + Z takes it all back.",
                    if switched == 1 {
                        "The object was".to_string()
                    } else {
                        format!("{switched} objects were")
                    }
                )
            } else {
                format!("{label}. Ctrl + Z takes it back.")
            });
        }
    }

    /// The Mesh menu's Smooth by angle switch (decision 0126): ticked when
    /// every selected shape is smoothed by angle. Ticking smooths them by
    /// the standard angle; unticking shades them by their marked edges.
    pub(super) fn smoothing_entry(&mut self, ui: &mut egui::Ui) {
        let scene = self.document.scene();
        let ids: Vec<scene::ObjectId> = self
            .document
            .selection()
            .into_iter()
            .filter(|id| scene.object(*id).is_some_and(|o| o.kind.has_shape()))
            .collect();
        let mut on = !ids.is_empty()
            && ids.iter().all(|id| {
                scene
                    .object(*id)
                    .is_some_and(|o| matches!(o.hardness, mesh::Hardness::ByAngle(_)))
            });
        let response = ui
            .add_enabled(
                !ids.is_empty(),
                egui::Checkbox::new(&mut on, tr(crate::viewport::chrome::AUTO_HARD_EDGES_ENTRY)),
            )
            .on_hover_text(format!(
                "Shade the selected objects by angle: edges where polygons meet more sharply \
                 than {}° are crisp, the rest smooth, after every change. Untick it to shade \
                 them only by the hard edges you mark. Each object keeps its own (the \
                 Attributes panel shows its angle).",
                scene::STANDARD_SMOOTHING_ANGLE
            ))
            .on_disabled_hover_text("Select the objects to set how they're smoothed");
        if response.changed() {
            self.set_smoothing(
                ids,
                if on {
                    mesh::Hardness::ByAngle(scene::STANDARD_SMOOTHING_ANGLE)
                } else {
                    mesh::Hardness::Marked
                },
            );
            ui.close();
        }
    }

    /// Sets the objects' smoothing, as one undo step.
    pub(crate) fn set_smoothing(&mut self, ids: Vec<scene::ObjectId>, hardness: mesh::Hardness) {
        if let Err(why) = self
            .document
            .execute(scene::Command::SetHardness { ids, hardness })
        {
            self.note = Some(format!("Couldn't change the smoothing: {why}."));
        }
    }

    /// Before marking edges on objects smoothed by angle (decision 0126,
    /// issue #114): their current look is kept as marks (Hard edges by
    /// angle at their angle), and they switch to their marked edges, so the
    /// marks made next show. Part of the undo step `key`. Returns how many
    /// objects switched.
    fn switch_to_marks(&mut self, ids: &[scene::ObjectId], key: u64, label: &str) -> usize {
        let scene = self.document.scene();
        let by_angle: Vec<(scene::ObjectId, f32)> = ids
            .iter()
            .filter_map(|id| match scene.object(*id)?.hardness {
                mesh::Hardness::ByAngle(angle) => Some((*id, angle)),
                mesh::Hardness::Marked => None,
            })
            .collect();
        for (id, angle) in &by_angle {
            let _ = self.document.execute_labelled(
                Command::Record {
                    id: *id,
                    operation: Operation::HardEdgesByAngle { angle: *angle },
                },
                label,
                Some(key),
            );
            let _ = self.document.execute_labelled(
                Command::SetHardness {
                    ids: vec![*id],
                    hardness: mesh::Hardness::Marked,
                },
                label,
                Some(key),
            );
        }
        by_angle.len()
    }

    /// Hard edges by angle: in each selected object, the edges sharper
    /// than the angle become hard and the rest soft.
    pub(super) fn run_hard_by_angle(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some("Hard edges by angle: select the objects first.".into());
            return;
        }
        let angle = self.attributes.hard_angle.0;
        let ops: Vec<_> = self
            .selected_parts()
            .into_iter()
            .map(|(id, _, _)| (id, Operation::HardEdgesByAngle { angle }))
            .collect();
        if ops.is_empty() {
            self.note = Some("Hard edges by angle: the selection has no shapes.".into());
            return;
        }
        if self.record_each("Hard edges by angle", ops) {
            self.note = Some(format!(
                "Edges sharper than {angle:.0}° are marked hard, the rest soft. Change the \
                 Angle under Last edit; Ctrl + Z takes it back."
            ));
        }
    }
}
