//! The UV part of the bake check
//! ([decision 0105](../../../docs/decisions/0105-bake-readiness-for-uvs.md)):
//! what in a shape's UVs would spoil a bake, element by element, so each
//! problem can be selected or fixed.
//!
//! - [`UvReport`] looks at one shape: hard edges where the UVs don't split
//!   (the normal map can't hold the sharp turn there), polygons folded
//!   over or squashed flat in the texture, pieces sticking out of the
//!   texture square, and pieces whose texel density is far from the
//!   shape's usual one.
//! - [`check_sheet`] looks at the pieces that share one texture (from one
//!   object or several): pieces lying over each other, pieces stacked on
//!   purpose (identical ones sharing pixels), and pieces closer than the
//!   padding, which bleed into each other in the bake's padding and in the
//!   texture's smaller versions (mipmaps) a game shows at a distance. It
//!   draws the pieces into a grid of up to [`RESOLUTION`] cells across.

use glam::Vec2;
use mesh::uvs::Uvs;
use mesh::{EdgeKey, FaceId, Mesh};
use std::collections::{BTreeSet, VecDeque};

/// The most grid cells across the texture square that [`check_sheet`]
/// draws pieces into (a cell is one texture pixel or more).
pub const RESOLUTION: u32 = 1024;

/// How far a piece's density may be from the shape's usual one, as a
/// factor of length, before it counts as uneven.
pub const UNEVEN_DENSITY: f32 = 2.0;

/// How far past the texture square's edge a UV may lie and still count as
/// inside.
const EDGE: f32 = 1e-4;

/// What in one shape's UVs would spoil a bake.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UvReport {
    /// Hard edges inside a piece: the UVs don't split there.
    pub hard_joined: Vec<EdgeKey>,
    /// Polygons with no room in the texture, or mirrored against the rest
    /// of their piece (folded over).
    pub folded: Vec<FaceId>,
    /// Polygons of pieces that stick out of the texture square.
    pub outside: Vec<FaceId>,
    /// Polygons of pieces whose texel density is [`UNEVEN_DENSITY`] times
    /// or more off the shape's usual one.
    pub uneven: Vec<FaceId>,
    /// The shape's usual texel density (texture widths per unit of
    /// length): the area-weighted middle of its pieces'.
    pub usual_density: Option<f32>,
}

impl UvReport {
    /// Checks `mesh`'s UVs, with `hard` its hard edges (as the export
    /// will shade them); `None` if it has no UVs. Pieces lying on one of
    /// the trim sheet's `rows` (bottom, top) run past the square and have
    /// the row's density on purpose, so they aren't counted as outside or
    /// uneven.
    pub fn new(mesh: &Mesh, hard: &[EdgeKey], rows: &[(f32, f32)]) -> Option<Self> {
        let uvs = Uvs::of(mesh)?;
        let splits: BTreeSet<EdgeKey> = mesh::uvs::uv_splits(mesh).into_iter().collect();
        let topology = mesh.topology();
        let hard_joined = hard
            .iter()
            .copied()
            .filter(|e| topology.edge_uses(*e).len() == 2 && !splits.contains(e))
            .collect();
        let islands = mesh::uvs::islands(mesh);
        let on_trim = |island: &[FaceId]| {
            rows.iter()
                .any(|(bottom, top)| crate::trim::on_row(mesh, island, *bottom, *top))
        };
        let stretch: std::collections::HashMap<FaceId, crate::stretch::Stretch> =
            crate::stretch::stretch_of(mesh)
                .unwrap_or_default()
                .into_iter()
                .collect();
        let mut out = Self {
            hard_joined,
            ..Self::default()
        };
        // Folded: no room, or facing the other way from most of the piece.
        for island in &islands {
            let area = |f: &FaceId| crate::density::areas(mesh, &[*f], &uvs).1;
            let mirrored: f32 = island
                .iter()
                .filter(|f| stretch.get(f).is_some_and(|s| s.flipped))
                .map(area)
                .sum();
            let total: f32 = island.iter().map(area).sum();
            let mostly = mirrored > total / 2.0;
            for f in island {
                if let Some(s) = stretch.get(f)
                    && (s.empty || s.flipped != mostly)
                {
                    out.folded.push(*f);
                }
            }
        }
        // Outside the square.
        let (starts, _) = mesh.face_layout();
        for island in islands.iter().filter(|i| !on_trim(i)) {
            let out_of_square = island.iter().any(|f| {
                let start = starts[f.get() as usize] as usize;
                let n = mesh.face(*f).map_or(0, <[mesh::VertexId]>::len);
                (start..start + n).any(|i| {
                    let v = uvs.at(i);
                    v.x < -EDGE || v.y < -EDGE || v.x > 1.0 + EDGE || v.y > 1.0 + EDGE
                })
            });
            if out_of_square {
                out.outside.extend(island.iter().copied());
            }
        }
        // Uneven: each piece's density against the area-weighted middle.
        let mut densities: Vec<(f32, f32, usize)> = islands
            .iter()
            .enumerate()
            .filter(|(_, island)| !on_trim(island))
            .filter_map(|(i, island)| {
                let d = crate::density::density(mesh, island)?;
                let (_, real) = crate::density::areas(mesh, island, &uvs);
                (d > 0.0 && real > 0.0).then_some((d, real, i))
            })
            .collect();
        densities.sort_by(|a, b| a.0.total_cmp(&b.0));
        let total: f32 = densities.iter().map(|d| d.1).sum();
        let mut so_far = 0.0;
        let usual = densities.iter().find_map(|(d, real, _)| {
            so_far += real;
            (so_far >= total / 2.0).then_some(*d)
        });
        out.usual_density = usual;
        if let Some(usual) = usual {
            for (d, _, i) in &densities {
                if *d > usual * UNEVEN_DENSITY || d * UNEVEN_DENSITY < usual {
                    out.uneven.extend(islands[*i].iter().copied());
                }
            }
        }
        out.folded.sort();
        out.outside.sort();
        out.uneven.sort();
        Some(out)
    }

