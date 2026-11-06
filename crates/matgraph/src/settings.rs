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
    /// A real-world length in centimetres, from `min` to `max`, held as a
    /// [`SettingValue::Number`]. A box turns it into pixels with the
    /// graph's real-world size (Material out's), so a bevel 2 cm wide
    /// stays 2 cm wide whatever the texture's size in pixels.
    Length {
        /// The smallest allowed, in centimetres.
        min: f32,
        /// The largest allowed, in centimetres.
        max: f32,
        /// Its standard value, in centimetres.
        standard: f32,
    },
    /// A gradient: colours at places from 0 to 1 (Gradient map's).
    Gradient {
        /// Its standard gradient.
        standard: ColourGradient,
    },
}

/// The most colours a gradient holds.
pub const MAX_STOPS: usize = 8;

/// One colour of a gradient, and where it sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    /// Where it sits: 0 (black, the low end) to 1 (white, the high end).
    pub at: f32,
    /// Its colour (sRGB, 0 to 1).
    pub colour: [f32; 3],
}

/// A gradient: 2 to [`MAX_STOPS`] colours at places from 0 to 1, blended
/// between. The colours are kept in the order they were added (so a list
/// of them doesn't jump about while one is dragged past another), and
/// sorted by place when the gradient is read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourGradient {
    stops: [GradientStop; MAX_STOPS],
    count: u8,
}

impl ColourGradient {
    /// A gradient of `stops`; `None` unless there are 2 to [`MAX_STOPS`]
    /// of them. Places and colours are kept as given: [`Self::is_allowed`]
    /// says whether they are real and in range.
    pub fn new(stops: &[GradientStop]) -> Option<Self> {
        if !(2..=MAX_STOPS).contains(&stops.len()) {
            return None;
        }
        let mut all = [stops[0]; MAX_STOPS];
        all[..stops.len()].copy_from_slice(stops);
        Some(Self {
            stops: all,
            count: stops.len() as u8,
        })
    }

    /// From one colour at 0 to another at 1.
    pub fn two(from: [f32; 3], to: [f32; 3]) -> Self {
        let last = GradientStop {
            at: 1.0,
            colour: to,
        };
        let mut stops = [last; MAX_STOPS];
        stops[0] = GradientStop {
            at: 0.0,
            colour: from,
        };
        Self { stops, count: 2 }
    }

    /// Its colours, in the order they were added.
    pub fn stops(&self) -> &[GradientStop] {
        &self.stops[..usize::from(self.count)]
    }

    /// Whether every place and colour is a real number from 0 to 1.
    pub fn is_allowed(&self) -> bool {
        self.stops()
            .iter()
            .all(|s| s.at.is_finite() && (0.0..=1.0).contains(&s.at) && in_unit(s.colour))
    }

    /// The same gradient with every place and colour kept from 0 to 1;
    /// `None` if any isn't a real number.
    pub fn clamped(&self) -> Option<Self> {
        let mut out = *self;
        for s in &mut out.stops[..usize::from(self.count)] {
            if !s.at.is_finite() || !s.colour.iter().all(|c| c.is_finite()) {
                return None;
            }
            s.at = s.at.clamp(0.0, 1.0);
            s.colour = s.colour.map(|c| c.clamp(0.0, 1.0));
        }
        Some(out)
    }

    /// The colour at `t` (0 to 1): blended between the colours either
    /// side, the first colour before the first place and the last after
    /// the last.
    pub fn colour_at(&self, t: f32) -> [f32; 3] {
        let mut sorted = *self;
        let stops = &mut sorted.stops[..usize::from(self.count)];
        stops.sort_by(|a, b| a.at.total_cmp(&b.at));
        let t = if t.is_finite() { t } else { 0.0 };
        let first = stops[0];
        if t <= first.at {
            return first.colour;
        }
        for pair in stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t <= b.at {
                let span = b.at - a.at;
                let k = if span > 1e-6 { (t - a.at) / span } else { 1.0 };
                return std::array::from_fn(|i| a.colour[i] + (b.colour[i] - a.colour[i]) * k);
            }
        }
        stops[stops.len() - 1].colour
    }

    /// The same gradient with one more colour, placed in the middle of
    /// the widest gap, coloured as the gradient is there; `None` when it
    /// is full.
    pub fn with_stop_added(&self) -> Option<Self> {
        if usize::from(self.count) >= MAX_STOPS {
            return None;
        }
        let mut places: Vec<f32> = self.stops().iter().map(|s| s.at).collect();
        places.push(0.0);
        places.push(1.0);
        places.sort_by(f32::total_cmp);
        let (mut at, mut widest) = (0.5, -1.0);
        for pair in places.windows(2) {
            if pair[1] - pair[0] > widest {
                widest = pair[1] - pair[0];
                at = (pair[0] + pair[1]) * 0.5;
            }
        }
        let mut out = *self;
        out.stops[usize::from(self.count)] = GradientStop {
            at,
            colour: self.colour_at(at),
        };
        out.count += 1;
        Some(out)
    }

    /// The same gradient without colour `index`; `None` if it would have
    /// fewer than two, or there is no such colour.
    pub fn with_stop_removed(&self, index: usize) -> Option<Self> {
        let count = usize::from(self.count);
        if count <= 2 || index >= count {
            return None;
        }
        let mut out = *self;
        out.stops.copy_within(index + 1..count, index);
        out.count -= 1;
        Some(out)
    }

    /// The same gradient with colour `index` changed to `stop`.
    pub fn with_stop(&self, index: usize, stop: GradientStop) -> Self {
        let mut out = *self;
        if index < usize::from(self.count) {
            out.stops[index] = stop;
        }
        out
    }

    fn hash_into(&self, h: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        self.count.hash(h);
        for s in self.stops() {
            (s.at.to_bits(), s.colour.map(f32::to_bits)).hash(h);
        }
    }
}

