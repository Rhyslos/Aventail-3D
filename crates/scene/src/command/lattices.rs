//! The commands that add lattice cages and change their points
//! ([decision 0144](../../../../docs/decisions/0144-lattice-cage.md)).

use super::{Command, Run, existing};
use crate::lattice::LatticeOptions;
use crate::modifier::{Modifier, ModifierKind, ModifierRefusal, Purpose};
use crate::object::{Object, ObjectId, ObjectKind, Transform};
use crate::tree::{Edit, Place, Scene, SceneError};
use glam::{Mat4, Vec3};
use mesh::Bounds;
use mesh::lattice::{Lattice, LatticeEdit};
use words::tr;

/// The undo label for a lattice command.
pub(super) fn label(command: &Command) -> Option<String> {
    Some(match command {
        Command::AddLattice { ids, .. } if ids.len() > 1 => {
            words::trf!("Add lattice round {} objects", ids.len())
        }
        Command::AddLattice { .. } => tr("Add lattice").to_string(),
        Command::EditLattice { edit, .. } => match edit {
            LatticeEdit::Transform { points, .. } if points.len() == 1 => {
                tr("Move 1 cage point").to_string()
            }
            LatticeEdit::Transform { points, .. } => {
                words::trf!("Move {} cage points", points.len())
            }
            LatticeEdit::Divisions(_) => tr("Change lattice divisions").to_string(),
            LatticeEdit::Interpolation(_) => tr("Change lattice interpolation").to_string(),
            LatticeEdit::Reset => tr("Reset cage points").to_string(),
        },
        Command::FitCage { .. } => tr("Fit cage").to_string(),
        _ => return None,
    })
}

impl Run<'_> {
    /// Adds a lattice cage with `divisions` points along each side, fitted
    /// round the shapes `ids` (their shapes as the viewport shows them),
    /// and gives each a Lattice modifier naming it. Refused, changing
    /// nothing, if any of them isn't a shape that can take modifiers.
    /// Gives the cage, which becomes the selection.
    pub(super) fn add_lattice(
        &mut self,
        ids: &[ObjectId],
        divisions: [u32; 3],
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
        let ids = existing(self.scene, ids)?;
        if ids.is_empty() {
            return Err(SceneError::Modifier(ModifierRefusal::NotAShape));
        }
        for id in &ids {
            match self.scene.object(*id).map(|o| &o.kind) {
                Some(ObjectKind::Instance(_)) => {
                    return Err(SceneError::Modifier(ModifierRefusal::Instance));
                }
                Some(kind) if !kind.has_shape() => {
                    return Err(SceneError::Modifier(ModifierRefusal::NotAShape));
                }
                _ => {}
            }
        }
        let (lattice, place) = self.fit_cage(&ids, divisions);
        let mut object = Object::new(ObjectKind::Lattice(lattice));
        object.name = self.scene.unused_name(tr("Lattice"));
        object.transform = Transform::from_matrix(place).ok_or(SceneError::NotANumber)?;
        let cage = self.insert(
            object,
            Place {
                parent: None,
                index: 0,
            },
        )?;
        let kind = ModifierKind::Lattice(LatticeOptions { cage });
        for id in ids {
            // Where a new modifier of its kind goes: before Weighted
            // normals and Triangulate, which finish the shape.
            let mut after = self.scene.modifiers(id).to_vec();
            let at = after
                .iter()
                .position(|m| kind.goes_before(&m.kind))
                .unwrap_or(after.len());
            after.insert(at, Modifier::new(kind.clone()));
            self.scene
                .check_modifiers(id, &after)
                .map_err(SceneError::Modifier)?;
            let before = self.scene.modifiers(id).to_vec();
            self.edit(Edit::SetModifiers { id, before, after })?;
        }
        Ok(Some(vec![cage]))
    }

    /// A cage round `ids`, and where it goes in the world ([`fitted_cage`]).
    fn fit_cage(&self, ids: &[ObjectId], divisions: [u32; 3]) -> (Lattice, Mat4) {
        fitted_cage(self.scene, ids, divisions)
    }

    /// Fits cage `cage` round shape `shape` again: at rest, its divisions
    /// and interpolation kept, placed as a new cage round the shape would
    /// be (Size to kit grid, decision 0160, does this for a cage nobody
    /// has changed).
    pub(super) fn fit_cage_again(
        &mut self,
        cage: ObjectId,
        shape: ObjectId,
    ) -> Result<(), SceneError> {
        let object = self
            .scene
            .object(cage)
            .ok_or(SceneError::UnknownObject(cage))?;
        let ObjectKind::Lattice(old) = &object.kind else {
            return Err(SceneError::Lattice(crate::lattice::NOT_A_CAGE));
        };
        let before_kind = object.kind.clone();
        let before_place = object.transform;
        let (fitted, place) = self.fit_cage(&[shape], old.divisions());
        let fitted = fitted
            .edited(&LatticeEdit::Interpolation(old.interpolation()))
            .map_err(|why| SceneError::Lattice(crate::lattice::problem_reason(why)))?;
        let after_place = Transform::from_matrix(place).ok_or(SceneError::NotANumber)?;
        if ObjectKind::Lattice(fitted.clone()) != before_kind {
            self.edit(Edit::SetKind {
                id: cage,
                before: before_kind,
                after: ObjectKind::Lattice(fitted),
            })?;
        }
        if after_place != before_place {
            self.edit(Edit::SetTransform {
                id: cage,
                before: before_place,
                after: after_place,
            })?;
        }
        Ok(())
    }

    /// Changes cage `id` by `edit`.
    pub(super) fn edit_lattice(
        &mut self,
        id: ObjectId,
        edit: &LatticeEdit,
    ) -> Result<(), SceneError> {
        let before = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .kind
            .clone();
        let ObjectKind::Lattice(lattice) = &before else {
            return Err(SceneError::Lattice(crate::lattice::NOT_A_CAGE));
        };
        let edited = lattice
            .edited(edit)
            .map_err(|why| SceneError::Lattice(crate::lattice::problem_reason(why)))?;
        if edited == *lattice {
            return Ok(());
        }
        self.edit(Edit::SetKind {
            id,
            before,
            after: ObjectKind::Lattice(edited),
        })
    }
}

