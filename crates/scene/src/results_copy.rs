//! A copy of a scene that keeps what it has worked out, for work done on
//! another thread (saving and exporting, decision 0163): a plain copy
//! starts with empty caches and would work every shape out again.

use crate::tree::Scene;

impl Scene {
    /// A copy of the scene holding everything worked out so far: each
    /// object's evaluated shape, its booleans' cut, its modifiers, its
    /// collision shapes and levels of detail. Quick: shapes are shared
    /// between the two, never copied. What the copy works out later stays
    /// with the copy.
    pub fn copy_with_results(&self) -> Scene {
        let mut copy = self.clone();
        copy.cache = self.cache.copied();
        copy.cuts = self.cuts.copied();
        copy.stacks = self.stacks.copied();
        copy.collisions = self.collisions.copied();
        copy.lods = self.lods.copied();
        copy
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::sync::Arc;

    #[test]
    fn a_copy_with_results_reuses_the_shapes_worked_out() {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Sphere.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::Subdivide(SubdivideOptions::default()),
        })
        .unwrap();
        let scene = doc.scene();
        let shown = scene.shown_shape(id).unwrap();
        let evaluated = scene.evaluated(id).unwrap().mesh;
        let copy = scene.copy_with_results();
        assert_eq!(copy, *scene);
        assert!(Arc::ptr_eq(&copy.evaluated(id).unwrap().mesh, &evaluated));
        assert!(Arc::ptr_eq(&copy.shown_shape(id).unwrap(), &shown));
        // A plain copy works them out again.
        let plain = scene.clone();
        assert!(!Arc::ptr_eq(&plain.shown_shape(id).unwrap(), &shown));
        assert_eq!(*plain.shown_shape(id).unwrap(), *shown);
    }
}
