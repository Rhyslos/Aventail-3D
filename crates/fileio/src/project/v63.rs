//! Version 63 of the project file ([decision 0165](../../../../docs/decisions/0165-levels-panel.md)):
//! version 62's, with each modifier's id (`v43::ModifierV43::id`) and,
//! per object, what version 56 didn't store of its levels of detail
//! ([`LevelsV63`], in `v43::ProjectV43::levels`): whether each level is
//! reduced, its triangle budget, its changes to the modifiers (by the
//! modifiers' ids) and its shape made by hand (an object inside it, saved
//! as every object is).
//!
//! Files before version 63 hold none of it: their modifiers get ids in
//! order, and their levels are reduced, as they were made then.

use super::v1::Problem;
use super::v43::KindV43;
use crate::limits::Bounded;
use scene::{Document, HandLevel, ModifierId, ModifierOverride, ObjectId};
use serde::{Deserialize, Serialize};
use words::tr;

/// One object's levels' details, level by level (in the order of version
/// 56's levels).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LevelsV63 {
    /// The object.
    pub(crate) id: u64,
    /// Its levels' details; more than the object has are left out.
    pub(crate) levels: Bounded<LevelV63>,
}

/// One level's details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LevelV63 {
    /// Whether it is reduced automatically.
    pub(crate) reduce: bool,
    /// The triangles aimed for (0: none).
    pub(crate) budget: u32,
    /// Its changes to the modifiers.
    pub(crate) overrides: Bounded<OverrideV63>,
    /// Its shape made by hand: the object, and the model's fingerprint
    /// when it was made.
    pub(crate) hand: Option<(u64, u64)>,
}

/// A level's change to one modifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OverrideV63 {
    /// The modifier's id.
    pub(crate) modifier: u32,
    /// Whether it is on at the level.
    pub(crate) on: bool,
    /// Its settings at the level, if its own.
    pub(crate) settings: Option<KindV43>,
}

/// The stored details of every object whose levels have any that aren't
/// the standard (every level reduced, no budget, no changes, none by
/// hand). A level's shape made by hand is stored only while that object
/// is there.
pub(crate) fn levels_of(document: &Document) -> Bounded<LevelsV63> {
    let scene = document.scene();
    scene
        .walk()
        .iter()
        .filter_map(|(id, _)| {
            let lods = &scene.object(*id)?.lods;
            let levels: Vec<LevelV63> = lods
                .levels
                .iter()
                .map(|l| LevelV63 {
                    reduce: l.reduce,
                    budget: l.budget,
                    overrides: l
                        .overrides
                        .iter()
                        .map(|o| OverrideV63 {
                            modifier: o.modifier.0,
                            on: o.on,
                            settings: o.settings.as_ref().map(KindV43::of),
                        })
                        .collect(),
                    hand: l
                        .hand
                        .filter(|h| scene.contains(h.shape))
                        .map(|h| (h.shape.get(), h.made_from)),
                })
                .collect();
            let standard = levels
                .iter()
                .all(|l| l.reduce && l.budget == 0 && l.overrides.is_empty() && l.hand.is_none());
            (!standard).then(|| LevelsV63 {
                id: id.get(),
                levels: levels.into(),
            })
        })
        .collect()
}

/// `document` (with its modifiers in) with each object's levels' details
/// read back. A shape made by hand must be an object in the file; changes
/// naming no modifier of the object's are left out.
pub(crate) fn with_levels(
    mut document: Document,
    stored: Bounded<LevelsV63>,
) -> Result<Document, Problem> {
    let mut seen = std::collections::BTreeSet::new();
    for entry in stored.0 {
        if !seen.insert(entry.id) {
            return Err(tr("an object's levels of detail are listed twice").into());
        }
        let id = ObjectId::from_raw(entry.id);
        let Some(object) = document.scene().object(id) else {
            return Err(tr("its levels of detail name an object that isn't there").into());
        };
        let mut lods = object.lods.clone();
        for (level, details) in lods.levels.iter_mut().zip(entry.levels.0) {
            level.reduce = details.reduce;
            level.budget = details.budget;
            level.overrides = details
                .overrides
                .0
                .into_iter()
                .map(|o| ModifierOverride {
                    modifier: ModifierId(o.modifier),
                    on: o.on,
                    settings: o.settings.map(|k| k.kind()),
                })
                .collect();
            level.hand = match details.hand {
                None => None,
                Some((shape, made_from)) => {
                    let shape = ObjectId::from_raw(shape);
                    if shape == id || !document.scene().contains(shape) {
                        return Err(
                            tr("a level made by hand names an object that isn't there").into()
                        );
                    }
                    Some(HandLevel { shape, made_from })
                }
            };
        }
        document = document.with_lods(id, lods).map_err(Problem::from)?;
    }
    Ok(document)
}
