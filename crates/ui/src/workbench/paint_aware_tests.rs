//! Tests for paint-aware detail and the bake seam preview (decision 0249).

use super::seams_tests::{add, shape};
use super::*;
use crate::OverlayStyle;
use crate::components::bake_seams;

#[test]
fn a_cubes_seams_lie_on_its_hard_edges_so_no_bake_line_shows() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let m = shape(&harness);
    // Hard everywhere by angle: every seam is on a hard edge, and every
    // hard edge that isn't a seam would smear.
    let found = bake_seams(&m, mesh::Hardness::ByAngle(30.0));
    assert!(
        found.iter().all(|(_, s)| *s != OverlayStyle::BakeSeam),
        "{found:?}"
    );
    // All smooth: every seam would show as a line.
    let smooth = bake_seams(&m, mesh::Hardness::ByAngle(180.0));
    assert!(smooth.iter().any(|(_, s)| *s == OverlayStyle::BakeSeam));
    assert!(smooth.iter().all(|(_, s)| *s == OverlayStyle::BakeSeam));
}

#[test]
fn a_shape_without_uvs_has_no_bake_seams() {
    let mut m = scene::Primitive::Cube(Default::default()).mesh();
    m.remove_attribute(mesh::attributes::UV);
    assert!(bake_seams(&m, mesh::Hardness::ByAngle(30.0)).is_empty());
}

#[test]
fn the_display_switch_draws_them() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cylinder(Default::default()));
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .show_bake_seams = true;
    harness.run();
    harness.run();
    let overlay = harness.state().workbench.components.overlay();
    assert!(
        overlay
            .lines
            .iter()
            .any(|(_, s)| matches!(s, OverlayStyle::BakeSeam | OverlayStyle::BakeUnsplit)),
        "a smooth cylinder's side seam or its hard rims show"
    );
}

#[test]
fn pack_gives_worn_pieces_more_texture_only_when_asked() {
    let mut harness = tests::fresh();
    // A tall cylinder: its long side has little rim for its size, its
    // caps a lot.
    let base = mesh::CylinderParams::default();
    add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams {
            height: base.radius * 8.0,
            ..base
        }),
    );
    let area = |h: &egui_kittest::Harness<'static, tests::State>| {
        let m = shape(h);
        let faces: Vec<scene::FaceId> = m.faces().map(|(f, _)| f).collect();
        let uvs = mesh::uvs::Uvs::of(&m).unwrap();
        // The caps: the polygons with more than four corners.
        let caps: Vec<scene::FaceId> = faces
            .iter()
            .copied()
            .filter(|f| m.face(*f).unwrap().len() > 4)
            .collect();
        let (cap, _) = uv::density::areas(&m, &caps, &uvs);
        let (all, _) = uv::density::areas(&m, &faces, &uvs);
        cap / all
    };
    harness.state_mut().workbench.run_pack();
    let plain = area(&harness);
    harness.state_mut().workbench.document_mut().undo();
    harness.state_mut().workbench.uv_settings.wear_texture = true;
    harness.state_mut().workbench.run_pack();
    let worn = area(&harness);
    // The caps' rims are sharp outer edges: the caps get a bigger share.
    assert!(worn > plain * 1.01, "{plain} {worn}");
}

#[test]
fn old_settings_have_the_wear_switch_off() {
    let old: UvSettings = ron::from_str("(padding: 8)").unwrap();
    assert!(!old.wear_texture);
    assert!(!crate::ViewOptions::default().show_bake_seams);
}
