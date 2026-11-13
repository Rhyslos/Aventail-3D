//! Variations and the dice (Phase 12, milestone 18,
//! [decision 0280](../../../../docs/decisions/0280-variations-and-the-dice.md)).
//!
//! A **variation** is a named look of one graph (Clean, Mossy, Ruined…):
//! the values of the settings it shows in the painter, every seed in it
//! (each box's seed and the graph's own) and the moment of its animation
//! it bakes. Using one sets those back, so one graph gives several looks;
//! a graph fill in the painter can pick one too. Bypassed boxes aren't
//! part of a variation: bypass is for comparing while you work, and a
//! box left bypassed in a saved look would be easy to forget.
//!
//! **The dice** give every seed in the graph a new number at once, for a
//! fresh take on the same look. The new numbers come from a roll number
//! the caller gives, so the same roll on the same graph always gives the
//! same seeds.
//!
//! Variations are kept in the graph, so they are saved with it, copied
//! with it and undone with it. Their ids come from the graph's one
//! counter, as boxes' and frames' do.

use super::{BoxId, Graph, GraphEdit, MAX_NAME, MAX_PLACE};
use crate::refusal::Refusal;
use crate::registry::Registry;
use crate::settings::{SettingKind, SettingValue};
use words::{text, tr, trf};

/// The most variations a graph may hold.
pub const MAX_VARIATIONS: usize = 100;

/// The most values one variation may hold.
pub const MAX_VARIATION_VALUES: usize = 8192;

/// The largest seed the dice give (the Seed rows go from 0 to this).
pub const MAX_ROLLED_SEED: u32 = 999_999;

/// The longest setting key kept in a variation, in bytes.
const MAX_KEY: usize = 64;

/// A variation's id in its graph: never reused (boxes, frames and
/// variations share one counter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariationId(pub u64);

/// One setting's value in a variation.
#[derive(Debug, Clone, PartialEq)]
pub struct VariationValue {
    /// The box.
    pub node: BoxId,
    /// The setting's key.
    pub key: String,
    /// The value.
    pub value: SettingValue,
}

/// A named look of a graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Variation {
    /// Its name ("Mossy").
    pub name: String,
    /// The values of the settings shown in the painter and of every seed
    /// setting, when it was saved. A value whose box or setting is no
    /// longer in the graph is kept and skipped when it is used.
    pub values: Vec<VariationValue>,
    /// The graph's own seed.
    pub seed: u32,
    /// The moment of the animation baked, in seconds.
    pub moment: f32,
}

impl Variation {
    /// Its value of setting `key` of box `node`, if it holds one.
    pub fn value(&self, node: BoxId, key: &str) -> Option<SettingValue> {
        self.values
            .iter()
            .find(|v| v.node == node && v.key == key)
            .map(|v| v.value)
    }

    /// Why it can't be kept, if it can't.
    pub(super) fn problem(&self) -> Option<&'static str> {
        let real = |v: &SettingValue| match v {
            SettingValue::Number(n) => n.is_finite() && n.abs() <= MAX_PLACE,
            SettingValue::Colour(c) => c.iter().all(|x| x.is_finite()),
            SettingValue::Gradient(g) => g.is_allowed(),
            _ => true,
        };
        if self.name.trim().is_empty() || self.name.len() > MAX_NAME {
            Some(text!("a variation's name is missing or too long"))
        } else if self.values.len() > MAX_VARIATION_VALUES
            || self
                .values
                .iter()
                .any(|v| v.key.is_empty() || v.key.len() > MAX_KEY || !real(&v.value))
        {
            Some(text!("a variation's values are damaged"))
        } else if !self.moment.is_finite() || !(0.0..=super::MAX_MOMENT).contains(&self.moment) {
            Some(text!("a variation's moment is out of range"))
        } else {
            None
        }
    }
}

/// The keys of the seed settings of a box of kind `kind`: its Seed
/// settings, or for your own box, its sliders that set a seed inside
/// (through own boxes inside own boxes too).
pub fn seed_keys(registry: &Registry, kind: &str) -> Vec<String> {
    seed_keys_within(registry, kind, 0)
}

