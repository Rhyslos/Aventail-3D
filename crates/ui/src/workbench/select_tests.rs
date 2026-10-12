//! Tests for selecting in the window: clicking and dragging in the
//! viewport, the select keys, the Select menu and the switches.

use super::tests::{State, camera, fresh, main_menu, quick_harness};
use super::*;
use crate::selecting::SelectTool;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use glam::{Vec2, Vec3};
use scene::ElementKind;

type H = Harness<'static, State>;

/// Where `world` appears on screen.
fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

/// A spot in the viewport's top-left corner, away from the cube.
fn empty_spot(harness: &H) -> Pos2 {
    harness.get_by_label(crate::VIEWPORT_LABEL).rect().min + egui::vec2(20.0, 20.0)
}

fn hold(harness: &mut H, keys: Modifiers) {
    harness.event(Event::ModifiersChanged(keys));
    harness.run();
}

fn button(harness: &mut H, at: Pos2, pressed: bool, keys: Modifiers) {
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: keys,
    });
    harness.run();
}

/// A left click at `at` with `keys` held.
fn click(harness: &mut H, at: Pos2, keys: Modifiers) {
    hold(harness, keys);
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(harness, at, true, keys);
    button(harness, at, false, keys);
    hold(harness, Modifiers::NONE);
}

/// A left drag through `path` with `keys` held.
fn drag(harness: &mut H, path: &[Pos2], keys: Modifiers) {
    hold(harness, keys);
    harness.event(Event::PointerMoved(path[0]));
    harness.run();
    button(harness, path[0], true, keys);
    for at in &path[1..] {
        harness.event(Event::PointerMoved(*at));
        harness.run();
    }
    button(harness, *path.last().unwrap(), false, keys);
    hold(harness, Modifiers::NONE);
}

/// A box drag from corner to corner of the whole viewport.
fn box_over_everything(harness: &mut H, keys: Modifiers) {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let a = rect.min + egui::vec2(5.0, 5.0);
    let b = rect.max - egui::vec2(5.0, 5.0);
    drag(harness, &[a, a.lerp(b, 0.5), b], keys);
}

fn cube_id(harness: &H) -> scene::ObjectId {
    harness.state().workbench.document().scene().children(None)[0]
}

fn selected_points(harness: &H) -> usize {
    let doc = harness.state().workbench.document();
    doc.elements(cube_id(harness))
        .map_or(0, |s| s.count(ElementKind::Points))
}

