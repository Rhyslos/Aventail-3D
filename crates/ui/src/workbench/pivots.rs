//! The workbench's part in placing pivots, in the Object mode
//! ([decision 0068](../../../../docs/decisions/0068-pivot-editing-and-scene-units.md)):
//! the pivot to the middle of the shape, to the world origin or to
//! another object's pivot, and the object to the world origin.
//!
//! Moving or rotating with the gizmo in the Object mode moves the pivots
//! instead (see `transforming`); these are the ready-made places.

use super::Workbench;
use crate::selecting::Gesture;
use glam::Vec3;
use scene::{Command, ObjectId, Transform};

/// Where a ready-made placement puts things.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Each selected object's pivot to the middle of its shape.
    PivotToCentre,
    /// Each selected object's pivot to the world origin.
    PivotToOrigin,
    /// Each selected object, shape and pivot together, so its pivot sits
    /// at the world origin.
    ObjectToOrigin,
}

impl Workbench {
    /// The selected objects not inside another selected one: what moves.
    fn top_selected(&self) -> Vec<ObjectId> {
        let scene = self.document.scene();
        let selection = self.document.selection();
        selection
            .iter()
            .copied()
            .filter(|id| {
                !selection
                    .iter()
                    .any(|o| o != id && scene.is_inside(*id, *o))
            })
            .collect()
    }

    /// `id`'s transform with its pivot moved to `world` (turn and size
    /// kept), or `None` when it is already there.
    fn placed_at(&self, id: ObjectId, world: Vec3) -> Option<Transform> {
        let scene = self.document.scene();
        let object = scene.object(id)?;
        let parent = scene
            .parent(id)
            .map_or(glam::Mat4::IDENTITY, |p| scene.world_matrix(p));
        let inverse = parent.inverse();
        if !inverse.is_finite() {
            return None;
        }
        let mut t = object.transform;
        t.position = inverse.transform_point3(world);
        (t != object.transform).then_some(t)
    }

    /// The middle of `id`'s own shape in the world; for a group, of
    /// everything in it.
    fn shape_centre(&self, id: ObjectId) -> Option<Vec3> {
        let scene = self.document.scene();
        match scene.local_bounds(id) {
            Some(local) => Some(scene.shape_matrix(id).transform_point3(local.centre())),
            None => scene.bounds_of([id]).map(|b| b.centre()),
        }
    }

    /// Carries out a ready-made placement on the selection.
    pub(super) fn run_placement(&mut self, placement: Placement) {
        let ids = self.top_selected();
        if ids.is_empty() {
            self.note = Some("Select one or more objects first.".into());
            return;
        }
        let changes: Vec<(ObjectId, Transform)> = ids
            .iter()
            .filter_map(|id| {
                let to = match placement {
                    Placement::PivotToCentre => self.shape_centre(*id)?,
                    Placement::PivotToOrigin | Placement::ObjectToOrigin => Vec3::ZERO,
                };
                Some((*id, self.placed_at(*id, to)?))
            })
            .collect();
        let name = match placement {
            Placement::PivotToCentre => "Pivot to centre",
            Placement::PivotToOrigin => "Pivot to world origin",
            Placement::ObjectToOrigin => "Object to world origin",
        };
        if changes.is_empty() {
            self.note = Some(format!("{name}: already there."));
            return;
        }
        let command = match placement {
            Placement::ObjectToOrigin => Command::SetTransforms(changes),
            _ => Command::MovePivots(changes),
        };
        self.note = Some(match self.document.execute_labelled(command, name, None) {
            Ok(()) => format!("{name}. Ctrl + Z takes it back."),
            Err(why) => format!("Couldn't do {}: {why}.", name.to_lowercase()),
        });
    }

    /// Starts Pivot to object: the next click on an object moves the
    /// selected objects' pivots to that object's pivot.
    pub(super) fn start_pivot_to_object(&mut self) {
        if self.top_selected().is_empty() {
            self.note = Some("Select one or more objects first.".into());
            return;
        }
        self.pivot_target = true;
        self.note = Some(PIVOT_TARGET_NOTE.into());
    }

    /// Whether Pivot to object is waiting for a click.
    pub fn is_picking_pivot_target(&self) -> bool {
        self.pivot_target
    }

    /// Stops Pivot to object without changing anything.
    pub(super) fn cancel_pivot_target(&mut self) {
        if self.pivot_target {
            self.pivot_target = false;
            self.note = Some("Pivot to object cancelled.".into());
        }
    }

    /// While Pivot to object waits, takes this frame's clicks from
    /// selecting: a click on an object finishes it, other gestures are
    /// dropped. Returns the gestures left for selecting.
    pub(super) fn take_pivot_target_click(
        &mut self,
        gestures: Vec<Gesture>,
        drawing: &crate::drawing::SceneDrawing,
    ) -> Vec<Gesture> {
        if !self.pivot_target {
            return gestures;
        }
        if self.document.selection().is_empty() {
            self.pivot_target = false;
            return gestures;
        }
        for gesture in gestures {
            let Gesture::Click { pointer, .. } = gesture else {
                continue;
            };
            let target = self
                .components
                .pick_scene(drawing)
                .object_under(&pointer.camera, pointer.size, pointer.at)
                .and_then(|i| drawing.items.get(i))
                .map(|item| item.id);
            match target {
                Some(target) => {
                    self.pivot_target = false;
                    self.pivot_to_object(target);
                    break;
                }
                None => self.note = Some(PIVOT_TARGET_NOTE.into()),
            }
        }
        Vec::new()
    }

    /// Moves the selected objects' pivots to `target`'s pivot.
    pub(crate) fn pivot_to_object(&mut self, target: ObjectId) {
        let scene = self.document.scene();
        let at = scene.world_matrix(target).w_axis.truncate();
        let name = scene
            .object(target)
            .map_or_else(String::new, |o| o.name.clone());
        let changes: Vec<(ObjectId, Transform)> = self
            .top_selected()
            .into_iter()
            .filter(|id| *id != target)
            .filter_map(|id| Some((id, self.placed_at(id, at)?)))
            .collect();
        if changes.is_empty() {
            self.note = Some(format!(
                "Pivot to object: the pivot is already at {name}'s pivot."
            ));
            return;
        }
        let label = format!("Pivot to {name}");
        self.note = Some(
            match self
                .document
                .execute_labelled(Command::MovePivots(changes), label.clone(), None)
            {
                Ok(()) => format!("{label}. Ctrl + Z takes it back."),
                Err(why) => format!("Couldn't move the pivot: {why}."),
            },
        );
    }
}

/// The status bar while Pivot to object waits for a click.
pub(crate) const PIVOT_TARGET_NOTE: &str =
    "Pivot to object: click the object whose pivot to use. Esc cancels.";
