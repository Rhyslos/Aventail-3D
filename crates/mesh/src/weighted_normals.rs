//! Weighted normals ([decision 0148](../../../docs/decisions/0148-weighted-normals.md)):
//! shading normals where bigger polygons count for more, so a bevelled
//! hard-surface shape keeps its big flat faces looking flat while its
//! bevels shade round.
//!
//! The usual shading ([`crate::corner_shading_with`]) gives each corner
//! the average of the polygons round its point (within its smooth
//! group), each counted by the angle it fills there. On a bevelled box
//! the thin bevel strips count as much as the big face beside them, so
//! the big face's corners lean towards the bevel and it shades with a
//! gradient, as if dented. Weighting by **area** lets the big face win:
//! its corners point straight out of it, and the bevel's corners bend
//! to meet it, which is what a game engine's lighting and a baked normal
//! map want from a low-polygon shape.
//!
//! The result is the input with custom normals set on every corner
//! ([`crate::custom_normals`]); nothing else changes.

use crate::custom_normals::{forget_custom_normals, set_custom_normals};
use crate::{FaceId, Hardness, Mesh};
use glam::Vec3;

/// What makes a polygon count for more at a corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeightBy {
    /// Its area: big polygons win (the standard).
    #[default]
    Area,
    /// The angle it fills at the point, as the usual shading counts it.
    Angle,
    /// Both: its area times its angle.
    AreaAndAngle,
}

/// The settings of [`weighted_normals`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedNormalsOptions {
    /// What makes a polygon count for more.
    pub weight_by: WeightBy,
    /// How far the normals turn from the usual shading towards the
    /// weighted ones: 0 to 1 (1, the standard: all the way).
    pub strength: f32,
    /// How much the biggest polygon at a point wins over smaller ones: 0
    /// to 1. At 0 each counts by its weight alone; higher, smaller ones
    /// count ever less (their share of the biggest's weight raised to a
    /// power); at 1 only the biggest count. Standard 0.75.
    pub influence: f32,
    /// Whether the shading still breaks at the edges the object's shading
    /// shows hard (on, the standard); off, every point has one normal and
    /// the whole shape shades smooth.
    pub keep_hard_edges: bool,
}

impl Default for WeightedNormalsOptions {
    fn default() -> Self {
        Self {
            weight_by: WeightBy::Area,
            strength: 1.0,
            influence: 0.75,
            keep_hard_edges: true,
        }
    }
}

impl WeightedNormalsOptions {
    /// The settings brought within their limits; values that aren't real
    /// numbers become the standard ones.
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        let share = |x: f32, fallback: f32| {
            if x.is_finite() {
                x.clamp(0.0, 1.0)
            } else {
                fallback
            }
        };
        Self {
            strength: share(self.strength, standard.strength),
            influence: share(self.influence, standard.influence),
            ..self
        }
    }

    /// The power a polygon's share of the biggest weight at its point is
    /// raised to: 1 at no influence, growing to 64 (only the biggest
    /// count, in effect) at full.
    fn power(self) -> f32 {
        let influence = self.clamped().influence;
        if influence >= 1.0 {
            64.0
        } else {
            (1.0 / (1.0 - influence)).min(64.0)
        }
    }
}

/// `mesh` with weighted custom normals on every corner. The smooth groups
/// are those `hardness` gives (the object's Smoothing) when hard edges are
/// kept, else one per point. Any custom normals it had are replaced. A
/// polygon with no area counts for nothing; a corner whose polygons all
/// have none keeps its usual normal. Never fails.
pub fn weighted_normals(mesh: &Mesh, options: &WeightedNormalsOptions, hardness: Hardness) -> Mesh {
    let options = options.clamped();
    let mut out = mesh.clone();
    forget_custom_normals(&mut out);
    let (_, corners) = out.face_layout();
    if corners.is_empty() {
        return out;
    }
    let (_, usual) = crate::corner_shading_with(&out, hardness);
    let groups = if options.keep_hard_edges {
        crate::smooth_groups_with(&out, hardness)
    } else {
        one_group_per_point(&out)
    };
    let (weights, facing) = corner_weights(&out, options.weight_by);
    // Each group's biggest weight, at its first corner.
    let mut biggest = vec![0.0f32; corners.len()];
    for (k, w) in weights.iter().enumerate() {
        let g = groups[k] as usize;
        biggest[g] = biggest[g].max(*w);
    }
    let power = options.power();
    let mut sums = vec![Vec3::ZERO; corners.len()];
    for (k, w) in weights.iter().enumerate() {
        let g = groups[k] as usize;
        if *w > 0.0 && biggest[g] > 0.0 {
            sums[g] += facing[k] * (*w / biggest[g]).powf(power);
        }
    }
    let normals: Vec<Vec3> = (0..corners.len())
        .map(|k| {
            let weighted = sums[groups[k] as usize].try_normalize().unwrap_or(usual[k]);
            usual[k]
                .lerp(weighted, options.strength)
                .try_normalize()
                .unwrap_or(weighted)
        })
        .collect();
    set_custom_normals(&mut out, &normals);
    out
}

/// For each corner (in the mesh's corner order), the first corner at the
/// same point: every corner of a point in one group.
fn one_group_per_point(mesh: &Mesh) -> Vec<u32> {
    let (_, corners) = mesh.face_layout();
    let mut first = vec![u32::MAX; mesh.point_slot_count()];
    corners
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let slot = &mut first[v.get() as usize];
            if *slot == u32::MAX {
                *slot = k as u32;
            }
            *slot
        })
        .collect()
}

/// Each corner's weight and its polygon's facing (unit, or zero for a
/// polygon with no area), in the mesh's corner order.
fn corner_weights(mesh: &Mesh, weight_by: WeightBy) -> (Vec<f32>, Vec<Vec3>) {
    let (starts, corners) = mesh.face_layout();
    let mut weights = vec![0.0f32; corners.len()];
    let mut facing = vec![Vec3::ZERO; corners.len()];
    for (f, pair) in starts.windows(2).enumerate() {
        let (a, b) = (pair[0] as usize, pair[1] as usize);
        if mesh.face(FaceId::from_raw(f as u32)).is_none() || b <= a {
            continue;
        }
        let points: Vec<Vec3> = corners[a..b]
            .iter()
            .map(|v| mesh.position(*v).unwrap_or(Vec3::ZERO))
            .collect();
        let newell = newell(&points);
        let area = newell.length() / 2.0;
        let Some(normal) = newell
            .try_normalize()
            .filter(|_| area.is_finite() && area > 1e-12)
        else {
            continue;
        };
        let n = points.len();
        for i in 0..n {
            let here = points[i];
            let angle = (points[(i + n - 1) % n] - here).angle_between(points[(i + 1) % n] - here);
            let angle = if angle.is_finite() { angle } else { 0.0 };
            weights[a + i] = match weight_by {
                WeightBy::Area => area,
                WeightBy::Angle => angle,
                WeightBy::AreaAndAngle => area * angle,
            };
            facing[a + i] = normal;
        }
    }
    (weights, facing)
}

/// A polygon's Newell vector: its facing, twice its area long.
fn newell(points: &[Vec3]) -> Vec3 {
    let mut normal = Vec3::ZERO;
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        normal += Vec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    normal
}

#[cfg(test)]
mod tests;
