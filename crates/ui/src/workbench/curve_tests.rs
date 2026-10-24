//! Tests for curves in the window (decision 0138): drawing with clicks
//! and drags, finishing, closing and cancelling, the Create menu, shape
//! lists and command search, the Points mode (moving, deleting, the
//! double-click), picking a curve's line, and the Attributes panel.

use super::modelling_tests::{H, button, note, on_screen, press};
use super::tests::{fresh, main_menu, quick_harness};
use super::*;
use crate::shape_picker::{Shape, ShapeList};
use crate::transforming::{Axes, Target, Tool};
use egui::{Event, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use scene::{
    Curve, CurveObject, CurvePart, CurvePointId, CurveShape, CurveSolid, Element, ElementSelection,
    ObjectId, ObjectKind, ReadyCurveKind, Sweep, VertexId,
};
use tools::Change;

fn click(harness: &mut H, at: Pos2, which: PointerButton) {
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(harness, at, true, which, Modifiers::NONE);
    button(harness, at, false, which, Modifiers::NONE);
    harness.run();
}

fn drag(harness: &mut H, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(harness, from, true, PointerButton::Primary, Modifiers::NONE);
    for k in 1..=6 {
        harness.event(Event::PointerMoved(from + (to - from) * (k as f32 / 6.0)));
        harness.run();
    }
    button(harness, to, false, PointerButton::Primary, Modifiers::NONE);
    harness.run();
}

/// The fresh window (a cube in the middle) with Draw curve picked up.
fn drawing() -> H {
    let mut harness = fresh();
    harness.state_mut().workbench.start_drawing_curve();
    harness.run();
    harness
}

/// Spots on the ground in front of the cube.
fn ground(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

/// The curve objects in the scene.
fn curves(harness: &H) -> Vec<ObjectId> {
    let scene = harness.state().workbench.document().scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| scene.object(*id).unwrap().kind.curve().is_some())
        .collect()
}

fn curve_of(harness: &H, id: ObjectId) -> CurveObject {
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
        .clone()
}

/// The curve's points, in the world.
fn world_points(harness: &H, id: ObjectId) -> Vec<Vec3> {
    let scene = harness.state().workbench.document().scene();
    let matrix = scene.shape_matrix(id);
    curve_of(harness, id)
        .shape
        .curve()
        .points()
        .iter()
        .map(|p| matrix.transform_point3(p.position))
        .collect()
}

fn three_clicks(harness: &mut H) -> [Vec3; 3] {
    let spots = [
        ground(-150.0, -150.0),
        ground(0.0, -200.0),
        ground(150.0, -150.0),
    ];
    for spot in spots {
        let at = on_screen(harness, spot);
        click(harness, at, PointerButton::Primary);
    }
    spots
}

#[test]
fn clicks_place_sharp_points_and_enter_finishes_in_one_step() {
    let mut harness = drawing();
    assert_eq!(harness.state().workbench.tool(), Tool::DrawCurve);
    assert!(
        note(&harness).starts_with("Draw curve: click"),
        "{}",
        note(&harness)
    );
    let steps = harness.state().workbench.document().undo_steps();
    let spots = three_clicks(&mut harness);
    // Nothing is made until it is finished; the line shows meanwhile.
    assert!(curves(&harness).is_empty());
    let path = harness.state().workbench.viewport().tool_path().unwrap();
    assert_eq!(path.dots.as_ref().map(Vec::len), Some(3));
    assert!(
        !path.preview.is_empty(),
        "the next stretch follows the mouse"
    );
    press(&mut harness, Modifiers::NONE, Key::Enter);
    let made = curves(&harness);
    assert_eq!(made.len(), 1);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Draw curve"));
    assert_eq!(doc.selection(), made);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    let points = world_points(&harness, made[0]);
    for (p, spot) in points.iter().zip(spots) {
        assert!(p.distance(spot) < 1.0, "{p} {spot}");
    }
    let curve = curve_of(&harness, made[0]).shape.curve();
    assert!(curve.points().iter().all(|p| !p.smooth && !p.has_handles()));
    assert!(!curve.closed());
    // The pivot sits in the middle of the points.
    let pivot = doc.scene().world_matrix(made[0]).w_axis.truncate();
    assert!(pivot.distance(Vec3::new(0.0, -175.0, 0.0)) < 1.0, "{pivot}");
    // One undo takes the whole curve back.
    harness.state_mut().workbench.document_mut().undo();
    assert!(curves(&harness).is_empty());
}

#[test]
fn pressing_and_dragging_pulls_out_handles_for_a_smooth_bend() {
    let mut harness = drawing();
    let first = on_screen(&harness, ground(-150.0, -150.0));
    click(&mut harness, first, PointerButton::Primary);
    let second = on_screen(&harness, ground(0.0, -200.0));
    let pulled = on_screen(&harness, ground(60.0, -200.0));
    drag(&mut harness, second, pulled);
    let third = on_screen(&harness, ground(150.0, -150.0));
    click(&mut harness, third, PointerButton::Primary);
    let path = harness.state().workbench.viewport().tool_path().unwrap();
    assert_eq!(
        path.handles.len(),
        2,
        "the dragged point shows both handles"
    );
    press(&mut harness, Modifiers::NONE, Key::Enter);
    let id = curves(&harness)[0];
    let curve = curve_of(&harness, id).shape.curve();
    let middle = curve.points()[1];
    assert!(middle.smooth);
    // Its out handle points where the mouse went, its in handle opposite.
    assert!(middle.handle_out.x > 40.0, "{}", middle.handle_out);
    assert!(middle.handle_in.distance(-middle.handle_out) < 1e-3);
}

#[test]
fn a_right_click_finishes_and_esc_cancels() {
    let mut harness = drawing();
    three_clicks(&mut harness);
    let away = on_screen(&harness, ground(0.0, -300.0));
    click(&mut harness, away, PointerButton::Secondary);
    assert_eq!(curves(&harness).len(), 1);
    assert_eq!(
        curve_of(&harness, curves(&harness)[0])
            .shape
            .curve()
            .points()
            .len(),
        3
    );
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    // Esc: nothing made, the tool put away.
    harness.state_mut().workbench.start_drawing_curve();
    harness.run();
    three_clicks(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(curves(&harness).len(), 1);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(note(&harness).contains("cancelled"), "{}", note(&harness));
}

#[test]
fn backspace_takes_the_last_point_back() {
    let mut harness = drawing();
    three_clicks(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    // Nothing deleted from the scene: the cube is still there.
    assert_eq!(harness.state().workbench.document().scene().len(), 1);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    let id = curves(&harness)[0];
    assert_eq!(curve_of(&harness, id).shape.curve().points().len(), 2);
}

#[test]
fn one_point_makes_nothing() {
    let mut harness = drawing();
    let at = on_screen(&harness, ground(0.0, -150.0));
    click(&mut harness, at, PointerButton::Primary);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(curves(&harness).is_empty());
    assert!(note(&harness).contains("two points"), "{}", note(&harness));
}

#[test]
fn clicking_the_first_point_closes_the_curve() {
    let mut harness = drawing();
    let spots = three_clicks(&mut harness);
    let first = on_screen(&harness, spots[0]);
    click(&mut harness, first, PointerButton::Primary);
    let made = curves(&harness);
    assert_eq!(made.len(), 1);
    let curve = curve_of(&harness, made[0]).shape.curve();
    assert!(curve.closed());
    assert_eq!(curve.points().len(), 3, "the first point isn't doubled");
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
}

#[test]
fn another_mode_or_tool_keeps_the_curve_drawn_so_far() {
    let mut harness = drawing();
    three_clicks(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::W);
    assert_eq!(curves(&harness).len(), 1);
    assert_eq!(harness.state().workbench.tool(), Tool::Move);
}

#[test]
fn snapping_puts_points_on_the_grid() {
    let mut harness = drawing();
    harness.state_mut().workbench.toggle_snapping();
    harness.run();
    for spot in [ground(-147.0, -152.0), ground(48.0, -203.0)] {
        let at = on_screen(&harness, spot);
        click(&mut harness, at, PointerButton::Primary);
    }
    press(&mut harness, Modifiers::NONE, Key::Enter);
    let id = curves(&harness)[0];
    let step =
        crate::transforming::grid_step(harness.state().workbench.viewport().options().grid_detail);
    for p in world_points(&harness, id) {
        let off = (p / step).round() * step - p;
        assert!(off.length() < 0.01, "{p} isn't on the {step} cm grid");
    }
}

#[test]
fn curves_are_in_the_more_list_the_create_menu_and_search() {
    let more = ShapeList::More.kinds();
    for shape in Shape::CURVES {
        assert!(more.contains(&shape), "{shape:?}");
        assert!(!ShapeList::Basic.kinds().contains(&shape));
    }
    assert_eq!(more.last(), Some(&Shape::Curve(ReadyCurveKind::Spiral)));
    // Command search finds each of them.
    let mut harness = fresh();
    let ctx = harness.ctx.clone();
    let entries = harness.state().workbench.search_entries(&ctx);
    for name in ["Draw curve", "Line", "Circle", "Arc", "Spiral"] {
        assert!(entries.iter().any(|e| e.name == name), "{name}");
    }
    // The Create menu makes a ready-made circle.
    main_menu(&harness, "Create").click();
    harness.run();
    harness.get_by_label("Circle").click();
    harness.run();
    let made = curves(&harness);
    assert_eq!(made.len(), 1);
    assert!(matches!(
        curve_of(&harness, made[0]).shape,
        CurveShape::Ready(scene::ReadyCurve::Circle { .. })
    ));
    // And picks up Draw curve.
    main_menu(&harness, "Create").click();
    harness.run();
    harness.get_by_label("Draw curve").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::DrawCurve);
}

#[test]
fn alt_b_reaches_the_curves() {
    let mut harness = fresh();
    // Shift + Alt + B opens the More list at its bottom: the spiral.
    harness.key_press_modifiers(Modifiers::SHIFT | Modifiers::ALT, Key::B);
    harness.run();
    assert_eq!(
        harness.state().workbench.shape_picker().highlighted(),
        Some(Shape::Curve(ReadyCurveKind::Spiral))
    );
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(curves(&harness).len(), 1);
}

/// A drawn curve of three sharp points, selected, in the Points mode.
fn editing_points() -> (H, ObjectId) {
    editing_points_in(fresh())
}

/// The same in `harness`.
fn editing_points_in(mut harness: H) -> (H, ObjectId) {
    let curve = Curve::corners(
        &[
            ground(-150.0, -150.0),
            ground(0.0, -200.0),
            ground(150.0, -150.0),
        ],
        false,
    )
    .unwrap();
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(ObjectKind::Curve(
        CurveObject::line(CurveShape::Points(curve)),
    )))
    .unwrap();
    let id = doc.selection()[0];
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    (harness, id)
}

fn select_part(harness: &mut H, id: ObjectId, part: CurvePart) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                ElementSelection::of([Element::Point(VertexId::from_raw(part.code()))]),
            )],
            "Select",
            None,
        );
    harness.run();
}

