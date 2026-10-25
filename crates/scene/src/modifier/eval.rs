//! Working out an object's modifiers (decision 0142): one after another,
//! each result kept, so changing a modifier works out only it and those
//! after it; heavy work handed out to run on another thread, as heavy
//! booleans are (decision 0135).

use super::{ModifierContext, ModifierKind, Purpose, SourceData, SourceShape};
use crate::object::ObjectId;
use crate::tree::Scene;
use mesh::Mesh;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod estimate;
mod problems;
mod sources;

use estimate::Estimate;
use sources::PendingSource;

/// From this much work ([`ModifierKind::work`], in polygons of a
/// boolean's worth), the interface works an object's modifiers out on
/// another thread, showing the last result meanwhile, as it does for
/// booleans ([`crate::CUT_IN_BACKGROUND_FROM`]).
pub const MODIFIERS_IN_BACKGROUND_FROM: usize = 5_000;

/// One modifier worked out: what from, and what it gave.
#[derive(Debug, Clone)]
struct Step {
    /// Its place in the object's list.
    index: usize,
    kind: ModifierKind,
    sources: Vec<SourceShape>,
    /// How the object was smoothed (see [`ModifierContext::hardness`]).
    hardness: mesh::Hardness,
    /// The object's pivot (see [`ModifierContext::pivot`]).
    pivot: glam::Vec3,
    /// Its result, or why it couldn't be made (its input is then passed
    /// on unchanged).
    result: Result<Arc<Mesh>, String>,
}

/// An object's modifiers worked out for one purpose.
#[derive(Debug, Clone)]
struct Run {
    input: Arc<Mesh>,
    steps: Vec<Step>,
}

/// A modifier still to work out.
#[derive(Debug, Clone)]
struct Planned {
    index: usize,
    kind: ModifierKind,
    sources: Vec<SourceShape>,
    /// Sources whose own modifiers aren't worked out yet: done first, by
    /// the job, so working out which shapes a modifier reads never waits
    /// on the interface thread (each one's place in `sources` holds its
    /// shape before its modifiers meanwhile).
    pending: Vec<PendingSource>,
    hardness: mesh::Hardness,
    pivot: glam::Vec3,
}

/// An object's shape with its live mirror's copies, with what it was
/// made from.
#[derive(Debug, Clone)]
struct Mirrored {
    base: Arc<Mesh>,
    mirror: crate::LiveMirror,
    /// The object's shape matrix and world matrix.
    places: (glam::Mat4, glam::Mat4),
    result: Arc<Mesh>,
}

/// The modifiers' results, by object and purpose, and each object's
/// mirrored shape they start from (not part of what the scene is: copies
/// start empty, and it never affects equality).
#[derive(Default)]
pub(crate) struct StackCache {
    runs: Mutex<HashMap<(ObjectId, Purpose), Run>>,
    mirrored: Mutex<HashMap<ObjectId, Mirrored>>,
    estimates: Mutex<HashMap<(ObjectId, Purpose), Estimate>>,
}

impl StackCache {
    /// How many work estimates are kept (for tests).
    #[cfg(test)]
    pub(crate) fn estimates_kept(&self) -> usize {
        self.estimates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Forgets the results for `id` (deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.runs
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(object, _), _| *object != id);
        self.mirrored
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&id);
        self.estimates
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(object, _), _| *object != id);
    }

    fn get(&self, id: ObjectId, purpose: Purpose) -> Option<Run> {
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(id, purpose))
            .cloned()
    }

    fn put(&self, id: ObjectId, purpose: Purpose, run: Run) {
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((id, purpose), run);
        // Worked out: the estimate is no longer needed.
        self.estimates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(id, purpose));
    }
}

impl StackCache {
    /// A copy holding the same results, for
    /// [`crate::Scene::copy_with_results`].
    pub(crate) fn copied(&self) -> Self {
        fn copy<T: Clone>(m: &Mutex<T>) -> Mutex<T> {
            Mutex::new(
                m.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone(),
            )
        }
        Self {
            runs: copy(&self.runs),
            mirrored: copy(&self.mirrored),
            estimates: copy(&self.estimates),
        }
    }
}

