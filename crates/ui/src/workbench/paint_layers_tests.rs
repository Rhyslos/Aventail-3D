//! The Paint layers panel and the Paint layout (decision 0181): Paint
//! mode lays out the Paint panels and puts the layout back after; the
//! panel adds, changes and reorders layers, one undo step each, and the
//! brush paints on the chosen layer.

use super::LayoutChoice;
use super::brushing_tests::{painting, stroke};
use super::tests::camera;
use crate::palette::Mode;
use crate::panel::Panel;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec3;
use paint::{Blend, Content, TextureSet};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Paint mode with the cube selected.
pub(super) fn painting_the_cube() -> H {
    let mut harness = painting();
    let cube = harness.state().workbench.document().scene().walk()[0].0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([cube]);
    harness.run();
    harness.run();
    harness
}

pub(super) fn set(harness: &H) -> TextureSet {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .set("Default")
        .map(|s| (**s).clone())
        .expect("the material has a texture set")
}

pub(super) fn names(harness: &H) -> Vec<String> {
    let set = set(harness);
    set.listed(true)
        .iter()
        .map(|(p, _)| set.layer(p).unwrap().name.clone())
        .collect()
}

pub(super) fn press(harness: &mut H, label: &str) {
    // The rarer layer buttons are in the ⋯ menu under the list
    // (decision 0227): open it first.
    if harness.query_by_label(label).is_none()
        && harness
            .query_by_label(crate::paint_layers::MORE_LABEL)
            .is_some()
    {
        harness
            .get_by_label(crate::paint_layers::MORE_LABEL)
            .scroll_to_me();
        harness.run();
        harness
            .get_by_label(crate::paint_layers::MORE_LABEL)
            .click();
        harness.run();
    }
    harness.get_by_label(label).scroll_to_me();
    harness.run();
    harness.get_by_label(label).click();
    // Slow work (made layers, packing) is said first and done a frame
    // later: a few frames more than settling takes.
    harness.run_steps(8);
}

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

pub(super) fn paint_front(harness: &mut H) {
    let from = on_screen(harness, Vec3::new(-20.0, -50.0, 50.0));
    let to = on_screen(harness, Vec3::new(20.0, -50.0, 50.0));
    stroke(harness, from, to, Modifiers::NONE, PointerButton::Primary);
}

#[test]
fn paint_mode_lays_out_the_paint_panels_and_puts_the_layout_back() {
    let mut harness = painting_the_cube();
    let bench = &harness.state().workbench;
    assert_eq!(bench.layout_choice(), LayoutChoice::Paint);
    assert!(bench.dock.find_tab(&Panel::PaintLayers).is_some());
    assert!(bench.dock.find_tab(&Panel::UvEditor).is_some());
    // Leaving Paint mode: the standard layout, as it was.
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.layout_choice(), LayoutChoice::Standard);
    assert!(bench.dock.find_tab(&Panel::PaintLayers).is_none());
    // The Paint tab enters Paint mode; Standard leaves it.
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::Paint);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Paint);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::Standard);
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Model);
}

#[test]
fn from_uv_edit_paint_mode_goes_back_to_uv_edit() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.state_mut().workbench.mode = Mode::Paint;
    harness.run();
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::Paint
    );
    harness.state_mut().workbench.mode = Mode::Uv;
    harness.run();
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::UvEdit
    );
}

#[test]
fn the_panel_adds_layers_above_the_chosen_one_and_the_brush_paints_there() {
    let mut harness = painting_the_cube();
    assert!(
        harness
            .query_by_label(crate::paint_layers::LAYER_LIST_LABEL)
            .is_some()
    );
    paint_front(&mut harness);
    assert_eq!(names(&harness), ["Paint"]);
    // A fill above it, chosen.
    press(&mut harness, "+ Fill");
    assert_eq!(names(&harness), ["Fill", "Paint"]);
    assert!(matches!(set(&harness).layers[1].content, Content::Fill(_)));
    // Painting with the fill chosen makes a paint layer above it.
    paint_front(&mut harness);
    assert_eq!(names(&harness), ["Paint 2", "Fill", "Paint"]);
    // Choosing the bottom layer paints there.
    harness.get_by_label("Layer Paint").click();
    harness.run();
    let before = set(&harness).layers[0].strokes.len();
    paint_front(&mut harness);
    assert_eq!(set(&harness).layers[0].strokes.len(), before + 1);
    assert_eq!(names(&harness).len(), 3, "no new layer");
    // Each change is one undo step.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert_eq!(names(&harness), ["Paint"]);
}

#[test]
fn layers_move_group_merge_and_go() {
    let mut harness = painting_the_cube();
    paint_front(&mut harness);
    press(&mut harness, "+ Paint");
    assert_eq!(names(&harness), ["Paint 2", "Paint"]);
    press(&mut harness, "Down");
    assert_eq!(names(&harness), ["Paint", "Paint 2"]);
    press(&mut harness, "Up");
    assert_eq!(names(&harness), ["Paint 2", "Paint"]);
    press(&mut harness, "Group");
    assert_eq!(names(&harness), ["Group", "Paint 2", "Paint"]);
    assert!(set(&harness).layers[1].children().is_some());
    press(&mut harness, "Ungroup");
    assert_eq!(names(&harness), ["Paint 2", "Paint"]);
    press(&mut harness, "Merge down");
    assert_eq!(names(&harness), ["Paint"]);
    press(&mut harness, "Duplicate");
    assert_eq!(names(&harness), ["Paint 2", "Paint"]);
    press(&mut harness, "Delete");
    assert_eq!(names(&harness), ["Paint"]);
}

#[test]
fn the_eye_hides_and_the_settings_change_the_chosen_layer() {
    let mut harness = painting_the_cube();
    paint_front(&mut harness);
    press(&mut harness, "Hide Paint");
    assert!(!set(&harness).layers[0].visible);
    press(&mut harness, "Show Paint");
    assert!(set(&harness).layers[0].visible);
    press(&mut harness, "Clip to the layer below");
    assert!(set(&harness).layers[0].clipped);
    // The blend mode from its list.
    harness
        .get_all_by_label("Blend")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::ComboBox)
        .expect("the blend mode choice")
        .click();
    harness.run();
    press(&mut harness, "Multiply");
    assert_eq!(set(&harness).layers[0].blend, Blend::Multiply);
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert_eq!(set(&harness).layers[0].blend, Blend::Normal);
}

#[test]
fn the_paint_layout_is_kept_between_sessions() {
    let mut harness = painting_the_cube();
    let tabs =
        |d: &egui_dock::DockState<Panel>| d.iter_all_tabs().map(|(_, p)| *p).collect::<Vec<_>>();
    let saved = harness.state().workbench.paint_layout().clone();
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    assert_eq!(tabs(harness.state().workbench.paint_layout()), tabs(&saved));
    let restored = crate::layout::restore_paint_layout(Some(saved.clone()));
    assert_eq!(tabs(&restored), tabs(&saved));
    // A saved layout without the Paint layers is replaced.
    let restored = crate::layout::restore_paint_layout(Some(crate::layout::default_layout()));
    assert!(restored.find_tab(&Panel::PaintLayers).is_some());
}
