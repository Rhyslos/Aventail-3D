//! Variations and the dice in the Material mode (decision 0280), used
//! through the real interface: the canvas bar's Variations menu saves the
//! graph's look and flips between looks; Attributes (nothing selected)
//! uses, renames, updates and deletes them; the dice give every seed a
//! new number and a box's menu rolls one box's; each is one undo step.
//! A graph fill in the painter picking a variation is made with its look.

use super::material_mode_tests::{
    LEVELS, NOISE, TILES, box_at, graph, page, point_at_canvas, undo,
};
use super::paint_graph_tests::{add_graph_fill, moss_on_top, top_fill, wait, with_graph};
use crate::material::variations::{
    DELETE_LABEL, DICE_LABEL, ROLL_BOX_LABEL, SAVE_LABEL, UPDATE_LABEL, USE_LABEL, VARIATIONS_LABEL,
};
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use matgraph::{BoxId, GraphEdit, SettingValue, VariationId};
use scene::Command;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn note(harness: &H) -> String {
    harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string()
}

fn steps(harness: &H) -> usize {
    harness.state().workbench.document().undo_steps()
}

fn seed_of(harness: &H, id: BoxId) -> SettingValue {
    graph(harness)
        .get(id)
        .unwrap()
        .settings
        .get("seed")
        .unwrap_or(SettingValue::Whole(0))
}

/// Changes the graph shown, as a change in Attributes would.
fn edit(harness: &mut H, edit: GraphEdit) {
    let bench = &mut harness.state_mut().workbench;
    let id = bench.material().graph.unwrap();
    bench
        .document_mut()
        .execute(Command::EditGraph { id, edit })
        .unwrap();
    harness.run();
}

fn levels_low(harness: &H) -> Option<SettingValue> {
    graph(harness).get(LEVELS).unwrap().settings.get("low")
}

/// Opens the canvas bar's Variations menu (its title starts with
/// "Variations").
fn variations_menu(harness: &mut H) {
    // The bar's menu, not Attributes' heading: the topmost.
    harness
        .get_all_by_label_contains(VARIATIONS_LABEL)
        .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap()
        .click();
    harness.run();
}

/// Clicks the topmost node labelled `label` (the bar's menu opens under
/// the bar, above Attributes' rows).
fn click_topmost(harness: &mut H, label: &str) {
    harness
        .get_all_by_label(label)
        .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap()
        .click();
    harness.run();
}

fn save_from_the_bar(harness: &mut H) {
    variations_menu(harness);
    click_topmost(harness, SAVE_LABEL);
}

/// `page` with Levels' Low shown in the painter and two variations saved
/// from the bar: Low 0.1 ("Variation 1"), then Low 0.6 ("Variation 2").
fn two_looks() -> (H, VariationId, VariationId) {
    let mut harness = page();
    edit(
        &mut harness,
        GraphEdit::ShowInPainter {
            node: LEVELS,
            key: "low".into(),
            shown: true,
        },
    );
    let set_low = |low| GraphEdit::SetSetting {
        node: LEVELS,
        key: "low".into(),
        value: SettingValue::Number(low),
    };
    edit(&mut harness, set_low(0.1));
    let first = VariationId(graph(&harness).next_id());
    save_from_the_bar(&mut harness);
    edit(&mut harness, set_low(0.6));
    let second = VariationId(graph(&harness).next_id());
    save_from_the_bar(&mut harness);
    (harness, first, second)
}

#[test]
fn the_bar_saves_the_look_as_a_variation_in_one_undo_step() {
    let mut harness = page();
    let before = steps(&harness);
    save_from_the_bar(&mut harness);
    assert_eq!(steps(&harness), before + 1);
    let g = graph(&harness);
    let names: Vec<&str> = g.variations().map(|(_, v)| v.name.as_str()).collect();
    assert_eq!(names, ["Variation 1"]);
    assert!(note(&harness).contains("Saved the graph's look as Variation 1"));
    // The bar names the variation in use.
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Variations: Variation 1")
            .is_some()
    );
    // Its seeds are held: Noise's and Tiles' (at their standard).
    let (_, v) = g.variations().next().unwrap();
    assert!(v.value(NOISE, "seed").is_some() && v.value(TILES, "seed").is_some());
    undo(&mut harness);
    assert_eq!(graph(&harness).variations().count(), 0);
}

