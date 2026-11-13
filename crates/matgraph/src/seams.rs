//! The seam check ([decision 0260](../../../docs/decisions/0260-preview-and-checks.md)):
//! whether a texture's joins show when it repeats.
//!
//! Where a texture repeats, its right edge meets its left edge and its
//! bottom meets its top. If they don't match, a line shows across the
//! surface every repeat. The check compares the step in value across
//! each join with the steps between neighbouring pixels everywhere else:
//! a join that jumps much more than the picture usually does shows.
//! A join stepping pixel for pixel as a line inside does (the edge of a
//! mortar line one repeat of the pattern earlier) doesn't: it is a line
//! of the pattern that happens to fall on the edge (milestone 27,
//! decision 0290).
//!
//! Every box in this crate reads its neighbours wrapping round
//! ([`crate::Image::at`]), so a graph's textures pass. The check is still
//! worth having: it says so in plain words, and it will catch pictures
//! brought in from outside (later milestones) that don't tile.

use crate::image::Grey;
use crate::value::Value;
use words::{tr, trf};

/// How many times bigger than the usual step a join's step may be before
/// it shows.
const TIMES_USUAL: f32 = 3.0;
/// A join's step that never shows, however smooth the picture is: about
/// two levels in 255, too little for the eye.
const TOO_SMALL_TO_SEE: f32 = 0.008;
/// How differently from every line of the picture's own a join must step
/// to show, as a share of its step (decision 0290: a join stepping as a
/// line inside does is a line of the pattern falling on the edge).
const LIKE_A_LINE: f32 = 0.25;

/// One direction's join: the step in value across it, the usual step
/// between neighbouring pixels away from it, and how far the line inside
/// most like it differs from it. All are averages along the join or line,
/// in 0 to 1 (the largest of every channel checked).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Join {
    /// The average step across the join.
    pub step: f32,
    /// The average step between neighbours elsewhere.
    pub usual: f32,
    /// How far the line inside most like the join (running the same way)
    /// steps differently, pixel for pixel along it: near 0 when the join
    /// is a line of the pattern (a mortar line's edge falling on the
    /// texture's edge, one repeat of the pattern earlier inside).
    pub alike: f32,
}

impl Join {
    /// Whether the join shows: its step is much bigger than the usual
    /// step, big enough to see, and unlike every line of the picture's own
    /// (a pattern whose straight lines fall on the edge, as a grid of tiles
    /// can, steps there exactly as at the same line one repeat inside).
    pub fn shows(&self) -> bool {
        self.step > TOO_SMALL_TO_SEE
            && self.step > self.usual * TIMES_USUAL
            && self.alike > self.step * LIKE_A_LINE
    }

    /// How much more the join jumps than the picture usually does (1 for
    /// a join no different from anywhere else).
    pub fn ratio(&self) -> f32 {
        if self.usual > 1e-6 {
            self.step / self.usual
        } else if self.step > 1e-6 {
            f32::INFINITY
        } else {
            1.0
        }
    }

    /// The worse of two joins (by how much they show).
    fn worse(self, other: Join) -> Join {
        match (self.shows(), other.shows()) {
            (true, false) => self,
            (false, true) => other,
            _ if other.ratio() > self.ratio() => other,
            _ => self,
        }
    }
}

/// What the seam check found: the join where the right edge meets the
/// left (`across`), and where the bottom meets the top (`down`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SeamCheck {
    /// Left and right edges, side by side when repeated.
    pub across: Join,
    /// Top and bottom edges.
    pub down: Join,
}

