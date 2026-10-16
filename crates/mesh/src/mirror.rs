//! The live mirror ([decision 0088](../../../docs/decisions/0088-live-mirror.md)):
//! a shape with mirrored copies of itself added, welded where they meet.
//!
//! The shape itself is kept exactly as it is, its points and polygons
//! keeping their ids, so edits and selections still find them; each copy
//! comes after, with new ids. Points close to the seam (as `on_seam`
//! says) and within the weld distance of each other become one, so the
//! halves join into one surface instead of two that touch.

use crate::{Mesh, VertexId};
use glam::{Mat4, Vec3};

/// `mesh` with a copy placed by each of `copies` (mirroring matrices, in
/// the shape's own space), and the points `on_seam` picks welded where
/// they lie within `distance` of each other. A distance of 0 (or one that
/// isn't a number) welds nothing.
pub fn with_copies(
    mesh: &Mesh,
    copies: &[Mat4],
    on_seam: impl Fn(Vec3) -> bool,
    distance: f32,
) -> Mesh {
    let mut out = mesh.clone();
    for matrix in copies {
        if matrix.is_finite() && matrix.determinant().abs() > 1e-12 {
            crate::parts::append(&mut out, mesh, *matrix);
        }
    }
    if distance > 0.0 && !copies.is_empty() {
        let seam: Vec<VertexId> = out
            .vertices()
            .filter(|(_, p)| on_seam(*p))
            .map(|(v, _)| v)
            .collect();
        crate::join::weld_close(&mut out, &seam, distance);
    }
    out
}

/// The reflection across the planes of `axes` through `centre`: one
/// matrix (X and Y together mirror across both, a half turn).
pub fn reflection(axes: [bool; 3], centre: Vec3) -> Mat4 {
    let sign = |on: bool| if on { -1.0 } else { 1.0 };
    let scale = Vec3::new(sign(axes[0]), sign(axes[1]), sign(axes[2]));
    Mat4::from_translation(centre) * Mat4::from_scale(scale) * Mat4::from_translation(-centre)
}

/// Every combination of `axes` to copy across: X alone gives X; X and Y
/// give X, Y and both (three copies, four quarters in all).
pub fn combinations(axes: [bool; 3]) -> Vec<[bool; 3]> {
    let mut out = Vec::new();
    for bits in 1u8..8 {
        let pick = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
        if (0..3).all(|a| !pick[a] || axes[a]) {
            out.push(pick);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MeshReport, PlaneParams, Primitive};

    /// A cube's right half: a box from x = 0 to 50, open at x = 0.
    fn half_box() -> Mesh {
        let mut mesh = Primitive::Cube(Default::default()).mesh();
        // Move the cube so its left side sits on x = 0, then remove that
        // side: an open half.
        let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
        for (v, p) in points {
            assert!(mesh.set_position(v, p + Vec3::new(50.0, 0.0, 0.0)));
        }
        let left = mesh
            .faces()
            .find(|(_, c)| c.iter().all(|v| mesh.position(*v).unwrap().x.abs() < 1e-3))
            .map(|(f, _)| f)
            .unwrap();
        crate::cleanup::delete_elements(&mut mesh, &[], &[], &[left]);
        mesh
    }

    #[test]
    fn a_half_box_and_its_mirror_weld_into_one_closed_box() {
        let half = half_box();
        let before = MeshReport::new(&half);
        assert!(!before.is_closed());
        let whole = with_copies(
            &half,
            &[reflection([true, false, false], Vec3::ZERO)],
            |p| p.x.abs() < 0.1,
            0.1,
        );
        let r = MeshReport::new(&whole);
        assert!(r.is_closed(), "{r:?}");
        assert!(!r.has_problems(), "{r:?}");
        assert_eq!(r.pieces, 1);
        // 5 + 5 sides; 8 + 8 points less the 4 shared on the seam.
        assert_eq!(whole.face_count(), 10);
        assert_eq!(whole.vertex_count(), 12);
        // The mirror faces out too: the shape has volume either side.
        let xs: Vec<f32> = whole.vertices().map(|(_, p)| p.x).collect();
        assert!(xs.iter().any(|x| *x < -99.0));
        // The original keeps its points and polygons, with their ids.
        for (v, p) in half.vertices() {
            assert_eq!(whole.position(v), Some(p));
        }
        for (f, corners) in half.faces() {
            assert_eq!(whole.face(f).map(|c| c.to_vec()), Some(corners.to_vec()));
        }
    }

    #[test]
    fn without_a_weld_the_halves_stay_apart_and_copies_follow_the_axes() {
        let half = half_box();
        let apart = with_copies(
            &half,
            &[reflection([true, false, false], Vec3::ZERO)],
            |_| true,
            0.0,
        );
        assert_eq!(apart.vertex_count(), 16);
        assert_eq!(MeshReport::new(&apart).pieces, 2);
        assert_eq!(
            combinations([true, false, false]),
            vec![[true, false, false]]
        );
        assert_eq!(combinations([true, true, false]).len(), 3);
        assert_eq!(combinations([true, true, true]).len(), 7);
        assert!(combinations([false; 3]).is_empty());
        // Two axes: four quarters.
        let quarters = with_copies(
            &half,
            &combinations([true, true, false])
                .into_iter()
                .map(|a| reflection(a, Vec3::ZERO))
                .collect::<Vec<_>>(),
            |_| false,
            0.1,
        );
        assert_eq!(quarters.face_count(), 4 * half.face_count());
    }

    #[test]
    fn broken_matrices_and_an_empty_shape_change_nothing() {
        let plane = Primitive::Plane(PlaneParams::default()).mesh();
        let same = with_copies(&plane, &[Mat4::ZERO, Mat4::NAN], |_| true, 0.1);
        assert_eq!(same, plane);
        let empty = with_copies(
            &Mesh::new(),
            &[reflection([true, false, false], Vec3::ZERO)],
            |_| true,
            0.1,
        );
        assert_eq!(empty.vertex_count(), 0);
        // A reflection through a moved centre puts the copy beside it.
        let r = reflection([true, false, false], Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(
            r.transform_point3(Vec3::new(15.0, 2.0, 3.0)),
            Vec3::new(5.0, 2.0, 3.0)
        );
    }
}
