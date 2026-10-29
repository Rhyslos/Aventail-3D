//! The paint tools along the viewport's bottom (decision 0213).

use super::paint_layers_tests::{painting_the_cube, press};
use crate::attributes::PaintTool;
use crate::palette::Mode;
use egui_kittest::kittest::Queryable;
use paint::BrushKind;

#[test]
fn the_tools_show_while_painting_and_choose_the_tool() {
    let mut harness = painting_the_cube();
    assert!(harness.query_by_label(crate::paint_tools::TOOLS_LABEL).is_some());
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let row = harness.get_by_label(crate::paint_tools::TOOLS_LABEL).rect();
    assert!(view.contains_rect(row), "{row:?} in {view:?}");
    press(&mut harness, "Eraser (2)");
    let brush = harness.state().workbench.attributes.brush;
    assert_eq!(brush.tool, PaintTool::Brush);
    assert_eq!(brush.brush.kind, BrushKind::Erase);
    press(&mut harness, "Polygon fill (4)");
    assert_eq!(
        harness.state().workbench.attributes.brush.tool,
        PaintTool::Fill
    );
    press(&mut harness, "Paint brush (1)");
    let brush = harness.state().workbench.attributes.brush;
    assert_eq!(brush.tool, PaintTool::Brush);
    assert_eq!(brush.brush.kind, BrushKind::Paint);
}

#[test]
fn a_click_on_a_tool_paints_nothing() {
    let mut harness = painting_the_cube();
    let strokes = |h: &egui_kittest::Harness<'static, super::tests::State>| {
        h.state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .map(|s| s.layers.iter().map(|l| l.strokes.len()).sum::<usize>())
            .unwrap_or(0)
    };
    let before = strokes(&harness);
    press(&mut harness, "Smudge (5)");
    press(&mut harness, "Paint brush (1)");
    assert_eq!(strokes(&harness), before);
}

#[test]
fn the_tools_are_only_there_while_painting() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.mode = Mode::Model;
    harness.run();
    harness.run();
    assert!(harness.query_by_label(crate::paint_tools::TOOLS_LABEL).is_none());
}