impl Clone for StackCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for StackCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for StackCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackCache").finish_non_exhaustive()
    }
}

/// An object's shape after its modifiers, and the modifiers that couldn't
/// do their work.
#[derive(Debug, Clone)]
pub struct StackResult {
    /// The shape, in the object's own space.
    pub mesh: Arc<Mesh>,
    /// Each modifier that couldn't do its work (by its place in the
    /// object's list), and why, in plain words starting with a small
    /// letter. Its input was passed on unchanged.
    pub problems: Vec<(usize, String)>,
}

/// Whether an object's modifiers are worked out.
#[derive(Debug, Clone)]
pub enum StackState {
    /// Worked out for its shape and settings as they are now.
    Ready(StackResult),
    /// Still to be worked out: [`StackJob::run`] does it, on any thread.
    Needed(StackJob),
}

/// Everything still to do to work out an object's modifiers, gathered from
/// the scene so it can be done on another thread (it holds only shared
/// shapes and settings).
#[derive(Debug, Clone)]
pub struct StackJob {
    id: ObjectId,
    purpose: Purpose,
    input: Arc<Mesh>,
    /// The modifiers already worked out (kept from before).
    done: Vec<Step>,
    /// Those still to do, in order.
    plan: Vec<Planned>,
    /// Roughly how much work `plan` is, its pending sources' included.
    work: usize,
}

/// An object's modifiers worked out: hand them back with
/// [`Scene::store_stack`].
#[derive(Debug, Clone)]
pub struct StackDone {
    id: ObjectId,
    purpose: Purpose,
    run: Run,
    /// The sources' modifiers worked out on the way, kept with it.
    sources: Vec<StackDone>,
}

/// The shape after `steps` (the last one that worked, or `input`).
fn last_shape(input: &Arc<Mesh>, steps: &[Step]) -> Arc<Mesh> {
    steps
        .iter()
        .rev()
        .find_map(|s| s.result.as_ref().ok())
        .map_or_else(|| Arc::clone(input), Arc::clone)
}

impl Run {
    fn result(&self) -> StackResult {
        StackResult {
            mesh: last_shape(&self.input, &self.steps),
            problems: self
                .steps
                .iter()
                .filter_map(|s| s.result.as_ref().err().map(|why| (s.index, why.clone())))
                .collect(),
        }
    }
}

impl StackJob {
    /// The object whose modifiers these are.
    pub fn object(&self) -> ObjectId {
        self.id
    }

    /// What the shape is for.
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    /// How many of its modifiers were kept from before (for tests).
    #[cfg(test)]
    pub(crate) fn steps_kept(&self) -> usize {
        self.done.len()
    }

    /// Roughly how much work is left, counting the sources' modifiers
    /// still to work out: from [`MODIFIERS_IN_BACKGROUND_FROM`], do it on
    /// another thread. Quick: worked out (or kept from before) when the
    /// job was made.
    pub fn work(&self) -> usize {
        self.work
    }

