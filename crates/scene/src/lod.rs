//! Levels of detail ([decision 0155](../../../docs/decisions/0155-levels-of-detail.md)):
//! simpler copies of an object's shape for a game to draw when it is far
//! away, each a share of its triangles ([`LodLevel::ratio`]) and shown
//! from a distance ([`LodLevel::distance`]).
//!
//! A per-object setting ([`Lods`], on [`crate::Object::lods`]), not a
//! modifier: the levels are extra shapes beside the object, made from its
//! final shape (every modifier included), as its collision shapes are
//! (decision 0151). The object itself never changes; the viewport can
//! show one level in its place to check it ([`Lods::preview`]), and an
//! export can write each level as its own mesh.
//!
//! The levels are made by Decimate's work ([`mesh::decimate::decimate_levels`],
//! decision 0145) in one go, each going on from the one before, so they
//! are nested (a coarser level's points are some of the finer one's) and
//! cost about as much as the smallest alone. They are kept per object
//! while the shape and settings stay the same; heavy shapes are worked on
//! another thread by the interface ([`LodJob`]), as heavy modifiers are.

use crate::Purpose;
use crate::object::ObjectId;
use crate::tree::Scene;
use mesh::Mesh;
use mesh::decimate::{DecimateOptions, Keep, decimate_levels, triangle_count};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use words::trf;

/// From this many triangles, the interface makes an object's levels on
/// another thread, as it does heavy modifiers
/// ([`crate::MODIFIERS_IN_BACKGROUND_FROM`]).
pub const LODS_IN_BACKGROUND_FROM: usize = 5_000;

/// One level of detail: how much of the shape it keeps, and from how far
/// away a game shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodLevel {
    /// The share of the full shape's triangles it keeps (1 is all).
    pub ratio: f32,
    /// How far from the camera, in centimetres, a game starts showing it
    /// instead of the level before.
    pub distance: f32,
}

/// An object's levels of detail (see the module notes).
#[derive(Debug, Clone, PartialEq)]
pub struct Lods {
    /// Whether it has levels of detail. Off, its settings are kept, so
    /// switching it on again brings them back.
    pub on: bool,
    /// The levels, from the most detailed to the simplest (the full shape,
    /// level 0, isn't listed). Between 1 and [`Lods::MAX_LEVELS`].
    pub levels: Vec<LodLevel>,
    /// Which level the viewport shows in the shape's place: 0 for the
    /// full shape, 1 for the first level and so on. Only the view changes.
    pub preview: usize,
}

impl Lods {
    /// The most levels an object can have.
    pub const MAX_LEVELS: usize = 8;
    /// How many levels a new setting has.
    pub const STANDARD_LEVELS: usize = 3;
    /// The farthest distance a level can be shown from, in centimetres:
    /// 10 km.
    pub const MAX_DISTANCE: f32 = 1_000_000.0;
    /// The standard levels: half, a quarter and an eighth of the triangles
    /// (12%), shown from 10 m, 20 m and 40 m.
    pub const STANDARD: [LodLevel; 3] = [
        LodLevel {
            ratio: 0.5,
            distance: 1_000.0,
        },
        LodLevel {
            ratio: 0.25,
            distance: 2_000.0,
        },
        LodLevel {
            ratio: 0.12,
            distance: 4_000.0,
        },
    ];

    /// The level that would come after `before` (or the first): half its
    /// triangles, shown from twice as far.
    pub fn next_level(before: Option<&LodLevel>) -> LodLevel {
        match before {
            None => Self::STANDARD[0],
            Some(b) => LodLevel {
                ratio: (b.ratio * 0.5).max(DecimateOptions::MIN_RATIO),
                distance: if b.distance > 0.0 {
                    (b.distance * 2.0).min(Self::MAX_DISTANCE)
                } else {
                    Self::STANDARD[0].distance
                },
            },
        }
    }

    /// The same settings with `count` levels (within 1 and
    /// [`Lods::MAX_LEVELS`]): levels added after the last as
    /// [`Lods::next_level`] makes them, or the last ones taken away.
    pub fn with_count(&self, count: usize) -> Self {
        let count = count.clamp(1, Self::MAX_LEVELS);
        let mut levels = self.levels.clone();
        levels.truncate(count);
        while levels.len() < count {
            levels.push(Self::next_level(levels.last()));
        }
        Self {
            levels,
            preview: self.preview.min(count),
            ..self.clone()
        }
    }

