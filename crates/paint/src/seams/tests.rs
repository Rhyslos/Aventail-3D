//! Tests for finding UV seams (decision 0293).

use super::*;
use crate::surface::PaintTriangle;
use mesh::PrimitiveKind;

/// A square of two triangles sharing the edge from point 0 to point 2,
/// laid on the texture as two pieces: `left` gives the first triangle's
/// UVs (points 0, 1, 2), `right` the second's (points 0, 2, 3).
pub(crate) fn two_pieces(left: [Vec2; 3], right: [Vec2; 3]) -> PaintSurface {
    let p = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let tri = |corners: [u32; 3], uvs: [Vec2; 3], face: u32| PaintTriangle {
        points: corners.map(|c| p[c as usize]),
        uvs,
        normal: Vec3::Z,
        slot: 0,
        face,
        corners,
    };
    PaintSurface::from_triangles(vec![tri([0, 1, 2], left, 0), tri([0, 2, 3], right, 1)])
}

/// The two pieces side by side, the right one moved across.
pub(crate) fn side_by_side() -> PaintSurface {
    two_pieces(
        [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.4, 0.1),
            Vec2::new(0.4, 0.4),
        ],
        [
            Vec2::new(0.6, 0.1),
            Vec2::new(0.9, 0.4),
            Vec2::new(0.6, 0.4),
        ],
    )
}

#[test]
fn an_edge_cut_on_the_texture_is_a_seam_with_its_two_sides() {
    let surface = side_by_side();
    let seams = seams_of(&surface, &[0]);
    assert_eq!(seams.len(), 1);
    let seam = &seams[0];
    assert_eq!(seam.points, [0, 2]);
    assert_eq!(seam.ends, [Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)]);
    let [a, b] = seam.sides;
    assert_eq!(a.uvs, [Vec2::new(0.1, 0.1), Vec2::new(0.4, 0.4)]);
    assert_eq!(b.uvs, [Vec2::new(0.6, 0.1), Vec2::new(0.9, 0.4)]);
    assert_ne!(a.piece, b.piece);
    assert_eq!(a.inward, Vec2::new(0.4, 0.1));
    assert_eq!(seam.facing(&surface), [Vec3::Z; 2]);
    assert_eq!(find(&seams, [0, 2]), Some(0));
    assert_eq!(find(&seams, [2, 0]), None, "ids lower first");
}

#[test]
fn an_edge_with_the_same_uvs_on_both_sides_is_not_a_seam() {
    let surface = two_pieces(
        [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.4, 0.1),
            Vec2::new(0.4, 0.4),
        ],
        [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.4, 0.4),
            Vec2::new(0.1, 0.4),
        ],
    );
    assert!(seams_of(&surface, &[0]).is_empty());
}

#[test]
fn another_material_or_an_open_edge_is_not_a_seam_of_this_texture() {
    let surface = side_by_side();
    assert!(seams_of(&surface, &[3]).is_empty(), "no triangle of slot 3");
    let mut triangles = surface.triangles().to_vec();
    triangles[1].slot = 1;
    let mixed = PaintSurface::from_triangles(triangles);
    assert!(seams_of(&mixed, &[0]).is_empty(), "a material border");
    assert!(seams_of(&PaintSurface::default(), &[0]).is_empty());
}

#[test]
fn the_cubes_seams_join_its_pieces_and_lines_follow_one_cut() {
    let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
    let seams = seams_of(&surface, &[0]);
    assert!(!seams.is_empty(), "the cube is cut open");
    assert!(seams.windows(2).all(|w| w[0].points < w[1].points));
    for seam in &seams {
        let [a, b] = seam.sides;
        assert!(a.uvs[0].distance(b.uvs[0]) > 0.0 || a.uvs[1].distance(b.uvs[1]) > 0.0);
        // Both sides' triangles hold both of the seam's points.
        for side in seam.sides {
            let corners = surface.triangles()[side.triangle].corners;
            assert!(seam.points.iter().all(|p| corners.contains(p)));
        }
    }
    // A line is every edge between its two pieces joined end to end, and
    // starts with the one clicked.
    for start in 0..seams.len() {
        let line = line_of(&seams, start);
        assert_eq!(line[0], start);
        let pieces = seams[start].pieces();
        assert!(line.iter().all(|i| seams[*i].pieces() == pieces));
    }
    assert!(line_of(&seams, seams.len()).is_empty(), "past the end");
}

