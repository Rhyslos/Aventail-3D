//! Sculpt mode looks and works like Paint mode (decision 0311): the plain
//! page, the tool row and its left-bar column, the brush bar, the Look
//! and Show lists, and the Sculpt tab's folding sections.

use super::material_mode_tests::click;
use super::sculpt_tests::{front, sculpting, shape};
use super::*;
use egui::{Event, Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;

type H = super::heavy_check::H;

fn last_view(harness: &H) -> &crate::ViewportView {
    harness.state().renderer.drawn.last().expect("drawn")
}

#[test]
fn sculpting_has_the_plain_page() {
    let mut harness = sculpting();
    harness.run();
    assert!(harness.query_by_label("Undo").is_none(), "no toolbar");
    assert!(harness.query_by_label("Points mode").is_none(), "no modes");
    assert!(
        harness
            .query_by_label(crate::layers::ADD_LAYER_LABEL)
            .is_none(),
        "no modifiers palette"
    );
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::Slate
    );
    let view = last_view(&harness);
    assert!(!view.outline_selection);
    assert!(!view.show_grid);
    assert!(!view.show_edges);
    assert!(harness.state().workbench.viewport.plain_page());
    // The view's controls in place of its menu line.
    assert!(
        harness
            .query_by_label(crate::viewport::paint_row::VIEW_CONTROLS_LABEL)
            .is_some()
    );
    // Leaving: all of it back.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Undo").is_some());
    assert!(harness.query_by_label("Points mode").is_some());
    assert!(!harness.state().workbench.viewport.plain_page());
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::Red
    );
}

#[test]
fn sculpting_shows_one_view_and_the_grid_only_when_ticked() {
    let mut harness = sculpting();
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    harness.event(Event::PointerMoved(rect.center()));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: rect.center(),
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    assert!(!harness.state().workbench.viewport.four_views());
    assert_eq!(harness.query_all_by_label(crate::VIEWPORT_LABEL).count(), 1);
    // The Show list's Grid, the same switch as the Paint page's.
    harness
        .get_by_label(crate::viewport::paint_row::SHOW_LABEL)
        .click();
    harness.run();
    harness
        .get_by_label(crate::viewport::look::PAINT_GRID_LABEL)
        .click();
    harness.run();
    assert!(last_view(&harness).show_grid);
    assert!(harness.state().workbench.view_options().paint_grid);
}

#[test]
fn the_tool_row_picks_brushes_and_the_transform_tools() {
    let mut harness = sculpting();
    assert!(
        harness
            .query_by_label(crate::sculpt_tools::TOOLS_LABEL)
            .is_some()
    );
    harness.get_by_label("Grab (6)").click();
    harness.run();
    assert_eq!(harness.state().workbench.attributes.sculpt.brush, sculpt::BrushKind::Grab);
    harness.get_by_label("Move (W)").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool, Tool::Move);
    harness.get_by_label("Draw (1)").click();
    harness.run();
    let workbench = &harness.state().workbench;
    assert_eq!(workbench.tool, Tool::Select);
    assert_eq!(workbench.attributes.sculpt.brush, sculpt::BrushKind::Draw);
    // The row sits right after the view's controls, level with them.
    let controls = harness
        .get_by_label(crate::viewport::paint_row::VIEW_CONTROLS_LABEL)
        .rect();
    let row = harness.get_by_label(crate::sculpt_tools::TOOLS_LABEL).rect();
    assert!(row.left() > controls.right());
    assert!((row.top() - controls.top()).abs() < 0.5);
}

#[test]
fn the_select_arrow_selects_instead_of_sculpting() {
    let mut harness = sculpting();
    let before = shape(&harness);
    harness.key_press(egui::Key::Q);
    harness.run();
    assert!(harness.state().workbench.sculpting.selecting);
    // A click on the sphere sculpts nothing; one on empty space
    // deselects, as clicks do with the select arrow.
    let (_, at) = front(&harness);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(shape(&harness).vertex_count(), before.vertex_count());
    let moved = shape(&harness)
        .vertices()
        .zip(before.vertices())
        .any(|((_, a), (_, b))| a != b);
    assert!(!moved, "nothing sculpted");
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    click(&mut harness, view.left_top() + egui::vec2(200.0, 200.0), Modifiers::NONE);
    assert!(harness.state().workbench.document().selection().is_empty());
    // A brush's key puts the arrow down.
    harness.key_press(egui::Key::Num1);
    harness.run();
    assert!(!harness.state().workbench.sculpting.selecting);
    // So does a brush picked on the shelf.
    harness.key_press(egui::Key::Q);
    harness.run();
    assert!(harness.state().workbench.sculpting.selecting);
    harness.state_mut().workbench.attributes.sculpt.brush = sculpt::BrushKind::Clay;
    harness.run();
    assert!(!harness.state().workbench.sculpting.selecting);
}