impl SeamCheck {
    /// Checks `value`: each of its channels (a material's colour, height
    /// and roughness, and an alpha card's opacity, its colour where it is
    /// solid; a colour's red, green and blue; a shape's or height's
    /// greys), the worst of them counting.
    pub fn of(value: &Value) -> Self {
        let mut out: Option<SeamCheck> = None;
        let mut add = |check: SeamCheck| {
            out = Some(match out {
                None => check,
                Some(o) => SeamCheck {
                    across: o.across.worse(check.across),
                    down: o.down.worse(check.down),
                },
            });
        };
        match value {
            Value::Shape(g) | Value::Height(g) => add(check_grey(|x, y| g.at(x, y), g.size())),
            Value::Colour(c) => {
                for channel in 0..3 {
                    add(check_grey(|x, y| c.at(x, y)[channel], c.size()));
                }
            }
            Value::Material(m) => {
                // An alpha card's colour shows only where it is solid
                // (decision 0276): a join in its see-through parts
                // doesn't show, but one in its opacity does.
                for channel in 0..3 {
                    add(check_grey(
                        |x, y| m.colour.at(x, y)[channel] * m.opacity_at(x, y),
                        m.size(),
                    ));
                }
                add(of_grey(&m.height));
                add(of_grey(&m.roughness));
                if let Some(opacity) = &m.opacity {
                    add(of_grey(opacity));
                }
            }
        }
        out.unwrap_or_default()
    }

    /// The same check with the top-bottom join left out: a trim sheet's
    /// last strip meets its first in a gap there, which is expected
    /// (decision 0289).
    pub fn across_only(self) -> Self {
        Self {
            down: Join::default(),
            ..self
        }
    }

    /// Whether any join shows.
    pub fn shows(&self) -> bool {
        self.across.shows() || self.down.shows()
    }

    /// What it found, in plain words, with what to do about it.
    pub fn describe(&self) -> String {
        match (self.across.shows(), self.down.shows()) {
            (false, false) => tr(
                "No joins show: the texture repeats seamlessly. If the repeats are still easy \
                 to spot, a large feature is repeating in a grid; more variation, or a larger \
                 real-world size, hides it.",
            )
            .to_string(),
            (true, false) => trf!(
                "A join shows where the left and right edges meet: they jump {} times more \
                 than the rest of the texture. Every box in a graph tiles, so the picture \
                 that doesn't is one that came from outside the graph.",
                rounded(self.across.ratio())
            ),
            (false, true) => trf!(
                "A join shows where the top and bottom edges meet: they jump {} times more \
                 than the rest of the texture. Every box in a graph tiles, so the picture \
                 that doesn't is one that came from outside the graph.",
                rounded(self.down.ratio())
            ),
            (true, true) => tr(
                "Joins show on both sides: the edges don't match the ones they meet when the \
                 texture repeats. Every box in a graph tiles, so the picture that doesn't is \
                 one that came from outside the graph.",
            )
            .to_string(),
        }
    }
}

/// A ratio for people: a whole number, or "many" past a thousand.
fn rounded(ratio: f32) -> String {
    if ratio.is_finite() && ratio < 1000.0 {
        format!("{}", ratio.round() as i64)
    } else {
        tr("many").to_string()
    }
}

fn of_grey(g: &Grey) -> SeamCheck {
    check_grey(|x, y| g.at(x, y), g.size())
}