fn press(harness: &mut H, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// The fresh window with its cube selected, in the Points mode.
fn points_mode() -> H {
    let mut harness = fresh();
    let id = cube_id(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    press(&mut harness, Key::Num1);
    harness
}

const MIDDLE_OF_FRONT: Vec3 = Vec3::new(0.0, -50.0, 50.0);

#[test]
fn clicking_a_shape_selects_it_and_clicking_nothing_unselects() {
    let mut harness = fresh();
    let cube = cube_id(&harness);
    assert!(harness.state().workbench.document().selection().is_empty());
    let at = on_screen(&harness, MIDDLE_OF_FRONT);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(harness.state().workbench.document().selection(), vec![cube]);
    let spot = empty_spot(&harness);
    click(&mut harness, spot, Modifiers::NONE);
    assert!(harness.state().workbench.document().selection().is_empty());
    // Ctrl + click adds, and takes away again.
    click(&mut harness, at, Modifiers::COMMAND);
    assert_eq!(harness.state().workbench.document().selection(), vec![cube]);
    click(&mut harness, at, Modifiers::COMMAND);
    assert!(harness.state().workbench.document().selection().is_empty());
}

#[test]
fn a_box_selects_shapes_and_shift_box_unselects() {
    let mut harness = fresh();
    let cube = cube_id(&harness);
    box_over_everything(&mut harness, Modifiers::NONE);
    assert_eq!(harness.state().workbench.document().selection(), vec![cube]);
    box_over_everything(&mut harness, Modifiers::SHIFT);
    assert!(harness.state().workbench.document().selection().is_empty());
}

#[test]
fn a_box_selects_points_through_the_shape_as_standard() {
    let mut harness = points_mode();
    box_over_everything(&mut harness, Modifiers::NONE);
    assert_eq!(selected_points(&harness), 8, "the hidden corner too");
    assert!(
        harness
            .query_by_label_contains("8 points selected")
            .is_some()
    );
    // The selected points are drawn in the selection style.
    let overlay = &harness.state().renderer.drawn.last().unwrap().overlay;
    assert_eq!(overlay.points.len(), 8);

    // With "select only visible" on, the hidden corner is left out.
    harness
        .get_by_label(crate::palette::VISIBLE_ONLY_LABEL)
        .click();
    harness.run();
    assert!(harness.state().workbench.view_options().select_visible_only);
    box_over_everything(&mut harness, Modifiers::NONE);
    assert_eq!(selected_points(&harness), 7);
}

#[test]
fn clicks_select_points_and_undo_takes_them_back() {
    let mut harness = points_mode();
    let corner = on_screen(&harness, Vec3::new(50.0, -50.0, 100.0));
    click(&mut harness, corner, Modifiers::NONE);
    assert_eq!(selected_points(&harness), 1);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(selected_points(&harness), 0);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Y);
    harness.run();
    assert_eq!(selected_points(&harness), 1);
    // The shape stays selected however the points change.
    assert_eq!(harness.state().workbench.document().selection().len(), 1);
}

#[test]
fn ctrl_a_esc_and_ctrl_i() {
    let mut harness = points_mode();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    assert_eq!(selected_points(&harness), 8);
    press(&mut harness, Key::Escape);
    assert_eq!(selected_points(&harness), 0);
    assert_eq!(
        harness.state().workbench.document().selection().len(),
        1,
        "Esc in the Points mode keeps the shape selected"
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::I);
    harness.run();
    assert_eq!(
        selected_points(&harness),
        8,
        "inverting nothing selects all"
    );
    // Back in the Model mode, Esc unselects the shape.
    press(&mut harness, Key::Num1);
    press(&mut harness, Key::Escape);
    assert!(harness.state().workbench.document().selection().is_empty());
}

#[test]
fn q_steps_through_the_select_tools() {
    let mut harness = fresh();
    let tool = |h: &H| h.state().workbench.viewport().select_tool();
    assert_eq!(tool(&harness), SelectTool::Box);
    assert!(harness.query_by_label("Box selection").is_some());
    press(&mut harness, Key::Q);
    assert_eq!(tool(&harness), SelectTool::Lasso);
    assert!(harness.query_by_label("Lasso selection").is_some());
    assert!(harness.query_by_label_contains("draw a loop").is_some());
    press(&mut harness, Key::Q);
    assert_eq!(tool(&harness), SelectTool::Brush);
    // The palette button steps on too.
    harness.get_by_label("Live selection").click();
    harness.run();
    assert_eq!(tool(&harness), SelectTool::Box);
}

#[test]
fn a_lasso_selects_what_it_goes_around() {
    let mut harness = points_mode();
    press(&mut harness, Key::Q);
    let corner = on_screen(&harness, Vec3::new(50.0, -50.0, 100.0));
    let around = [
        corner + egui::vec2(-12.0, -12.0),
        corner + egui::vec2(12.0, -12.0),
        corner + egui::vec2(12.0, 12.0),
        corner + egui::vec2(-12.0, 12.0),
        corner + egui::vec2(-12.0, -10.0),
    ];
    drag(&mut harness, &around, Modifiers::NONE);
    assert_eq!(selected_points(&harness), 1);
}

#[test]
fn the_brush_paints_a_selection_in_one_undo_step() {
    let mut harness = points_mode();
    press(&mut harness, Key::Q);
    press(&mut harness, Key::Q);
    let steps = harness.state().workbench.document().undo_steps();
    let a = on_screen(&harness, Vec3::new(50.0, -50.0, 100.0));
    let b = on_screen(&harness, Vec3::new(-50.0, -50.0, 100.0));
    let path: Vec<Pos2> = (0..=10).map(|i| a.lerp(b, i as f32 / 10.0)).collect();
    drag(&mut harness, &path, Modifiers::NONE);
    assert!(
        selected_points(&harness) >= 2,
        "{}",
        selected_points(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
}

#[test]
fn ctrl_wheel_grows_the_selection_or_sizes_the_brush() {
    let mut harness = points_mode();
    let corner = on_screen(&harness, Vec3::new(50.0, -50.0, 100.0));
    click(&mut harness, corner, Modifiers::NONE);
    let wheel = |harness: &mut H, up: bool| {
        hold(harness, Modifiers::COMMAND);
        harness.event(Event::PointerMoved(corner));
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, if up { 1.0 } else { -1.0 }),
            modifiers: Modifiers::COMMAND,
            phase: egui::TouchPhase::Move,
        });
        harness.run();
        hold(harness, Modifiers::NONE);
    };
    let camera_before = *harness.state().workbench.viewport().camera();
    wheel(&mut harness, true);
    assert_eq!(
        selected_points(&harness),
        4,
        "the corner and its three neighbours"
    );
    assert_eq!(
        *harness.state().workbench.viewport().camera(),
        camera_before,
        "Ctrl + wheel doesn't zoom"
    );
    wheel(&mut harness, false);
    assert_eq!(selected_points(&harness), 1);

    // With the brush, the wheel sizes the brush instead.
    press(&mut harness, Key::Q);
    press(&mut harness, Key::Q);
    let radius = harness.state().workbench.viewport().brush_radius();
    wheel(&mut harness, true);
    assert!(harness.state().workbench.viewport().brush_radius() > radius);
    assert_eq!(selected_points(&harness), 1);
}

