//! Levels of detail ([decision 0155](../../../docs/decisions/0155-levels-of-detail.md),
//! [decision 0165](../../../docs/decisions/0165-levels-panel.md)): simpler
//! copies of an object's shape for a game to draw when it is far away,
//! each shown from a distance ([`LodLevel::distance`]).
//!
//! A per-object setting ([`Lods`], on [`crate::Object::lods`]), not a
//! modifier: the levels are extra shapes beside the object. The object
//! itself never changes; the viewport can show one level in its place to
//! check it ([`Lods::preview`]), and an export can write each level as its
//! own mesh.
//!
//! A level is made in one of three ways (decision 0165):
//!
//! 1. **With its own changes to the modifiers** ([`LodLevel::overrides`]):
//!    the object's shape worked out again for that level, with chosen
//!    modifiers switched off or given simpler settings (one bevel segment,
//!    Subdivide off). It stays live: editing the model changes every level.
//! 2. **Reduced automatically** ([`LodLevel::reduce`]): Decimate's work
//!    ([`mesh::decimate::decimate_levels`], decision 0145) as a last step,
//!    keeping a share of the level's triangles ([`LodLevel::ratio`]).
//!    Levels made from the same shape are reduced in one go, each going on
//!    from the one before, so they are nested and cost about as much as
//!    the smallest alone.
//! 3. **By hand** ([`LodLevel::hand`]): a shape of its own, inside the
//!    object in the Objects panel, edited like any other; it no longer
//!    follows the model ([`crate::Scene::level_drifted`] says when the
//!    model has changed since it was made).
//!
//! The levels are worked out in [`make`] and kept per object while what
//! they are made from stays the same; heavy ones on another thread by the
//! interface ([`LodJob`]), as heavy modifiers are.

mod hand;
mod make;

pub use hand::level_name;
pub(crate) use make::LodCache;
pub use make::{LODS_IN_BACKGROUND_FROM, LodDone, LodJob, LodShapes, LodState, lod_shapes};

use crate::modifier::{MAX_MODIFIERS, Modifier, ModifierId, ModifierKind};
use crate::object::ObjectId;
use mesh::decimate::DecimateOptions;

/// One level of detail: how it is made, and from how far away a game shows
/// it.
#[derive(Debug, Clone, PartialEq)]
pub struct LodLevel {
    /// With [`Self::reduce`], the share of the level's triangles (after its
    /// changes to the modifiers) it keeps: 1 is all.
    pub ratio: f32,
    /// How far from the camera, in centimetres, a game starts showing it
    /// instead of the level before.
    pub distance: f32,
    /// Whether the level is reduced automatically as a last step, to
    /// [`Self::ratio`] of its triangles (decision 0165; on for every level
    /// made before, which were made that way only).
    pub reduce: bool,
    /// The triangles aimed for (0: no aim). Only shown beside the level's
    /// count, in amber when it has more; nothing is made to fit it.
    pub budget: u32,
    /// The level's own changes to the object's modifiers, by the
    /// modifier's id: at most one each.
    pub overrides: Vec<ModifierOverride>,
    /// The level's own shape, when it is made by hand.
    pub hand: Option<HandLevel>,
}

impl LodLevel {
    /// A level reduced automatically to `ratio` of the triangles, shown
    /// from `distance` centimetres, with no changes to the modifiers and
    /// no budget.
    pub const fn new(ratio: f32, distance: f32) -> Self {
        Self {
            ratio,
            distance,
            reduce: true,
            budget: 0,
            overrides: Vec::new(),
            hand: None,
        }
    }

    /// The level's change to modifier `id`, if it has one.
    pub fn override_for(&self, id: ModifierId) -> Option<&ModifierOverride> {
        self.overrides.iter().find(|o| o.modifier == id)
    }

    /// `modifiers` (the object's) as this level has them: each one its
    /// change names switched on or off, with its settings if it gives
    /// them.
    pub fn applied(&self, modifiers: &[Modifier]) -> Vec<Modifier> {
        modifiers
            .iter()
            .map(|m| match self.override_for(m.id) {
                Some(o) => Modifier {
                    on: o.on,
                    kind: o
                        .settings
                        .as_ref()
                        .filter(|k| k.same_kind(&m.kind))
                        .map_or_else(|| m.kind.clone(), ModifierKind::clamped),
                    ..m.clone()
                },
                None => m.clone(),
            })
            .collect()
    }

    /// The same level with modifier `modifier` (one of `modifiers`, the
    /// object's) switched `on` or off and given `settings` (`None`: the
    /// model's) at this level. A change that changes nothing is taken
    /// away, so the level follows the model again.
    pub fn with_override(
        &self,
        modifiers: &[Modifier],
        modifier: ModifierId,
        on: bool,
        settings: Option<ModifierKind>,
    ) -> Self {
        let mut level = self.clone();
        level.overrides.retain(|o| o.modifier != modifier);
        level.overrides.push(ModifierOverride {
            modifier,
            on,
            settings,
        });
        level.overrides = tidied_overrides(&level.overrides, modifiers);
        level
    }
}

/// A level's change to one of the object's modifiers (decision 0165).
#[derive(Debug, Clone, PartialEq)]
pub struct ModifierOverride {
    /// Which modifier, by its id ([`crate::Modifier::id`]).
    pub modifier: ModifierId,
    /// Whether it is on at this level.
    pub on: bool,
    /// Its settings at this level, of its own kind; `None` for the
    /// model's.
    pub settings: Option<ModifierKind>,
}

