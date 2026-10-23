//! The workbench's part in placing pivots, in the Object mode
//! ([decision 0068](../../../../docs/decisions/0068-pivot-editing-and-scene-units.md)):
//! the pivot to the middle of the shape, to the world origin or to
//! another object's pivot, and the object to the world origin.
//!
//! Moving or rotating with the gizmo in the Object mode moves the pivots
//! instead (see `transforming`); these are the ready-made places.

use super::Workbench;
use crate::selecting::Gesture;
use crate::text::tr;
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
    pub(super) fn top_selected(&self) -> Vec<ObjectId> {
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
    pub(super) fn placed_at(&self, id: ObjectId, world: Vec3) -> Option<Transform> {
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
            self.note = Some(tr("Select one or more objects first.").into());
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
            Placement::PivotToCentre => tr("Pivot to centre"),
            Placement::PivotToOrigin => tr("Pivot to world origin"),
            Placement::ObjectToOrigin => tr("Object to world origin"),
        };
        if changes.is_empty() {
            self.note = Some(crate::trf!("{}: already there.", name));
            return;
        }
        let command = match placement {
            Placement::ObjectToOrigin => Command::SetTransforms(changes),
            _ => Command::MovePivots(changes),
        };
        self.note = Some(match self.document.execute_labelled(command, name, None) {
            Ok(()) => crate::trf!("{}. Ctrl + Z takes it back.", name),
            Err(why) => crate::trf!("Couldn't do {}: {}.", name.to_lowercase(), why),
        });
    }

    /// Starts Pivot to object: the next click on an object moves the
    /// selected objects' pivots to that object's pivot.
    pub(super) fn start_pivot_to_object(&mut self) {
        self.start_target_pick(super::aligning::TargetUse::Pivot);
    }

    /// Whether Pivot to object is waiting for a click.
    pub fn is_picking_pivot_target(&self) -> bool {
        self.target_pick == Some(super::aligning::TargetUse::Pivot)
    }

    /// While a target is to be clicked (Pivot to object, Snap to, Align),
    /// takes this frame's clicks from selecting: a click on an object
    /// finishes it, other gestures are dropped. Returns the gestures left
    /// for selecting.
    pub(super) fn take_target_click(
        &mut self,
        gestures: Vec<Gesture>,
        drawing: &crate::drawing::SceneDrawing,
    ) -> Vec<Gesture> {
        let Some(what) = self.target_pick else {
            return gestures;
        };
        if self.document.selection().is_empty() {
            self.target_pick = None;
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
                    self.target_pick = None;
                    self.target_picked(what, target);
                    break;
                }
                None => self.note = Some(what.note().into()),
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
            self.note = Some(crate::trf!(
                "Pivot to object: the pivot is already at {}'s pivot.",
                name
            ));
            return;
        }
        let label = crate::trf!("Pivot to {}", name);
        self.note = Some(
            match self
                .document
                .execute_labelled(Command::MovePivots(changes), label.clone(), None)
            {
                Ok(()) => crate::trf!("{}. Ctrl + Z takes it back.", label),
                Err(why) => crate::trf!("Couldn't move the pivot: {}.", why),
            },
        );
    }
}

/// The status bar while Pivot to object waits for a click.
pub(crate) const PIVOT_TARGET_NOTE: &str =
    crate::text!("Pivot to object: click the object whose pivot to use. Esc cancels.");
