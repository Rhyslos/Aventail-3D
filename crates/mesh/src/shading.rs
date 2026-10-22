//! Turning a mesh into triangles to draw, with smooth or sharp shading
//! decided by its hard edges.
//!
//! Each corner of each face gets its own normal (the direction light
//! bounces from): the average of the faces around that point that are
//! joined to this face across soft edges, each counted by the angle it
//! fills at that point. Across a hard edge the shading
//! breaks, giving a crisp edge; across soft edges it flows, giving a
//! smooth curve. Normals are always worked out like this, never stored
//! (DESIGN.md §4.1).
//!
//! The shading can also break by angle instead of by the marks
//! ([`Hardness::ByAngle`], [decision 0064](../../../docs/decisions/0064-automatic-hard-edges.md)):
//! wherever two polygons meet more sharply than the angle, whatever the
//! edge is marked. The viewport does this while *Automatic hard edges* is
//! on, so a cylinder's sides stay smooth and its caps crisp after any edit.

use crate::EdgeKey;
use crate::polygon::{FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;

/// What decides where the shading breaks.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Hardness {
    /// The edges marked hard (the standard).
    #[default]
    Marked,
    /// Every edge where two polygons meet more sharply than this many
    /// degrees, and every edge marked hard (decision 0126: the marks add
    /// to the angle); an edge shared by more than two polygons too. Edges
    /// between polygons with no area stay soft unless marked.
    ByAngle(f32),
}

/// Whether two polygons with these normals meet more sharply than the
/// angle whose cosine is `cos`. A polygon with no area has no facing:
/// such an edge stays soft.
fn meet_sharply(n1: Vec3, n2: Vec3, cos: f32) -> bool {
    n1.length_squared() > 0.5 && n2.length_squared() > 0.5 && n1.dot(n2) < cos - 1e-6
}

/// The cosine of `angle` degrees, kept within 0–180°.
fn cosine(angle: f32) -> f32 {
    let angle = if angle.is_finite() { angle } else { 0.0 };
    angle
        .clamp(0.0, crate::hardness::MAX_ANGLE)
        .to_radians()
        .cos()
}

/// One triangle ready to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadedTriangle {
    /// The face it is part of.
    pub face: FaceId,
    /// Its corners, counter-clockwise seen from outside.
    pub positions: [Vec3; 3],
    /// The shading direction at each corner; unit length.
    pub normals: [Vec3; 3],
}

/// Every face cut into triangles ([`crate::triangulate()`]: bent and
/// dented polygons are cut along lines inside them), with shading normals.
pub fn shaded_triangles(mesh: &Mesh) -> Vec<ShadedTriangle> {
    let normals = corner_normal_list(mesh);
    let (starts, corners) = mesh.face_layout();
    let mut out = Vec::new();
    for (index, pair) in starts.windows(2).enumerate() {
        let (a, b) = (pair[0] as usize, pair[1] as usize);
        let face = FaceId::from_raw(index as u32);
        let corner = |k: usize| {
            let position = mesh.position(corners[k]).unwrap_or(Vec3::ZERO);
            (position, normals[k])
        };
        crate::triangulate::face_triangles(mesh, &corners[a..b], |[i, j, k]| {
            let (p, q, r) = (corner(a + i), corner(a + j), corner(a + k));
            out.push(ShadedTriangle {
                face,
                positions: [p.0, q.0, r.0],
                normals: [p.1, q.1, r.1],
            });
        });
    }
    out
}

/// The shading normal of every face corner, keyed by face and point.
pub fn corner_normals(mesh: &Mesh) -> BTreeMap<(FaceId, VertexId), Vec3> {
    let normals = corner_normal_list(mesh);
    let (starts, corners) = mesh.face_layout();
    let mut out = BTreeMap::new();
    for (index, pair) in starts.windows(2).enumerate() {
        for k in pair[0] as usize..pair[1] as usize {
            out.insert((FaceId::from_raw(index as u32), corners[k]), normals[k]);
        }
    }
    out
}

/// The shading normal of every face corner, in the order the mesh lists
/// its faces' corners (face by face).
pub fn corner_normal_list(mesh: &Mesh) -> Vec<Vec3> {
    corner_shading(mesh).1
}

