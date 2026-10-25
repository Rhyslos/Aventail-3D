//! Commands: every change a user can make to the scene, each one undoable.

use crate::object::{Object, ObjectId, ObjectKind, Transform};
use crate::stack::{EditId, Entry, Layer, LayerChange, Operation, Stack};
use crate::tree::{Edit, Place, Scene, SceneError};
use mesh::Primitive;
use std::sync::Arc;
use words::{text, tr, trf};

mod booleans;
mod curves;
mod lattices;
mod modifiers;

/// A change to the scene. Every change goes through one, so it can be
/// undone and redone.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Adds a new object of `kind` at the top of the Objects panel's
    /// list, named after its kind (with a number if that name is taken).
    /// It becomes the selection.
    Create(ObjectKind),
    /// Adds a ready-made object at `place`. It becomes the selection.
    Insert {
        /// The object.
        object: Object,
        /// Where it goes.
        place: Place,
    },
    /// Adds ready-made objects at the end of the top level, in order, as
    /// one step (importing a file, decision 0071). A name already taken
    /// gets a number. They become the selection.
    InsertMany(Vec<Object>),
    /// Renames an object. Spaces around the name are trimmed; an empty
    /// name is refused.
    Rename {
        /// The object.
        id: ObjectId,
        /// Its new name.
        name: String,
    },
    /// Shows or hides objects (and so everything inside them).
    SetVisible {
        /// The objects.
        ids: Vec<ObjectId>,
        /// Shown (`true`) or hidden.
        visible: bool,
    },
    /// Moves objects, with everything inside them, under `parent` (`None`
    /// for the top of the tree), before the child now at `index` (or at
    /// the end). Moved objects keep their order. An object can't move into
    /// itself or anything inside it.
    Move {
        /// The objects. Any inside another one listed move with it.
        ids: Vec<ObjectId>,
        /// The new parent.
        parent: Option<ObjectId>,
        /// The position among the parent's children, counted before the
        /// move.
        index: usize,
    },
    /// Deletes objects and everything inside them.
    Delete(Vec<ObjectId>),
    /// Puts objects into a new group, which takes the place of the first
    /// of them. The group becomes the selection.
    Group(Vec<ObjectId>),
    /// Takes everything out of the groups among these objects, putting it
    /// where each group was, and deletes the emptied groups. What was
    /// inside becomes the selection. Objects that aren't groups are left
    /// alone; with no group among them, nothing happens.
    Ungroup(Vec<ObjectId>),
    /// Gives primitives new settings (brought within limits first). Every
    /// object listed must be a primitive of the same kind as `primitive`:
    /// a cube's settings can't turn a sphere into a cube.
    SetPrimitive {
        /// The objects.
        ids: Vec<ObjectId>,
        /// Their new settings.
        primitive: Primitive,
    },
    /// Records an edit on top of an object's shape, in its top layer (an
    /// "Edits" layer is made if it has none). The base stays as it was, so
    /// a primitive's settings can still change and the edit replays on the
    /// new shape. Groups have no shape and are refused.
    Record {
        /// The object.
        id: ObjectId,
        /// The edit.
        operation: Operation,
    },
    /// Puts a changed version of a recorded edit in its place: an
    /// extrude's distance changed in the Attributes panel afterwards, say
    /// (decision 0038). Everything recorded above it replays on the new
    /// result.
    ReplaceOperation {
        /// The object.
        id: ObjectId,
        /// Which layer, from the bottom (0).
        layer: usize,
        /// Which operation in the layer, from the first (0).
        index: usize,
        /// The edit to put there.
        operation: Operation,
    },
    /// Switches a recorded edit off (the shape is as if it weren't there)
    /// or on again (decision 0044).
    SetEditEnabled {
        /// The object.
        id: ObjectId,
        /// The edit.
        edit: EditId,
        /// On or off.
        enabled: bool,
    },
    /// Pins a recorded edit (it stays changeable) or unpins it (decision
    /// 0044).
    SetEditPinned {
        /// The object.
        id: ObjectId,
        /// The edit.
        edit: EditId,
        /// Pinned or not.
        pinned: bool,
    },
    /// Moves, turns or scales objects: each gets a new transform, relative
    /// to its parent (decision 0030). A transform with a value that isn't
    /// a number is refused.
    SetTransforms(Vec<(ObjectId, Transform)>),
    /// Moves objects' pivots (their axes) without moving their shapes or
    /// their children (decision 0068): each object gets a new transform,
    /// relative to its parent, and its shape's offset and its children's
    /// transforms change the opposite way. A transform with a value that
    /// isn't a number, or that flattens the object, is refused.
    MovePivots(Vec<(ObjectId, Transform)>),
    /// Changes the project's units (decision 0068): how lengths are shown
    /// and typed. The shapes don't change.
    SetUnits(crate::Units),
    /// Applies objects' placement (decision 0089): their position,
    /// rotation and/or scale go back to none (0, 0° and 100%) while they
    /// look exactly as before. Their shapes take the change (a recorded
    /// edit placing the whole shape), and the objects inside them stay
    /// where they are. Instances are left alone: their shape is their
    /// original's.
    ApplyTransforms {
        /// The objects.
        ids: Vec<ObjectId>,
        /// What to apply.
        parts: crate::ApplyParts,
    },
    /// Adds an empty edit layer on top of each object with a shape of its
    /// own (decision 0091), named "Edits 2" and so on. New edits go into
    /// it. Groups and instances are left alone.
    AddLayer(Vec<ObjectId>),
    /// Changes edit layer `index` (from the bottom, 0) of object `id`
    /// (decision 0091).
    ChangeLayer {
        /// The object.
        id: ObjectId,
        /// Which layer, from the bottom (0).
        index: usize,
        /// What to change.
        change: crate::LayerChange,
    },
    /// Removes recorded edit `edit` from object `id` (decision 0092).
    /// The edits after it are made again without it.
    DeleteEdit {
        /// The object.
        id: ObjectId,
        /// The edit.
        edit: EditId,
    },
    /// Freezes object `id`'s edits up to and including `edit` (decision
    /// 0092): the base and those edits are baked into a plain mesh, which
    /// becomes the base, and they are removed, so later edits apply to it
    /// as before and the shape doesn't change. The layers below `edit`'s
    /// go (their edits are all frozen); its own layer keeps the edits after
    /// it. Refused if any of them is kept editable (pinned) or switched
    /// off, or a layer they are in is off.
    Freeze {
        /// The object.
        id: ObjectId,
        /// The last edit to freeze.
        edit: EditId,
    },
    /// Gives objects with a shape a live mirror (decision 0088), changes
    /// it, or takes it away (`None`). Groups are left alone; a mirror
    /// with no axis or numbers out of range is refused.
    SetMirror {
        /// The objects.
        ids: Vec<ObjectId>,
        /// The mirror.
        mirror: Option<crate::LiveMirror>,
    },
    /// Sets where objects' shading breaks (decision 0126): by angle, with
    /// marked hard edges added, or only by the marks. Groups are left
    /// alone; an angle that isn't a real number within 0–180° is refused.
    SetHardness {
        /// The objects.
        ids: Vec<ObjectId>,
        /// How their shading breaks.
        hardness: mesh::Hardness,
    },
    /// Gives objects with a shape a collision shape for game engines
    /// (decision 0078). Groups are left alone.
    SetCollision {
        /// The objects.
        ids: Vec<ObjectId>,
        /// The collision shape.
        collision: crate::Collision,
    },
    /// Changes the project's game project folder (decision 0076), or takes
    /// it away (`None`). A folder path that is empty, too long or has
    /// line breaks is refused.
    SetExportTarget(Option<crate::ExportTarget>),
    /// Changes the project's texture settings (decision 0100): its size
    /// and the texel density aimed for. The UVs don't change.
    SetTexture(crate::TextureSettings),
    /// Changes how the project's edits mend the UVs they break (decision
    /// 0110). Limits outside `uv::mend::LIMITS` are refused. Every shape
    /// is worked out again with the new settings.
    SetMending(crate::MendOptions),
    /// Changes the project's trim sheet (decision 0106): its rows. A sheet
    /// that isn't valid (`TrimSheet::is_valid`) is refused. The UVs don't
    /// change.
    SetTrim(crate::TrimSheet),
    /// Changes the project's layout snapshots (decision 0114): takes,
    /// renames, replaces or removes them. A list that isn't valid
    /// (`snapshot::snapshots_are_valid`) is refused. The UVs don't change:
    /// putting a snapshot back is a recorded edit on each object
    /// (`UvSnapshot::restore`).
    SetSnapshots(Vec<crate::UvSnapshot>),
    /// Gives an object a new list of materials (decision 0074): names
    /// trimmed, at most `mesh::material::MAX_SLOTS`. A list with an empty
    /// name or a colour outside 0 to 1 is refused.
    SetMaterials {
        /// The object.
        id: ObjectId,
        /// Its materials.
        materials: Vec<crate::Material>,
    },
    /// Bakes each shape's base and recorded edits into a plain mesh, which
    /// becomes its new base (its settings are gone; its edits are part of
    /// the mesh). Groups, and meshes with nothing recorded, are left alone.
    /// An instance gets its own copy of its original's shape.
    MakeEditable(Vec<ObjectId>),
    /// Applies each object's live mirror (decision 0126, as Blender's
    /// Apply on a Mirror modifier): the mirrored copies, welded at the
    /// middle, become part of a plain mesh that is its new base (its
    /// edits are baked in too), and the mirror is taken away, so turning
    /// symmetry off later keeps both halves. Objects without a mirror are
    /// left alone.
    ApplyMirror(Vec<ObjectId>),
    /// Copies objects, with everything inside them, each copy placed just
    /// after its original and named like it (decision 0055). With
    /// `instances`, each shape's copy is an instance of it instead: it
    /// shows the original's shape, and follows its changes. The copies
    /// become the selection.
    Duplicate {
        /// The objects. Any inside another one listed is copied with it.
        ids: Vec<ObjectId>,
        /// Instances rather than independent copies.
        instances: bool,
    },
    /// Moves polygons of a shape into a new plain-mesh object of their
    /// own, placed just after it, in the same place (decision 0055). They
    /// are deleted from the shape as a recorded edit. The new object
    /// becomes the selection.
    SplitOff {
        /// The shape.
        id: ObjectId,
        /// Its polygons to split off.
        faces: Vec<mesh::FaceId>,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<mesh::Anchors>,
    },
    /// Cuts `kept` with `cutters` (decision 0135): a live boolean, worked
    /// out whenever the shape is shown, so moving or editing a cutter
    /// moves the cut. Added to the kept object's last boolean if that is
    /// of the same `kind`, else as a new one. Refused (with a reason) for
    /// a group or an instance, a shape cutting itself, a loop (a cutter
    /// that is cut by the kept shape), or no cutter. The kept object
    /// becomes the selection.
    AddBoolean {
        /// The object cut.
        kept: ObjectId,
        /// The cutters, in order.
        cutters: Vec<ObjectId>,
        /// What they do.
        kind: crate::BooleanKind,
    },
    /// Replaces an object's booleans (decision 0135): to change a kind, or
    /// take a boolean or a cutter away. Checked as [`Command::AddBoolean`]
    /// is; booleans without cutters are left out.
    SetBooleans {
        /// The object cut.
        id: ObjectId,
        /// Its booleans.
        booleans: Vec<crate::Boolean>,
    },
    /// Adds a modifier of `kind` at the end of each object's modifiers
    /// (decision 0142), on and shown in the viewport, its settings brought
    /// within limits. Refused, changing nothing, if any object can't take
    /// it (a group, an instance, too many modifiers).
    AddModifier {
        /// The objects.
        ids: Vec<ObjectId>,
        /// The modifier and its settings.
        kind: crate::ModifierKind,
    },
    /// Changes the modifier at `index` of an object's list (decision
    /// 0142): its settings, its on/off switch or whether the viewport
    /// shows it. It must stay the same kind.
    ChangeModifier {
        /// The object.
        id: ObjectId,
        /// Its place in the list, from the first.
        index: usize,
        /// The modifier as it should be.
        modifier: crate::Modifier,
    },
    /// Takes the modifier at `index` off an object (decision 0142).
    RemoveModifier {
        /// The object.
        id: ObjectId,
        /// Its place in the list, from the first.
        index: usize,
    },
    /// Moves an object's modifier from place `from` to place `to` in its
    /// list (decision 0142): the order they are worked out in.
    MoveModifier {
        /// The object.
        id: ObjectId,
        /// Where it is.
        from: usize,
        /// Where it goes.
        to: usize,
    },
    /// Changes a curve's points (decision 0138): moves, deletes, makes
    /// smooth or sharp, closes or opens. A ready-made curve becomes points
    /// first. Refused for an object that isn't a curve, or a change that
    /// would leave fewer than two points.
    EditCurve {
        /// The curve.
        id: ObjectId,
        /// The change.
        edit: mesh::curve::CurveEdit,
    },
    /// Gives curves a solid (decision 0138): none (a line), a sweep or a
    /// lathe, with its settings (brought within limits). A sweep's
    /// cross-section from another curve must be a curve object other than
    /// the curve itself.
    SetCurveSolid {
        /// The curves.
        ids: Vec<ObjectId>,
        /// The solid.
        solid: crate::CurveSolid,
    },
    /// Gives ready-made curves new settings (brought within limits), each
    /// of the same kind as `ready`.
    SetReadyCurve {
        /// The curves.
        ids: Vec<ObjectId>,
        /// Their new settings.
        ready: mesh::curve::ReadyCurve,
    },
    /// Joins two curve points that are ends of open curves (decision
    /// 0139): `weld` melts them into one at their middle, else a straight
    /// stretch joins them. On one curve, its two ends: it closes. On two,
    /// the second joins onto the first, which stays (with its settings);
    /// the second is deleted. Refused for points that aren't open ends.
    JoinCurveEnds {
        /// The first curve and its end point.
        first: (ObjectId, mesh::curve::CurvePointId),
        /// The second curve (or the same) and its end point.
        second: (ObjectId, mesh::curve::CurvePointId),
        /// Melt the ends into one, or join them with a stretch.
        weld: bool,
    },
    /// Cuts a curve in pieces at its points `points` (decision 0139): the
    /// curve keeps the first piece, and each other becomes a new curve
    /// object beside it with the same settings, transform and materials.
    /// They become the selection.
    SplitCurve {
        /// The curve.
        id: ObjectId,
        /// Where to cut.
        points: Vec<mesh::curve::CurvePointId>,
    },
    /// Joins shapes into the first of them, which becomes a plain mesh
    /// holding them all where they were; the others are deleted (anything
    /// inside them takes their place) (decision 0055). Groups listed are
    /// left alone. It becomes the selection. When every object listed is a
    /// curve, they join into the first as one curve instead, end to end at
    /// their nearest ends (decision 0139).
    Join(Vec<ObjectId>),
    /// Adds a lattice cage (decision 0144) with `divisions` points along
    /// each side (brought within 2 to 16), fitted round the shapes `ids`,
    /// and gives each of them a Lattice modifier naming it, as one step.
    /// Refused, changing nothing, if any isn't a shape that can take
    /// modifiers. The cage becomes the selection.
    AddLattice {
        /// The shapes.
        ids: Vec<ObjectId>,
        /// Points along each side (X, Y, Z).
        divisions: [u32; 3],
    },
    /// Changes a lattice cage (decision 0144): moves its points, changes
    /// its divisions or interpolation, or puts its points back at rest.
    EditLattice {
        /// The cage.
        id: ObjectId,
        /// The change.
        edit: mesh::lattice::LatticeEdit,
    },
}

