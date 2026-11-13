//! UV seams on a painted shape ([decision 0293](../../../docs/decisions/0293-blend-seams.md)):
//! the edges where the model is cut open on the texture, each with its two
//! sides, for the Blend seams tool to pick and the Seam blend layer to
//! blend.
//!
//! A seam is an edge two triangles share by its points but not by its
//! UVs: one place on the model, two places on the texture. Its sides are
//! told apart by their triangles, the UVs at its two ends and the piece
//! (UV island) each side belongs to. Seams are named by the object and
//! their two point ids, which stay put while the model is posed or moved
//! and while its UVs are moved, packed or unwrapped again.

use crate::image::Image;
use crate::surface::PaintSurface;
use glam::{Vec2, Vec3};
use std::collections::{HashMap, HashSet, VecDeque};

/// How far apart (on the texture, 0 to 1) the two sides' UVs at an end
/// may be and still count as one place: closer is not cut.
const SAME_UV: f32 = 1e-6;

/// A seam, by the object it is on and its two points' ids (the lower
/// first): how a Seam blend layer keeps the seams it blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SeamId {
    /// The object's id.
    pub object: u64,
    /// The edge's two points' ids, the lower first.
    pub points: [u32; 2],
}

/// One side of a seam: a triangle along it and where the edge lies on
/// the texture from that side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeamSide {
    /// The triangle (its number in the surface).
    pub triangle: usize,
    /// The UVs at the seam's two ends (in the order of [`Seam::points`]).
    pub uvs: [Vec2; 2],
    /// The UV of the triangle's third corner: which way its piece lies
    /// from the edge.
    pub inward: Vec2,
    /// Its UV piece (as [`crate::fill::pieces`] numbers them).
    pub piece: usize,
}

/// A seam edge on one shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seam {
    /// The edge's two points' ids, the lower first.
    pub points: [u32; 2],
    /// Where its ends are, in the shape's own space.
    pub ends: [Vec3; 2],
    /// Its two sides.
    pub sides: [SeamSide; 2],
}

impl Seam {
    /// Which way each side faces (its triangle's normal), in the shape's
    /// own space.
    pub fn facing(&self, surface: &PaintSurface) -> [Vec3; 2] {
        self.sides.map(|s| {
            surface
                .triangles()
                .get(s.triangle)
                .map_or(Vec3::ZERO, |t| t.normal)
        })
    }

    /// Its two pieces, the lower first: the pair a seam line runs between.
    fn pieces(&self) -> (usize, usize) {
        let [a, b] = self.sides.map(|s| s.piece);
        (a.min(b), a.max(b))
    }
}

/// A triangle along an edge: its number, and its corners at the edge's
/// lower and higher point and the third.
type Along = (usize, usize, usize, usize);

/// The seams of `surface`'s triangles in material `slots`, sorted by
/// their points. An edge shared by more than two triangles, or between a
/// triangle of the material and one of another, isn't a seam of its
/// texture.
pub fn seams_of(surface: &PaintSurface, slots: &[u32]) -> Vec<Seam> {
    let triangles = surface.triangles();
    let pieces = crate::fill::pieces(surface);
    // Each edge by its points: the triangles along it, with the corners
    // at its lower and higher point and the third.
    let mut edges: HashMap<[u32; 2], Vec<Along>> = HashMap::new();
    for (i, t) in triangles.iter().enumerate() {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            let (pa, pb) = (t.corners[a], t.corners[b]);
            if pa == pb {
                continue;
            }
            let third = 3 - a - b;
            let (lo, hi) = if pa < pb { (a, b) } else { (b, a) };
            edges
                .entry([pa.min(pb), pa.max(pb)])
                .or_default()
                .push((i, lo, hi, third));
        }
    }
    let mut out: Vec<Seam> = edges
        .into_iter()
        .filter_map(|(points, along)| {
            let [x, y] = along.as_slice() else {
                return None;
            };
            let side = |&(i, lo, hi, third): &Along| {
                let t = &triangles[i];
                SeamSide {
                    triangle: i,
                    uvs: [t.uvs[lo], t.uvs[hi]],
                    inward: t.uvs[third],
                    piece: pieces[i],
                }
            };
            let (a, b) = (side(x), side(y));
            let in_material =
                |s: &SeamSide| slots.contains(&triangles[s.triangle].slot) && s.uvs[0].is_finite();
            if !in_material(&a) || !in_material(&b) {
                return None;
            }
            let cut =
                a.uvs[0].distance(b.uvs[0]) > SAME_UV || a.uvs[1].distance(b.uvs[1]) > SAME_UV;
            if !cut {
                return None;
            }
            let t = &triangles[x.0];
            Some(Seam {
                points,
                ends: [t.points[x.1], t.points[x.2]],
                sides: [a, b],
            })
        })
        .collect();
    out.sort_by_key(|s| s.points);
    out
}

/// The seam with `points` in `seams` (sorted, as [`seams_of`] gives
/// them), if it is there.
pub fn find(seams: &[Seam], points: [u32; 2]) -> Option<usize> {
    seams.binary_search_by_key(&points, |s| s.points).ok()
}

/// The seam line `start` is on: every seam between the same two pieces
/// joined to it end to end, `start` first. Clicking one edge of a seam
/// picks the whole cut between two UV pieces, as a person sees it.
pub fn line_of(seams: &[Seam], start: usize) -> Vec<usize> {
    let Some(first) = seams.get(start) else {
        return Vec::new();
    };
    let pair = first.pieces();
    let mut by_point: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, s) in seams.iter().enumerate() {
        if s.pieces() == pair {
            for p in s.points {
                by_point.entry(p).or_default().push(i);
            }
        }
    }
    let mut seen = HashSet::from([start]);
    let mut out = vec![start];
    let mut queue = VecDeque::from([start]);
    while let Some(i) = queue.pop_front() {
        for p in seams[i].points {
            for &j in by_point.get(&p).into_iter().flatten() {
                if seen.insert(j) {
                    out.push(j);
                    queue.push_back(j);
                }
            }
        }
    }
    out
}

/// The seams that show on `image` (the painting they would be blended
/// from): those whose two sides are painted differently enough to see,
/// as "Why does this look wrong?" finds them (decision 0203).
pub fn showing(seams: &[Seam], image: &Image) -> Vec<usize> {
    seams
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let [a, b] = s.sides;
            crate::advice::painted_differently(image, a.uvs, a.inward, b.uvs, b.inward)
        })
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
pub(crate) mod tests;