/// A cage round `ids`, and where it goes in the world. Round one shape
/// it lines up with the shape (turned and sized as it is); round several
/// it lines up with the world.
fn fitted_cage(scene: &Scene, ids: &[ObjectId], divisions: [u32; 3]) -> (Lattice, Mat4) {
    let shown = |id: ObjectId| {
        scene
            .final_shape(id, Purpose::Viewport)
            .and_then(|m| m.bounds())
    };
    let unit = || Bounds::around([Vec3::splat(-50.0), Vec3::splat(50.0)]);
    if let [id] = ids {
        let matrix = scene.shape_matrix(*id);
        if let Some(bounds) = shown(*id).or_else(unit) {
            let lattice = Lattice::fitted(bounds.size(), divisions);
            return (lattice, matrix * Mat4::from_translation(bounds.centre()));
        }
    }
    let world = ids
        .iter()
        .filter_map(|id| {
            let local = shown(*id)?;
            let matrix = scene.shape_matrix(*id);
            Bounds::around(local.corners().map(|c| matrix.transform_point3(c)))
        })
        .reduce(Bounds::union)
        .or_else(unit);
    match world {
        Some(bounds) => (
            Lattice::fitted(bounds.size(), divisions),
            Mat4::from_translation(bounds.centre()),
        ),
        None => (
            Lattice::fitted(Vec3::splat(100.0), divisions),
            Mat4::IDENTITY,
        ),
    }
}

impl Scene {
    /// Whether cage `cage` is as Add lattice left it round shape `shape`:
    /// its points at rest, at the top of the Objects panel's tree, used by
    /// no other shape, and the size and place a new cage round the shape
    /// would have. Such a cage can be fitted again when the shape changes
    /// size (Size to kit grid) without losing anything the person did.
    pub fn cage_fits(&self, cage: ObjectId, shape: ObjectId) -> bool {
        let Some(ObjectKind::Lattice(lattice)) = self.object(cage).map(|o| &o.kind) else {
            return false;
        };
        let used_elsewhere = self.walk().into_iter().any(|(other, _)| {
            other != shape
                && self
                    .modifiers(other)
                    .iter()
                    .any(|m| matches!(&m.kind, ModifierKind::Lattice(o) if o.cage == cage))
        });
        if !lattice.is_at_rest() || self.parent(cage).is_some() || used_elsewhere {
            return false;
        }
        let (fitted, place) = fitted_cage(self, &[shape], lattice.divisions());
        let Some(object) = self.object(cage) else {
            return false;
        };
        fitted.size().abs_diff_eq(lattice.size(), 0.01)
            && object.transform.matrix().abs_diff_eq(place, 0.01)
    }
}
