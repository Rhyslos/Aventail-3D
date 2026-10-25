//! Ready-made curves: a line, a circle, an arc and a spiral, built from
//! settings that stay editable (like the primitives), lying on the ground
//! round the pivot.

use super::{Curve, CurveError};
use glam::Vec3;
use std::f32::consts::{FRAC_PI_2, TAU};
use words::{text, tr};

/// The smallest length a ready-made curve takes: 0.01 cm.
const MIN_LENGTH: f32 = 0.01;
/// The largest: 1 km.
const MAX_LENGTH: f32 = 100_000.0;
/// The most turns a spiral may make.
pub const MAX_TURNS: f32 = 50.0;

/// Which ready-made curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadyCurveKind {
    /// A straight line.
    Line,
    /// A circle.
    Circle,
    /// Part of a circle.
    Arc,
    /// A spiral (a helix), climbing as it turns.
    Spiral,
}

impl ReadyCurveKind {
    /// Every kind, in the order the menus list them.
    pub const ALL: [Self; 4] = [Self::Line, Self::Circle, Self::Arc, Self::Spiral];

    /// Its name, in the language in use.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Line => text!("Line"),
            Self::Circle => text!("Circle"),
            Self::Arc => text!("Arc"),
            Self::Spiral => text!("Spiral"),
        })
    }

    /// What it is, for menus and tooltips.
    pub fn description(self) -> &'static str {
        tr(match self {
            Self::Line => text!("A straight curve 1 m long, lying on the ground"),
            Self::Circle => text!("A round curve 1 m across, lying on the ground"),
            Self::Arc => text!("A quarter of a circle 1 m across, lying on the ground"),
            Self::Spiral => text!("A curve climbing 1 m in three turns, like a spring"),
        })
    }

    /// This kind with its standard settings.
    pub fn standard(self) -> ReadyCurve {
        match self {
            Self::Line => ReadyCurve::Line { length: 100.0 },
            Self::Circle => ReadyCurve::Circle { radius: 50.0 },
            Self::Arc => ReadyCurve::Arc {
                radius: 50.0,
                angle: 90.0,
            },
            Self::Spiral => ReadyCurve::Spiral {
                radius: 50.0,
                height: 100.0,
                turns: 3.0,
            },
        }
    }
}

/// A ready-made curve with its settings (lengths in centimetres, angles
/// in degrees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadyCurve {
    /// A straight line along X, its middle on the pivot.
    Line {
        /// How long.
        length: f32,
    },
    /// A circle round the pivot.
    Circle {
        /// How far from the middle.
        radius: f32,
    },
    /// Part of a circle round the pivot, from +X turning towards +Y.
    Arc {
        /// How far from the middle.
        radius: f32,
        /// How far round: 1° to 360°.
        angle: f32,
    },
    /// A spiral round the pivot, from the ground up.
    Spiral {
        /// How far from the middle.
        radius: f32,
        /// How high it climbs.
        height: f32,
        /// How many times round: a quarter to [`MAX_TURNS`].
        turns: f32,
    },
}

/// `value` within `low` to `high`; not a number becomes `low`.
fn within(value: f32, low: f32, high: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        low
    }
}

impl ReadyCurve {
    /// Which kind it is.
    pub fn kind(&self) -> ReadyCurveKind {
        match self {
            Self::Line { .. } => ReadyCurveKind::Line,
            Self::Circle { .. } => ReadyCurveKind::Circle,
            Self::Arc { .. } => ReadyCurveKind::Arc,
            Self::Spiral { .. } => ReadyCurveKind::Spiral,
        }
    }

    /// Its kind's name.
    pub fn name(&self) -> &'static str {
        self.kind().name()
    }

    /// The same settings within limits.
    pub fn clamped(&self) -> Self {
        let length = |v| within(v, MIN_LENGTH, MAX_LENGTH);
        match *self {
            Self::Line { length: l } => Self::Line { length: length(l) },
            Self::Circle { radius } => Self::Circle {
                radius: length(radius),
            },
            Self::Arc { radius, angle } => Self::Arc {
                radius: length(radius),
                angle: within(angle, 1.0, 360.0),
            },
            Self::Spiral {
                radius,
                height,
                turns,
            } => Self::Spiral {
                radius: length(radius),
                height: within(height, 0.0, MAX_LENGTH),
                turns: within(turns, 0.25, MAX_TURNS),
            },
        }
    }

    /// The curve its settings make (within limits).
    pub fn curve(&self) -> Curve {
        let made = match self.clamped() {
            Self::Line { length } => Curve::corners(
                &[
                    Vec3::new(-length / 2.0, 0.0, 0.0),
                    Vec3::new(length / 2.0, 0.0, 0.0),
                ],
                false,
            ),
            Self::Circle { radius } => round(radius, TAU, 0.0, true),
            Self::Arc { radius, angle } => round(radius, angle.to_radians(), 0.0, false),
            Self::Spiral {
                radius,
                height,
                turns,
            } => round(radius, turns * TAU, height, false),
        };
        made.unwrap_or_default()
    }
}

/// A curve round the up axis at `radius`, through `sweep` radians from
/// +X, climbing `rise` as it goes: a quarter turn (at most) between
/// points, each point's handles along the way it goes. With `closed`, a
/// whole circle.
fn round(radius: f32, sweep: f32, rise: f32, closed: bool) -> Result<Curve, CurveError> {
    let spans = ((sweep / FRAC_PI_2) - 1e-4).ceil().max(1.0) as usize;
    let step = sweep / spans as f32;
    // The handle length that makes a cubic follow a circle closely.
    let reach = 4.0 / 3.0 * (step / 4.0).tan() * radius;
    let climb = rise / spans as f32 / 3.0;
    let count = if closed { spans } else { spans + 1 };
    let points = (0..count).map(|i| {
        let a = step * i as f32;
        let place = Vec3::new(
            radius * a.cos(),
            radius * a.sin(),
            rise * i as f32 / spans as f32,
        );
        let way = Vec3::new(-a.sin(), a.cos(), 0.0) * reach + Vec3::Z * climb;
        let first = !closed && i == 0;
        let last = !closed && i == count - 1;
        let handle_in = if first { Vec3::ZERO } else { -way };
        let handle_out = if last { Vec3::ZERO } else { way };
        (place, handle_in, handle_out, !first && !last)
    });
    Curve::new(points, closed)
}
