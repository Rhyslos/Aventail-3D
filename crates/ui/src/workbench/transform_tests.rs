//! Tests for moving, rotating and scaling in the window: the tool keys,
//! dragging the gizmo, cancelling, fine steps, locks, snapping, the
//! palette and menus, and the Coordinates panel.

use super::tests::{State, camera, fresh, main_menu};
use super::*;
use crate::transforming::Tool;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use glam::{Vec2, Vec3};

type H = Harness<'static, State>;

fn press(harness: &mut H, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// The fresh window with its cube selected and the move tool on.
fn moving_cube() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    press(&mut harness, Key::W);
    harness
}

fn cube(harness: &H) -> scene::ObjectId {
    harness.state().workbench.document().scene().children(None)[0]
}

fn position(harness: &H) -> Vec3 {
    let doc = harness.state().workbench.document();
    doc.scene()
        .object(cube(harness))
        .unwrap()
        .transform
        .position
}

/// Where `world` appears on screen.
fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

/// Where the tip of the gizmo's X arrow is, a little in from the end.
fn x_arrow(harness: &H) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let gizmo = harness
        .state()
        .workbench
        .viewport()
        .gizmo()
        .expect("a gizmo");
    let length = gizmo.length(&camera(harness), size);
    on_screen(harness, gizmo.origin + Vec3::X * length * 0.8)
}

fn hold(harness: &mut H, keys: Modifiers) {
    harness.event(Event::ModifiersChanged(keys));
    harness.run();
}

fn button(harness: &mut H, at: Pos2, pressed: bool, button: PointerButton) {
    harness.event(Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

/// Presses at `from` and moves through `path`, without letting go.
fn drag_without_letting_go(harness: &mut H, from: Pos2, path: &[Pos2]) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(harness, from, true, PointerButton::Primary);
    for at in path {
        harness.event(Event::PointerMoved(*at));
        harness.run();
    }
}

fn let_go(harness: &mut H, at: Pos2) {
    button(harness, at, false, PointerButton::Primary);
}

/// Screen points along the X arrow's direction, from `from`, `pixels`
/// further.
fn along_x(harness: &H, from: Pos2, pixels: f32) -> Pos2 {
    let a = on_screen(harness, Vec3::ZERO);
    let b = on_screen(harness, Vec3::X * 100.0);
    from + (b - a).normalized() * pixels
}

#[test]
fn tool_keys_switch_tools_and_q_goes_back_to_selecting() {
    let mut harness = moving_cube();
    let tool = |h: &H| h.state().workbench.tool();
    assert_eq!(tool(&harness), Tool::Move);
    assert!(harness.state().workbench.viewport().gizmo().is_some());
    for (key, expected) in [
        (Key::R, Tool::Rotate),
        (Key::S, Tool::Scale),
        (Key::T, Tool::All),
    ] {
        press(&mut harness, key);
        assert_eq!(tool(&harness), expected);
    }
    // Q goes back to selecting, keeping the select tool; again, the next.
    press(&mut harness, Key::Q);
    assert_eq!(tool(&harness), Tool::Select);
    assert!(harness.state().workbench.viewport().gizmo().is_none());
    assert_eq!(
        harness.state().workbench.viewport().select_tool(),
        crate::SelectTool::Box
    );
    press(&mut harness, Key::Q);
    assert_eq!(
        harness.state().workbench.viewport().select_tool(),
        crate::SelectTool::Lasso
    );
}

#[test]
fn no_selection_no_gizmo() {
    let mut harness = fresh();
    press(&mut harness, Key::W);
    assert!(harness.state().workbench.viewport().gizmo().is_none());
}

#[test]
fn dragging_the_x_arrow_moves_along_x_as_one_undo_step() {
    let mut harness = moving_cube();
    let steps = harness.state().workbench.document().undo_steps();
    let start = x_arrow(&harness);
    let path: Vec<Pos2> = (1..=6)
        .map(|i| along_x(&harness, start, i as f32 * 15.0))
        .collect();
    drag_without_letting_go(&mut harness, start, &path);
    let_go(&mut harness, *path.last().unwrap());
    let moved = position(&harness);
    assert!(moved.x > 20.0, "{moved}");
    assert!(moved.y.abs() < 1e-3 && moved.z.abs() < 1e-3, "{moved}");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "the whole drag is one step");
    assert_eq!(doc.undo_label(), Some("Move Cube"));
    assert!(harness.query_by_label_contains("Moving: X").is_some());
    // Undo puts it back.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(position(&harness), Vec3::ZERO);
}

