//! The painting area (decision 0215): the brush bar and the paint tools lie
//! over the viewport and the UV Editor beside it, as one place to paint.

use super::paint_layers_tests::painting_the_cube;
use egui_kittest::kittest::Queryable;

#[test]
fn the_bar_and_the_tools_lie_over_both_views() {
    let mut harness = painting_the_cube();
    // A narrow window: the viewport alone is too narrow for the bar.
    harness.set_size(egui::vec2(1100.0, 800.0));
    for _ in 0..4 {
        harness.run();
    }
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let uv = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect();
    assert!(
        (view.right() - uv.left()).abs() < 12.0,
        "side by side: {view:?} {uv:?}"
    );
    let both = view.union(uv);
    let bar = harness.get_by_label(crate::paint_bar::BAR_LABEL).rect();
    assert!(
        bar.right() > view.right(),
        "the bar reaches over the UV Editor: {bar:?} {view:?}"
    );
    assert!(both.contains_rect(bar), "{bar:?} in {both:?}");
    let tools = harness.get_by_label(crate::paint_tools::TOOLS_LABEL).rect();
    assert!(both.contains_rect(tools), "{tools:?} in {both:?}");
    // Clear of the UV Editor's notes and the viewport's buttons.
    let (shading, navigation) = harness.state().workbench.viewport.bottom_buttons().unwrap();
    for other in [shading, navigation] {
        assert!(!tools.intersects(other), "{tools:?} over {other:?}");
    }
    for note in harness
        .state()
        .workbench
        .uv_editor
        .bottom_notes()
        .into_iter()
        .flatten()
    {
        assert!(!tools.intersects(note), "{tools:?} over {note:?}");
    }
}
