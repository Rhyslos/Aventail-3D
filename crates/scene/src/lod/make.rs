//! Working out an object's levels of detail (decisions 0155 and 0165):
//! each level's shape (the model's, the model with the level's changes to
//! its modifiers, or a shape made by hand), then the automatic reduction
//! of the levels that have it, levels made from the same shape reduced in
//! one go so they are nested. The results are kept per object while what
//! they are made from stays the same; heavy work is handed out as a
//! [`LodJob`] to run on another thread, as heavy modifiers are.

use crate::modifier::{Purpose, StackDone, StackJob, StackState};
use crate::object::ObjectId;
use crate::tree::Scene;
use glam::Mat4;
use mesh::Mesh;
use mesh::decimate::{Keep, decimate_levels, triangle_count};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use words::trf;

/// From this much work (in triangles, or a modifier's estimate), the
/// interface makes an object's levels on another thread, as it does heavy
/// modifiers ([`crate::MODIFIERS_IN_BACKGROUND_FROM`]).
pub const LODS_IN_BACKGROUND_FROM: usize = 5_000;

/// An object's levels worked out, in its shape's own space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LodShapes {
    /// Each level's shape, in the order of [`super::Lods::levels`]; empty
    /// when they couldn't be made.
    pub levels: Vec<Arc<Mesh>>,
    /// The full shape's triangles (as exported).
    pub full: usize,
    /// Why there are no levels, in plain words starting with a capital.
    pub problem: Option<String>,
}

impl LodShapes {
    /// Each level's triangles, as a game engine counts them.
    pub fn triangles(&self) -> Vec<usize> {
        self.levels.iter().map(|m| triangle_count(m)).collect()
    }
}

/// The levels `lods` asks for, every one reduced from `mesh` shaded by
/// `hardness` (its hard edges are kept, as Decimate keeps them): what a
/// shape whose levels change nothing in its modifiers gets.
pub fn lod_shapes(mesh: &Mesh, lods: &super::Lods, hardness: mesh::Hardness) -> LodShapes {
    let full = triangle_count(mesh);
    let targets = lods.targets(full);
    match decimate_levels(mesh, Keep::default(), hardness, &targets, false) {
        Ok(levels) => LodShapes {
            levels: levels.into_iter().map(Arc::new).collect(),
            full,
            problem: None,
        },
        Err(why) => LodShapes {
            levels: Vec::new(),
            full,
            problem: Some(trf!("No levels of detail: {}.", why)),
        },
    }
}

/// A shape worked out, or the work still to do for it.
#[derive(Debug, Clone)]
enum Pending {
    Ready(Arc<Mesh>),
    Work(StackJob),
}

impl Pending {
    fn ready(&self) -> Option<&Arc<Mesh>> {
        match self {
            Self::Ready(mesh) => Some(mesh),
            Self::Work(_) => None,
        }
    }

    fn work(&self) -> usize {
        match self {
            Self::Ready(_) => 0,
            Self::Work(job) => job.work(),
        }
    }

    /// The shape, working it out if needed (reusing what `earlier` worked
    /// out on the way); the work done is added to `dones`.
    fn resolve(self, dones: &mut Vec<StackDone>, earlier: Option<&StackDone>) -> Arc<Mesh> {
        match self {
            Self::Ready(mesh) => mesh,
            Self::Work(job) => {
                let job = match earlier {
                    Some(done) => job.reusing(done),
                    None => job,
                };
                let done = job.run();
                let mesh = done.result().mesh;
                dones.push(done);
                mesh
            }
        }
    }
}

/// What one level is made from.
#[derive(Debug, Clone)]
enum From {
    /// The model's full shape.
    Full,
    /// The model with the level's changes to its modifiers.
    Own(Pending),
    /// A shape made by hand, placed into the model's space by the matrix
    /// (none when it sits exactly on the model).
    Hand(Pending, Option<Mat4>),
}

/// One level still to make.
#[derive(Debug, Clone)]
struct Planned {
    from: From,
    /// The share of the triangles it is reduced to, if it is.
    reduce: Option<f32>,
}

/// An object's levels kept, with what they were made from.
#[derive(Debug, Clone)]
struct Kept {
    full: Arc<Mesh>,
    /// Each level's shape before its reduction.
    inputs: Vec<Arc<Mesh>>,
    reduces: Vec<Option<f32>>,
    hardness: mesh::Hardness,
    shapes: Arc<LodShapes>,
    /// Whether it was made from what the object holds now, as last asked:
    /// once anything changes it is kept to show meanwhile, but no longer
    /// said to be the object's.
    fresh: bool,
}