    /// Whether nothing was found.
    pub fn is_clean(&self) -> bool {
        self.hard_joined.is_empty()
            && self.folded.is_empty()
            && self.outside.is_empty()
            && self.uneven.is_empty()
    }
}

/// One piece on a texture: polygons of a shape that lie joined in it.
#[derive(Debug, Clone)]
pub struct SheetPiece<'a> {
    /// The shape.
    pub mesh: &'a Mesh,
    /// The piece's polygons.
    pub faces: Vec<FaceId>,
}

/// What lies wrong among the pieces of one texture: indices into the
/// pieces given to [`check_sheet`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SheetReport {
    /// Pieces lying over another piece that isn't the same as them.
    pub overlapping: BTreeSet<usize>,
    /// Pieces lying exactly on an identical piece (stacked to share
    /// pixels).
    pub stacked: BTreeSet<usize>,
    /// Pieces closer to another than the padding.
    pub too_close: BTreeSet<usize>,
}

impl SheetReport {
    /// Whether nothing was found.
    pub fn is_clean(&self) -> bool {
        self.overlapping.is_empty() && self.stacked.is_empty() && self.too_close.is_empty()
    }
}

/// A piece's triangles in the texture, and its box.
struct Drawn {
    triangles: Vec<[Vec2; 3]>,
    min: Vec2,
    max: Vec2,
    area: f32,
    corners: usize,
}

fn drawn(piece: &SheetPiece) -> Option<Drawn> {
    let uvs = Uvs::of(piece.mesh)?;
    let (starts, _) = piece.mesh.face_layout();
    let mut triangles = Vec::new();
    let mut corners = 0;
    for f in &piece.faces {
        let Some(list) = piece.mesh.face(*f) else {
            continue;
        };
        corners += list.len();
        let start = starts[f.get() as usize] as usize;
        mesh::triangulate::face_triangles(piece.mesh, list, |[a, b, c]| {
            triangles.push([uvs.at(start + a), uvs.at(start + b), uvs.at(start + c)]);
        });
    }
    let mut min = Vec2::INFINITY;
    let mut max = Vec2::NEG_INFINITY;
    let mut area = 0.0;
    for t in &triangles {
        for v in t {
            min = min.min(*v);
            max = max.max(*v);
        }
        area += ((t[1] - t[0]).perp_dot(t[2] - t[0]) / 2.0).abs();
    }
    (min.is_finite() && max.is_finite()).then_some(Drawn {
        triangles,
        min,
        max,
        area,
        corners,
    })
}

/// Whether two pieces are the same lying on one another (stacked): as
/// many corners, the same area and the same box, to within `cell`.
fn same_spot(a: &Drawn, b: &Drawn, cell: f32) -> bool {
    a.corners == b.corners
        && (a.area - b.area).abs() <= a.area.max(b.area) * 0.02 + cell * cell
        && a.min.distance(b.min) <= cell
        && a.max.distance(b.max) <= cell
}

/// The cell index at column `x`, row `y` of a grid `size` across.
fn index(size: usize, x: i64, y: i64) -> Option<usize> {
    let n = size as i64;
    (x >= 0 && y >= 0 && x < n && y < n).then(|| (y * n + x) as usize)
}

