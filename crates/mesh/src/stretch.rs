//! Edits on stretched objects ([decision 0081](../../../docs/decisions/0081-bends-keep-their-shape-on-stretched-objects.md),
//! [decision 0082](../../../docs/decisions/0082-edits-keep-their-shape-on-stretched-objects.md)).
//!
//! An object scaled more along one axis than another is drawn stretched.
//! Edits that measure across a shape (a bevel's width, an inset's
//! distance, which way is "straight out", a bend) are worked out in the
//! mesh's own space, so on a stretched object they would come out
//! stretched too: a bevel wider one way than the other, a bend that
//! widens a pipe. [`as_seen`] runs an edit on the points as they are
//! seen instead, stretched by the object's stretch (its scale on each
//! axis over their average), and takes the result back into the mesh's
//! own space.

use crate::{Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;

/// The most an object's stretch may be along one axis compared with the
/// others' average: beyond it a stretch is taken as none.
pub const MAX_STRETCH: f32 = 1000.0;

/// `stretch` if it is a real, usable stretch (each axis above
/// 1 / [`crate::stretch::MAX_STRETCH`] and below [`crate::stretch::MAX_STRETCH`]), otherwise none
/// ([`Vec3::ONE`]).
#[must_use]
pub fn usable(stretch: Vec3) -> Vec3 {
    let fine = |x: f32| {
        x.is_finite() && x > 1.0 / crate::stretch::MAX_STRETCH && x < crate::stretch::MAX_STRETCH
    };
    if stretch.to_array().into_iter().all(fine) {
        stretch
    } else {
        Vec3::ONE
    }
}

/// Runs `edit` on `mesh` as it is seen on an object stretched by
/// `stretch` (see the module): the points are stretched, `edit` runs,
/// and every point is taken back into the mesh's own space. Points the
/// edit doesn't move keep exactly where they were (no rounding creeps
/// in). With no stretch, or one that isn't [`usable`], `edit` simply
/// runs on the mesh.
///
/// Distances the edit uses count as seen at the object's average scale.
pub fn as_seen<R>(mesh: &mut Mesh, stretch: Vec3, edit: impl FnOnce(&mut Mesh) -> R) -> R {
    let stretch = usable(stretch);
    if stretch == Vec3::ONE {
        return edit(mesh);
    }
    let before: BTreeMap<VertexId, Vec3> = mesh.vertices().collect();
    for (v, p) in &before {
        mesh.set_position(*v, *p * stretch);
    }
    let result = edit(mesh);
    let after: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    for (v, p) in after {
        let back = match before.get(&v) {
            // Not moved by the edit: exactly as it was.
            Some(old) if *old * stretch == p => *old,
            _ => p / stretch,
        };
        mesh.set_position(v, back);
    }
    result
}

/// A facing (a normal) in the mesh's own space as it is seen on an object
/// stretched by `stretch`: facings stretch the opposite way to points.
/// Not made unit length.
#[must_use]
pub fn facing_as_seen(facing: Vec3, stretch: Vec3) -> Vec3 {
    facing / usable(stretch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CubeParams, Primitive};

    #[test]
    fn usable_stretches_are_kept_and_others_are_none() {
        assert_eq!(usable(Vec3::new(0.5, 1.0, 1.5)), Vec3::new(0.5, 1.0, 1.5));
        for broken in [
            Vec3::new(f32::NAN, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, f32::INFINITY, 1.0),
            Vec3::new(1.0, 1.0, crate::stretch::MAX_STRETCH),
            Vec3::new(1.0, 1.0, 1.0 / crate::stretch::MAX_STRETCH),
        ] {
            assert_eq!(usable(broken), Vec3::ONE, "{broken}");
        }
    }

    #[test]
    fn an_edit_sees_the_stretched_points_and_unmoved_points_stay_exact() {
        let before = Primitive::Cube(CubeParams::default()).mesh();
        let stretch = Vec3::new(0.3, 1.1, 1.6);
        let mut mesh = before.clone();
        let (first, _) = mesh.vertices().next().unwrap();
        let seen = as_seen(&mut mesh, stretch, |m| {
            let at = m.position(first).unwrap();
            m.set_position(first, at + Vec3::X);
            at
        });
        assert_eq!(seen, before.position(first).unwrap() * stretch);
        // The moved point moved by one as seen: 1 / 0.3 in its own space.
        let moved = mesh.position(first).unwrap() - before.position(first).unwrap();
        assert!(
            (moved - Vec3::new(1.0 / 0.3, 0.0, 0.0)).length() < 1e-3,
            "{moved}"
        );
        for (v, p) in before.vertices().skip(1) {
            assert_eq!(mesh.position(v), Some(p));
        }
    }

    #[test]
    fn no_stretch_runs_the_edit_as_it_is() {
        let before = Primitive::Cube(CubeParams::default()).mesh();
        for stretch in [Vec3::ONE, Vec3::new(f32::NAN, 2.0, 1.0)] {
            let mut mesh = before.clone();
            as_seen(&mut mesh, stretch, |m| {
                let points: Vec<Vec3> = m.vertices().map(|(_, p)| p).collect();
                assert_eq!(
                    points,
                    before.vertices().map(|(_, p)| p).collect::<Vec<_>>()
                );
            });
            assert_eq!(mesh, before);
        }
    }

    #[test]
    fn facings_stretch_the_opposite_way() {
        let f = facing_as_seen(Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 0.5, 1.0));
        assert_eq!(f, Vec3::new(0.5, 2.0, 0.0));
    }
}
