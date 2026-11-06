//! Tests for UV selection, isolating, the editor's own selection and
//! copying UVs, in the window (decision 0239).

use super::modelling_tests::{H, note, press};
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::uv_select::UvSelect;
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection};

fn cube(harness: &mut H) -> scene::ObjectId {
    add(harness, scene::Primitive::Cube(Default::default()))
}

fn select_polygons(harness: &mut H, id: scene::ObjectId, faces: &[scene::FaceId]) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                ElementSelection::of(faces.iter().copied().map(Element::Polygon)),
            )],
            "Select",
            None,
        );
    harness.run();
}

fn selected_polygons(harness: &H, id: scene::ObjectId) -> usize {
    harness
        .state()
        .workbench
        .document()
        .elements(id)
        .map_or(0, |s| s.polygons().count())
}

#[test]
fn similar_pieces_of_a_cube_are_all_its_sides() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = shape(&harness).faces().next().unwrap().0;
    select_polygons(&mut harness, id, &[first]);
    harness
        .state_mut()
        .workbench
        .run_uv_select(UvSelect::Similar);
    harness.run();
    assert!(selected_polygons(&harness, id) >= 2, "{}", note(&harness));
    assert_eq!(harness.state().workbench.mode, Mode::Polygons);
}

#[test]
fn nothing_overlapping_says_so_and_selects_nothing() {
    let mut harness = fresh();
    cube(&mut harness);
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_select(UvSelect::Overlapping);
    assert!(
        note(&harness).contains("nothing found"),
        "{}",
        note(&harness)
    );
    assert_eq!(steps(&harness), before);
}

#[test]
fn the_path_needs_two_points() {
    let mut harness = fresh();
    cube(&mut harness);
    harness.state_mut().workbench.run_uv_select(UvSelect::Path);
    assert!(note(&harness).contains("two points"), "{}", note(&harness));
}

#[test]
fn copy_uvs_lays_the_other_sides_out_like_the_first() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let all: Vec<scene::FaceId> = shape(&harness).faces().map(|(f, _)| f).collect();
    select_polygons(&mut harness, id, &all);
    let before = steps(&harness);
    harness.state_mut().workbench.run_copy_uvs();
    harness.run();
    assert_eq!(steps(&harness), before + 1, "{}", note(&harness));
    assert!(note(&harness).contains("Copy UVs to"), "{}", note(&harness));
    // With one piece selected, there's nothing to copy to.
    select_polygons(&mut harness, id, &all[..1]);
    harness.state_mut().workbench.run_copy_uvs();
    assert!(
        note(&harness).contains("two or more pieces"),
        "{}",
        note(&harness)
    );
}

#[test]
fn the_editor_can_keep_its_own_selection() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = shape(&harness).faces().next().unwrap().0;
    select_polygons(&mut harness, id, &[first]);
    let w = &mut harness.state_mut().workbench;
    w.uv_editor.set_own_selection(true, &w.document);
    assert!(w.uv_editor.own_selection());
    // It starts as the viewport's.
    assert_eq!(w.uv_editor.selected(&w.document, id).polygons().count(), 1);
    w.uv_editor.set_own_selection(false, &w.document);
    assert!(!w.uv_editor.own_selection());
}

#[test]
fn isolating_is_a_view_setting() {
    let mut harness = fresh();
    cube(&mut harness);
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    w.uv_editor.set_isolating(true, &w.document);
    harness.run();
    assert!(harness.state().workbench.uv_editor.isolating());
    assert_eq!(steps(&harness), before, "nothing recorded");
}

#[test]
fn the_uv_menu_has_the_select_group() {
    let mut harness = fresh();
    cube(&mut harness);
    uv_group(&mut harness, uv_select::UV_SELECT_MENU);
    for what in UvSelect::ALL {
        assert!(harness.query_by_label(what.name()).is_some(), "{what:?}");
    }
    for label in [
        uv_select::ISOLATE_LABEL,
        uv_select::SYNC_LABEL,
        uv_select::COPY_UVS_LABEL,
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
}
