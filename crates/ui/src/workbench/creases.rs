//! The workbench's part in creases ([decision 0143](../../../../docs/decisions/0143-subdivide-and-creases.md)):
//! how sharply the Subdivide modifier keeps edges; and in bevel weights
//! ([decision 0149](../../../../docs/decisions/0149-live-bevel.md)), set
//! the same way through the Attributes panel's Bevel weight field.
//!
//! No keys: Crease edges and Uncrease are in the Edges mode's tool menu
//! (`E`, in Flip normals' place, which works on polygons only), the Mesh
//! menu and command search; the Attributes panel's Crease field sets any
//! crease from 0 to 1 for the selected edges. Every change is a recorded
//! edit (made on both sides with symmetry on), so it is undone like any
//! other. One drag of the Crease (or Bevel weight) field is one edit: the
//! edit made at its start is changed as it goes.

use super::Workbench;
use super::joining::counted;
use crate::attributes::{SelectedEdgeValue, SelectedEdges};
use crate::edge_values::EdgeValue;
use crate::palette::Mode;
use crate::text::tr;
use mesh::Anchors;
use scene::{Command, EdgeKey, ElementKind, ObjectId, Operation, VertexId};

/// A drag of the Crease or Bevel weight field in progress: which, its
/// gesture, its undo step, and where each edit it made sits in its
/// object's stack.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CreaseDrag {
    what: EdgeValue,
    gesture: u64,
    key: u64,
    edits: Vec<(ObjectId, usize, usize)>,
}

impl Workbench {
    /// The selected edges' creases and bevel weights, for the Attributes
    /// panel: in the Edges mode only.
    pub(super) fn edge_values_for_attributes(&self) -> Option<SelectedEdges> {
        if self.mode != Mode::Edges {
            return None;
        }
        let parts = self.selected_parts();
        let read = |what: EdgeValue| {
            let mut edges = 0;
            let mut first: Option<f32> = None;
            let mut mixed = false;
            for (_, selected, mesh) in &parts {
                for edge in selected.edges() {
                    let value = what.of(mesh, edge);
                    edges += 1;
                    match first {
                        None => first = Some(value),
                        Some(c) if (c - value).abs() > 1e-6 => mixed = true,
                        Some(_) => {}
                    }
                }
            }
            SelectedEdgeValue {
                edges,
                value: first.unwrap_or(0.0),
                mixed,
            }
        };
        Some(SelectedEdges {
            crease: read(EdgeValue::Crease),
            bevel_weight: read(EdgeValue::BevelWeight),
        })
    }

    /// The edits giving the selected edges `value` of `what`, one per
    /// object, with how many edges they change. With `all`, every selected
    /// edge is in them (so a drag can change the value later); otherwise
    /// only those whose value differs. Also how many were already so.
    fn edge_value_edits(
        &self,
        what: EdgeValue,
        value: f32,
        all: bool,
    ) -> (Vec<(ObjectId, Operation)>, usize, usize) {
        let mut ops = Vec::new();
        let mut count = 0;
        let mut already = 0;
        for (id, selected, mesh) in self.selected_parts() {
            let picked: Vec<EdgeKey> = selected.edges().collect();
            let differs = |e: &EdgeKey| (what.of(&mesh, *e) - value).abs() > 1e-6;
            let differing = picked.iter().filter(|e| differs(e)).count();
            already += picked.len() - differing;
            let edges: Vec<EdgeKey> = if all {
                if differing == 0 {
                    continue;
                }
                picked
            } else {
                picked.into_iter().filter(|e| differs(e)).collect()
            };
            if edges.is_empty() {
                continue;
            }
            count += differing;
            let ends: Vec<Vec<VertexId>> = edges
                .iter()
                .map(|e| {
                    let (a, b) = e.ends();
                    vec![a, b]
                })
                .collect();
            let anchors = Anchors::record(&mesh, ElementKind::Edges, &ends);
            ops.push((id, what.operation(edges, value, anchors)));
        }
        (ops, count, already)
    }

