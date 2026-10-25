//! Tests for what two testers found in Phase 10's interface (the review of
//! 2026-10-25): the file window failing without a word, a long status note
//! over the text beside it, the modifier rows jumping and their warnings cut
//! off, the game bevel's last edit, command search for Sweep and Lathe,
//! framing an array's copies, "-0.0°", the Export window's levels and
//! collision, calm notes for modifiers with nothing to do, and the palette's
//! names under a question.

use super::modelling_tests::{H, cube, note};
use super::path_entry::{FilePurpose, PATH_BOX_LABEL};
use super::tests::{camera, fresh};
use super::*;
use crate::attributes::modifier_rows::ON_LABEL;
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec3;
use scene::{ArrayOptions, ArraySettings, Command, Modifier, ModifierKind, ObjectId};

/// The fresh window's cube, selected.
fn selected_cube() -> (H, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id)
}

fn run(harness: &mut H, command: Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run_steps(3);
}

/// A line of `count` copies of the shape, each a shape's width apart.
fn array(count: u32) -> ModifierKind {
    ModifierKind::Array(ArraySettings {
        options: ArrayOptions {
            count,
            relative: true,
            offset: Vec3::new(1.2, 0.0, 0.0),
            ..ArrayOptions::default()
        },
        ..ArraySettings::default()
    })
}

/// Types `text` into the box with keyboard focus.
fn type_text(harness: &mut H, text: &str) {
    harness.event(egui::Event::Text(text.to_string()));
    harness.run();
}

/// Clears the path box and types `text` into it.
fn type_path(harness: &mut H, text: &str) {
    harness.get_by_label(PATH_BOX_LABEL).focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    type_text(harness, text);
}

fn temp_folder(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("review-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn a_file_window_that_cant_open_says_so_and_offers_a_path_box() {
    let mut harness = fresh();
    let folder = temp_folder("save");
    harness.state_mut().workbench.file_window_missing(
        FilePurpose::SaveProject {
            extension: "avtl".into(),
        },
        Some(folder.join("Untitled.avtl")),
    );
    harness.run();
    assert!(
        note(&harness).contains("file window didn't open"),
        "{}",
        note(&harness)
    );
    assert!(harness.state().workbench.is_typing_a_path());
    // The box starts with the suggested path; a typed name gets the
    // project's extension.
    type_path(&mut harness, &folder.join("crate").display().to_string());
    harness.get_by_label("Save").click();
    harness.run();
    assert!(!harness.state().workbench.is_typing_a_path());
    let saved = harness.state().actions.iter().any(
        |a| matches!(a, WorkbenchAction::Save { path: Some(p) } if *p == folder.join("crate.avtl")),
    );
    assert!(saved, "{:?}", harness.state().actions);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_quit_windows_save_goes_on_to_close_through_the_path_box() {
    let mut harness = fresh();
    let folder = temp_folder("quit");
    // Unsaved changes: closing asks first; Save asks where (never saved).
    assert!(!harness.state_mut().workbench.request_close());
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::Save { path: None }))
    );
    // The app finds no file window.
    harness.state_mut().actions.clear();
    harness.state_mut().workbench.file_window_missing(
        FilePurpose::SaveProject {
            extension: "avtl".into(),
        },
        None,
    );
    harness.run();
    type_path(
        &mut harness,
        &folder.join("closing.avtl").display().to_string(),
    );
    harness.key_press(Key::Enter);
    harness.run();
    let path = folder.join("closing.avtl");
    assert!(
        harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::Save { path: Some(p) } if *p == path))
    );
    // Saved: the window closes, as asked before.
    let revision = harness.state().workbench.document().revision();
    harness.state_mut().workbench.saved_at(path, revision, None);
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::Quit))
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn cancelling_the_path_box_cancels_what_waited_and_bad_paths_wait() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .file_window_missing(FilePurpose::OpenProject, None);
    harness.run();
    type_path(&mut harness, "/no/such/place/crate.avtl");
    assert!(
        harness
            .query_by_label_contains("There's no file there")
            .is_some(),
        "says why"
    );
    harness.key_press(Key::Enter);
    harness.run();
    assert!(
        harness.state().workbench.is_typing_a_path(),
        "a bad path waits"
    );
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(!harness.state().workbench.is_typing_a_path());
    assert!(
        !harness
            .state()
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::Open { .. }))
    );
}