/// Whether every channel is a real number from 0 to 1.
fn in_unit(c: [f32; 3]) -> bool {
    c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
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
    /// A gradient.
    Gradient(ColourGradient),
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
            Self::Gradient(g) => {
                5u8.hash(h);
                g.hash_into(h);
            }
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
            SettingKind::Length { standard, .. } => SettingValue::Number(*standard),
            SettingKind::Gradient { standard } => SettingValue::Gradient(*standard),
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
            (
                SettingKind::Number { min, max, .. } | SettingKind::Length { min, max, .. },
                SettingValue::Number(n),
            ) => {
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
            (SettingKind::Gradient { .. }, SettingValue::Gradient(g)) => {
                if g.is_allowed() {
                    Ok(value)
                } else {
                    Err(
                        Refusal::new(trf!("{} needs colours at places from 0 to 1.", name))
                            .with_fixes([Fix::UseNearest]),
                    )
                }
            }
            (SettingKind::Colour { .. }, SettingValue::Colour(c)) => {
                if in_unit(c) {
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
            (
                SettingKind::Number { min, max, .. } | SettingKind::Length { min, max, .. },
                SettingValue::Number(n),
            ) if n.is_finite() => SettingValue::Number(n.clamp(*min, *max)),
            (SettingKind::Gradient { .. }, SettingValue::Gradient(g)) => g
                .clamped()
                .map_or_else(|| self.standard(), SettingValue::Gradient),
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

    fn stop(at: f32, grey: f32) -> GradientStop {
        GradientStop {
            at,
            colour: [grey; 3],
        }
    }

    #[test]
    fn gradients_blend_between_their_colours_in_order_of_place() {
        // Added out of order: read in order of place.
        let g = ColourGradient::new(&[stop(1.0, 1.0), stop(0.0, 0.0), stop(0.5, 0.2)]).unwrap();
        assert_eq!(g.colour_at(0.0), [0.0; 3]);
        assert_eq!(g.colour_at(0.5), [0.2; 3]);
        assert!((g.colour_at(0.75)[0] - 0.6).abs() < 1e-5);
        assert_eq!(g.colour_at(-3.0), [0.0; 3]);
        assert_eq!(g.colour_at(f32::NAN), [0.0; 3]);
        assert_eq!(g.colour_at(9.0), [1.0; 3]);
        // Two colours in one place: a sharp step, no division by nothing.
        let step = ColourGradient::new(&[stop(0.5, 0.0), stop(0.5, 1.0)]).unwrap();
        assert_eq!(step.colour_at(0.25), [0.0; 3]);
        assert_eq!(step.colour_at(0.75), [1.0; 3]);
        // Too few or too many colours.
        assert!(ColourGradient::new(&[stop(0.0, 0.0)]).is_none());
        assert!(ColourGradient::new(&[stop(0.0, 0.0); MAX_STOPS + 1]).is_none());
    }

    #[test]
    fn gradient_colours_are_added_in_the_widest_gap_and_removed_down_to_two() {
        let g = ColourGradient::two([0.0; 3], [1.0; 3]);
        let three = g.with_stop_added().unwrap();
        assert_eq!(three.stops().len(), 3);
        assert_eq!(three.stops()[2].at, 0.5);
        assert!((three.stops()[2].colour[0] - 0.5).abs() < 1e-6);
        let mut full = three;
        while let Some(more) = full.with_stop_added() {
            full = more;
        }
        assert_eq!(full.stops().len(), MAX_STOPS);
        let fewer = three.with_stop_removed(0).unwrap();
        assert_eq!(fewer.stops()[0].at, 1.0);
        assert!(fewer.with_stop_removed(0).is_none());
        assert!(three.with_stop_removed(7).is_none());
        let moved = g.with_stop(1, stop(0.25, 0.3));
        assert_eq!(moved.stops()[1], stop(0.25, 0.3));
        assert_eq!(g.with_stop(5, stop(0.1, 0.1)), g);
    }

    #[test]
    fn gradients_and_lengths_are_checked_and_brought_into_range() {
        let spec = SettingSpec {
            key: "colours".into(),
            name: Label::Key(text!("Colours")),
            about: Label::Key(text!("The colours.")),
            kind: SettingKind::Gradient {
                standard: ColourGradient::two([0.0; 3], [1.0; 3]),
            },
        };
        let wild = ColourGradient::new(&[stop(-1.0, 2.0), stop(3.0, 0.5)]).unwrap();
        assert!(spec.check(SettingValue::Gradient(wild)).is_err());
        let SettingValue::Gradient(near) = spec.nearest(SettingValue::Gradient(wild)) else {
            panic!("a gradient stays a gradient");
        };
        assert!(near.is_allowed());
        assert_eq!(near.stops()[0], stop(0.0, 1.0));
        let broken = ColourGradient::new(&[stop(f32::NAN, 0.0), stop(1.0, 0.0)]).unwrap();
        assert_eq!(
            spec.nearest(SettingValue::Gradient(broken)),
            spec.standard()
        );
        assert_eq!(spec.nearest(SettingValue::Number(1.0)), spec.standard());
        let length = SettingSpec {
            kind: SettingKind::Length {
                min: 0.0,
                max: 10.0,
                standard: 1.0,
            },
            ..number()
        };
        assert!(length.check(SettingValue::Number(5.0)).is_ok());
        assert!(length.check(SettingValue::Number(50.0)).is_err());
        assert_eq!(
            length.nearest(SettingValue::Number(50.0)),
            SettingValue::Number(10.0)
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
