//! Quadify ([decision 0150](../../../docs/decisions/0150-quadify.md)):
//! joining pairs of neighbouring triangles into four-sided polygons
//! (quads), as a modifier and as a one-off recorded edit.
//!
//! A shape made of triangles (from a game, a scan, another program's
//! export, or Decimate) is hard to edit: loop cuts and edge loops stop at
//! every triangle, and smoothing makes lumps. Most of its triangles come
//! in pairs that were once a quad. Quadify finds those pairs and takes
//! away the edge between them:
//!
//! 1. **Candidates.** Every edge shared by exactly two triangles facing
//!    the same way round (and, for the edit, both selected) may go,
//!    unless it is kept: a UV seam or an edge where the UVs split
//!    ([`QuadifyOptions::keep_seams`]), a hard edge as the object's
//!    shading shows it ([`QuadifyOptions::keep_hard_edges`]), or an edge
//!    between two materials ([`QuadifyOptions::keep_materials`]). An edge
//!    with a crease or a bevel weight always stays, as it means something
//!    to a modifier.
//! 2. **Limits.** The two triangles may lean from each other by at most
//!    [`QuadifyOptions::max_angle`], and no corner of the quad they make
//!    may be further from a right angle than
//!    [`QuadifyOptions::max_distortion`]. A quad with a dent is never
//!    made.
//! 3. **Squarest first.** Each candidate is scored by how far its corners
//!    are from right angles, plus how far it is bent; the best go first,
//!    each triangle joining one pair at most, as Blender's *Tris to
//!    Quads* does (`pairing::best_pairs`, shared with Decimate's *Quads
//!    where flat*). Ties go by the triangles' ids, so the result is always
//!    the same.
//!
//! Joined pairs keep the lower id of the two (as Dissolve does,
//! [`crate::cleanup`]), and their corners' values (UVs and the rest)
//! come with them. Points never move and nothing else changes.

use crate::cleanup::merge;
use crate::edit::Outcome;
use crate::{EdgeKey, FaceId, Hardness, Mesh, VertexId};
use glam::DVec3;
use pairing::{Candidate, best_pairs, quad_shape};
use std::collections::{BTreeSet, HashSet};
use words::tr;

pub(crate) mod pairing;

/// The largest angle the two triangles may lean from each other, in
/// degrees: folded flat onto each other.
pub const MAX_ANGLE: f32 = 180.0;

/// The most a corner can be from a right angle in a quad with no dent, in
/// degrees.
pub const MAX_DISTORTION: f32 = 90.0;

/// Quadify's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadifyOptions {
    /// How far, in degrees, two triangles may lean from each other (the
    /// angle between their facings) and still be joined. Standard 40°.
    pub max_angle: f32,
    /// How far, in degrees, any corner of the quad may be from a right
    /// angle. Standard 40° (corners from 50° to 130°).
    pub max_distortion: f32,
    /// Never join across a UV seam or where the UVs split (on, the
    /// standard).
    pub keep_seams: bool,
    /// Never join across a hard edge, as the object's shading shows it
    /// (on, the standard).
    pub keep_hard_edges: bool,
    /// Never join triangles of different materials (on, the standard).
    pub keep_materials: bool,
}

impl Default for QuadifyOptions {
    fn default() -> Self {
        Self {
            max_angle: 40.0,
            max_distortion: 40.0,
            keep_seams: true,
            keep_hard_edges: true,
            keep_materials: true,
        }
    }
}

impl QuadifyOptions {
    /// The settings brought within their limits: Max angle 0 to 180°, Max
    /// shape distortion 0 to 90°. Values that aren't real numbers become
    /// the standard ones.
    #[must_use]
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        let real = |x: f32, or: f32| if x.is_finite() { x } else { or };
        Self {
            max_angle: real(self.max_angle, standard.max_angle).clamp(0.0, MAX_ANGLE),
            max_distortion: real(self.max_distortion, standard.max_distortion)
                .clamp(0.0, MAX_DISTORTION),
            ..self
        }
    }
}

/// Why Quadify joined nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadifyProblem {
    /// The shape has no triangles.
    NoTriangles,
    /// It has triangles, but no two neighbours may be joined.
    NothingJoins,
}

impl std::fmt::Display for QuadifyProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoTriangles => tr("the shape has no triangles to join"),
            Self::NothingJoins => tr(
                "no two neighbouring triangles can be joined: raise Max angle or Max shape \
                 distortion, or untick a Keep setting",
            ),
        })
    }
}

impl std::error::Error for QuadifyProblem {}

/// The pairs of triangles Quadify would join on `mesh`, in the order
/// chosen (squarest first), each as its two ids, lower first. With
/// `faces`, only pairs of two of those polygons. `hardness` is the
/// object's Smoothing, for [`QuadifyOptions::keep_hard_edges`].
pub fn pairs(
    mesh: &Mesh,
    faces: Option<&[FaceId]>,
    options: &QuadifyOptions,
    hardness: Hardness,
) -> Vec<[FaceId; 2]> {
    let options = options.clamped();
    let allowed: Option<HashSet<FaceId>> = faces.map(|f| f.iter().copied().collect());
    let kept = kept_edges(mesh, &options, hardness);
    let topology = mesh.topology();
    let mut candidates = Vec::new();
    for edge in topology.edges() {
        if kept.contains(&edge) {
            continue;
        }
        let Some(pair) = Pair::across(mesh, edge) else {
            continue;
        };
        if let Some(allowed) = &allowed
            && !(allowed.contains(&pair.t1) && allowed.contains(&pair.t2))
        {
            continue;
        }
        if options.keep_materials
            && crate::material::slot_of(mesh, pair.t1) != crate::material::slot_of(mesh, pair.t2)
        {
            continue;
        }
        if let Some(score) = pair.score(mesh, &options) {
            let (first, second) = if pair.t1 < pair.t2 {
                (pair.t1, pair.t2)
            } else {
                (pair.t2, pair.t1)
            };
            candidates.push(Candidate {
                score,
                first: first.get(),
                second: second.get(),
                item: [first, second],
            });
        }
    }
    best_pairs(candidates).into_iter().map(|c| c.item).collect()
}

