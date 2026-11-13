//! Where Scatter's copies go: in a grid or at random, each with its own
//! size, turn, height and colour, all from the box's seed, and a grid of
//! cells to find the copies near a pixel quickly.
//!
//! Places are on the texture from 0 to 1 each way and wrap round: a copy
//! near an edge reaches over it and comes back on the other side, so
//! what Scatter makes tiles seamlessly. Everything here is worked out on
//! one core in a fixed order, so the same seed always places the same
//! copies.

use crate::image::Grey;
use crate::seed::{mix, unit};

/// The most copies one Scatter box places.
pub(crate) const MAX_COPIES: usize = 16_384;

/// The most copies in a grid across (or down).
pub(crate) const MAX_GRID: u32 = 128;

/// How many times over the copies may cover the texture, on average:
/// more is left out, so very big copies placed very close never take
/// long to work out.
pub(crate) const MAX_DEPTH: f32 = 32.0;

/// How many tries per copy wanted, when copies are kept apart at random.
const TRIES: usize = 4;

/// What Scatter's settings say about placing, in texture widths.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan {
    /// A grid (true) or at random.
    pub(crate) grid: bool,
    /// The distance between neighbouring copies.
    pub(crate) spacing: f32,
    /// How far each copy may stray from its grid place, as a share of
    /// the spacing (a grid only).
    pub(crate) loose: f32,
    /// How far every other row moves along, as a share of the spacing.
    pub(crate) shift: f32,
    /// The biggest copy's width.
    pub(crate) size: f32,
    /// How much smaller a copy may be, at random (0 to under 1).
    pub(crate) size_vary: f32,
    /// Every copy's turn, in radians, clockwise.
    pub(crate) turn: f32,
    /// How far each copy may turn either way from that, in radians.
    pub(crate) turn_vary: f32,
    /// How much lower a copy may be, at random (0 to 1).
    pub(crate) height_vary: f32,
    /// How much a copy's colour may change, at random (0 to 1).
    pub(crate) colour_vary: f32,
    /// Whether copies are kept from overlapping.
    pub(crate) apart: bool,
    /// The box's seed.
    pub(crate) seed: u64,
}

/// One copy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placed {
    /// Its middle, across and down (0 to 1).
    pub(crate) x: f32,
    /// Down.
    pub(crate) y: f32,
    /// Its width (and height), in texture widths.
    pub(crate) size: f32,
    /// Its turn's cosine.
    pub(crate) cos: f32,
    /// Its turn's sine.
    pub(crate) sin: f32,
    /// What its height is multiplied by (1 is as tall as the input).
    pub(crate) height: f32,
    /// What its colour is multiplied by, channel by channel.
    pub(crate) tint: [f32; 3],
    /// Its own grey, for the Random output (0.1 to 1).
    pub(crate) grey: f32,
    /// Which copy is on top where copies of the same height meet (0 to 1).
    pub(crate) rank: f32,
}

impl Placed {
    /// Half the width of the square around it, turned.
    pub(crate) fn reach(&self) -> f32 {
        self.size * 0.5 * (self.cos.abs() + self.sin.abs())
    }
}