/// [`seed_keys`], at most a few own boxes deep.
fn seed_keys_within(registry: &Registry, kind: &str, depth: u32) -> Vec<String> {
    let Some(t) = registry.get(kind) else {
        return Vec::new();
    };
    match t.own() {
        Some(own) if depth < 8 => own
            .sliders
            .iter()
            .filter(|s| {
                own.inner.get(s.node).is_some_and(|b| {
                    seed_keys_within(registry, &b.kind, depth + 1).contains(&s.setting)
                })
            })
            .map(|s| s.key.clone())
            .collect(),
        Some(_) => Vec::new(),
        None => t
            .info()
            .settings
            .iter()
            .filter(|s| matches!(s.kind, SettingKind::Seed))
            .map(|s| s.key.to_string())
            .collect(),
    }
}

/// A key's bytes hashed (FNV-1a), to mix into a roll.
fn key_hash(key: &str) -> u64 {
    key.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A seed from 0 to [`MAX_ROLLED_SEED`] from a hash.
fn rolled_seed(hash: u64) -> u32 {
    (hash % (u64::from(MAX_ROLLED_SEED) + 1)) as u32
}

impl Graph {
    /// Every variation, in the order they were made.
    pub fn variations(&self) -> impl Iterator<Item = (VariationId, &Variation)> {
        self.variations.iter().map(|(id, v)| (*id, v))
    }

    /// Variation `id`.
    pub fn variation(&self, id: VariationId) -> Option<&Variation> {
        self.variations.get(&id)
    }

    /// The graph as it is now, as a variation called `name`: the values
    /// of the settings shown in the painter and of every seed setting (a
    /// setting left at its standard is held too), its seed and its
    /// moment.
    pub fn variation_now(&self, registry: &Registry, name: impl Into<String>) -> Variation {
        let mut values: Vec<VariationValue> = self
            .shown(registry)
            .into_iter()
            .map(|s| VariationValue {
                node: s.node,
                key: s.spec.key.to_string(),
                value: s.value,
            })
            .collect();
        for (id, b) in self.boxes() {
            let Some(info) = registry.get(&b.kind).map(|t| t.info()) else {
                continue;
            };
            for key in seed_keys(registry, &b.kind) {
                if values.iter().any(|v| v.node == id && v.key == key) {
                    continue;
                }
                let Some(spec) = info.setting(&key) else {
                    continue;
                };
                let value = b.settings.get(&key).unwrap_or_else(|| spec.standard());
                values.push(VariationValue {
                    node: id,
                    key,
                    value: spec.nearest(value),
                });
            }
        }
        values.truncate(MAX_VARIATION_VALUES);
        Variation {
            name: name.into(),
            values,
            seed: self.seed,
            moment: self.moment,
        }
    }

    /// The edits that make the graph look as variation `id` says: each
    /// of its values that differs (a box or setting no longer there is
    /// skipped, a value out of range brought into it), then the seed and
    /// the moment. None when it already does, or there's no such
    /// variation.
    pub fn variation_edits(&self, registry: &Registry, id: VariationId) -> Vec<GraphEdit> {
        let Some(variation) = self.variations.get(&id) else {
            return Vec::new();
        };
        let mut edits = Vec::new();
        for v in &variation.values {
            let Some(b) = self.get(v.node) else { continue };
            let Some(spec) = registry.get(&b.kind).and_then(|t| t.info().setting(&v.key)) else {
                continue;
            };
            let now = spec.nearest(b.settings.get(&v.key).unwrap_or_else(|| spec.standard()));
            let value = spec.nearest(v.value);
            if value != now {
                edits.push(GraphEdit::SetSetting {
                    node: v.node,
                    key: v.key.clone(),
                    value,
                });
            }
        }
        if variation.seed != self.seed {
            edits.push(GraphEdit::SetSeed(variation.seed));
        }
        if variation.moment != self.moment {
            edits.push(GraphEdit::SetMoment(super::usable_moment(variation.moment)));
        }
        edits
    }

    /// This graph as variation `id` makes it look (unchanged when there's
    /// no such variation): what a graph fill in the painter that picked
    /// the variation runs.
    pub fn with_variation(&self, registry: &Registry, id: VariationId) -> Graph {
        let mut out = self.clone();
        for edit in self.variation_edits(registry, id) {
            let _ = out.apply(registry, &edit);
        }
        out
    }

    /// The first variation the graph looks as now (using it would change
    /// nothing), if any.
    pub fn variation_in_use(&self, registry: &Registry) -> Option<VariationId> {
        self.variations
            .keys()
            .copied()
            .find(|id| self.variation_edits(registry, *id).is_empty())
    }

    /// This graph without its variations: what the painter makes a layer
    /// from (its variations don't change the look, so they stay out of
    /// what is hashed).
    pub fn without_variations(mut self) -> Graph {
        self.variations.clear();
        self
    }

    /// A name for a new variation: "Variation N", the first not taken.
    pub fn new_variation_name(&self) -> String {
        (1..)
            .map(|n| trf!("Variation {}", n))
            .find(|name| !self.variations.values().any(|v| v.name == *name))
            .unwrap_or_default()
    }

    /// The dice: the edits giving every seed in the graph a new number
    /// from roll `roll` (each box's seed settings, and the graph's own
    /// seed), or only box `only`'s seeds. The same roll on the same graph
    /// gives the same seeds. None when there are no seeds to roll.
    pub fn rolled(&self, registry: &Registry, roll: u64, only: Option<BoxId>) -> Vec<GraphEdit> {
        use crate::seed::mix;
        let mut edits = Vec::new();
        for (id, b) in self.boxes() {
            if only.is_some_and(|o| o != id) {
                continue;
            }
            for key in seed_keys(registry, &b.kind) {
                let seed = rolled_seed(mix(mix(roll, id.0), key_hash(&key)));
                edits.push(GraphEdit::SetSetting {
                    node: id,
                    key,
                    value: SettingValue::Whole(seed as i32),
                });
            }
        }
        if only.is_none() {
            edits.push(GraphEdit::SetSeed(rolled_seed(mix(roll, u64::MAX))));
        }
        edits
    }

    /// A roll number for the dice from the graph as it is and `count`
    /// (how many times the dice were rolled before): rolling again after
    /// undo gives another take when `count` has moved on.
    pub fn roll_from(&self, count: u64) -> u64 {
        use crate::seed::mix;
        let seeds = self
            .boxes()
            .flat_map(|(id, b)| b.settings.iter().map(move |(k, v)| (id, k, v)))
            .fold(u64::from(self.seed), |h, (id, k, v)| match v {
                SettingValue::Whole(n) => mix(h, mix(mix(id.0, key_hash(k)), n as u32 as u64)),
                _ => h,
            });
        mix(seeds, count)
    }

    /// [`GraphEdit::AddVariation`].
    pub(super) fn add_variation(&mut self, variation: &Variation) -> Result<(), Refusal> {
        if self.variations.len() >= MAX_VARIATIONS {
            return Err(Refusal::new(trf!(
                "A graph can hold {} variations; delete one first.",
                MAX_VARIATIONS
            )));
        }
        variation_ok(variation)?;
        let id = VariationId(self.take_id());
        self.variations.insert(id, variation.clone());
        Ok(())
    }

    /// [`GraphEdit::ChangeVariation`].
    pub(super) fn change_variation(
        &mut self,
        id: VariationId,
        variation: &Variation,
    ) -> Result<(), Refusal> {
        variation_ok(variation)?;
        let slot = self.variations.get_mut(&id).ok_or_else(gone)?;
        *slot = variation.clone();
        Ok(())
    }

    /// [`GraphEdit::RemoveVariation`].
    pub(super) fn remove_variation(&mut self, id: VariationId) -> Result<(), Refusal> {
        self.variations.remove(&id).map(|_| ()).ok_or_else(gone)
    }
}

/// The refusal for a variation that isn't there.
fn gone() -> Refusal {
    Refusal::new(tr("That variation isn't in the graph any more."))
}

/// Why a variation can't be kept, as a refusal.
fn variation_ok(variation: &Variation) -> Result<(), Refusal> {
    match variation.problem() {
        Some(why) => Err(Refusal::new(trf!(
            "That variation can't be kept: {}.",
            tr(why)
        ))),
        None => Ok(()),
    }
}
