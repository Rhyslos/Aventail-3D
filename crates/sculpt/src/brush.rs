//! The sculpting brushes' settings and what one dab of a brush does
//! (decision 0304): Draw pushes the surface out (or in), Smooth evens it.
//! Everything here is plain data and plain functions; the live sculpt
//! ([`crate::LiveSculpt`]) finds the points in reach and applies them.

use glam::Vec3;
use words::text;

/// What a brush does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BrushKind {
    /// Pushes the surface out along its average facing (in, inverted).
    #[default]
    Draw,
    /// Evens the surface out: bumps go, the shape's flow stays.
    Smooth,
}

impl BrushKind {
    /// Every brush, in shelf order.
    pub const ALL: [Self; 2] = [Self::Draw, Self::Smooth];

    /// Its name (English: the key of its translation).
    pub fn name(self) -> &'static str {
        match self {
            Self::Draw => text!("Draw"),
            Self::Smooth => text!("Smooth"),
        }
    }

    /// What it does, in plain words (English: the key of its
    /// translation), for the shelf's "What does this do?".
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Draw => text!(
                "Pushes the surface out where you stroke, along the way it faces on average under the brush. Hold Ctrl to push it in instead"
            ),
            Self::Smooth => text!(
                "Evens the surface out: bumps and dents go, the big shape stays. Hold Shift with any brush to smooth"
            ),
        }
    }
}

/// How a brush's strength fades from its middle to its rim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Falloff {
    /// A soft, even fade: the most used.
    #[default]
    Smooth,
    /// Strong in the middle, falling quickly.
    Sharp,
    /// Full strength almost to the rim.
    Flat,
    /// A narrow point in the middle.
    Spike,
}

impl Falloff {
    /// Every falloff, in menu order.
    pub const ALL: [Self; 4] = [Self::Smooth, Self::Sharp, Self::Flat, Self::Spike];

    /// Its name (English: the key of its translation).
    pub fn name(self) -> &'static str {
        match self {
            Self::Smooth => text!("Smooth"),
            Self::Sharp => text!("Sharp"),
            Self::Flat => text!("Flat"),
            Self::Spike => text!("Spike"),
        }
    }

    /// The strength at `x` of the way from the middle (0) to the rim (1):
    /// 1 in the middle, 0 at and past the rim.
    pub fn at(self, x: f32) -> f32 {
        if !(x < 1.0) {
            return 0.0;
        }
        let x = x.max(0.0);
        let smooth = |t: f32| 1.0 - t * t * (3.0 - 2.0 * t);
        match self {
            Self::Smooth => smooth(x),
            Self::Sharp => (1.0 - x) * (1.0 - x),
            Self::Flat => {
                if x < 0.8 {
                    1.0
                } else {
                    smooth((x - 0.8) / 0.2)
                }
            }
            Self::Spike => (1.0 - x).powi(4),
        }
    }
}

/// A brush's settings, kept per brush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushSettings {
    /// Its radius on screen, in points.
    pub radius: f32,
    /// How strongly it works, 0 to 1.
    pub strength: f32,
    /// How its strength fades to its rim.
    pub falloff: Falloff,
    /// The gap between its dabs along a stroke, as a share of its radius.
    pub spacing: f32,
}

/// The radius's range on screen, in points.
pub const RADIUS_RANGE: (f32, f32) = (2.0, 1000.0);
/// The spacing's range, as a share of the radius.
pub const SPACING_RANGE: (f32, f32) = (0.02, 1.0);

impl BrushSettings {
    /// A brush's standard settings.
    pub fn standard(kind: BrushKind) -> Self {
        Self {
            radius: 50.0,
            strength: match kind {
                BrushKind::Draw => 0.5,
                BrushKind::Smooth => 0.5,
            },
            falloff: Falloff::Smooth,
            spacing: 0.1,
        }
    }

    /// The settings brought within their ranges (a number that isn't one
    /// becomes the standard).
    pub fn clamped(self) -> Self {
        let fix = |v: f32, (low, high): (f32, f32), standard: f32| {
            if v.is_finite() {
                v.clamp(low, high)
            } else {
                standard
            }
        };
        Self {
            radius: fix(self.radius, RADIUS_RANGE, 50.0),
            strength: fix(self.strength, (0.0, 1.0), 0.5),
            falloff: self.falloff,
            spacing: fix(self.spacing, SPACING_RANGE, 0.1),
        }
    }
}

