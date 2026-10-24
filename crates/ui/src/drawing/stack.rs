//! The drawing cache's part in modifiers (decision 0142): an object's
//! modifiers are worked out for the viewport from its whole shape before
//! them; light ones at once, heavy ones on another thread while the last
//! result (or the shape without them) is shown, as heavy booleans are.

use super::DrawingCache;
use mesh::Mesh;
use scene::{ObjectId, Purpose, StackState};
use std::sync::Arc;

impl DrawingCache {
    /// The object's shape after the modifiers the viewport shows, from
    /// `input` (its whole shape before them: edits, booleans, mirror).
    pub(super) fn modified(
        &mut self,
        scene: &scene::Scene,
        id: ObjectId,
        input: Arc<Mesh>,
    ) -> Arc<Mesh> {
        let mesh = match scene.stack_state(id, Purpose::Viewport, Arc::clone(&input)) {
            None => {
                self.last_stack.remove(&id);
                return input;
            }
            Some(StackState::Ready(result)) => result.mesh,
            Some(StackState::Needed(job)) if job.work() < scene::MODIFIERS_IN_BACKGROUND_FROM => {
                let done = job.run();
                let mesh = done.result().mesh;
                scene.store_stack(done);
                mesh
            }
            Some(StackState::Needed(job)) => {
                // One at a time per object: newer work waits until the
                // work under way is in, so a drag doesn't pile it up.
                self.stacking
                    .entry(id)
                    .or_insert_with(|| std::thread::spawn(move || job.run()));
                return self.last_stack.get(&id).cloned().unwrap_or(input);
            }
        };
        self.last_stack.insert(id, Arc::clone(&mesh));
        mesh
    }

    /// Hands every finished piece of modifier work to the scene; whether
    /// any was.
    pub(super) fn collect_stacks(&mut self, scene: &scene::Scene) -> bool {
        let done: Vec<ObjectId> = self
            .stacking
            .iter()
            .filter(|(_, job)| job.is_finished())
            .map(|(id, _)| *id)
            .collect();
        for id in &done {
            if let Some(job) = self.stacking.remove(id)
                && let Ok(result) = job.join()
            {
                scene.store_stack(result);
            }
        }
        !done.is_empty()
    }
}