#[test]
fn double_clicking_an_edge_selects_its_loop() {
    let mut harness = quick_harness();
    let id = cube_id(&harness);
    // A cube cut in two each way, so loops go round it.
    let params = mesh::CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(Command::SetPrimitive {
            ids: vec![id],
            primitive: mesh::Primitive::Cube(params),
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    press(&mut harness, Key::Num2);
    let at = on_screen(&harness, Vec3::new(25.0, -50.0, 50.0));
    hold(&mut harness, Modifiers::NONE);
    harness.event(Event::PointerMoved(at));
    harness.run();
    for _ in 0..2 {
        button(&mut harness, at, true, Modifiers::NONE);
        button(&mut harness, at, false, Modifiers::NONE);
    }
    let doc = harness.state().workbench.document();
    assert_eq!(
        doc.elements(id).map(|s| s.count(ElementKind::Edges)),
        Some(8)
    );
}

#[test]
fn the_select_menu_works() {
    let mut harness = points_mode();
    main_menu(&harness, "Select").click();
    harness.run();
    harness.get_by_label_contains("Select all").click();
    harness.run();
    assert_eq!(selected_points(&harness), 8);
    main_menu(&harness, "Select").click();
    harness.run();
    harness.get_by_label_contains("Lasso selection").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.viewport().select_tool(),
        SelectTool::Lasso
    );
}

/// The cube standing on a 3 m plane.
fn cube_and_plane() -> H {
    let mut harness = quick_harness();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::Create(scene::ObjectKind::Primitive(
        mesh::Primitive::Plane(mesh::PlaneParams {
            width: 300.0,
            depth: 300.0,
            segments: [1, 1],
        }),
    )))
    .unwrap();
    doc.clear_selection();
    harness.run();
    harness
}

#[test]
fn quick_clicks_on_two_shapes_swap_between_them() {
    let mut harness = cube_and_plane();
    let ids = harness.state().workbench.document().scene().children(None);
    let at_cube = on_screen(&harness, MIDDLE_OF_FRONT);
    let at_plane = on_screen(&harness, Vec3::new(-100.0, -100.0, 0.0));
    // Clicks a frame apart: quick enough for a double-click, but in
    // different places.
    for (at, expected) in [(at_cube, ids[1]), (at_plane, ids[0]), (at_cube, ids[1])] {
        harness.event(Event::PointerMoved(at));
        harness.run();
        button(&mut harness, at, true, Modifiers::NONE);
        button(&mut harness, at, false, Modifiers::NONE);
        let selection = harness.state().workbench.document().selection();
        assert_eq!(selection, vec![expected], "only the shape clicked last");
    }
}