#[test]
fn esc_or_a_right_click_cancels_a_drag() {
    let mut harness = moving_cube();
    let steps = harness.state().workbench.document().undo_steps();
    let start = x_arrow(&harness);
    let path = [
        along_x(&harness, start, 40.0),
        along_x(&harness, start, 80.0),
    ];
    drag_without_letting_go(&mut harness, start, &path);
    assert!(position(&harness).x > 1.0);
    press(&mut harness, Key::Escape);
    assert_eq!(position(&harness), Vec3::ZERO);
    let_go(&mut harness, path[1]);
    assert_eq!(position(&harness), Vec3::ZERO, "letting go after: nothing");
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(
        harness.state().workbench.document().selection().len(),
        1,
        "Esc only cancelled; the cube is still selected"
    );

    drag_without_letting_go(&mut harness, start, &path);
    assert!(position(&harness).x > 1.0);
    button(&mut harness, path[1], true, PointerButton::Secondary);
    assert_eq!(position(&harness), Vec3::ZERO);
    button(&mut harness, path[1], false, PointerButton::Secondary);
    let_go(&mut harness, path[1]);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn shift_drags_ten_times_finer() {
    let mut normal = moving_cube();
    let start = x_arrow(&normal);
    let path = [along_x(&normal, start, 30.0), along_x(&normal, start, 60.0)];
    drag_without_letting_go(&mut normal, start, &path);
    let_go(&mut normal, path[1]);
    let full = position(&normal).x;

    let mut fine = moving_cube();
    hold(&mut fine, Modifiers::SHIFT);
    drag_without_letting_go(&mut fine, start, &path);
    let_go(&mut fine, path[1]);
    let small = position(&fine).x;
    assert!(full > 5.0 && small > 0.0, "{full} {small}");
    // The first step happens before Shift counts (as the drag begins), so
    // allow a little more than a tenth.
    assert!(small < full * 0.6, "{full} {small}");
}

#[test]
fn a_locked_axis_doesnt_move() {
    let mut harness = moving_cube();
    // X alone doesn't lock (it is kept for Slit); Shift+X does.
    press(&mut harness, Key::X);
    assert!(!harness.state().workbench.view_options().locked[0]);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::X);
    harness.run();
    assert!(harness.state().workbench.view_options().locked[0]);
    // The X arrow is gone; drag the centre instead, sideways.
    let centre = on_screen(&harness, Vec3::ZERO);
    let path = [
        centre + egui::vec2(30.0, 10.0),
        centre + egui::vec2(60.0, 20.0),
    ];
    drag_without_letting_go(&mut harness, centre, &path);
    let_go(&mut harness, path[1]);
    let moved = position(&harness);
    assert!(moved.x.abs() < 1e-3, "{moved}");
    assert!(moved.length() > 1.0, "it moved along Y or Z: {moved}");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::X);
    harness.run();
    assert!(!harness.state().workbench.view_options().locked[0]);
}

#[test]
fn l_switches_world_and_object_axes() {
    let mut harness = moving_cube();
    press(&mut harness, Key::L);
    assert_eq!(
        harness.state().workbench.view_options().axes,
        crate::Axes::Object
    );
    harness.get_by_label(crate::palette::AXES_LABEL).click();
    harness.run();
    assert_eq!(
        harness.state().workbench.view_options().axes,
        crate::Axes::World
    );
}

