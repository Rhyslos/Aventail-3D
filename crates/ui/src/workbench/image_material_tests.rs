//! From a photo to a material (decision 0284), used through the real
//! interface: an Image box asking for a picture (Attributes, a
//! double-click), the picture handed back as one undo step, the box's
//! plain refusal with nothing brought in, Make into material adding the
//! framed chain as one undo step and making a seamless material, the
//! real-world size row, and the picture saved with the project.

use super::material_mode_tests::{H, box_at, click, graph, quick_page, undo};
use super::*;
use crate::material::from_photo::{BRING_IN_LABEL, FRAME_TITLE_TEXT, MAKE_LABEL};
use crate::material::look::COLUMN;
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::boxes::image_box::IMAGE;
use matgraph::{BoxId, GraphEdit, GraphId, Photo, PlugRef, SeamCheck, Value};

/// The Image box added to the page's graph.
const BOX: BoxId = BoxId(6);

/// The page with an Image box beside its boxes, selected, nothing
/// brought in yet.
fn with_image_box() -> H {
    let mut harness = quick_page();
    let edit = GraphEdit::AddBox {
        kind: IMAGE.into(),
        at: Vec2::new(4.0 * COLUMN, 0.0),
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit,
        })
        .unwrap();
    harness.state_mut().workbench.material().canvas.selected = [BOX].into();
    harness.run();
    harness.run();
    harness
}

/// A photo that doesn't tile, lit from the left, brought in as a file
/// would be (a BMP, written here).
fn wall_photo() -> Photo {
    fileio::image_file::read_photo(&bmp(160, 120), "wall.png").unwrap()
}

/// A 24-bit BMP file `w` × `h` of lumpy ground lit from the left:
/// bumps of light and dark, getting lighter to the right.
fn bmp(w: u32, h: u32) -> Vec<u8> {
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut b = Vec::with_capacity(size as usize);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]);
    // Rows from the bottom, blue green red.
    for y in (0..h).rev() {
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let bumps = (fx * 0.37 + fy * 0.21).sin() * (fy * 0.29 - fx * 0.13).cos();
            let light = 0.4 + 0.6 * fx / w as f32;
            let v = (140.0 + 80.0 * bumps) * light;
            b.extend_from_slice(&[(v * 0.8) as u8, (v * 0.9) as u8, v as u8]);
        }
        b.resize(b.len() + (row - w * 3) as usize, 0);
    }
    b
}

fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

fn asked(harness: &H) -> bool {
    harness
        .state()
        .actions
        .contains(&WorkbenchAction::ChoosePhoto { path: None })
}

/// The Image box with the wall photo on it, brought in through
/// Attributes.
fn with_photo() -> H {
    let mut harness = with_image_box();
    harness.get_by_label(BRING_IN_LABEL).click();
    harness.run();
    harness.state_mut().workbench.set_box_photo(wall_photo());
    harness.run();
    harness.run();
    harness
}

#[test]
fn attributes_asks_for_a_picture_and_it_comes_back_as_one_undo_step() {
    let mut harness = with_image_box();
    // Nothing brought in: the box says why it gives nothing.
    assert!(
        harness
            .get_all_by_label(matgraph::boxes::image_box::NO_PICTURE)
            .next()
            .is_some()
    );
    harness.get_by_label(BRING_IN_LABEL).click();
    harness.run();
    assert!(asked(&harness));
    harness.state_mut().workbench.set_box_photo(wall_photo());
    harness.run();
    let photo = graph(&harness).get(BOX).unwrap().photo.clone();
    assert_eq!(photo.as_deref().map(Photo::name), Some("wall.png"));
    assert!(note(&harness).contains("wall.png"), "{}", note(&harness));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Bring in a picture")
    );
    undo(&mut harness);
    assert!(graph(&harness).get(BOX).unwrap().photo.is_none());
    // Handed a picture nobody asked for: it says where pictures go.
    harness.state_mut().workbench.set_box_photo(wall_photo());
    assert!(note(&harness).contains("Image box"), "{}", note(&harness));
    assert!(graph(&harness).get(BOX).unwrap().photo.is_none());
}