#[test]
fn a_game_folder_typed_in_is_used() {
    let mut harness = fresh();
    let folder = temp_folder("game");
    harness
        .state_mut()
        .workbench
        .file_window_missing(FilePurpose::GameFolder, Some(folder.clone()));
    harness.run();
    harness.get_by_label("Choose folder").click();
    harness.run();
    let target = harness
        .state()
        .workbench
        .document()
        .scene()
        .export_target()
        .cloned();
    assert_eq!(
        target.map(|t| std::path::PathBuf::from(t.folder)),
        Some(folder.clone())
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_long_status_note_is_cut_short_beside_the_left_text() {
    let (mut harness, _) = selected_cube();
    let long = "Game bevel: the shape bevelled 2 cm at every edge sharper than 30°, with \
                round bevels and flat faces (Live bevel, then Weighted normals). Change either \
                in the Attributes panel; Ctrl + Z takes it all back. Smoothing now follows the \
                marked edges, with the shape's look kept as marks.";
    harness.state_mut().workbench.note = Some(long.repeat(2));
    harness.run();
    let left = harness.get_by_label_contains("Model mode:").rect();
    let right = harness
        .get_by_label_contains("Game bevel: the shape")
        .rect();
    assert!(
        left.max.x <= right.min.x,
        "the note starts after the left text ends: {left:?} {right:?}"
    );
    assert!(right.max.x <= 1440.0 + 0.5, "and stays in the window");
}

#[test]
fn switching_a_later_modifier_off_keeps_the_rows_in_place() {
    let (mut harness, id) = selected_cube();
    run(
        &mut harness,
        Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::WeightedNormals(Default::default()),
        },
    );
    run(
        &mut harness,
        Command::AddModifier {
            ids: vec![id],
            kind: array(3),
        },
    );
    // The array goes before the normals; move it after them, where it
    // drops them, as the tester had it.
    run(&mut harness, Command::MoveModifier { id, from: 0, to: 1 });
    let switch_y = |harness: &H| {
        harness
            .get_all_by_label(ON_LABEL)
            .map(|n| n.rect().min.y)
            .fold(f32::MIN, f32::max)
    };
    let before = switch_y(&harness);
    let warning = harness
        .get_by_label_contains("comes after this and changes the shape")
        .rect();
    // The warning wraps inside the panel rather than running off it.
    let panel = harness.get_by_label("Attributes").rect();
    assert!(warning.width() < 500.0, "{warning:?} {panel:?}");
    let modifiers = harness
        .state()
        .workbench
        .document()
        .scene()
        .modifiers(id)
        .to_vec();
    run(
        &mut harness,
        Command::ChangeModifier {
            id,
            index: 1,
            modifier: Modifier {
                on: false,
                ..modifiers[1].clone()
            },
        },
    );
    assert!(
        harness
            .query_by_label_contains("comes after this and is off")
            .is_some(),
        "a calm note in its place"
    );
    assert!(
        (switch_y(&harness) - before).abs() < 0.5,
        "the Array's switch stays under the mouse"
    );
}

#[test]
fn after_a_game_bevel_the_last_edit_is_the_game_bevel() {
    let (mut harness, _) = selected_cube();
    harness.state_mut().workbench.run_game_bevel();
    harness.run_steps(3);
    assert!(harness.query_by_label("Last edit: Game bevel").is_some());
    assert!(
        harness
            .query_by_label_contains("Hard edges by angle")
            .is_none()
    );
    assert!(
        harness
            .query_by_label_contains("Smoothing switched from By angle")
            .is_some(),
        "the change of Smoothing is said"
    );
}

#[test]
fn command_search_finds_sweep_and_lathe() {
    let mut harness = fresh();
    let ctx = harness.ctx.clone();
    let entries = harness.state().workbench.search_entries(&ctx);
    for name in ["Sweep the curve", "Lathe the curve"] {
        let entry = entries.iter().find(|e| e.name == name).expect(name);
        assert!(entry.not_now.is_some(), "greyed out with no curve selected");
    }
    let found = entries
        .iter()
        .filter(|e| crate::command_search::score(e, "sweep").is_some())
        .any(|e| e.name == "Sweep the curve");
    assert!(found);
    // A curve selected: the entry makes it a sweep.
    let curve = scene::Curve::corners(
        &[
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
        ],
        false,
    )
    .unwrap();
    run(
        &mut harness,
        Command::Create(scene::ObjectKind::Curve(scene::CurveObject::line(
            scene::CurveShape::Points(curve),
        ))),
    );
    let id = harness.state().workbench.document().selection()[0];
    let entries = harness.state().workbench.search_entries(&ctx);
    let sweep = entries
        .iter()
        .find(|e| e.name == "Sweep the curve")
        .unwrap();
    assert!(sweep.not_now.is_none());
    let action = sweep.action;
    let mut actions = Vec::new();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_search(action, &ctx, &mut actions, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    let solid = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .and_then(|o| o.kind.curve())
        .map(|c| c.solid);
    assert!(matches!(solid, Some(scene::CurveSolid::Sweep(_))));
}

/// Whether `world` is inside the viewport's picture.
fn in_view(harness: &H, world: Vec3) -> bool {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    camera(harness)
        .project(world, size)
        .is_some_and(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y)
}

#[test]
fn framing_takes_in_an_arrays_copies() {
    for what in [crate::Frame::All, crate::Frame::Selected] {
        let (mut harness, id) = selected_cube();
        run(
            &mut harness,
            Command::AddModifier {
                ids: vec![id],
                kind: array(12),
            },
        );
        // The last copy's far corner: 11 steps of 1.2 cube widths along X.
        let far = Vec3::new(11.0 * 120.0 + 50.0, 50.0, 100.0);
        harness.state_mut().workbench.viewport.frame(what);
        harness.run_steps(30);
        assert!(in_view(&harness, far), "{what:?} frames the copies");
    }
}

#[test]
fn rotation_fields_never_show_minus_zero() {
    let (mut harness, id) = selected_cube();
    let mut transform = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .transform;
    // A hair below zero, as rounding leaves after turning back.
    transform.rotation = Vec3::new(-1e-7, -1e-6, -1e-9);
    run(&mut harness, Command::SetTransforms(vec![(id, transform)]));
    for axis in ["X", "Y", "Z"] {
        let label = format!("Rotation {axis}");
        let node = harness.get_by_label(&label);
        let shown = node.accesskit_node().value().unwrap_or_default();
        assert!(!shown.contains("-0"), "Rotation {axis}: {shown}");
        assert!(shown.contains("0.0"), "Rotation {axis}: {shown}");
    }
}

#[test]
fn a_weld_with_nothing_to_do_is_a_calm_note() {
    let (mut harness, id) = selected_cube();
    run(
        &mut harness,
        Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::WeldByDistance(Default::default()),
        },
    );
    assert!(harness.query_by_label_contains("Nothing to do:").is_some());
    assert!(harness.query_by_label_contains("Not worked out").is_none());
}

#[test]
fn the_export_window_starts_with_levels_on_and_lists_collision() {
    let (mut harness, id) = selected_cube();
    run(
        &mut harness,
        Command::SetLods {
            ids: vec![id],
            lods: scene::Lods {
                on: true,
                ..scene::Lods::default()
            },
        },
    );
    run(
        &mut harness,
        Command::SetCollision {
            ids: vec![id],
            collision: scene::Collision::Box,
        },
    );
    harness.state_mut().workbench.exporting.open = true;
    harness.run_steps(3);
    let chip = |harness: &H| {
        harness
            .get_all_by_label("Levels of detail")
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
            .map(|n| n.accesskit_node().toggled())
    };
    assert_eq!(
        chip(&harness),
        Some(Some(egui::accesskit::Toggled::True)),
        "on: the cube has levels"
    );
    assert!(
        harness
            .query_by_label_contains("Collision goes out for 1 object: A box × 1")
            .is_some()
    );
    // Switched off by hand, it stays off, even with levels made.
    harness
        .get_all_by_label("Levels of detail")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .unwrap()
        .click();
    harness.run_steps(3);
    assert!(harness.state().workbench.exporting.choice.lod_chosen);
    assert!(!harness.state().workbench.exporting.choice.lod_levels);
    harness.run_steps(3);
    assert!(!harness.state().workbench.exporting.choice.lod_levels);
}

#[test]
fn the_palettes_names_stay_away_under_a_question() {
    let mut harness = fresh();
    // Rest the mouse on the mode palette until its names slide out.
    let at = harness.get_by_label("Points mode").rect().center();
    harness.event(egui::Event::PointerMoved(at));
    harness.run();
    let wait = |harness: &mut H, seconds: f64| {
        let start = harness.ctx.input(|i| i.time);
        while harness.ctx.input(|i| i.time) - start < seconds {
            harness.step();
        }
    };
    let named = |harness: &H| {
        harness
            .query_by_label(&format!("{}Points", crate::palette::NAMES_PREFIX))
            .is_some()
    };
    wait(&mut harness, crate::palette::NAMES_AFTER + 0.3);
    assert!(named(&harness), "the names slide out");
    assert!(!harness.state_mut().workbench.request_close());
    wait(&mut harness, 0.5);
    assert!(harness.state().workbench.is_asking_to_save());
    assert!(!named(&harness), "put away while Save changes? is asked");
}

/// How many texts drawn in the last frame start with `start`.
fn drawn_texts(harness: &H, start: &str) -> usize {
    harness
        .output()
        .shapes
        .iter()
        .filter(|c| matches!(&c.shape, egui::epaint::Shape::Text(t) if t.galley.text().starts_with(start)))
        .count()
}

#[test]
fn one_tooltip_at_a_time_on_the_modifiers_palette() {
    // A tester saw pieces of two tooltips at once, and one left behind as
    // the palette scrolled. egui shows one tooltip at a time, and the app
    // draws none of its own: moving from button to button leaves only the
    // new one, and scrolling puts the old one away.
    let (mut harness, _) = selected_cube();
    let wait = |harness: &mut H, seconds: f64| {
        let start = harness.ctx.input(|i| i.time);
        while harness.ctx.input(|i| i.time) - start < seconds {
            harness.step();
        }
    };
    let subtract = harness.get_by_label("Subtract").rect().center();
    harness.event(egui::Event::PointerMoved(subtract));
    wait(&mut harness, 1.0);
    assert_eq!(drawn_texts(&harness, "Subtract\n"), 1, "its tooltip");
    let union = harness.get_by_label("Union").rect().center();
    harness.event(egui::Event::PointerMoved(union));
    harness.step();
    assert_eq!(
        drawn_texts(&harness, "Subtract\n"),
        0,
        "Subtract's went at once"
    );
    assert!(drawn_texts(&harness, "Union\n") <= 1);
    for _ in 0..4 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -3.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        harness.step();
    }
    assert_eq!(
        drawn_texts(&harness, "Union\n"),
        0,
        "none left behind while scrolling"
    );
}
