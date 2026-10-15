//! The model part of the bake check ([decision 0077](../../../docs/decisions/0077-bake-check.md)):
//! what in a shape would spoil a bake in Substance Painter, element by
//! element, so each problem can be selected or fixed.
//!
//! [`crate::MeshReport`] counts problems for the Attributes panel; this
//! lists them, adding two that matter for baking: many-cornered polygons
//! (n-gons) that are bent out of flat, or that cut into sliver triangles.
//! Both shade unevenly, and a baked normal map shows it.

use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;

/// Two points closer than this are on the same spot (the same as the mesh
/// check's).
const SAME_SPOT: f32 = 1e-4;

/// A polygon with less area than this has none.
const NO_AREA: f32 = 1e-8;

/// An n-gon is bent when a corner is further out of its plane than this
/// share of its size.
pub const BENT_SHARE: f32 = 0.05;

/// A triangle is a sliver when its sharpest corner is under this many
/// degrees.
pub const SLIVER_DEGREES: f32 = 2.0;

/// What in a shape would spoil a bake.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BakeReport {
    /// Polygons facing the wrong way round.
    pub flipped: Vec<FaceId>,
    /// Edges shared by three or more polygons.
    pub non_manifold: Vec<EdgeKey>,
    /// Polygons with no area.
    pub flat: Vec<FaceId>,
    /// Edges with no length.
    pub short_edges: Vec<EdgeKey>,
    /// Polygons of five or more corners bent out of flat.
    pub bent: Vec<FaceId>,
    /// Polygons of five or more corners that cut into sliver triangles.
    pub slivers: Vec<FaceId>,
    /// Points no polygon uses.
    pub loose: Vec<VertexId>,
}

impl BakeReport {
    /// Checks `mesh`.
    pub fn new(mesh: &Mesh) -> Self {
        let topology = mesh.topology();
        let short_edges = topology
            .edges()
            .filter(|e| {
                let (a, b) = e.ends();
                match (mesh.position(a), mesh.position(b)) {
                    (Some(p), Some(q)) => p.distance(q) < SAME_SPOT,
                    _ => false,
                }
            })
            .collect();
        let mut out = Self {
            flipped: crate::cleanup::faces_facing_wrong(mesh),
            non_manifold: topology.non_manifold_edges(),
            short_edges,
            loose: mesh
                .vertices()
                .map(|(v, _)| v)
                .filter(|v| topology.vertex_faces(*v).is_empty())
                .collect(),
            ..Self::default()
        };
        for (face, corners) in mesh.faces() {
            let points: Vec<Vec3> = corners.iter().filter_map(|v| mesh.position(*v)).collect();
            if points.len() != corners.len() {
                continue;
            }
            let normal = newell(&points);
            if normal.length() / 2.0 < NO_AREA {
                out.flat.push(face);
                continue;
            }
            if corners.len() < 5 {
                continue;
            }
            if is_bent(&points, normal.normalize()) {
                out.bent.push(face);
            }
            if has_sliver(mesh, corners, &points) {
                out.slivers.push(face);
            }
        }
        out
    }

    /// Whether nothing was found.
    pub fn is_clean(&self) -> bool {
        self == &Self::default()
    }

    /// The short edges' points in groups that touch, each group to be
    /// welded into one point.
    pub fn short_edge_groups(&self) -> Vec<Vec<VertexId>> {
        let mut group: BTreeMap<VertexId, usize> = BTreeMap::new();
        let mut groups: Vec<Vec<VertexId>> = Vec::new();
        for edge in &self.short_edges {
            let (a, b) = edge.ends();
            match (group.get(&a).copied(), group.get(&b).copied()) {
                (Some(x), Some(y)) if x != y => {
                    let moved = std::mem::take(&mut groups[y]);
                    for v in &moved {
                        group.insert(*v, x);
                    }
                    groups[x].extend(moved);
                }
                (Some(_), Some(_)) => {}
                (Some(x), None) => {
                    group.insert(b, x);
                    groups[x].push(b);
                }
                (None, Some(y)) => {
                    group.insert(a, y);
                    groups[y].push(a);
                }
                (None, None) => {
                    group.insert(a, groups.len());
                    group.insert(b, groups.len());
                    groups.push(vec![a, b]);
                }
            }
        }
        groups.retain(|g| g.len() > 1);
        for g in &mut groups {
            g.sort_unstable();
        }
        groups
    }
}

/// A polygon's Newell normal: its direction, and twice its area as long.
fn newell(points: &[Vec3]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for (i, p) in points.iter().enumerate() {
        sum += p.cross(points[(i + 1) % points.len()]);
    }
    sum
}

/// Whether a corner of the polygon through `points` (facing `normal`) is
/// further out of its plane than [`BENT_SHARE`] of its size.
fn is_bent(points: &[Vec3], normal: Vec3) -> bool {
    let middle = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    let size = points
        .iter()
        .map(|p| p.distance(middle))
        .fold(0.0f32, f32::max)
        * 2.0;
    if size <= 0.0 {
        return false;
    }
    points
        .iter()
        .any(|p| (*p - middle).dot(normal).abs() > BENT_SHARE * size)
}