/// A level made by hand (decision 0165): its own shape, an object inside
/// the object with the levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandLevel {
    /// The object holding the level's shape. It is the level's only while
    /// it is inside the object with the levels and has a shape; otherwise
    /// the level is made from the model again.
    pub shape: ObjectId,
    /// A fingerprint of the model it was made from
    /// ([`crate::Scene::model_fingerprint`]), to tell when the model has
    /// changed since.
    pub made_from: u64,
}

/// `overrides` kept to those that mean something for `modifiers`: naming
/// one of them, once, with settings of its kind (brought within limits),
/// and changing something.
pub(crate) fn tidied_overrides(
    overrides: &[ModifierOverride],
    modifiers: &[Modifier],
) -> Vec<ModifierOverride> {
    let mut out: Vec<ModifierOverride> = Vec::new();
    for o in overrides {
        let Some(m) = modifiers.iter().find(|m| m.id == o.modifier) else {
            continue;
        };
        if out.iter().any(|kept| kept.modifier == o.modifier) {
            continue;
        }
        let settings = o
            .settings
            .as_ref()
            .filter(|k| k.same_kind(&m.kind))
            .map(ModifierKind::clamped)
            .filter(|k| *k != m.kind);
        if o.on == m.on && settings.is_none() {
            continue;
        }
        out.push(ModifierOverride {
            modifier: o.modifier,
            on: o.on,
            settings,
        });
    }
    out
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
    /// Which level the viewport shows: 0 for the full shape, 1 for the
    /// first level and so on. Only the view changes.
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
    /// The largest triangle budget a level can have.
    pub const MAX_BUDGET: u32 = 100_000_000;
    /// The standard levels: half, a quarter and an eighth of the triangles
    /// (12%), shown from 10 m, 20 m and 40 m.
    pub const STANDARD: [LodLevel; 3] = [
        LodLevel::new(0.5, 1_000.0),
        LodLevel::new(0.25, 2_000.0),
        LodLevel::new(0.12, 4_000.0),
    ];

    /// The level that would come after `before` (or the first): reduced to
    /// half its triangles, shown from twice as far, with no changes to the
    /// modifiers.
    pub fn next_level(before: Option<&LodLevel>) -> LodLevel {
        match before {
            None => Self::STANDARD[0].clone(),
            Some(b) => LodLevel::new(
                (b.ratio * 0.5).max(DecimateOptions::MIN_RATIO),
                if b.distance > 0.0 {
                    (b.distance * 2.0).min(Self::MAX_DISTANCE)
                } else {
                    Self::STANDARD[0].distance
                },
            ),
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
    /// budgets up to [`Lods::MAX_BUDGET`], at most one change per modifier
    /// (at most [`MAX_MODIFIERS`]), and a preview of a level that is there.
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
            let mut overrides: Vec<ModifierOverride> = Vec::new();
            for o in level.overrides.iter() {
                if overrides.len() < MAX_MODIFIERS
                    && !overrides.iter().any(|kept| kept.modifier == o.modifier)
                {
                    overrides.push(ModifierOverride {
                        settings: o.settings.as_ref().map(ModifierKind::clamped),
                        ..o.clone()
                    });
                }
            }
            levels.push(LodLevel {
                ratio,
                distance,
                reduce: level.reduce,
                budget: level.budget.min(Self::MAX_BUDGET),
                overrides,
                hand: level.hand,
            });
        }
        Self {
            preview: self.preview.min(levels.len()),
            levels,
            on: self.on,
        }
    }

    /// [`Self::clamped`], with each level's changes kept to those that mean
    /// something for `modifiers` (the object's: see [`tidied_overrides`]).
    pub fn tidied(&self, modifiers: &[Modifier]) -> Self {
        let mut lods = self.clamped();
        for level in &mut lods.levels {
            level.overrides = tidied_overrides(&level.overrides, modifiers);
        }
        lods
    }

    /// The objects holding its levels made by hand, by level.
    pub fn hand_shapes(&self) -> Vec<(usize, ObjectId)> {
        self.levels
            .iter()
            .enumerate()
            .filter_map(|(k, l)| l.hand.map(|h| (k, h.shape)))
            .collect()
    }

    /// Each level's target in triangles when reduced, for a shape of
    /// `triangles`: its ratio of them (at least one), and never more than
    /// the level before's, so the levels always get simpler.
    pub fn targets(&self, triangles: usize) -> Vec<usize> {
        let ratios: Vec<f32> = self.clamped().levels.iter().map(|l| l.ratio).collect();
        targets(&ratios, triangles)
    }
}

/// The targets in triangles for levels reduced to `ratios` of a shape of
/// `triangles`, in order: each its ratio of them (at least one), never
/// more than the one before's.
pub(crate) fn targets(ratios: &[f32], triangles: usize) -> Vec<usize> {
    let mut most = triangles;
    ratios
        .iter()
        .map(|ratio| {
            let target = DecimateOptions {
                ratio: *ratio,
                ..DecimateOptions::default()
            }
            .target(triangles)
            .min(most);
            most = target;
            target
        })
        .collect()
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
