//! Tests for the UV Editor's numbers, warnings and measuring texts
//! (decision 0115).

use super::*;
use scene::PrimitiveKind;
use std::sync::Arc;

fn shape(kind: PrimitiveKind) -> UvShape {
    UvShape::of(Arc::new(kind.standard().mesh())).unwrap()
}

#[test]
fn each_piece_gets_its_box_density_and_stretch() {
    let cube = shape(PrimitiveKind::Cube);
    let numbers = piece_numbers(&cube);
    assert_eq!(numbers.len(), 6);
    let first = &numbers[0].density.unwrap();
    for n in &numbers {
        assert!(n.max.x > n.min.x && n.max.y > n.min.y);
        // A cube's sides are laid out at the same density, unstretched.
        assert!((n.density.unwrap() - first).abs() < first * 0.01);
        assert!((n.stretch - 1.0).abs() < 0.01, "{}", n.stretch);
    }
}

#[test]
fn the_numbers_read_plainly() {
    let texture = scene::TextureSettings::default();
    let n = PieceNumbers {
        min: Vec2::ZERO,
        max: Vec2::new(0.25, 0.125),
        density: Some(texture.uv_per_unit()),
        stretch: 1.0,
    };
    let text = numbers_text(&n, texture);
    let size = texture.size as f32;
    assert!(
        text.starts_with(&format!("{} × {} px", size / 4.0, size / 8.0)),
        "{text}"
    );
    assert!(
        text.contains(&format!("{} px/m", texture.density.round())),
        "{text}"
    );
    assert!(!text.contains("stretched"));
    let stretched = numbers_text(
        &PieceNumbers {
            stretch: 1.5,
            density: None,
            ..n
        },
        texture,
    );
    assert!(stretched.contains("stretched 1.5×"), "{stretched}");
    assert!(!stretched.contains("px/m"));
}

#[test]
fn measures_and_warnings_read_plainly() {
    assert_eq!(
        measure_text(127.6, 12.34),
        "128 px · 12.3 cm on the surface"
    );
    assert_eq!(measure_text(40.0, 0.0), "40 px · not over a piece");
    assert_eq!(warning_text(0, 0), None);
    assert_eq!(warning_text(1, 0).unwrap(), "1 piece overlapping");
    assert_eq!(
        warning_text(2, 3).unwrap(),
        "2 pieces overlapping · 3 pieces mirrored"
    );
}

#[test]
fn switches_start_off() {
    let mut editor = UvEditor::default();
    assert!(!editor.piece_numbers() && !editor.measuring());
    editor.set_piece_numbers(true);
    editor.set_measuring(true);
    assert!(editor.piece_numbers() && editor.measuring());
    assert!(editor.measured().is_none());
    editor.set_measuring(false);
    assert!(!editor.measuring());
}
