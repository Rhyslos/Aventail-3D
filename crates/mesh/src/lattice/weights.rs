//! How much each of a lattice's points along one side pulls a place
//! (decision 0144): straight between neighbours, or a cubic B-spline.

use super::Interpolation;

/// The points along one side that pull a place, and how much (at most
/// four, adding up to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Weights {
    /// The first point's place along the side.
    first: usize,
    weights: [f32; 4],
    count: usize,
}

impl Weights {
    /// The weights at fraction `t` (0 to 1) across a side of `n` points
    /// (at least 2).
    pub(super) fn of(t: f32, n: u32, interpolation: Interpolation) -> Self {
        let n = n.max(2) as usize;
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match interpolation {
            Interpolation::Linear => linear(t, n),
            Interpolation::Smooth => b_spline(t, n),
        }
    }

    /// Each pulling point's place along the side, and its weight.
    pub(super) fn iter(&self) -> impl Iterator<Item = (usize, f32)> + '_ {
        self.weights[..self.count]
            .iter()
            .enumerate()
            .map(|(i, w)| (self.first + i, *w))
    }
}

/// Straight between the two points either side.
fn linear(t: f32, n: usize) -> Weights {
    let s = t * (n - 1) as f32;
    let i = (s.floor() as usize).min(n - 2);
    let f = s - i as f32;
    Weights {
        first: i,
        weights: [1.0 - f, f, 0.0, 0.0],
        count: 2,
    }
}

/// A clamped uniform B-spline of degree `min(3, n − 1)`: the end points
/// pull the ends fully, and with 2 points it is straight, 3 a quadratic
/// and 4 a cubic Bézier. The basis functions are worked out by the
/// Cox–de Boor recurrence (Piegl and Tiller, *The NURBS Book*, A2.2).
fn b_spline(t: f32, n: usize) -> Weights {
    let degree = (n - 1).min(3);
    let pieces = n - degree;
    // The knots: `degree + 1` zeros, evenly spaced inner knots, and
    // `degree + 1` ones.
    let knot = |k: usize| -> f32 {
        if k <= degree {
            0.0
        } else if k >= n {
            1.0
        } else {
            (k - degree) as f32 / pieces as f32
        }
    };
    let span = (degree + (t * pieces as f32).floor() as usize).min(n - 1);
    let mut basis = [0.0f32; 4];
    let mut left = [0.0f32; 4];
    let mut right = [0.0f32; 4];
    basis[0] = 1.0;
    for j in 1..=degree {
        left[j] = t - knot(span + 1 - j);
        right[j] = knot(span + j) - t;
        let mut saved = 0.0;
        for r in 0..j {
            let gap = right[r + 1] + left[j - r];
            let temp = if gap.abs() > f32::EPSILON {
                basis[r] / gap
            } else {
                0.0
            };
            basis[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        basis[j] = saved;
    }
    Weights {
        first: span - degree,
        weights: basis,
        count: degree + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(w: &Weights) -> f32 {
        w.iter().map(|(_, w)| w).sum()
    }

    #[test]
    fn weights_add_up_to_one_and_stay_in_the_side() {
        for interpolation in Interpolation::ALL {
            for n in 2..=16 {
                for step in 0..=200 {
                    let t = step as f32 / 200.0;
                    let w = Weights::of(t, n, interpolation);
                    assert!((total(&w) - 1.0).abs() < 1e-5, "{n} {t} {interpolation:?}");
                    assert!(w.iter().all(|(i, w)| i < n as usize && w >= -1e-6));
                }
            }
        }
    }

    #[test]
    fn the_ends_follow_the_end_points_fully() {
        for interpolation in Interpolation::ALL {
            for n in 2..=16 {
                let start = Weights::of(0.0, n, interpolation);
                let first = start.iter().find(|(i, _)| *i == 0);
                assert!(
                    (first.unwrap().1 - 1.0).abs() < 1e-6,
                    "{n} {interpolation:?}"
                );
                let end = Weights::of(1.0, n, interpolation);
                let last = end.iter().find(|(i, _)| *i == n as usize - 1);
                assert!(
                    (last.unwrap().1 - 1.0).abs() < 1e-6,
                    "{n} {interpolation:?}"
                );
            }
        }
    }

    #[test]
    fn two_points_are_linear_either_way() {
        for t in [0.0, 0.25, 0.7, 1.0] {
            let a = Weights::of(t, 2, Interpolation::Smooth);
            let b = Weights::of(t, 2, Interpolation::Linear);
            for ((ia, wa), (ib, wb)) in a.iter().zip(b.iter()) {
                assert_eq!(ia, ib);
                assert!((wa - wb).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn linear_weights_peak_at_their_point() {
        let w = Weights::of(0.5, 3, Interpolation::Linear);
        assert_eq!(w.iter().find(|(_, w)| *w > 0.99).map(|(i, _)| i), Some(1));
    }

    #[test]
    fn odd_fractions_are_brought_within_the_side() {
        for t in [f32::NAN, f32::INFINITY, -3.0, 7.0] {
            for interpolation in Interpolation::ALL {
                let w = Weights::of(t, 5, interpolation);
                assert!((total(&w) - 1.0).abs() < 1e-5);
            }
        }
        // Fewer than two points count as two.
        assert!((total(&Weights::of(0.3, 0, Interpolation::Smooth)) - 1.0).abs() < 1e-5);
    }
}
