//! Texel density: how much texture each piece gets for its real size
//! ([decision 0100](../../../docs/decisions/0100-texture-size-and-texel-density.md)).
//!
//! Measured here as a length ratio: texture widths per unit of length on
//! the shape (the square root of a piece's UV area over its real area).
//! The app turns it into pixels per metre with the project's texture size.

use glam::{Vec2, Vec3};
use mesh::edit::Outcome;
use mesh::uvs::Uvs;
use mesh::{FaceId, Mesh};
use std::collections::BTreeSet;

/// The UV and real areas of `faces`, by their current UVs.
pub fn areas(mesh: &Mesh, faces: &[FaceId], uvs: &Uvs) -> (f32, f32) {
    let (starts, _) = mesh.face_layout();
    let mut flat = 0.0;
    let mut real = 0.0;
    for f in faces {
        let Some(list) = mesh.face(*f) else {
            continue;
        };
        let start = starts[f.get() as usize] as usize;
        mesh::triangulate::face_triangles(mesh, list, |[a, b, c]| {
            let (ua, ub, uc) = (uvs.at(start + a), uvs.at(start + b), uvs.at(start + c));
            flat += ((ub - ua).perp_dot(uc - ua) / 2.0).abs();
            let p = |i: usize| mesh.position(list[i]).unwrap_or(Vec3::ZERO);
            real += (p(b) - p(a)).cross(p(c) - p(a)).length() / 2.0;
        });
    }
    (flat, real)
}

/// The texel density of `faces` (texture widths per unit of length), if
/// they have UVs and some area.
pub fn density(mesh: &Mesh, faces: &[FaceId]) -> Option<f32> {
    let uvs = Uvs::of(mesh)?;
    let (flat, real) = areas(mesh, faces, &uvs);
    (flat > 1e-20 && real > 1e-20).then(|| (flat / real).sqrt())
}

/// Scales each piece holding any of `faces` about the middle of its box so
/// its texel density becomes `target` (texture widths per unit of
/// length). Pieces without area, and locked pieces, are skipped.
pub fn set_density(mesh: &mut Mesh, faces: &[FaceId], target: f32) -> Outcome {
    let found = faces.iter().filter(|f| mesh.face(**f).is_some()).count();
    let unchanged = Outcome {
        matched: 0,
        missing: faces.len() - found,
        skipped: found,
    };
    let Some(uvs) = Uvs::of(mesh) else {
        return unchanged;
    };
    if !(target.is_finite() && target > 0.0) {
        return unchanged;
    }
    let cuts: BTreeSet<mesh::EdgeKey> = if mesh.attribute_values(mesh::attributes::SEAM).is_some() {
        mesh::uvs::seams(mesh).into_iter().collect()
    } else {
        mesh::uvs::uv_splits(mesh).into_iter().collect()
    };
    let wanted: BTreeSet<FaceId> = faces.iter().copied().collect();
    // Locked pieces are left as they are (decision 0112).
    let pieces: Vec<Vec<FaceId>> = crate::seams::pieces(mesh, &cuts)
        .into_iter()
        .filter(|p| p.iter().any(|f| wanted.contains(f)))
        .filter(|p| !mesh::uvs::any_locked(mesh, p))
        .collect();
    let (starts, _) = mesh.face_layout();
    let starts = starts.to_vec();
    let count = mesh.face_layout().1.len();
    let mut values = uvs.clone().into_values(count);
    let mut any = false;
    for piece in &pieces {
        let (flat, real) = areas(mesh, piece, &uvs);
        if flat <= 1e-20 || real <= 1e-20 {
            continue;
        }
        let k = target / (flat / real).sqrt();
        let mut corners = Vec::new();
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for f in piece {
            let n = mesh.face(*f).map_or(0, <[mesh::VertexId]>::len);
            let start = starts[f.get() as usize] as usize;
            for (c, v) in values.iter().enumerate().skip(start).take(n) {
                corners.push(c);
                min = min.min(*v);
                max = max.max(*v);
            }
        }
        let middle = (min + max) / 2.0;
        for c in corners {
            values[c] = middle + (values[c] - middle) * k;
        }
        any = true;
    }
    if !any || !mesh::uvs::set_uvs(mesh, values) {
        return unchanged;
    }
    Outcome {
        matched: found,
        missing: faces.len() - found,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::{CubeParams, Primitive};

    #[test]
    fn a_cubes_sides_are_set_to_one_density_about_their_middles() {
        let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        let before = density(&mesh, &faces).unwrap();
        let top = faces[0];
        let middle_before = {
            let uvs = Uvs::of(&mesh).unwrap();
            let c: Vec<Vec2> = (0..4).map(|k| uvs.corner(&mesh, top, k).unwrap()).collect();
            (c[0] + c[2]) / 2.0
        };
        let done = set_density(&mut mesh, &[top], before * 2.0);
        assert_eq!(done.matched, 1);
        let now = density(&mesh, &[top]).unwrap();
        assert!((now - before * 2.0).abs() < 1e-4 * before);
        let uvs = Uvs::of(&mesh).unwrap();
        let c: Vec<Vec2> = (0..4).map(|k| uvs.corner(&mesh, top, k).unwrap()).collect();
        assert!(((c[0] + c[2]) / 2.0 - middle_before).length() < 1e-5);
        // The other sides are as they were.
        let rest: Vec<FaceId> = faces[1..].to_vec();
        assert!((density(&mesh, &rest).unwrap() - before).abs() < 1e-4 * before);
    }

    #[test]
    fn nonsense_changes_nothing() {
        let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        for target in [0.0, -1.0, f32::NAN] {
            assert_eq!(set_density(&mut mesh, &faces, target).matched, 0);
        }
        let ghost = FaceId::from_raw(500);
        let done = set_density(&mut mesh, &[ghost], 1.0);
        assert_eq!((done.matched, done.missing), (0, 1));
        mesh.remove_attribute(mesh::attributes::UV);
        assert!(density(&mesh, &faces).is_none());
        assert_eq!(set_density(&mut mesh, &faces, 1.0).matched, 0);
    }
}