/// Checks a square picture `size` across, read through `at` (which wraps
/// round).
fn check_grey(at: impl Fn(i64, i64) -> f32, size: u32) -> SeamCheck {
    let n = i64::from(size.max(1));
    if n < 2 {
        return SeamCheck::default();
    }
    // The signed step across each join, along it: what a line inside
    // must match to be the same line of the pattern.
    let edge_across: Vec<f32> = (0..n).map(|y| at(0, y) - at(n - 1, y)).collect();
    let edge_down: Vec<f32> = (0..n).map(|x| at(x, 0) - at(x, n - 1)).collect();
    let mut inner_across = 0.0f64;
    let mut inner_down = 0.0f64;
    let mut join_across = 0.0f64;
    let mut join_down = 0.0f64;
    // How far each line inside differs from its join: column `x` to
    // `x + 1`, row `y` to `y + 1`.
    let mut columns = vec![0.0f64; n as usize];
    let mut rows = vec![0.0f64; n as usize];
    let differs = |step: f32, edge: f32| {
        let d = (step - edge).abs();
        f64::from(if d.is_finite() { d } else { 1.0 })
    };
    for y in 0..n {
        for x in 0..n {
            let here = at(x, y);
            let right = at(x + 1, y) - here;
            let below = at(x, y + 1) - here;
            columns[x as usize] += differs(right, edge_across[y as usize]);
            rows[y as usize] += differs(below, edge_down[x as usize]);
            // Not a number counts as the largest step: it would show.
            let right = if right.is_finite() { right.abs() } else { 1.0 };
            let below = if below.is_finite() { below.abs() } else { 1.0 };
            if x == n - 1 {
                join_across += f64::from(right);
            } else {
                inner_across += f64::from(right);
            }
            if y == n - 1 {
                join_down += f64::from(below);
            } else {
                inner_down += f64::from(below);
            }
        }
    }
    let lines = n as f64;
    let inner = (n * (n - 1)) as f64;
    let most_alike = |sums: &[f64]| {
        sums[..sums.len() - 1]
            .iter()
            .fold(f64::INFINITY, |least, s| least.min(*s))
            / lines
    };
    SeamCheck {
        across: Join {
            step: (join_across / lines) as f32,
            usual: (inner_across / inner) as f32,
            alike: most_alike(&columns) as f32,
        },
        down: Join {
            step: (join_down / lines) as f32,
            usual: (inner_down / inner) as f32,
            alike: most_alike(&rows) as f32,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{Image, Size};
    use crate::{Evaluator, Graph, GraphEdit, Registry, ValueKind};
    use std::sync::Arc;

    fn grey(size: u32, f: impl Fn(u32, u32) -> f32) -> Value {
        let s = Size::new(size).unwrap();
        let pixels = (0..size * size).map(|i| f(i % size, i / size)).collect();
        Value::Height(Arc::new(Image::from_pixels(s, pixels).unwrap()))
    }

    #[test]
    fn a_tiling_pattern_has_no_joins_that_show() {
        let registry = Registry::standard();
        for kind in ["noise", "cells", "bricks", "clouds", "planks"] {
            let mut g = Graph::new("Check");
            g.apply(
                registry,
                &GraphEdit::AddBox {
                    kind: kind.into(),
                    at: glam::Vec2::ZERO,
                },
            )
            .unwrap();
            let mut e = Evaluator::new();
            let size = Size::new(128).unwrap();
            let eval = e.run(registry, &g, size, &[]);
            let value = eval.picture(crate::BoxId(1)).expect(kind);
            let check = SeamCheck::of(value);
            assert!(!check.shows(), "{kind}: {check:?}");
            assert!(check.describe().starts_with("No joins show"), "{kind}");
        }
    }

    #[test]
    fn a_ramp_shows_a_join_across_only() {
        let ramp = grey(64, |x, _| x as f32 / 63.0);
        let check = SeamCheck::of(&ramp);
        assert!(check.across.shows(), "{check:?}");
        assert!(!check.down.shows(), "{check:?}");
        assert!(check.across.ratio() > 50.0);
        assert!(check.describe().contains("left and right"));
        // Turned on its side, the join is the other one.
        let ramp = grey(64, |_, y| y as f32 / 63.0);
        let check = SeamCheck::of(&ramp);
        assert!(check.down.shows() && !check.across.shows());
        assert!(check.describe().contains("top and bottom"));
        let both = grey(64, |x, y| (x + y) as f32 / 126.0);
        assert!(
            SeamCheck::of(&both)
                .describe()
                .starts_with("Joins show on both")
        );
    }

    #[test]
    fn flat_and_noisy_pictures_pass() {
        // Flat: nothing steps anywhere.
        let flat = Value::flat_grey(ValueKind::Height, Size::new(16).unwrap(), 0.4).unwrap();
        let check = SeamCheck::of(&flat);
        assert!(!check.shows());
        assert_eq!(check.across.ratio(), 1.0);
        // White noise: the join steps as much as anywhere else.
        let noisy = grey(64, |x, y| {
            crate::seed::unit(crate::seed::mix(u64::from(x), u64::from(y)))
        });
        assert!(!SeamCheck::of(&noisy).shows());
        // A tiny step at the join of a smooth picture is too small to see.
        let faint = grey(64, |x, _| 0.5 + x as f32 * 0.0001);
        assert!(!SeamCheck::of(&faint).shows());
    }

    #[test]
    fn a_colour_or_material_is_checked_in_every_channel() {
        let size = Size::new(32).unwrap();
        // Only blue fails to tile.
        let pixels = (0..32 * 32)
            .map(|i| [0.5, 0.5, (i % 32) as f32 / 31.0])
            .collect();
        let colour = Value::Colour(Arc::new(Image::from_pixels(size, pixels).unwrap()));
        assert!(SeamCheck::of(&colour).across.shows());
        // A material whose height alone doesn't tile.
        let flat = Arc::new(Image::filled(size, 0.5f32));
        let ramp = Arc::new(
            Image::from_pixels(size, (0..32 * 32).map(|i| (i / 32) as f32 / 31.0).collect())
                .unwrap(),
        );
        let m = crate::MaterialMaps {
            colour: Arc::new(Image::filled(size, [0.3, 0.3, 0.3])),
            height: ramp,
            roughness: flat.clone(),
            metal: flat.clone(),
            normal: Arc::new(Image::filled(size, [0.0, 0.0, 1.0])),
            occlusion: flat.clone(),
            curvature: flat,
            opacity: None,
            translucency: None,
        };
        let check = SeamCheck::of(&Value::Material(Arc::new(m.clone())));
        assert!(check.down.shows() && !check.across.shows(), "{check:?}");
        // An alpha card: a colour that doesn't tile where it is cut away
        // doesn't show; an opacity that doesn't tile does.
        let pixels = (0..32 * 32)
            .map(|i| [(i % 32) as f32 / 31.0, 0.3, 0.3])
            .collect();
        let middle = |x: u32, _| if (4..28).contains(&x) { 1.0 } else { 0.0 };
        let card = crate::MaterialMaps {
            colour: Arc::new(Image::from_pixels(size, pixels).unwrap()),
            height: flat_tile(size),
            opacity: Some(Arc::new(crate::parallel::fill(size, 1, middle))),
            ..m.clone()
        };
        assert!(!SeamCheck::of(&Value::Material(Arc::new(card.clone()))).shows());
        let leaning = |x: u32, _| x as f32 / 31.0;
        let card = crate::MaterialMaps {
            opacity: Some(Arc::new(crate::parallel::fill(size, 1, leaning))),
            ..card
        };
        assert!(
            SeamCheck::of(&Value::Material(Arc::new(card)))
                .across
                .shows()
        );
    }

    fn flat_tile(size: Size) -> Arc<crate::image::Grey> {
        Arc::new(Image::filled(size, 0.5))
    }

    #[test]
    fn a_line_of_the_pattern_on_the_edge_is_not_a_join() {
        // Regression (milestone 21's false alarm): a grid whose lines'
        // edges fall on the texture's edge steps there no more than at its
        // other lines.
        let grid = grey(64, |x, y| if x % 16 < 3 || y % 16 < 3 { 0.1 } else { 0.8 });
        let check = SeamCheck::of(&grid);
        assert!(!check.shows(), "{check:?}");
        assert!(check.across.alike < 1e-6);
        // A grid whose right half is shifted down a few rows (a picture
        // that doesn't tile): its lines don't meet at the join, which shows.
        let joined = grey(64, |x, y| {
            let y = if x >= 32 { y + 5 } else { y };
            if (5..8).contains(&(x % 16)) || y % 16 < 3 {
                0.1
            } else {
                0.8
            }
        });
        assert!(
            SeamCheck::of(&joined).across.shows(),
            "{:?}",
            SeamCheck::of(&joined)
        );
    }

    #[test]
    fn a_trim_sheets_check_leaves_the_top_bottom_join_out() {
        let ramp = grey(64, |_, y| y as f32 / 63.0);
        let check = SeamCheck::of(&ramp);
        assert!(check.down.shows());
        let across = check.across_only();
        assert!(!across.shows());
        assert!(across.describe().starts_with("No joins show"));
    }

    #[test]
    fn broken_numbers_count_as_a_join_that_shows() {
        let broken = grey(16, |x, _| if x == 15 { f32::NAN } else { 0.5 });
        let check = SeamCheck::of(&broken);
        assert!(check.across.shows(), "{check:?}");
        assert_eq!(rounded(f32::INFINITY), "many");
        assert_eq!(rounded(12.4), "12");
    }
}