    /// Crease edges (`crease` 1) or Uncrease (0): the selected edges, in
    /// the Edges mode.
    pub(super) fn run_creases(&mut self, crease: f32) {
        let name = if crease > 0.0 {
            tr("Crease edges")
        } else {
            tr("Uncrease")
        };
        if self.mode != Mode::Edges {
            self.note = Some(crate::trf!(
                "{}: press 2 for the Edges mode and select some edges first.",
                name
            ));
            return;
        }
        let (ops, count, already) = self.edge_value_edits(EdgeValue::Crease, crease, false);
        if ops.is_empty() {
            self.note = Some(if already > 0 {
                if crease > 0.0 {
                    crate::trf!("{}: the selected edges are already creased.", name)
                } else {
                    crate::trf!("{}: the selected edges have no crease.", name)
                }
            } else {
                crate::trf!("{}: select some edges first.", name)
            });
            return;
        }
        let label = counted(
            if crease > 0.0 {
                tr("Crease")
            } else {
                tr("Uncrease")
            },
            count,
            tr("edge"),
        );
        if self.record_each(&label, ops) {
            self.note = Some(if crease > 0.0 {
                crate::trf!(
                    "{}: the Subdivide modifier keeps them sharp. Set a softer crease in the \
                     Attributes panel. Ctrl + Z takes it back.",
                    label
                )
            } else {
                crate::trf!("{}. Ctrl + Z takes it back.", label)
            });
        }
    }

    /// The Attributes panel's Crease or Bevel weight field changed to
    /// `value` by `gesture`: records the value on the selected edges at
    /// the start of a drag, and changes that edit as the drag goes on.
    pub(super) fn set_edge_value_from_field(&mut self, what: EdgeValue, value: f32, gesture: u64) {
        let value = what.usable(value);
        if let Some(drag) = self.crease_drag.clone()
            && drag.gesture == gesture
            && drag.what == what
            && self.change_crease_drag(&drag, value)
        {
            return;
        }
        let (ops, count, _) = self.edge_value_edits(what, value, true);
        if ops.is_empty() {
            self.crease_drag = None;
            return;
        }
        let label = counted(what.name(), count, tr("edge"));
        let key = self.new_step_key();
        let ids: Vec<ObjectId> = ops.iter().map(|(id, _)| *id).collect();
        if !self.record_each_in(&label, ops, key) {
            self.crease_drag = None;
            return;
        }
        // Where each edit landed: its object's last edit.
        let edits = ids
            .into_iter()
            .filter_map(|id| {
                let object = self.document.scene().object(id)?;
                let (layer, index, op) = crate::attributes::last_edit(&object.stack)?;
                what.sets(op.inner()).then_some((id, layer, index))
            })
            .collect();
        self.crease_drag = Some(CreaseDrag {
            what,
            gesture,
            key,
            edits,
        });
    }

    /// Changes the edits of the field's drag to `value`. `false` when they
    /// aren't there any more (something else was done since).
    fn change_crease_drag(&mut self, drag: &CreaseDrag, value: f32) -> bool {
        let mut changes = Vec::new();
        for (id, layer, index) in &drag.edits {
            let Some(op) = self
                .document
                .scene()
                .object(*id)
                .and_then(|o| o.stack.layers().get(*layer))
                .and_then(|l| l.entries.get(*index))
                .map(|e| e.operation.clone())
            else {
                return false;
            };
            let Some(changed) = drag.what.with_value(&op, value) else {
                return false;
            };
            changes.push((*id, *layer, *index, changed));
        }
        if changes.is_empty() {
            return false;
        }
        let label = counted(drag.what.name(), self.creases_selected(), tr("edge"));
        for (id, layer, index, operation) in changes {
            let command = Command::ReplaceOperation {
                id,
                layer,
                index,
                operation,
            };
            if let Err(why) = self
                .document
                .execute_labelled(command, &label, Some(drag.key))
            {
                self.note = Some(crate::trf!("Couldn't do that: {}.", why));
                return true;
            }
        }
        true
    }

    /// How many edges are selected.
    fn creases_selected(&self) -> usize {
        self.selected_parts()
            .iter()
            .map(|(_, s, _)| s.count(ElementKind::Edges))
            .sum()
    }
}