/// The smooth groups ([`smooth_groups`]) and the shading normal of every
/// face corner, in the order the mesh lists its faces' corners. Worked out
/// in a few passes over the corners, so it stays quick for heavy meshes.
pub fn corner_shading(mesh: &Mesh) -> (Vec<u32>, Vec<Vec3>) {
    corner_shading_with(mesh, Hardness::Marked)
}

/// [`corner_shading`], with the shading breaking where `hardness` says.
pub fn corner_shading_with(mesh: &Mesh, hardness: Hardness) -> (Vec<u32>, Vec<Vec3>) {
    let groups = smooth_groups_with(mesh, hardness);
    let (starts, corners) = mesh.face_layout();
    let faces = starts.len().saturating_sub(1);
    // Each face's normal, and how much it counts at each of its corners
    // (its normal times the corner's angle), worked out a share of the
    // faces per thread.
    let parts = crate::parallel::in_parallel(faces, |range| {
        let mut face_normals = Vec::with_capacity(range.len());
        let mut weights = Vec::new();
        for f in range {
            let (a, b) = (starts[f] as usize, starts[f + 1] as usize);
            let normal = mesh.face_normal(FaceId::from_raw(f as u32));
            face_normals.push(normal);
            let n = b.saturating_sub(a);
            let at = |i: usize| mesh.position(corners[a + i]).unwrap_or(Vec3::ZERO);
            for i in 0..n {
                let here = at(i);
                let angle = (at((i + n - 1) % n) - here).angle_between(at((i + 1) % n) - here);
                let angle = if angle.is_finite() { angle } else { 0.0 };
                weights.push(normal * angle.max(1e-6));
            }
        }
        (face_normals, weights)
    });
    let mut face_normals = Vec::with_capacity(faces);
    let mut weights = Vec::with_capacity(corners.len());
    for (normals, part) in parts {
        face_normals.extend(normals);
        weights.extend(part);
    }
    // Each group's weights add up at its first corner.
    let mut sums = vec![Vec3::ZERO; corners.len()];
    for (k, weight) in weights.iter().enumerate() {
        sums[groups[k] as usize] += *weight;
    }
    let mut normals = vec![Vec3::Z; corners.len()];
    for (f, pair) in starts.windows(2).enumerate() {
        for k in pair[0] as usize..pair[1] as usize {
            normals[k] = sums[groups[k] as usize]
                .try_normalize()
                .unwrap_or(face_normals[f]);
        }
    }
    (groups, normals)
}

/// Where point `v` is in the mesh's corner list as a corner of `face`.
fn corner_at(mesh: &Mesh, face: FaceId, v: VertexId) -> Option<usize> {
    let (starts, corners) = mesh.face_layout();
    let f = face.get() as usize;
    let (a, b) = (*starts.get(f)? as usize, *starts.get(f + 1)? as usize);
    (a..b).find(|k| corners[*k] == v)
}

/// The smooth groups at every point: for each face corner (in the order
/// the mesh lists them), the first corner of the group it shades with.
/// Corners around a point are in one group when their faces are joined
/// through soft edges at that point; a hard edge splits them. Groups
/// depend only on the faces and hard edges, not on where points are, so
/// they stay the same while points move.
pub fn smooth_groups(mesh: &Mesh) -> Vec<u32> {
    smooth_groups_with(mesh, Hardness::Marked)
}

