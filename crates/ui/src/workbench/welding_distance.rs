//! The workbench's part in Weld by distance now ([decision 0152](../../../../docs/decisions/0152-weld-by-distance.md)):
//! the one-off Weld by distance, in the Mesh menu's Clean up group and
//! command search (no key). In the Points, Edges and Polygons modes it
//! welds among the selected elements' points; in the Model mode, across
//! the selected shapes. It is recorded per object as an
//! `Operation::WeldByDistance`, one undo step in all; its settings
//! (remembered for the next one) can be changed under Last edit.

use super::Workbench;
use crate::commands::CommandId;
use crate::palette::Mode;
use crate::text::tr;
use crate::tool_menu::Ready;
use mesh::Anchors;
use scene::{ElementKind, ElementSelection, Mesh, ObjectId, Operation, VertexId};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The points of the elements selected in `mode`, in order of id.
fn points_of(mode: Mode, selected: &ElementSelection, mesh: &Mesh) -> Vec<VertexId> {
    let mut points: BTreeSet<VertexId> = BTreeSet::new();
    match mode {
        Mode::Points => points.extend(selected.points()),
        Mode::Edges => {
            for edge in selected.edges() {
                let (a, b) = edge.ends();
                points.extend([a, b]);
            }
        }
        Mode::Polygons => {
            for face in selected.polygons() {
                points.extend(mesh.face(face).unwrap_or_default());
            }
        }
        _ => {}
    }
    points.into_iter().collect()
}

impl Workbench {
    /// Whether Weld by distance now can be used: in the Model mode (whole
    /// shapes) or an element mode (the selected elements' points), with
    /// something selected.
    pub(super) fn weld_distance_readiness(&self) -> Ready {
        if !CommandId::WeldByDistanceNow.modes().contains(&self.mode) {
            return Ready::NotNow(
                tr(
                    "Weld by distance now works on whole shapes in the Model mode, or on the \
                     selected points, edges or polygons",
                )
                .into(),
            );
        }
        if self.document.selection().is_empty() {
            return Ready::NotNow(tr("Select one or more shapes first").into());
        }
        Ready::Yes
    }

    /// The objects Weld by distance now works on, with their shapes and
    /// the points chosen on each (none for the whole shape, in the Model
    /// mode). Curves' solids, instances and groups are left out.
    fn weld_targets(&self) -> Vec<(ObjectId, Option<Vec<VertexId>>, Arc<Mesh>)> {
        if self.mode != Mode::Model {
            return self
                .selected_parts()
                .into_iter()
                .map(|(id, selected, mesh)| {
                    let points = points_of(self.mode, &selected, &mesh);
                    (id, Some(points), mesh)
                })
                .collect();
        }
        let scene = self.document.scene();
        self.document
            .selection()
            .into_iter()
            .filter_map(|id| {
                let kind = &scene.object(id)?.kind;
                // An instance shows its original's shape: weld that.
                if kind.curve().is_some() || matches!(kind, scene::ObjectKind::Instance(_)) {
                    return None;
                }
                Some((id, None, scene.evaluated(id)?.mesh))
            })
            .collect()
    }

    /// Weld by distance now: welds points closer together than the
    /// distance into one, with the settings last used.
    pub(super) fn run_weld_distance_now(&mut self) {
        let options = self.attributes.weld.clamped();
        let mut ops = Vec::new();
        let mut went = 0;
        for (id, chosen, mesh) in self.weld_targets() {
            let (points, anchors) = match chosen {
                Some(points) => {
                    let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
                    let anchors = Anchors::record(&mesh, ElementKind::Points, &places);
                    (points, anchors)
                }
                None => (Vec::new(), None),
            };
            let within = (!points.is_empty()).then_some(points.as_slice());
            let count = within.map_or_else(|| mesh.vertex_count(), <[VertexId]>::len);
            // A single point has nothing to weld with.
            if count < 2 {
                continue;
            }
            // Recorded even when this distance welds nothing, so it can be
            // raised under Last edit.
            went += mesh::weld_distance::weld_groups_of(&mesh, within, &options)
                .iter()
                .map(|g| g.len() - 1)
                .sum::<usize>();
            ops.push((
                id,
                Operation::WeldByDistance {
                    points,
                    options,
                    anchors,
                },
            ));
        }
        if ops.is_empty() {
            self.note = Some(if self.mode == Mode::Model {
                tr("Weld by distance now: select one or more shapes first.").into()
            } else {
                tr(
                    "Weld by distance now: select two or more points first (or edges or \
                     polygons), or press Tab for the Model mode to weld whole shapes.",
                )
                .into()
            });
            return;
        }
        let label = match ops.as_slice() {
            [(_, op)] => op.describe(),
            _ => tr("Weld by distance").to_string(),
        };
        if self.record_each(&label, ops) {
            self.note = Some(match went {
                0 => tr(
                    "Weld by distance now found no points closer together than the Distance: \
                     raise it under Last edit.",
                )
                .into(),
                1 => tr(
                    "Welded 1 point away. Ctrl + Z takes it back; change the Distance under Last \
                     edit.",
                )
                .into(),
                _ => crate::trf!(
                    "Welded {} points away. Ctrl + Z takes it back; change the Distance under \
                     Last edit.",
                    went
                ),
            });
        }
    }
}
