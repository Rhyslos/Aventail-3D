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
use mesh::Anchors;
use scene::{EdgeKey, ElementKind, Operation, VertexId};
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
        if self.record_each(&label, ops) {
            self.note = Some(format!(
                "{label}. Ctrl + Z takes it back.{}",
                self.auto_hard_note()
            ));
        }
    }

    /// What to add to a hard or soft edge command's note while Automatic
    /// hard edges is on: the marks are kept, but the angle decides the
    /// shading until it is switched off (decision 0064).
    fn auto_hard_note(&self) -> &'static str {
        if self.viewport.options().hard_edges_by_hand {
            ""
        } else {
            " Automatic hard edges is on (Display menu), so the angle decides the shading \
             for now; switch it off to see edges set by hand."
        }
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
                "Edges sharper than {angle:.0}° are hard, the rest soft. Change the Angle \
                 under Last edit; Ctrl + Z takes it back.{}",
                self.auto_hard_note()
            ));
        }
    }
}
