//! The shelf like the mockup (decision 0229): Brushes, Colours,
//! Materials, Smart masks and Stencils; square cards with names.

use super::paint_layers_tests::{paint_front, painting_the_cube, press, set};
use crate::colour_shelf::{BRUSHES_TAB, MATERIALS_TAB, SMART_MASKS_TAB, STENCILS_TAB};
use egui_kittest::kittest::Queryable;
use paint::{MaterialKind, SmartKind};

#[test]
fn the_shelf_has_five_tabs() {
    let harness = painting_the_cube();
    for tab in [
        BRUSHES_TAB,
        crate::colour_shelf::COLOURS_TAB,
        MATERIALS_TAB,
        SMART_MASKS_TAB,
        STENCILS_TAB,
    ] {
        assert!(harness.query_by_label(tab).is_some(), "{tab}");
    }
    // The brushes show to start with, as square cards.
    assert!(harness.query_by_label("Brush Soft round").is_some());
}

#[test]
fn a_material_card_adds_that_material_above_the_chosen_layer() {
    let mut harness = painting_the_cube();
    paint_front(&mut harness);
    press(&mut harness, MATERIALS_TAB);
    press(&mut harness, "Material Wood");
    let after = set(&harness);
    let top = after.layers.last().unwrap();
    assert_eq!(
        top.material.as_ref().map(|m| m.kind),
        Some(MaterialKind::Wood)
    );
    assert_eq!(top.name, "Wood");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Add a material layer"));
    // Back to the brushes.
    press(&mut harness, BRUSHES_TAB);
    assert!(harness.query_by_label("Brush Soft round").is_some());
}

#[test]
fn a_smart_mask_card_masks_the_chosen_layer_or_makes_a_fill() {
    // No layer yet: a fill shown through the mask.
    let mut harness = painting_the_cube();
    press(&mut harness, SMART_MASKS_TAB);
    assert!(
        harness.query_by_label("Smart mask Rust").is_none(),
        "the realism masks wait for the realism channels"
    );
    press(&mut harness, "Smart mask Cavities");
    let first = set(&harness);
    assert_eq!(first.layers.len(), 1);
    let smart = first.layers[0].mask.as_ref().and_then(|m| m.smart);
    assert_eq!(smart.map(|s| s.kind), Some(SmartKind::Cavities));
    assert!(
        first.layers[0].mask.as_ref().unwrap().image.painted_tiles() > 0,
        "made on the shape"
    );
    // With a layer chosen, it gets the mask, in place of the one it had.
    press(&mut harness, "Smart mask Facing up");
    let second = set(&harness);
    assert_eq!(second.layers.len(), 1);
    let smart = second.layers[0].mask.as_ref().and_then(|m| m.smart);
    assert_eq!(smart.map(|s| s.kind), Some(SmartKind::FacingUp));
}

#[test]
fn the_stencils_tab_chooses_a_picture_and_paints_with_it() {
    let mut harness = painting_the_cube();
    press(&mut harness, STENCILS_TAB);
    // Choosing a picture asks the app for a file.
    press(&mut harness, crate::shelf_cards::CHOOSE_STENCIL);
    // A picture chosen: its card picks the projection brush.
    harness.state_mut().workbench.attributes.stencil_name = Some("bricks.png".into());
    harness.run();
    press(&mut harness, "Stencil bricks.png");
    let brush = &harness.state().workbench.attributes.brush;
    assert_eq!(brush.brush.kind, paint::BrushKind::Projection);
}

#[test]
fn a_material_card_with_nothing_selected_says_why() {
    let mut harness = painting_the_cube();
    press(&mut harness, MATERIALS_TAB);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    press(&mut harness, "Material Stone");
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .sets()
            .next()
            .is_none()
    );
}
