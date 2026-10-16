//! Tests for the texture settings and texel density in the window
//! (decision 0100).

use super::modelling_tests::{H, cube, note};
use super::tests::{fresh, quick_harness};
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{FaceId, TextureSettings};

fn density_px(harness: &H, faces: &[FaceId]) -> f32 {
    let doc = harness.state().workbench.document();
    let mesh = doc.scene().evaluated(cube(harness)).unwrap().mesh;
    let uv = uv::density::density(&mesh, faces).unwrap();
    doc.scene().texture().pixels_per_metre(uv)
}

fn faces(harness: &H) -> Vec<FaceId> {
    let doc = harness.state().workbench.document();
    let mesh = doc.scene().evaluated(cube(harness)).unwrap().mesh;
    mesh.faces().map(|(f, _)| f).collect()
}

fn selected_cube() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

#[test]
fn set_texel_density_scales_every_piece_to_the_aim_in_one_step() {
    let mut harness = selected_cube();
    let all = faces(&harness);
    let aim = harness
        .state()
        .workbench
        .document()
        .scene()
        .texture()
        .density;
    assert!(
        (density_px(&harness, &all) - aim).abs() > 1.0,
        "not there yet"
    );
    let steps = harness.state().workbench.document().undo_steps();
    harness.state_mut().workbench.run_texel_density(true);
    harness.run();
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    for f in &all {
        assert!((density_px(&harness, &[*f]) - aim).abs() < 0.5, "{f:?}");
    }
    assert!(note(&harness).contains("px/m"), "{}", note(&harness));
}

#[test]
fn match_texel_density_evens_pieces_out_at_their_average() {
    let mut harness = selected_cube();
    let all = faces(&harness);
    let id = cube(&harness);
    // Make one side twice as dense first.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::TransformUvs {
                faces: vec![all[0]],
                points: Vec::new(),
                transform: glam::Affine2::from_scale(glam::Vec2::splat(2.0)),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    let average = density_px(&harness, &all);
    harness.state_mut().workbench.run_texel_density(false);
    harness.run();
    for f in &all {
        assert!((density_px(&harness, &[*f]) - average).abs() < 0.5);
    }
}

#[test]
fn the_texture_menu_changes_the_size_as_one_undo_step() {
    let mut harness = quick_harness();
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    // The UV Editor's own menu line, and the UV menu's submenu.
    harness
        .get_all_by_label("Texture")
        .max_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .unwrap()
        .click();
    harness.run();
    harness.get_by_label("4096 × 4096 px").click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().texture().size, 4096);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(
        harness.state().workbench.document().scene().texture(),
        TextureSettings::default()
    );
    // UV → Lay out lists the density commands.
    harness.key_press(egui::Key::Escape);
    harness.run();
    super::tests::uv_group(&mut harness, super::UV_LAY_OUT_MENU);
    for label in [
        super::unwrapping::SET_DENSITY_LABEL,
        super::unwrapping::MATCH_DENSITY_LABEL,
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
}

#[test]
fn the_density_line_reads_well() {
    let t = TextureSettings::default();
    assert_eq!(
        crate::uv_editor::density_text(t, Some(498.4), true),
        "2048 px · selected 498 px/m (aim 512)"
    );
    assert_eq!(
        crate::uv_editor::density_text(t, Some(600.0), false),
        "2048 px · 600 px/m (aim 512)"
    );
    assert_eq!(
        crate::uv_editor::density_text(t, None, false),
        "2048 px · aim 512 px/m"
    );
}