/// [`smooth_groups`], with the groups split where `hardness` says. By
/// angle, the groups depend on where points are too: see
/// [`angle_hardness_changed`].
pub fn smooth_groups_with(mesh: &Mesh, hardness: Hardness) -> Vec<u32> {
    let topology = mesh.topology();
    let (starts, corners) = mesh.face_layout();
    let mut groups: Vec<u32> = (0..corners.len() as u32).collect();
    let any_hard = mesh.hard_edge_count() > 0;
    // By angle: each face's normal, worked out once.
    let by_angle = match hardness {
        Hardness::Marked => None,
        Hardness::ByAngle(angle) => {
            // Worked out a share of the faces per thread.
            let faces = starts.len().saturating_sub(1);
            let parts = crate::parallel::in_parallel(faces, |range| {
                range
                    .map(|f| mesh.face_normal(FaceId::from_raw(f as u32)))
                    .collect::<Vec<Vec3>>()
            });
            let normals: Vec<Vec3> = parts.into_iter().flatten().collect();
            Some((cosine(angle), normals))
        }
    };
    // Whether the edge from `v` to `other`, between the faces of corners
    // `i` and `j` at `v`, breaks the shading. By angle, an edge shared by
    // more than two of the faces around `v` breaks it too: counted from
    // the corners at hand rather than looked up.
    let hard = |v: VertexId,
                other: VertexId,
                local: &[(usize, VertexId, VertexId, usize)],
                i: usize,
                j: usize| match &by_angle {
        None => any_hard && mesh.is_hard(v, other),
        Some((cos, normals)) => {
            // Hand-marked hard edges add to the angle (decision 0126):
            // marking an edge hard always shows.
            if any_hard && mesh.is_hard(v, other) {
                return true;
            }
            let sharing = local
                .iter()
                .filter(|(_, p, n, _)| *p == other || *n == other)
                .count();
            sharing > 2 || meet_sharply(normals[local[i].3], normals[local[j].3], *cos)
        }
    };
    // Each corner's previous and next corner in its face.
    let neighbours = |k: usize, f: usize| {
        let (a, b) = (starts[f] as usize, starts[f + 1] as usize);
        let prev = if k == a { b - 1 } else { k - 1 };
        let next = if k + 1 == b { a } else { k + 1 };
        (corners[prev], corners[next])
    };
    // Each point's corners are grouped on their own, so the points are
    // shared out between threads; each gives (corner, its group's first
    // corner) for its points.
    let parts = crate::parallel::in_parallel(mesh.point_slot_count(), |range| {
        // The corners at one point: (corner, its two neighbouring points,
        // its face).
        let mut local: Vec<(usize, VertexId, VertexId, usize)> = Vec::new();
        let mut parent: Vec<usize> = Vec::new();
        let mut out: Vec<(u32, u32)> = Vec::new();
        for slot in range {
            let v = VertexId::from_raw(slot as u32);
            if mesh.position(v).is_none() {
                continue;
            }
            local.clear();
            for face in topology.vertex_faces(v) {
                if let Some(k) = corner_at(mesh, *face, v) {
                    let f = face.get() as usize;
                    let (prev, next) = neighbours(k, f);
                    local.push((k, prev, next, f));
                }
            }
            if local.len() < 2 {
                continue;
            }
            parent.clear();
            parent.extend(0..local.len());
            // Two corners at `v` join when their faces share a soft edge
            // through `v`: one corner's neighbour is the other's.
            for i in 0..local.len() {
                for j in i + 1..local.len() {
                    let (_, pi, ni, _) = local[i];
                    let (_, pj, nj, _) = local[j];
                    for other in [pi, ni] {
                        if (other == pj || other == nj) && !hard(v, other, &local, i, j) {
                            join(&mut parent, i, j);
                        }
                    }
                }
            }
            for i in 0..local.len() {
                let root = find(&mut parent, i);
                let first = (0..local.len())
                    .filter(|j| find(&mut parent, *j) == root)
                    .map(|j| local[j].0)
                    .min()
                    .unwrap_or(local[i].0);
                out.push((local[i].0 as u32, first as u32));
            }
        }
        out
    });
    for part in parts {
        for (k, first) in part {
            groups[k as usize] = first;
        }
    }
    groups
}