#[test]
fn the_left_bar_holds_only_the_sculpt_tools_when_set() {
    let mut harness = sculpting();
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .paint_tools_left = true;
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::sculpt_tools::COLUMN_LABEL)
            .is_some()
    );
    assert!(
        harness
            .query_by_label(crate::sculpt_tools::TOOLS_LABEL)
            .is_none(),
        "not along the bottom too"
    );
    assert!(harness.query_by_label("Points mode").is_none(), "tools only");
    harness.get_by_label("Inflate (3)").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.sculpt.brush,
        sculpt::BrushKind::Inflate
    );
}

#[test]
fn the_look_and_show_lists_change_the_sculpts_look() {
    let mut harness = sculpting();
    // The tool row's Look button (the Sculpt tab's Look section has the
    // same name).
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    harness
        .query_all_by_label(crate::viewport::paint_row::LOOK_LABEL)
        .find(|n| view.contains_rect(n.rect()))
        .expect("the Look button")
        .click();
    harness.run();
    harness.get_by_label("Red wax").click();
    harness.run();
    harness.run();
    let sculpt = &harness.state().workbench.attributes.sculpt;
    assert_eq!(sculpt.look, crate::sculpt_tab::SculptLook::RedWax);
    assert_eq!(last_view(&harness).matcap, Some(1));
    harness
        .get_by_label(crate::viewport::paint_row::SHOW_LABEL)
        .click();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::viewport::look::EDGES_LABEL)
            .is_none(),
        "no edges while sculpting"
    );
    harness
        .get_by_label(crate::viewport::chrome::CAVITY_ENTRY)
        .click();
    harness.run();
    harness.run();
    assert!(!harness.state().workbench.attributes.sculpt.cavity);
    assert!(!last_view(&harness).cavity);
}

#[test]
fn the_brush_bar_sits_at_the_views_top_with_its_switches() {
    let mut harness = sculpting();
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let bar = harness.get_by_label(crate::sculpt_tab::BAR_LABEL).rect();
    assert!(bar.top() - view.top() < 12.0, "no menu line above it");
    // Only in the bar: the Sculpt tab leaves its numbers out, as the
    // Paint tab does.
    assert_eq!(
        harness
            .query_all_by_label(crate::sculpt_tab::SIZE_LABEL)
            .count(),
        1
    );
    harness
        .get_by_label(crate::sculpt_bar::SYMMETRY_SWITCH)
        .click();
    harness.run();
    assert!(harness.state().workbench.attributes.sculpt.symmetry.on);
    harness.get_by_label(crate::sculpt_bar::PEN_SWITCH).click();
    harness.run();
    let sculpt = &harness.state().workbench.attributes.sculpt;
    assert!(!sculpt.size_pressure.on && !sculpt.strength_pressure.on);
    harness
        .get_by_label(crate::sculpt_bar::SMOOTHING_SWITCH)
        .click();
    harness.run();
    assert!(harness.state().workbench.attributes.sculpt.smoothing_on);
}

#[test]
fn the_sculpt_tab_folds_its_sections_and_keeps_them() {
    let mut harness = sculpting();
    // Closed to start with, saying what's inside.
    assert!(harness.query_by_label("Radial").is_none());
    let symmetry = harness.get_by_label("Symmetry").rect();
    harness.key_press(egui::Key::L);
    harness.run();
    assert!(harness.query_by_label("Radial").is_none());
    // Opened, it shows its rows, and stays open.
    click(&mut harness, symmetry.center(), Modifiers::NONE);
    harness.run();
    assert!(harness.query_by_label("Radial").is_some());
    harness.run();
    assert!(harness.query_by_label("Radial").is_some());
}
