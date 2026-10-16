//! Grooves and ridges: a thin line cut into a shape or raised out of it,
//! along edges, in one step ([decision 0086](../../../docs/decisions/0086-groove.md)).
//!
//! Each selected edge becomes a strip of new polygons [`GrooveShape::width`]
//! wide (a bevel, half the width on each side), and the strip is pushed in
//! by the depth (a groove, a panel line) or out (a ridge). Where a line of
//! edges ends inside a surface, the bevel's end polygons are part of the
//! strip, so the groove is closed off there, not left open.
//!
//! - **Straight sides** extrude the strip along its facing: walls square
//!   to the surface and a flat bottom, a crisp panel line.
//! - **Sloped sides** fold the strip along its middle instead: the middle
//!   goes in (or out) by the depth and the sides slope down to it, a V.
//!
//! Like every edit, it follows the edges by id, and the bevel and extrude
//! it is made of keep ids, values and hard edges as they do.

use crate::bevel::{BevelOptions, BevelShape, bevel_edges};
use crate::edit::Outcome;
use crate::extrude::{Grouping, extrude_polygons};
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// The widest a groove can be, in the mesh's units (1 m).
pub const MAX_GROOVE_WIDTH: f32 = 100.0;

/// How a groove's sides go down to its bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GrooveSides {
    /// Walls square to the surface and a flat bottom.
    #[default]
    Straight,
    /// Sloping down to a fold along the middle, a V.
    Sloped,
}

/// The shape of a groove (or ridge).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrooveShape {
    /// How wide the line is, in the mesh's units.
    pub width: f32,
    /// How far in (negative: a groove) or out (positive: a ridge), in the
    /// mesh's units.
    pub depth: f32,
    /// Straight or sloped sides.
    pub sides: GrooveSides,
}

impl Default for GrooveShape {
    fn default() -> Self {
        Self {
            width: 1.0,
            depth: -1.0,
            sides: GrooveSides::Straight,
        }
    }
}

impl GrooveShape {
    /// The shape in range: width 0 to [`MAX_GROOVE_WIDTH`]. `None` if a
    /// value isn't a number.
    pub fn clamped(self) -> Option<Self> {
        (self.width.is_finite() && self.depth.is_finite()).then_some(Self {
            width: self.width.clamp(0.0, MAX_GROOVE_WIDTH),
            ..self
        })
    }
}

/// Cuts a groove (or raises a ridge) along `edges` with `shape`. Edges
/// not in the mesh are counted as missing; edges without a polygon on each
/// side are skipped, as bevelling skips them. A width of 0, or a value
/// that isn't a number, changes nothing.
pub fn groove_edges(mesh: &mut Mesh, edges: &[EdgeKey], shape: GrooveShape) -> Outcome {
    let Some(shape) = shape.clamped() else {
        return Outcome::default();
    };
    let original = mesh.clone();
    // The strip: a flat bevel, half the width on each side. It never
    // crosses over and leaves no doubled points, so the walls built on
    // it are clean.
    let tidy = BevelOptions {
        clamp: true,
        weld: true,
        ..BevelOptions::EARLIER
    };
    let segments = match shape.sides {
        GrooveSides::Straight => 1,
        GrooveSides::Sloped => 2,
    };
    let bevel = BevelShape {
        width: shape.width * 0.5,
        segments,
        profile: 0.0,
        options: tidy,
    };
    if shape.width <= 0.0 {
        let mut nothing = original.clone();
        return bevel_edges(
            &mut nothing,
            edges,
            BevelShape {
                width: 0.0,
                ..bevel
            },
        );
    }
    let outcome = bevel_edges(mesh, edges, bevel);
    close_flat_ends(mesh, &original);
    let strip: Vec<FaceId> = mesh
        .faces()
        .map(|(f, _)| f)
        .filter(|f| original.face(*f).is_none())
        .collect();
    if strip.is_empty() || shape.depth == 0.0 {
        return outcome;
    }
    match shape.sides {
        GrooveSides::Straight => {
            extrude_polygons(mesh, &strip, shape.depth, Grouping::Together);
        }
        GrooveSides::Sloped => fold(mesh, &original, &strip, shape.depth),
    }
    outcome
}