#[test]
fn the_points_mode_finds_a_curves_points() {
    let (mut harness, id) = editing_points();
    let at = on_screen(&harness, ground(0.0, -200.0));
    click(&mut harness, at, PointerButton::Primary);
    let selected: Vec<VertexId> = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .unwrap()
        .points()
        .collect();
    let middle = CurvePart::point(CurvePointId::from_raw(1));
    assert_eq!(selected, vec![VertexId::from_raw(middle.code())]);
    // The overlay draws the curve's line and its points.
    let overlay = harness.state().workbench.components.overlay();
    assert!(
        overlay
            .lines
            .iter()
            .any(|(_, s)| *s == crate::OverlayStyle::CurveSelected)
    );
    assert!(
        overlay
            .points
            .iter()
            .any(|(_, s)| *s == crate::OverlayStyle::Handle)
    );
}

#[test]
fn moving_curve_points_is_a_curve_edit() {
    let (mut harness, id) = editing_points();
    let middle = CurvePart::point(CurvePointId::from_raw(1));
    select_part(&mut harness, id, middle);
    let workbench = &mut harness.state_mut().workbench;
    let summary = workbench
        .drawing
        .update(
            &workbench.document,
            Some(tools::ElementKind::Points),
            &workbench.hidden,
        )
        .clone();
    let target = Target::of(
        &workbench.document,
        &summary.drawing,
        Mode::Points,
        Axes::World,
    )
    .unwrap();
    assert!(target.pivot.distance(ground(0.0, -200.0)) < 1e-3);
    let commands = target.commands(&Change::Translate(Vec3::new(0.0, 0.0, 30.0)));
    assert!(matches!(commands[0], scene::Command::EditCurve { .. }));
    for command in commands {
        workbench.document.execute(command).unwrap();
    }
    let points = world_points(&harness, id);
    assert!(points[1].distance(ground(0.0, -200.0) + Vec3::Z * 30.0) < 1e-3);
    assert_eq!(points[0], ground(-150.0, -150.0));
}

