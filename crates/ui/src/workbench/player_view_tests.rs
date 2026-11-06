//! Tests for player-view UVs (decision 0247).

use super::modelling_tests::note;
use super::player_view::{PLAYER_VIEW_LABEL, PlayerChoice};
use super::seams_tests::{add, shape};
use super::unwrapping::SeamHiding;
use super::*;
use std::sync::Arc;

fn the_object(harness: &egui_kittest::Harness<'static, tests::State>) -> scene::ObjectId {
    harness.state().workbench.document().selection()[0]
}

#[test]
fn suggest_seams_hides_cuts_where_the_player_doesnt_look() {
    let mut harness = fresh_cylinder();
    harness.state_mut().workbench.uv_settings.seam_hiding = SeamHiding::Player;
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    let m = shape(&harness);
    let seams = mesh::uvs::seams(&m);
    assert!(!seams.is_empty(), "{}", note(&harness));
    // The player view never left a layer on the model.
    assert!(m.attribute_values(uv::seams::PLAYER_SEEN).is_none());
}

fn fresh_cylinder() -> egui_kittest::Harness<'static, tests::State> {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cylinder(Default::default()));
    harness
}

#[test]
fn the_seen_shapes_are_kept_until_the_shape_or_the_view_changes() {
    let mut harness = fresh_cylinder();
    let id = the_object(&harness);
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    let w = &mut harness.state_mut().workbench;
    let a = w.player_seen(id, &mesh);
    let b = w.player_seen(id, &mesh);
    assert!(Arc::ptr_eq(&a, &b));
    w.uv_settings.player = PlayerChoice::TopDown;
    let c = w.player_seen(id, &mesh);
    assert!(!Arc::ptr_eq(&a, &c));
    assert_eq!(a.len(), mesh.face_count());
}

#[test]
fn pack_scales_pieces_by_the_player_only_when_asked() {
    let mut harness = fresh_cylinder();
    let id = the_object(&harness);
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    let bottom: Vec<scene::FaceId> = mesh
        .faces()
        .filter(|(f, _)| mesh.face_normal(*f).z < -0.9)
        .map(|(f, _)| f)
        .collect();
    let w = &mut harness.state_mut().workbench;
    assert_eq!(w.player_scale(id, &mesh, &bottom), 1.0);
    w.uv_settings.player_texture = true;
    // Never seen: half as much texture across.
    assert_eq!(w.player_scale(id, &mesh, &bottom), 0.5);
    w.run_pack();
    assert!(note(&harness).contains("Pack"), "{}", note(&harness));
}

#[test]
fn the_view_follows_the_settings_and_odd_distances_are_kept_sane() {
    let mut harness = tests::fresh();
    let w = &mut harness.state_mut().workbench;
    assert_eq!(w.player_view().kind, bake::player::PlayerKind::ThirdPerson);
    w.uv_settings.player_distance = f32::NAN;
    assert_eq!(w.player_view().distance, 3.0);
    w.uv_settings.player_distance = -5.0;
    assert!(w.player_view().distance > 0.0);
}

#[test]
fn the_uv_editor_colours_texels_per_screen_pixel() {
    let mut harness = fresh_cylinder();
    harness
        .state_mut()
        .workbench
        .uv_editor
        .set_player_texels(true);
    harness.run();
    let id = the_object(&harness);
    assert!(
        harness
            .state()
            .workbench
            .player_seen
            .kept()
            .contains_key(&id)
    );
}

#[test]
fn old_settings_read_the_player_view_as_standard() {
    let old: UvSettings = ron::from_str("(padding: 8)").unwrap();
    assert_eq!(old.player, PlayerChoice::ThirdPerson);
    assert_eq!(old.player_distance, 3.0);
    assert!(!old.player_texture);
    assert!(!PLAYER_VIEW_LABEL.is_empty());
}
