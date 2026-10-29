//! The UV Editor drawn plain while painting (decision 0225): its name and
//! the object's at its top, no tab, no menu line, no 0 and 1, no notes.

use super::paint_layers_tests::painting_the_cube;
use egui_kittest::kittest::Queryable;

#[test]
fn painting_shows_the_uv_editor_plain() {
    let mut harness = painting_the_cube();
    harness.run();
    let uv = &harness.state().workbench.uv_editor;
    assert_eq!(uv.plain_title(), Some("UV · Cube"));
    assert_eq!(uv.bottom_notes(), [None, None], "no notes");
    assert!(
        harness.query_by_label("Snapshots").is_none(),
        "no menu line"
    );
    assert!(harness.query_by_label("Trim").is_none());
}

#[test]
fn with_nothing_selected_it_says_what_to_do() {
    let mut harness = painting_the_cube();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([]);
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.uv_editor.plain_title(),
        Some("UV · select an object to see its UVs")
    );
}

#[test]
fn elsewhere_it_keeps_its_menus_and_notes() {
    let mut harness = painting_the_cube();
    harness
        .state_mut()
        .workbench
        .switch_layout(crate::workbench::LayoutChoice::UvEdit);
    harness.run_steps(12);
    assert_eq!(harness.state().workbench.mode(), crate::palette::Mode::Uv);
    let uv = &harness.state().workbench.uv_editor;
    assert_eq!(uv.plain_title(), None);
    assert!(uv.bottom_notes()[1].is_some());
    assert!(harness.query_by_label("Snapshots").is_some());
}

#[test]
fn the_tool_row_fits_on_one_line_beside_the_view_controls() {
    // Decision 0224: with the notes gone, nothing along the bottom is in
    // the way.
    let mut harness = painting_the_cube();
    harness.set_size(egui::vec2(1440.0, 900.0));
    for _ in 0..4 {
        harness.run();
    }
    let controls = harness
        .get_by_label(crate::viewport::paint_row::VIEW_CONTROLS_LABEL)
        .rect();
    let tools = harness.get_by_label(crate::paint_tools::TOOLS_LABEL).rect();
    assert!(
        tools.left() > controls.right() && (tools.top() - controls.top()).abs() < 1.0,
        "{tools:?} beside {controls:?}"
    );
}