impl Kept {
    fn made_from(
        &self,
        full: &Arc<Mesh>,
        inputs: &[Arc<Mesh>],
        reduces: &[Option<f32>],
        hardness: mesh::Hardness,
    ) -> bool {
        self.hardness == hardness
            && self.reduces == reduces
            && same(&self.full, full)
            && self.inputs.len() == inputs.len()
            && self.inputs.iter().zip(inputs).all(|(a, b)| same(a, b))
    }
}

/// `mesh` with every point placed by `matrix`.
fn place(mesh: &Mesh, matrix: Mat4) -> Mesh {
    let mut placed = mesh.clone();
    mesh::parts::place(&mut placed, matrix);
    placed
}

/// Whether two shapes are the same: the same one, or equal.
fn same(a: &Arc<Mesh>, b: &Arc<Mesh>) -> bool {
    Arc::ptr_eq(a, b) || **a == **b
}

/// An object's levels still to make, gathered from the scene so the work
/// can be done on another thread (it holds only shared shapes and
/// settings).
#[derive(Debug, Clone)]
pub struct LodJob {
    id: ObjectId,
    full: Pending,
    levels: Vec<Planned>,
    hardness: mesh::Hardness,
    /// What was made last, for the levels that haven't changed.
    kept: Option<Kept>,
}

impl LodJob {
    /// About how much work it is, in triangles or the modifiers' estimate
    /// (see [`LODS_IN_BACKGROUND_FROM`]).
    pub fn work(&self) -> usize {
        let mut work = self.full.work();
        let mut full_reduced = false;
        for level in &self.levels {
            match &level.from {
                // Levels made from the model are reduced in one go.
                From::Full => full_reduced |= level.reduce.is_some(),
                From::Own(p) | From::Hand(p, _) => {
                    work = work.saturating_add(p.work());
                    if level.reduce.is_some() {
                        work = work.saturating_add(p.ready().map_or(0, |m| triangle_count(m)));
                    }
                }
            }
        }
        if full_reduced {
            work = work.saturating_add(self.full.ready().map_or(0, |m| triangle_count(m)));
        }
        work
    }

    /// Does the work, on any thread.
    pub fn run(self) -> LodDone {
        let mut stacks: Vec<StackDone> = Vec::new();
        let full = self.full.resolve(&mut stacks, None);
        let full_done = stacks.first().cloned();
        let mut placed = Vec::new();
        let mut inputs: Vec<Arc<Mesh>> = Vec::with_capacity(self.levels.len());
        for (k, level) in self.levels.iter().enumerate() {
            inputs.push(match level.from.clone() {
                From::Full => Arc::clone(&full),
                From::Own(p) => p.resolve(&mut stacks, full_done.as_ref()),
                From::Hand(p, None) => p.resolve(&mut stacks, None),
                From::Hand(p, Some(matrix)) => {
                    let from = p.resolve(&mut stacks, None);
                    let result = Arc::new(place(&from, matrix));
                    placed.push((
                        k,
                        Placed {
                            from,
                            matrix,
                            result: Arc::clone(&result),
                        },
                    ));
                    result
                }
            });
        }
        let reduces: Vec<Option<f32>> = self.levels.iter().map(|l| l.reduce).collect();
        let shapes = reduced(&full, &inputs, &reduces, self.hardness, self.kept.as_ref());
        LodDone {
            id: self.id,
            kept: Kept {
                full,
                inputs,
                reduces,
                hardness: self.hardness,
                shapes: Arc::new(shapes),
                fresh: true,
            },
            stacks,
            placed,
        }
    }
}

