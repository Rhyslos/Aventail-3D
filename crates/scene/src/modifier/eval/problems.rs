//! The problems the Attributes panel shows for an object's modifiers,
//! worked out without heavy work on the interface thread.

use super::{MODIFIERS_IN_BACKGROUND_FROM, StackJob, StackState};
use crate::modifier::Purpose;
use crate::object::ObjectId;
use crate::tree::Scene;
use std::sync::Arc;

impl Scene {
    /// Why each of the object's modifiers that can't do its work can't,
    /// by its place in the list, in plain words starting with a small
    /// letter. Left out while heavy work (its booleans or its modifiers)
    /// is still to be done: the interface does that on another thread.
    /// An export's problems (for modifiers the viewport doesn't show, or
    /// that do something else for an export) are only those already
    /// worked out, by an export or by [`Scene::export_problems_job`]'s job:
    /// asked every frame, this never works an export out.
    pub fn modifier_problems(&self, id: ObjectId) -> Vec<(usize, String)> {
        let modifiers = self.modifiers(id);
        if !modifiers.iter().any(|m| m.on) {
            return Vec::new();
        }
        // Asked every frame: the shape before the modifiers is kept by
        // the scene while nothing changes, so this is quick.
        let Some(input) = self.input_if_light(id) else {
            return Vec::new();
        };
        let mut purposes = vec![Purpose::Viewport];
        if self.export_differs(id) {
            purposes.push(Purpose::Export);
        }
        let mut problems: Vec<(usize, String)> = Vec::new();
        for purpose in purposes {
            let found = match self.stack_state(id, purpose, Arc::clone(&input)) {
                Some(StackState::Ready(result)) => result.problems,
                Some(StackState::Needed(job))
                    if purpose == Purpose::Viewport
                        && job.work() < MODIFIERS_IN_BACKGROUND_FROM =>
                {
                    let done = job.run();
                    let result = done.result();
                    self.store_stack(done);
                    result.problems
                }
                _ => Vec::new(),
            };
            for (index, why) in found {
                if !problems.iter().any(|(i, _)| *i == index) {
                    problems.push((index, why));
                }
            }
        }
        problems.sort_by_key(|(index, _)| *index);
        problems
    }

    /// Whether an export works out object `id`'s modifiers differently
    /// from the viewport: some are on but not shown, or do something else
    /// for an export (Subdivide's levels).
    fn export_differs(&self, id: ObjectId) -> bool {
        self.modifiers(id)
            .iter()
            .any(|m| m.on && (!m.in_viewport || m.kind.differs_by_purpose()))
    }

    /// The light work still to do to find object `id`'s problems for an
    /// export ([`Scene::modifier_problems`]), to do on another thread and
    /// hand back with [`Scene::store_stack`]; `None` when there is none,
    /// it is done, or it is heavy (an export works that out).
    pub fn export_problems_job(&self, id: ObjectId) -> Option<StackJob> {
        if !self.export_differs(id) || !self.modifiers(id).iter().any(|m| m.on) {
            return None;
        }
        let input = self.input_if_light(id)?;
        match self.stack_state(id, Purpose::Export, input)? {
            StackState::Needed(job) if job.work() < MODIFIERS_IN_BACKGROUND_FROM => Some(job),
            _ => None,
        }
    }
}
