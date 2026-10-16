//! Tests for seeing and checking in the UV Editor (decision 0115):
//! always-on warnings, numbers on pieces and measuring.

use super::LayoutChoice;
use super::modelling_tests::{H, cube, shape};
use super::tests::quick_harness;
use crate::uv_editor::UV_EDITOR_LABEL;
use crate::uv_editor::checking::{MEASURE, PIECE_NUMBERS};
use egui::{Event, PointerButton};
use egui_kittest::kittest::Queryable;
use glam::{Affine2, Vec2};
use scene::Operation;

fn uv_edit_with_the_cube() -> (H, scene::ObjectId) {
    let mut harness = quick_harness();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    (harness, id)
}

/// Opens the UV Editor's own View menu (the one just above it).
fn editor_view_menu(harness: &mut H) {
    let editor = harness.get_by_label(UV_EDITOR_LABEL).rect();
    harness
        .get_all_by_label("View")
        .filter(|n| n.rect().center().x > editor.min.x && n.rect().center().x < editor.max.x)
        .min_by(|a, b| {
            (a.rect().max.y - editor.min.y)
                .abs()
                .total_cmp(&(b.rect().max.y - editor.min.y).abs())
        })
        .unwrap()
        .click();
    harness.run();
}

fn drag(harness: &mut H, from: egui::Pos2, to: egui::Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();
    for k in 1..=8 {
        harness.event(Event::PointerMoved(from.lerp(to, k as f32 / 8.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn the_view_menu_writes_numbers_on_pieces() {
    let (mut harness, _) = uv_edit_with_the_cube();
    assert!(!harness.state().workbench.uv_editor.piece_numbers());
    editor_view_menu(&mut harness);
    harness.get_by_label(PIECE_NUMBERS).click();
    harness.run();
    assert!(harness.state().workbench.uv_editor.piece_numbers());
}

#[test]
fn measuring_a_line_across_a_side_gives_its_real_length() {
    let (mut harness, _) = uv_edit_with_the_cube();
    editor_view_menu(&mut harness);
    harness.get_by_label(MEASURE).click();
    harness.run();
    assert!(harness.state().workbench.uv_editor.measuring());
    // Across one side of the cube, in the texture, from edge to edge.
    let mesh = shape(&harness);
    let island = mesh::uvs::islands(&mesh)[0].clone();
    let (min, max) = mesh::uvs::uv_bounds(&mesh, &island, &[]).unwrap();
    let mid = (min.y + max.y) / 2.0;
    let editor = &harness.state().workbench.uv_editor;
    let inset = (max.x - min.x) * 0.01;
    let from = editor.screen_of(Vec2::new(min.x + inset, mid)).unwrap();
    let to = editor.screen_of(Vec2::new(max.x - inset, mid)).unwrap();
    let steps = harness.state().workbench.document().undo_steps();
    let selected = harness.state().workbench.document().element_revision();
    drag(&mut harness, from, to);
    let ((a, b), cm) = harness.state().workbench.uv_editor.measured().unwrap();
    assert!(a.distance(b) > 0.0);
    // The standard cube is 1 m across.
    assert!((cm - 98.0).abs() < 3.0, "{cm}");
    // Measuring took the drag: nothing was selected or moved.
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps, "no edit");
    assert_eq!(doc.element_revision(), selected, "no selection");
    // Esc over the editor stops it.
    let centre = harness.get_by_label(UV_EDITOR_LABEL).rect().center();
    harness.event(Event::PointerMoved(centre));
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.uv_editor.measuring());
}

#[test]
fn overlapping_and_mirrored_pieces_are_warned_about() {
    let (mut harness, id) = uv_edit_with_the_cube();
    assert_eq!(harness.state().workbench.uv_editor.warning_counts(), (0, 0));
    let mesh = shape(&harness);
    let islands = mesh::uvs::islands(&mesh);
    let centre = |i: usize| mesh::uvs::uv_centre(&mesh, &islands[i], &[]).unwrap();
    let onto = centre(1) - centre(0) + Vec2::splat(0.01);
    let c = centre(2);
    let flip = Affine2::from_translation(c)
        * Affine2::from_scale(Vec2::new(-1.0, 1.0))
        * Affine2::from_translation(-c);
    for (faces, transform) in [
        (islands[0].clone(), Affine2::from_translation(onto)),
        (islands[2].clone(), flip),
    ] {
        harness
            .state_mut()
            .workbench
            .document_mut()
            .execute(scene::Command::Record {
                id,
                operation: Operation::TransformUvs {
                    faces,
                    points: Vec::new(),
                    transform,
                    anchors: None,
                },
            })
            .unwrap();
    }
    harness.run();
    assert_eq!(harness.state().workbench.uv_editor.warning_counts(), (2, 1));
    // Undone, the warnings go.
    harness.state_mut().workbench.document_mut().undo();
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(harness.state().workbench.uv_editor.warning_counts(), (0, 0));
}

#[test]
fn explain_choices_says_why_a_seam_or_piece_is_there() {
    use crate::uv_editor::explain::{EXPLAIN_CHOICES, Pointed};
    let (mut harness, id) = uv_edit_with_the_cube();
    let mesh = shape(&harness);
    let islands = mesh::uvs::islands(&mesh);
    let middle = mesh::uvs::uv_centre(&mesh, &islands[0], &[]).unwrap();
    let at = harness
        .state()
        .workbench
        .uv_editor
        .screen_of(middle)
        .unwrap();
    // Off at first: pointing explains nothing.
    harness.event(Event::PointerMoved(at));
    harness.run();
    assert!(harness.state().workbench.uv_editor.pointed().is_none());
    editor_view_menu(&mut harness);
    harness.get_by_label(EXPLAIN_CHOICES).click();
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().workbench.uv_editor.explaining());
    harness.event(Event::PointerMoved(at + egui::vec2(1.0, 0.0)));
    harness.run();
    let (pointed, text) = harness
        .state()
        .workbench
        .uv_editor
        .pointed()
        .cloned()
        .unwrap();
    assert_eq!(pointed, Pointed::Piece(id, islands[0].clone()));
    assert!(text.contains("px/m"), "{text}");
    // A seam: the piece's bottom edge, half way along.
    let (min, max) = mesh::uvs::uv_bounds(&mesh, &islands[0], &[]).unwrap();
    let seam_at = harness
        .state()
        .workbench
        .uv_editor
        .screen_of(Vec2::new((min.x + max.x) / 2.0, min.y))
        .unwrap();
    harness.event(Event::PointerMoved(seam_at));
    harness.run();
    let (pointed, text) = harness
        .state()
        .workbench
        .uv_editor
        .pointed()
        .cloned()
        .unwrap();
    assert!(
        matches!(pointed, Pointed::Seam(o, _) if o == id),
        "{pointed:?}"
    );
    assert!(text.starts_with("Seam"), "{text}");
    assert!(text.contains("Clear seams"), "{text}");
}