/// The shading normal of each smooth group at point `v`: the faces of
/// the group, each counted by its normal times the angle it fills at `v`
/// (so a face counts by how much of the view around the point it fills,
/// however it is cut up). Gives each group's first corner (as
/// [`smooth_groups`] names it) and its normal.
pub fn group_normals_at(mesh: &Mesh, groups: &[u32], v: VertexId) -> Vec<(u32, Vec3)> {
    let topology = mesh.topology();
    let mut sums: Vec<(u32, Vec3, FaceId)> = Vec::new();
    for face in topology.vertex_faces(v) {
        let Some(k) = corner_at(mesh, *face, v) else {
            continue;
        };
        let own = mesh.face(*face).unwrap_or_default();
        let n = own.len();
        let i = own.iter().position(|c| *c == v).unwrap_or(0);
        let at = |j: usize| mesh.position(own[j]).unwrap_or(Vec3::ZERO);
        let here = at(i);
        let angle = (at((i + n - 1) % n) - here).angle_between(at((i + 1) % n) - here);
        let angle = if angle.is_finite() { angle } else { 0.0 };
        let weighted = mesh.face_normal(*face) * angle.max(1e-6);
        let rep = groups.get(k).copied().unwrap_or(k as u32);
        match sums.iter_mut().find(|(r, ..)| *r == rep) {
            Some(entry) => entry.1 += weighted,
            None => sums.push((rep, weighted, *face)),
        }
    }
    sums.into_iter()
        .map(|(rep, sum, face)| {
            (
                rep,
                sum.try_normalize()
                    .unwrap_or_else(|| mesh.face_normal(face)),
            )
        })
        .collect()
}

