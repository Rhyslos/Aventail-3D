//! Modelling operations: pure functions that change a mesh.
//!
//! Each takes the mesh and the elements to work on by id. Ids recorded
//! earlier may no longer be in the mesh (an earlier change, such as fewer
//! segments on a cube, removed them), so every operation works on the ids
//! that still match and says how many didn't, instead of failing. The
//! scene's operation stack turns that count into a warning (DESIGN.md
//! §4.2). More operations arrive with the modelling tools (Phases 3 and 4).
//!
//! Extruding and insetting are in [`crate::extrude`]; they are re-exported
//! here so every operation is found in one place.

pub use crate::bevel::{BevelShape, bevel_edges, bevel_points};
pub use crate::cleanup::{
    delete_elements, dissolve_edges, dissolve_faces, dissolve_points, fill_holes, fix_facing,
    flip_faces,
};
pub use crate::extrude::{
    Bend, Grouping, extrude_bent, extrude_edges, extrude_polygons, inset_polygons,
};
pub use crate::join::{bridge_edges, bridge_faces, connect_edges, connect_points, weld};
pub use crate::loopcut::loop_cut;
pub use crate::slit::{Eye, SlitLine, slit};
use crate::{Mesh, VertexId};
use glam::{Mat4, Vec3};

/// What an operation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outcome {
    /// How many of the elements asked for were changed.
    pub matched: usize,
    /// How many of the elements asked for aren't in the mesh, and so were
    /// left out.
    pub missing: usize,
    /// How many of the elements asked for are in the mesh but don't suit
    /// the operation (an edge that isn't on an open border, for extruding
    /// edges), and so were left out.
    pub skipped: usize,
}

/// Moves `points` by `offset` (in the mesh's own space). Each point moves
/// once, even if listed twice; points not in the mesh are counted as
/// missing. A non-finite offset moves nothing (every listed point still
/// counts as matched or missing), so a bad value can't break the mesh.
pub fn move_points(mesh: &mut Mesh, points: &[VertexId], offset: Vec3) -> Outcome {
    let mut outcome = Outcome::default();
    let mut seen = std::collections::BTreeSet::new();
    for id in points {
        if !seen.insert(*id) {
            continue;
        }
        let Some(position) = mesh.position(*id) else {
            outcome.missing += 1;
            continue;
        };
        outcome.matched += 1;
        if offset.is_finite() {
            mesh.set_position(*id, position + offset);
        }
    }
    outcome
}