    /// Works the modifiers out, the pending sources' first. Slow for
    /// heavy ones: run it away from the interface thread.
    pub fn run(self) -> StackDone {
        let mut steps = self.done;
        let mut sources_done: Vec<StackDone> = Vec::new();
        for mut planned in self.plan {
            for pending in std::mem::take(&mut planned.pending) {
                let (id, purpose) = (pending.job.id, pending.job.purpose);
                // Two modifiers reading the same source work it out once.
                let shape = match sources_done
                    .iter()
                    .find(|d| d.id == id && d.purpose == purpose)
                {
                    Some(done) => done.result().mesh,
                    None => {
                        let done = pending.job.run();
                        let shape = done.result().mesh;
                        sources_done.push(done);
                        shape
                    }
                };
                if let Some(source) = planned.sources.get_mut(pending.at) {
                    source.data = SourceData::Shape(shape);
                }
            }
            let input = last_shape(&self.input, &steps);
            let context = ModifierContext {
                purpose: self.purpose,
                hardness: planned.hardness,
                sources: planned.sources.clone(),
                pivot: planned.pivot,
            };
            let result = planned.kind.apply(&input, &context).and_then(|mut mesh| {
                if !planned.kind.carries_custom_normals() {
                    mesh::custom_normals::forget_custom_normals(&mut mesh);
                }
                if mesh.face_count() > crate::MAX_SHAPE_POLYGONS {
                    Err(words::trf!(
                        "the result would have more than {} polygons",
                        crate::MAX_SHAPE_POLYGONS
                    ))
                } else {
                    Ok(Arc::new(mesh))
                }
            });
            steps.push(Step {
                index: planned.index,
                kind: planned.kind,
                sources: planned.sources,
                hardness: planned.hardness,
                pivot: planned.pivot,
                result,
            });
        }
        StackDone {
            id: self.id,
            purpose: self.purpose,
            run: Run {
                input: self.input,
                steps,
            },
            sources: sources_done,
        }
    }
}

/// The steps of `steps` (another run's, from the same input) that do the
/// work `done` (steps already kept) and then `plan` ask for, in order, up
/// to the first that doesn't: each with its place in this list.
fn reuse(plan: &[Planned], done: &[Step], steps: &[Step]) -> Vec<Step> {
    let mut out = Vec::new();
    let wanted = done
        .iter()
        .map(|d| {
            (
                d.index,
                d.kind.clone(),
                &d.sources,
                d.hardness,
                d.pivot,
                true,
            )
        })
        .chain(plan.iter().map(|p| {
            (
                p.index,
                p.kind.clone(),
                &p.sources,
                p.hardness,
                p.pivot,
                p.pending.is_empty(),
            )
        }));
    for ((index, kind, sources, hardness, pivot, settled), step) in wanted.zip(steps) {
        if !(settled
            && kind == step.kind
            && *sources == step.sources
            && hardness == step.hardness
            && pivot == step.pivot)
        {
            break;
        }
        out.push(Step {
            index,
            ..step.clone()
        });
    }
    out
}

impl StackJob {
    /// The same work, with whatever `other` (a run from the same input)
    /// already did that it asks for taken from it (decision 0165: a level
    /// of detail's changes reuse the export's steps before them).
    pub(crate) fn reusing(mut self, other: &StackDone) -> Self {
        if !same(&self.input, &other.run.input) {
            return self;
        }
        let reused = reuse(&self.plan, &self.done, &other.run.steps);
        if reused.len() <= self.done.len() {
            return self;
        }
        let skip = reused.len() - self.done.len();
        self.plan.drain(..skip.min(self.plan.len()));
        self.done = reused;
        self
    }
}

impl StackDone {
    /// The object whose modifiers these are.
    pub fn object(&self) -> ObjectId {
        self.id
    }

    /// The shape after them, and the problems.
    pub fn result(&self) -> StackResult {
        self.run.result()
    }
}

/// Whether two shapes are the same: the same one, or equal.
fn same(a: &Arc<Mesh>, b: &Arc<Mesh>) -> bool {
    Arc::ptr_eq(a, b) || **a == **b
}

impl Scene {
    /// `shape` (the object's own, whole: as evaluated, or cut by its
    /// booleans) with its live mirror's copies, or `shape` itself without
    /// a mirror. Kept while the shape, mirror and place stay the same, so
    /// asking again gives the same shared shape, quickly (decision 0142:
    /// the modifiers start from it, and their cache knows it at once).
    pub fn mirrored_shape(&self, id: ObjectId, shape: Arc<Mesh>) -> Arc<Mesh> {
        let Some(mirror) = self.object(id).and_then(|o| o.mirror) else {
            return shape;
        };
        let places = (self.shape_matrix(id), self.world_matrix(id));
        let mut kept = self
            .stacks
            .mirrored
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(m) = kept.get(&id)
            && Arc::ptr_eq(&m.base, &shape)
            && m.mirror == mirror
            && m.places == places
        {
            return Arc::clone(&m.result);
        }
        let result = self
            .with_mirror(id, &shape)
            .map_or_else(|| Arc::clone(&shape), Arc::new);
        kept.insert(
            id,
            Mirrored {
                base: shape,
                mirror,
                places,
                result: Arc::clone(&result),
            },
        );
        result
    }

