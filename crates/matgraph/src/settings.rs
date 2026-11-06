//! A box's settings: what each kind of box offers (its schema,
//! [`SettingSpec`]) and the values a box in a graph has ([`Settings`]).
//!
//! Settings are kept by key, so a box kind can gain settings later and
//! older graphs still read: a setting a box doesn't have is its standard.

use crate::refusal::{Fix, Refusal};
use crate::registry::Label;
use std::borrow::Cow;
use std::collections::BTreeMap;
use words::trf;

/// What one setting is, and the values it allows.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingSpec {
    /// Its key, saved in files: never changed once released.
    pub key: Cow<'static, str>,
    /// Its plain name.
    pub name: Label,
    /// What it does, in a sentence.
    pub about: Label,
    /// The values it allows.
    pub kind: SettingKind,
}

/// The values a setting allows.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingKind {
    /// A number from `min` to `max`.
    Number {
        /// The smallest allowed.
        min: f32,
        /// The largest allowed.
        max: f32,
        /// Its standard value.
        standard: f32,
    },
    /// A whole number from `min` to `max`.
    Whole {
        /// The smallest allowed.
        min: i32,
        /// The largest allowed.
        max: i32,
        /// Its standard value.
        standard: i32,
    },
    /// A colour (sRGB, 0 to 1).
    Colour {
        /// Its standard colour.
        standard: [f32; 3],
    },
    /// One of a list of choices, by index.
    Choice {
        /// The choices' plain names.
        options: Vec<Label>,
        /// The standard choice.
        standard: u32,
    },
    /// On or off.
    Switch {
        /// Its standard state.
        standard: bool,
    },
    /// A random seed: a whole number mixed with the graph's own seed, so
    /// changing either gives another pattern of the same look.
    Seed,
}

/// A setting's value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingValue {
    /// A number.
    Number(f32),
    /// A whole number (and seeds).
    Whole(i32),
    /// A colour.
    Colour([f32; 3]),
    /// A choice, by index.
    Choice(u32),
    /// On or off.
    Switch(bool),
}

impl SettingValue {
    /// Feeds the value into a hasher, exactly (numbers by their bits).
    pub(crate) fn hash_into(&self, h: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        match *self {
            Self::Number(n) => (0u8, n.to_bits()).hash(h),
            Self::Whole(n) => (1u8, n).hash(h),
            Self::Colour(c) => (2u8, c.map(f32::to_bits)).hash(h),
            Self::Choice(n) => (3u8, n).hash(h),
            Self::Switch(b) => (4u8, b).hash(h),
        }
    }
}

impl SettingSpec {
    /// Its standard value.
    pub fn standard(&self) -> SettingValue {
        match &self.kind {
            SettingKind::Number { standard, .. } => SettingValue::Number(*standard),
            SettingKind::Whole { standard, .. } => SettingValue::Whole(*standard),
            SettingKind::Colour { standard } => SettingValue::Colour(*standard),
            SettingKind::Choice { standard, .. } => SettingValue::Choice(*standard),
            SettingKind::Switch { standard } => SettingValue::Switch(*standard),
            SettingKind::Seed => SettingValue::Whole(0),
        }
    }

    /// `value` if this setting allows it, or why not (offering the
    /// nearest allowed value as a fix when there is one).
    pub fn check(&self, value: SettingValue) -> Result<SettingValue, Refusal> {
        let name = self.name.show();
        let out_of_range = |low: String, high: String| {
            Refusal::new(trf!("{} goes from {} to {}.", name, low, high))
                .with_fixes([Fix::UseNearest])
        };
        match (&self.kind, value) {
            (SettingKind::Number { min, max, .. }, SettingValue::Number(n)) => {
                if !n.is_finite() {
                    Err(Refusal::new(trf!("{} needs a number.", name)))
                } else if n < *min || n > *max {
                    Err(out_of_range(min.to_string(), max.to_string()))
                } else {
                    Ok(value)
                }
            }
            (SettingKind::Whole { min, max, .. }, SettingValue::Whole(n)) => {
                if n < *min || n > *max {
                    Err(out_of_range(min.to_string(), max.to_string()))
                } else {
                    Ok(value)
                }
            }
            (SettingKind::Seed, SettingValue::Whole(_)) => Ok(value),
            (SettingKind::Colour { .. }, SettingValue::Colour(c)) => {
                if c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) {
                    Ok(value)
                } else {
                    Err(Refusal::new(trf!("{} needs a colour.", name))
                        .with_fixes([Fix::UseNearest]))
                }
            }
            (SettingKind::Choice { options, .. }, SettingValue::Choice(i)) => {
                if (i as usize) < options.len() {
                    Ok(value)
                } else {
                    Err(Refusal::new(trf!("{} has no such choice.", name)))
                }
            }
            (SettingKind::Switch { .. }, SettingValue::Switch(_)) => Ok(value),
            _ => Err(Refusal::new(trf!(
                "{} doesn't take that kind of value.",
                name
            ))),
        }
    }

    /// The allowed value nearest `value`: kept in range, a wrong kind of
    /// value (or not a number) replaced by the standard. Used for settings
    /// read from files, and for [`Fix::UseNearest`].
    pub fn nearest(&self, value: SettingValue) -> SettingValue {
        match (&self.kind, value) {
            (SettingKind::Number { min, max, .. }, SettingValue::Number(n)) if n.is_finite() => {
                SettingValue::Number(n.clamp(*min, *max))
            }
            (SettingKind::Whole { min, max, .. }, SettingValue::Whole(n)) => {
                SettingValue::Whole(n.clamp(*min, *max))
            }
            (SettingKind::Seed, SettingValue::Whole(n)) => SettingValue::Whole(n),
            (SettingKind::Colour { .. }, SettingValue::Colour(c))
                if c.iter().all(|v| v.is_finite()) =>
            {
                SettingValue::Colour(c.map(|v| v.clamp(0.0, 1.0)))
            }
            (SettingKind::Choice { options, .. }, SettingValue::Choice(i))
                if (i as usize) < options.len() =>
            {
                value
            }
            (SettingKind::Switch { .. }, SettingValue::Switch(_)) => value,
            _ => self.standard(),
        }
    }
}