#[test]
fn the_bar_flips_between_variations_each_one_undo_step() {
    let (mut harness, _, _) = two_looks();
    assert_eq!(levels_low(&harness), Some(SettingValue::Number(0.6)));
    let before = steps(&harness);
    variations_menu(&mut harness);
    click_topmost(&mut harness, "Variation 1");
    assert_eq!(levels_low(&harness), Some(SettingValue::Number(0.1)));
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("Using the variation Variation 1"));
    variations_menu(&mut harness);
    click_topmost(&mut harness, "Variation 2");
    assert_eq!(levels_low(&harness), Some(SettingValue::Number(0.6)));
    undo(&mut harness);
    assert_eq!(levels_low(&harness), Some(SettingValue::Number(0.1)));
}

#[test]
fn attributes_use_update_rename_and_delete_variations() {
    let (mut harness, first, second) = two_looks();
    // Nothing selected: the graph's Attributes list both.
    point_at_canvas(&mut harness);
    harness.run();
    // Variation 2 is in use, so only Variation 1 has Use.
    harness.get_by_label(USE_LABEL).click();
    harness.run();
    assert_eq!(levels_low(&harness), Some(SettingValue::Number(0.1)));
    // Changed, then Update keeps the change in the variation in use's
    // neighbour: Variation 2 (not in use) gets Low 0.3.
    edit(
        &mut harness,
        GraphEdit::SetSetting {
            node: LEVELS,
            key: "low".into(),
            value: SettingValue::Number(0.3),
        },
    );
    let before = steps(&harness);
    assert_eq!(harness.get_all_by_label(UPDATE_LABEL).count(), 2);
    harness
        .get_all_by_label(UPDATE_LABEL)
        .nth(1)
        .unwrap()
        .click();
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let g = graph(&harness);
    assert_eq!(
        g.variation(second).unwrap().value(LEVELS, "low"),
        Some(SettingValue::Number(0.3))
    );
    assert_eq!(
        g.variation_in_use(matgraph::Registry::standard()),
        Some(second)
    );
    // Renamed by typing in its name field: one undo step for the typing.
    let field = || egui_kittest::kittest::by().value("Variation 1");
    harness.get_all(field()).next().unwrap().click();
    harness.run();
    harness.get_all(field()).next().unwrap().type_text("x");
    harness.run();
    // (The click puts the cursor where it lands, inside the name.)
    let renamed = graph(&harness).variation(first).unwrap().name.clone();
    assert!(
        renamed.contains('x') && renamed.len() == "Variation 1x".len(),
        "{renamed}"
    );
    // Deleted (the first Delete is the renamed one's).
    harness
        .get_all_by_label(DELETE_LABEL)
        .next()
        .unwrap()
        .click();
    harness.run();
    assert!(graph(&harness).variation(first).is_none());
    undo(&mut harness);
    assert!(graph(&harness).variation(first).is_some());
}

#[test]
fn the_dice_roll_every_seed_in_one_undo_step() {
    let mut harness = page();
    let (noise, tiles, seed) = (
        seed_of(&harness, NOISE),
        seed_of(&harness, TILES),
        graph(&harness).seed(),
    );
    let before = steps(&harness);
    harness.get_all_by_label(DICE_LABEL).next().unwrap().click();
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    assert_ne!(seed_of(&harness, NOISE), noise);
    assert_ne!(seed_of(&harness, TILES), tiles);
    assert_ne!(graph(&harness).seed(), seed);
    assert!(note(&harness).contains("Rolled the dice"));
    let rolled = seed_of(&harness, NOISE);
    undo(&mut harness);
    assert_eq!(seed_of(&harness, NOISE), noise);
    assert_eq!(graph(&harness).seed(), seed);
    // Rolled again after undo: another take.
    harness.get_all_by_label(DICE_LABEL).next().unwrap().click();
    harness.run();
    assert_ne!(seed_of(&harness, NOISE), rolled);
}

