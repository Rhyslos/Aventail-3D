//! What the interface needs to sculpt an object ([decision 0304](../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! where its sculpting goes and the shape coming in there, and handing a
//! finished stroke's shape to the cache so it isn't worked out again.

use crate::object::{ObjectId, ObjectKind};
use crate::tree::Scene;
use mesh::Mesh;
use std::sync::Arc;

/// Shapes joined into one, and the materials of the one they joined into
/// when any of them had its own (see [`Scene::joined_shape`]).
pub type Joined = (Mesh, Option<Vec<crate::Material>>);

/// Where an object's sculpting goes, and what it works on.
#[derive(Debug, Clone)]
pub struct SculptTarget {
    /// The Sculpt layer sculpting goes into, or `None` when a new one
    /// will be added on top (`Command::Sculpt`).
    pub layer: Option<usize>,
    /// The shape coming into that layer: the layers under it worked out
    /// (for a new layer, the whole stack).
    pub input: Arc<Mesh>,
    /// The layer's sculpting so far, if it has a layer.
    pub sculpt: Option<sculpt::Sculpt>,
}

impl Scene {
    /// Where object `id`'s sculpting goes. `None` for an object that
    /// can't be sculpted: a group, an instance, a curve or a cage.
    pub fn sculpt_target(&self, id: ObjectId) -> Option<SculptTarget> {
        let object = self.object(id)?;
        if !matches!(object.kind, ObjectKind::Mesh(_) | ObjectKind::Primitive(_)) {
            return None;
        }
        let layer = object.stack.sculpt_layer();
        let input = match layer {
            Some(index) => self.cache.with(|cache| {
                cache.layer_input(
                    id,
                    &object.kind,
                    &object.stack,
                    &self.eval_context(),
                    None,
                    index,
                )
            })?,
            None => self.evaluated(id)?.mesh,
        };
        let sculpt = layer.and_then(|i| object.stack.layers()[i].sculpt().cloned());
        Some(SculptTarget {
            layer,
            input,
            sculpt,
        })
    }

    /// Takes `mesh` as object `id`'s Sculpt layer's result: the shape the
    /// interface worked out for the stroke it just sent, exactly what
    /// working the layer out would give, so the next frame doesn't do it
    /// again. Whether it was taken (the layers under the Sculpt layer must
    /// be as the cache last worked them out).
    pub fn prime_sculpt(&self, id: ObjectId, mesh: Arc<Mesh>) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        let Some(index) = object.stack.sculpt_layer() else {
            return false;
        };
        self.cache.with(|cache| {
            cache.prime(
                id,
                &object.kind,
                &object.stack,
                &self.eval_context(),
                None,
                index,
                mesh,
            )
        })
    }
}

impl Scene {
    /// The object's shape from its edits as an export or a bake takes it
    /// ([decision 0315](../../../docs/decisions/0315-detail-levels.md)):
    /// [`Scene::evaluated`]'s, but with each Sculpt layer at the detail
    /// level chosen for exports and bakes instead of the one shown. The
    /// same shape (shared) when they are the same level.
    pub fn evaluated_for_export(&self, id: ObjectId) -> Option<crate::Evaluated> {
        let object = self.object(id)?;
        if let ObjectKind::Instance(source) = object.kind {
            return self
                .object(source)
                .filter(|o| !matches!(o.kind, ObjectKind::Instance(_)))
                .and_then(|_| self.evaluated_for_export(source));
        }
        if !object.stack.exports_another_level() {
            return self.evaluated(id);
        }
        let profile = self.profile_curve(id);
        self.cache
            .with(|cache| {
                cache.evaluate_for_export(
                    id,
                    &object.kind,
                    &object.stack,
                    &self.eval_context(),
                    profile.as_ref(),
                )
            })
            .or_else(|| self.evaluated(id))
    }

    /// Whether an export or a bake takes another shape from object `id`'s
    /// edits than the one shown (an instance's original's): a sculpt
    /// whose export level isn't the level shown.
    pub fn exports_another_level(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        match object.kind {
            ObjectKind::Instance(source) => self
                .object(source)
                .is_some_and(|o| o.stack.exports_another_level()),
            _ => object.stack.exports_another_level(),
        }
    }

    /// The object's shape before its modifiers for `purpose`:
    /// [`Scene::unmodified_shape`], but for an export, a bake or a level
    /// of detail, from the shape [`Scene::evaluated_for_export`] gives
    /// (cut by its booleans as they stand, worked out here and not kept,
    /// and mirrored).
    pub fn unmodified_shape_for(&self, id: ObjectId, purpose: crate::Purpose) -> Option<Arc<Mesh>> {
        if purpose == crate::Purpose::Viewport || !self.exports_another_level(id) {
            return self.unmodified_shape(id);
        }
        let shape = self.evaluated_for_export(id)?.mesh;
        let shape = match self.cut_of(id, &shape) {
            Some(Ok(cut)) => cut,
            _ => shape,
        };
        Some(self.mirrored_shape(id, shape))
    }
}

impl Scene {
    /// The shapes of `target` and `others` as one shape in `target`'s own
    /// space, each with its edits worked out, as Join makes it (decision
    /// 0055); and, when any of them has materials, `target`'s material
    /// list with the names the others bring that it lacks, their polygons
    /// renumbered into it by name (decision 0074). Objects without a shape
    /// are left out. `None` when `target` has no shape. Remesh for
    /// sculpting merges selected shapes through it (decision 0312).
    pub fn joined_shape(&self, target: ObjectId, others: &[ObjectId]) -> Option<Joined> {
        self.evaluated(target)?;
        let shapes: Vec<ObjectId> = std::iter::once(target)
            .chain(others.iter().copied().filter(|id| *id != target))
            .filter(|id| self.evaluated(*id).is_some())
            .collect();
        let into = self.shape_matrix(target).inverse();
        let list = |id: ObjectId| {
            self.object(id)
                .map(crate::Object::material_list)
                .unwrap_or_default()
        };
        let any_materials = shapes
            .iter()
            .any(|id| self.object(*id).is_some_and(|o| !o.materials.is_empty()));
        let mut merged = list(target);
        let meshes: Vec<(Arc<Mesh>, glam::Mat4)> = shapes
            .iter()
            .filter_map(|id| {
                let shape = self.evaluated(*id)?;
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
                Some((part, into * self.shape_matrix(*id)))
            })
            .collect();
        let materials = any_materials.then(|| {
            merged.truncate(mesh::material::MAX_SLOTS as usize);
            merged
        });
        let parts: Vec<(&Mesh, glam::Mat4)> =
            meshes.iter().map(|(m, x)| (m.as_ref(), *x)).collect();
        Some((mesh::parts::combine(&parts), materials))
    }
}