/// Joins the pairs [`pairs`] chooses on `mesh`, in place: the polygons
/// `faces` only (missing ones counted), or every polygon with `None`.
/// Matched is how many pairs were joined; nothing joined is one skipped.
pub fn quadify(
    mesh: &mut Mesh,
    faces: Option<&[FaceId]>,
    options: &QuadifyOptions,
    hardness: Hardness,
) -> Outcome {
    let missing = faces.map_or(0, |f| f.iter().filter(|f| mesh.face(**f).is_none()).count());
    let chosen = pairs(mesh, faces, options, hardness);
    let groups: Vec<BTreeSet<FaceId>> =
        chosen.iter().map(|p| p.iter().copied().collect()).collect();
    let joined = if groups.is_empty() {
        0
    } else {
        merge(mesh, &groups)
    };
    Outcome {
        matched: joined,
        missing,
        skipped: usize::from(joined == 0),
    }
}

/// The Quadify modifier's work: `mesh` with its pairs of triangles joined,
/// or why none could be.
pub fn quadified(
    mesh: &Mesh,
    options: &QuadifyOptions,
    hardness: Hardness,
) -> Result<Mesh, QuadifyProblem> {
    if !mesh.faces().any(|(_, corners)| corners.len() == 3) {
        return Err(QuadifyProblem::NoTriangles);
    }
    let mut out = mesh.clone();
    if quadify(&mut out, None, options, hardness).matched == 0 {
        return Err(QuadifyProblem::NothingJoins);
    }
    Ok(out)
}

/// How many triangles `mesh` has: a measure of Quadify's work.
pub fn triangle_count(mesh: &Mesh) -> usize {
    mesh.faces().filter(|(_, c)| c.len() == 3).count()
}

/// The edges that are never taken away: those `options` keeps, and every
/// edge with a crease or a bevel weight.
fn kept_edges(mesh: &Mesh, options: &QuadifyOptions, hardness: Hardness) -> HashSet<EdgeKey> {
    let mut kept: HashSet<EdgeKey> = HashSet::new();
    kept.extend(crate::crease::creases(mesh).into_iter().map(|(e, _)| e));
    kept.extend(
        crate::bevel_weight::bevel_weights(mesh)
            .into_iter()
            .map(|(e, _)| e),
    );
    if options.keep_seams {
        kept.extend(crate::uvs::seams(mesh));
        kept.extend(crate::uvs::uv_splits(mesh));
    }
    if options.keep_hard_edges {
        // Custom normals' breaks aren't hard edges a person chose.
        if crate::custom_normals::has_custom_normals(mesh) {
            let mut plain = mesh.clone();
            crate::custom_normals::forget_custom_normals(&mut plain);
            kept.extend(crate::hardness::shading_hard_edges(&plain, hardness));
        } else {
            kept.extend(crate::hardness::shading_hard_edges(mesh, hardness));
        }
    }
    kept
}

/// Two triangles sharing an edge: `t1` runs `a → b → c`, `t2` runs
/// `b → a → d`, so the quad they make runs `a → d → b → c`.
struct Pair {
    t1: FaceId,
    t2: FaceId,
    quad: [VertexId; 4],
}

impl Pair {
    /// The two triangles across `edge`, if it has exactly two, both
    /// triangles, facing the same way round, with different far corners.
    fn across(mesh: &Mesh, edge: EdgeKey) -> Option<Self> {
        let [one, two] = mesh.topology().edge_uses(edge) else {
            return None;
        };
        if one.forward == two.forward || one.face == two.face {
            return None;
        }
        // The triangle running from the edge's lower point to its higher.
        let (t1, t2) = if one.forward {
            (one.face, two.face)
        } else {
            (two.face, one.face)
        };
        let (a, b) = edge.ends();
        let far = |face: FaceId| -> Option<VertexId> {
            let corners = mesh.face(face)?;
            if corners.len() != 3 {
                return None;
            }
            corners.iter().copied().find(|v| *v != a && *v != b)
        };
        let (c, d) = (far(t1)?, far(t2)?);
        if c == d {
            return None;
        }
        Some(Self {
            t1,
            t2,
            quad: [a, d, b, c],
        })
    }

    /// How good a quad the two make (lower is better: its corners' total
    /// distance from right angles plus its bend, in radians), or `None`
    /// when it is beyond the limits or has a dent.
    fn score(&self, mesh: &Mesh, options: &QuadifyOptions) -> Option<f64> {
        let mut p = [DVec3::ZERO; 4];
        for (slot, v) in p.iter_mut().zip(self.quad) {
            *slot = mesh.position(v)?.as_dvec3();
        }
        let shape = quad_shape(p)?;
        let limit = |degrees: f32| f64::from(degrees).to_radians() + 1e-9;
        if shape.fold > limit(options.max_angle) || shape.worst > limit(options.max_distortion) {
            return None;
        }
        Some(shape.squareness + shape.fold)
    }
}

#[cfg(test)]
mod tests;
