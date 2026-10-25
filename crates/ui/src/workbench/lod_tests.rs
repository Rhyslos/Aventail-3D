//! Tests for levels of detail in the window (decisions 0155 and 0165):
//! the Levels panel, the preview in the viewport, and Make levels of
//! detail in command search.

use super::modelling_tests::{H, cube};
use super::tests::fresh;
use crate::objects::DocRequest;
use egui_kittest::kittest::{NodeT, Queryable};
use mesh::decimate::triangle_count;
use scene::{Lods, ObjectId};

/// Runs the window until no levels are made elsewhere.
fn settle(harness: &mut H) {
    harness.run_steps(1);
    for _ in 0..600 {
        if !harness.state().workbench.drawing.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

/// The window with the cube selected and the Levels panel in front.
fn selected_cube() -> (H, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    crate::layout::show_levels(&mut harness.state_mut().workbench.dock);
    harness.run();
    (harness, id)
}

/// Clicks the widget named `name` with `role` (its row's label has the
/// same name).
fn click(harness: &mut H, name: &str, role: egui::accesskit::Role) {
    for scroll in [true, false] {
        {
            let node = harness
                .get_all_by_label(name)
                .find(|n| n.accesskit_node().role() == role)
                .unwrap_or_else(|| panic!("no {role:?} {name}"));
            if scroll {
                node.scroll_to_me();
            } else {
                node.click();
            }
        }
        if scroll {
            // Long enough for the scroll to finish moving.
            harness.run_steps(30);
        }
    }
}

/// Clicks the only widget named `name`, scrolled into view first.
fn press(harness: &mut H, name: &str) {
    press_nth(harness, name, 0);
}

/// Clicks widget `n` (from 0) of those named `name`, scrolled into view
/// first.
fn press_nth(harness: &mut H, name: &str, n: usize) {
    harness
        .get_all_by_label(name)
        .nth(n)
        .unwrap_or_else(|| panic!("no {name} {n}"))
        .scroll_to_me();
    // Long enough for the scroll to finish moving.
    harness.run_steps(30);
    harness
        .get_all_by_label(name)
        .nth(n)
        .unwrap_or_else(|| panic!("no {name} {n}"))
        .click();
}

fn lods(harness: &H, id: ObjectId) -> Lods {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .lods
        .clone()
}

#[test]
fn make_levels_shows_each_levels_settings_and_triangles() {
    let (mut harness, id) = selected_cube();
    assert_eq!(harness.query_all_by_label("Level 1").count(), 0);
    click(&mut harness, "Make levels", egui::accesskit::Role::CheckBox);
    settle(&mut harness);
    assert!(lods(&harness, id).on);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change the levels of detail")
    );
    for level in ["Level 1", "Level 2", "Level 3"] {
        assert!(harness.query_all_by_label(level).count() >= 1, "{level}");
    }
    // Made at once for a light shape, so its counts and the full shape's
    // are shown.
    assert!(harness.query_all_by_label_contains("12 triangles").count() >= 1);
    press(&mut harness, "Increase Levels");
    settle(&mut harness);
    assert_eq!(lods(&harness, id).levels.len(), 4);
    assert!(harness.query_all_by_label("Level 4").count() >= 1);
    press(&mut harness, "Decrease Level 1 reduced to");
    settle(&mut harness);
    assert!((lods(&harness, id).levels[0].ratio - 0.49).abs() < 1e-4);
    // A budget, and the count against it.
    press(&mut harness, "Increase Budget");
    settle(&mut harness);
    assert!(lods(&harness, id).levels.iter().any(|l| l.budget == 100));
    assert!(
        harness
            .query_by_label_contains("of 100 triangles")
            .is_some()
    );
}

#[test]
fn preview_draws_the_level_and_undo_brings_the_full_shape_back() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Sphere.standard(),
        )))
        .unwrap();
    crate::layout::show_levels(&mut harness.state_mut().workbench.dock);
    harness.run();
    let id = harness.state().workbench.document().selection()[0];
    let full = drawn(&mut harness, 0);
    click(&mut harness, "Make levels", egui::accesskit::Role::CheckBox);
    settle(&mut harness);
    press(&mut harness, "Preview Level 3");
    settle(&mut harness);
    assert_eq!(lods(&harness, id).preview, 3);
    assert!(drawn(&mut harness, 0) < full / 4, "the simplest level");
    harness.state_mut().workbench.document_mut().undo().unwrap();
    settle(&mut harness);
    assert_eq!(drawn(&mut harness, 0), full);
}

/// The triangles drawn for item `at`.
fn drawn(harness: &mut H, at: usize) -> usize {
    let workbench = &mut harness.state_mut().workbench;
    let summary = workbench
        .drawing
        .update(&workbench.document, None, &workbench.hidden);
    triangle_count(&summary.drawing.items[at].drawn)
}