#[test]
fn a_double_click_on_the_box_asks_for_a_picture() {
    let mut harness = with_image_box();
    let (rect, _) = box_at(&mut harness, BOX);
    let spot = rect.left_top() + egui::vec2(30.0, 8.0);
    click(&mut harness, spot, Modifiers::NONE);
    click(&mut harness, spot, Modifiers::NONE);
    assert!(asked(&harness));
}

#[test]
fn make_into_material_adds_the_framed_chain_in_one_undo_step() {
    let mut harness = with_photo();
    let boxes_before = graph(&harness).box_count();
    // The seam check says the photo's edges don't match.
    let seam = harness
        .query_all_by_label_contains("The seam check:")
        .next()
        .map(|n| format!("{n:?}"));
    assert!(seam.is_some(), "the seam check is shown");
    harness.get_by_label(MAKE_LABEL).click();
    harness.run();
    harness.run();
    let g = graph(&harness);
    // Even out light, Make seamless, Guess height, Guess roughness,
    // Material from height and Material out.
    assert_eq!(g.box_count(), boxes_before + 6);
    let frame = g
        .frames()
        .find(|(_, f)| f.title == FRAME_TITLE_TEXT)
        .map(|(id, _)| id)
        .expect("a frame round the chain");
    let inside = g.boxes_in(frame);
    assert!(inside.contains(&BOX));
    assert_eq!(inside.len(), 7, "{inside:?}");
    let out = g.first_of("material_out").unwrap();
    assert!(g.wire_into(&PlugRef::new(out, "material")).is_some());
    let made = g.first_of("material_from_height").unwrap();
    assert!(g.wire_into(&PlugRef::new(made, "roughness")).is_some());
    // The material it makes repeats without a join.
    let registry = harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .registry()
        .clone();
    let material = matgraph::Evaluator::with_threads(2)
        .material(&registry, &g, matgraph::Size::new(64).unwrap())
        .unwrap();
    let check = SeamCheck::of(&Value::Material(material));
    assert!(!check.shows(), "{}", check.describe());
    // One undo step takes it all away.
    undo(&mut harness);
    let g = graph(&harness);
    assert_eq!(g.box_count(), boxes_before);
    assert_eq!(g.frames().count(), 0);
    assert!(g.get(BOX).unwrap().photo.is_some());
}

#[test]
fn with_a_material_out_already_wired_the_chain_is_left_beside_it() {
    let mut harness = with_photo();
    harness.get_by_label(MAKE_LABEL).click();
    harness.run();
    // A second time: Material out already has a material.
    harness.state_mut().workbench.material().canvas.selected = [BOX].into();
    harness.run();
    let outs_before = graph(&harness)
        .boxes()
        .filter(|(_, b)| b.kind == "material_out")
        .count();
    harness.get_by_label(MAKE_LABEL).click();
    harness.run();
    let g = graph(&harness);
    let outs = g.boxes().filter(|(_, b)| b.kind == "material_out").count();
    assert_eq!((outs_before, outs), (1, 1));
    assert!(note(&harness).contains("already"), "{}", note(&harness));
}

#[test]
fn the_real_world_size_row_shows_once_there_is_a_material_out() {
    let mut harness = with_photo();
    assert!(
        harness
            .query_all_by_label_contains("set on Material out")
            .next()
            .is_some()
    );
    harness.get_by_label(MAKE_LABEL).click();
    harness.run();
    harness.state_mut().workbench.material().canvas.selected = [BOX].into();
    harness.run();
    assert!(
        harness
            .query_all_by_label_contains("measure something in the photo")
            .next()
            .is_some()
    );
}

#[test]
fn a_picture_is_saved_with_the_project() {
    let harness = with_photo();
    let bytes = fileio::project::to_bytes(harness.state().workbench.document()).unwrap();
    let back = fileio::project::from_bytes(&bytes).unwrap();
    let kept = back
        .scene()
        .graphs()
        .get(GraphId(1))
        .unwrap()
        .get(BOX)
        .unwrap()
        .photo
        .clone();
    assert_eq!(kept, graph(&harness).get(BOX).unwrap().photo.clone());
}
