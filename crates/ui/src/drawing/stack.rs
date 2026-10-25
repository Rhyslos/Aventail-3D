//! The drawing cache's part in modifiers (decision 0142): an object's
//! modifiers are worked out for the viewport from its whole shape before
//! them; light ones at once, heavy ones on another thread while the last
//! result (or the shape without them) is shown, as heavy booleans are.
//! The selected objects' modifiers are also worked out for an export on
//! another thread when that differs, only so the Attributes panel can say
//! which can't do their work.

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

    /// Starts working out, on other threads, the selected objects'
    /// modifiers for an export where that is light and still to do (one
    /// at a time per object), so their problems show.
    pub(super) fn find_export_problems(&mut self, document: &scene::Document) {
        let scene = document.scene();
        for id in document.selection() {
            if self.exporting.contains_key(&id) {
                continue;
            }
            if let Some(job) = scene.export_problems_job(id) {
                self.exporting
                    .insert(id, std::thread::spawn(move || job.run()));
            }
        }
    }

    /// Hands every finished export's worth of modifiers to the scene;
    /// whether any was.
    pub(super) fn collect_export_problems(&mut self, scene: &scene::Scene) -> bool {
        let done: Vec<ObjectId> = self
            .exporting
            .iter()
            .filter(|(_, job)| job.is_finished())
            .map(|(id, _)| *id)
            .collect();
        for id in &done {
            if let Some(job) = self.exporting.remove(id)
                && let Ok(result) = job.join()
            {
                scene.store_stack(result);
            }
        }
        !done.is_empty()
    }
}