/// Where a line of edges ends inside a surface, the bevel closes the
/// strip's end with a triangle from the old point to the two points that
/// slid away from it along a straight edge: a triangle with no area. The
/// old point joins the strip's end instead, and the triangle goes, so the
/// groove's end is a clean wall.
fn close_flat_ends(mesh: &mut Mesh, original: &Mesh) {
    let flat: Vec<(FaceId, [VertexId; 3])> = mesh
        .faces()
        .filter(|(f, _)| original.face(*f).is_none())
        .filter_map(|(f, c)| {
            let [a, b, d] = <[VertexId; 3]>::try_from(c).ok()?;
            let [pa, pb, pd] = [a, b, d].map(|v| mesh.position(v).unwrap_or_default());
            let area = (pb - pa).cross(pd - pa).length();
            let size = (pb - pa).length() + (pd - pb).length() + (pa - pd).length();
            (area <= 1e-4 * size * size).then_some((f, [a, b, d]))
        })
        .collect();
    if flat.is_empty() {
        return;
    }
    let mut changes: BTreeMap<FaceId, Vec<(VertexId, crate::builder::CornerFrom)>> =
        BTreeMap::new();
    let mut gone = Vec::new();
    for (triangle, corners) in &flat {
        // The old point, between the two that slid.
        let Some(k) = (0..3).find(|k| original.position(corners[*k]).is_some()) else {
            continue;
        };
        let old = corners[k];
        let (next, prev) = (corners[(k + 1) % 3], corners[(k + 2) % 3]);
        // The strip polygon on the other side of the triangle's edge from
        // `next` to `prev` runs it the other way, from `prev` to `next`.
        let Some((face, list)) = mesh
            .faces()
            .filter(|(f, _)| *f != *triangle && original.face(*f).is_none())
            .find_map(|(f, c)| {
                let n = c.len();
                (0..n)
                    .find(|i| c[*i] == prev && c[(i + 1) % n] == next)
                    .map(|i| (f, (c.to_vec(), i)))
            })
        else {
            continue;
        };
        if changes.contains_key(&face) {
            continue;
        }
        let (corners_of, at) = list;
        let mut new: Vec<(VertexId, crate::builder::CornerFrom)> = corners_of
            .iter()
            .map(|v| (*v, crate::builder::CornerFrom::Copy(face, *v)))
            .collect();
        new.insert(
            at + 1,
            (old, crate::builder::CornerFrom::Copy(*triangle, old)),
        );
        changes.insert(face, new);
        gone.push(*triangle);
    }
    if changes.is_empty() {
        return;
    }
    let before = mesh.clone();
    mesh.delete_faces(&gone);
    let mut build = crate::builder::Builder::new(&before, mesh);
    if build.rewrite_faces(&changes).is_err() {
        *mesh = before;
    }
}