/// The levels made from `inputs` (each level's shape before its
/// reduction), each reduced if `reduces` says so: levels side by side made
/// from the same shape are reduced in one go, nested. Groups unchanged
/// since `kept` are kept.
fn reduced(
    full: &Arc<Mesh>,
    inputs: &[Arc<Mesh>],
    reduces: &[Option<f32>],
    hardness: mesh::Hardness,
    kept: Option<&Kept>,
) -> LodShapes {
    let mut levels: Vec<Arc<Mesh>> = Vec::with_capacity(inputs.len());
    let kept_groups = kept.map(|k| groups(&k.inputs, &k.reduces));
    for range in groups(inputs, reduces) {
        let Some(_) = reduces[range.start] else {
            levels.push(Arc::clone(&inputs[range.start]));
            continue;
        };
        let reusable = kept.zip(kept_groups.as_ref()).filter(|(k, g)| {
            k.hardness == hardness
                && k.shapes.problem.is_none()
                && k.shapes.levels.len() == k.inputs.len()
                && g.contains(&range)
                && range
                    .clone()
                    .all(|j| k.reduces[j] == reduces[j] && same(&k.inputs[j], &inputs[j]))
        });
        if let Some((k, _)) = reusable {
            levels.extend(range.map(|j| Arc::clone(&k.shapes.levels[j])));
            continue;
        }
        let input = &inputs[range.start];
        let ratios: Vec<f32> = range.clone().filter_map(|j| reduces[j]).collect();
        let targets = super::targets(&ratios, triangle_count(input));
        match decimate_levels(input, Keep::default(), hardness, &targets, false) {
            Ok(made) => levels.extend(made.into_iter().map(Arc::new)),
            Err(why) => {
                return LodShapes {
                    levels: Vec::new(),
                    full: triangle_count(full),
                    problem: Some(trf!("No levels of detail: {}.", why)),
                };
            }
        }
    }
    LodShapes {
        levels,
        full: triangle_count(full),
        problem: None,
    }
}

/// The levels in groups made together: a level not reduced alone, and
/// reduced levels side by side made from the same shape together.
fn groups(inputs: &[Arc<Mesh>], reduces: &[Option<f32>]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < inputs.len() {
        let mut end = start + 1;
        if reduces[start].is_some() {
            while end < inputs.len()
                && reduces[end].is_some()
                && Arc::ptr_eq(&inputs[end], &inputs[start])
            {
                end += 1;
            }
        }
        out.push(start..end);
        start = end;
    }
    out
}

/// Levels made, to hand to [`Scene::store_lods`].
#[derive(Debug, Clone)]
pub struct LodDone {
    id: ObjectId,
    kept: Kept,
    /// The shapes worked out on the way (the model's, a level's with its
    /// changes, a hand-made level's modifiers), kept with them.
    stacks: Vec<StackDone>,
    /// The hand-made levels placed on the way, by level.
    placed: Vec<(usize, Placed)>,
}

impl LodDone {
    /// The levels.
    pub fn shapes(&self) -> Arc<LodShapes> {
        Arc::clone(&self.kept.shapes)
    }
}

/// Whether an object's levels are made.
#[derive(Debug, Clone)]
pub enum LodState {
    /// They are.
    Ready(Arc<LodShapes>),
    /// They still need making.
    Needed(LodJob),
    /// What they are made from isn't worked out yet (heavy booleans the
    /// interface works out on another thread): ask again later.
    Waiting,
}

/// Each object's levels last made (not part of what the scene is: copies
/// start empty, and it never affects equality), and the model fingerprints
/// last worked out ([`crate::Scene::model_fingerprint`]).
#[derive(Default)]
pub(crate) struct LodCache {
    kept: Mutex<HashMap<ObjectId, Kept>>,
    pub(super) fingerprints: Mutex<HashMap<ObjectId, (Arc<Mesh>, u64)>>,
    /// Hand-made levels placed into their model's space, by object and
    /// level: the shape placed, where, and the result.
    placed: Mutex<HashMap<(ObjectId, usize), Placed>>,
}

/// A hand-made level's shape placed into its model's space.
#[derive(Debug, Clone)]
struct Placed {
    from: Arc<Mesh>,
    matrix: Mat4,
    result: Arc<Mesh>,
}