    /// The settings brought within limits: 1 to [`Lods::MAX_LEVELS`]
    /// levels (none at all is the standard three), ratios within
    /// Decimate's (one that isn't a number halves the level before),
    /// distances within 0 and [`Lods::MAX_DISTANCE`] (not a number is 0),
    /// and a preview of a level that is there.
    pub fn clamped(&self) -> Self {
        if self.levels.is_empty() {
            return Self {
                levels: Self::STANDARD.to_vec(),
                preview: self.preview.min(Self::STANDARD.len()),
                on: self.on,
            };
        }
        let mut levels: Vec<LodLevel> = Vec::new();
        for level in self.levels.iter().take(Self::MAX_LEVELS) {
            let ratio = if level.ratio.is_finite() {
                level.ratio.clamp(DecimateOptions::MIN_RATIO, 1.0)
            } else {
                Self::next_level(levels.last()).ratio
            };
            let distance = if level.distance.is_finite() {
                level.distance.clamp(0.0, Self::MAX_DISTANCE)
            } else {
                0.0
            };
            levels.push(LodLevel { ratio, distance });
        }
        Self {
            preview: self.preview.min(levels.len()),
            levels,
            on: self.on,
        }
    }

    /// Each level's target in triangles, for a shape of `triangles`: its
    /// ratio of them (at least one), and never more than the level
    /// before's, so the levels always get simpler.
    pub fn targets(&self, triangles: usize) -> Vec<usize> {
        let mut most = triangles;
        self.clamped()
            .levels
            .iter()
            .map(|level| {
                let target = DecimateOptions {
                    ratio: level.ratio,
                    ..DecimateOptions::default()
                }
                .target(triangles)
                .min(most);
                most = target;
                target
            })
            .collect()
    }
}

impl Default for Lods {
    fn default() -> Self {
        Self {
            on: false,
            levels: Self::STANDARD.to_vec(),
            preview: 0,
        }
    }
}

/// An object's levels worked out, in its shape's own space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LodShapes {
    /// Each level's shape, in the order of [`Lods::levels`]; empty when
    /// they couldn't be made.
    pub levels: Vec<Arc<Mesh>>,
    /// The full shape's triangles.
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

/// The levels `lods` asks for, made from `mesh` shaded by `hardness` (its
/// hard edges are kept, as Decimate keeps them).
pub fn lod_shapes(mesh: &Mesh, lods: &Lods, hardness: mesh::Hardness) -> LodShapes {
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

/// What an object's levels are made from, beside its shape.
#[derive(Debug, Clone, PartialEq)]
struct Setting {
    /// The levels' ratios, in order (their distances don't change them).
    ratios: Vec<f32>,
    hardness: mesh::Hardness,
}

impl Setting {
    fn lods(&self) -> Lods {
        Lods {
            on: true,
            levels: self
                .ratios
                .iter()
                .map(|ratio| LodLevel {
                    ratio: *ratio,
                    distance: 0.0,
                })
                .collect(),
            preview: 0,
        }
    }
}

/// An object's levels still to make.
#[derive(Debug, Clone)]
pub struct LodJob {
    id: ObjectId,
    purpose: Purpose,
    input: Arc<Mesh>,
    setting: Setting,
}

impl LodJob {
    /// About how much work it is, in triangles (see
    /// [`LODS_IN_BACKGROUND_FROM`]).
    pub fn work(&self) -> usize {
        triangle_count(&self.input)
    }

    /// Does the work, on any thread.
    pub fn run(self) -> LodDone {
        let shapes = lod_shapes(&self.input, &self.setting.lods(), self.setting.hardness);
        LodDone {
            id: self.id,
            purpose: self.purpose,
            kept: Kept {
                input: self.input,
                setting: self.setting,
                shapes: Arc::new(shapes),
                fresh: true,
            },
        }
    }
}

/// Levels made, to hand to [`Scene::store_lods`].
#[derive(Debug, Clone)]
pub struct LodDone {
    id: ObjectId,
    purpose: Purpose,
    kept: Kept,
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
}

/// Levels kept, with what they were made from.
#[derive(Debug, Clone)]
struct Kept {
    input: Arc<Mesh>,
    setting: Setting,
    shapes: Arc<LodShapes>,
    /// Whether it was made from the shape last asked about for its
    /// purpose: once a newer shape is asked about, it is kept to show
    /// meanwhile, but no longer said to be the object's.
    fresh: bool,
}

/// Each object's levels last made (not part of what the scene is: copies
/// start empty, and it never affects equality).
#[derive(Default)]
pub(crate) struct LodCache(Mutex<HashMap<(ObjectId, Purpose), Kept>>);

impl LodCache {
    /// Forgets `id`'s (deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(object, _), _| *object != id);
    }

    /// `id`'s for `purpose`, if it was made from the shape last asked
    /// about.
    fn fresh(&self, id: ObjectId, purpose: Purpose) -> Option<Kept> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(id, purpose))
            .filter(|kept| kept.fresh)
            .cloned()
    }

    /// `id`'s for `purpose` if it was made with `setting` from `shape`;
    /// otherwise, as a newer shape is now asked about, the one kept is no
    /// longer fresh.
    fn made_from(
        &self,
        id: ObjectId,
        purpose: Purpose,
        setting: &Setting,
        shape: &Arc<Mesh>,
    ) -> Option<Kept> {
        let mut kept = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = kept.get_mut(&(id, purpose))?;
        if entry.setting == *setting
            && (Arc::ptr_eq(&entry.input, shape) || *entry.input == **shape)
        {
            entry.fresh = true;
            return Some(entry.clone());
        }
        entry.fresh = false;
        None
    }
}

