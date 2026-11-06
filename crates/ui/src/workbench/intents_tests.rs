//! Tests for UV intents in the window (decision 0248).

use super::intents::{CLEAR_INTENTS_LABEL, INTENTS_LABEL, UPRIGHT_LABEL};
use super::modelling_tests::note;
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;
use mesh::uv_intents::{UvIntent, piece_intent};

fn all_faces(m: &scene::Mesh) -> Vec<scene::FaceId> {
    m.faces().map(|(f, _)| f).collect()
}

/// Every polygon's V range.
fn v_range(m: &scene::Mesh) -> (f32, f32) {
    let uvs = mesh::uvs::Uvs::of(m).unwrap();
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for (f, corners) in m.faces() {
        for k in 0..corners.len() {
            let v = uvs.corner(m, f, k).unwrap().y;
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    (lo, hi)
}

#[test]
fn put_on_trim_row_tells_the_pieces_to_stay_and_an_inset_keeps_them_there() {
    let mut harness = fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Plane(scene::PlaneParams {
            segments: [3, 1],
            ..Default::default()
        }),
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTrim(scene::TrimSheet::even(4)))
        .unwrap();
    let before = steps(&harness);
    harness.state_mut().workbench.run_put_on_trim(1);
    harness.run();
    assert_eq!(steps(&harness), before + 1, "the intent and the placing in one step");
    let row = harness.state().workbench.document().scene().trim().rows[1].clone();
    let m = shape(&harness);
    assert_eq!(
        piece_intent(&m, &all_faces(&m)),
        UvIntent::TrimRow {
            bottom: row.bottom,
            top: row.top
        }
    );
    // Inset a polygon: mending lays its new ring out, and the piece is
    // fitted back into the row.
    let face = all_faces(&m)[1];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::InsetPolygons {
                faces: vec![face],
                distance: 10.0,
                grouping: scene::Grouping::Together,
                stretch: glam::Vec3::ONE,
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    let (lo, hi) = v_range(&shape(&harness));
    assert!(lo >= row.bottom - 1e-3 && hi <= row.top + 1e-3, "{lo} {hi} {row:?}");
}

#[test]
fn straighten_and_density_set_their_intents() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .run_uv_tool(super::unwrapping::UvTool::Straighten);
    let m = shape(&harness);
    assert_eq!(piece_intent(&m, &all_faces(&m)), UvIntent::Strip);
    harness.state_mut().workbench.run_texel_density(true);
    let m = shape(&harness);
    assert!(matches!(piece_intent(&m, &all_faces(&m)), UvIntent::Density(d) if d > 0.0));
}

#[test]
fn keep_this_way_up_and_clear_are_one_step_each() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let before = steps(&harness);
    harness.state_mut().workbench.run_set_intent(UvIntent::Upright);
    assert_eq!(steps(&harness), before + 1);
    let m = shape(&harness);
    assert_eq!(piece_intent(&m, &all_faces(&m)), UvIntent::Upright);
    harness.state_mut().workbench.run_set_intent(UvIntent::None);
    assert_eq!(steps(&harness), before + 2);
    assert!(!mesh::uv_intents::has_intents(&shape(&harness)));
    assert!(note(&harness).contains("Ctrl + Z"), "{}", note(&harness));
}

#[test]
fn nothing_selected_says_so() {
    let mut harness = fresh();
    harness.state_mut().workbench.document_mut().clear_selection();
    harness.state_mut().workbench.run_set_intent(UvIntent::Upright);
    assert!(note(&harness).contains("select objects"), "{}", note(&harness));
}

#[test]
fn the_intents_words_name_each_intent() {
    let texture = scene::TextureSettings::default();
    assert!(intent_words(UvIntent::None, texture).is_none());
    for intent in [
        UvIntent::Strip,
        UvIntent::Density(texture.uv_per_unit()),
        UvIntent::TrimRow { bottom: 0.0, top: 0.25 },
        UvIntent::Upright,
    ] {
        assert!(intent_words(intent, texture).unwrap().starts_with("Keeps"));
    }
}

#[test]
fn the_pieces_menu_offers_the_intents() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    uv_group(&mut harness, "Pieces");
    harness.get_by_label_contains(INTENTS_LABEL).hover();
    harness.run_steps(8);
    assert!(harness.query_by_label(UPRIGHT_LABEL).is_some());
    assert!(harness.query_by_label(CLEAR_INTENTS_LABEL).is_some());
}