fn right_click(harness: &mut H, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

#[test]
fn a_box_s_menu_rolls_only_its_seed() {
    let mut harness = page();
    let (tiles, seed) = (seed_of(&harness, TILES), graph(&harness).seed());
    let (noise_box, _) = box_at(&mut harness, NOISE);
    right_click(&mut harness, noise_box.left_top() + egui::vec2(20.0, 6.0));
    let before = steps(&harness);
    harness.get_by_label(ROLL_BOX_LABEL).click();
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    assert_ne!(seed_of(&harness, NOISE), SettingValue::Whole(0));
    assert_eq!(seed_of(&harness, TILES), tiles);
    assert_eq!(graph(&harness).seed(), seed);
    // Levels has no seed: its entry is greyed.
    let (levels_box, _) = box_at(&mut harness, LEVELS);
    right_click(&mut harness, levels_box.left_top() + egui::vec2(20.0, 6.0));
    assert!(
        harness
            .get_by_label(ROLL_BOX_LABEL)
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_graph_fill_picking_a_variation_is_made_with_its_look() {
    let (mut harness, id) = with_graph(moss_on_top());
    let moss_colour = BoxId(4);
    let blue = [0.1, 0.1, 0.9];
    {
        let doc = harness.state_mut().workbench.document_mut();
        let registry = doc.scene().graphs().registry().clone();
        let show = GraphEdit::ShowInPainter {
            node: moss_colour,
            key: "colour".into(),
            shown: true,
        };
        doc.execute(Command::EditGraph { id, edit: show }).unwrap();
        let paint = GraphEdit::SetSetting {
            node: moss_colour,
            key: "colour".into(),
            value: SettingValue::Colour(blue),
        };
        doc.execute(Command::EditGraph { id, edit: paint }).unwrap();
        let look = doc
            .scene()
            .graphs()
            .get(id)
            .unwrap()
            .variation_now(&registry, "Blue moss");
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::AddVariation(look),
        })
        .unwrap();
        // The graph itself back to green.
        let green = GraphEdit::SetSetting {
            node: moss_colour,
            key: "colour".into(),
            value: SettingValue::Colour([0.1, 0.6, 0.1]),
        };
        doc.execute(Command::EditGraph { id, edit: green }).unwrap();
    }
    add_graph_fill(&mut harness, id);
    wait(&mut harness);
    let bluish = |harness: &H| {
        let set = harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .unwrap()
            .clone();
        let image = set.layers.last().unwrap().image().unwrap().clone();
        (0..64)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let [r, _, b, a] = image.pixel(x, y);
                a > 0 && b > r + 60
            })
            .count()
    };
    assert_eq!(bluish(&harness), 0, "green moss as the graph is");
    let made = top_fill(&harness).made;
    // The layer's settings offer the variation.
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Variation: As the graph is")
            .is_some()
    );
    // Picked (as the drop-down does): made again, blue.
    let variation = graph_of(&harness, id).variations().next().unwrap().0;
    pick(&mut harness, Some(variation));
    wait(&mut harness);
    assert_ne!(top_fill(&harness).made, made);
    assert!(bluish(&harness) > 0, "blue moss from the variation");
    // A new variation another layer doesn't use changes nothing made.
    let made = top_fill(&harness).made;
    {
        let doc = harness.state_mut().workbench.document_mut();
        let registry = doc.scene().graphs().registry().clone();
        let other = graph_of_doc(doc, id).variation_now(&registry, "Other");
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::AddVariation(other),
        })
        .unwrap();
    }
    wait(&mut harness);
    assert_eq!(top_fill(&harness).made, made);
    // The variation deleted: the graph as it is, green again.
    {
        let doc = harness.state_mut().workbench.document_mut();
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::RemoveVariation(variation),
        })
        .unwrap();
    }
    wait(&mut harness);
    assert_eq!(bluish(&harness), 0);
}

fn graph_of(harness: &H, id: matgraph::GraphId) -> std::sync::Arc<matgraph::Graph> {
    graph_of_doc(harness.state().workbench.document(), id)
}

fn graph_of_doc(doc: &scene::Document, id: matgraph::GraphId) -> std::sync::Arc<matgraph::Graph> {
    doc.scene().graphs().get(id).unwrap().clone()
}

/// Sets the top layer's variation, as its drop-down does.
fn pick(harness: &mut H, variation: Option<VariationId>) {
    let doc = harness.state_mut().workbench.document_mut();
    let mut changed = (**doc.scene().paint().set("Default").unwrap()).clone();
    changed
        .layers
        .last_mut()
        .unwrap()
        .graph
        .as_mut()
        .unwrap()
        .variation = variation;
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(changed),
    })
    .unwrap();
}
