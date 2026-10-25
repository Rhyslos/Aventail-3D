//! Tests for spline editing in the window (decision 0139): a double-click
//! on a curve's line adds a point (and on a point still switches it), M
//! and A on end points, Join objects on curves, carrying a curve on from
//! either end, the curve commands' readiness in every case, the E menu's
//! curve ring, the Mesh menu's Curve list, command search and the
//! Attributes panel's Curve points section.

use super::modelling_tests::{H, button, note, on_screen, press};
use super::tests::{fresh, mesh_group, quick_harness};
use super::*;
use crate::commands::CommandId;
use crate::tool_menu::Ready;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec3;
use scene::{
    Curve, CurveObject, CurvePart, CurvePointId, CurveShape, Element, ElementSelection, ObjectId,
    ObjectKind, PointKind, VertexId,
};

fn click(harness: &mut H, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(harness, at, true, PointerButton::Primary, Modifiers::NONE);
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
    harness.run();
}

fn double_click(harness: &mut H, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.run();
    for _ in 0..2 {
        button(harness, at, true, PointerButton::Primary, Modifiers::NONE);
        button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
    }
    harness.run();
}

fn click_on(harness: &mut H, world: Vec3) {
    let at = on_screen(harness, world);
    click(harness, at);
}

fn double_click_on(harness: &mut H, world: Vec3) {
    let at = on_screen(harness, world);
    double_click(harness, at);
}