#[test]
fn delete_takes_curve_points_away() {
    let (mut harness, id) = editing_points();
    select_part(
        &mut harness,
        id,
        CurvePart::point(CurvePointId::from_raw(1)),
    );
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(curve_of(&harness, id).shape.curve().points().len(), 2);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Delete curve points")
    );
    // Down to two, a point can't go.
    select_part(
        &mut harness,
        id,
        CurvePart::point(CurvePointId::from_raw(0)),
    );
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(curve_of(&harness, id).shape.curve().points().len(), 2);
    assert!(note(&harness).contains("two points"), "{}", note(&harness));
}

#[test]
fn a_double_click_switches_a_point_between_sharp_and_smooth() {
    // Frames 1/60 s apart, so two clicks make a double-click.
    let (mut harness, id) = editing_points_in(quick_harness());
    let at = on_screen(&harness, ground(0.0, -200.0));
    harness.event(Event::PointerMoved(at));
    harness.run();
    for _ in 0..2 {
        button(
            &mut harness,
            at,
            true,
            PointerButton::Primary,
            Modifiers::NONE,
        );
        button(
            &mut harness,
            at,
            false,
            PointerButton::Primary,
            Modifiers::NONE,
        );
    }
    harness.run();
    let point = curve_of(&harness, id).shape.curve().points()[1];
    assert!(point.smooth && point.has_handles(), "{point:?}");
    assert!(
        note(&harness).starts_with("Point made smooth"),
        "{}",
        note(&harness)
    );
}