/// Moves the strip's middle points (those only the strip uses) along their
/// facing by `depth`, folding it into a V.
fn fold(mesh: &mut Mesh, original: &Mesh, strip: &[FaceId], depth: f32) {
    let in_strip: BTreeSet<FaceId> = strip.iter().copied().collect();
    let mut facing: BTreeMap<VertexId, Vec3> = BTreeMap::new();
    let mut outside: BTreeSet<VertexId> = BTreeSet::new();
    for (face, corners) in mesh.faces() {
        let normal = mesh.face_normal(face);
        for v in corners {
            if in_strip.contains(&face) {
                *facing.entry(*v).or_default() += normal;
            } else {
                outside.insert(*v);
            }
        }
    }
    for (v, sum) in facing {
        // New points used by the strip alone: its fold.
        if outside.contains(&v) || original.position(v).is_some() {
            continue;
        }
        let Some(at) = mesh.position(v) else {
            continue;
        };
        let moved = at + sum.normalize_or_zero() * depth;
        if moved.is_finite() {
            mesh.set_position(v, moved);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MeshReport, PlaneParams, Primitive};

    fn cube() -> Mesh {
        Primitive::Cube(Default::default()).mesh()
    }

    /// The cube's edge along its top front, at y = -50, z = 100.
    fn top_front(mesh: &Mesh) -> EdgeKey {
        *mesh
            .edges()
            .iter()
            .find(|e| {
                let (a, b) = e.ends();
                let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
                pa.y < -49.0 && pb.y < -49.0 && pa.z > 99.0 && pb.z > 99.0
            })
            .unwrap()
    }

    fn clean(mesh: &Mesh) -> MeshReport {
        let r = MeshReport::new(mesh);
        assert!(!r.has_problems(), "{r:?}");
        for (v, p) in mesh.vertices() {
            assert!(p.is_finite(), "{v:?} {p}");
        }
        r
    }

    /// A flat grid, 4 × 4 squares of 25, facing up at z = 0.
    fn grid() -> Mesh {
        Primitive::Plane(PlaneParams {
            segments: [4, 4],
            ..Default::default()
        })
        .mesh()
    }

    /// The grid's edges along the line y = 0, between x = -25 and 25: a
    /// line that ends inside the surface at both ends.
    fn middle_line(mesh: &Mesh) -> Vec<EdgeKey> {
        mesh.edges()
            .iter()
            .filter(|e| {
                let (a, b) = e.ends();
                let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
                pa.y.abs() < 1e-3 && pb.y.abs() < 1e-3 && pa.x.abs() < 26.0 && pb.x.abs() < 26.0
            })
            .copied()
            .collect()
    }

    #[test]
    fn a_groove_across_a_grid_is_cut_in_with_closed_ends() {
        let before = grid();
        let line = middle_line(&before);
        assert_eq!(line.len(), 2);
        let mut mesh = before.clone();
        let shape = GrooveShape {
            width: 4.0,
            depth: -3.0,
            sides: GrooveSides::Straight,
        };
        let out = groove_edges(&mut mesh, &line, shape);
        assert_eq!(out.matched, 2);
        let r = clean(&mesh);
        // No new holes: the ends are closed, only the grid's own border is
        // open.
        assert_eq!(
            r.border_edges,
            MeshReport::new(&before).border_edges,
            "{r:?}"
        );
        // The bottom is 3 down, 4 wide.
        let bottom: Vec<Vec3> = mesh
            .vertices()
            .map(|(_, p)| p)
            .filter(|p| (p.z + 3.0).abs() < 1e-3)
            .collect();
        assert!(!bottom.is_empty());
        let across = bottom.iter().map(|p| p.y).fold(f32::MIN, f32::max)
            - bottom.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        assert!((across - 4.0).abs() < 1e-2, "{across}");
        // Nothing went below the bottom or above the surface.
        for (_, p) in mesh.vertices() {
            assert!(p.z <= 1e-3 && p.z >= -3.0 - 1e-3, "{p}");
        }
    }

    #[test]
    fn a_ridge_goes_out_and_sloped_sides_fold_to_a_v() {
        let before = grid();
        let line = middle_line(&before);
        let mut ridge = before.clone();
        groove_edges(
            &mut ridge,
            &line,
            GrooveShape {
                width: 4.0,
                depth: 2.0,
                sides: GrooveSides::Straight,
            },
        );
        clean(&ridge);
        let top = ridge.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
        assert!((top - 2.0).abs() < 1e-3, "{top}");
        // Sloped: only the middle line goes down; the strip's edges stay on
        // the surface.
        let mut v = before.clone();
        groove_edges(
            &mut v,
            &line,
            GrooveShape {
                width: 4.0,
                depth: -3.0,
                sides: GrooveSides::Sloped,
            },
        );
        let r = clean(&v);
        assert_eq!(r.border_edges, MeshReport::new(&before).border_edges);
        let down: Vec<Vec3> = v
            .vertices()
            .map(|(_, p)| p)
            .filter(|p| p.z < -1e-3)
            .collect();
        assert!(!down.is_empty());
        for p in &down {
            assert!((p.z + 3.0).abs() < 1e-3, "{p}");
            assert!(p.y.abs() < 1e-3, "on the middle line: {p}");
        }
    }

    #[test]
    fn a_groove_round_a_cube_edge_and_round_its_top_stays_closed() {
        let mut one = cube();
        let edge = top_front(&one);
        groove_edges(&mut one, &[edge], GrooveShape::default());
        let r = clean(&one);
        assert!(r.is_closed(), "{r:?}");
        let ring = cube();
        let top: Vec<EdgeKey> = ring
            .edges()
            .iter()
            .filter(|e| {
                let (a, b) = e.ends();
                ring.position(a).unwrap().z > 99.0 && ring.position(b).unwrap().z > 99.0
            })
            .copied()
            .collect();
        for sides in [GrooveSides::Straight, GrooveSides::Sloped] {
            let mut m = ring.clone();
            groove_edges(
                &mut m,
                &top,
                GrooveShape {
                    width: 3.0,
                    depth: -2.0,
                    sides,
                },
            );
            let r = clean(&m);
            assert!(r.is_closed(), "{sides:?} {r:?}");
            assert_eq!(r.pieces, 1);
        }
    }

    #[test]
    fn nothing_missing_zero_and_broken_change_nothing() {
        let before = grid();
        let line = middle_line(&before);
        for shape in [
            GrooveShape {
                width: 0.0,
                ..GrooveShape::default()
            },
            GrooveShape {
                width: f32::NAN,
                ..GrooveShape::default()
            },
            GrooveShape {
                depth: f32::INFINITY,
                ..GrooveShape::default()
            },
        ] {
            let mut mesh = before.clone();
            groove_edges(&mut mesh, &line, shape);
            assert_eq!(mesh, before, "{shape:?}");
        }
        let mut mesh = before.clone();
        let out = groove_edges(
            &mut mesh,
            &[EdgeKey::new(
                VertexId::from_raw(900),
                VertexId::from_raw(901),
            )],
            GrooveShape::default(),
        );
        assert_eq!(out.missing, 1);
        assert_eq!(mesh, before);
        // Border edges are skipped.
        let border = *before
            .edges()
            .iter()
            .find(|e| {
                let (a, b) = e.ends();
                before.position(a).unwrap().y < -49.0 && before.position(b).unwrap().y < -49.0
            })
            .unwrap();
        let mut mesh = before.clone();
        let out = groove_edges(&mut mesh, &[border], GrooveShape::default());
        assert_eq!(out.skipped, 1);
        assert_eq!(mesh, before);
        // Width beyond the limit is brought in.
        assert_eq!(
            GrooveShape {
                width: 1e9,
                ..GrooveShape::default()
            }
            .clamped()
            .unwrap()
            .width,
            MAX_GROOVE_WIDTH
        );
    }

    #[test]
    fn a_rim_next_to_an_n_gon_cap_and_two_crossing_lines_stay_clean() {
        // The cylinder's top rim: every edge borders the many-sided cap.
        let mesh = Primitive::Cylinder(crate::CylinderParams::default()).mesh();
        let rim: Vec<EdgeKey> = mesh
            .edges()
            .iter()
            .filter(|e| {
                let (a, b) = e.ends();
                mesh.position(a).unwrap().z > 99.0 && mesh.position(b).unwrap().z > 99.0
            })
            .copied()
            .collect();
        for sides in [GrooveSides::Straight, GrooveSides::Sloped] {
            let mut m = mesh.clone();
            let out = groove_edges(
                &mut m,
                &rim,
                GrooveShape {
                    width: 2.0,
                    depth: -1.5,
                    sides,
                },
            );
            assert_eq!(out.matched, rim.len());
            let r = clean(&m);
            assert!(r.is_closed(), "{sides:?} {r:?}");
        }
        // Two lines crossing in the middle of a grid: one groove, one shared
        // corner, no holes.
        let before = grid();
        let cross: Vec<EdgeKey> = before
            .edges()
            .iter()
            .filter(|e| {
                let (a, b) = e.ends();
                let (pa, pb) = (before.position(a).unwrap(), before.position(b).unwrap());
                let on_x = pa.y.abs() < 1e-3 && pb.y.abs() < 1e-3;
                let on_y = pa.x.abs() < 1e-3 && pb.x.abs() < 1e-3;
                (on_x && pa.x.abs() < 26.0 && pb.x.abs() < 26.0)
                    || (on_y && pa.y.abs() < 26.0 && pb.y.abs() < 26.0)
            })
            .copied()
            .collect();
        assert_eq!(cross.len(), 4);
        let mut m = before.clone();
        groove_edges(&mut m, &cross, GrooveShape::default());
        let r = clean(&m);
        assert_eq!(r.border_edges, MeshReport::new(&before).border_edges);
    }

    #[test]
    fn non_manifold_edges_are_skipped() {
        let mut mesh = Mesh::new();
        let v: Vec<VertexId> = [
            (0.0, 0.0, 0.0),
            (10.0, 0.0, 0.0),
            (5.0, 10.0, 0.0),
            (5.0, -10.0, 0.0),
            (5.0, 0.0, 10.0),
        ]
        .into_iter()
        .map(|(x, y, z)| mesh.add_vertex(Vec3::new(x, y, z)).unwrap())
        .collect();
        mesh.add_face(&[v[0], v[1], v[2]]).unwrap();
        mesh.add_face(&[v[1], v[0], v[3]]).unwrap();
        mesh.add_face(&[v[0], v[1], v[4]]).unwrap();
        let before = mesh.clone();
        let out = groove_edges(
            &mut mesh,
            &[EdgeKey::new(v[0], v[1])],
            GrooveShape::default(),
        );
        assert_eq!(out.skipped, 1);
        assert_eq!(mesh, before);
    }

    #[test]
    fn the_same_groove_gives_the_same_ids_and_polygons_keep_theirs() {
        let before = grid();
        let line = middle_line(&before);
        let mut a = before.clone();
        let mut b = before.clone();
        groove_edges(&mut a, &line, GrooveShape::default());
        groove_edges(&mut b, &line, GrooveShape::default());
        assert_eq!(a, b);
        for (f, _) in before.faces() {
            assert!(a.face(f).is_some(), "{f:?} kept");
        }
    }
}
