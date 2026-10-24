//! Working out an object's modifiers (decision 0142): one after another,
//! each result kept, so changing a modifier works out only it and those
//! after it; heavy work handed out to run on another thread, as heavy
//! booleans are (decision 0135).

use super::{ModifierContext, ModifierKind, Purpose, SourceShape};
use crate::object::ObjectId;
use crate::tree::Scene;
use mesh::Mesh;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
}

impl Planned {
    fn is_done_by(&self, step: &Step) -> bool {
        self.kind == step.kind && self.sources == step.sources
    }
}

/// The modifiers' results, by object and purpose (not part of what the
/// scene is: copies start empty, and it never affects equality).
#[derive(Default)]
pub(crate) struct StackCache(Mutex<HashMap<(ObjectId, Purpose), Run>>);

impl StackCache {
    /// Forgets the results for `id` (deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(object, _), _| *object != id);
    }

    fn get(&self, id: ObjectId, purpose: Purpose) -> Option<Run> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(id, purpose))
            .cloned()
    }

    fn put(&self, id: ObjectId, purpose: Purpose, run: Run) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((id, purpose), run);
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
}

/// An object's modifiers worked out: hand them back with
/// [`Scene::store_stack`].
#[derive(Debug, Clone)]
pub struct StackDone {
    id: ObjectId,
    purpose: Purpose,
    run: Run,
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

    /// Roughly how much work is left: from
    /// [`MODIFIERS_IN_BACKGROUND_FROM`], do it on another thread.
    pub fn work(&self) -> usize {
        let input = last_shape(&self.input, &self.done);
        self.plan.iter().map(|p| p.kind.work(&input)).sum()
    }

    /// Works the modifiers out. Slow for heavy ones: run it away from the
    /// interface thread.
    pub fn run(self) -> StackDone {
        let mut steps = self.done;
        for planned in self.plan {
            let input = last_shape(&self.input, &steps);
            let context = ModifierContext {
                purpose: self.purpose,
                sources: planned.sources.clone(),
            };
            let result = planned.kind.apply(&input, &context).and_then(|mesh| {
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
        }
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
        let owner = self.modifier_owner(id);
        let plan: Vec<Planned> = self
            .modifiers(owner)
            .iter()
            .enumerate()
            .filter(|(_, m)| m.applies(purpose))
            .map(|(index, m)| Planned {
                index,
                kind: m.kind.clone(),
                sources: self.source_shapes(id, &m.kind, purpose),
            })
            .collect();
        if plan.is_empty() {
            return None;
        }
        let mut done: Vec<Step> = Vec::new();
        if let Some(run) = self
            .stacks
            .get(id, purpose)
            .filter(|run| same(&run.input, &input))
        {
            for (planned, step) in plan.iter().zip(run.steps) {
                if !planned.is_done_by(&step) {
                    break;
                }
                done.push(Step {
                    index: planned.index,
                    ..step
                });
            }
        }
        if done.len() == plan.len() {
            let run = Run { input, steps: done };
            return Some(StackState::Ready(run.result()));
        }
        let rest = plan.into_iter().skip(done.len()).collect();
        Some(StackState::Needed(StackJob {
            id,
            purpose,
            input,
            done,
            plan: rest,
        }))
    }

    /// Keeps modifiers worked out elsewhere ([`StackJob::run`]), for
    /// [`Scene::stack_state`] to find while the shape and settings stay as
    /// they were.
    pub fn store_stack(&self, done: StackDone) {
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

    /// Why each of the object's modifiers that can't do its work can't,
    /// by its place in the list, in plain words starting with a small
    /// letter. Left out while heavy work (its booleans or its modifiers)
    /// is still to be done: the interface does that on another thread.
    pub fn modifier_problems(&self, id: ObjectId) -> Vec<(usize, String)> {
        let modifiers = self.modifiers(id);
        if !modifiers.iter().any(|m| m.on) {
            return Vec::new();
        }
        if let Some(crate::CutState::Needed(job)) = self.cut_state(id)
            && job.polygons() >= crate::CUT_IN_BACKGROUND_FROM
        {
            return Vec::new();
        }
        let Some(input) = self.unmodified_shape(id) else {
            return Vec::new();
        };
        // The viewport's, and an export's for those it doesn't show.
        let mut purposes = vec![Purpose::Viewport];
        if modifiers.iter().any(|m| m.on && !m.in_viewport) {
            purposes.push(Purpose::Export);
        }
        let mut problems: Vec<(usize, String)> = Vec::new();
        for purpose in purposes {
            let found = match self.stack_state(id, purpose, Arc::clone(&input)) {
                Some(StackState::Ready(result)) => result.problems,
                Some(StackState::Needed(job)) if job.work() < MODIFIERS_IN_BACKGROUND_FROM => {
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

    /// The shapes `kind` reads for object `id`, each placed into `id`'s
    /// space; a source without a shape is left out.
    fn source_shapes(
        &self,
        id: ObjectId,
        kind: &ModifierKind,
        purpose: Purpose,
    ) -> Vec<SourceShape> {
        let into = self.shape_matrix(id);
        let back = if into.determinant().abs() >= 1e-12 {
            into.inverse()
        } else {
            glam::Mat4::IDENTITY
        };
        kind.sources()
            .into_iter()
            .filter(|source| *source != id)
            .filter_map(|source| {
                Some(SourceShape {
                    id: source,
                    mesh: self.final_shape(source, purpose)?,
                    matrix: back * self.shape_matrix(source),
                })
            })
            .collect()
    }
}