#[test]
fn a_seam_painted_alike_on_both_sides_doesnt_show() {
    let surface = side_by_side();
    let seams = seams_of(&surface, &[0]);
    let mut alike = Image::new(64).unwrap();
    for y in 0..64 {
        for x in 0..64 {
            alike.set_pixel(x, y, [200, 50, 50, 255]);
        }
    }
    assert!(showing(&seams, &alike).is_empty());
    let mut differ = alike.clone();
    for y in 0..64 {
        for x in 32..64 {
            differ.set_pixel(x, y, [20, 50, 220, 255]);
        }
    }
    assert_eq!(showing(&seams, &differ), vec![0]);
    // Nothing painted: nothing to compare, nothing shows.
    assert!(showing(&seams, &Image::new(64).unwrap()).is_empty());
}

/// The square's two triangles in two materials (slots 0 and 1) with
/// matching UVs: one piece on the texture, but two paintings.
pub(crate) fn two_materials() -> PaintSurface {
    let surface = two_pieces(
        [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.4, 0.1),
            Vec2::new(0.4, 0.4),
        ],
        [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.4, 0.4),
            Vec2::new(0.1, 0.4),
        ],
    );
    let mut triangles = surface.triangles().to_vec();
    triangles[1].slot = 1;
    PaintSurface::from_triangles(triangles)
}

#[test]
fn where_two_materials_meet_is_a_seam_between_them_whatever_the_uvs() {
    let surface = two_materials();
    assert!(seams_of(&surface, &[0]).is_empty());
    assert!(seams_of(&surface, &[1]).is_empty());
    let mine = seams_between(&surface, &[0], &[1]);
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].points, [0, 2]);
    assert_eq!(mine[0].sides[0].triangle, 0, "this material's side first");
    assert_eq!(mine[0].sides[1].triangle, 1);
    assert_eq!(mine[0].sides[0].uvs, mine[0].sides[1].uvs, "matching UVs");
    let theirs = seams_between(&surface, &[1], &[0]);
    assert_eq!(theirs[0].sides[0].triangle, 1);
    assert_eq!(theirs[0].sides[1].triangle, 0);
    // Cut UVs too.
    let mut triangles = side_by_side().triangles().to_vec();
    triangles[1].slot = 1;
    let cut = PaintSurface::from_triangles(triangles);
    assert_eq!(seams_between(&cut, &[0], &[1]).len(), 1);
    // Not with a material it doesn't meet, nor within one material, nor
    // the same slots on both sides.
    assert!(seams_between(&surface, &[0], &[2]).is_empty());
    assert!(seams_between(&side_by_side(), &[0], &[1]).is_empty());
    assert!(seams_between(&surface, &[0, 1], &[0, 1]).is_empty());
    assert!(seams_between(&PaintSurface::default(), &[0], &[1]).is_empty());
}

#[test]
fn a_seam_between_two_paintings_shows_when_they_differ() {
    let surface = two_materials();
    let seams = seams_between(&surface, &[0], &[1]);
    let filled = |size: u32, colour: [u8; 4]| {
        let mut image = Image::new(size).unwrap();
        for y in 0..size {
            for x in 0..size {
                image.set_pixel(x, y, colour);
            }
        }
        image
    };
    let red = filled(64, [200, 50, 50, 255]);
    let also_red = filled(32, [200, 50, 50, 255]);
    let blue = filled(32, [20, 50, 220, 255]);
    assert_eq!(showing_between(&seams, &red, &blue), vec![0]);
    assert!(showing_between(&seams, &red, &also_red).is_empty());
    assert!(showing_between(&seams, &red, &Image::new(32).unwrap()).is_empty());
}
