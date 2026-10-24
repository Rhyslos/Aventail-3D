//! Weld by distance ([decision 0152](../../../docs/decisions/0152-weld-by-distance.md)):
//! points closer together than a distance melt into one, as a modifier and
//! as a one-off recorded edit.
//!
//! Shapes often hold points on top of each other that should be one:
//! a model from another program or a game file split along every UV seam
//! and hard edge, pieces placed edge to edge, copies laid side by side
//! (the array modifier calls [`weld_by_distance`] to join its copies), or
//! points left almost on one spot by a bevel or a scale to zero. Each
//! such pair is a crack in the surface: it shades with a line, loop cuts
//! and selections stop at it, and smoothing pulls it open.
//!
//! How it works:
//!
//! 1. **Candidates.** Every point (or only the ones given), with
//!    [`WeldOptions::only_borders`] only those on an open border (an edge
//!    with a polygon on one side only): the points of a crack are always
//!    on one, so this leaves the rest of the shape alone.
//! 2. **A grid of cells** [`WeldOptions::distance`] wide, so each point
//!    is only compared with the points in its own and the 26 cells around
//!    it: the work grows with the number of points, not its square.
//! 3. **Lowest id first.** Taking the points in order of their ids, each
//!    one not yet taken keeps its place and takes every point not yet
//!    taken within the distance of it (as Blender's *Merge by Distance*
//!    does): points are never pulled along a chain further than the
//!    distance, and the result is always the same.
//! 4. **The weld** itself is Weld's ([`crate::join`]): polygons keep their
//!    ids and every corner keeps its own values (UVs and the rest);
//!    corners repeated next to each other go, polygons left with fewer
//!    than three corners or folded onto themselves go, and edges that
//!    went to the kept point keep their values (hard, seam, crease).
//! 5. **UV seams.** Joining two pieces whose UVs don't meet there makes
//!    an edge where the UVs split. With [`WeldOptions::keep_seams`] such
//!    edges are marked as UV seams, so a later Unwrap keeps the pieces as
//!    they were laid out; a shape with no seam marks at all is left
//!    unmarked, as Unwrap already cuts it where its UVs split.

use crate::attributes::SEAM;
use crate::join::{Group, weld_groups};
use crate::{EdgeKey, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeSet, HashMap};
use words::tr;

/// Weld by distance's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeldOptions {
    /// How close, in centimetres, two points must be to melt into one.
    /// Standard 0.01 cm (0.1 mm).
    pub distance: f32,
    /// Only points on open borders (edges with a polygon on one side
    /// only) are welded: cracks close, the rest stays (off, the standard).
    pub only_borders: bool,
    /// Where two pieces are joined and their UVs don't meet, the joining
    /// edge is marked as a UV seam (on, the standard).
    pub keep_seams: bool,
}

impl WeldOptions {
    /// The standard distance: 0.1 mm.
    pub const STANDARD_DISTANCE: f32 = 0.01;
    /// The largest distance: 1 m. Further than that, it isn't cleaning
    /// up any more.
    pub const MAX_DISTANCE: f32 = 100.0;

    /// The settings brought within their limits: Distance 0 to 1 m; one
    /// that isn't a real number becomes the standard one.
    #[must_use]
    pub fn clamped(self) -> Self {
        let distance = if self.distance.is_finite() {
            self.distance.clamp(0.0, Self::MAX_DISTANCE)
        } else {
            Self::STANDARD_DISTANCE
        };
        Self { distance, ..self }
    }
}

impl Default for WeldOptions {
    fn default() -> Self {
        Self {
            distance: Self::STANDARD_DISTANCE,
            only_borders: false,
            keep_seams: true,
        }
    }
}

/// Why Weld by distance welded nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeldProblem {
    /// No two points are within the distance of each other.
    NothingClose,
    /// Only open borders were asked for, and the shape has none.
    NoBorders,
}

impl std::fmt::Display for WeldProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NothingClose => tr(
                "no two points are closer together than the Distance: raise it to weld points \
                 further apart",
            ),
            Self::NoBorders => {
                tr("the shape has no open borders: untick Only at open borders to weld anywhere")
            }
        })
    }
}

impl std::error::Error for WeldProblem {}