/// The copies `plan` places, in order; a copy whose middle falls where
/// `mask` is dark is left out (where it is grey, some are).
pub(crate) fn place(plan: &Plan, mask: Option<&Grey>) -> Vec<Placed> {
    let size = finite_or(plan.size, 0.0).clamp(0.0, 1.0);
    let size_vary = finite_or(plan.size_vary, 0.0).clamp(0.0, 0.95);
    let spacing = finite_or(plan.spacing, 1.0).clamp(1e-4, 1.0);
    if size <= 0.0 {
        return Vec::new();
    }
    // The average copy's area, to keep the pile from getting too deep.
    let average = size * (1.0 - size_vary * 0.5);
    let most = ((MAX_DEPTH / (average * average)) as usize).clamp(1, MAX_COPIES);
    let candidates = if plan.grid {
        grid(plan, spacing, most)
    } else {
        let wanted = ((1.0 / (spacing * spacing)).round() as usize).clamp(1, most);
        let tries = if plan.apart { wanted * TRIES } else { wanted };
        Candidates {
            places: (0..tries)
                .map(|i| {
                    let h = mix(plan.seed, i as u64);
                    (unit(mix(h, 1)), unit(mix(h, 2)))
                })
                .collect(),
            wanted,
        }
    };
    let mut apart = Apart::new(size);
    let mut out = Vec::new();
    for (i, &(x, y)) in candidates.places.iter().enumerate() {
        if out.len() >= candidates.wanted {
            break;
        }
        let h = mix(mix(plan.seed, 0x5CA7), i as u64);
        let r = |k: u64| unit(mix(h, k));
        if let Some(mask) = mask
            && r(3) >= mask.sample(x, y)
        {
            continue;
        }
        let copy_size = size * (1.0 - size_vary * r(4));
        if plan.apart && !apart.fits(x, y, copy_size) {
            continue;
        }
        apart.add(x, y, copy_size);
        let turn = finite_or(plan.turn, 0.0) + finite_or(plan.turn_vary, 0.0) * (r(5) * 2.0 - 1.0);
        let (sin, cos) = turn.sin_cos();
        let hv = finite_or(plan.height_vary, 0.0).clamp(0.0, 1.0);
        let cv = finite_or(plan.colour_vary, 0.0).clamp(0.0, 1.0);
        let light = 1.0 + cv * 0.5 * (r(6) * 2.0 - 1.0);
        let tint = [7, 8, 9].map(|k| light * (1.0 + cv * 0.2 * (r(k) * 2.0 - 1.0)));
        out.push(Placed {
            x,
            y,
            size: copy_size,
            cos,
            sin,
            height: 1.0 - hv * r(10),
            tint,
            grey: 0.1 + 0.9 * r(11),
            rank: r(12),
        });
    }
    out
}

/// `v`, or `or` when it isn't a real number.
fn finite_or(v: f32, or: f32) -> f32 {
    if v.is_finite() { v } else { or }
}

/// Places to try, in order, and how many copies are wanted.
struct Candidates {
    places: Vec<(f32, f32)>,
    wanted: usize,
}

/// A grid's places, `spacing` apart (made to fit a whole number of times
/// across and down), each moved a little at random by Loose.
fn grid(plan: &Plan, spacing: f32, most: usize) -> Candidates {
    let fit = (most as f32).sqrt().floor().max(1.0) as u32;
    let across = ((1.0 / spacing).round() as u32).clamp(1, MAX_GRID.min(fit));
    // Shifted rows only meet up again after an even number of rows.
    let shift = finite_or(plan.shift, 0.0).clamp(0.0, 1.0);
    let down = if shift > 0.0 && across % 2 == 1 {
        across + 1
    } else {
        across
    };
    let loose = finite_or(plan.loose, 0.0).clamp(0.0, 1.0);
    let (a, d) = (across as f32, down as f32);
    let mut places = Vec::with_capacity((across * down) as usize);
    for row in 0..down {
        let along = if row % 2 == 1 { shift } else { 0.0 };
        for col in 0..across {
            let h = mix(
                mix(plan.seed, 0x6121D),
                (u64::from(row) << 32) | u64::from(col),
            );
            let jx = loose * (unit(mix(h, 1)) - 0.5);
            let jy = loose * (unit(mix(h, 2)) - 0.5);
            let x = (col as f32 + 0.5 + along + jx) / a;
            let y = (row as f32 + 0.5 + jy) / d;
            places.push((x.rem_euclid(1.0), y.rem_euclid(1.0)));
        }
    }
    let wanted = places.len();
    Candidates { places, wanted }
}

/// The copies placed so far, in cells, to keep new ones from
/// overlapping them (each copy's round outline, its width across).
struct Apart {
    cells: u32,
    placed: Vec<Vec<(f32, f32, f32)>>,
}

impl Apart {
    fn new(biggest: f32) -> Self {
        // Cells at least as wide as the biggest copy, so only the cells
        // around a place need looking at; fewer than three across is one.
        let cells = ((1.0 / biggest.max(1e-4)).floor() as u32).clamp(1, MAX_GRID);
        let cells = if cells < 3 { 1 } else { cells };
        Self {
            cells,
            placed: vec![Vec::new(); (cells * cells) as usize],
        }
    }

    fn cell(&self, v: f32) -> i64 {
        ((v * self.cells as f32).floor() as i64).rem_euclid(i64::from(self.cells))
    }

