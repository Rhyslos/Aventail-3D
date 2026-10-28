//! Textures on materials (decision 0177): images imported as paint layers,
//! the Painted surface, and what the views are given.

use super::tests::{State, fresh};
use crate::viewport::Surface;
use egui_kittest::kittest::Queryable;
use std::path::Path;

type H = egui_kittest::Harness<'static, State>;

fn picture(width: u32, height: u32, rgba: [u8; 4]) -> fileio::image_file::ReadImage {
    fileio::image_file::ReadImage {
        width,
        height,
        rgba: (0..width * height).flat_map(|_| rgba).collect(),
    }
}

fn cube_selected() -> H {
    let mut harness = fresh();
    let id = super::modelling_tests::cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
}

#[test]
fn an_imported_image_becomes_a_paint_layer_and_shows_painted() {
    let mut harness = cube_selected();
    let red = picture(4, 4, [220, 30, 20, 255]);
    harness.state_mut().workbench.import_paint_image(
        Path::new("/pictures/rust.png"),
        "Default",
        &red,
    );
    harness.run();
    let workbench = &harness.state().workbench;
    let set = workbench.document().scene().paint().set("Default").unwrap();
    assert_eq!(set.size, workbench.document().scene().texture().size);
    assert_eq!(set.layers.len(), 1);
    assert_eq!(set.layers[0].name, "rust");
    assert_eq!(
        set.layers[0].image().unwrap().pixel(10, 10),
        [220, 30, 20, 255]
    );
    assert_eq!(workbench.viewport.options().surface, Surface::Painted);
    // The views are given the texture, and the cube's slot its key.
    let drawing = harness.state().renderer.drawn.last().unwrap().scene.clone();
    assert_eq!(drawing.textures.len(), 1);
    let key = crate::paint_view::texture_key("Default");
    assert_eq!(drawing.textures[0].key, key);
    assert_eq!(drawing.items[0].textures, vec![key]);
    // A second image goes on top, with its own name.
    harness.state_mut().workbench.import_paint_image(
        Path::new("/pictures/rust.png"),
        "Default",
        &picture(1, 1, [0, 0, 255, 128]),
    );
    let set = harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .set("Default")
        .unwrap()
        .clone();
    assert_eq!(set.layers.len(), 2);
    assert_eq!(set.layers[1].name, "rust 2");
    // One undo step each.
    let doc = harness.state_mut().workbench.document_mut();
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert!(doc.scene().paint().is_empty());
}

#[test]
fn the_image_button_asks_for_a_picture_for_that_material() {
    let mut harness = cube_selected();
    harness
        .get_by_label("Import an image onto Default")
        .scroll_to_me();
    harness.run();
    harness.get_by_label("Import an image onto Default").click();
    harness.run();
    let asked = harness.state().actions.iter().any(|a| {
        matches!(a, super::WorkbenchAction::ImportPaintImage { material, path: None } if material == "Default")
    });
    assert!(asked);
}

#[test]
fn the_painted_surface_is_offered_in_the_display_menu() {
    assert!(Surface::ALL.contains(&Surface::Painted));
    assert_eq!(Surface::Painted.label(), "Painted");
}

#[test]
fn the_uv_editor_shows_the_painted_texture_behind_the_uvs() {
    let mut harness = cube_selected();
    harness.state_mut().workbench.import_paint_image(
        Path::new("/pictures/grass.png"),
        "Default",
        &picture(2, 2, [40, 200, 60, 255]),
    );
    harness
        .state_mut()
        .workbench
        .switch_layout(super::LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    assert!(harness.state().workbench.uv_editor.shows_painted());
    // Another surface: the numbered checker or plain square again.
    harness.state_mut().workbench.viewport.options_mut().surface = Surface::Plain;
    harness.run();
    let before = harness.state().workbench.uv_editor.shows_painted();
    assert!(
        before,
        "the last painted image is kept for when it's shown again"
    );
}
