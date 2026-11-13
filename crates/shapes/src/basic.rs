//! The basic shapes (B on the canvas): a circle, a square, a polygon and
//! a star, made as ordinary paths so their points can be edited after.

use crate::path::{Knot, Path};
use glam::Vec2;
use std::f32::consts::{FRAC_PI_2, TAU};
use words::text;

/// The fewest and most sides of a polygon, or points of a star.
pub const SIDES: std::ops::RangeInclusive<u32> = 3..=32;

/// A basic shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basic {
    /// A circle.
    Circle,
    /// A square.
    Square,
    /// A polygon with this many sides.
    Polygon(u32),
    /// A star with this many points.
    Star(u32),
}

/// How far out a curve's handles reach, as a share of the radius, for a
/// quarter circle (the usual four-curve circle).
const QUARTER: f32 = 0.552_284_8;

/// How far in a star's inner points are, as a share of its radius.
const STAR_INNER: f32 = 0.45;

impl Basic {
    /// The shape's plain name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Circle => text!("Circle"),
            Self::Square => text!("Square"),
            Self::Polygon(_) => text!("Polygon"),
            Self::Star(_) => text!("Star"),
        }
    }

    /// The shape as a closed path round `middle`, reaching `radius` from
    /// it (a square's corners at the radius on each side; a polygon's and
    /// star's first point straight up).
    pub fn path(self, middle: Vec2, radius: f32) -> Path {
        match self {
            Self::Circle => {
                let knots = (0..4)
                    .map(|i| {
                        let a = i as f32 * FRAC_PI_2 - FRAC_PI_2;
                        let dir = Vec2::new(a.cos(), a.sin());
                        let along = Vec2::new(-dir.y, dir.x) * radius * QUARTER;
                        Knot::smooth(middle + dir * radius, along)
                    })
                    .collect();
                Path::new(knots, true)
            }
            Self::Square => Path::polygon(
                [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .map(|(x, y)| middle + Vec2::new(x, y) * radius),
            ),
            Self::Polygon(sides) => {
                let n = sides.clamp(*SIDES.start(), *SIDES.end());
                Path::polygon((0..n).map(|i| round(middle, radius, i, n)))
            }
            Self::Star(points) => {
                let n = points.clamp(*SIDES.start(), *SIDES.end());
                Path::polygon((0..2 * n).map(|i| {
                    let r = if i % 2 == 0 {
                        radius
                    } else {
                        radius * STAR_INNER
                    };
                    round(middle, r, i, 2 * n)
                }))
            }
        }
    }
}

/// Point `i` of `n` round `middle` at `radius`, the first straight up.
fn round(middle: Vec2, radius: f32, i: u32, n: u32) -> Vec2 {
    let a = i as f32 / n as f32 * TAU - FRAC_PI_2;
    middle + Vec2::new(a.cos(), a.sin()) * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_basic_shape_is_closed_round_its_middle_with_the_right_area() {
        let m = Vec2::splat(0.5);
        let circle = Basic::Circle.path(m, 0.25);
        let true_area = std::f32::consts::PI * 0.0625;
        assert!((circle.area() - true_area).abs() / true_area < 0.01);
        assert!(circle.is_closed() && circle.contains(m));
        assert!((Basic::Square.path(m, 0.25).area() - 0.25).abs() < 1e-6);
        let hexagon = Basic::Polygon(6).path(m, 0.25);
        assert_eq!(hexagon.knots().len(), 6);
        assert!((hexagon.knots()[0].at - Vec2::new(0.5, 0.25)).length() < 1e-6);
        let star = Basic::Star(5).path(m, 0.25);
        assert_eq!(star.knots().len(), 10);
        assert!(star.area() < hexagon.area());
        // Out of range counts are kept to the allowed ones.
        assert_eq!(Basic::Polygon(1).path(m, 0.2).knots().len(), 3);
        assert_eq!(Basic::Star(500).path(m, 0.2).knots().len(), 64);
        assert_eq!(Basic::Star(5).name(), "Star");
    }
}
