//! A whole shape cut into triangles: the Triangulate modifier's work, and
//! the lines *Show triangles* draws ([decision 0161](../../../../docs/decisions/0161-triangulate.md)).
//!
//! Both use the cut the export uses ([`super::face_triangles`]), so what
//! the viewport shows and what the modifier bakes are exactly what a glTF
//! file (or an OBJ with *Triangulate* ticked) gets. *Beauty* then evens
//! out flat polygons' triangles ([`super::beauty`]).
//!
//! The first triangle of each cut polygon keeps the polygon's id; the
//! others are new, in polygon order, so the ids are the same every time.
//! Points never move, so every edge's values (hard, seam, crease) stay;
//! each triangle's corners keep the polygon's corner values (UVs, custom
//! normals and the rest) and its polygon values (material). Shading
//! doesn't change on flat polygons.

use super::face_triangles;
use crate::builder::{Builder, CornerFrom};
use crate::{FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;
use words::tr;

/// How a polygon's triangles are chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriangulateMethod {
    /// The export's cut, then flat polygons' triangles evened out: fewer
    /// long thin slivers. The standard.
    #[default]
    Beauty,
    /// Exactly the cut the export uses, as *Show triangles* draws it.
    Fixed,
}

impl TriangulateMethod {
    /// Every method, in the order offered.
    pub const ALL: [Self; 2] = [Self::Beauty, Self::Fixed];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Beauty => tr("Beauty"),
            Self::Fixed => tr("Fixed"),
        }
    }

    /// What it does, for its tooltip.
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Beauty => tr(
                "Even out the triangles of flat polygons, so there are fewer long thin ones. \
                 Bent polygons keep the cut along their fold.",
            ),
            Self::Fixed => tr(
                "Cut exactly as the export does, the cut Display → Show triangles draws: \
                 nothing changes when you export.",
            ),
        }
    }
}

/// The Triangulate modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TriangulateOptions {
    /// How the triangles are chosen (Beauty, the standard, or Fixed).
    pub method: TriangulateMethod,
    /// Cut only four-sided polygons, leaving bigger ones as they are (off,
    /// the standard: every polygon with four or more corners).
    pub quads_only: bool,
}

impl TriangulateOptions {
    /// The settings within their limits: every value is already one, so
    /// they are as they are (kept for the modifier stack's recipe).
    #[must_use]
    pub fn clamped(self) -> Self {
        self
    }

    /// Whether a polygon with `corners` corners is cut.
    pub fn cuts(&self, corners: usize) -> bool {
        if self.quads_only {
            corners == 4
        } else {
            corners >= 4
        }
    }
}

/// Why a shape couldn't be triangulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriangulateProblem {
    /// Every polygon is a triangle already.
    AllTriangles,
    /// Quads only is on, and the shape has no quads (but bigger polygons).
    NoQuads,
    /// The shape has no polygons.
    Empty,
}

impl std::fmt::Display for TriangulateProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AllTriangles => tr("every polygon is a triangle already"),
            Self::NoQuads => tr(
                "the shape has no four-sided polygons to cut: switch Quads only off to cut its \
                 bigger polygons",
            ),
            Self::Empty => tr("the shape has no polygons"),
        })
    }
}

impl std::error::Error for TriangulateProblem {}

/// The triangles of polygon `corners` of `mesh`, as places in the
/// polygon, by `method`.
pub fn polygon_triangles(
    mesh: &Mesh,
    corners: &[VertexId],
    method: TriangulateMethod,
) -> Vec<[usize; 3]> {
    let mut out = Vec::with_capacity(corners.len().saturating_sub(2));
    face_triangles(mesh, corners, |t| out.push(t));
    if method == TriangulateMethod::Beauty && out.len() > 1 {
        let points: Vec<Vec3> = corners
            .iter()
            .map(|v| mesh.position(*v).unwrap_or(Vec3::ZERO))
            .collect();
        super::beauty::beautify(&points, &mut out);
    }
    out
}

/// How many corners the polygons `options` cuts have: the modifier's
/// work.
pub fn corners_to_cut(mesh: &Mesh, options: &TriangulateOptions) -> usize {
    mesh.faces()
        .map(|(_, c)| c.len())
        .filter(|n| options.cuts(*n))
        .sum()
}

/// `mesh` with its polygons cut into triangles as `options` say (see the
/// module notes), or why not: no polygons, all triangles already, or no
/// quads with Quads only on.
pub fn triangulated(mesh: &Mesh, options: &TriangulateOptions) -> Result<Mesh, TriangulateProblem> {
    let options = options.clamped();
    if mesh.face_count() == 0 {
        return Err(TriangulateProblem::Empty);
    }
    let cut: Vec<(FaceId, &[VertexId])> = mesh
        .faces()
        .filter(|(_, c)| options.cuts(c.len()))
        .collect();
    if cut.is_empty() {
        let bigger = mesh.faces().any(|(_, c)| c.len() > 3);
        return Err(if bigger {
            TriangulateProblem::NoQuads
        } else {
            TriangulateProblem::AllTriangles
        });
    }
    let mut out = mesh.clone();
    let mut builder = Builder::new(mesh, &mut out);
    let mut first = BTreeMap::new();
    let mut rest = Vec::new();
    for (face, corners) in cut {
        let triangles = polygon_triangles(mesh, corners, options.method);
        let corner = |i: usize| (corners[i], CornerFrom::Copy(face, corners[i]));
        let mut each = triangles.iter().map(|t| t.map(corner).to_vec());
        if let Some(triangle) = each.next() {
            first.insert(face, triangle);
        }
        rest.extend(each.map(|triangle| (face, triangle)));
    }
    // The polygons' own corners are always fine as a triangle's: nothing
    // here can be refused, but if it were, the shape is passed on as is.
    if builder.rewrite_faces(&first).is_err() {
        return Ok(mesh.clone());
    }
    for (face, triangle) in rest {
        builder.face(&triangle, Some(face));
    }
    Ok(out)
}

/// The lines inside polygons where the export's cut divides them into
/// triangles (not the polygons' own sides), in the mesh's space, each
/// once: what *Show triangles* draws.
pub fn inner_lines(mesh: &Mesh) -> Vec<[Vec3; 2]> {
    let mut out = Vec::new();
    let mut lines = Vec::new();
    for (_, corners) in mesh.faces() {
        let n = corners.len();
        if n < 4 {
            continue;
        }
        let side = |a: usize, b: usize| (a + 1) % n == b || (b + 1) % n == a;
        lines.clear();
        face_triangles(mesh, corners, |[a, b, c]| {
            for (x, y) in [(a, b), (b, c), (c, a)] {
                if !side(x, y) {
                    lines.push((x.min(y), x.max(y)));
                }
            }
        });
        // Each inside line is in two triangles: kept once.
        lines.sort_unstable();
        lines.dedup();
        for (x, y) in &lines {
            if let (Some(p), Some(q)) = (mesh.position(corners[*x]), mesh.position(corners[*y])) {
                out.push([p, q]);
            }
        }
    }
    out
}
