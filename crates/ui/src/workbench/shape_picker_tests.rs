//! Tests for the shape lists: `B` opens the basic shapes, `B` again moves
//! down it, and `Enter` or `Space` creates the highlighted shape; `Alt +
//! B` does the same for the other shapes (decision 0130).

use super::tests::{State, empty};
use super::*;
use crate::SHAPE_LIST_KEY;
use crate::shape_picker::ShapeList;
use egui::{Event, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use scene::{ObjectKind, PrimitiveKind};

/// The basic shapes, the ones `B` lists.
fn basic() -> Vec<PrimitiveKind> {
    ShapeList::Basic.kinds()
}

fn press(harness: &mut Harness<'static, State>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn highlighted(harness: &Harness<'static, State>) -> Option<PrimitiveKind> {
    harness.state().workbench.shape_picker().highlighted()
}

/// The kinds of the objects in the scene, top to bottom.
fn kinds(harness: &Harness<'static, State>) -> Vec<PrimitiveKind> {
    let scene = harness.state().workbench.document().scene();
    scene
        .children(None)
        .iter()
        .filter_map(|id| match scene.object(*id).unwrap().kind {
            ObjectKind::Primitive(p) => Some(p.kind()),
            _ => None,
        })
        .collect()
}

#[test]
fn b_opens_the_list_on_the_first_shape() {
    let mut harness = empty();
    assert!(!harness.state().workbench.shape_picker().is_open());
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    assert_eq!(highlighted(&harness), Some(basic()[0]));
    assert!(harness.query_by_label("Create a shape").is_some());
    assert!(kinds(&harness).is_empty(), "opening creates nothing");
}

#[test]
fn pressing_b_again_moves_down_the_list_and_round() {
    let mut harness = empty();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    for expected in basic().into_iter().skip(1) {
        press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
        assert_eq!(highlighted(&harness), Some(expected));
    }
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    assert_eq!(highlighted(&harness), Some(basic()[0]), "round");
}

#[test]
fn shift_b_and_the_arrow_keys_move_both_ways() {
    let mut harness = empty();
    press(&mut harness, Modifiers::SHIFT, SHAPE_LIST_KEY);
    let last = *basic().last().unwrap();
    assert_eq!(highlighted(&harness), Some(last), "opens at the bottom");
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(highlighted(&harness), Some(basic()[0]));
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(highlighted(&harness), Some(last));
    press(&mut harness, Modifiers::SHIFT, SHAPE_LIST_KEY);
    let before_last = basic()[basic().len() - 2];
    assert_eq!(highlighted(&harness), Some(before_last));
}

#[test]
fn enter_creates_the_highlighted_shape_selected_and_closes_the_list() {
    let mut harness = empty();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    let chosen = basic()[1];
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(kinds(&harness), [chosen]);
    assert!(!harness.state().workbench.shape_picker().is_open());
    let doc = harness.state().workbench.document();
    assert_eq!(doc.selection(), doc.scene().children(None).to_vec());
    assert_eq!(
        doc.undo_label(),
        Some(format!("Create {}", chosen.name().to_lowercase()).as_str())
    );
}

#[test]
fn space_creates_too() {
    let mut harness = empty();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    press(&mut harness, Modifiers::NONE, Key::Space);
    assert_eq!(kinds(&harness), [PrimitiveKind::ALL[0]]);
}

#[test]
fn escape_closes_the_list_without_creating() {
    let mut harness = empty();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!harness.state().workbench.shape_picker().is_open());
    assert!(harness.query_by_label("Create a shape").is_none());
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(kinds(&harness).is_empty());
}

#[test]
fn clicking_a_shape_in_the_list_creates_it() {
    let mut harness = empty();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    let target = PrimitiveKind::ALL[3];
    let at = harness
        .get_by_label(&format!(
            "{}{}",
            crate::shape_picker::ENTRY_PREFIX,
            target.name()
        ))
        .rect()
        .center();
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
    assert_eq!(kinds(&harness), [target]);
    assert!(!harness.state().workbench.shape_picker().is_open());
}

#[test]
fn clicking_elsewhere_closes_the_list() {
    let mut harness = empty();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    let at = harness.get_by_label(crate::VIEWPORT_LABEL).rect().center();
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
    assert!(!harness.state().workbench.shape_picker().is_open());
    assert!(kinds(&harness).is_empty());
}

#[test]
fn b_types_into_a_name_field_instead_of_opening_the_list() {
    let mut harness = super::tests::fresh();
    let id = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Object name").click();
    harness.run();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    assert!(!harness.state().workbench.shape_picker().is_open());
}

#[test]
fn the_create_menu_mentions_the_key() {
    let mut harness = empty();
    super::tests::main_menu(&harness, "Create").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("B for the basic shapes")
            .is_some()
    );
    assert!(
        harness
            .query_by_label_contains("Alt + B for the rest")
            .is_some()
    );
}

#[test]
fn the_two_lists_hold_every_shape_once() {
    let (basic, more) = (ShapeList::Basic.kinds(), ShapeList::More.kinds());
    assert_eq!(basic.len(), 7);
    assert_eq!(more.len(), 11);
    assert!(more.contains(&PrimitiveKind::Icosphere));
    assert!(basic.iter().all(|k| !more.contains(k)), "no shape in both");
    assert_eq!(basic.len() + more.len(), PrimitiveKind::ALL.len());
}

#[test]
fn alt_b_opens_more_shapes_and_moves_down_it() {
    let mut harness = empty();
    press(&mut harness, Modifiers::ALT, SHAPE_LIST_KEY);
    let more = ShapeList::More.kinds();
    let picker = |h: &Harness<'static, State>| h.state().workbench.shape_picker().list();
    assert_eq!(picker(&harness), Some(ShapeList::More));
    assert_eq!(highlighted(&harness), Some(more[0]));
    assert!(harness.query_by_label("More shapes").is_some());
    press(&mut harness, Modifiers::ALT, SHAPE_LIST_KEY);
    assert_eq!(highlighted(&harness), Some(more[1]));
    // Shift + Alt + B goes back; Enter creates.
    press(
        &mut harness,
        Modifiers::SHIFT | Modifiers::ALT,
        SHAPE_LIST_KEY,
    );
    assert_eq!(highlighted(&harness), Some(more[0]));
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(kinds(&harness), [more[0]]);
    assert!(!harness.state().workbench.shape_picker().is_open());
}

#[test]
fn b_and_alt_b_switch_between_the_lists() {
    let mut harness = empty();
    let picker = |h: &Harness<'static, State>| h.state().workbench.shape_picker().list();
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    assert_eq!(picker(&harness), Some(ShapeList::Basic));
    press(&mut harness, Modifiers::ALT, SHAPE_LIST_KEY);
    assert_eq!(picker(&harness), Some(ShapeList::More));
    assert_eq!(highlighted(&harness), Some(ShapeList::More.kinds()[0]));
    press(&mut harness, Modifiers::NONE, SHAPE_LIST_KEY);
    assert_eq!(picker(&harness), Some(ShapeList::Basic));
    assert_eq!(highlighted(&harness), Some(basic()[0]));
    // Shift + Alt + B opens More at its bottom.
    press(&mut harness, Modifiers::NONE, Key::Escape);
    press(
        &mut harness,
        Modifiers::SHIFT | Modifiers::ALT,
        SHAPE_LIST_KEY,
    );
    assert_eq!(
        highlighted(&harness),
        ShapeList::More.kinds().last().copied()
    );
}