impl LodCache {
    /// A copy holding the same results, for
    /// [`crate::Scene::copy_with_results`].
    pub(crate) fn copied(&self) -> Self {
        Self(Mutex::new(
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        ))
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
    /// What the object's levels are made from, if it has them on.
    fn lod_setting(&self, id: ObjectId) -> Option<Setting> {
        let object = self.object(id)?;
        if !object.lods.on || !object.kind.has_shape() {
            return None;
        }
        // Shaded as it is drawn: an instance as its original.
        let hardness = self
            .object(self.modifier_owner(id))
            .map_or(mesh::Hardness::Marked, |o| o.hardness);
        Some(Setting {
            ratios: object
                .lods
                .clamped()
                .levels
                .iter()
                .map(|l| l.ratio)
                .collect(),
            hardness,
        })
    }

    /// Whether the object's levels for `shape` (its final shape, as the
    /// viewport shows it or an export makes it) are made, or the work to
    /// do. `None` when it has no levels of detail on or isn't in the
    /// scene.
    pub fn lod_state(&self, id: ObjectId, purpose: Purpose, shape: Arc<Mesh>) -> Option<LodState> {
        let setting = self.lod_setting(id)?;
        if let Some(kept) = self.lods.made_from(id, purpose, &setting, &shape) {
            return Some(LodState::Ready(kept.shapes));
        }
        Some(LodState::Needed(LodJob {
            id,
            purpose,
            input: shape,
            setting,
        }))
    }

    /// Keeps levels made elsewhere ([`LodJob::run`]), for
    /// [`Scene::lod_state`] to find while the shape and settings stay the
    /// same.
    pub fn store_lods(&self, done: LodDone) {
        self.lods
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((done.id, done.purpose), done.kept);
    }

    /// The object's levels for `shape`, made here, on this thread, when
    /// not kept from before (an export's). `None` as for
    /// [`Scene::lod_state`].
    pub fn lod_shapes(
        &self,
        id: ObjectId,
        purpose: Purpose,
        shape: Arc<Mesh>,
    ) -> Option<Arc<LodShapes>> {
        match self.lod_state(id, purpose, shape)? {
            LodState::Ready(shapes) => Some(shapes),
            LodState::Needed(job) => {
                let done = job.run();
                let shapes = done.shapes();
                self.store_lods(done);
                Some(shapes)
            }
        }
    }

    /// The object's levels last made, if they were made with its current
    /// settings (for the Attributes panel's triangle counts; never makes
    /// anything itself).
    pub fn lods_if_ready(&self, id: ObjectId) -> Option<Arc<LodShapes>> {
        let setting = self.lod_setting(id)?;
        // The viewport's, for the shape it last asked about.
        let kept = self.lods.fresh(id, Purpose::Viewport)?;
        (kept.setting == setting).then_some(kept.shapes)
    }
}
