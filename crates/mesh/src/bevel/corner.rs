//! Rounded corners where three or more bevelled edges meet
//! ([decision 0141](../../../../docs/decisions/0141-improved-bevel.md)).
//!
//! Where bevelled edges meet at a point, their strips end round a hole.
//! When every side of that hole is a strip's end row (a cube's corner,
//! a pyramid's tip, a cylinder's rim where the side edges are bevelled
//! too), the hole is filled with a patch of quads that carries the strips'
//! curve on into the corner, like a quarter of a ball on a rounded cube.
//!
//! The patch is cut into one region per corner of the hole (each a grid
//! of quads), meeting in the middle:
//! - with an even number of segments, at one centre point;
//! - with an odd number, round a small centre polygon, with a strip of
//!   quads between each two regions (one segment wide, from the middle
//!   segment of each side in to the centre polygon).
//!
//! Each region is shaped as a Coons patch: a surface spanned between its
//! four borders, two halves of the hole's sides and two *spokes* from the
//! sides' middles to the centre. The spokes leave each side along its
//! strip (so the surface carries on smoothly from it) and bend over to
//! meet in the middle, more for a round profile, not at all for a flat
//! one, the other way for a hollow one.

use super::{Wedge, profile_point};
use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;

/// One side of the hole: a strip's end row.
#[derive(Debug, Clone)]
pub(super) struct Side {
    /// The row's points, from this side's first corner of the hole to the
    /// next (`segments + 1` of them).
    pub(super) points: Vec<VertexId>,
    /// The bevelled edge whose row it is.
    pub(super) edge: EdgeKey,
}

/// The sides of the hole at `v`, in order round it, if every side is a
/// strip's end row and there are at least three. `lists` are each
/// wedge's new corners, `rows` each strip's rows at each end.
pub(super) fn round_sides(
    v: VertexId,
    wedges: &[Wedge],
    lists: &[Vec<VertexId>],
    rows: &BTreeMap<(EdgeKey, VertexId), Vec<VertexId>>,
    bevelled: impl Fn(VertexId, VertexId) -> bool,
) -> Option<Vec<Side>> {
    #[derive(Clone, Copy, PartialEq)]
    enum Item {
        Point(VertexId),
        Row(EdgeKey),
    }
    let mut items = Vec::new();
    for (w, list) in wedges.iter().zip(lists) {
        items.extend(list.iter().map(|p| Item::Point(*p)));
        if bevelled(v, w.after) {
            items.push(Item::Row(EdgeKey::new(v, w.after)));
        }
    }
    items.dedup();
    while items.len() > 1 && items.first() == items.last() {
        items.pop();
    }
    // Start at a point, then points and rows must take turns.
    let start = items.iter().position(|i| matches!(i, Item::Point(_)))?;
    items.rotate_left(start);
    if items.len() < 6 || !items.len().is_multiple_of(2) {
        return None;
    }
    let mut corners = Vec::new();
    let mut edges = Vec::new();
    for pair in items.chunks(2) {
        match pair {
            [Item::Point(p), Item::Row(e)] => {
                corners.push(*p);
                edges.push(*e);
            }
            _ => return None,
        }
    }
    let mut seen = corners.clone();
    seen.sort();
    seen.dedup();
    if seen.len() != corners.len() {
        return None;
    }
    let n = corners.len();
    let mut sides = Vec::with_capacity(n);
    for j in 0..n {
        let (from, to) = (corners[j], corners[(j + 1) % n]);
        let row = rows.get(&(edges[j], v))?;
        let points = if row.first() == Some(&from) && row.last() == Some(&to) {
            row.clone()
        } else if row.first() == Some(&to) && row.last() == Some(&from) {
            row.iter().rev().copied().collect()
        } else {
            return None;
        };
        sides.push(Side {
            points,
            edge: edges[j],
        });
    }
    Some(sides)
}

/// A quadratic curve from `a` through control `c` to `b`, `t` of the way.
fn quadratic(a: Vec3, c: Vec3, b: Vec3, t: f32) -> Vec3 {
    let s = 1.0 - t;
    a * (s * s) + c * (2.0 * s * t) + b * (t * t)
}