#[test]
fn the_edges_mode_says_a_curve_has_none() {
    let (mut harness, _) = editing_points();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    assert!(
        note(&harness).contains("no edges or polygons"),
        "{}",
        note(&harness)
    );
}

#[test]
fn clicking_a_curves_line_selects_it() {
    let (mut harness, id) = editing_points();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    // Halfway along the first straight stretch.
    let at = on_screen(&harness, ground(-75.0, -175.0));
    click(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state().workbench.document().selection(), vec![id]);
}

#[test]
fn the_attributes_panel_shows_the_curve_and_its_solid() {
    let (mut harness, id) = editing_points();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    harness.run();
    assert!(
        harness
            .query_all_by_label("What the curve makes")
            .next()
            .is_some()
    );
    assert!(harness.query_all_by_label("Curve closed").next().is_some());
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetCurveSolid {
            ids: vec![id],
            solid: CurveSolid::Sweep(Sweep::default()),
        })
        .unwrap();
    harness.run();
    for name in [
        "Cross-section",
        "Width",
        "Sides",
        "Segments",
        "Twist",
        "Caps",
    ] {
        assert!(harness.query_all_by_label(name).next().is_some(), "{name}");
    }
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetCurveSolid {
            ids: vec![id],
            solid: CurveSolid::Lathe(scene::LatheShape::default()),
        })
        .unwrap();
    harness.run();
    for name in ["Lathe axis", "Spin angle", "Segments round", "Adaptive"] {
        assert!(harness.query_all_by_label(name).next().is_some(), "{name}");
    }
}