/// One dab of a brush on the shape, in the object's own space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dab {
    /// What it does.
    pub kind: BrushKind,
    /// Its middle, on the surface.
    pub centre: Vec3,
    /// Its reach.
    pub radius: f32,
    /// How strongly, 0 to 1.
    pub strength: f32,
    /// How its strength fades.
    pub falloff: Falloff,
    /// Dig in instead of building up (Ctrl).
    pub invert: bool,
}

/// How far one Draw dab at full strength pushes the middle of the brush,
/// as a share of its radius. Dabs overlap (a tenth of the radius apart),
/// so a stroke builds up several times this.
pub const DRAW_RATE: f32 = 0.04;
/// How far one Smooth dab at full strength moves a point towards the
/// middle of its neighbours.
pub const SMOOTH_RATE: f32 = 0.5;

/// Where a Draw dab moves a point at `p`, `distance` from its middle,
/// pushing along `facing` (of unit length).
pub fn draw(dab: &Dab, facing: Vec3, p: Vec3, distance: f32) -> Vec3 {
    let sign = if dab.invert { -1.0 } else { 1.0 };
    let amount = dab.falloff.at(distance / dab.radius.max(1e-12))
        * dab.strength.clamp(0.0, 1.0)
        * DRAW_RATE
        * dab.radius;
    p + facing * (amount * sign)
}

/// Where a Smooth dab moves a point at `p`, `distance` from its middle,
/// whose neighbours' middle is `middle`.
pub fn smooth(dab: &Dab, middle: Vec3, p: Vec3, distance: f32) -> Vec3 {
    let share = (dab.falloff.at(distance / dab.radius.max(1e-12))
        * dab.strength.clamp(0.0, 1.0)
        * SMOOTH_RATE)
        .clamp(0.0, 1.0);
    p + (middle - p) * share
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_falloff_is_full_in_the_middle_and_nothing_at_the_rim() {
        for f in Falloff::ALL {
            assert!((f.at(0.0) - 1.0).abs() < 1e-6, "{f:?}");
            assert_eq!(f.at(1.0), 0.0);
            assert_eq!(f.at(1.5), 0.0);
            assert_eq!(f.at(f32::NAN), 0.0);
            // Never grows towards the rim.
            let mut last = 1.0;
            for i in 0..=100 {
                let v = f.at(i as f32 / 100.0);
                assert!(v <= last + 1e-6 && v >= 0.0, "{f:?} {i}");
                last = v;
            }
        }
    }

    #[test]
    fn draw_pushes_out_and_ctrl_pushes_in() {
        let dab = Dab {
            kind: BrushKind::Draw,
            centre: Vec3::ZERO,
            radius: 10.0,
            strength: 1.0,
            falloff: Falloff::Smooth,
            invert: false,
        };
        let out = draw(&dab, Vec3::Z, Vec3::ZERO, 0.0);
        assert!((out.z - DRAW_RATE * 10.0).abs() < 1e-6);
        let inv = draw(
            &Dab {
                invert: true,
                ..dab
            },
            Vec3::Z,
            Vec3::ZERO,
            0.0,
        );
        assert!((inv.z + DRAW_RATE * 10.0).abs() < 1e-6);
        assert_eq!(draw(&dab, Vec3::Z, Vec3::ONE, 10.0), Vec3::ONE);
        let none = draw(
            &Dab {
                strength: 0.0,
                ..dab
            },
            Vec3::Z,
            Vec3::ONE,
            0.0,
        );
        assert_eq!(none, Vec3::ONE);
    }

    #[test]
    fn settings_are_kept_in_range() {
        let wild = BrushSettings {
            radius: 1e9,
            strength: f32::NAN,
            falloff: Falloff::Flat,
            spacing: -1.0,
        }
        .clamped();
        assert_eq!(wild.radius, RADIUS_RANGE.1);
        assert_eq!(wild.strength, 0.5);
        assert_eq!(wild.spacing, SPACING_RANGE.0);
    }
}