/// Whether the viewport's cut of the polygon makes a triangle whose
/// sharpest corner is under [`SLIVER_DEGREES`].
fn has_sliver(mesh: &Mesh, corners: &[VertexId], points: &[Vec3]) -> bool {
    let limit = SLIVER_DEGREES.to_radians();
    let mut found = false;
    crate::face_triangles(mesh, corners, |[i, j, k]| {
        let (a, b, c) = (points[i], points[j], points[k]);
        let corner = |p: Vec3, q: Vec3, r: Vec3| (q - p).angle_between(r - p);
        let sharpest = corner(a, b, c).min(corner(b, c, a)).min(corner(c, a, b));
        if sharpest.is_finite() && sharpest < limit {
            found = true;
        }
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrimitiveKind;

    fn build(points: &[[f32; 3]], faces: &[&[u32]]) -> Mesh {
        let mut mesh = Mesh::new();
        for p in points {
            mesh.add_vertex(Vec3::from_array(*p)).unwrap();
        }
        for f in faces {
            let c: Vec<VertexId> = f.iter().map(|n| VertexId::from_raw(*n)).collect();
            mesh.add_face(&c).unwrap();
        }
        mesh
    }

    #[test]
    fn every_primitive_is_ready_to_bake() {
        for kind in PrimitiveKind::ALL {
            let report = BakeReport::new(&kind.standard().mesh());
            assert!(report.is_clean(), "{kind:?}: {report:?}");
        }
    }

    #[test]
    fn a_flipped_polygon_and_a_loose_point_are_found() {
        let mut cube = PrimitiveKind::Cube.standard().mesh();
        let top = cube.faces().next().unwrap().0;
        crate::cleanup::flip_faces(&mut cube, &[top]);
        cube.add_vertex(Vec3::splat(500.0)).unwrap();
        let report = BakeReport::new(&cube);
        assert_eq!(report.flipped.len(), 1);
        assert_eq!(report.loose.len(), 1);
        assert!(report.bent.is_empty() && report.slivers.is_empty());
    }

    #[test]
    fn bent_and_sliver_ngons_are_found_but_quads_are_left_alone() {
        // A flat hexagon is fine; lift one corner and it is bent.
        let ring: Vec<[f32; 3]> = (0..6)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                [a.cos() * 100.0, a.sin() * 100.0, 0.0]
            })
            .collect();
        let flat = build(&ring, &[&[0, 1, 2, 3, 4, 5]]);
        assert!(BakeReport::new(&flat).is_clean());
        let mut lifted = ring.clone();
        lifted[2][2] = 30.0;
        let bent = build(&lifted, &[&[0, 1, 2, 3, 4, 5]]);
        assert_eq!(BakeReport::new(&bent).bent.len(), 1);
        // A long thin five-sided strip cuts into slivers.
        let strip = build(
            &[
                [0.0, 0.0, 0.0],
                [500.0, 0.0, 0.0],
                [1000.0, 0.0, 0.0],
                [1000.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            &[&[0, 1, 2, 3, 4]],
        );
        assert_eq!(BakeReport::new(&strip).slivers.len(), 1);
        // The same as a thin quad: fine (a bevel strip, say).
        let quad = build(
            &[
                [0.0, 0.0, 0.0],
                [1000.0, 0.0, 0.0],
                [1000.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            &[&[0, 1, 2, 3]],
        );
        assert!(BakeReport::new(&quad).is_clean());
    }

    #[test]
    fn flat_polygons_and_short_edges_are_found_and_grouped() {
        // Two triangles side by side, one squashed onto a line, and a
        // point doubled on the spot.
        let mesh = build(
            &[
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [0.0, 10.0, 0.0],
                [5.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
            ],
            &[&[0, 1, 2], &[0, 3, 1], &[1, 4, 2]],
        );
        let report = BakeReport::new(&mesh);
        assert_eq!(report.flat.len(), 2, "{report:?}");
        assert_eq!(report.short_edges.len(), 1);
        assert_eq!(
            report.short_edge_groups(),
            vec![vec![VertexId::from_raw(1), VertexId::from_raw(4)]]
        );
    }

    #[test]
    fn touching_short_edges_make_one_group() {
        let report = BakeReport {
            short_edges: vec![
                EdgeKey::new(VertexId::from_raw(1), VertexId::from_raw(2)),
                EdgeKey::new(VertexId::from_raw(5), VertexId::from_raw(6)),
                EdgeKey::new(VertexId::from_raw(2), VertexId::from_raw(3)),
                EdgeKey::new(VertexId::from_raw(3), VertexId::from_raw(5)),
            ],
            ..BakeReport::default()
        };
        let groups = report.short_edge_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 5);
    }

    #[test]
    fn an_empty_mesh_is_clean() {
        assert!(BakeReport::new(&Mesh::new()).is_clean());
    }
}
