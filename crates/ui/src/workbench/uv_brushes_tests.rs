//! Tests for the UV brushes in the window (decision 0238).

use super::modelling_tests::{H, note, press};
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;
use uv::brush::UvBrushKind;

fn grid() -> H {
    let mut harness = fresh();
    add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [4, 4],
            ..Default::default()
        }),
    );
    harness
}

fn uvs(harness: &H) -> Vec<glam::Vec2> {
    let mesh = shape(harness);
    mesh::uvs::Uvs::of(&mesh)
        .unwrap()
        .into_values(mesh.face_layout().1.len())
}

#[test]
fn a_grab_stroke_moves_the_uvs_in_one_undo_step() {
    let mut harness = grid();
    let before_uvs = uvs(&harness);
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    w.start_uv_brush(UvBrushKind::Grab);
    assert_eq!(w.uv_brush(), Some(UvBrushKind::Grab));
    w.begin_uv_stroke(glam::Vec2::splat(0.5), false);
    for k in 1..=5 {
        w.uv_dab(glam::Vec2::new(0.5 + 0.01 * k as f32, 0.5), 0.3);
    }
    harness.run();
    assert_ne!(uvs(&harness), before_uvs);
    assert_eq!(steps(&harness), before + 1, "one step per stroke");
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(uvs(&harness), before_uvs);
}

#[test]
fn ctrl_pinch_pushes_and_a_stroke_off_the_uvs_records_nothing() {
    let mut harness = grid();
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    w.start_uv_brush(UvBrushKind::Pinch);
    w.begin_uv_stroke(glam::Vec2::splat(5.0), true);
    w.uv_dab(glam::Vec2::splat(5.0), 0.1);
    assert_eq!(w.document().undo_steps(), before, "nothing under the brush");
    w.begin_uv_stroke(glam::Vec2::splat(0.5), true);
    w.uv_dab(glam::Vec2::splat(0.5), 0.4);
    assert_eq!(w.document().undo_steps(), before + 1);
}

#[test]
fn brackets_size_the_brush_and_esc_puts_it_down() {
    let mut harness = grid();
    harness
        .state_mut()
        .workbench
        .start_uv_brush(UvBrushKind::Relax);
    harness.run();
    let r = harness.state().workbench.uv_brushing.as_ref().unwrap().radius;
    press(&mut harness, Modifiers::NONE, Key::CloseBracket);
    assert!(harness.state().workbench.uv_brushing.as_ref().unwrap().radius > r);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(harness.state().workbench.uv_brush().is_none());
    assert!(note(&harness).contains("put down"), "{}", note(&harness));
    // Esc only put the brush down.
    assert!(!harness.state().workbench.document().selection().is_empty());
}

#[test]
fn the_brush_and_the_lattice_take_turns() {
    let mut harness = grid();
    let w = &mut harness.state_mut().workbench;
    w.start_uv_brush(UvBrushKind::Grab);
    w.start_lattice();
    assert!(w.uv_brush().is_none() && w.warping());
    w.start_uv_brush(UvBrushKind::Grab);
    assert!(!w.warping());
}

#[test]
fn the_uv_menu_offers_the_brushes() {
    let mut harness = grid();
    uv_group(&mut harness, super::UV_PIECES_MENU);
    harness
        .get_by_label_contains(uv_brushes::UV_BRUSH_MENU)
        .hover();
    harness.run_steps(8);
    for kind in UvBrushKind::ALL {
        assert!(harness.query_by_label(kind.name()).is_some(), "{kind:?}");
    }
}
