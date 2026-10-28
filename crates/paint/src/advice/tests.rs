//! Tests for "Why does this look wrong?" (decision 0203).

use super::*;
use crate::brush::Brush;
use crate::fill::fill_triangles;
use crate::stroke::StrokeCanvas;
use mesh::PrimitiveKind;

fn cube() -> PaintSurface {
    PaintSurface::of(&PrimitiveKind::Cube.standard().mesh())
}

/// The cube's texture with each side (its two triangles) in `colour(side)`.
fn painted(surface: &PaintSurface, colour: impl Fn(usize) -> [u8; 4]) -> Image {
    let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
    for side in 0..6 {
        let brush = Brush {
            color: colour(side),
            ..Brush::default()
        };
        fill_triangles(&mut canvas, surface, &[side * 2, side * 2 + 1], &brush);
    }
    canvas.image().clone()
}

fn kinds(findings: &[Finding]) -> Vec<&'static str> {
    findings
        .iter()
        .map(|f| match f {
            Finding::Seams { .. } => "seams",
            Finding::Stretched { .. } => "stretched",
            Finding::Blurry { .. } => "blurry",
            Finding::FlatLight { .. } => "flat",
        })
        .collect()
}

#[test]
fn an_evenly_painted_cube_has_only_flat_light() {
    let s = cube();
    let image = painted(&s, |_| [120, 100, 80, 255]);
    let found = look_for(&s, &[0], &image, 1.0, 50.0);
    assert_eq!(kinds(&found), vec!["flat"]);
}

#[test]
fn sides_painted_differently_break_at_the_seams() {
    let s = cube();
    let image = painted(&s, |side| {
        let v = (side * 40) as u8;
        [v, 255 - v, v / 2, 255]
    });
    let found = look_for(&s, &[0], &image, 1.0, 50.0);
    let Some(Finding::Seams { faces, edges }) = found.first() else {
        panic!("{found:?}");
    };
    assert!(*edges >= 4 && faces.len() >= 4, "{edges} {faces:?}");
    // Lights and darks differ enough: not flat.
    assert!(!kinds(&found).contains(&"flat"));
}

#[test]
fn too_few_pixels_for_the_size_is_blurry() {
    let s = cube();
    let image = painted(&s, |side| [(side * 40) as u8; 4].map(|v| v.max(1)));
    // About 85 pixels a metre on a 256 texture: soft against 512 aimed for.
    let found = look_for(&s, &[0], &image, 1.0, 512.0);
    let blurry = found
        .iter()
        .find_map(|f| match f {
            Finding::Blurry {
                faces,
                pixels_per_metre,
                ..
            } => Some((faces.len(), *pixels_per_metre)),
            _ => None,
        })
        .expect("blurry");
    assert_eq!(blurry.0, 6);
    assert!((blurry.1 - 85.0).abs() < 15.0, "{}", blurry.1);
    // Ten times bigger in the world: ten times fewer pixels a metre.
    let big = look_for(&s, &[0], &image, 10.0, 100.0);
    assert!(kinds(&big).contains(&"blurry"));
    assert!(!kinds(&look_for(&s, &[0], &image, 1.0, 100.0)).contains(&"blurry"));
}

#[test]
fn squashed_uvs_are_stretched() {
    let mut m = PrimitiveKind::Cube.standard().mesh();
    let faces: Vec<_> = m.faces().map(|(f, _)| f).collect();
    mesh::uvs::transform_uvs(
        &mut m,
        &faces,
        &[],
        glam::Affine2::from_scale(Vec2::new(1.0, 0.25)),
    );
    let s = PaintSurface::of(&m);
    let image = painted(&s, |side| [(side * 40) as u8 + 20, 50, 50, 255]);
    let found = look_for(&s, &[0], &image, 1.0, 10.0);
    let Some(Finding::Stretched { faces, worst }) = found
        .iter()
        .find(|f| matches!(f, Finding::Stretched { .. }))
    else {
        panic!("{found:?}");
    };
    assert_eq!(faces.len(), 6);
    assert!((worst - 4.0).abs() < 0.1, "{worst}");
}

#[test]
fn nothing_painted_or_another_material_finds_nothing() {
    let s = cube();
    let blank = Image::new(256).unwrap();
    assert!(look_for(&s, &[0], &blank, 1.0, 0.0).is_empty());
    let image = painted(&s, |_| [120, 100, 80, 255]);
    assert!(look_for(&s, &[5], &image, 1.0, 512.0).is_empty());
}