/// The sphere with Subdivide and levels on, selected, the Levels panel in
/// front.
fn smoothed_sphere() -> (H, ObjectId) {
    let mut harness = fresh();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(scene::Command::AddModifier {
        ids: vec![id],
        kind: scene::ModifierKind::Subdivide(scene::SubdivideOptions::default()),
    })
    .unwrap();
    doc.execute(scene::Command::SetLods {
        ids: vec![id],
        lods: Lods {
            on: true,
            ..Lods::default()
        },
    })
    .unwrap();
    crate::layout::show_levels(&mut harness.state_mut().workbench.dock);
    settle(&mut harness);
    (harness, id)
}

#[test]
fn a_modifier_switched_off_at_one_level_changes_only_that_level() {
    let (mut harness, id) = smoothed_sphere();
    click(
        &mut harness,
        "Subdivide at Level 1",
        egui::accesskit::Role::CheckBox,
    );
    settle(&mut harness);
    let level = &lods(&harness, id).levels[0];
    assert_eq!(level.overrides.len(), 1);
    assert!(!level.overrides[0].on);
    assert!(lods(&harness, id).levels[1].overrides.is_empty());
    assert!(harness.query_by_label("changed here").is_some());
    press(&mut harness, "Same as the model");
    settle(&mut harness);
    assert!(lods(&harness, id).levels[0].overrides.is_empty());
    // Undo goes back one change at a time.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    settle(&mut harness);
    assert_eq!(lods(&harness, id).levels[0].overrides.len(), 1);
}

#[test]
fn a_level_made_by_hand_is_edited_in_the_objects_place_with_a_banner() {
    let (mut harness, id) = smoothed_sphere();
    press_nth(&mut harness, crate::attributes::MAKE_BY_HAND, 1);
    settle(&mut harness);
    let doc = harness.state().workbench.document();
    let (shape, _) = doc.scene().level_shape(id, 1).expect("made by hand");
    assert_eq!(doc.selection(), [shape], "edited at once");
    // The viewport shows the level's shape and not the object, with the
    // banner.
    let ids = |harness: &mut H| -> Vec<ObjectId> {
        let workbench = &mut harness.state_mut().workbench;
        let summary = workbench
            .drawing
            .update(&workbench.document, None, &workbench.hidden);
        summary.drawing.items.iter().map(|i| i.id).collect()
    };
    assert!(ids(&mut harness).contains(&shape));
    assert!(!ids(&mut harness).contains(&id));
    assert!(harness.query_by_label(super::STOP_EDITING_LEVEL).is_some());
    // Edits go to the level's shape.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id: shape,
            operation: scene::Operation::FixFacing,
        })
        .unwrap();
    assert!(
        !harness
            .state()
            .workbench
            .document()
            .scene()
            .object(shape)
            .unwrap()
            .stack
            .is_empty()
    );
    // The banner's button stops editing: the object is back.
    press(&mut harness, super::STOP_EDITING_LEVEL);
    settle(&mut harness);
    assert_eq!(harness.state().workbench.document().selection(), [id]);
    assert!(!ids(&mut harness).contains(&shape));
    assert!(ids(&mut harness).contains(&id));
    assert!(harness.query_by_label(super::STOP_EDITING_LEVEL).is_none());
    // Edit this level goes back in.
    press(&mut harness, crate::attributes::EDIT_LEVEL);
    settle(&mut harness);
    assert_eq!(harness.state().workbench.document().selection(), [shape]);
}

#[test]
fn the_panel_says_when_the_model_has_changed_since_a_level_was_made() {
    let (mut harness, id) = smoothed_sphere();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::MakeLevelByHand { id, level: 0 })
        .unwrap();
    doc.set_selection([id]);
    settle(&mut harness);
    assert!(
        harness
            .query_by_label("Start again from the model")
            .is_none()
    );
    let mut bigger = scene::PrimitiveKind::Sphere.standard();
    if let scene::Primitive::Sphere(p) = &mut bigger {
        p.sides += 4;
    }
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetPrimitive {
            ids: vec![id],
            primitive: bigger,
        })
        .unwrap();
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("The model has changed since")
            .is_some()
    );
    press(&mut harness, "Start again from the model");
    settle(&mut harness);
    assert!(
        harness
            .query_by_label("Start again from the model")
            .is_none()
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Start level 1 again from the model")
    );
}

