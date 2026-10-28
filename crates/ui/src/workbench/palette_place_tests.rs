//! Where the modifiers palette sits (decision 0176): beside Attributes,
//! never between the viewport and the UV Editor, and not at all in the
//! UV mode, where nothing on it works on UVs.

use super::LayoutChoice;
use super::tests::fresh;
use crate::palette::Mode;
use egui_kittest::kittest::Queryable;

#[test]
fn the_modifiers_palette_is_beside_attributes_and_gone_in_the_uv_mode() {
    let mut harness = fresh();
    // Standard layout, Model mode: the palette on the viewport's right,
    // touching Attributes.
    let palette = harness.get_by_label(crate::layers::ADD_LAYER_LABEL).rect();
    let attributes = harness.get_by_label("Attributes").rect();
    assert!(attributes.left() - palette.right() < 20.0);

    // UV Edit switches to the UV mode: no palette.
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Uv);
    assert!(
        harness
            .query_by_label(crate::layers::ADD_LAYER_LABEL)
            .is_none(),
        "no modifiers palette in the UV mode"
    );

    // Another mode in UV Edit: the palette comes back, on the UV Editor's
    // right beside Attributes, not between the two views.
    harness.state_mut().workbench.mode = Mode::Polygons;
    harness.run();
    harness.run();
    let palette = harness.get_by_label(crate::layers::ADD_LAYER_LABEL).rect();
    let editor = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect();
    let attributes = harness.get_by_label("Attributes").rect();
    assert!(
        palette.left() >= editor.left(),
        "the palette is right of the UV Editor's start, not between the views: {palette:?} {editor:?}"
    );
    assert!(
        attributes.left() - palette.right() < 20.0,
        "the palette touches Attributes: {palette:?} {attributes:?}"
    );
}
