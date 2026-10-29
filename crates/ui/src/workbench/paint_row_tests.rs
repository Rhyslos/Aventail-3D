//! The Paint page's tool row (decision 0224): the view's controls (☰, 3D,
//! Lit, Show) in place of its menu line, dice and buttons, then select,
//! move, rotate and scale, then the paint tools with blur, the bucket and
//! the colour picker.

use super::brushing_tests::{painted_pixels, stroke};
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use crate::attributes::PaintTool;
use crate::transforming::Tool;
use crate::viewport::paint_row::{CAMERA_LABEL, VIEW_CONTROLS_LABEL, VIEW_MENUS_LABEL};
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::BrushKind;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn click(harness: &mut H, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

fn strokes(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .sets()
        .map(|(_, s)| s.layers.iter().map(|l| l.strokes.len()).sum::<usize>())
        .sum()
}

fn drag_across_the_front(harness: &mut H) {
    let from = on_screen(harness, Vec3::new(-30.0, -50.0, 50.0));
    let to = on_screen(harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(harness, from, to, Modifiers::NONE, PointerButton::Primary);
}

#[test]
fn the_view_controls_replace_the_menu_line_dice_and_buttons() {
    let harness = painting_the_cube();
    assert!(harness.query_by_label(VIEW_CONTROLS_LABEL).is_some());
    assert!(harness.query_by_label("Cameras").is_none(), "no menu line");
    assert!(
        harness
            .query_by_label(crate::viewport::DICE_LABEL)
            .is_none()
    );
    assert!(
        harness
            .query_by_label(crate::viewport::SHADING_LABEL)
            .is_none()
    );
    assert!(
        harness
            .query_by_label(crate::viewport::PICKER_LABEL)
            .is_none()
    );
    // The tools sit right after the controls, level with them.
    let controls = harness.get_by_label(VIEW_CONTROLS_LABEL).rect();
    let tools = harness.get_by_label(crate::paint_tools::TOOLS_LABEL).rect();
    let beside = tools.left() > controls.right() && (tools.top() - controls.top()).abs() < 1.0;
    let above = tools.bottom() < controls.top();
    assert!(beside || above, "{tools:?} after {controls:?}");
}

#[test]
fn the_menus_and_cameras_are_in_the_controls() {
    let mut harness = painting_the_cube();
    harness.get_by_label(VIEW_MENUS_LABEL).click();
    harness.run();
    // The view's menus, as submenus.
    assert!(harness.query_by_label("Cameras ⏵").is_some());
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.get_by_label(CAMERA_LABEL).click();
    harness.run();
    harness
        .get_by_label(&format!("{} F2", tools::ViewKind::Top.name()))
        .click();
    harness.run();
    assert_eq!(
        harness.state().workbench.viewport.active_kind(),
        tools::ViewKind::Top
    );
}

#[test]
fn move_puts_the_brush_down_and_a_paint_key_picks_it_up() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Move (W)");
    assert_eq!(harness.state().workbench.tool, Tool::Move);
    drag_across_the_front(&mut harness);
    assert_eq!(strokes(&harness), 0, "nothing painted while moving");
    harness.key_press(egui::Key::Num1);
    harness.run();
    assert_eq!(harness.state().workbench.tool, Tool::Select);
    drag_across_the_front(&mut harness);
    assert_eq!(strokes(&harness), 1);
}

#[test]
fn the_select_arrow_selects_instead_of_painting() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Select (Q)");
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([]);
    harness.run();
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    click(&mut harness, at);
    harness.run();
    assert_eq!(strokes(&harness), 0);
    assert_eq!(harness.state().workbench.document().selection().len(), 1);
    // A paint tool's button: painting again.
    press(&mut harness, "Paint brush (1)");
    assert!(!harness.state().workbench.paint_hands().selecting);
}

#[test]
fn seven_is_blur_and_eight_the_bucket_which_fills_the_whole_material() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num7);
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.kind,
        BrushKind::Blur
    );
    // Polygon fill: one side of the cube.
    harness.key_press(egui::Key::Num4);
    harness.run();
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let one_side = painted_pixels(&harness);
    assert!(one_side > 0);
    harness.key_press(egui::Key::Num8);
    harness.run();
    let brush = harness.state().workbench.attributes.brush;
    assert!(brush.tool == PaintTool::Fill && brush.bucket);
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let all = painted_pixels(&harness);
    assert!(
        all > one_side * 4,
        "the bucket fills every side: {all} vs {one_side}"
    );
}

#[test]
fn the_picker_takes_the_colour_clicked() {
    let mut harness = painting_the_cube();
    // Paint a red band first.
    harness.state_mut().workbench.attributes.brush.brush.color = [220, 30, 30, 255];
    harness.key_press(egui::Key::Num4);
    harness.run();
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    harness.state_mut().workbench.attributes.brush.brush.color = [10, 10, 200, 255];
    press(&mut harness, "Pick a colour (P)");
    assert!(harness.state().workbench.attributes.picking);
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let colour = harness.state().workbench.attributes.brush.brush.color;
    assert!(colour[0] > 150 && colour[2] < 100, "picked red: {colour:?}");
    assert!(!harness.state().workbench.attributes.picking, "once");
}
