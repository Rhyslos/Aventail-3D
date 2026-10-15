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
    harness.get_by_label("Rotation H").click();
    harness.run();
    harness.get_by_label("Rotation H").type_text("90");
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
fn with_symmetry_moving_a_point_moves_its_mirror() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    // The palette switch turns it on, across X as standard.
    harness.get_by_label("Symmetry").click();
    harness.run();
    assert!(harness.state().workbench.viewport().options().symmetry.on);
    assert!(
        harness
            .query_by_label_contains("Symmetry on, across X")
            .is_some()
    );
    press(&mut harness, Key::Num1);
    // Its X, Y and Z switches are in the menu beside the switch.
    assert!(harness.query_by_label("Symmetry X").is_none());
    harness
        .get_by_label(crate::palette::SYMMETRY_MENU_LABEL)
        .click();
    harness.run();
    assert!(harness.query_by_label("Symmetry X").is_some());
    harness.key_press(Key::Escape);
    harness.run();
    // One top corner on the right.
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
    let mesh = shape(&harness);
    let moved = mesh.position(right).unwrap();
    assert!(moved.x > 70.0, "{moved}");
    // Its mirror went the other way, by as much.
    let left = mesh
        .vertices()
        .find(|(v, p)| *v != right && p.y > 0.0 && p.z > 99.0 && p.x < -70.0)
        .map(|(_, p)| p)
        .expect("the mirror moved left");
    assert!((left.x + moved.x).abs() < 1e-3, "{left} {moved}");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Move 2 points"), "one undo step");
    // Off again (the Tools menu, or the palette, whose button shares its
    // name with the panel's section now).
    harness.state_mut().workbench.toggle_symmetry();
    harness.run();
    assert!(!harness.state().workbench.viewport().options().symmetry.on);
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
fn the_symmetry_menu_sets_the_axes_and_where_the_mirror_goes() {
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
    // Turned on, across Y as well, through the world centre.
    harness.get_by_label("On").click();
    harness.run();
    harness.get_by_label("Symmetry Y").click();
    harness.run();
    harness.get_by_label("World centre").click();
    harness.run();
    let symmetry = harness.state().workbench.viewport().options().symmetry;
    assert!(symmetry.on);
    assert_eq!(symmetry.axes, [true, true, false]);
    assert_eq!(symmetry.centre, crate::SymmetryCentre::World);
    // Through the world centre, an object moved to x = 100 mirrors about
    // its own x = -100.
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
    let mirror = harness.state().workbench.mirror_for(id).unwrap();
    assert!(mirror.centre.distance(glam::Vec3::new(-100.0, 0.0, 0.0)) < 1e-3);
    // Its middle: the box's centre, moved by the offset.
    let options = harness.state_mut().workbench.viewport_mut().options_mut();
    options.symmetry.centre = crate::SymmetryCentre::Middle;
    options.symmetry.offset = [5.0, 0.0, 0.0];
    let mirror = harness.state().workbench.mirror_for(id).unwrap();
    assert!(mirror.centre.distance(glam::Vec3::new(5.0, 0.0, 50.0)) < 1e-3);
    // Off: no mirror.
    harness.state_mut().workbench.toggle_symmetry();
    assert!(harness.state().workbench.mirror_for(id).is_none());
}

#[test]
fn with_symmetry_points_can_be_kept_from_crossing_the_middle() {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(scene::Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 1, 1],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    harness.state_mut().workbench.toggle_symmetry();
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .symmetry
        .clip = true;
    press(&mut harness, Key::Num1);
    let mesh = shape(&harness);
    let right = mesh
        .vertices()
        .find(|(_, p)| p.x > 49.0 && p.y < -49.0 && p.z > 99.0)
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
    // Dragged far to the left: it stops at the middle.
    let start = x_arrow(&harness);
    let path: Vec<Pos2> = (1..=8)
        .map(|i| along_x(&harness, start, -(i as f32) * 20.0))
        .collect();
    drag_without_letting_go(&mut harness, start, &path);
    let_go(&mut harness, *path.last().unwrap());
    let moved = shape(&harness).position(right).unwrap();
    assert!(moved.x.abs() < 1e-3, "{moved}");
}

#[test]
fn weld_the_middle_now_joins_the_halves_once_when_asked() {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(scene::Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 1, 1],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    let middle = shape(&harness)
        .vertices()
        .find(|(_, p)| p.x.abs() < 1e-3)
        .map(|(v, _)| v)
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::MovePoints {
                points: vec![middle],
                offset: glam::Vec3::new(0.05, 0.0, 0.0),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    // Nothing joins by itself: the point stays off the middle.
    assert!((shape(&harness).position(middle).unwrap().x - 0.05).abs() < 1e-5);
    harness
        .get_by_label(crate::palette::SYMMETRY_MENU_LABEL)
        .click();
    harness.run();
    harness
        .get_by_label(crate::palette::WELD_MIDDLE_LABEL)
        .click();
    harness.run();
    assert_eq!(shape(&harness).position(middle).unwrap().x, 0.0);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Weld the middle")
    );
    // Ctrl + Z takes it back; with nothing near the middle, nothing is
    // recorded.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!((shape(&harness).position(middle).unwrap().x - 0.05).abs() < 1e-5);
    harness
        .state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .symmetry
        .weld_distance = 0.01;
    let steps = harness.state().workbench.document().undo_steps();
    harness.state_mut().workbench.weld_middle_now();
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}
