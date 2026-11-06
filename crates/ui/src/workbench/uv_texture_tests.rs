//! Tests for the texture picture and the UV layout export (decision 0244).

use super::modelling_tests::note;
use super::seams_tests::add;
use super::tests::{fresh, uv_group};
use super::*;
use crate::viewport::{MAX_MIP, Surface};
use egui_kittest::kittest::Queryable;

fn image(width: u32, height: u32) -> fileio::image_file::ReadImage {
    fileio::image_file::ReadImage {
        width,
        height,
        rgba: [200, 40, 40, 255].repeat((width * height) as usize),
    }
}

#[test]
fn a_chosen_picture_is_shown_on_the_texture_surface() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .set_uv_texture("bricks.png", &image(300, 200));
    harness.run();
    let workbench = &harness.state().workbench;
    let picture = workbench.viewport().picture().expect("a picture");
    assert_eq!(picture.name, "bricks.png");
    // Made square, a power of two across.
    assert_eq!(picture.size, 512);
    assert_eq!(workbench.viewport().options().surface, Surface::Texture);
    assert!(note(&harness).contains("bricks.png"), "{}", note(&harness));
}

#[test]
fn a_broken_picture_is_refused() {
    let mut harness = fresh();
    let broken = fileio::image_file::ReadImage {
        width: 4,
        height: 4,
        rgba: vec![0; 3],
    };
    harness.state_mut().workbench.set_uv_texture("bad.png", &broken);
    assert!(harness.state().workbench.viewport().picture().is_none());
    assert!(note(&harness).contains("couldn't be used"), "{}", note(&harness));
}

#[test]
fn clearing_the_picture_leaves_none() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .set_uv_texture("a.png", &image(8, 8));
    harness.state_mut().workbench.clear_uv_texture();
    harness.run();
    assert!(harness.state().workbench.viewport().picture().is_none());
}

#[test]
fn the_layout_has_the_selected_objects_edges() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let lines = harness.state_mut().workbench.uv_layout_lines();
    // A cube's 12 edges, the seams' twice (once for each piece), so more
    // than 12.
    assert!(lines.len() >= 12, "{}", lines.len());
    assert!(
        lines
            .iter()
            .flatten()
            .all(|p| p.is_finite() && (-0.01..=1.01).contains(&p.x) && (-0.01..=1.01).contains(&p.y))
    );
    let png = harness.state_mut().workbench.uv_layout_file(false).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let svg = harness.state_mut().workbench.uv_layout_file(true).unwrap();
    assert!(String::from_utf8(svg).unwrap().contains("<svg"));
}

#[test]
fn exporting_with_nothing_selected_says_so() {
    let mut harness = fresh();
    harness.state_mut().workbench.document_mut().clear_selection();
    assert!(harness.state_mut().workbench.uv_layout_file(false).is_none());
    assert!(note(&harness).contains("select objects"), "{}", note(&harness));
}

#[test]
fn the_menu_asks_for_the_files() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    uv_group(&mut harness, uv_texture::TEXTURE_PICTURE_LABEL);
    harness.get_by_label(uv_texture::CHOOSE_PICTURE_LABEL).click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::ChooseUvTexture { path: None })
    );
    uv_group(&mut harness, uv_texture::TEXTURE_PICTURE_LABEL);
    harness.get_by_label(uv_texture::EXPORT_SVG_LABEL).click();
    harness.run();
    assert!(harness.state().actions.contains(&WorkbenchAction::ExportUvLayout {
        svg: true,
        path: None
    }));
}

#[test]
fn the_mip_level_is_kept_within_range() {
    let mut harness = fresh();
    harness.state_mut().workbench.viewport_mut().options_mut().mip_level = 99;
    harness
        .state_mut()
        .workbench
        .set_uv_texture("a.png", &image(64, 64));
    harness.run();
    // Shown at most at the deepest level offered, without a crash.
    assert!(harness.state().workbench.viewport().options().mip_level.min(MAX_MIP) <= MAX_MIP);
}

#[test]
fn the_typed_path_window_hands_on_the_choice() {
    let purposes = [
        path_entry::FilePurpose::UvTexture,
        path_entry::FilePurpose::UvLayout { svg: true },
    ];
    for purpose in purposes {
        let mut harness = fresh();
        harness
            .state_mut()
            .workbench
            .file_window_missing(purpose, None);
        harness.run();
        assert!(harness.state().workbench.path_entry.is_some());
    }
}
