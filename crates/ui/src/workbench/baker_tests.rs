//! Tests for File → Bake… (decision 0250).

use super::baker::{BAKE_BUTTON, BAKER_ENTRY, BAKER_TITLE};
use super::seams_tests::add;
use super::tests::{fresh, main_menu};
use super::*;
use bake::maps::MapKind;
use egui_kittest::kittest::Queryable;

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bake-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_chosen_maps_are_written_into_the_folder() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.document_mut()
        .execute(scene::Command::SetTexture(scene::TextureSettings {
            size: 256,
            ..Default::default()
        }))
        .unwrap();
    w.baker.maps = [true, false, false, false, true, false, true];
    let dir = folder("written");
    w.bake_into(dir.clone());
    assert!(w.baking_maps());
    w.finish_bake();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names.iter().any(|n| n.ends_with("_normal.png")));
    assert!(names.iter().any(|n| n.ends_with("_id.png")));
    assert!(names.iter().any(|n| n.ends_with("_height.png")));
    let normal = names.iter().find(|n| n.ends_with("_normal.png")).unwrap();
    let image = fileio::image_file::open_image(&dir.join(normal)).unwrap();
    assert_eq!((image.width, image.height), (256, 256));
    // Baked from itself: flat everywhere.
    let at = ((128 * 256 + 128) * 4) as usize;
    let p = &image.rgba[at..at + 4];
    assert!(p[2] > 250 && (i32::from(p[1]) - 128).abs() <= 2, "{p:?}");
    assert!(w.baker.last.as_deref().unwrap_or("").contains("Baked 3 maps"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_high_detail_object_is_baked_from() {
    let mut harness = fresh();
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([low]);
    w.baker.high = Some(high);
    let order = w.bake_order().unwrap();
    assert!(order.high.is_some());
    // Choosing the low object itself as the high one bakes from itself.
    w.baker.high = Some(low);
    assert!(w.bake_order().unwrap().high.is_none());
}

#[test]
fn nothing_to_bake_says_why() {
    let mut harness = fresh();
    harness.state_mut().workbench.document_mut().clear_selection();
    assert!(harness.state().workbench.bake_order().is_err());
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.baker.maps = [false; 7];
    let why = harness.state().workbench.bake_order().err().unwrap();
    assert!(why.contains("at least one"), "{why}");
}

#[test]
fn a_bake_can_be_cancelled() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [false, true, false, true, false, false, false];
    let dir = folder("cancelled");
    let order = w.bake_order().unwrap();
    let progress = bake::Progress::default();
    progress.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    let step = std::sync::atomic::AtomicUsize::new(0);
    let result = super::baker::run_order(&order, &dir, &progress, &step);
    assert!(result.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_file_menu_opens_the_window_and_its_button_asks_for_a_folder() {
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    main_menu(&harness, "File").click();
    harness.run();
    harness.get_by_label(BAKER_ENTRY).click();
    harness.run();
    assert!(harness.state().workbench.baker.open);
    assert!(harness.query_by_label_contains(BAKER_TITLE).is_some());
    for kind in MapKind::ALL {
        assert!(harness.query_by_label(super::baker::map_name(kind)).is_some());
    }
    harness.get_by_label(BAKE_BUTTON).click();
    harness.run();
    assert!(harness.state().actions.contains(&WorkbenchAction::ChooseBakeFolder));
}
