//! Symmetry while sculpting (decision 0310): each dab repeated reflected
//! across the shape's own planes through its pivot (the origin of its
//! own space), and turned round an axis for radial symmetry. The copies
//! of a dab are worked out together from the shape as it was before the
//! dab, so a stroke on a symmetric shape stays symmetric.
//!
//! Make symmetrical copies one side's sculpting onto the other, for a
//! sculpt that drifted: it needs the shape's points to be symmetric in
//! position, and says when they aren't.

use crate::nearest::Grid;
use glam::{Mat3, Vec3};

/// The most copies round an axis.
pub const MOST_RADIAL: u32 = 64;

/// Which symmetry a dab is repeated with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Symmetry {
    /// Reflected across the plane square to each axis (X: left and
    /// right; Y: front and back; Z: up and down).
    pub axes: [bool; 3],
    /// Turned round this axis into this many copies (2 to
    /// [`MOST_RADIAL`]; fewer than 2 is none).
    pub radial: Option<(usize, u32)>,
}

/// One copy of a dab: how a place is carried to it, and whether that
/// turns the brush's picture over (a reflection).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Repeat {
    /// Carries a place or direction in the shape's own space to the copy.
    pub turn: Mat3,
    /// Whether it reflects (an odd number of mirrors).
    pub reflects: bool,
}

impl Repeat {
    /// The dab itself.
    pub const SAME: Self = Self {
        turn: Mat3::IDENTITY,
        reflects: false,
    };
}

impl Symmetry {
    /// None at all.
    pub const NONE: Self = Self {
        axes: [false; 3],
        radial: None,
    };

    /// Whether a dab is repeated at all.
    pub fn is_on(&self) -> bool {
        self.copies().len() > 1
    }

    /// Every copy of a dab, the dab itself first: each reflection by the
    /// chosen axes (every combination of them), each turned round the
    /// radial axis.
    pub fn copies(&self) -> Vec<Repeat> {
        let mut mirrors = vec![Repeat::SAME];
        for (axis, on) in self.axes.iter().enumerate() {
            if !on {
                continue;
            }
            let mut scale = Vec3::ONE;
            scale[axis] = -1.0;
            let reflect = Mat3::from_diagonal(scale);
            let more: Vec<Repeat> = mirrors
                .iter()
                .map(|c| Repeat {
                    turn: reflect * c.turn,
                    reflects: !c.reflects,
                })
                .collect();
            mirrors.extend(more);
        }
        let Some((axis, count)) = self.radial.filter(|(a, n)| *a < 3 && *n >= 2) else {
            return mirrors;
        };
        let count = count.min(MOST_RADIAL);
        let mut around = Vec3::ZERO;
        around[axis] = 1.0;
        let mut out = Vec::with_capacity(mirrors.len() * count as usize);
        for k in 0..count {
            let angle = std::f32::consts::TAU * k as f32 / count as f32;
            let turn = Mat3::from_axis_angle(around, angle);
            for c in &mirrors {
                out.push(Repeat {
                    turn: turn * c.turn,
                    reflects: c.reflects,
                });
            }
        }
        out
    }
}

/// Why a shape couldn't be made symmetrical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotSymmetric {
    /// This many of the points on the side copied to have no point at
    /// their reflected place on the shape under the sculpt.
    Unmatched(usize),
    /// The shape has no points.
    Empty,
}

/// For each point slot on the side `positive` (or negative) of `axis`,
/// the slot of the point at its reflected place (within `tolerance`),
/// from `places` (the shape under the sculpt, by slot; not a number for an
/// empty slot). Points on the plane match themselves. An error when any
/// point on the copied side has no match.
pub fn mirror_pairs(
    places: &[Vec3],
    axis: usize,
    positive: bool,
    tolerance: f32,
) -> Result<Vec<(usize, usize)>, NotSymmetric> {
    let axis = axis.min(2);
    let grid = Grid::new(places).ok_or(NotSymmetric::Empty)?;
    let mut scale = Vec3::ONE;
    scale[axis] = -1.0;
    let mut pairs = Vec::new();
    let mut unmatched = 0;
    for (i, p) in places.iter().enumerate() {
        if !p.is_finite() {
            continue;
        }
        // Each point on the side written to takes its partner's moves.
        let written = if positive {
            p[axis] < 0.0
        } else {
            p[axis] > 0.0
        };
        if !written && p[axis].abs() > tolerance {
            continue;
        }
        let reflected = *p * scale;
        match grid.nearest(places, reflected) {
            Some(j) if places[j as usize].distance(reflected) <= tolerance => {
                pairs.push((i, j as usize));
            }
            _ => unmatched += 1,
        }
    }
    if unmatched > 0 {
        return Err(NotSymmetric::Unmatched(unmatched));
    }
    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_combine_every_axis_and_the_radial_turns() {
        assert_eq!(Symmetry::NONE.copies(), vec![Repeat::SAME]);
        assert!(!Symmetry::NONE.is_on());
        let x = Symmetry {
            axes: [true, false, false],
            radial: None,
        };
        let copies = x.copies();
        assert_eq!(copies.len(), 2);
        assert_eq!(
            copies[1].turn * Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-1.0, 2.0, 3.0)
        );
        assert!(copies[1].reflects);
        let all = Symmetry {
            axes: [true, true, true],
            radial: None,
        };
        assert_eq!(all.copies().len(), 8);
        // Radial: 2 and 64 copies (the edges of its range), more is 64.
        for (asked, made) in [(2, 2), (64, 64), (1000, 64), (1, 1), (0, 1)] {
            let radial = Symmetry {
                axes: [false; 3],
                radial: Some((2, asked)),
            };
            assert_eq!(radial.copies().len(), made, "{asked}");
        }
        let four = Symmetry {
            axes: [false; 3],
            radial: Some((2, 4)),
        };
        let p = four.copies()[1].turn * Vec3::X;
        assert!(p.distance(Vec3::Y) < 1e-6, "{p}");
        let both = Symmetry {
            axes: [true, false, false],
            radial: Some((2, 3)),
        };
        assert_eq!(both.copies().len(), 6);
    }

    #[test]
    fn pairs_need_a_point_at_each_reflected_place() {
        let places = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::NAN,
        ];
        let pairs = mirror_pairs(&places, 0, true, 1e-4).unwrap();
        assert!(pairs.contains(&(1, 0)));
        assert!(pairs.contains(&(2, 2)));
        assert!(!pairs.iter().any(|(i, _)| *i == 0));
        let lopsided = vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.5, 0.0, 0.0)];
        assert_eq!(
            mirror_pairs(&lopsided, 0, true, 1e-4),
            Err(NotSymmetric::Unmatched(1))
        );
        assert_eq!(mirror_pairs(&[], 0, true, 1e-4), Err(NotSymmetric::Empty));
    }
}