impl LodCache {
    /// Forgets `id`'s (deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.kept
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&id);
        self.fingerprints
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&id);
        self.placed
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(object, _), _| *object != id);
    }

    /// `mesh` (object `id`'s level `k`, made by hand) placed by `matrix`,
    /// kept while both stay the same.
    fn placed(&self, id: ObjectId, k: usize, mesh: &Arc<Mesh>, matrix: Mat4) -> Arc<Mesh> {
        let mut placed = self
            .placed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(p) = placed.get(&(id, k))
            && Arc::ptr_eq(&p.from, mesh)
            && p.matrix == matrix
        {
            return Arc::clone(&p.result);
        }
        let result = Arc::new(place(mesh, matrix));
        placed.insert(
            (id, k),
            Placed {
                from: Arc::clone(mesh),
                matrix,
                result: Arc::clone(&result),
            },
        );
        result
    }

    fn get(&self, id: ObjectId) -> Option<Kept> {
        self.kept
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&id)
            .cloned()
    }

    fn set_fresh(&self, id: ObjectId, fresh: bool) {
        if let Some(kept) = self
            .kept
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(&id)
        {
            kept.fresh = fresh;
        }
    }

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
            kept: copy(&self.kept),
            fingerprints: copy(&self.fingerprints),
            placed: copy(&self.placed),
        }
    }
}

impl Clone for LodCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for LodCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for LodCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LodCache").finish_non_exhaustive()
    }
}

impl Scene {
    /// The shape `id` has for `purpose`, or the work still to do for it.
    /// `None` when it has no shape, or (with `light_only`) while its
    /// booleans are heavy and still to work out.
    fn pending_shape(&self, id: ObjectId, purpose: Purpose, light_only: bool) -> Option<Pending> {
        let input = if light_only {
            self.input_if_light(id)?
        } else {
            self.unmodified_shape(id)?
        };
        Some(match self.stack_state(id, purpose, Arc::clone(&input)) {
            None => Pending::Ready(input),
            Some(StackState::Ready(result)) => Pending::Ready(result.mesh),
            Some(StackState::Needed(job)) => Pending::Work(job),
        })
    }

    /// What the object's levels are made from, or `None` without levels
    /// of detail on (or with `light_only`, while something they need is
    /// heavy and worked out elsewhere: `Some(None)`).
    fn lod_plan(&self, id: ObjectId, light_only: bool) -> Option<Option<LodJob>> {
        self.lod_plan_as(id, light_only, None)
    }

    /// [`Self::lod_plan`], with level `automatic` (if any) made from the
    /// model even if it is made by hand.
    pub(super) fn lod_plan_as(
        &self,
        id: ObjectId,
        light_only: bool,
        automatic: Option<usize>,
    ) -> Option<Option<LodJob>> {
        let object = self.object(id)?;
        if !object.lods.on || !object.kind.has_shape() {
            return None;
        }
        let lods = object.lods.clamped();
        // Shaded as it is drawn: an instance as its original.
        let hardness = self
            .object(self.modifier_owner(id))
            .map_or(mesh::Hardness::Marked, |o| o.hardness);
        let Some(full) = self.pending_shape(id, Purpose::Export, light_only) else {
            return Some(None);
        };
        let mut levels = Vec::with_capacity(lods.levels.len());
        for (k, level) in lods.levels.iter().enumerate() {
            let hand = self.level_shape(id, k).filter(|_| automatic != Some(k));
            let planned = if let Some((shape, place)) = hand {
                let Some(pending) = self.pending_shape(shape, Purpose::Export, light_only) else {
                    return Some(None);
                };
                // Placed here when it is ready (kept, so asking again gives
                // the same shape); by the job otherwise.
                let from = match (pending, place) {
                    (Pending::Ready(mesh), Some(matrix)) => {
                        From::Hand(Pending::Ready(self.lods.placed(id, k, &mesh, matrix)), None)
                    }
                    (pending, place) => From::Hand(pending, place),
                };
                Planned { from, reduce: None }
            } else {
                let from = if self.level_changes_modifiers(id, k) {
                    let purpose = Purpose::Level(u8::try_from(k).unwrap_or(u8::MAX));
                    match self.pending_shape(id, purpose, light_only) {
                        Some(pending) => From::Own(pending),
                        None => return Some(None),
                    }
                } else {
                    From::Full
                };
                Planned {
                    from,
                    reduce: level.reduce.then_some(level.ratio),
                }
            };
            levels.push(planned);
        }
        Some(Some(LodJob {
            id,
            full,
            levels,
            hardness,
            kept: self.lods.get(id),
        }))
    }

