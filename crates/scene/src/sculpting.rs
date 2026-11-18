//! What the interface needs to sculpt an object ([decision 0304](../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! where its sculpting goes and the shape coming in there, and handing a
//! finished stroke's shape to the cache so it isn't worked out again.

use crate::object::{ObjectId, ObjectKind};
use crate::tree::Scene;
use mesh::Mesh;
use std::sync::Arc;

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
