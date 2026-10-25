//! The struts' and joints' UVs ([decision 0159](../../../../docs/decisions/0159-struts.md)).
//!
//! **Each piece is one island.** Its rings are laid flat as rows, bottom
//! to top, each row as long as its ring is round and centred, the rows as
//! far apart as the bands between them are high; so a strut's sides
//! unroll into an exact rectangle, and a joint's into a lens shape, a
//! little sheared towards its edges. Each end polygon is laid flat on its
//! own plane and hung below (or above) the first side of its row, sharing
//! that edge. Polygons keep their winding, so the UVs face the same way
//! as the surface (normal maps bake the right way round).
//!
//! **Packing:** every island at the same scale (so the texel density is
//! even), tallest first, in shelves filling a square, with a small gap
//! round each, then the lot scaled into the texture square.

use super::pieces::{Made, Part, Piece};
use crate::FaceId;
use glam::{Vec2, Vec3};

/// The gap round each island and the texture's border, as a share of the
/// texture's width.
const GAP: f32 = 0.004;

/// A piece laid flat: each polygon's corner UVs, in its corner order,
/// and the box round them all.
pub(super) struct Island {
    /// Each polygon's corners' UVs.
    pub(super) faces: Vec<(FaceId, Vec<Vec2>)>,
    /// The lowest corner of the box round the island.
    min: Vec2,
    /// The highest corner.
    max: Vec2,
}

/// Where an island goes: `uv * scale + shift`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Placed {
    scale: f32,
    shift: Vec2,
}

impl Placed {
    /// An island's UV placed in the texture square.
    pub(super) fn apply(self, uv: Vec2) -> Vec2 {
        uv * self.scale + self.shift
    }
}

/// The corners of `points` laid flat on their own plane, keeping their
/// winding, the side from `points[a]` to `points[b]` placed from `from`
/// along +U.
fn hang(points: &[Vec3], a: usize, b: usize, from: Vec2) -> Vec<Vec2> {
    let mut normal = Vec3::ZERO;
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        normal += p.cross(q);
    }
    let normal = normal.try_normalize().unwrap_or(Vec3::Z);
    let origin = points[a];
    let x = (points[b] - origin).try_normalize().unwrap_or(Vec3::X);
    let y = normal.cross(x);
    points
        .iter()
        .map(|p| {
            let d = *p - origin;
            from + Vec2::new(d.dot(x), d.dot(y))
        })
        .collect()
}

/// `piece` (as added, `made`) laid flat as one island.
pub(super) fn island(piece: &Piece, made: &Made) -> Island {
    let rings = &piece.rings;
    let n = rings.first().map_or(0, Vec::len);
    // Each row's spacing (a side's length) and height.
    let widths: Vec<f32> = rings
        .iter()
        .map(|ring| ring[0].distance(ring[1 % n]))
        .collect();
    let mut rows = vec![0.0f32];
    for k in 0..rings.len().saturating_sub(1) {
        let slant = rings[k][0].distance(rings[k + 1][0]);
        let shift = (widths[k + 1] - widths[k]) / 2.0;
        let high = (slant * slant - shift * shift).max(0.0).sqrt();
        rows.push(rows[k] + high);
    }
    let at = |k: usize, j: usize| Vec2::new((j as f32 - n as f32 / 2.0) * widths[k], rows[k]);
    let last = rings.len() - 1;
    let mut faces = Vec::with_capacity(made.faces.len());
    for (face, part) in &made.faces {
        let uvs = match *part {
            Part::Band(k, j) => vec![at(k, j), at(k, j + 1), at(k + 1, j + 1), at(k + 1, j)],
            Part::Bottom => {
                // Corners: ring 0 backwards, so point 1 then point 0 sit at
                // n - 2 and n - 1.
                let points: Vec<Vec3> = rings[0].iter().rev().copied().collect();
                hang(&points, n - 1, n - 2, at(0, 0))
            }
            Part::Top => hang(&rings[last], 0, 1, at(last, 0)),
        };
        faces.push((*face, uvs));
    }
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for uv in faces.iter().flat_map(|(_, uvs)| uvs) {
        min = min.min(*uv);
        max = max.max(*uv);
    }
    if !(min.is_finite() && max.is_finite()) {
        (min, max) = (Vec2::ZERO, Vec2::ZERO);
    }
    Island { faces, min, max }
}

/// Places each island in shelves `gap` apart: each one's lowest corner,
/// and the size of the whole.
fn shelves(sizes: &[Vec2], gap: f32) -> (Vec<Vec2>, Vec2) {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|a, b| sizes[*b].y.total_cmp(&sizes[*a].y).then(a.cmp(b)));
    let area: f32 = sizes.iter().map(|s| (s.x + gap) * (s.y + gap)).sum();
    let widest = sizes.iter().map(|s| s.x).fold(0.0f32, f32::max);
    let row_width = area.sqrt().max(widest + gap);
    let mut places = vec![Vec2::ZERO; sizes.len()];
    let (mut x, mut y, mut row_high, mut wide) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for i in order {
        let size = sizes[i];
        if x > 0.0 && x + size.x > row_width {
            y += row_high + gap;
            x = 0.0;
            row_high = 0.0;
        }
        places[i] = Vec2::new(x, y);
        x += size.x + gap;
        wide = wide.max(x - gap);
        row_high = row_high.max(size.y);
    }
    (places, Vec2::new(wide, y + row_high))
}

/// Where each island goes in the texture square (see the module's notes).
pub(super) fn pack(islands: &[Island]) -> Vec<Placed> {
    let sizes: Vec<Vec2> = islands.iter().map(|i| i.max - i.min).collect();
    // Once without gaps, to know how big a texture-width gap is.
    let (_, rough) = shelves(&sizes, 0.0);
    let gap = GAP * rough.max_element();
    let (places, whole) = shelves(&sizes, gap);
    let side = whole.max_element();
    let scale = if side > 1e-12 && side.is_finite() {
        (1.0 - 2.0 * GAP) / side
    } else {
        1.0
    };
    islands
        .iter()
        .zip(places)
        .map(|(island, place)| Placed {
            scale,
            shift: (place - island.min) * scale + Vec2::splat(GAP),
        })
        .collect()
}