    /// Whether the object's modifiers (an instance's original's) are
    /// worked out for `purpose` on `input` (its shape before them,
    /// [`Scene::unmodified_shape`]), or the work still to do. `None` when
    /// no modifier is worked out for `purpose`.
    pub fn stack_state(
        &self,
        id: ObjectId,
        purpose: Purpose,
        input: Arc<Mesh>,
    ) -> Option<StackState> {
        // An instance shows its original's final shape (decision 0142):
        // the sources are placed, and the pivot found, in the original's
        // space, so the instance has the same shape wherever it stands.
        let owner = self.modifier_owner(id);
        let hardness = self
            .object(owner)
            .map_or(mesh::Hardness::Marked, |o| o.hardness);
        let pivot = self.pivot_in_shape(owner);
        let plan: Vec<Planned> = self
            .modifiers_for(id, purpose)
            .iter()
            .enumerate()
            .filter(|(_, m)| m.applies(purpose))
            .map(|(index, m)| {
                let (sources, pending) = self.source_shapes(owner, &m.kind, purpose.for_sources());
                Planned {
                    index,
                    kind: m.kind.clone(),
                    sources,
                    pending,
                    hardness,
                    // Only the kinds that read it, so moving the pivot works
                    // out nothing else again.
                    pivot: if m.kind.reads_pivot() {
                        pivot
                    } else {
                        glam::Vec3::ZERO
                    },
                }
            })
            .collect();
        if plan.is_empty() {
            return None;
        }
        // The instance's own last result, or else its original's when they
        // start from the same shape (no mirror of the instance's own), so
        // an instance usually costs nothing more; for a level of detail,
        // the export's too, whose steps before the level's first change
        // are the level's (decision 0165). The one that keeps the most.
        let mut candidates = vec![self.stacks.get(id, purpose)];
        if owner != id {
            candidates.push(self.stacks.get(owner, purpose));
        }
        if let Purpose::Level(_) = purpose {
            candidates.push(self.stacks.get(id, Purpose::Export));
            if owner != id {
                candidates.push(self.stacks.get(owner, Purpose::Export));
            }
        }
        let mut done: Vec<Step> = Vec::new();
        for run in candidates
            .into_iter()
            .flatten()
            .filter(|run| same(&run.input, &input))
        {
            let reused = reuse(&plan, &[], &run.steps);
            if reused.len() > done.len() {
                done = reused;
            }
        }
        if done.len() == plan.len() {
            let run = Run { input, steps: done };
            return Some(StackState::Ready(run.result()));
        }
        let rest: Vec<Planned> = plan.into_iter().skip(done.len()).collect();
        let own = self.stacks.plan_work((id, purpose), &input, &done, &rest);
        let work = rest
            .iter()
            .flat_map(|p| &p.pending)
            .map(|p| p.job.work)
            .fold(own, usize::saturating_add);
        Some(StackState::Needed(StackJob {
            id,
            purpose,
            input,
            done,
            plan: rest,
            work,
        }))
    }

    /// Keeps modifiers worked out elsewhere ([`StackJob::run`]), the
    /// sources' worked out on the way with them, for
    /// [`Scene::stack_state`] to find while the shape and settings stay as
    /// they were.
    pub fn store_stack(&self, done: StackDone) {
        for source in done.sources {
            self.store_stack(source);
        }
        self.stacks.put(done.id, done.purpose, done.run);
    }