#[test]
fn snapping_lands_moves_on_the_grid_and_ctrl_flips_it() {
    let mut harness = moving_cube();
    harness.get_by_label("Snapping").click();
    harness.run();
    assert!(harness.state().workbench.view_options().snap.on);
    let start = x_arrow(&harness);
    let path = [
        along_x(&harness, start, 25.0),
        along_x(&harness, start, 70.0),
    ];
    drag_without_letting_go(&mut harness, start, &path);
    let_go(&mut harness, path[1]);
    let snapped = position(&harness).x;
    assert!(
        (snapped / 100.0 - (snapped / 100.0).round()).abs() < 1e-4,
        "on a metre line: {snapped}"
    );
    // Holding Ctrl flips it off for one drag.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    hold(&mut harness, Modifiers::COMMAND);
    drag_without_letting_go(&mut harness, start, &path);
    let_go(&mut harness, path[1]);
    hold(&mut harness, Modifiers::NONE);
    let free = position(&harness).x;
    assert!(
        (free / 100.0 - (free / 100.0).round()).abs() > 1e-3,
        "anywhere: {free}"
    );
}

#[test]
fn the_palette_and_tools_menu_pick_tools() {
    let mut harness = fresh();
    harness.get_by_label("Rotate").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Rotate);
    main_menu(&harness, "Tools").click();
    harness.run();
    harness.get_by_label("Scale S").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Scale);
    main_menu(&harness, "Tools").click();
    harness.run();
    harness.get_by_label("Lock Y Shift+Y").click();
    harness.run();
    assert!(harness.state().workbench.view_options().locked[1]);
    harness.get_by_label("Lock the Y axis").click();
    harness.run();
    assert!(!harness.state().workbench.view_options().locked[1]);
}

#[test]
fn a_press_on_the_gizmo_doesnt_select() {
    let mut harness = moving_cube();
    let start = x_arrow(&harness);
    harness.event(Event::PointerMoved(start));
    harness.run();
    button(&mut harness, start, true, PointerButton::Primary);
    button(&mut harness, start, false, PointerButton::Primary);
    assert_eq!(harness.state().workbench.document().selection().len(), 1);
    assert_eq!(position(&harness), Vec3::ZERO);
}

#[test]
fn typing_in_the_coordinates_panel_moves_and_scales() {
    let mut harness = moving_cube();
    // Nothing selected: the fields are empty.
    let field = |h: &H, label: &str| h.get_by_label(label).accesskit_node().is_disabled();
    assert!(!field(&harness, "Position X"));
    let steps = harness.state().workbench.document().undo_steps();
    harness.get_by_label("Position X").click();
    harness.run();
    harness.get_by_label("Position X").type_text("0.25");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert!(
        (position(&harness).x - 25.0).abs() < 1e-3,
        "{}",
        position(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Move Cube")
    );
    // Rotation is in degrees.
    harness.get_by_label("Rotation Z").click();
    harness.run();
    harness.get_by_label("Rotation Z").type_text("90");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    let doc = harness.state().workbench.document();
    let turn = doc
        .scene()
        .object(cube(&harness))
        .unwrap()
        .transform
        .rotation
        .x;
    assert!((turn - std::f32::consts::FRAC_PI_2).abs() < 1e-4, "{turn}");

    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    assert!(field(&harness, "Position X"));
}

#[test]
fn the_attributes_panel_says_when_an_edit_found_its_points_again() {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let corner = scene::VertexId::from_raw(0);
    let anchors = mesh::Anchors::record(&shape, scene::ElementKind::Points, &[vec![corner]]);
    doc.execute(Command::Record {
        id,
        operation: scene::Operation::MovePoints {
            points: vec![corner],
            offset: Vec3::Z * 10.0,
            anchors,
        },
    })
    .unwrap();
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: mesh::Primitive::Cube(mesh::CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    assert!(
        harness
            .query_by_label("1 edit found its points again on the changed shape")
            .is_some()
    );
    assert!(
        harness
            .query_by_label("1 edit no longer fully fits the shape")
            .is_none()
    );
}

#[test]
fn a_shape_squashed_flat_gets_no_gizmo_for_its_points() {
    // A regression test from the whole-app review (L2): moving points of
    // a shape scaled to 0 recorded a move that isn't a number, which
    // saved but couldn't be opened again.
    let mut harness = moving_cube();
    let id = cube(&harness);
    let mut squashed = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .transform;
    squashed.scale.x = 0.0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(id, squashed)]))
        .unwrap();
    harness.run();
    press(&mut harness, Key::Num1);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                scene::ElementSelection::of([scene::Element::Point(scene::VertexId::from_raw(0))]),
            )],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Key::W);
    harness.run();
    assert!(harness.state().workbench.viewport().gizmo().is_none());
}