/// Moves `points` by `matrix` (an affine transform in the mesh's own
/// space: a turn, a scale or a move, or several together). Each point
/// moves once, even if listed twice; points not in the mesh are counted as
/// missing. A matrix with a value that isn't a number moves nothing, so a
/// bad value can't break the mesh.
pub fn transform_points(mesh: &mut Mesh, points: &[VertexId], matrix: Mat4) -> Outcome {
    let mut outcome = Outcome::default();
    let usable = matrix.is_finite();
    let mut seen = std::collections::BTreeSet::new();
    for id in points {
        if !seen.insert(*id) {
            continue;
        }
        let Some(position) = mesh.position(*id) else {
            outcome.missing += 1;
            continue;
        };
        outcome.matched += 1;
        if usable {
            let moved = matrix.transform_point3(position);
            if moved.is_finite() {
                mesh.set_position(*id, moved);
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Primitive;

    fn cube() -> Mesh {
        Primitive::Cube(Default::default()).mesh()
    }

    #[test]
    fn listed_points_move_and_the_rest_stay() {
        let before = cube();
        let mut mesh = before.clone();
        let moved = [VertexId::from_raw(0), VertexId::from_raw(3)];
        let outcome = move_points(&mut mesh, &moved, Vec3::new(0.0, 0.0, 10.0));
        assert_eq!(
            outcome,
            Outcome {
                matched: 2,
                missing: 0,
                skipped: 0
            }
        );
        for (id, p) in before.vertices() {
            let now = mesh.position(id).unwrap();
            if moved.contains(&id) {
                assert_eq!(now, p + Vec3::Z * 10.0);
            } else {
                assert_eq!(now, p);
            }
        }
        assert_eq!(mesh.face_count(), before.face_count(), "same topology");
    }

    #[test]
    fn missing_points_are_counted_not_fatal() {
        let mut mesh = cube();
        let ids = [VertexId::from_raw(1), VertexId::from_raw(500)];
        let outcome = move_points(&mut mesh, &ids, Vec3::X);
        assert_eq!(outcome.matched, 1);
        assert_eq!(outcome.missing, 1);
    }

    #[test]
    fn a_point_listed_twice_moves_once() {
        let mut mesh = cube();
        let id = VertexId::from_raw(2);
        let start = mesh.position(id).unwrap();
        let outcome = move_points(&mut mesh, &[id, id], Vec3::Y);
        assert_eq!(outcome.matched, 1);
        assert_eq!(mesh.position(id).unwrap(), start + Vec3::Y);
    }

    #[test]
    fn nothing_listed_or_an_empty_mesh_changes_nothing() {
        let mut mesh = cube();
        let before = mesh.clone();
        assert_eq!(move_points(&mut mesh, &[], Vec3::X), Outcome::default());
        assert_eq!(mesh, before);
        let mut empty = Mesh::new();
        let outcome = move_points(&mut empty, &[VertexId::from_raw(0)], Vec3::X);
        assert_eq!(outcome.missing, 1);
        assert_eq!(empty, Mesh::new());
    }

    #[test]
    fn a_broken_offset_moves_nothing() {
        let mut mesh = cube();
        let before = mesh.clone();
        for bad in [Vec3::NAN, Vec3::INFINITY, Vec3::new(1.0, f32::NAN, 0.0)] {
            let outcome = move_points(&mut mesh, &[VertexId::from_raw(0)], bad);
            assert_eq!(outcome.matched, 1);
            assert_eq!(mesh, before);
        }
    }

    #[test]
    fn moving_is_repeatable() {
        let ids: Vec<VertexId> = (0..8).map(VertexId::from_raw).collect();
        let mut a = cube();
        let mut b = cube();
        move_points(&mut a, &ids, Vec3::new(1.5, -2.0, 3.25));
        move_points(&mut b, &ids, Vec3::new(1.5, -2.0, 3.25));
        assert_eq!(a, b);
    }

    #[test]
    fn points_turn_and_scale_with_a_matrix() {
        let mut mesh = cube();
        let ids: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
        let before: Vec<Vec3> = mesh.vertices().map(|(_, p)| p).collect();
        let turn = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let outcome = transform_points(&mut mesh, &ids[..2], turn);
        assert_eq!(
            outcome,
            Outcome {
                matched: 2,
                missing: 0,
                skipped: 0
            }
        );
        let after: Vec<Vec3> = mesh.vertices().map(|(_, p)| p).collect();
        for i in 0..2 {
            let expected = turn.transform_point3(before[i]);
            assert!(after[i].distance(expected) < 1e-3);
        }
        assert_eq!(after[2..], before[2..], "the others stay");
        // Listed twice: moved once. Unknown: missing.
        let mut mesh = cube();
        let double = Mat4::from_scale(Vec3::splat(2.0));
        let first = mesh.vertices().next().unwrap();
        let outcome = transform_points(
            &mut mesh,
            &[first.0, first.0, VertexId::from_raw(999)],
            double,
        );
        assert_eq!(
            outcome,
            Outcome {
                matched: 1,
                missing: 1,
                skipped: 0
            }
        );
        assert!(mesh.position(first.0).unwrap().distance(first.1 * 2.0) < 1e-3);
        // A broken matrix moves nothing.
        let mut mesh = cube();
        let outcome = transform_points(&mut mesh, &ids, Mat4::from_scale(Vec3::NAN));
        assert_eq!(outcome.matched, ids.len());
        assert_eq!(mesh, cube());
    }
}