/// The settings a box in a graph has, by key. Only those changed from
/// their standard need be kept.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings(BTreeMap<String, SettingValue>);

impl Settings {
    /// No settings changed.
    pub fn new() -> Self {
        Self::default()
    }

    /// The value kept for `key`, if any.
    pub fn get(&self, key: &str) -> Option<SettingValue> {
        self.0.get(key).copied()
    }

    /// Keeps `value` for `key`.
    pub fn set(&mut self, key: impl Into<String>, value: SettingValue) {
        self.0.insert(key.into(), value);
    }

    /// Every kept value, by key.
    pub fn iter(&self) -> impl Iterator<Item = (&str, SettingValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// How many values are kept.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no value is kept.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Every setting in `specs`, with its kept value made allowed
    /// ([`SettingSpec::nearest`]) or its standard: what a box runs with.
    /// Kept keys `specs` doesn't list are left out.
    pub fn resolved(&self, specs: &[SettingSpec]) -> Vec<SettingValue> {
        specs
            .iter()
            .map(|s| {
                self.get(&s.key)
                    .map_or_else(|| s.standard(), |v| s.nearest(v))
            })
            .collect()
    }

    /// The same settings with only keys `specs` lists, each made allowed.
    pub fn normalised(&self, specs: &[SettingSpec]) -> Self {
        Self(
            specs
                .iter()
                .filter_map(|s| self.get(&s.key).map(|v| (s.key.to_string(), s.nearest(v))))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use words::text;

    fn number() -> SettingSpec {
        SettingSpec {
            key: "amount".into(),
            name: Label::Key(text!("Amount")),
            about: Label::Key(text!("How much.")),
            kind: SettingKind::Number {
                min: 0.0,
                max: 1.0,
                standard: 0.5,
            },
        }
    }

    #[test]
    fn values_in_range_pass_and_others_say_why() {
        let spec = number();
        assert!(spec.check(SettingValue::Number(0.25)).is_ok());
        let refusal = spec.check(SettingValue::Number(3.0)).unwrap_err();
        assert!(refusal.reason.contains("Amount"), "{}", refusal.reason);
        assert_eq!(refusal.fixes, vec![Fix::UseNearest]);
        assert!(spec.check(SettingValue::Number(f32::NAN)).is_err());
        assert!(spec.check(SettingValue::Switch(true)).is_err());
    }

    #[test]
    fn nearest_clamps_and_replaces_nonsense_with_the_standard() {
        let spec = number();
        assert_eq!(
            spec.nearest(SettingValue::Number(9.0)),
            SettingValue::Number(1.0)
        );
        assert_eq!(
            spec.nearest(SettingValue::Number(f32::INFINITY)),
            SettingValue::Number(0.5)
        );
        assert_eq!(
            spec.nearest(SettingValue::Choice(2)),
            SettingValue::Number(0.5)
        );
    }

    #[test]
    fn missing_settings_are_their_standard_and_unknown_keys_are_dropped() {
        let spec = number();
        let mut settings = Settings::new();
        assert_eq!(
            settings.resolved(std::slice::from_ref(&spec)),
            vec![SettingValue::Number(0.5)]
        );
        settings.set("amount", SettingValue::Number(2.0));
        settings.set("gone", SettingValue::Whole(1));
        assert_eq!(
            settings.resolved(std::slice::from_ref(&spec)),
            vec![SettingValue::Number(1.0)]
        );
        let tidy = settings.normalised(std::slice::from_ref(&spec));
        assert_eq!(tidy.len(), 1);
        assert_eq!(tidy.get("amount"), Some(SettingValue::Number(1.0)));
    }
}