#[test]
fn the_attributes_panel_says_when_an_edit_was_too_large() {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: mesh::Primitive::Cube(mesh::CubeParams {
            segments: [256, 256, 256],
            ..Default::default()
        }),
    })
    .unwrap();
    let edges = doc.scene().evaluated(id).unwrap().mesh.edges().to_vec();
    doc.execute(Command::Record {
        id,
        operation: scene::Operation::BevelEdges {
            edges,
            shape: scene::BevelShape {
                width: 0.1,
                segments: 64,
                profile: 1.0,
                options: scene::BevelOptions::EARLIER,
            },
            anchors: None,
            stretch: glam::Vec3::ONE,
        },
    })
    .unwrap();
    harness.run();
    assert!(
        harness
            .query_by_label("1 edit was skipped: the shape would be too large")
            .is_some()
    );
}

// ------------------------------------------------------------ symmetry ---

/// The cube's evaluated shape.
fn shape(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    doc.scene().evaluated(cube(harness)).unwrap().mesh
}

#[test]
fn the_symmetry_button_gives_the_selected_object_a_live_mirror() {
    let mut harness = fresh();
    let id = cube(&harness);
    // Nothing selected: it says what to do.
    harness.state_mut().workbench.toggle_symmetry();
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .contains("select the objects")
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Symmetry").click();
    harness.run();
    let workbench = &harness.state().workbench;
    let doc = workbench.document();
    assert_eq!(
        doc.scene().object(id).unwrap().mirror,
        Some(scene::LiveMirror::on_object_x())
    );
    assert_eq!(doc.undo_label(), Some("Mirror"));
    assert!(
        harness
            .query_by_label_contains("Symmetry on, across X")
            .is_some()
    );
    // The mirror belongs to the object (decision 0126): moved to the
    // side, its twin moves with it, mirrored across its own pivot.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(
            id,
            scene::Transform {
                position: glam::Vec3::new(100.0, 0.0, 0.0),
                ..Default::default()
            },
        )]))
        .unwrap();
    harness.run();
    let workbench = &harness.state().workbench;
    let doc = workbench.document();
    let bounds = doc.scene().bounds().unwrap();
    assert!((bounds.min.x - 50.0).abs() < 1e-3, "{bounds:?}");
    assert!((bounds.max.x - 150.0).abs() < 1e-3, "{bounds:?}");
    let drawn = doc.scene().shown_shape(id).unwrap();
    assert_eq!(drawn.face_count(), 2 * shape(&harness).face_count());
    // Off again: one undo step.
    harness.state_mut().workbench.toggle_symmetry();
    harness.run();
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().object(id).unwrap().mirror, None);
    assert_eq!(doc.undo_label(), Some("Mirror off"));
}

#[test]
fn the_symmetry_menu_changes_the_selected_objects_mirror() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness
        .get_by_label(crate::palette::SYMMETRY_MENU_LABEL)
        .click();
    harness.run();
    // On, from the menu, then across Y as well.
    harness.get_by_label("On").click();
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .object(id)
            .unwrap()
            .mirror
            .is_some()
    );
    harness.get_by_label("Symmetry Y").click();
    harness.run();
    let mirror = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .mirror
        .unwrap();
    assert_eq!(mirror.axes, [true, true, false]);
    // Kept for the next mirror too.
    assert_eq!(
        harness.state().workbench.viewport().options().symmetry.axes,
        [true, true, false]
    );
    // Moving the mirror is the same kind of change, merged into one step.
    let steps = harness.state().workbench.document().undo_steps();
    let mut settings = harness.state().workbench.viewport().options().symmetry;
    settings.offset = [20.0, 0.0, 0.0];
    harness.state_mut().workbench.change_symmetry(settings);
    settings.offset = [30.0, 0.0, 0.0];
    harness.state_mut().workbench.change_symmetry(settings);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps);
    assert_eq!(
        doc.scene().object(id).unwrap().mirror.unwrap().centre,
        glam::Vec3::new(30.0, 0.0, 0.0)
    );
}