#[test]
fn the_attributes_line_and_the_window_menu_bring_the_panel_back() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    let dock = &mut harness.state_mut().workbench.dock;
    if let Some(path) = dock.find_tab(&crate::panel::Panel::Levels) {
        dock.remove_tab(path);
    }
    harness.run_steps(2);
    press(&mut harness, crate::attributes::OPEN_LEVELS);
    harness.run_steps(2);
    assert!(
        harness
            .state()
            .workbench
            .dock
            .find_tab(&crate::panel::Panel::Levels)
            .is_some()
    );
    assert!(
        harness.query_all_by_label("Make levels").count() >= 1,
        "in front"
    );
    // Window → Levels puts it away, and brings it back.
    for shown in [false, true] {
        super::tests::main_menu(&harness, "Window").click();
        harness.run();
        // The menu's entry is drawn last, over the panel's tab.
        harness
            .get_all_by_label(super::LEVELS_ENTRY)
            .last()
            .expect("the entry")
            .click();
        harness.run();
        let dock = &harness.state().workbench.dock;
        assert_eq!(dock.find_tab(&crate::panel::Panel::Levels).is_some(), shown);
    }
}

#[test]
fn heavy_levels_are_made_on_another_thread_while_the_window_keeps_drawing() {
    let mut harness = fresh();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Mesh(
        std::sync::Arc::new(mesh::samples::heavy_ball(20_000, 100.0)),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(scene::Command::AddModifier {
        ids: vec![id],
        kind: scene::ModifierKind::Triangulate(Default::default()),
    })
    .unwrap();
    let modifiers = doc.scene().modifiers(id).to_vec();
    let mut levels = Lods {
        on: true,
        preview: 1,
        ..Lods::default()
    };
    levels.levels[0] = levels.levels[0].with_override(&modifiers, modifiers[0].id, false, None);
    doc.execute(scene::Command::SetLods {
        ids: vec![id],
        lods: levels,
    })
    .unwrap();
    // The first frame hands the work out and comes back at once.
    harness.run_steps(1);
    assert!(harness.state().workbench.drawing.busy());
    settle(&mut harness);
    assert!(!harness.state().workbench.drawing.busy());
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .lods_if_ready(id)
            .is_some()
    );
}

#[test]
fn make_levels_of_detail_switches_them_on_for_the_selected_shapes() {
    let (mut harness, id) = selected_cube();
    let mut requests = Vec::new();
    harness.state_mut().workbench.make_lods(&mut requests);
    let [DocRequest::Run(scene::Command::SetLods { ids, lods })] = requests.as_slice() else {
        panic!("one command: {requests:?}");
    };
    assert_eq!(ids, &[id]);
    assert!(lods.on);
    // The Levels panel is brought to the front, to set them.
    let dock = &harness.state().workbench.dock;
    assert!(dock.find_tab(&crate::panel::Panel::Levels).is_some());
    // Nothing selected: a note, no command.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    let mut requests = Vec::new();
    harness.state_mut().workbench.make_lods(&mut requests);
    assert!(requests.is_empty());
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .is_some_and(|n| n.contains("select a shape first"))
    );
}

/// Measures how smooth switching levels is on a heavy shape (decision
/// 0165): a 100 000-polygon ball with Triangulate and Weighted normals,
/// three levels, the first without Triangulate. Prints the slowest frame
/// while each level is previewed and how long the levels took in the
/// background. Run in a release build:
/// `cargo test --release -p ui --lib levels_stay_smooth -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement: run by hand in a release build"]
fn levels_stay_smooth_on_a_heavy_shape() {
    use std::time::{Duration, Instant};
    let mut harness = fresh();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Mesh(
        std::sync::Arc::new(mesh::samples::heavy_ball(100_000, 100.0)),
    )))
    .unwrap();
    let id = doc.selection()[0];
    for kind in [
        scene::ModifierKind::Triangulate(Default::default()),
        scene::ModifierKind::WeightedNormals(Default::default()),
    ] {
        doc.execute(scene::Command::AddModifier {
            ids: vec![id],
            kind,
        })
        .unwrap();
    }
    let modifiers = doc.scene().modifiers(id).to_vec();
    let mut levels = Lods {
        on: true,
        ..Lods::default()
    };
    levels.levels[0] = levels.levels[0].with_override(&modifiers, modifiers[0].id, false, None);
    doc.execute(scene::Command::SetLods {
        ids: vec![id],
        lods: levels,
    })
    .unwrap();
    crate::layout::show_levels(&mut harness.state_mut().workbench.dock);
    let started = Instant::now();
    settle(&mut harness);
    println!("levels made in the background: {:?}", started.elapsed());
    for preview in [1, 2, 3, 0, 2] {
        let mut lods = lods(&harness, id);
        lods.preview = preview;
        harness
            .state_mut()
            .workbench
            .document_mut()
            .execute(scene::Command::SetLods {
                ids: vec![id],
                lods,
            })
            .unwrap();
        let mut slowest = Duration::ZERO;
        let mut frames = 0;
        let began = Instant::now();
        loop {
            let frame = Instant::now();
            harness.run_steps(1);
            slowest = slowest.max(frame.elapsed());
            frames += 1;
            if !harness.state().workbench.drawing.busy() || frames > 2_000 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        println!(
            "preview {preview}: slowest frame {slowest:?} over {frames} frames, ready after {:?}",
            began.elapsed()
        );
    }
}