#[test]
fn undo_takes_back_clicked_selections() {
    let mut harness = fresh();
    let cube = cube_id(&harness);
    let at = on_screen(&harness, MIDDLE_OF_FRONT);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(harness.state().workbench.document().selection(), vec![cube]);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(harness.state().workbench.document().selection().is_empty());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Y);
    harness.run();
    assert_eq!(harness.state().workbench.document().selection(), vec![cube]);
    // Selecting changes nothing that needs saving.
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Select Cube")
    );
}

#[test]
fn shift_click_removes_with_the_standard_keys() {
    let mut harness = points_mode();
    let corner = on_screen(&harness, Vec3::new(50.0, -50.0, 100.0));
    box_over_everything(&mut harness, Modifiers::NONE);
    assert_eq!(selected_points(&harness), 8);
    click(&mut harness, corner, Modifiers::SHIFT);
    assert_eq!(selected_points(&harness), 7);
    click(&mut harness, corner, Modifiers::COMMAND);
    assert_eq!(selected_points(&harness), 8, "Ctrl + click adds it back");
}

#[test]
fn another_key_pattern_can_be_chosen_in_settings() {
    let mut harness = points_mode();
    crate::theme::apply(&harness.ctx);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    harness.get_by_label("Settings section: Viewport").click();
    harness.run();
    harness.get_by_label("Shift adds, Ctrl removes").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.view_options().select_keys,
        crate::SelectKeys::ShiftAddsCtrlRemoves
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Comma);
    harness.run();
    box_over_everything(&mut harness, Modifiers::NONE);
    let corner = on_screen(&harness, Vec3::new(50.0, -50.0, 100.0));
    click(&mut harness, corner, Modifiers::COMMAND);
    assert_eq!(selected_points(&harness), 7, "Ctrl removes now");
}

#[test]
fn alt_double_click_selects_the_ring() {
    let mut harness = quick_harness();
    let id = cube_id(&harness);
    let params = mesh::CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    };
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: mesh::Primitive::Cube(params),
    })
    .unwrap();
    doc.set_selection([id]);
    press(&mut harness, Key::Num2);
    let at = on_screen(&harness, Vec3::new(25.0, -50.0, 50.0));
    let edges = |h: &H| h.state().workbench.document().elements(id).cloned();
    // First the loop, with a plain double-click.
    harness.event(Event::PointerMoved(at));
    harness.run();
    for _ in 0..2 {
        button(&mut harness, at, true, Modifiers::NONE);
        button(&mut harness, at, false, Modifiers::NONE);
    }
    let the_loop = edges(&harness).unwrap();
    assert_eq!(the_loop.count(ElementKind::Edges), 8);
    // Wait out the double-click time, so the next two clicks start afresh.
    let start = harness.ctx.input(|i| i.time);
    while harness.ctx.input(|i| i.time) - start < 1.0 {
        harness.event(Event::PointerMoved(at));
        harness.run();
    }
    hold(&mut harness, Modifiers::ALT);
    harness.event(Event::PointerMoved(at));
    harness.run();
    for _ in 0..2 {
        button(&mut harness, at, true, Modifiers::ALT);
        button(&mut harness, at, false, Modifiers::ALT);
    }
    hold(&mut harness, Modifiers::NONE);
    // The ring: the rungs across a belt running the other way, also 8
    // edges on this cube, but different ones.
    let ring = edges(&harness).unwrap();
    assert_eq!(ring.count(ElementKind::Edges), 8);
    assert_ne!(ring, the_loop);
}

#[test]
fn double_clicks_are_as_forgiving_as_windows() {
    let harness = fresh();
    let delay = harness
        .ctx
        .options(|o| o.input_options.max_double_click_delay);
    assert_eq!(delay, crate::DOUBLE_CLICK_SECONDS);
}