#[test]
fn with_a_mirror_edits_are_made_once_and_the_twin_follows() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.state_mut().workbench.toggle_symmetry();
    press(&mut harness, Key::Num1);
    let mesh = shape(&harness);
    let right = mesh
        .vertices()
        .find(|(_, p)| p.x > 0.0 && p.y > 0.0 && p.z > 99.0)
        .map(|(v, _)| v)
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                scene::ElementSelection::of([scene::Element::Point(right)]),
            )],
            "Select",
            None,
        );
    press(&mut harness, Key::W);
    let start = x_arrow(&harness);
    let path: Vec<Pos2> = (1..=6)
        .map(|i| along_x(&harness, start, i as f32 * 15.0))
        .collect();
    drag_without_letting_go(&mut harness, start, &path);
    let_go(&mut harness, *path.last().unwrap());
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Move 1 point"), "made once");
    let moved = shape(&harness).position(right).unwrap();
    assert!(moved.x > 70.0, "{moved}");
    // The twin's copy of that point went the other way.
    let drawn = doc.scene().shown_shape(id).unwrap();
    assert!(
        drawn
            .vertices()
            .any(|(_, p)| (p.x + moved.x).abs() < 1e-3 && p.y > 0.0 && p.z > 99.0)
    );
}

// ------------------------------------------------------------- sliding ---

#[test]
fn alt_dragging_the_move_gizmo_slides_points_along_their_edges() {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(scene::Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [1, 1, 2],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    press(&mut harness, Key::Num1);
    // The ring of points half way up.
    let ring: Vec<scene::Element> = shape(&harness)
        .vertices()
        .filter(|(_, p)| (p.z - 50.0).abs() < 1e-3)
        .map(|(v, _)| scene::Element::Point(v))
        .collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, scene::ElementSelection::of(ring.clone()))],
            "Select",
            None,
        );
    press(&mut harness, Key::W);
    // Alt + drag up the Z arrow.
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let gizmo = harness.state().workbench.viewport().gizmo().unwrap();
    let length = gizmo.length(&camera(&harness), size);
    let start = on_screen(&harness, gizmo.origin + Vec3::Z * length * 0.8);
    let up = (on_screen(&harness, Vec3::Z * 100.0) - on_screen(&harness, Vec3::ZERO)).normalized();
    let camera_before = camera(&harness);
    hold(&mut harness, Modifiers::ALT);
    let path: Vec<Pos2> = (1..=4).map(|i| start + up * (i as f32 * 8.0)).collect();
    drag_without_letting_go(&mut harness, start, &path);
    let_go(&mut harness, *path.last().unwrap());
    hold(&mut harness, Modifiers::NONE);
    assert_eq!(camera(&harness), camera_before, "it didn't orbit");
    let mesh = shape(&harness);
    for element in &ring {
        let scene::Element::Point(v) = element else {
            continue;
        };
        let p = mesh.position(*v).unwrap();
        assert!(p.z > 52.0 && p.z <= 100.0, "slid up: {p}");
        assert!((p.x.abs() - 50.0).abs() < 1e-3, "stayed on the side: {p}");
    }
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Slide 4 points")
    );
}

#[test]
fn apply_mirror_keeps_both_halves_and_turns_symmetry_off() {
    // As Blender's Apply (decision 0126): turning symmetry off afterwards
    // keeps the shape whole.
    let mut harness = fresh();
    let id = cube(&harness);
    harness.state_mut().workbench.apply_mirror();
    assert!(
        harness
            .state()
            .workbench
            .note
            .as_deref()
            .unwrap()
            .contains("symmetry on first")
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Symmetry").click();
    harness.run();
    let shown = harness
        .state()
        .workbench
        .document()
        .scene()
        .shown_shape(id)
        .unwrap()
        .face_count();
    // In the Symmetry menu (the arrow beside the button).
    harness
        .get_by_label(crate::palette::SYMMETRY_MENU_LABEL)
        .click();
    harness.run();
    harness
        .get_by_label(crate::palette::APPLY_MIRROR_LABEL)
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    let object = doc.scene().object(id).unwrap();
    assert!(object.mirror.is_none());
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), shown);
    assert_eq!(doc.undo_label(), Some("Apply mirror"));
    // Symmetry is off, and the shape still has both halves.
    assert!(harness.state().workbench.selected_mirror().is_none());
}