/// The rounded patch's polygons at the bevelled point `v`: each polygon's
/// corners (with where their values come from) and the original polygon
/// it is like. Makes the patch's new points, and reshapes the inner
/// points of the sides' rows so each row's curve lies across its strip
/// (bending towards the bevelled edge beside it, not towards `v`).
#[allow(clippy::too_many_arguments)]
pub(super) fn round_patch(
    build: &mut Builder,
    original: &Mesh,
    v: VertexId,
    wedges: &[Wedge],
    lists: &[Vec<VertexId>],
    sides: &[Side],
    segments: u32,
    profile: f32,
) -> Vec<(Vec<(VertexId, CornerFrom)>, FaceId)> {
    let n = sides.len();
    let seg = segments as usize;
    let pv = original.position(v).unwrap_or_default();
    let at = |build: &mut Builder, p: VertexId| build.mesh().position(p).unwrap_or(pv);
    let corner_points: Vec<Vec3> = sides.iter().map(|s| at(build, s.points[0])).collect();

    // Each side's curve: from its corner to the next, bent towards the
    // bevelled edge where it crosses it (the point on the edge's line
    // level with the row's middle), as much as the profile says.
    let mut controls = Vec::with_capacity(n);
    let mut along = Vec::with_capacity(n);
    for (j, side) in sides.iter().enumerate() {
        let (a, b) = (corner_points[j], corner_points[(j + 1) % n]);
        let (lo, hi) = side.edge.ends();
        let far = if lo == v { hi } else { lo };
        let toward = (pv - original.position(far).unwrap_or(pv)).normalize_or_zero();
        let middle = (a + b) * 0.5;
        let on_edge = pv + toward * (middle - pv).dot(toward);
        let on_edge = if on_edge.is_finite() { on_edge } else { pv };
        // The control point profile_point uses.
        controls.push(middle + (on_edge - middle) * profile);
        along.push(toward);
        for k in 1..seg {
            let p = profile_point(a, b, on_edge, profile, k as f32 / seg as f32);
            if p.is_finite() {
                build.mesh().set_position(side.points[k], p);
            }
        }
    }
    let side_at =
        |j: usize, t: f32| quadratic(corner_points[j], controls[j], corner_points[(j + 1) % n], t);
    let middles: Vec<Vec3> = (0..n).map(|j| side_at(j, 0.5)).collect();

    // The centre: as a quadratic triangle's middle is a third its corners
    // and two thirds its sides' controls.
    let mean = |list: &[Vec3]| list.iter().copied().sum::<Vec3>() / list.len().max(1) as f32;
    let centre = mean(&corner_points) / 3.0 + mean(&controls) * (2.0 / 3.0);
    let centre = if centre.is_finite() { centre } else { pv };
    // Which way the corner faces.
    let mut axis = wedges
        .iter()
        .map(|w| original.face_normal(w.face))
        .sum::<Vec3>()
        .normalize_or_zero();
    if axis == Vec3::ZERO {
        axis = (pv - mean(&corner_points)).normalize_or_zero();
    }
    // Each spoke's control: leaving the side along its strip and arriving
    // level with the centre (round), straight (flat), or the other way
    // (hollow).
    let spokes: Vec<Vec3> = (0..n)
        .map(|j| {
            let m = middles[j];
            let straight = (m + centre) * 0.5;
            let rise = along[j].dot(axis);
            let reach = (centre - m).dot(axis) / rise;
            let round = if rise > 1e-3 && reach.is_finite() {
                m + along[j] * reach.clamp(0.0, 2.0 * m.distance(centre))
            } else {
                straight
            };
            straight + (round - straight) * profile
        })
        .collect();
    let spoke_at = |j: usize, s: f32| quadratic(middles[j], spokes[j], centre, s);
    // Region j's Coons patch: u along side j from its corner, w along the
    // side before it.
    let region_at = |j: usize, u: f32, w: f32| {
        let before = (j + n - 1) % n;
        let s0 = side_at(j, u * 0.5);
        let t0 = side_at(before, 1.0 - w * 0.5);
        let s1 = spoke_at(before, u);
        let t1 = spoke_at(j, w);
        let bilinear = corner_points[j] * ((1.0 - u) * (1.0 - w))
            + middles[j] * (u * (1.0 - w))
            + middles[before] * ((1.0 - u) * w)
            + centre * (u * w);
        (1.0 - w) * s0 + w * s1 + (1.0 - u) * t0 + u * t1 - bilinear
    };

    // The points, made in a fixed order: the centre, the spokes, then
    // each region's inside.
    let even = seg.is_multiple_of(2);
    let m = seg / 2;
    let param = |a: usize| 2.0 * a as f32 / seg as f32;
    let point = |build: &mut Builder, p: Vec3| {
        build.point(if p.is_finite() { p } else { pv }, PointFrom::Copy(v))
    };
    let centre_id = even.then(|| point(build, centre));
    let spoke_ids: Vec<Vec<VertexId>> = (0..n)
        .map(|j| {
            if !even {
                return Vec::new();
            }
            let mut ids = vec![sides[j].points[m]];
            for b in 1..m {
                ids.push(point(build, spoke_at(j, param(b))));
            }
            ids.extend(centre_id);
            ids
        })
        .collect();
    // grid[j][a][b]: region j's point a along side j and b along the side
    // before it.
    let mut grid: Vec<Vec<Vec<VertexId>>> = Vec::with_capacity(n);
    for j in 0..n {
        let before = (j + n - 1) % n;
        let mut region = vec![vec![v; m + 1]; m + 1];
        for (a, column) in region.iter_mut().enumerate() {
            for (b, slot) in column.iter_mut().enumerate() {
                *slot = if b == 0 {
                    sides[j].points[a]
                } else if a == 0 {
                    sides[before].points[seg - b]
                } else if even && a == m {
                    spoke_ids[j][b]
                } else if even && b == m {
                    spoke_ids[before][a]
                } else {
                    point(build, region_at(j, param(a), param(b)))
                };
            }
        }
        grid.push(region);
    }

    // The polygons. Each region is like the polygon its hole corner came
    // from (its material); every corner copies the first polygon's corner
    // at `v`. Without seams round `v` all those corners are the same in
    // the texture; with them, mixing pieces stretched the corner across
    // the texture (second review).
    let like: Vec<FaceId> = sides
        .iter()
        .map(|s| {
            let i = lists
                .iter()
                .position(|l| l.contains(&s.points[0]))
                .unwrap_or(0);
            wedges[i].face
        })
        .collect();
    let first = wedges.first().map(|w| w.face);
    let from = |j: usize| CornerFrom::Copy(first.unwrap_or(like[j]), v);
    // Quads are made going the hole's way round, then turned over to
    // face out, as the corner polygon was.
    let mut faces = Vec::new();
    let mut add = |corners: Vec<(VertexId, CornerFrom)>, face: FaceId| {
        let mut corners = corners;
        corners.reverse();
        corners.dedup_by_key(|c| c.0);
        while corners.len() > 1 && corners.first().map(|c| c.0) == corners.last().map(|c| c.0) {
            corners.pop();
        }
        if corners.len() >= 3 {
            faces.push((corners, face));
        }
    };
    for j in 0..n {
        let g = &grid[j];
        for a in 0..m {
            for b in 0..m {
                add(
                    vec![
                        (g[a][b], from(j)),
                        (g[a + 1][b], from(j)),
                        (g[a + 1][b + 1], from(j)),
                        (g[a][b + 1], from(j)),
                    ],
                    like[j],
                );
            }
        }
    }
    if !even {
        // The strips between regions, from each side's middle segment in.
        for j in 0..n {
            let next = (j + 1) % n;
            for k in 0..m {
                add(
                    vec![
                        (grid[j][m][k], from(j)),
                        (grid[next][k][m], from(next)),
                        (grid[next][k + 1][m], from(next)),
                        (grid[j][m][k + 1], from(j)),
                    ],
                    like[j],
                );
            }
        }
        add((0..n).map(|j| (grid[j][m][m], from(j))).collect(), like[0]);
    }
    faces
}