/// The groups of points [`weld_by_distance`] would melt into one, each
/// in order of id, the point that stays first; groups in order of their
/// first point. With `points`, only those (the ones not in the mesh left
/// out); with `None`, every point.
pub fn weld_groups_of(
    mesh: &Mesh,
    points: Option<&[VertexId]>,
    options: &WeldOptions,
) -> Vec<Vec<VertexId>> {
    let options = options.clamped();
    let placed = candidates(mesh, points, options.only_borders);
    if placed.len() < 2 {
        return Vec::new();
    }
    let distance = options.distance;
    // Cells `distance` wide: points within the distance are in the same
    // or neighbouring cells. Far out, or with no distance, the cells only
    // need to be small enough to tell points apart.
    let cell = distance.max(1e-6);
    let key = |p: Vec3| {
        let k = (p / cell).floor();
        // `as` saturates: points beyond i64 share the end cells, which
        // the distance test still sorts out.
        (k.x as i64, k.y as i64, k.z as i64)
    };
    let mut cells: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
    for (i, (_, p)) in placed.iter().enumerate() {
        cells.entry(key(*p)).or_default().push(i);
    }
    let mut taken = vec![false; placed.len()];
    let mut groups = Vec::new();
    for (i, (v, p)) in placed.iter().enumerate() {
        if taken[i] {
            continue;
        }
        taken[i] = true;
        let mut group = vec![*v];
        let (x, y, z) = key(*p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(near) = cells.get_mut(&(
                        x.saturating_add(dx),
                        y.saturating_add(dy),
                        z.saturating_add(dz),
                    )) else {
                        continue;
                    };
                    // Points already taken leave their cell, so crowded
                    // cells empty as the work goes on.
                    near.retain(|j| !taken[*j]);
                    for &j in near.iter() {
                        if placed[j].1.distance(*p) <= distance {
                            taken[j] = true;
                            group.push(placed[j].0);
                        }
                    }
                }
            }
        }
        if group.len() > 1 {
            // The point that stays first, the rest in order of id.
            group[1..].sort_unstable();
            groups.push(group);
        }
    }
    groups
}

/// Welds the points of `mesh` within [`WeldOptions::distance`] of each
/// other into one, in place (see the module's notes): with `points`, only
/// those; with `None`, every point. Each group's lowest id stays where it
/// is and the others come to it. Returns how many points went (0 when
/// nothing was close, and then the mesh is unchanged).
///
/// The array modifier joins its copies with this: placed side by side,
/// copy after copy, then welded with [`WeldOptions::only_borders`] on,
/// so only the points where the copies touch melt.
pub fn weld_by_distance(
    mesh: &mut Mesh,
    points: Option<&[VertexId]>,
    options: &WeldOptions,
) -> usize {
    let options = options.clamped();
    let groups: Vec<Group> = weld_groups_of(mesh, points, &options)
        .into_iter()
        .filter_map(|points| {
            let keep = *points.first()?;
            Some(Group {
                place: mesh.position(keep)?,
                keep,
                points,
            })
        })
        .collect();
    if groups.is_empty() {
        return 0;
    }
    let had_seams = mesh.attribute_values(SEAM).is_some();
    if !weld_groups(mesh, &groups) {
        return 0;
    }
    if options.keep_seams && had_seams {
        let kept: BTreeSet<VertexId> = groups.iter().map(|g| g.keep).collect();
        let joined: Vec<EdgeKey> = crate::uvs::uv_splits(mesh)
            .into_iter()
            .filter(|e| {
                let (a, b) = e.ends();
                kept.contains(&a) || kept.contains(&b)
            })
            .collect();
        if !joined.is_empty() {
            crate::uvs::set_seams(mesh, &joined, true);
        }
    }
    groups.iter().map(|g| g.points.len() - 1).sum()
}

/// The Weld by distance modifier's work: `mesh` with its close points
/// welded, or why none were.
pub fn welded(mesh: &Mesh, options: &WeldOptions) -> Result<Mesh, WeldProblem> {
    let options = options.clamped();
    if options.only_borders && mesh.topology().border_edges().is_empty() {
        return Err(WeldProblem::NoBorders);
    }
    let mut out = mesh.clone();
    if weld_by_distance(&mut out, None, &options) == 0 {
        return Err(WeldProblem::NothingClose);
    }
    Ok(out)
}

/// The points that may be welded, in order of id, with their places:
/// `points` (or every point) that are in the mesh at a real place, and
/// with `only_borders`, on an open border.
fn candidates(
    mesh: &Mesh,
    points: Option<&[VertexId]>,
    only_borders: bool,
) -> Vec<(VertexId, Vec3)> {
    let chosen: BTreeSet<VertexId> = match points {
        Some(points) => points.iter().copied().collect(),
        None => mesh.vertices().map(|(v, _)| v).collect(),
    };
    let on_border: Option<BTreeSet<VertexId>> = only_borders.then(|| {
        mesh.topology()
            .border_edges()
            .into_iter()
            .flat_map(|e| {
                let (a, b) = e.ends();
                [a, b]
            })
            .collect()
    });
    chosen
        .into_iter()
        .filter(|v| on_border.as_ref().is_none_or(|b| b.contains(v)))
        .filter_map(|v| Some((v, mesh.position(v)?)))
        .filter(|(_, p)| p.is_finite())
        .collect()
}

#[cfg(test)]
mod tests;