    fn fits(&self, x: f32, y: f32, size: f32) -> bool {
        let (cx, cy) = (self.cell(x), self.cell(y));
        let n = i64::from(self.cells);
        let near: &[i64] = if self.cells == 1 { &[0] } else { &[-1, 0, 1] };
        for dy in near {
            for dx in near {
                let at = ((cy + dy).rem_euclid(n) * n + (cx + dx).rem_euclid(n)) as usize;
                for &(ox, oy, os) in &self.placed[at] {
                    let (mut ddx, mut ddy) = (x - ox, y - oy);
                    ddx -= ddx.round();
                    ddy -= ddy.round();
                    let reach = (size + os) * 0.5;
                    if ddx * ddx + ddy * ddy < reach * reach {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn add(&mut self, x: f32, y: f32, size: f32) {
        let at = (self.cell(y) * i64::from(self.cells) + self.cell(x)) as usize;
        self.placed[at].push((x, y, size));
    }
}

/// One copy in a lookup cell: which, and its middle as seen from pixels
/// in that cell (moved by whole textures for copies reaching over an
/// edge).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    pub(crate) copy: u32,
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// The copies near each part of the texture: a grid of cells, each
/// listing every copy reaching into it.
#[derive(Debug)]
pub(crate) struct Lookup {
    cells: u32,
    starts: Vec<u32>,
    entries: Vec<Entry>,
}

impl Lookup {
    /// The lookup for `copies`.
    pub(crate) fn new(copies: &[Placed]) -> Self {
        let reach = copies.iter().map(Placed::reach).fold(0.0_f32, f32::max);
        // Cells about as wide as a copy reaches: a pixel then looks at
        // few copies too far away, and each copy is listed in about nine.
        let cells = if reach > 0.0 {
            ((1.0 / reach).floor() as u32).clamp(1, 256)
        } else {
            1
        };
        let g = cells as f32;
        let n = i64::from(cells);
        let mut found: Vec<(u32, Entry)> = Vec::new();
        for (i, c) in copies.iter().enumerate() {
            let r = c.reach();
            let span = |mid: f32| {
                (
                    ((mid - r) * g).floor() as i64,
                    ((mid + r) * g).floor() as i64,
                )
            };
            let (x0, x1) = span(c.x);
            let (y0, y1) = span(c.y);
            for cy in y0..=y1 {
                for cx in x0..=x1 {
                    let at = cy.rem_euclid(n) * n + cx.rem_euclid(n);
                    found.push((
                        at as u32,
                        Entry {
                            copy: i as u32,
                            x: c.x - cx.div_euclid(n) as f32,
                            y: c.y - cy.div_euclid(n) as f32,
                        },
                    ));
                }
            }
        }
        // In cell order, and within a cell in the copies' order.
        found.sort_by_key(|(at, _)| *at);
        let mut starts = vec![0u32; (cells * cells + 1) as usize];
        for (at, _) in &found {
            starts[*at as usize + 1] += 1;
        }
        for i in 1..starts.len() {
            starts[i] += starts[i - 1];
        }
        Self {
            cells,
            starts,
            entries: found.into_iter().map(|(_, e)| e).collect(),
        }
    }

    /// The copies reaching the place `(x, y)` (0 to 1) might cover.
    pub(crate) fn near(&self, x: f32, y: f32) -> &[Entry] {
        let g = self.cells as f32;
        let last = self.cells - 1;
        let cx = ((x * g) as u32).min(last);
        let cy = ((y * g) as u32).min(last);
        let at = (cy * self.cells + cx) as usize;
        &self.entries[self.starts[at] as usize..self.starts[at + 1] as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{Image, Size};

    fn plan() -> Plan {
        Plan {
            grid: false,
            spacing: 0.1,
            loose: 0.0,
            shift: 0.0,
            size: 0.1,
            size_vary: 0.3,
            turn: 0.0,
            turn_vary: 1.0,
            height_vary: 0.3,
            colour_vary: 0.3,
            apart: false,
            seed: 9,
        }
    }

    #[test]
    fn at_random_places_one_copy_per_spacing_squared_inside_the_texture() {
        let copies = place(&plan(), None);
        assert_eq!(copies.len(), 100);
        for c in &copies {
            assert!((0.0..1.0).contains(&c.x) && (0.0..1.0).contains(&c.y));
            assert!(c.size <= 0.1 + 1e-6 && c.size >= 0.1 * 0.7 - 1e-6);
            assert!((0.7..=1.0).contains(&c.height));
            assert!((0.1..=1.0).contains(&c.grey));
        }
        assert_eq!(copies, place(&plan(), None));
        let other = place(&Plan { seed: 10, ..plan() }, None);
        assert_ne!(copies, other);
    }

    #[test]
    fn a_grid_has_whole_rows_and_shifted_rows_come_in_pairs() {
        let p = Plan {
            grid: true,
            spacing: 0.2,
            ..plan()
        };
        let copies = place(&p, None);
        assert_eq!(copies.len(), 25);
        assert!((copies[0].x - 0.1).abs() < 1e-6 && (copies[0].y - 0.1).abs() < 1e-6);
        let shifted = place(&Plan { shift: 0.5, ..p }, None);
        assert_eq!(shifted.len(), 30, "five across, six down");
    }

    #[test]
    fn kept_apart_copies_never_overlap_even_across_the_edges() {
        let p = Plan {
            apart: true,
            spacing: 0.05,
            ..plan()
        };
        let copies = place(&p, None);
        assert!(copies.len() > 20, "{}", copies.len());
        for (i, a) in copies.iter().enumerate() {
            for b in &copies[i + 1..] {
                let (mut dx, mut dy) = (a.x - b.x, a.y - b.y);
                dx -= dx.round();
                dy -= dy.round();
                let d = (dx * dx + dy * dy).sqrt();
                assert!(d >= (a.size + b.size) * 0.5 - 1e-5, "{d}");
            }
        }
    }

    #[test]
    fn the_mask_says_where_copies_may_go() {
        let size = Size::new(16).unwrap();
        let black = Image::filled(size, 0.0);
        assert!(place(&plan(), Some(&black)).is_empty());
        let white = Image::filled(size, 1.0);
        assert_eq!(place(&plan(), Some(&white)).len(), 100);
        // The left half only.
        let pixels = (0..256)
            .map(|i| if i % 16 < 8 { 1.0 } else { 0.0 })
            .collect();
        let half = Image::from_pixels(size, pixels).unwrap();
        for c in place(&plan(), Some(&half)) {
            // Grey only near the middle, and over the wrapping edge.
            assert!(c.x < 0.55 || c.x > 0.96, "{}", c.x);
        }
    }

    #[test]
    fn too_many_big_copies_are_cut_down_and_nothing_breaks() {
        let huge = Plan {
            spacing: 1e-4,
            size: 1.0,
            size_vary: 0.0,
            ..plan()
        };
        assert_eq!(place(&huge, None).len(), MAX_DEPTH as usize);
        let tiny = Plan {
            spacing: 1e-4,
            size: 0.001,
            ..plan()
        };
        assert_eq!(place(&tiny, None).len(), MAX_COPIES);
        let broken = Plan {
            spacing: f32::NAN,
            size: f32::NAN,
            turn: f32::INFINITY,
            ..plan()
        };
        assert!(place(&broken, None).is_empty());
        let one = Plan {
            size: 0.5,
            spacing: f32::NAN,
            turn: f32::INFINITY,
            ..plan()
        };
        assert_eq!(place(&one, None).len(), 1);
    }

    #[test]
    fn the_lookup_finds_every_copy_covering_a_place_even_over_an_edge() {
        let copies = vec![Placed {
            x: 0.98,
            y: 0.02,
            size: 0.1,
            cos: 1.0,
            sin: 0.0,
            height: 1.0,
            tint: [1.0; 3],
            grey: 1.0,
            rank: 0.0,
        }];
        let lookup = Lookup::new(&copies);
        // Just past the left edge and the top: seen from there, the copy
        // sits a whole texture to the left and below.
        let near = lookup.near(0.01, 0.99);
        assert_eq!(near.len(), 1);
        assert!((near[0].x - (-0.02)).abs() < 1e-6, "{}", near[0].x);
        assert!((near[0].y - 1.02).abs() < 1e-6, "{}", near[0].y);
        assert!(lookup.near(0.5, 0.5).is_empty());
        // A copy bigger than the texture reaches every place, from more
        // than one side.
        let big = vec![Placed {
            size: 1.0,
            cos: std::f32::consts::FRAC_1_SQRT_2,
            sin: std::f32::consts::FRAC_1_SQRT_2,
            ..copies[0]
        }];
        let lookup = Lookup::new(&big);
        assert!(lookup.near(0.5, 0.5).len() >= 2);
        assert!(Lookup::new(&[]).near(0.3, 0.3).is_empty());
    }
}