/// The points whose shading changes when `moved` points move: every
/// corner of every face that uses one of them (the faces turn, and the
/// angles at their corners change).
pub fn shading_reach(mesh: &Mesh, moved: &[VertexId]) -> Vec<VertexId> {
    let topology = mesh.topology();
    let mut out: Vec<VertexId> = Vec::new();
    for v in moved {
        out.push(*v);
        for face in topology.vertex_faces(*v) {
            out.extend_from_slice(mesh.face(*face).unwrap_or_default());
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Whether moving points from where they are in `before` to where they
/// are in `after` (the same faces) changes which edges around `around`
/// meet more sharply than `angle` degrees: if so, shading by angle has
/// new groups and must be worked out afresh rather than updated.
pub fn angle_hardness_changed(
    before: &Mesh,
    after: &Mesh,
    around: &[VertexId],
    angle: f32,
) -> bool {
    let cos = cosine(angle);
    let topology = after.topology();
    around.iter().any(|v| {
        topology.vertex_faces(*v).iter().any(|face| {
            let corners = after.face(*face).unwrap_or_default();
            let n = corners.len();
            (0..n).any(|i| {
                let edge = EdgeKey::new(corners[i], corners[(i + 1) % n]);
                match topology.edge_uses(edge) {
                    [one, two] => {
                        meet_sharply(
                            before.face_normal(one.face),
                            before.face_normal(two.face),
                            cos,
                        ) != meet_sharply(
                            after.face_normal(one.face),
                            after.face_normal(two.face),
                            cos,
                        )
                    }
                    _ => false,
                }
            })
        })
    })
}

/// The group's representative, shortening the path on the way.
fn find(group: &mut [usize], mut i: usize) -> usize {
    while group[i] != i {
        group[i] = group[group[i]];
        i = group[i];
    }
    i
}

fn join(group: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(group, a), find(group, b));
    if ra != rb {
        group[ra.max(rb)] = ra.min(rb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two squares meeting at a right angle along the X axis: one on the
    /// ground facing up, one standing up facing -Y.
    fn folded(hard: bool) -> (Mesh, [VertexId; 6]) {
        let mut mesh = Mesh::new();
        let v = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ]
        .map(|p| mesh.add_vertex(p).unwrap());
        mesh.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();
        mesh.add_face(&[v[0], v[5], v[4], v[1]]).unwrap();
        mesh.set_hard(v[0], v[1], hard);
        (mesh, v)
    }

    #[test]
    fn hard_edges_keep_each_face_its_own_normal() {
        let (mesh, v) = folded(true);
        let normals = corner_normals(&mesh);
        let top = mesh.faces().next().unwrap().0;
        assert!((normals[&(top, v[0])] - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn soft_edges_blend_the_normals_of_the_faces_they_join() {
        let (mesh, v) = folded(false);
        let normals = corner_normals(&mesh);
        let top = mesh.faces().next().unwrap().0;
        let blended = normals[&(top, v[0])];
        let expected = (Vec3::Z + Vec3::NEG_Y).normalize();
        assert!((blended - expected).length() < 1e-5, "{blended}");
        // A corner touching only one face keeps that face's normal.
        assert!((normals[&(top, v[2])] - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn faces_are_cut_into_fans_of_triangles() {
        let (mesh, _) = folded(true);
        let triangles = shaded_triangles(&mesh);
        assert_eq!(triangles.len(), 4, "two quads, two triangles each");
        for t in &triangles {
            // Counter-clockwise: the winding agrees with the normal.
            let winding = (t.positions[1] - t.positions[0]).cross(t.positions[2] - t.positions[0]);
            assert!(winding.dot(t.normals[0]) > 0.0);
        }
    }

    #[test]
    fn an_empty_mesh_draws_nothing() {
        assert!(shaded_triangles(&Mesh::new()).is_empty());
        assert!(corner_shading(&Mesh::new()).0.is_empty());
    }

    #[test]
    fn groups_split_at_hard_edges_and_stay_while_points_move() {
        let (soft, v) = folded(false);
        let groups = smooth_groups(&soft);
        // v[0] is a corner of both faces: one group when soft.
        let (starts, corners) = soft.face_layout();
        let at = |f: usize| {
            (starts[f] as usize..starts[f + 1] as usize)
                .find(|k| corners[*k] == v[0])
                .unwrap()
        };
        assert_eq!(groups[at(0)], groups[at(1)]);
        let (hard, _) = folded(true);
        let groups = smooth_groups(&hard);
        assert_ne!(groups[at(0)], groups[at(1)]);
        // Moving a point changes the normals, not the groups.
        let mut moved = soft.clone();
        moved.set_position(v[2], Vec3::new(1.0, 1.0, 0.5));
        assert_eq!(smooth_groups(&moved), smooth_groups(&soft));
        assert_ne!(corner_normal_list(&moved), corner_normal_list(&soft));
    }

    #[test]
    fn faces_meeting_only_at_a_point_shade_apart() {
        // Two triangles touching at one point, sharing no edge (a "bow
        // tie"): each keeps its own normal there.
        let mut mesh = Mesh::new();
        let p = [
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.5),
            Vec3::new(0.0, -1.0, 0.5),
        ]
        .map(|q| mesh.add_vertex(q).unwrap());
        mesh.add_face(&[p[0], p[1], p[2]]).unwrap();
        mesh.add_face(&[p[0], p[3], p[4]]).unwrap();
        let groups = smooth_groups(&mesh);
        assert_ne!(groups[0], groups[3], "the shared point's two corners");
    }

    #[test]
    fn updating_around_moved_points_matches_working_everything_out() {
        let mesh = crate::samples::heavy_ball(3_000, 50.0);
        let mut moved = mesh.clone();
        let points: Vec<VertexId> = mesh.faces().nth(1_000).unwrap().1.to_vec();
        for v in &points {
            let q = moved.position(*v).unwrap();
            moved.set_position(*v, q * 1.2);
        }
        let (groups, full) = corner_shading(&moved);
        let reach = shading_reach(&moved, &points);
        assert!(
            reach.len() > points.len() && reach.len() < 40,
            "{}",
            reach.len()
        );
        for v in reach {
            for (rep, normal) in group_normals_at(&moved, &groups, v) {
                assert!((full[rep as usize] - normal).length() < 1e-4);
            }
        }
    }

    /// The number of different groups among `point`'s corners.
    fn groups_at(mesh: &Mesh, groups: &[u32], point: VertexId) -> usize {
        let (_, corners) = mesh.face_layout();
        let mut found: Vec<u32> = (0..corners.len())
            .filter(|k| corners[*k] == point)
            .map(|k| groups[k])
            .collect();
        found.sort_unstable();
        found.dedup();
        found.len()
    }

    #[test]
    fn by_angle_the_marks_are_ignored() {
        // A 90° fold: hard by angle whatever it is marked.
        for marked in [false, true] {
            let (mesh, v) = folded(marked);
            let groups = smooth_groups_with(&mesh, Hardness::ByAngle(30.0));
            assert_eq!(groups_at(&mesh, &groups, v[0]), 2, "marked {marked}");
            // Above the fold's angle, it is soft.
            let groups = smooth_groups_with(&mesh, Hardness::ByAngle(120.0));
            assert_eq!(groups_at(&mesh, &groups, v[0]), 1, "marked {marked}");
        }
    }

    #[test]
    fn a_cylinder_marked_hard_all_over_shades_smooth_sides_and_crisp_caps() {
        // As after an extrude that marked every new edge hard.
        let mut cylinder = crate::Primitive::Cylinder(crate::CylinderParams {
            sides: 16,
            ..Default::default()
        })
        .mesh();
        let edges: Vec<EdgeKey> = cylinder.edges().to_vec();
        for e in &edges {
            let (a, b) = e.ends();
            cylinder.set_hard(a, b, true);
        }
        let marked = smooth_groups(&cylinder);
        let by_angle = smooth_groups_with(&cylinder, Hardness::ByAngle(30.0));
        for (v, p) in cylinder.vertices() {
            // A point on a cap's rim: the sides (22.5° apart) shade as one,
            // apart from the cap (90°).
            if p.truncate().length() > 1.0 {
                assert_eq!(groups_at(&cylinder, &by_angle, v), 2, "{p}");
                assert!(groups_at(&cylinder, &marked, v) > 2);
            }
        }
        // The normals on the sides point straight out, as on a smooth
        // cylinder.
        let (_, normals) = corner_shading_with(&cylinder, Hardness::ByAngle(30.0));
        let (_, corners) = cylinder.face_layout();
        for (k, n) in normals.iter().enumerate() {
            let p = cylinder.position(corners[k]).unwrap();
            if n.z.abs() < 0.5 {
                let out = p.truncate().normalize().extend(0.0);
                assert!(n.dot(out) > 0.99, "{p} {n}");
            }
        }
    }

    #[test]
    fn by_angle_handles_odd_shapes() {
        // Three polygons on one edge: hard. A polygon with no area: soft.
        let mut mesh = Mesh::new();
        let p = [
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(2.0, 0.0, 0.0),
        ]
        .map(|q| mesh.add_vertex(q).unwrap());
        mesh.add_face(&[p[0], p[1], p[2]]).unwrap();
        mesh.add_face(&[p[1], p[0], p[3]]).unwrap();
        mesh.add_face(&[p[0], p[1], p[4]]).unwrap();
        let groups = smooth_groups_with(&mesh, Hardness::ByAngle(170.0));
        assert_eq!(groups_at(&mesh, &groups, p[0]), 3, "non-manifold: apart");
        // A flat sliver with no area beside a polygon.
        let mut sliver = Mesh::new();
        let q = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(2.0, 0.0, 0.0)]
            .map(|x| sliver.add_vertex(x).unwrap());
        sliver.add_face(&[q[0], q[1], q[2]]).unwrap();
        sliver.add_face(&[q[1], q[0], q[3]]).unwrap();
        let groups = smooth_groups_with(&sliver, Hardness::ByAngle(1.0));
        assert_eq!(groups_at(&sliver, &groups, q[0]), 1);
        // Numbers that aren't real count as 0°.
        let (mesh, v) = folded(false);
        let groups = smooth_groups_with(&mesh, Hardness::ByAngle(f32::NAN));
        assert_eq!(groups_at(&mesh, &groups, v[0]), 2);
        // An empty mesh.
        assert!(smooth_groups_with(&Mesh::new(), Hardness::ByAngle(30.0)).is_empty());
    }

    #[test]
    fn moving_points_can_change_which_edges_are_sharp() {
        let (flat, v) = folded(false);
        // Lift the far edge of the second face a little: still sharp.
        let mut slight = flat.clone();
        slight.set_position(v[4], Vec3::new(1.0, -0.1, -1.0));
        let around = shading_reach(&flat, &[v[4]]);
        assert!(!angle_hardness_changed(&flat, &slight, &around, 30.0));
        // Fold it nearly flat: the edge turns soft.
        let mut opened = flat.clone();
        opened.set_position(v[4], Vec3::new(1.0, -1.0, -0.05));
        opened.set_position(v[5], Vec3::new(0.0, -1.0, -0.05));
        let around = shading_reach(&flat, &[v[4], v[5]]);
        assert!(angle_hardness_changed(&flat, &opened, &around, 30.0));
    }
}