fn ground(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

/// Adds a curve of sharp corners in `harness`, selected; returns its id.
fn add_curve(harness: &mut H, points: &[Vec3], closed: bool) -> ObjectId {
    let curve = Curve::corners(points, closed).unwrap();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(ObjectKind::Curve(
        CurveObject::line(CurveShape::Points(curve)),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    id
}

/// A bent line of four points across the front of the view.
fn four() -> [Vec3; 4] {
    [
        ground(-150.0, -150.0),
        ground(-50.0, -220.0),
        ground(50.0, -220.0),
        ground(150.0, -150.0),
    ]
}

/// `harness` with the four-point curve, selected, in the Points mode.
fn editing(mut harness: H) -> (H, ObjectId) {
    let id = add_curve(&mut harness, &four(), false);
    press(&mut harness, Modifiers::NONE, Key::Num1);
    (harness, id)
}

fn curve_of(harness: &H, id: ObjectId) -> Curve {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .object(id)
        .unwrap()
        .kind
        .curve()
        .unwrap()
        .shape
        .curve()
}

fn world_points(harness: &H, id: ObjectId) -> Vec<Vec3> {
    let scene = harness.state().workbench.document().scene();
    let matrix = scene.shape_matrix(id);
    curve_of(harness, id)
        .points()
        .iter()
        .map(|p| matrix.transform_point3(p.position))
        .collect()
}

fn curves(harness: &H) -> Vec<ObjectId> {
    let scene = harness.state().workbench.document().scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| scene.object(*id).unwrap().kind.curve().is_some())
        .collect()
}

/// Selects these points of these curves (and the curves).
fn select_points(harness: &mut H, chosen: &[(ObjectId, u32)]) {
    let mut by_object: Vec<(ObjectId, Vec<Element>)> = Vec::new();
    for (id, point) in chosen {
        let code = CurvePart::point(CurvePointId::from_raw(*point)).code();
        let element = Element::Point(VertexId::from_raw(code));
        match by_object.iter_mut().find(|(o, _)| o == id) {
            Some((_, list)) => list.push(element),
            None => by_object.push((*id, vec![element])),
        }
    }
    let doc = harness.state_mut().workbench.document_mut();
    // One at a time, so the order they were selected is the order given.
    doc.clear_selection();
    for k in 1..=by_object.len() {
        doc.set_selection(by_object[..k].iter().map(|(id, _)| *id));
    }
    doc.select_elements(
        by_object
            .into_iter()
            .map(|(id, list)| (id, ElementSelection::of(list))),
        "Select",
        None,
    );
    harness.run();
}

fn ready(harness: &H, command: CommandId) -> Ready {
    harness.state().workbench.readiness(command)
}

// --------------------------------------------- the double-click on a line

#[test]
fn a_double_click_on_the_line_adds_a_point_keeping_the_shape() {
    let (mut harness, id) = editing(quick_harness());
    let before = curve_of(&harness, id);
    let steps = harness.state().workbench.document().undo_steps();
    // A quarter of the way along the middle stretch.
    let spot = ground(-25.0, -220.0);
    double_click_on(&mut harness, spot);
    let after = curve_of(&harness, id);
    assert_eq!(after.points().len(), 5, "{}", note(&harness));
    let added = world_points(&harness, id)[2];
    assert!(added.distance(spot) < 3.0, "{added}");
    // Straight stretches stay straight: the new point is a corner on them.
    assert_eq!(after.points()[2].kind, PointKind::Sharp);
    assert_eq!(after.points()[2].id, CurvePointId::from_raw(4));
    assert!(
        note(&harness).starts_with("Point added"),
        "{}",
        note(&harness)
    );
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Add curve point"));
    assert_eq!(doc.undo_steps(), steps + 1);
    harness.state_mut().workbench.document_mut().undo();
    assert_eq!(curve_of(&harness, id), before);
}

#[test]
fn a_double_click_on_a_point_still_switches_it_and_adds_nothing() {
    let (mut harness, id) = editing(quick_harness());
    double_click_on(&mut harness, four()[1]);
    let curve = curve_of(&harness, id);
    assert_eq!(curve.points().len(), 4);
    assert_eq!(curve.points()[1].kind, PointKind::Smooth);
    assert!(note(&harness).starts_with("Point made smooth"));
    // Its handle ends take a double-click too: back to sharp.
    let end = world_points(&harness, id)[1] + curve.points()[1].handle_out;
    double_click_on(&mut harness, end);
    let curve = curve_of(&harness, id);
    assert_eq!(curve.points().len(), 4);
    assert_eq!(curve.points()[1].kind, PointKind::Sharp);
}

#[test]
fn a_double_click_away_from_the_line_adds_nothing() {
    let (mut harness, id) = editing(quick_harness());
    double_click_on(&mut harness, ground(0.0, -100.0));
    assert_eq!(curve_of(&harness, id).points().len(), 4);
}

// ------------------------------------------------------- M and A on ends

#[test]
fn m_on_a_curves_two_ends_welds_them_into_a_loop() {
    let (mut harness, id) = editing(fresh());
    select_points(&mut harness, &[(id, 0), (id, 3)]);
    press(&mut harness, Modifiers::NONE, Key::M);
    let curve = curve_of(&harness, id);
    assert!(curve.closed(), "{}", note(&harness));
    assert_eq!(curve.points().len(), 3);
    assert!(note(&harness).contains("welded"), "{}", note(&harness));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Weld curve ends")
    );
}

#[test]
fn a_on_a_curves_two_ends_closes_it_with_a_stretch() {
    let (mut harness, id) = editing(fresh());
    select_points(&mut harness, &[(id, 0), (id, 3)]);
    press(&mut harness, Modifiers::NONE, Key::A);
    let curve = curve_of(&harness, id);
    assert!(curve.closed());
    assert_eq!(curve.points().len(), 4);
}

#[test]
fn m_and_a_on_two_curves_ends_join_them_into_the_first() {
    for (key, count, label) in [
        (Key::M, 6, "Weld curve ends"),
        (Key::A, 7, "Connect curve ends"),
    ] {
        let (mut harness, a) = editing(fresh());
        let b = add_curve(
            &mut harness,
            &[
                ground(170.0, -150.0),
                ground(250.0, -100.0),
                ground(300.0, -50.0),
            ],
            false,
        );
        select_points(&mut harness, &[(a, 3), (b, 0)]);
        press(&mut harness, Modifiers::NONE, key);
        assert_eq!(curves(&harness), vec![a], "{}", note(&harness));
        assert_eq!(curve_of(&harness, a).points().len(), count);
        let doc = harness.state().workbench.document();
        assert_eq!(doc.undo_label(), Some(label));
        assert_eq!(doc.selection(), vec![a]);
    }
}

#[test]
fn m_and_a_say_why_when_the_points_arent_two_ends() {
    for (key, word) in [(Key::M, "Weld"), (Key::A, "Connect")] {
        let (mut harness, id) = editing(fresh());
        select_points(&mut harness, &[(id, 1), (id, 3)]);
        press(&mut harness, Modifiers::NONE, key);
        assert!(
            note(&harness).starts_with(word) && note(&harness).contains("ends of open curves"),
            "{}",
            note(&harness)
        );
        select_points(&mut harness, &[(id, 0), (id, 1), (id, 3)]);
        press(&mut harness, Modifiers::NONE, key);
        assert!(
            note(&harness).contains("just the two ends"),
            "{}",
            note(&harness)
        );
        assert!(!curve_of(&harness, id).closed());
    }
}

// ----------------------------------------------------------- Join objects

#[test]
fn join_objects_joins_curves_end_to_end() {
    let mut harness = fresh();
    let a = add_curve(&mut harness, &[ground(0.0, 0.0), ground(100.0, 0.0)], false);
    let b = add_curve(
        &mut harness,
        &[ground(200.0, 0.0), ground(100.0, 0.0)],
        false,
    );
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([a]);
    doc.set_selection([a, b]);
    harness.run();
    assert_eq!(ready(&harness, CommandId::Join), Ready::Yes);
    let ctx = harness.ctx.clone();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::Join, &ctx, &mut requests);
    for request in requests {
        if let crate::objects::DocRequest::Run(command) = request {
            harness
                .state_mut()
                .workbench
                .document_mut()
                .execute(command)
                .unwrap();
        }
    }
    harness.run();
    assert_eq!(curves(&harness).len(), 1);
    let xs: Vec<f32> = world_points(&harness, curves(&harness)[0])
        .iter()
        .map(|p| p.x.round())
        .collect();
    assert_eq!(xs.len(), 3, "the ends on one spot melt");
    // A loop among them can't be joined: greyed out with the reason.
    let first = curves(&harness)[0];
    let c = add_curve(
        &mut harness,
        &[ground(0.0, 50.0), ground(50.0, 50.0), ground(0.0, 90.0)],
        true,
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([first, c]);
    harness.run();
    match ready(&harness, CommandId::Join) {
        Ready::NotNow(why) => assert!(why.contains("no ends"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// -------------------------------------------------- carrying a curve on

fn carry_on(from_start: bool) {
    let mut harness = fresh();
    let id = add_curve(
        &mut harness,
        &[ground(-150.0, -150.0), ground(0.0, -200.0)],
        false,
    );
    harness.state_mut().workbench.start_drawing_curve();
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    let end = if from_start {
        ground(-150.0, -150.0)
    } else {
        ground(0.0, -200.0)
    };
    // The selected curve's ends show as dots before anything is drawn.
    let path = harness.state().workbench.viewport().tool_path().unwrap();
    assert_eq!(path.dots.as_ref().map(Vec::len), Some(2));
    click_on(&mut harness, end);
    assert!(
        note(&harness).contains("carrying the curve on"),
        "{}",
        note(&harness)
    );
    for spot in [ground(100.0, -150.0), ground(150.0, -50.0)] {
        click_on(&mut harness, spot);
    }
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(curves(&harness), vec![id], "no new object");
    let points = world_points(&harness, id);
    assert_eq!(points.len(), 4);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "one undo step");
    assert_eq!(doc.undo_label(), Some("Carry on curve"));
    if from_start {
        // In front: the newest point first.
        assert!(points[0].distance(ground(150.0, -50.0)) < 5.0, "{points:?}");
        assert!(
            points[2].distance(ground(-150.0, -150.0)) < 1.0,
            "{points:?}"
        );
    } else {
        assert!(points[3].distance(ground(150.0, -50.0)) < 5.0, "{points:?}");
    }
    harness.state_mut().workbench.document_mut().undo();
    assert_eq!(curve_of(&harness, id).points().len(), 2);
}

#[test]
fn draw_curve_carries_a_curve_on_from_its_last_point() {
    carry_on(false);
}

#[test]
fn draw_curve_carries_a_curve_on_from_its_first_point() {
    carry_on(true);
}

#[test]
fn carrying_a_curve_on_closes_it_on_its_other_end() {
    let mut harness = fresh();
    let id = add_curve(
        &mut harness,
        &[ground(-150.0, -150.0), ground(0.0, -200.0)],
        false,
    );
    harness.state_mut().workbench.start_drawing_curve();
    harness.run();
    click_on(&mut harness, ground(0.0, -200.0));
    click_on(&mut harness, ground(100.0, -100.0));
    click_on(&mut harness, ground(-150.0, -150.0));
    let curve = curve_of(&harness, id);
    assert!(curve.closed(), "{}", note(&harness));
    assert_eq!(curve.points().len(), 3);
}

#[test]
fn clicking_away_from_the_ends_draws_a_new_curve() {
    let mut harness = fresh();
    add_curve(
        &mut harness,
        &[ground(-150.0, -150.0), ground(0.0, -200.0)],
        false,
    );
    harness.state_mut().workbench.start_drawing_curve();
    harness.run();
    click_on(&mut harness, ground(100.0, -100.0));
    click_on(&mut harness, ground(150.0, -50.0));
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(curves(&harness).len(), 2);
}

// --------------------------------------------------- the curve commands

const POINT_COMMANDS: [CommandId; 6] = [
    CommandId::CurveSplit,
    CommandId::CurveSharp,
    CommandId::CurveSmooth,
    CommandId::CurveBroken,
    CommandId::CurveAuto,
    CommandId::CurveRound,
];
const WHOLE_COMMANDS: [CommandId; 3] = [
    CommandId::CurveReverse,
    CommandId::CurveOpenClose,
    CommandId::CurveEven,
];

fn not_now(harness: &H, command: CommandId) -> String {
    match ready(harness, command) {
        Ready::NotNow(why) => why,
        other => panic!("{command:?} is {other:?}"),
    }
}

#[test]
fn curve_commands_are_greyed_out_with_a_reason() {
    // The cube selected, not a curve.
    let mut harness = fresh();
    let cube = harness.state().workbench.document().scene().walk()[0].0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([cube]);
    harness.run();
    for command in POINT_COMMANDS.iter().chain(&WHOLE_COMMANDS) {
        assert!(not_now(&harness, *command).contains("curve"), "{command:?}");
    }
    // A curve in the Model mode: whole-curve commands work, point ones ask
    // for the Points mode.
    let id = add_curve(&mut harness, &four(), false);
    for command in WHOLE_COMMANDS {
        assert_eq!(ready(&harness, command), Ready::Yes, "{command:?}");
    }
    for command in POINT_COMMANDS {
        assert!(
            not_now(&harness, command).contains("press 1"),
            "{command:?}"
        );
    }
    // The Points mode with nothing selected: select points first.
    press(&mut harness, Modifiers::NONE, Key::Num1);
    for command in POINT_COMMANDS {
        assert!(
            not_now(&harness, command).contains("Select some"),
            "{command:?}"
        );
    }
    // An end point: handle types yes, split and round no.
    select_points(&mut harness, &[(id, 0)]);
    assert_eq!(ready(&harness, CommandId::CurveSmooth), Ready::Yes);
    assert!(not_now(&harness, CommandId::CurveSplit).contains("ends cut nothing"));
    assert!(not_now(&harness, CommandId::CurveRound).contains("sharp corners"));
    // A sharp corner inside: all of them.
    select_points(&mut harness, &[(id, 1)]);
    for command in POINT_COMMANDS.iter().chain(&WHOLE_COMMANDS) {
        assert_eq!(ready(&harness, *command), Ready::Yes, "{command:?}");
    }
    // A two-point curve can't close.
    let two = add_curve(&mut harness, &[ground(0.0, 0.0), ground(10.0, 0.0)], false);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([two]);
    harness.run();
    assert!(not_now(&harness, CommandId::CurveOpenClose).contains("three points"));
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut requests);
    assert!(requests.is_empty());
    harness.run();
}

#[test]
fn curve_commands_do_what_they_say_and_undo() {
    let (mut harness, id) = editing(fresh());
    select_points(&mut harness, &[(id, 1), (id, 2)]);
    let start = curve_of(&harness, id);
    for (command, label) in [
        (CommandId::CurveAuto, "Handle type: Auto"),
        (CommandId::CurveBroken, "Handle type: Broken"),
        (CommandId::CurveSmooth, "Handle type: Smooth"),
        (CommandId::CurveSharp, "Handle type: Sharp"),
        (CommandId::CurveRound, "Round corners"),
        (CommandId::CurveReverse, "Reverse curve"),
        (CommandId::CurveOpenClose, "Open or close curve"),
        (CommandId::CurveEven, "Even spacing"),
    ] {
        let before = curve_of(&harness, id);
        run(&mut harness, command);
        let doc = harness.state().workbench.document();
        assert_eq!(
            doc.undo_label(),
            Some(label),
            "{command:?}: {}",
            note(&harness)
        );
        assert!(note(&harness).contains("Ctrl + Z"), "{}", note(&harness));
        let after = curve_of(&harness, id);
        match command {
            CommandId::CurveAuto | CommandId::CurveBroken | CommandId::CurveSmooth => {
                let kind = after.point(CurvePointId::from_raw(1)).unwrap().kind;
                assert_ne!(kind, PointKind::Sharp, "{command:?}");
            }
            CommandId::CurveRound => assert_eq!(after.points().len(), 6),
            CommandId::CurveOpenClose => assert!(after.closed()),
            _ => assert_ne!(after, before, "{command:?}"),
        }
        harness.state_mut().workbench.document_mut().undo();
        assert_eq!(curve_of(&harness, id), before, "{command:?}");
        run(&mut harness, command);
    }
    while curve_of(&harness, id) != start {
        harness.state_mut().workbench.document_mut().undo();
    }
}

#[test]
fn split_curve_makes_two_objects_selected() {
    let (mut harness, id) = editing(fresh());
    select_points(&mut harness, &[(id, 1)]);
    run(&mut harness, CommandId::CurveSplit);
    let made = curves(&harness);
    assert_eq!(made.len(), 2, "{}", note(&harness));
    assert_eq!(harness.state().workbench.document().selection().len(), 2);
    assert!(note(&harness).contains("2 curves"), "{}", note(&harness));
}

#[test]
fn even_spacing_takes_its_count_from_the_attributes_panel() {
    let (mut harness, id) = editing(fresh());
    harness.state_mut().workbench.attributes.curve.count = Some(9);
    run(&mut harness, CommandId::CurveEven);
    assert_eq!(curve_of(&harness, id).points().len(), 9);
}

// ------------------------------------------- the E menu, menus, search

#[test]
fn the_e_menu_shows_the_curve_ring_for_curves_in_the_points_mode() {
    let (mut harness, _) = editing(fresh());
    let commands: Vec<CommandId> = harness
        .state()
        .workbench
        .tool_menu_entries()
        .iter()
        .map(|e| e.slot.command)
        .collect();
    for command in [
        CommandId::CurveHandles,
        CommandId::CurveRound,
        CommandId::CurveEven,
        CommandId::CurveSplit,
        CommandId::Weld,
        CommandId::Connect,
    ] {
        assert!(commands.contains(&command), "{command:?}");
    }
    assert!(!commands.contains(&CommandId::Extrude));
    // A mesh selected too: the modelling ring.
    let cube = harness.state().workbench.document().scene().walk()[1].0;
    let curve = curves(&harness)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([cube, curve]);
    harness.run();
    let commands: Vec<CommandId> = harness
        .state()
        .workbench
        .tool_menu_entries()
        .iter()
        .map(|e| e.slot.command)
        .collect();
    assert!(commands.contains(&CommandId::Extrude));
}

#[test]
fn the_mesh_menu_lists_the_curve_commands_greyed_out_with_reasons() {
    let (mut harness, id) = editing(fresh());
    mesh_group(&mut harness, super::mesh_menu::MESH_CURVE_MENU);
    for name in [
        "Split curve",
        "Reverse curve",
        "Open or close curve",
        "Sharp points",
        "Smooth points",
        "Broken handles",
        "Auto handles",
        "Round corners",
        "Even spacing",
    ] {
        assert!(harness.query_all_by_label(name).next().is_some(), "{name}");
    }
    let disabled = |harness: &H, name: &str| {
        harness
            .get_all_by_label(name)
            .all(|n| n.accesskit_node().is_disabled())
    };
    assert!(disabled(&harness, "Split curve"), "no points selected");
    assert!(!disabled(&harness, "Reverse curve"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    select_points(&mut harness, &[(id, 1)]);
    mesh_group(&mut harness, super::mesh_menu::MESH_CURVE_MENU);
    harness
        .get_all_by_label("Split curve")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Split curve can be chosen")
        .click();
    harness.run();
    assert_eq!(curves(&harness).len(), 2);
}

#[test]
fn command_search_finds_the_curve_commands() {
    let (harness, _) = editing(fresh());
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    for name in [
        "Split curve",
        "Round corners",
        "Even spacing",
        "Auto handles",
    ] {
        let entry = entries.iter().find(|e| e.name == name).expect(name);
        assert_eq!(entry.place, "Mesh");
    }
    let split = entries.iter().find(|e| e.name == "Split curve").unwrap();
    assert!(split.not_now.is_some(), "no points selected");
    let even = entries.iter().find(|e| e.name == "Even spacing").unwrap();
    assert!(even.not_now.is_none());
    // The group itself isn't listed: its four are.
    assert!(entries.iter().all(|e| e.name != "Handle type"));
}

// ------------------------------------------------- the Attributes panel

#[test]
fn the_attributes_panel_sets_handle_types_and_runs_the_commands() {
    let (mut harness, id) = editing(fresh());
    assert!(harness.query_by_label("Corner radius").is_some());
    assert!(harness.query_by_label("Points along").is_some());
    // Nothing selected: the handle type buttons are greyed out.
    let kind_button = |harness: &H, name: &str| {
        harness
            .get_all_by_label(name)
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
            .map(|n| n.accesskit_node().is_disabled())
    };
    assert_eq!(kind_button(&harness, "Auto"), Some(true));
    select_points(&mut harness, &[(id, 1)]);
    assert_eq!(kind_button(&harness, "Auto"), Some(false));
    harness
        .get_all_by_label("Auto")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .unwrap()
        .click();
    harness.run();
    harness.run();
    assert_eq!(
        curve_of(&harness, id)
            .point(CurvePointId::from_raw(1))
            .unwrap()
            .kind,
        PointKind::Auto
    );
    // Round, with the radius set in the panel.
    run(&mut harness, CommandId::CurveSharp);
    harness.state_mut().workbench.attributes.curve.radius = 5.0;
    harness.get_by_label("Round").click();
    harness.run();
    harness.run();
    assert_eq!(
        curve_of(&harness, id).points().len(),
        5,
        "{}",
        note(&harness)
    );
    // Space evenly.
    harness.get_by_label("Space evenly").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Even spacing")
    );
    // Not in the Model mode (1 again goes back to it).
    press(&mut harness, Modifiers::NONE, Key::Num1);
    assert!(harness.query_by_label("Corner radius").is_none());
}

#[test]
fn the_keys_window_lists_the_new_mouse_actions() {
    let lines = super::shortcuts::mouse();
    for action in [
        "Add a point to a curve, keeping its shape",
        "Draw curve: carry a selected curve on from an end",
        "Join two curve end points: weld at the middle / a straight stretch",
    ] {
        assert!(lines.iter().any(|b| b.action == action), "{action}");
    }
}