impl Command {
    /// A short description for the Edit menu, such as "Delete 3 objects".
    pub fn describe(&self, scene: &Scene) -> String {
        if let Some(label) = modifiers::label(scene, self) {
            return label;
        }
        if let Some(label) = lattices::label(self) {
            return label;
        }
        let count = |ids: &[ObjectId]| top_level(scene, ids).len();
        let objects = |n: usize| {
            if n == 1 {
                tr("object").to_string()
            } else {
                trf!("{} objects", n)
            }
        };
        match self {
            Self::Create(kind) => trf!("Create {}", kind.name().to_lowercase()),
            Self::Insert { object, .. } => trf!("Add {}", object.name),
            Self::InsertMany(list) => match list.as_slice() {
                [one] => trf!("Add {}", one.name),
                many => trf!("Add {} objects", many.len()),
            },
            Self::Rename { .. } => tr("Rename").into(),
            Self::SetVisible { ids, visible } => {
                if *visible {
                    trf!("Show {}", objects(count(ids)))
                } else {
                    trf!("Hide {}", objects(count(ids)))
                }
            }
            Self::Move { ids, .. } => trf!("Move {}", objects(count(ids))),
            Self::Delete(ids) => trf!("Delete {}", objects(count(ids))),
            Self::Group(ids) => trf!("Group {}", objects(count(ids))),
            Self::SetPrimitive { primitive, .. } => {
                trf!("Change {} settings", primitive.name().to_lowercase())
            }
            Self::Record { operation, .. } => operation.describe(),
            Self::ReplaceOperation { operation, .. } => trf!("Change {}", operation.describe()),
            Self::SetEditEnabled { enabled, .. } => {
                if *enabled {
                    tr("Switch edit on").into()
                } else {
                    tr("Switch edit off").into()
                }
            }
            Self::SetEditPinned { pinned, .. } => {
                if *pinned {
                    tr("Pin edit").into()
                } else {
                    tr("Unpin edit").into()
                }
            }
            Self::SetTransforms(changes) => {
                let ids: Vec<ObjectId> = changes.iter().map(|(id, _)| *id).collect();
                trf!("Transform {}", objects(count(&ids)))
            }
            Self::SetUnits(units) => trf!("Change units to {}", units.name()),
            Self::SetMaterials { .. } => tr("Change materials").to_string(),
            Self::ApplyTransforms { parts, .. } => trf!("Apply {}", parts.name()),
            Self::AddLayer(_) => tr("Add edit layer").to_string(),
            Self::DeleteEdit { id, edit } => scene
                .object(*id)
                .and_then(|o| o.stack.entry(*edit))
                .map_or_else(
                    || tr("Delete edit").to_string(),
                    |(_, _, e)| trf!("Delete {}", e.operation.describe().to_lowercase()),
                ),
            Self::Freeze { id, edit } => {
                let n = scene
                    .object(*id)
                    .map_or(0, |o| crate::stack::frozen_count(&o.stack, *edit));
                match n {
                    1 => tr("Freeze 1 edit").to_string(),
                    n => trf!("Freeze {} edits", n),
                }
            }
            Self::ChangeLayer { change, .. } => change.describe().to_string(),
            Self::SetMirror { mirror: None, .. } => tr("Mirror off").to_string(),
            Self::SetMirror { .. } => tr("Mirror").to_string(),
            Self::SetHardness {
                hardness: mesh::Hardness::Marked,
                ..
            } => tr("Smooth by marked edges only").to_string(),
            Self::SetHardness {
                hardness: mesh::Hardness::ByAngle(a),
                ..
            } => trf!("Smooth by angle ({:.0}°)", a),
            Self::SetCollision { collision, .. } => match collision {
                crate::Collision::None => tr("Remove collision").to_string(),
                c => trf!("Collision: {}", c.name().to_lowercase()),
            },
            Self::SetExportTarget(Some(_)) => tr("Change the game project folder").to_string(),
            Self::SetExportTarget(None) => tr("Clear the game project folder").to_string(),
            Self::SetTexture(_) => tr("Change the texture settings").to_string(),
            Self::SetMending(_) => tr("Change how edits keep UVs").to_string(),
            Self::SetTrim(_) => tr("Change the trim sheet").to_string(),
            Self::SetSnapshots(_) => tr("Change the layout snapshots").to_string(),
            Self::MovePivots(changes) => match changes.len() {
                1 => tr("Move pivot").to_string(),
                n => trf!("Move {} pivots", n),
            },
            Self::Duplicate { ids, instances } => {
                let n = count(ids);
                match (instances, n) {
                    (false, _) => trf!("Duplicate {}", objects(n)),
                    (true, 1) => tr("Instance 1 object").into(),
                    (true, n) => trf!("Instance {} objects", n),
                }
            }
            Self::SplitOff { faces, .. } => match faces.len() {
                1 => tr("Split off 1 polygon").into(),
                n => trf!("Split off {} polygons", n),
            },
            Self::Join(ids) => trf!("Join {}", objects(count(ids))),
            Self::AddBoolean { kind, .. } => kind.name().to_string(),
            Self::EditCurve { edit, .. } => curves::describe_edit(edit),
            Self::JoinCurveEnds { weld: true, .. } => tr("Weld curve ends").to_string(),
            Self::JoinCurveEnds { weld: false, .. } => tr("Connect curve ends").to_string(),
            Self::SplitCurve { .. } => tr("Split curve").to_string(),
            Self::SetCurveSolid { solid, .. } => match solid {
                crate::CurveSolid::Line => tr("Curve as a line only").to_string(),
                crate::CurveSolid::Sweep(_) => tr("Sweep").to_string(),
                crate::CurveSolid::Lathe(_) => tr("Lathe").to_string(),
            },
            Self::SetReadyCurve { ready, .. } => {
                trf!("Change {} settings", ready.name().to_lowercase())
            }
            Self::SetBooleans { .. } => tr("Change booleans").to_string(),
            Self::AddModifier { .. }
            | Self::ChangeModifier { .. }
            | Self::RemoveModifier { .. }
            | Self::MoveModifier { .. } => tr("Change modifiers").to_string(),
            Self::AddLattice { .. } | Self::EditLattice { .. } => tr("Lattice").to_string(),
            Self::ApplyMirror(ids) => {
                let n = ids
                    .iter()
                    .filter(|id| scene.object(**id).is_some_and(|o| o.mirror.is_some()))
                    .count();
                if n > 1 {
                    trf!("Apply {} mirrors", n)
                } else {
                    tr("Apply mirror").to_string()
                }
            }
            Self::MakeEditable(ids) => {
                let n = ids
                    .iter()
                    .filter(|id| scene.can_make_editable(**id))
                    .count();
                trf!("Make {} editable", objects(n.max(1)))
            }
            Self::Ungroup(ids) => match groups_among(scene, ids).len() {
                1 => tr("Ungroup").into(),
                n => trf!("Ungroup {} groups", n),
            },
        }
    }
}

