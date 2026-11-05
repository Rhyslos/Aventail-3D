//! The smaller requests from the texture painter review (2026-11-05):
//! Pick on a layer's colour swatch, starter palettes, the glow's
//! strength, the viewport look in the Lit list, and calm colours while
//! painting.

use super::paint_layers_tests::{painting_the_cube, press, set};
use crate::attributes::colour_picker;
use egui_kittest::kittest::{NodeT, Queryable};

type H = egui_kittest::Harness<'static, super::tests::State>;

fn click_cube(harness: &mut H) {
    let at = harness.get_by_label(crate::VIEWPORT_LABEL).rect().center();
    pointer_click(harness, at);
}

/// A real press and release of the left button at `at`.
fn pointer_click(harness: &mut H, at: egui::Pos2) {
    harness.event(egui::Event::PointerMoved(at));
    harness.run();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
}

#[test]
fn a_swatchs_pick_takes_the_colour_for_the_swatch_not_the_brush() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.attributes.brush.brush.color = [10, 20, 30, 255];
    // A swatch's Pick is waiting.
    let target = egui::Id::new("a layer's swatch");
    harness.ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("swatch_pick_target"), Some(target));
    });
    assert!(colour_picker::swatch_pick_waiting(&harness.ctx));
    harness.run();
    assert!(
        harness.state().workbench.attributes.picking,
        "the next click picks"
    );
    click_cube(&mut harness);
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.color,
        [10, 20, 30, 255],
        "the brush keeps its colour"
    );
    assert!(!colour_picker::swatch_pick_waiting(&harness.ctx));
    let picked = harness
        .ctx
        .data(|d| d.get_temp::<Option<(egui::Id, [u8; 3])>>(egui::Id::new("swatch_picked")))
        .flatten();
    assert_eq!(
        picked.map(|(id, _)| id),
        Some(target),
        "handed to the swatch"
    );
    assert!(!harness.state().workbench.attributes.picking);
}

#[test]
fn a_starter_palette_is_copied_into_the_project() {
    let mut harness = painting_the_cube();
    press(&mut harness, crate::colour_shelf::COLOURS_TAB);
    // The palette list, which says "Palette" while there is none.
    let at = harness
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Palette"))
        .unwrap()
        .rect()
        .center();
    harness.event(egui::Event::PointerMoved(at));
    harness.run();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }
    harness.run();
    harness.run();
    harness
        .get_by_label_contains(crate::colour_shelf::STARTER_LABEL)
        .click();
    harness.run();
    harness.get_by_label("Warm wood").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let palettes = doc.scene().paint().palettes();
    assert_eq!(palettes.len(), 1);
    assert_eq!(palettes[0].name, "Warm wood");
    assert_eq!(palettes[0].colours, paint::palette::starters()[0].colours);
    assert_eq!(doc.undo_label(), Some("Add a starter palette"));
}

#[test]
fn the_glow_strength_shows_with_realism_and_is_kept() {
    let mut harness = painting_the_cube();
    press(&mut harness, "+ Fill");
    harness
        .get_by_label(crate::paint_layers::MORE_LABEL)
        .click();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::paint_layers::GLOW_STRENGTH_LABEL)
            .is_none(),
        "only with the realism channels"
    );
    harness
        .get_by_label(crate::paint_layers::REALISM_LABEL)
        .click();
    harness.run();
    harness.run();
    // The number beside the words.
    fn field(harness: &H) -> egui_kittest::Node<'_> {
        harness
            .query_all_by_label(crate::paint_layers::GLOW_STRENGTH_LABEL)
            .last()
            .unwrap()
    }
    field(&harness).scroll_to_me();
    harness.run();
    let at = field(&harness).rect().center();
    pointer_click(&mut harness, at);
    // Clicking it opens the number for typing, all of it chosen.
    harness.event(egui::Event::Text("5".into()));
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(set(&harness).glow_strength, 5.0);
    // The view shows it brighter.
    let view = harness.state().renderer.drawn.last().expect("drawn");
    assert_eq!(view.scene.items[0].glow, 5.0);
}

#[test]
fn the_look_is_in_the_lit_list() {
    let mut harness = painting_the_cube();
    harness
        .get_by_label(crate::viewport::paint_row::LOOK_LABEL)
        .click();
    harness.run();
    harness.get_by_label("Studio").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.viewport.options().look,
        crate::viewport::LookChoice::Studio
    );
}

#[test]
fn the_paint_page_is_calm_slate_and_leaves_out_the_selection_outline() {
    let mut harness = painting_the_cube();
    harness.run();
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::Slate
    );
    let view = harness.state().renderer.drawn.last().expect("drawn");
    assert!(!view.outline_selection);
    // Back to modelling: the chosen accent and the outline again.
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.run();
    assert_eq!(
        crate::theme::accent(&harness.ctx),
        crate::theme::Accent::Red
    );
}
