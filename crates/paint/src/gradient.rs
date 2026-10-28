//! Gradient fills ([decision 0194](../../../docs/decisions/0194-gradient-fills.md)):
//! a colour running into another along one of the object's axes, the
//! hand painter's quickest way to give a shape form (darker low, lighter
//! high). A gradient layer keeps its settings and is made again from
//! them on the shape (by the `bake` crate, which knows the shape's
//! texels).

/// Which way a gradient runs: along one of the object's own axes, low
/// to high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientAxis {
    /// Left to right.
    X,
    /// Front to back.
    Y,
    /// Bottom to top (Z is up in the app).
    #[default]
    Z,
}

impl GradientAxis {
    /// Every axis, in the order shown.
    pub const ALL: [GradientAxis; 3] = [GradientAxis::X, GradientAxis::Y, GradientAxis::Z];

    /// Which coordinate it follows: 0, 1 or 2.
    pub fn index(self) -> usize {
        match self {
            GradientAxis::X => 0,
            GradientAxis::Y => 1,
            GradientAxis::Z => 2,
        }
    }

    /// The axis for coordinate `index`.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
}

/// What each gradient runs across, from its low side to its high side
/// (decision 0199).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientSpan {
    /// Each object sharing the material.
    #[default]
    Object,
    /// Each separate part of an object (pieces not joined to the rest).
    Part,
    /// Each UV piece.
    UvPiece,
}

impl GradientSpan {
    /// Every span, in the order shown.
    pub const ALL: [GradientSpan; 3] = [Self::Object, Self::Part, Self::UvPiece];

    /// Its number in a file.
    pub fn index(self) -> usize {
        match self {
            Self::Object => 0,
            Self::Part => 1,
            Self::UvPiece => 2,
        }
    }

    /// The span numbered `index`.
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
}

/// A gradient fill's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gradient {
    /// The colour at the low end (sRGB, with alpha).
    pub from: [u8; 4],
    /// The colour at the high end.
    pub to: [u8; 4],
    /// Which way it runs.
    pub axis: GradientAxis,
    /// Where it starts and ends along the object's box, 0 (its low side)
    /// to 1 (its high side): before `start` it is all `from`, after `end`
    /// all `to`.
    pub start: f32,
    /// See `start`.
    pub end: f32,
    /// What it runs across: each object, part or UV piece (decision 0199).
    pub span: GradientSpan,
}

impl Default for Gradient {
    /// Dark at the bottom to light at the top, over the whole height: the
    /// usual first pass of form.
    fn default() -> Self {
        Self {
            from: [40, 40, 52, 255],
            to: [235, 230, 215, 255],
            axis: GradientAxis::Z,
            start: 0.0,
            end: 1.0,
            span: GradientSpan::Object,
        }
    }
}

impl Gradient {
    /// Whether its numbers are real and in range (0 to 1).
    pub fn is_valid(&self) -> bool {
        [self.start, self.end]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }

    /// Its colour at `t`, 0 to 1 along the object's box.
    pub fn at(&self, t: f32) -> [u8; 4] {
        let span = self.end - self.start;
        let k = if span.abs() < 1e-6 {
            if t < self.start { 0.0 } else { 1.0 }
        } else {
            ((t - self.start) / span).clamp(0.0, 1.0)
        };
        let mut out = [0u8; 4];
        for (c, o) in out.iter_mut().enumerate() {
            let (a, b) = (self.from[c] as f32, self.to[c] as f32);
            *o = (a + (b - a) * k).round().clamp(0.0, 255.0) as u8;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_run_from_start_to_end_and_hold_outside() {
        let g = Gradient {
            from: [0, 0, 0, 255],
            to: [200, 100, 0, 255],
            axis: GradientAxis::Z,
            start: 0.25,
            end: 0.75,
            span: GradientSpan::Object,
        };
        assert_eq!(g.at(0.0), [0, 0, 0, 255]);
        assert_eq!(g.at(0.5), [100, 50, 0, 255]);
        assert_eq!(g.at(1.0), [200, 100, 0, 255]);
        // Start and end at one place: a hard step.
        let step = Gradient {
            start: 0.5,
            end: 0.5,
            ..g
        };
        assert_eq!(step.at(0.49), [0, 0, 0, 255]);
        assert_eq!(step.at(0.51), [200, 100, 0, 255]);
        // Reversed: runs the other way.
        let back = Gradient {
            start: 1.0,
            end: 0.0,
            ..g
        };
        assert_eq!(back.at(1.0), [0, 0, 0, 255]);
        assert!(
            g.is_valid()
                && !Gradient {
                    start: f32::NAN,
                    ..g
                }
                .is_valid()
        );
        assert_eq!(GradientAxis::from_index(1), Some(GradientAxis::Y));
        assert_eq!(GradientAxis::from_index(3), None);
        assert_eq!(GradientSpan::from_index(2), Some(GradientSpan::UvPiece));
        assert_eq!(GradientSpan::from_index(3), None);
    }
}
