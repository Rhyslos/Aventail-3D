//! The print check (decision 0298): before an STL file is written, what
//! would trouble a slicer, shape by shape, in plain words with what to
//! do, and how big the print comes out.
//!
//! It reuses the app's own mesh checks: [`mesh::MeshReport`] (the Mesh
//! section of the Attributes panel: open borders, edges shared by more
//! than two polygons, polygons with no area) and
//! [`mesh::cleanup::faces_facing_wrong`] (what Fix normals turns). It
//! only describes: the export goes ahead whatever it finds.

use crate::export::ExportWhat;
use glam::{Mat4, Vec3};
use mesh::Mesh;
use scene::Document;
use words::trf;

/// A point further than this (in centimetres) from the plane of an open
/// shape makes it more than a flat sheet.
const FLAT: f32 = 1e-3;

/// What could trouble a slicer in one shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// A flat sheet (a plane, say): no thickness, nothing to print.
    NoThickness,
    /// Open edges: holes or cracks in the surface.
    Open {
        /// How many edges are on an open border.
        edges: usize,
    },
    /// Edges shared by more than two polygons.
    NonManifold {
        /// How many.
        edges: usize,
    },
    /// Polygons facing the wrong way (against their neighbours, or a
    /// closed shape facing inward).
    Flipped {
        /// How many.
        polygons: usize,
    },
    /// Polygons with no area, flattened to a line or a point.
    NoArea {
        /// How many.
        polygons: usize,
    },
}

/// One thing the print check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintIssue {
    /// The shape's name.
    pub shape: String,
    /// What.
    pub kind: IssueKind,
}

impl PrintIssue {
    /// What it is and what to do, in plain words.
    pub fn words(&self) -> String {
        let name = &self.shape;
        match self.kind {
            IssueKind::NoThickness => trf!(
                "{} is a flat sheet with no thickness, so there is nothing to print. Give it \
                 thickness with Shell (Mesh → Modifiers), or extrude it.",
                name
            ),
            IssueKind::Open { edges } => trf!(
                "{} isn't closed: its surface has {} open edges (holes or cracks), so the \
                 slicer can't tell inside from outside. Close holes with Fill hole (J); for \
                 cracks between points on the same spot, use Weld by distance.",
                name,
                edges
            ),
            IssueKind::NonManifold { edges } => trf!(
                "{} has {} edges shared by more than two polygons (non-manifold), which no \
                 real solid has. Pull the parts apart, or weld or delete the extra polygons.",
                name,
                edges
            ),
            IssueKind::Flipped { polygons } => trf!(
                "{} has {} polygons facing the wrong way, so the slicer may take them for \
                 holes. Select it and choose Mesh → Shading → Fix normals.",
                name,
                polygons
            ),
            IssueKind::NoArea { polygons } => trf!(
                "{} has {} polygons with no area (flattened to a line or a point), which can \
                 confuse the slicer. Weld by distance usually clears them.",
                name,
                polygons
            ),
        }
    }

    /// Whether it stops the shapes merging into one solid: a boolean
    /// needs closed solids.
    pub fn stops_merge(&self) -> bool {
        matches!(
            self.kind,
            IssueKind::NoThickness | IssueKind::Open { .. } | IssueKind::NonManifold { .. }
        )
    }
}

/// What the print check found for an export.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrintCheck {
    /// How many shapes go out.
    pub shapes: usize,
    /// How many triangles they make.
    pub triangles: usize,
    /// The smallest and largest corner of a box round them all, in
    /// centimetres, Z up; `None` with nothing to print.
    pub bounds: Option<(Vec3, Vec3)>,
    /// What could trouble a slicer, shape by shape.
    pub issues: Vec<PrintIssue>,
}

impl PrintCheck {
    /// The print's width, depth and height (X, Y, Z) in centimetres.
    pub fn size(&self) -> Option<Vec3> {
        self.bounds.map(|(min, max)| max - min)
    }

    /// Whether any issue stops the shapes merging into one solid.
    pub fn stops_merge(&self) -> bool {
        self.issues.iter().any(PrintIssue::stops_merge)
    }
}

/// The print check of the shapes `document` exports with `what`: the
/// same ones an STL export takes.
pub fn print_check(document: &Document, what: ExportWhat) -> PrintCheck {
    let scene = document.scene();
    let selection = document.selection();
    let cutters = scene.cutters();
    let mut out = PrintCheck::default();
    for (id, _) in scene.walk() {
        if !scene.is_shown(id) || cutters.contains(&id) || scene.is_level_shape(id) {
            continue;
        }
        if what == ExportWhat::Selected
            && !selection
                .iter()
                .any(|s| *s == id || scene.is_inside(id, *s))
        {
            continue;
        }
        let Some(shape) = scene.exported_shape(id) else {
            continue;
        };
        if shape.face_count() == 0 {
            continue;
        }
        let name = scene
            .object(id)
            .map(|o| o.name.clone())
            .unwrap_or_default();
        out.add(&name, &shape, scene.shape_matrix(id));
    }
    out
}

impl PrintCheck {
    /// Adds the shape `mesh`, called `name`, placed in the world by
    /// `matrix`.
    pub fn add(&mut self, name: &str, mesh: &Mesh, matrix: Mat4) {
        let report = mesh::MeshReport::new(mesh);
        self.shapes += 1;
        self.triangles += report.triangles;
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for (_, p) in mesh.vertices() {
            let p = matrix.transform_point3(p);
            if p.is_finite() {
                min = min.min(p);
                max = max.max(p);
            }
        }
        if min.cmple(max).all() {
            self.bounds = Some(match self.bounds {
                Some((a, b)) => (a.min(min), b.max(max)),
                None => (min, max),
            });
        }
        let issue = |kind| PrintIssue {
            shape: name.to_string(),
            kind,
        };
        if report.border_edges > 0 {
            self.issues.push(issue(if is_flat(mesh, matrix) {
                IssueKind::NoThickness
            } else {
                IssueKind::Open {
                    edges: report.border_edges,
                }
            }));
        }
        if report.non_manifold_edges > 0 {
            self.issues.push(issue(IssueKind::NonManifold {
                edges: report.non_manifold_edges,
            }));
        }
        let flipped = mesh::cleanup::faces_facing_wrong(mesh).len();
        if flipped > 0 {
            self.issues
                .push(issue(IssueKind::Flipped { polygons: flipped }));
        }
        if report.flat_faces > 0 {
            self.issues.push(issue(IssueKind::NoArea {
                polygons: report.flat_faces,
            }));
        }
    }
}

/// Whether every point of `mesh`, placed by `matrix`, lies in one plane:
/// a sheet with no thickness.
fn is_flat(mesh: &Mesh, matrix: Mat4) -> bool {
    let points: Vec<Vec3> = mesh
        .vertices()
        .map(|(_, p)| matrix.transform_point3(p))
        .collect();
    let Some(first) = points.first().copied() else {
        return true;
    };
    // The plane's normal: the largest cross product from the first point,
    // found among the points furthest along each way.
    let far = points
        .iter()
        .copied()
        .max_by(|a, b| a.distance_squared(first).total_cmp(&b.distance_squared(first)))
        .unwrap_or(first);
    let along = far - first;
    let normal = points
        .iter()
        .map(|p| along.cross(*p - first))
        .max_by(|a, b| a.length_squared().total_cmp(&b.length_squared()))
        .and_then(Vec3::try_normalize);
    let Some(normal) = normal else {
        // All on one line (or one spot).
        return true;
    };
    points
        .iter()
        .all(|p| (*p - first).dot(normal).abs() <= FLAT)
}
