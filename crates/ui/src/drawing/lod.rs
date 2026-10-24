//! The drawing cache's part in levels of detail (decision 0155): an
//! object whose Preview shows a level is drawn as that level, made from
//! the shape the viewport would draw; light shapes at once, heavy ones on
//! another thread while the last level made (or the full shape) is shown,
//! as heavy modifiers are.

use super::DrawingCache;
use mesh::Mesh;
use scene::{LodState, ObjectId};
use std::sync::Arc;

impl DrawingCache {
    /// What to draw for object `id` given `drawn` (its shape as the
    /// viewport draws it): the level its Preview shows, or `drawn` itself.
    /// A `selected` object's levels are made even while it shows its full
    /// shape, so the Attributes panel can give their triangles.
    pub(super) fn previewed(
        &mut self,
        scene: &scene::Scene,
        id: ObjectId,
        drawn: Arc<Mesh>,
        selected: bool,
    ) -> Arc<Mesh> {
        let Some(lods) = scene.object(id).map(|o| &o.lods).filter(|l| l.on) else {
            self.last_lod.remove(&id);
            return drawn;
        };
        let preview = lods.clamped().preview;
        if preview == 0 {
            self.last_lod.remove(&id);
            if selected {
                self.make_lods(scene, id, &drawn);
            }
            return drawn;
        }
        let shapes = match scene.lod_state(id, Arc::clone(&drawn)) {
            None => {
                self.last_lod.remove(&id);
                return drawn;
            }
            Some(LodState::Ready(shapes)) => shapes,
            Some(LodState::Needed(job)) if job.work() < scene::LODS_IN_BACKGROUND_FROM => {
                let done = job.run();
                let shapes = done.shapes();
                scene.store_lods(done);
                shapes
            }
            Some(LodState::Needed(job)) => {
                // One at a time per object: newer work waits until the
                // work under way is in.
                self.lodding
                    .entry(id)
                    .or_insert_with(|| std::thread::spawn(move || job.run()));
                return self.last_lod.get(&id).cloned().unwrap_or(drawn);
            }
        };
        let Some(level) = shapes.levels.get(preview - 1) else {
            // None made (a shape at no real place): the shape itself.
            self.last_lod.remove(&id);
            return drawn;
        };
        self.last_lod.insert(id, Arc::clone(level));
        Arc::clone(level)
    }

    /// Makes object `id`'s levels from `drawn` if they aren't made: at once
    /// when light, on another thread when heavy.
    fn make_lods(&mut self, scene: &scene::Scene, id: ObjectId, drawn: &Arc<Mesh>) {
        if let Some(LodState::Needed(job)) = scene.lod_state(id, Arc::clone(drawn)) {
            if job.work() < scene::LODS_IN_BACKGROUND_FROM {
                scene.store_lods(job.run());
            } else {
                self.lodding
                    .entry(id)
                    .or_insert_with(|| std::thread::spawn(move || job.run()));
            }
        }
    }

    /// Hands every finished set of levels to the scene; whether any was.
    pub(super) fn collect_lods(&mut self, scene: &scene::Scene) -> bool {
        let done: Vec<ObjectId> = self
            .lodding
            .iter()
            .filter(|(_, job)| job.is_finished())
            .map(|(id, _)| *id)
            .collect();
        for id in &done {
            if let Some(job) = self.lodding.remove(id)
                && let Ok(result) = job.join()
            {
                scene.store_lods(result);
            }
        }
        !done.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::drawing::{DrawingCache, Hidden};
    use mesh::decimate::triangle_count;
    use scene::{Command, Document, Lods, ObjectKind, PrimitiveKind};

    #[test]
    fn the_previewed_level_is_drawn_and_the_full_shape_is_still_picked() {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Sphere.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        let mut cache = DrawingCache::default();
        let full = {
            let item = &cache.update(&doc, None, &Hidden::default()).drawing.items[0];
            triangle_count(&item.drawn)
        };
        let set = |doc: &mut Document, on: bool, preview: usize| {
            doc.execute(Command::SetLods {
                ids: vec![id],
                lods: Lods {
                    on,
                    preview,
                    ..Lods::default()
                },
            })
            .unwrap();
        };
        // On, showing the full shape: drawn as it was.
        set(&mut doc, true, 0);
        let item = cache.update(&doc, None, &Hidden::default()).drawing.items[0].clone();
        assert_eq!(triangle_count(&item.drawn), full);
        // Level 2: a quarter of the triangles drawn, the whole shape picked.
        set(&mut doc, true, 2);
        let item = cache.update(&doc, None, &Hidden::default()).drawing.items[0].clone();
        assert!(triangle_count(&item.drawn) <= full / 4 + 2);
        assert_eq!(triangle_count(&item.shown), full);
        // Levels off: the full shape again, though Preview is kept.
        set(&mut doc, false, 2);
        let item = cache.update(&doc, None, &Hidden::default()).drawing.items[0].clone();
        assert_eq!(triangle_count(&item.drawn), full);
        assert!(!cache.busy());
    }
}
