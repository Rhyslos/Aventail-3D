//! Collision shapes outlined over the view ([decision 0151](../../../../docs/decisions/0151-collision-hulls.md)):
//! the hulls a game engine will collide with (*Wrapped*, *Wrapped in
//! parts*, *A box*), as thin lines, for the selected objects, or for
//! every object with Display → Show collision.
//!
//! Worked out from each object's shape as drawn, kept by the scene; light
//! work at once, splitting into parts (and heavy wrapping) on another
//! thread, one job per object at a time, the last outline shown meanwhile,
//! as heavy modifiers are.

use super::OverlayLine;
use crate::drawing::{OverlayStyle, SceneDrawing};
use glam::Vec3;
use scene::{
    COLLISION_IN_BACKGROUND_FROM, CollisionDone, CollisionShapes, CollisionState, ObjectId,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;

/// Lines in an object's shape's space.
type Lines = Arc<Vec<[Vec3; 2]>>;

/// Each object's collision outline, and the work under way.
#[derive(Debug, Default)]
pub(crate) struct CollisionOutlines {
    /// Each object's last shapes and their lines, in its shape's space.
    kept: HashMap<ObjectId, (Arc<CollisionShapes>, Lines)>,
    /// Shapes being worked out on other threads.
    jobs: HashMap<ObjectId, JoinHandle<CollisionDone>>,
    /// Goes up whenever work from another thread comes in.
    pub(crate) revision: u64,
}

impl CollisionOutlines {
    /// Hands every finished piece of work to the scene.
    pub(crate) fn collect(&mut self, scene: &scene::Scene) {
        let done: Vec<ObjectId> = self
            .jobs
            .iter()
            .filter(|(_, job)| job.is_finished())
            .map(|(id, _)| *id)
            .collect();
        for id in done {
            if let Some(job) = self.jobs.remove(&id)
                && let Ok(result) = job.join()
            {
                scene.store_collision(result);
            }
            self.revision += 1;
        }
    }

    /// Whether work is under way on another thread.
    pub(crate) fn busy(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Forgets everything (another document).
    pub(crate) fn forget(&mut self) {
        self.kept.clear();
        self.jobs.clear();
    }

    /// The outlines to draw, in the world: the selected objects', or every
    /// shown object's with `show_all`.
    pub(crate) fn lines(
        &mut self,
        drawing: &SceneDrawing,
        scene: &scene::Scene,
        show_all: bool,
    ) -> Vec<OverlayLine> {
        let mut out = Vec::new();
        for item in &drawing.items {
            // A cutter goes out as part of what it cuts (decision 0135):
            // no collision of its own.
            if !(item.selected || show_all) || item.outline || !item.transform.is_finite() {
                continue;
            }
            let Some(lines) = self.lines_of(scene, item.id, &item.drawn) else {
                continue;
            };
            let world = |p: Vec3| item.transform.transform_point3(p);
            out.extend(
                lines
                    .iter()
                    .map(|[a, b]| ([world(*a), world(*b)], OverlayStyle::Collision)),
            );
        }
        out
    }

    /// Object `id`'s outline for `shape`: kept, worked out now when light,
    /// or the last one while it is worked out elsewhere.
    fn lines_of(
        &mut self,
        scene: &scene::Scene,
        id: ObjectId,
        shape: &Arc<scene::Mesh>,
    ) -> Option<Lines> {
        let shapes = match scene.collision_state(id, scene::Purpose::Viewport, Arc::clone(shape)) {
            None => {
                self.kept.remove(&id);
                return None;
            }
            Some(CollisionState::Ready(shapes)) => shapes,
            Some(CollisionState::Needed(job)) if job.work() < COLLISION_IN_BACKGROUND_FROM => {
                let done = job.run();
                let shapes = done.shapes();
                scene.store_collision(done);
                shapes
            }
            Some(CollisionState::Needed(job)) => {
                // One at a time per object: newer work waits until the
                // work under way is in.
                self.jobs
                    .entry(id)
                    .or_insert_with(|| std::thread::spawn(move || job.run()));
                return self.kept.get(&id).map(|(_, lines)| Arc::clone(lines));
            }
        };
        if let Some((kept, lines)) = self.kept.get(&id)
            && Arc::ptr_eq(kept, &shapes)
        {
            return Some(Arc::clone(lines));
        }
        let lines = Arc::new(outline(&shapes));
        self.kept.insert(id, (shapes, Arc::clone(&lines)));
        Some(lines)
    }
}

/// The lines of every hull in `shapes`: their edges, flat sides not
/// crossed.
fn outline(shapes: &CollisionShapes) -> Vec<[Vec3; 2]> {
    shapes
        .hulls
        .iter()
        .flat_map(|hull| {
            hull.outline()
                .into_iter()
                .map(|[a, b]| [hull.points[a as usize], hull.points[b as usize]])
        })
        .collect()
}
