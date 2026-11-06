//! Lightmap UVs for Godot's baked lighting ([decision 0242](../../../docs/decisions/0242-lightmap-uvs.md)).
//!
//! A lightmap needs every bit of the surface to have pixels of its own
//! (nothing stacked or mirrored onto another piece) with room round each
//! piece so the light doesn't bleed, and pieces at their real relative
//! sizes so the light is equally sharp everywhere. [`lightmap`] makes such
//! a layout without touching the shape's own UVs: on a copy, it cuts along
//! the hard edges (`hard`) as well as the seams, unwraps everything
//! keeping sizes, and packs it all into the square, never stacking, with
//! `padding` pixels between pieces at the lightmap's `size`.

use mesh::{EdgeKey, Mesh};

/// The standard lightmap size, and the padding round its pieces.
pub const LIGHTMAP_SIZE: u32 = 512;
/// See [`LIGHTMAP_SIZE`].
pub const LIGHTMAP_PADDING: u32 = 4;
/// The sizes a lightmap can be.
pub const LIGHTMAP_SIZES: [u32; 5] = [128, 256, 512, 1024, 2048];

/// The lightmap layout of `mesh` (one UV per corner, in corner order),
/// or `None` when it has no surface to lay out. `hard` are the edges to
/// cut as well as the seams (the shading's hard edges: a lightmap needs
/// them split, as a normal map does).
pub fn lightmap(mesh: &Mesh, hard: &[EdgeKey], size: u32, padding: u32) -> Option<Vec<glam::Vec2>> {
    if mesh.face_count() == 0 {
        return None;
    }
    let size = size.clamp(16, 8192);
    let mut work = mesh.clone();
    // Start from a plain projection when the shape has no UVs yet.
    if !mesh::uvs::has_uvs(&work) {
        let corners = work.face_layout().1.len();
        mesh::uvs::set_uvs(&mut work, vec![glam::Vec2::ZERO; corners]);
    }
    // Locked pieces must still be laid out here: the lightmap is its own.
    let faces: Vec<mesh::FaceId> = work.faces().map(|(f, _)| f).collect();
    mesh::uvs::set_locked(&mut work, &faces, false);
    mesh::uvs::set_pins(&mut work, &faces, &[], false);
    let mut cuts: Vec<EdgeKey> = mesh::uvs::seams(&work);
    cuts.extend(hard.iter().copied());
    mesh::uvs::set_seams(&mut work, &cuts, true);
    let options = crate::UnwrapOptions {
        strips: true,
        method: crate::UnwrapMethod::SizeKeeping,
        ..crate::UnwrapOptions::default()
    };
    crate::unwrap_with(&mut work, &faces, options);
    let islands = crate::pack::islands_of(&work, &faces);
    let pieces: Vec<crate::pack::Piece> = islands
        .iter()
        .map(|i| crate::pack::piece_of(&work, i))
        .collect();
    let pack = crate::pack::PackOptions {
        padding: padding as f32 / size as f32,
        rotate: true,
        stack: false,
        cells: (size as usize / 2).clamp(64, crate::pack::MAX_CELLS),
        ..crate::pack::PackOptions::default()
    };
    let moves = crate::pack::pack(&pieces, &pack)?;
    let placed: Vec<(Vec<mesh::FaceId>, glam::Affine2)> = islands.into_iter().zip(moves).collect();
    crate::pack::place(&mut work, &placed);
    let uvs = mesh::uvs::Uvs::of(&work)?;
    Some(uvs.into_values(work.face_layout().1.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh::{CubeParams, Primitive, SphereParams};

    #[test]
    fn a_cube_gets_six_unstacked_pieces_inside_the_square() {
        let cube = Primitive::Cube(CubeParams::default()).mesh();
        let hard: Vec<EdgeKey> = cube.hard_edges().collect();
        let uvs = lightmap(&cube, &hard, LIGHTMAP_SIZE, LIGHTMAP_PADDING).unwrap();
        assert_eq!(uvs.len(), cube.face_layout().1.len());
        for uv in &uvs {
            assert!(
                (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
                "{uv}"
            );
        }
        let mut laid = cube.clone();
        mesh::uvs::set_uvs(&mut laid, uvs);
        assert_eq!(mesh::uvs::islands(&laid).len(), 6);
        assert!(
            crate::select::overlapping(&laid).is_empty(),
            "nothing shares pixels"
        );
        // The shape's own UVs are untouched.
        assert_eq!(
            mesh::uvs::Uvs::of(&cube).unwrap().into_values(24),
            mesh::uvs::Uvs::of(&Primitive::Cube(CubeParams::default()).mesh())
                .unwrap()
                .into_values(24)
        );
    }

    #[test]
    fn stacked_and_locked_layouts_still_get_their_own_pixels() {
        let mut ball = Primitive::Sphere(SphereParams::default()).mesh();
        let faces: Vec<mesh::FaceId> = ball.faces().map(|(f, _)| f).collect();
        mesh::uvs::set_locked(&mut ball, &faces, true);
        let uvs = lightmap(&ball, &[], 256, 2).unwrap();
        let mut laid = ball.clone();
        mesh::uvs::set_uvs(&mut laid, uvs);
        assert!(crate::select::overlapping(&laid).is_empty());
    }

    #[test]
    fn an_empty_shape_has_no_lightmap() {
        assert_eq!(lightmap(&Mesh::default(), &[], 512, 4), None);
    }
}
