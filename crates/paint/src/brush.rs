//! The brush: its settings and how its strength falls off from its middle.

/// Smallest and largest brush radius, in screen points.
pub const RADIUS_RANGE: std::ops::RangeInclusive<f32> = 0.5..=1000.0;

/// What the brush does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushKind {
    /// Lays its colour on.
    #[default]
    Paint,
    /// Takes paint away.
    Erase,
}

/// A brush's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brush {
    /// Paint or erase.
    pub kind: BrushKind,
    /// Its radius on screen, in points.
    pub radius: f32,
    /// How much of its radius is at full strength, 0 to 1 (1: a hard
    /// edge; 0: fading from the middle).
    pub hardness: f32,
    /// How much one stroke lays on at most, 0 to 1. Within a stroke,
    /// paint doesn't pile up where dabs overlap.
    pub flow: f32,
    /// The gap between dabs, as a share of the brush's width.
    pub spacing: f32,
    /// Its colour (sRGB, with alpha).
    pub color: [u8; 4],
    /// Whether it paints parts hidden from the camera too.
    pub through: bool,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            kind: BrushKind::Paint,
            radius: 20.0,
            hardness: 0.8,
            flow: 1.0,
            spacing: 0.1,
            color: [200, 120, 60, 255],
            through: false,
        }
    }
}

impl Brush {
    /// The strength at `distance` from the middle, for a brush of
    /// `radius`: 1 inside its hard part, falling smoothly to 0 at its edge.
    pub fn falloff(&self, distance: f32, radius: f32) -> f32 {
        if radius <= 0.0 || distance >= radius {
            return 0.0;
        }
        let hard = self.hardness.clamp(0.0, 1.0) * radius;
        if distance <= hard {
            return 1.0;
        }
        let t = (distance - hard) / (radius - hard).max(1e-6);
        let s = 1.0 - t;
        s * s * (3.0 - 2.0 * s)
    }

    /// The distance between dabs, in points.
    pub fn step(&self) -> f32 {
        (self.spacing.clamp(0.01, 4.0) * self.radius * 2.0).max(0.5)
    }

    /// The same brush with its numbers kept in range.
    pub fn clamped(self) -> Self {
        Self {
            radius: self.radius.clamp(*RADIUS_RANGE.start(), *RADIUS_RANGE.end()),
            hardness: self.hardness.clamp(0.0, 1.0),
            flow: self.flow.clamp(0.0, 1.0),
            spacing: self.spacing.clamp(0.01, 4.0),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falloff_is_full_inside_and_none_outside() {
        let brush = Brush {
            hardness: 0.5,
            ..Brush::default()
        };
        assert_eq!(brush.falloff(0.0, 10.0), 1.0);
        assert_eq!(brush.falloff(5.0, 10.0), 1.0);
        assert_eq!(brush.falloff(10.0, 10.0), 0.0);
        let mid = brush.falloff(7.5, 10.0);
        assert!(mid > 0.3 && mid < 0.7, "{mid}");
        assert_eq!(brush.falloff(1.0, 0.0), 0.0);
    }

    #[test]
    fn a_hard_brush_has_an_edge() {
        let brush = Brush {
            hardness: 1.0,
            ..Brush::default()
        };
        assert_eq!(brush.falloff(9.99, 10.0), 1.0);
    }

    #[test]
    fn numbers_are_kept_in_range() {
        let wild = Brush {
            radius: -3.0,
            hardness: 4.0,
            flow: -1.0,
            spacing: 0.0,
            ..Brush::default()
        }
        .clamped();
        assert_eq!(wild.radius, 0.5);
        assert_eq!(wild.hardness, 1.0);
        assert_eq!(wild.flow, 0.0);
        assert!(wild.step() >= 0.5);
    }
}