    /// The object's shape after its modifiers for `purpose`, with the
    /// modifiers that couldn't do their work; worked out here, on this
    /// thread, when not kept from before. `None` for an object with no
    /// shape.
    pub fn final_result(&self, id: ObjectId, purpose: Purpose) -> Option<StackResult> {
        let input = self.unmodified_shape(id)?;
        Some(match self.stack_state(id, purpose, Arc::clone(&input)) {
            None => StackResult {
                mesh: input,
                problems: Vec::new(),
            },
            Some(StackState::Ready(result)) => result,
            Some(StackState::Needed(job)) => {
                let done = job.run();
                let result = done.result();
                self.store_stack(done);
                result
            }
        })
    }

    /// The object's final shape for `purpose`: its edits, booleans, live
    /// mirror and modifiers (see the module notes).
    pub fn final_shape(&self, id: ObjectId, purpose: Purpose) -> Option<Arc<Mesh>> {
        self.final_result(id, purpose).map(|r| r.mesh)
    }

    /// The object's shape as the viewport shows it: its edits, booleans,
    /// live mirror and the modifiers shown in the viewport.
    pub fn shown_shape(&self, id: ObjectId) -> Option<Arc<Mesh>> {
        self.final_shape(id, Purpose::Viewport)
    }

    /// The object's shape as it is exported and baked: its edits,
    /// booleans, live mirror and every modifier that is on.
    pub fn exported_shape(&self, id: ObjectId) -> Option<Arc<Mesh>> {
        self.final_shape(id, Purpose::Export)
    }

    /// The object's shape as the viewport shows it, if it is worked out
    /// already (nothing heavy is worked out here: for the Attributes
    /// panel, asked every frame).
    pub fn shown_shape_if_ready(&self, id: ObjectId) -> Option<Arc<Mesh>> {
        let input = self.input_if_light(id)?;
        match self.stack_state(id, Purpose::Viewport, Arc::clone(&input)) {
            None => Some(input),
            Some(StackState::Ready(result)) => Some(result.mesh),
            Some(StackState::Needed(_)) => None,
        }
    }

    /// What the modifier at `index` in object `id`'s list last started
    /// from in the viewport, as kept from the last time its modifiers were
    /// worked out (the shape before them when it isn't kept, or isn't
    /// shown in the viewport): for helpers drawn round it (decision 0158),
    /// never worked out here.
    pub fn modifier_input(&self, id: ObjectId, index: usize) -> Option<Arc<Mesh>> {
        if let Some(run) = self.stacks.get(id, Purpose::Viewport)
            && let Some(at) = run.steps.iter().position(|s| s.index == index)
        {
            return Some(match at.checked_sub(1) {
                None => Arc::clone(&run.input),
                Some(before) => last_shape(&run.input, &run.steps[..=before]),
            });
        }
        self.input_if_light(id)
    }

    /// The object's shape before its modifiers, unless its booleans are
    /// heavy and still to be worked out (the interface does that on
    /// another thread).
    pub(crate) fn input_if_light(&self, id: ObjectId) -> Option<Arc<Mesh>> {
        if let Some(crate::CutState::Needed(job)) = self.cut_state(id)
            && job.polygons() >= crate::CUT_IN_BACKGROUND_FROM
        {
            return None;
        }
        self.unmodified_shape(id)
    }

    /// Where object `id`'s pivot is in its shape's own space (the origin
    /// for a shape whose pivot hasn't been moved off it, decision 0068).
    pub(crate) fn pivot_in_shape(&self, id: ObjectId) -> glam::Vec3 {
        let offset = self
            .object(id)
            .map_or(glam::Mat4::IDENTITY, |o| o.offset.matrix());
        if offset.determinant().abs() < 1e-12 {
            return glam::Vec3::ZERO;
        }
        let pivot = offset.inverse().transform_point3(glam::Vec3::ZERO);
        if pivot.is_finite() {
            pivot
        } else {
            glam::Vec3::ZERO
        }
    }
}
