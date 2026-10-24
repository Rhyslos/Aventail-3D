//! The commands that add lattice cages and change their points
//! ([decision 0144](../../../../docs/decisions/0144-lattice-cage.md)).

use super::{Command, Run, existing};
use crate::lattice::LatticeOptions;
use crate::modifier::{Modifier, ModifierKind, ModifierRefusal, Purpose};
use crate::object::{Object, ObjectId, ObjectKind, Transform};
use crate::tree::{Edit, Place, SceneError};
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
            let mut after = self.scene.modifiers(id).to_vec();
            after.push(Modifier::new(kind.clone()));
            self.scene
                .check_modifiers(id, &after)
                .map_err(SceneError::Modifier)?;
            let before = self.scene.modifiers(id).to_vec();
            self.edit(Edit::SetModifiers { id, before, after })?;
        }
        Ok(Some(vec![cage]))
    }

    /// A cage round `ids`, and where it goes in the world. Round one shape
    /// it lines up with the shape (turned and sized as it is); round
    /// several it lines up with the world.
    fn fit_cage(&self, ids: &[ObjectId], divisions: [u32; 3]) -> (Lattice, Mat4) {
        let shown = |id: ObjectId| {
            self.scene
                .final_shape(id, Purpose::Viewport)
                .and_then(|m| m.bounds())
        };
        let unit = || Bounds::around([Vec3::splat(-50.0), Vec3::splat(50.0)]);
        if let [id] = ids {
            let matrix = self.scene.shape_matrix(*id);
            if let Some(bounds) = shown(*id).or_else(unit) {
                let lattice = Lattice::fitted(bounds.size(), divisions);
                return (lattice, matrix * Mat4::from_translation(bounds.centre()));
            }
        }
        let world = ids
            .iter()
            .filter_map(|id| {
                let local = shown(*id)?;
                let matrix = self.scene.shape_matrix(*id);
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