/// What carrying out a command did.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Applied {
    /// The edits, in the order they were applied.
    pub(crate) edits: Vec<Edit>,
    /// What should be selected afterwards, if the command picks it.
    pub(crate) select: Option<Vec<ObjectId>>,
}

/// Carries out `command` on `scene`. On failure the scene is left exactly
/// as it was.
pub(crate) fn apply(scene: &mut Scene, command: &Command) -> Result<Applied, SceneError> {
    let mut run = Run {
        scene,
        edits: Vec::new(),
    };
    match run.command(command) {
        Ok(select) => Ok(Applied {
            edits: run.edits,
            select,
        }),
        Err(error) => {
            run.roll_back();
            Err(error)
        }
    }
}

/// A command being carried out: the scene, and the edits made so far.
struct Run<'a> {
    scene: &'a mut Scene,
    edits: Vec<Edit>,
}

impl Run<'_> {
    fn edit(&mut self, edit: Edit) -> Result<(), SceneError> {
        self.scene.apply(&edit)?;
        self.edits.push(edit);
        Ok(())
    }

    /// Puts `after` in place of `before` (at `layer`, `index`).
    fn swap_entry(
        &mut self,
        id: ObjectId,
        layer: usize,
        index: usize,
        before: Entry,
        after: Entry,
    ) -> Result<(), SceneError> {
        self.edit(Edit::RemoveEntry {
            id,
            layer,
            index,
            entry: before,
        })?;
        self.edit(Edit::InsertEntry {
            id,
            layer,
            index,
            entry: after,
        })
    }

    /// Changes the switches of edit `edit` of object `id` with `change`;
    /// nothing is recorded if they are already so.
    fn change_entry(
        &mut self,
        id: ObjectId,
        edit: EditId,
        change: impl FnOnce(&mut Entry),
    ) -> Result<(), SceneError> {
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        let (layer, index, before) = object
            .stack
            .entry(edit)
            .ok_or(SceneError::History(text!("that edit no longer exists")))?;
        let before = before.clone();
        let mut after = before.clone();
        change(&mut after);
        if after == before {
            return Ok(());
        }
        self.swap_entry(id, layer, index, before, after)
    }

    /// Freezes `id`'s edits up to and including `edit` (decision 0092).
    fn freeze(&mut self, id: ObjectId, edit: EditId) -> Result<(), SceneError> {
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        if matches!(object.kind, ObjectKind::Instance(_)) {
            return Err(SceneError::Instance);
        }
        if !object.kind.has_shape() {
            return Err(SceneError::NoShape);
        }
        let (layer, index, _) = object
            .stack
            .entry(edit)
            .ok_or(SceneError::History(text!("that edit no longer exists")))?;
        let layers = object.stack.layers().to_vec();
        let mut frozen: Vec<Layer> = layers[..layer].to_vec();
        let mut last = layers[layer].clone();
        last.entries.truncate(index + 1);
        frozen.push(last);
        if let Some(problem) = object.stack.freeze_problem(edit) {
            return Err(SceneError::History(problem));
        }
        let context = self.scene.eval_context();
        let shape =
            crate::stack::evaluate_once(&object.kind, &Stack::from_layers(frozen), &context)
                .ok_or(SceneError::NoShape)?;
        let before = object.kind.clone();
        // The frozen edits of `edit`'s own layer, first to last...
        for entry in layers[layer].entries[..=index].iter().cloned() {
            self.edit(Edit::RemoveEntry {
                id,
                layer,
                index: 0,
                entry,
            })?;
        }
        // ...then the layers below it, now all frozen.
        for removed in layers[..layer].iter().cloned() {
            self.edit(Edit::RemoveLayer {
                id,
                index: 0,
                layer: removed,
            })?;
        }
        self.edit(Edit::SetKind {
            id,
            before,
            after: ObjectKind::Mesh(shape.mesh),
        })
    }

    /// Changes layer `index` of object `id` (decision 0091). A change
    /// that changes nothing records nothing.
    fn change_layer(
        &mut self,
        id: ObjectId,
        index: usize,
        change: &LayerChange,
    ) -> Result<(), SceneError> {
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        let layers = object.stack.layers().to_vec();
        let layer = layers
            .get(index)
            .cloned()
            .ok_or(SceneError::History(text!("that layer no longer exists")))?;
        let remove = |index: usize, layer: Layer| Edit::RemoveLayer { id, index, layer };
        let insert = |index: usize, layer: Layer| Edit::InsertLayer { id, index, layer };
        match change {
            LayerChange::SetEnabled(enabled) => {
                if layer.enabled != *enabled {
                    let after = Layer {
                        enabled: *enabled,
                        ..layer.clone()
                    };
                    self.edit(remove(index, layer))?;
                    self.edit(insert(index, after))?;
                }
            }
            LayerChange::Rename(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(SceneError::EmptyName);
                }
                if layer.name != name {
                    let after = Layer {
                        name: name.to_string(),
                        ..layer.clone()
                    };
                    self.edit(remove(index, layer))?;
                    self.edit(insert(index, after))?;
                }
            }
            LayerChange::Move { up } => {
                let to = if *up {
                    index + 1
                } else {
                    index.checked_sub(1).ok_or(SceneError::History(text!(
                        "the layer is already at the bottom"
                    )))?
                };
                if to >= layers.len() {
                    return Err(SceneError::History(text!(
                        "the layer is already at the top"
                    )));
                }
                self.edit(remove(index, layer.clone()))?;
                self.edit(insert(to, layer))?;
            }
            LayerChange::MergeDown => {
                let below_index = index.checked_sub(1).ok_or(SceneError::History(text!(
                    "there is no layer below to merge into"
                )))?;
                let below = layers[below_index].clone();
                if !layer.enabled || !below.enabled {
                    return Err(SceneError::History(text!(
                        "switch both layers on before merging them"
                    )));
                }
                let mut merged = below.clone();
                merged.entries.extend(layer.entries.iter().cloned());
                self.edit(remove(index, layer))?;
                self.edit(remove(below_index, below))?;
                self.edit(insert(below_index, merged))?;
            }
            LayerChange::Delete => self.edit(remove(index, layer))?,
        }
        Ok(())
    }

    /// Undoes the edits made so far.
    fn roll_back(&mut self) {
        while let Some(edit) = self.edits.pop() {
            // Undoing edits that were just applied cannot fail.
            let _ = self.scene.apply(&edit.inverse());
        }
    }

    /// Applies `parts` of `id`'s placement (decision 0089).
    fn apply_transform(
        &mut self,
        id: ObjectId,
        parts: crate::ApplyParts,
    ) -> Result<(), SceneError> {
        let object = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .clone();
        // An instance has no shape of its own; a lattice's box is its own
        // space, so its placement stays (decision 0144).
        if matches!(
            object.kind,
            ObjectKind::Instance(_) | ObjectKind::Lattice(_)
        ) {
            return Ok(());
        }
        let before = object.transform;
        let after = parts.kept(before);
        if after == before {
            return Ok(());
        }
        let new = after.matrix();
        if new.determinant().abs() < 1e-12 || !new.is_finite() {
            return Err(SceneError::NotANumber);
        }
        // From the old placement's space into the new one's.
        let shift = new.inverse() * before.matrix();
        if matches!(object.kind, ObjectKind::Curve(_)) {
            // A curve's points take the change (decision 0138).
            self.transform_curve(id, shift * object.offset.matrix())?;
            if object.offset != Transform::default() {
                self.edit(Edit::SetOffset {
                    id,
                    before: object.offset,
                    after: Transform::default(),
                })?;
            }
        } else if object.kind.has_shape() {
            let matrix = shift * object.offset.matrix();
            if !matrix.abs_diff_eq(glam::Mat4::IDENTITY, 1e-6) {
                self.command(&Command::Record {
                    id,
                    operation: Operation::PlaceShape { matrix },
                })?;
            }
            if object.offset != Transform::default() {
                self.edit(Edit::SetOffset {
                    id,
                    before: object.offset,
                    after: Transform::default(),
                })?;
            }
        }
        let children: Vec<(ObjectId, Transform)> = self
            .scene
            .children(Some(id))
            .iter()
            .filter_map(|child| {
                let t = self.scene.object(*child)?.transform;
                Some((*child, Transform::from_matrix(shift * t.matrix())?))
            })
            .collect();
        self.edit(Edit::SetTransform { id, before, after })?;
        for (child, t) in children {
            let before = self
                .scene
                .object(child)
                .ok_or(SceneError::UnknownObject(child))?
                .transform;
            if before != t {
                self.edit(Edit::SetTransform {
                    id: child,
                    before,
                    after: t,
                })?;
            }
        }
        Ok(())
    }

    /// Gives `id` the transform `after`, its pivot moving there on its
    /// own: its shape's offset and its children's transforms change the
    /// opposite way, so nothing moves in the world.
    fn move_pivot(&mut self, id: ObjectId, after: Transform) -> Result<(), SceneError> {
        if !after.is_finite() {
            return Err(SceneError::NotANumber);
        }
        let object = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .clone();
        let parent = self
            .scene
            .parent(id)
            .map_or(glam::Mat4::IDENTITY, |p| self.scene.world_matrix(p));
        let old = parent * object.transform.matrix();
        let new = parent * after.matrix();
        if new.determinant().abs() < 1e-12 {
            return Err(SceneError::NotANumber);
        }
        // From the old pivot's space into the new one's.
        let shift = new.inverse() * old;
        let offset =
            Transform::from_matrix(shift * object.offset.matrix()).ok_or(SceneError::NotANumber)?;
        let children: Vec<(ObjectId, Transform)> = self
            .scene
            .children(Some(id))
            .iter()
            .filter_map(|child| {
                let t = self.scene.object(*child)?.transform;
                Some((*child, Transform::from_matrix(shift * t.matrix())?))
            })
            .collect();
        if object.transform != after {
            self.edit(Edit::SetTransform {
                id,
                before: object.transform,
                after,
            })?;
        }
        if object.offset != offset {
            self.edit(Edit::SetOffset {
                id,
                before: object.offset,
                after: offset,
            })?;
        }
        for (child, t) in children {
            let before = self
                .scene
                .object(child)
                .ok_or(SceneError::UnknownObject(child))?
                .transform;
            if before != t {
                self.edit(Edit::SetTransform {
                    id: child,
                    before,
                    after: t,
                })?;
            }
        }
        Ok(())
    }

    fn place_of(&self, id: ObjectId) -> Result<Place, SceneError> {
        self.scene.place(id).ok_or(SceneError::UnknownObject(id))
    }

    /// Carries out the command; returns what to select, if it decides.
    fn command(&mut self, command: &Command) -> Result<Option<Vec<ObjectId>>, SceneError> {
        match command {
            Command::Create(kind) => {
                let mut object = Object::new(kind.clone());
                object.name = self.scene.unused_name(kind.name());
                let id = self.insert(
                    object,
                    Place {
                        parent: None,
                        index: 0,
                    },
                )?;
                Ok(Some(vec![id]))
            }
            Command::Insert { object, place } => {
                if object.name.trim().is_empty() {
                    return Err(SceneError::EmptyName);
                }
                let id = self.insert(object.clone(), *place)?;
                Ok(Some(vec![id]))
            }
            Command::InsertMany(objects) => {
                let mut ids = Vec::with_capacity(objects.len());
                for object in objects {
                    let mut object = object.clone();
                    let base = object.name.trim().to_string();
                    if base.is_empty() {
                        return Err(SceneError::EmptyName);
                    }
                    object.name = self.scene.unused_name(&base);
                    let place = Place {
                        parent: None,
                        index: self.scene.children(None).len(),
                    };
                    ids.push(self.insert(object, place)?);
                }
                Ok(Some(ids))
            }
            Command::Rename { id, name } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(SceneError::EmptyName);
                }
                let before = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?
                    .name
                    .clone();
                if before != name {
                    self.edit(Edit::Rename {
                        id: *id,
                        before,
                        after: name.to_string(),
                    })?;
                }
                Ok(None)
            }
            Command::SetVisible { ids, visible } => {
                for id in existing(self.scene, ids)? {
                    let before = self.scene.object(id).is_some_and(|o| o.visible);
                    if before != *visible {
                        self.edit(Edit::SetVisible {
                            id,
                            before,
                            after: *visible,
                        })?;
                    }
                }
                Ok(None)
            }
            Command::Move { ids, parent, index } => {
                let ids = top_level(self.scene, &existing(self.scene, ids)?);
                self.move_into(&ids, *parent, *index)?;
                Ok(None)
            }
            Command::Delete(ids) => {
                let top = top_level(self.scene, &existing(self.scene, ids)?);
                // Instances of what goes, that stay themselves, get their
                // own copy of its shape first, so they don't vanish.
                let going: Vec<ObjectId> =
                    top.iter().flat_map(|id| self.scene.subtree(*id)).collect();
                let orphans: Vec<ObjectId> = self
                    .scene
                    .walk()
                    .into_iter()
                    .map(|(id, _)| id)
                    .filter(|id| !going.contains(id))
                    .filter(|id| {
                        self.scene.object(*id).is_some_and(
                            |o| matches!(o.kind, ObjectKind::Instance(s) if going.contains(&s)),
                        )
                    })
                    .collect();
                self.bake_instances(&orphans)?;
                // Deleted cutters stop cutting (decision 0135), and
                // deleted cross-sections give way to round ones (0138).
                self.forget_cutters(&going)?;
                self.forget_profiles(&going)?;
                self.forget_modifier_sources(&going)?;
                for id in top {
                    // Empty the object first, last child first, so every
                    // removal is of an object with nothing inside.
                    for inner in self.scene.subtree(id).into_iter().rev() {
                        let object = self
                            .scene
                            .object(inner)
                            .ok_or(SceneError::UnknownObject(inner))?
                            .clone();
                        let place = self.place_of(inner)?;
                        self.edit(Edit::Remove {
                            id: inner,
                            object,
                            place,
                        })?;
                    }
                }
                Ok(Some(Vec::new()))
            }
            Command::Group(ids) => {
                let ids = top_level(self.scene, &existing(self.scene, ids)?);
                let Some(first) = ids.first() else {
                    return Ok(None);
                };
                let place = self.place_of(*first)?;
                let mut group = Object::new(ObjectKind::Group);
                group.name = self.scene.unused_name(ObjectKind::Group.name());
                let group_id = self.insert(group, place)?;
                self.move_into(&ids, Some(group_id), 0)?;
                Ok(Some(vec![group_id]))
            }
            Command::SetPrimitive { ids, primitive } => {
                let after = ObjectKind::Primitive(primitive.clamped());
                for id in existing(self.scene, ids)? {
                    let before = self
                        .scene
                        .object(id)
                        .ok_or(SceneError::UnknownObject(id))?
                        .kind
                        .clone();
                    match &before {
                        ObjectKind::Primitive(p) if p.kind() == primitive.kind() => {}
                        _ => return Err(SceneError::WrongKind(primitive.name())),
                    }
                    if before != after {
                        self.edit(Edit::SetKind {
                            id,
                            before,
                            after: after.clone(),
                        })?;
                    }
                }
                Ok(None)
            }
            Command::Record { id, operation } => {
                if !operation.numbers_are_real() {
                    return Err(SceneError::NotANumber);
                }
                let object = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                if !object.kind.has_shape() {
                    return Err(SceneError::NoShape);
                }
                if matches!(object.kind, ObjectKind::Instance(_)) {
                    return Err(SceneError::Instance);
                }
                if matches!(object.kind, ObjectKind::Curve(_)) {
                    return Err(SceneError::Curve(crate::curve::RECORD_ON_CURVE));
                }
                // The top switched-on layer, or a new one on top
                // (decision 0091).
                let layer = match object.stack.recording_layer() {
                    Some(layer) => layer,
                    None => {
                        let index = object.stack.layers().len();
                        let layer = Layer::new(object.stack.next_layer_name());
                        self.edit(Edit::InsertLayer {
                            id: *id,
                            index,
                            layer,
                        })?;
                        index
                    }
                };
                let stack = self.scene.object(*id).map(|o| &o.stack);
                let index = stack.map_or(0, |s| s.layers()[layer].entries.len());
                let edit = stack.map_or(EditId::from_raw(1), crate::stack::Stack::next_edit_id);
                self.edit(Edit::InsertEntry {
                    id: *id,
                    layer,
                    index,
                    entry: Entry::new(edit, operation.clone()),
                })?;
                Ok(None)
            }
            Command::ReplaceOperation {
                id,
                layer,
                index,
                operation,
            } => {
                if !operation.numbers_are_real() {
                    return Err(SceneError::NotANumber);
                }
                let object = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                let before = object
                    .stack
                    .layers()
                    .get(*layer)
                    .and_then(|l| l.entries.get(*index))
                    .cloned()
                    .ok_or(SceneError::History(text!("that edit no longer exists")))?;
                if before.operation == *operation {
                    return Ok(None);
                }
                // The same edit (id and switches) with new settings.
                let after = Entry {
                    operation: operation.clone(),
                    ..before.clone()
                };
                self.swap_entry(*id, *layer, *index, before, after)?;
                Ok(None)
            }
            Command::AddLayer(ids) => {
                for id in ids {
                    let object = self
                        .scene
                        .object(*id)
                        .ok_or(SceneError::UnknownObject(*id))?;
                    if !object.kind.has_shape()
                        || matches!(object.kind, ObjectKind::Instance(_) | ObjectKind::Curve(_))
                    {
                        continue;
                    }
                    let index = object.stack.layers().len();
                    let layer = Layer::new(object.stack.next_layer_name());
                    self.edit(Edit::InsertLayer {
                        id: *id,
                        index,
                        layer,
                    })?;
                }
                Ok(None)
            }
            Command::DeleteEdit { id, edit } => {
                let object = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?;
                let (layer, index, entry) = object
                    .stack
                    .entry(*edit)
                    .ok_or(SceneError::History(text!("that edit no longer exists")))?;
                let entry = entry.clone();
                self.edit(Edit::RemoveEntry {
                    id: *id,
                    layer,
                    index,
                    entry,
                })?;
                Ok(None)
            }
            Command::Freeze { id, edit } => {
                self.freeze(*id, *edit)?;
                Ok(None)
            }
            Command::ChangeLayer { id, index, change } => {
                self.change_layer(*id, *index, change)?;
                Ok(None)
            }
            Command::SetEditEnabled { id, edit, enabled } => {
                self.change_entry(*id, *edit, |e| e.enabled = *enabled)?;
                Ok(None)
            }
            Command::SetEditPinned { id, edit, pinned } => {
                self.change_entry(*id, *edit, |e| e.pinned = *pinned)?;
                Ok(None)
            }
            Command::SetTransforms(changes) => {
                for (id, after) in changes {
                    if !after.is_finite() {
                        return Err(SceneError::NotANumber);
                    }
                    let before = self
                        .scene
                        .object(*id)
                        .ok_or(SceneError::UnknownObject(*id))?
                        .transform;
                    if before != *after {
                        self.edit(Edit::SetTransform {
                            id: *id,
                            before,
                            after: *after,
                        })?;
                    }
                }
                Ok(None)
            }
            Command::SetMaterials { id, materials } => {
                let before = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?
                    .materials
                    .clone();
                let after: Vec<crate::Material> = materials
                    .iter()
                    .map(|m| crate::Material {
                        name: m.name.trim().to_string(),
                        color: m.color,
                    })
                    .collect();
                if after.len() > mesh::material::MAX_SLOTS as usize {
                    return Err(SceneError::TooManyMaterials);
                }
                if !after.iter().all(crate::Material::is_valid) {
                    return Err(SceneError::EmptyName);
                }
                if before != after {
                    self.edit(Edit::SetMaterials {
                        id: *id,
                        before,
                        after,
                    })?;
                }
                Ok(None)
            }
            Command::ApplyTransforms { ids, parts } => {
                let ids = existing(self.scene, ids)?;
                // Parents first: their children are placed again as they
                // go, then applied themselves.
                let order: Vec<ObjectId> = self
                    .scene
                    .walk()
                    .into_iter()
                    .map(|(id, _)| id)
                    .filter(|id| ids.contains(id))
                    .collect();
                for id in order {
                    self.apply_transform(id, *parts)?;
                }
                Ok(None)
            }
            Command::SetMirror { ids, mirror } => {
                if mirror.is_some_and(|m| !m.is_valid()) {
                    return Err(SceneError::NotANumber);
                }
                for id in ids {
                    let object = self
                        .scene
                        .object(*id)
                        .ok_or(SceneError::UnknownObject(*id))?;
                    if !object.kind.has_shape() || object.mirror == *mirror {
                        continue;
                    }
                    let before = object.mirror;
                    self.edit(Edit::SetMirror {
                        id: *id,
                        before,
                        after: *mirror,
                    })?;
                }
                Ok(None)
            }
            Command::SetHardness { ids, hardness } => {
                if !crate::object::hardness_is_valid(*hardness) {
                    return Err(SceneError::NotANumber);
                }
                for id in ids {
                    let object = self
                        .scene
                        .object(*id)
                        .ok_or(SceneError::UnknownObject(*id))?;
                    if !object.kind.has_shape() || object.hardness == *hardness {
                        continue;
                    }
                    let before = object.hardness;
                    self.edit(Edit::SetHardness {
                        id: *id,
                        before,
                        after: *hardness,
                    })?;
                }
                Ok(None)
            }
            Command::SetCollision { ids, collision } => {
                for id in ids {
                    let object = self
                        .scene
                        .object(*id)
                        .ok_or(SceneError::UnknownObject(*id))?;
                    if !object.kind.has_shape() || object.collision == *collision {
                        continue;
                    }
                    let before = object.collision;
                    self.edit(Edit::SetCollision {
                        id: *id,
                        before,
                        after: *collision,
                    })?;
                }
                Ok(None)
            }
            Command::SetExportTarget(target) => {
                if target.as_ref().is_some_and(|t| !t.is_valid()) {
                    return Err(SceneError::EmptyName);
                }
                let before = self.scene.export_target().cloned();
                if before != *target {
                    self.edit(Edit::SetExportTarget {
                        before,
                        after: target.clone(),
                    })?;
                }
                Ok(None)
            }
            Command::SetTexture(texture) => {
                if !texture.is_valid() {
                    return Err(SceneError::NotANumber);
                }
                let before = self.scene.texture();
                if before != *texture {
                    self.edit(Edit::SetTexture {
                        before,
                        after: *texture,
                    })?;
                }
                Ok(None)
            }
            Command::SetMending(mending) => {
                if !mending.is_real() {
                    return Err(SceneError::NotANumber);
                }
                let before = self.scene.mending();
                if before != *mending {
                    self.edit(Edit::SetMending {
                        before,
                        after: *mending,
                    })?;
                }
                Ok(None)
            }
            Command::SetSnapshots(list) => {
                if !crate::snapshot::snapshots_are_valid(list) {
                    return Err(SceneError::NotANumber);
                }
                let before = self.scene.snapshots().to_vec();
                if before != *list {
                    self.edit(Edit::SetSnapshots {
                        before,
                        after: list.clone(),
                    })?;
                }
                Ok(None)
            }
            Command::SetTrim(trim) => {
                if !trim.is_valid() {
                    return Err(SceneError::NotANumber);
                }
                let before = self.scene.trim().clone();
                if before != *trim {
                    self.edit(Edit::SetTrim {
                        before,
                        after: trim.clone(),
                    })?;
                }
                Ok(None)
            }
            Command::SetUnits(units) => {
                let before = self.scene.units();
                if before != *units {
                    self.edit(Edit::SetUnits {
                        before,
                        after: *units,
                    })?;
                }
                Ok(None)
            }
            Command::MovePivots(changes) => {
                for (id, after) in changes {
                    self.move_pivot(*id, *after)?;
                }
                Ok(None)
            }
            Command::Duplicate { ids, instances } => {
                let mut copies = Vec::new();
                let mut copied = Vec::new();
                for id in top_level(self.scene, &existing(self.scene, ids)?) {
                    let place = self.place_of(id)?;
                    let copy = self.copy_tree(
                        id,
                        Place {
                            parent: place.parent,
                            index: place.index + 1,
                        },
                        *instances,
                        &mut copied,
                    )?;
                    copies.push(copy);
                }
                self.follow_copied_cutters(&copied)?;
                self.follow_copied_profiles(&copied)?;
                self.follow_copied_sources(&copied)?;
                Ok(Some(copies))
            }
            Command::SplitOff { id, faces, anchors } => {
                let object = self
                    .scene
                    .object(*id)
                    .ok_or(SceneError::UnknownObject(*id))?
                    .clone();
                if matches!(object.kind, ObjectKind::Instance(_)) {
                    return Err(SceneError::Instance);
                }
                let shape = self.scene.evaluated(*id).ok_or(SceneError::NoShape)?;
                let part = mesh::parts::keep_only(&shape.mesh, faces);
                if part.face_count() == 0 {
                    return Ok(None);
                }
                let mut piece = Object::new(ObjectKind::Mesh(Arc::new(part)));
                piece.name = self.scene.unused_name(&object.name);
                piece.transform = object.transform;
                piece.offset = object.offset;
                piece.visible = object.visible;
                let place = self.place_of(*id)?;
                self.command(&Command::Record {
                    id: *id,
                    operation: Operation::DeleteElements {
                        points: Vec::new(),
                        edges: Vec::new(),
                        faces: faces.clone(),
                        anchors: anchors.clone(),
                    },
                })?;
                let new = self.insert(
                    piece,
                    Place {
                        parent: place.parent,
                        index: place.index + 1,
                    },
                )?;
                Ok(Some(vec![new]))
            }
            Command::AddBoolean {
                kept,
                cutters,
                kind,
            } => self.add_boolean(*kept, cutters, *kind),
            Command::SetBooleans { id, booleans } => {
                self.set_booleans(*id, booleans.clone())?;
                Ok(None)
            }
            Command::AddModifier { ids, kind } => {
                self.add_modifier(ids, kind)?;
                Ok(None)
            }
            Command::ChangeModifier {
                id,
                index,
                modifier,
            } => {
                self.change_modifier(*id, *index, modifier)?;
                Ok(None)
            }
            Command::RemoveModifier { id, index } => {
                self.remove_modifier(*id, *index)?;
                Ok(None)
            }
            Command::MoveModifier { id, from, to } => {
                self.move_modifier(*id, *from, *to)?;
                Ok(None)
            }
            Command::EditCurve { id, edit } => {
                self.edit_curve(*id, edit)?;
                Ok(None)
            }
            Command::SetCurveSolid { ids, solid } => {
                self.set_curve_solid(ids, solid)?;
                Ok(None)
            }
            Command::SetReadyCurve { ids, ready } => {
                self.set_ready_curve(ids, ready)?;
                Ok(None)
            }
            Command::JoinCurveEnds {
                first,
                second,
                weld,
            } => self.join_curve_ends(*first, *second, *weld),
            Command::SplitCurve { id, points } => self.split_curve(*id, points),
            Command::AddLattice { ids, divisions } => self.add_lattice(ids, *divisions),
            Command::EditLattice { id, edit } => {
                self.edit_lattice(*id, edit)?;
                Ok(None)
            }
            Command::Join(ids) if self.all_curves(ids) => self.join_curves(ids),
            Command::Join(ids) => {
                let shapes: Vec<ObjectId> = existing(self.scene, ids)?
                    .into_iter()
                    .filter(|id| self.scene.evaluated(*id).is_some())
                    .collect();
                let Some((&target, others)) = shapes.split_first() else {
                    return Ok(None);
                };
                if others.is_empty() {
                    return Ok(None);
                }
                let into = self.scene.shape_matrix(target).inverse();
                // Materials join by name: each part's slots are renumbered
                // into the target's list, which gains the names it lacks
                // (decision 0074).
                let list = |id: ObjectId| {
                    self.scene
                        .object(id)
                        .map(crate::Object::material_list)
                        .unwrap_or_default()
                };
                let any_materials = shapes.iter().any(|id| {
                    self.scene
                        .object(*id)
                        .is_some_and(|o| !o.materials.is_empty())
                });
                let mut merged = list(target);
                let meshes: Vec<(Arc<mesh::Mesh>, glam::Mat4)> = shapes
                    .iter()
                    .filter_map(|id| {
                        let shape = self.scene.evaluated(*id)?;
                        let mut part = shape.mesh;
                        if any_materials && *id != target {
                            let own = list(*id);
                            let renumber: Vec<u32> = own
                                .iter()
                                .map(|m| {
                                    let at = merged.iter().position(|n| n.name == m.name);
                                    at.unwrap_or_else(|| {
                                        merged.push(m.clone());
                                        merged.len() - 1
                                    }) as u32
                                })
                                .collect();
                            let mut changed = (*part).clone();
                            for (old, new) in renumber.iter().enumerate() {
                                if old as u32 != *new {
                                    let faces = mesh::material::faces_with_slot(&part, old as u32);
                                    mesh::material::set_material(&mut changed, &faces, *new);
                                }
                            }
                            part = Arc::new(changed);
                        }
                        Some((part, into * self.scene.shape_matrix(*id)))
                    })
                    .collect();
                if any_materials {
                    merged.truncate(mesh::material::MAX_SLOTS as usize);
                    let before = self
                        .scene
                        .object(target)
                        .ok_or(SceneError::UnknownObject(target))?
                        .materials
                        .clone();
                    if before != merged {
                        self.edit(Edit::SetMaterials {
                            id: target,
                            before,
                            after: merged,
                        })?;
                    }
                }
                let parts: Vec<(&mesh::Mesh, glam::Mat4)> =
                    meshes.iter().map(|(m, x)| (m.as_ref(), *x)).collect();
                let joined = mesh::parts::combine(&parts);
                // The target becomes the joined mesh, its edits baked in.
                let object = self
                    .scene
                    .object(target)
                    .ok_or(SceneError::UnknownObject(target))?;
                let before = object.kind.clone();
                let layers = object.stack.layers().to_vec();
                for (index, layer) in layers.into_iter().enumerate().rev() {
                    self.edit(Edit::RemoveLayer {
                        id: target,
                        index,
                        layer,
                    })?;
                }
                self.edit(Edit::SetKind {
                    id: target,
                    before,
                    after: ObjectKind::Mesh(Arc::new(joined)),
                })?;
                // The others go; anything inside them takes their place.
                for other in others {
                    if !self.scene.contains(*other) {
                        continue;
                    }
                    let place = self.place_of(*other)?;
                    let children = self.scene.children(Some(*other)).to_vec();
                    if !children.is_empty() {
                        self.move_into(&children, place.parent, place.index)?;
                    }
                    self.command(&Command::Delete(vec![*other]))?;
                }
                Ok(Some(vec![target]))
            }
            Command::ApplyMirror(ids) => {
                for id in existing(self.scene, ids)? {
                    let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
                    let Some(mirror) = object.mirror else {
                        continue;
                    };
                    if !object.kind.has_shape() {
                        continue;
                    }
                    // The shape as shown before its modifiers (which stay
                    // live, decision 0142): its edits, cut by its
                    // booleans, and the mirrored copies welded on.
                    let cut = self.cut_result(id).is_some();
                    let Some(shown) = self.scene.unmodified_shape(id) else {
                        continue;
                    };
                    let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
                    let before = object.kind.clone();
                    let layers = object.stack.layers().to_vec();
                    for (index, layer) in layers.into_iter().enumerate().rev() {
                        self.edit(Edit::RemoveLayer { id, index, layer })?;
                    }
                    self.edit(Edit::SetKind {
                        id,
                        before,
                        after: ObjectKind::Mesh(shown),
                    })?;
                    self.edit(Edit::SetMirror {
                        id,
                        before: Some(mirror),
                        after: None,
                    })?;
                    if cut {
                        self.end_booleans(id)?;
                    }
                }
                Ok(None)
            }
            Command::MakeEditable(ids) => {
                for id in existing(self.scene, ids)? {
                    if !self.scene.can_make_editable(id) {
                        continue;
                    }
                    if self.bakes_modifiers(id) {
                        self.bake_with_modifiers(id)?;
                        continue;
                    }
                    let Some(shape) = self.scene.evaluated(id) else {
                        continue;
                    };
                    // Booleans that work are baked in too, and an instance
                    // of a cut shape keeps the cut (decision 0135).
                    let cut = self.scene.cut_shape(id).and_then(Result::ok);
                    let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
                    let before = object.kind.clone();
                    let layers = object.stack.layers().to_vec();
                    // Top layer first, so every removal is of the top one.
                    for (index, layer) in layers.into_iter().enumerate().rev() {
                        self.edit(Edit::RemoveLayer { id, index, layer })?;
                    }
                    let baked = cut.is_some();
                    self.edit(Edit::SetKind {
                        id,
                        before,
                        after: ObjectKind::Mesh(cut.unwrap_or_else(|| Arc::clone(&shape.mesh))),
                    })?;
                    if baked {
                        self.end_booleans(id)?;
                    }
                }
                Ok(None)
            }
            Command::Ungroup(ids) => {
                existing(self.scene, ids)?;
                let groups = groups_among(self.scene, ids);
                if groups.is_empty() {
                    // Nothing to ungroup: never touch other objects.
                    return Ok(None);
                }
                let mut freed = Vec::new();
                for id in groups {
                    let place = self.place_of(id)?;
                    let children = self.scene.children(Some(id)).to_vec();
                    // Into the group's place, before the group itself.
                    self.move_into(&children, place.parent, place.index)?;
                    let object = self
                        .scene
                        .object(id)
                        .ok_or(SceneError::UnknownObject(id))?
                        .clone();
                    let place = self.place_of(id)?;
                    self.edit(Edit::Remove { id, object, place })?;
                    freed.extend(children);
                }
                Ok(Some(freed))
            }
        }
    }

    /// Copies `id` and everything inside it to `place`, as plain copies or
    /// (for shapes) instances. Returns the copy's id.
    fn copy_tree(
        &mut self,
        id: ObjectId,
        place: Place,
        instances: bool,
        copied: &mut Vec<(ObjectId, ObjectId)>,
    ) -> Result<ObjectId, SceneError> {
        let original = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .clone();
        let mut copy = original.clone();
        copy.name = self.scene.unused_name(&original.name);
        if instances && original.kind.has_shape() {
            // An instance of an instance shows the same original.
            let source = match original.kind {
                ObjectKind::Instance(source) => source,
                _ => id,
            };
            copy.kind = ObjectKind::Instance(source);
            copy.stack = crate::stack::Stack::default();
            // An instance shows its original's cut shape (decision 0135),
            // with its modifiers (decision 0142).
            copy.booleans.clear();
            copy.modifiers.clear();
        }
        let new = self.insert(copy, place)?;
        copied.push((id, new));
        let children = self.scene.children(Some(id)).to_vec();
        for (index, child) in children.into_iter().enumerate() {
            self.copy_tree(
                child,
                Place {
                    parent: Some(new),
                    index,
                },
                instances,
                copied,
            )?;
        }
        Ok(new)
    }

    /// Bakes `id`'s whole shape as an export gets it (decision 0142): its
    /// edits, booleans, live mirror and the modifiers that are on, into a
    /// plain mesh. The booleans that worked, the mirror and the modifiers
    /// that worked end; modifiers switched off, or that couldn't do their
    /// work, stay.
    fn bake_with_modifiers(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let cut = self.cut_result(id).is_some();
        let Some(result) = self.scene.final_result(id, crate::Purpose::Export) else {
            return Ok(());
        };
        let object = self.scene.object(id).ok_or(SceneError::UnknownObject(id))?;
        let before = object.kind.clone();
        let mirror = object.mirror;
        let layers = object.stack.layers().to_vec();
        for (index, layer) in layers.into_iter().enumerate().rev() {
            self.edit(Edit::RemoveLayer { id, index, layer })?;
        }
        self.edit(Edit::SetKind {
            id,
            before,
            after: ObjectKind::Mesh(result.mesh),
        })?;
        if cut {
            self.end_booleans(id)?;
        }
        if mirror.is_some() {
            self.edit(Edit::SetMirror {
                id,
                before: mirror,
                after: None,
            })?;
        }
        self.end_modifiers(id, &result.problems)
    }

    /// Gives each of `ids` (instances) its own copy of its original's
    /// final shape, as a plain mesh.
    fn bake_instances(&mut self, ids: &[ObjectId]) -> Result<(), SceneError> {
        for id in ids {
            let Some(shape) = self.scene.evaluated(*id) else {
                continue;
            };
            // An instance of a cut shape keeps the cut (decision 0135); of
            // a shape with modifiers, its whole final shape, its own
            // mirror included (decision 0142).
            let with_modifiers = self.bakes_modifiers(*id);
            let shape = if with_modifiers {
                match self.scene.exported_shape(*id) {
                    Some(mesh) => mesh,
                    None => shape.mesh,
                }
            } else {
                match self.scene.cut_shape(*id) {
                    Some(Ok(cut)) => cut,
                    _ => shape.mesh,
                }
            };
            let object = self
                .scene
                .object(*id)
                .ok_or(SceneError::UnknownObject(*id))?;
            let before = object.kind.clone();
            let mirror = object.mirror;
            let own_materials = object.materials.clone();
            // An instance shows its original's materials; its own copy
            // keeps them (decision 0074).
            let source_materials = match &before {
                ObjectKind::Instance(source) => self
                    .scene
                    .object(*source)
                    .map(|o| o.materials.clone())
                    .unwrap_or_default(),
                _ => own_materials.clone(),
            };
            self.edit(Edit::SetKind {
                id: *id,
                before,
                after: ObjectKind::Mesh(shape),
            })?;
            if with_modifiers && mirror.is_some() {
                self.edit(Edit::SetMirror {
                    id: *id,
                    before: mirror,
                    after: None,
                })?;
            }
            if own_materials != source_materials {
                self.edit(Edit::SetMaterials {
                    id: *id,
                    before: own_materials,
                    after: source_materials,
                })?;
            }
        }
        Ok(())
    }

    fn insert(&mut self, object: Object, place: Place) -> Result<ObjectId, SceneError> {
        let id = self.scene.next_id();
        self.edit(Edit::Insert { id, object, place })?;
        // Booleans naming objects that aren't (or can't be) its cutters
        // here lose them, so the scene always saves and opens again
        // (decision 0135).
        self.keep_usable_cutters(id)?;
        self.keep_usable_profile(id)?;
        self.keep_usable_modifiers(id)?;
        Ok(id)
    }

    /// Moves `ids` (top-level among themselves, in order) under `parent`,
    /// before the child now at `index`.
    fn move_into(
        &mut self,
        ids: &[ObjectId],
        parent: Option<ObjectId>,
        index: usize,
    ) -> Result<(), SceneError> {
        if let Some(p) = parent {
            if !self.scene.contains(p) {
                return Err(SceneError::UnknownObject(p));
            }
            if ids.iter().any(|id| self.scene.is_inside(p, *id)) {
                return Err(SceneError::NestedInItself);
            }
        }
        // The first child at or after `index` that isn't moving: everything
        // moved goes just before it, in order (or at the end, if none).
        let anchor = self
            .scene
            .children(parent)
            .iter()
            .skip(index)
            .find(|c| !ids.contains(c))
            .copied();
        for id in ids {
            let from = self.place_of(*id)?;
            let mut after_removal = self.scene.children(parent).to_vec();
            after_removal.retain(|c| c != id);
            let to_index = anchor
                .and_then(|a| after_removal.iter().position(|c| *c == a))
                .unwrap_or(after_removal.len());
            let to = Place {
                parent,
                index: to_index,
            };
            if to != from {
                self.edit(Edit::Relocate { id: *id, from, to })?;
            }
        }
        Ok(())
    }
}

/// `ids` without repeats, in tree order, failing if any doesn't exist.
fn existing(scene: &Scene, ids: &[ObjectId]) -> Result<Vec<ObjectId>, SceneError> {
    if let Some(missing) = ids.iter().find(|id| !scene.contains(**id)) {
        return Err(SceneError::UnknownObject(*missing));
    }
    Ok(scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| ids.contains(id))
        .collect())
}

/// The groups among `ids`, in tree order, leaving out any inside another
/// group listed (ungrouping the outer one frees them first).
pub(crate) fn groups_among(scene: &Scene, ids: &[ObjectId]) -> Vec<ObjectId> {
    let groups: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            scene
                .object(*id)
                .is_some_and(|o| o.kind == ObjectKind::Group)
        })
        .collect();
    top_level(scene, &groups)
}

/// `ids` in tree order, leaving out any that sit inside another one listed
/// (those go along with it anyway). Unknown ids are left out.
pub(crate) fn top_level(scene: &Scene, ids: &[ObjectId]) -> Vec<ObjectId> {
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| ids.contains(id))
        .filter(|id| {
            !ids.iter()
                .any(|other| other != id && scene.is_inside(*id, *other))
        })
        .collect()
}