/// Calls `each` with every cell whose middle lies inside triangle `t`.
fn cells_inside(size: usize, t: &[Vec2; 3], mut each: impl FnMut(usize)) {
    let n = size as f32;
    let p: Vec<Vec2> = t.iter().map(|v| *v * n).collect();
    let twice = (p[1] - p[0]).perp_dot(p[2] - p[0]);
    if twice.abs() < 1e-9 {
        return;
    }
    let min = p[0].min(p[1]).min(p[2]);
    let max = p[0].max(p[1]).max(p[2]);
    let x0 = (min.x.floor() as i64).max(0);
    let y0 = (min.y.floor() as i64).max(0);
    let x1 = (max.x.ceil() as i64).min(size as i64 - 1);
    let y1 = (max.y.ceil() as i64).min(size as i64 - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let side = |a: Vec2, b: Vec2| (b - a).perp_dot(c - a) * twice.signum();
            if side(p[0], p[1]) >= 0.0
                && side(p[1], p[2]) >= 0.0
                && side(p[2], p[0]) >= 0.0
                && let Some(i) = index(size, x, y)
            {
                each(i);
            }
        }
    }
}

/// Calls `each` with every cell the line from `a` to `b` passes through.
fn cells_along(size: usize, a: Vec2, b: Vec2, mut each: impl FnMut(usize)) {
    let n = size as f32;
    let length = (b - a).length() * n;
    let steps = (length * 2.0).ceil().clamp(1.0, 4.0 * n) as usize;
    let mut last = None;
    for k in 0..=steps {
        let v = a + (b - a) * (k as f32 / steps as f32);
        let (x, y) = ((v.x * n).floor() as i64, (v.y * n).floor() as i64);
        if let Some(i) = index(size, x, y)
            && last != Some(i)
        {
            each(i);
            last = Some(i);
        }
    }
}

/// Checks the pieces sharing one texture of `size` pixels for overlaps
/// and for being closer than `padding` pixels to each other.
pub fn check_sheet(pieces: &[SheetPiece], size: u32, padding: u32) -> SheetReport {
    let mut out = SheetReport::default();
    let drawn: Vec<Option<Drawn>> = pieces.iter().map(drawn).collect();
    let cells = size.clamp(1, RESOLUTION) as usize;
    let cell = 1.0 / cells as f32;
    // Pixels per cell, and the gap wanted, in cells (one cell's slack, as
    // drawing can be a cell out either way).
    let per_cell = (size as f32 / cells as f32).max(1.0);
    let wanted = ((padding as f32 / per_cell).ceil() as usize).saturating_sub(1);
    // Overlaps: cells whose middles lie in two pieces.
    let mut inside = vec![0u32; cells * cells];
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (p, d) in drawn.iter().enumerate() {
        let Some(d) = d else {
            continue;
        };
        let mark = p as u32 + 1;
        for t in &d.triangles {
            cells_inside(cells, t, |i| {
                let was = inside[i];
                if was != 0 && was != mark {
                    let q = was as usize - 1;
                    pairs.insert((q.min(p), q.max(p)));
                } else {
                    inside[i] = mark;
                }
            });
        }
    }
    for (p, q) in &pairs {
        let (Some(a), Some(b)) = (&drawn[*p], &drawn[*q]) else {
            continue;
        };
        if same_spot(a, b, cell * 2.0) {
            out.stacked.insert(*p);
            out.stacked.insert(*q);
        } else {
            out.overlapping.insert(*p);
            out.overlapping.insert(*q);
        }
    }
    if wanted == 0 {
        return out;
    }
    // Too close: each piece's cells (its inside and its outline) spread
    // outwards together; where two pieces' spreads meet, the gap between
    // them is the sum of how far each spread.
    let mut owner = inside;
    let mut far = vec![u16::MAX; cells * cells];
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut close: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (p, d) in drawn.iter().enumerate() {
        let Some(d) = d else {
            continue;
        };
        let mark = p as u32 + 1;
        for t in &d.triangles {
            for k in 0..3 {
                cells_along(cells, t[k], t[(k + 1) % 3], |i| {
                    let was = owner[i];
                    if was == 0 {
                        owner[i] = mark;
                    } else if was != mark && !pairs.contains(&order(was as usize - 1, p)) {
                        close.insert(order(was as usize - 1, p));
                    }
                });
            }
        }
    }
    for (i, o) in owner.iter().enumerate() {
        if *o != 0 {
            far[i] = 0;
            queue.push_back(i);
        }
    }
    let reach = wanted.div_ceil(2) as u16;
    let n = cells as i64;
    while let Some(i) = queue.pop_front() {
        let d = far[i];
        let mine = owner[i];
        let (x, y) = ((i % cells) as i64, (i / cells) as i64);
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= n || ny >= n {
                continue;
            }
            let j = (ny * n + nx) as usize;
            let theirs = owner[j];
            if theirs == 0 {
                if d < reach {
                    owner[j] = mine;
                    far[j] = d + 1;
                    queue.push_back(j);
                }
            } else if theirs != mine {
                // The empty cells between them.
                let gap = usize::from(d) + usize::from(far[j]);
                let pair = order(mine as usize - 1, theirs as usize - 1);
                if gap < wanted && !pairs.contains(&pair) {
                    close.insert(pair);
                }
            }
        }
    }
    for (p, q) in close {
        out.too_close.insert(p);
        out.too_close.insert(q);
    }
    out
}

fn order(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

#[cfg(test)]
mod tests;