    /// Whether level `k` of object `id` (0 for Level 1) has changes to the
    /// modifiers that change what is worked out for it.
    fn level_changes_modifiers(&self, id: ObjectId, k: usize) -> bool {
        let Some(level) = self.object(id).and_then(|o| o.lods.levels.get(k)) else {
            return false;
        };
        let modifiers = self.modifiers(self.modifier_owner(id));
        let applies = |m: &crate::Modifier| m.applies(Purpose::Export);
        let model: Vec<_> = modifiers
            .iter()
            .filter(|m| applies(m))
            .map(|m| &m.kind)
            .collect();
        let at_level = level.applied(modifiers);
        let changed: Vec<_> = at_level
            .iter()
            .filter(|m| applies(m))
            .map(|m| &m.kind)
            .collect();
        model != changed
    }

    /// The modifiers object `id`'s shape gets at `purpose`: its own (an
    /// instance's original's), with a level's changes for a level.
    pub(crate) fn modifiers_for(&self, id: ObjectId, purpose: Purpose) -> Vec<crate::Modifier> {
        let modifiers = self.modifiers(self.modifier_owner(id));
        match purpose {
            Purpose::Level(k) => self
                .object(id)
                .and_then(|o| o.lods.levels.get(usize::from(k)))
                .map_or_else(|| modifiers.to_vec(), |level| level.applied(modifiers)),
            _ => modifiers.to_vec(),
        }
    }

    /// Whether the object's levels are made, or the work to do. `None` when
    /// it has no levels of detail on or isn't in the scene. Nothing heavy
    /// is worked out here.
    pub fn lod_state(&self, id: ObjectId) -> Option<LodState> {
        let Some(job) = self.lod_plan(id, true)? else {
            return Some(LodState::Waiting);
        };
        let ready = job.full.ready().cloned().zip(
            job.levels
                .iter()
                .map(|level| match &level.from {
                    From::Full => job.full.ready().cloned(),
                    From::Own(p) => p.ready().cloned(),
                    From::Hand(p, None) => p.ready().cloned(),
                    From::Hand(_, Some(_)) => None,
                })
                .collect::<Option<Vec<_>>>(),
        );
        let reduces: Vec<Option<f32>> = job.levels.iter().map(|l| l.reduce).collect();
        if let (Some(kept), Some((full, inputs))) = (&job.kept, ready)
            && kept.made_from(&full, &inputs, &reduces, job.hardness)
        {
            self.lods.set_fresh(id, true);
            return Some(LodState::Ready(Arc::clone(&kept.shapes)));
        }
        self.lods.set_fresh(id, false);
        Some(LodState::Needed(job))
    }

    /// Keeps levels made elsewhere ([`LodJob::run`]), with the shapes
    /// worked out on the way, for [`Scene::lod_state`] to find while what
    /// they are made from stays the same.
    pub fn store_lods(&self, done: LodDone) {
        let done = self.store_lod_stacks(done);
        self.lods
            .kept
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(done.id, done.kept);
    }

    /// Keeps the shapes worked out on the way to `done`'s levels, and hands
    /// back the rest.
    pub(super) fn store_lod_stacks(&self, mut done: LodDone) -> LodDone {
        for stack in std::mem::take(&mut done.stacks) {
            self.store_stack(stack);
        }
        self.lods
            .placed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend(
                std::mem::take(&mut done.placed)
                    .into_iter()
                    .map(|(k, p)| ((done.id, k), p)),
            );
        done
    }

    /// The object's levels, made here, on this thread, when not kept from
    /// before (for exports, and making a level by hand). `None` as for
    /// [`Scene::lod_state`].
    pub fn lod_shapes(&self, id: ObjectId) -> Option<Arc<LodShapes>> {
        if let Some(LodState::Ready(shapes)) = self.lod_state(id) {
            return Some(shapes);
        }
        let job = self.lod_plan(id, false)??;
        let done = job.run();
        let shapes = done.shapes();
        self.store_lods(done);
        Some(shapes)
    }

    /// The object's levels last made, if they were made from what it holds
    /// now as last asked (for the Levels panel's triangle counts; never
    /// makes anything itself).
    pub fn lods_if_ready(&self, id: ObjectId) -> Option<Arc<LodShapes>> {
        let object = self.object(id)?;
        if !object.lods.on {
            return None;
        }
        let kept = self.lods.get(id).filter(|k| k.fresh)?;
        let lods = object.lods.clamped();
        let reduces: Vec<Option<f32>> = lods
            .levels
            .iter()
            .enumerate()
            .map(|(k, l)| (l.reduce && self.level_shape(id, k).is_none()).then_some(l.ratio))
            .collect();
        (kept.reduces == reduces).then_some(kept.shapes)
    }
}
