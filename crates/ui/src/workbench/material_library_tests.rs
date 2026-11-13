//! The library browser (decision 0292), used through the real interface:
//! every graph and own box as a card, searching names and tags, clicking
//! tags, choosing cards, tags as undo steps, keeping in your library,
//! opening a copy, packs written and brought back, and card pictures
//! made once.

use super::material_mode_tests::{H, page, undo};
use super::material_own_tests::with_own_box;
use super::*;
use crate::material::library::{
    ADD_TAG_LABEL, EXPORT_PACK, ItemId, LIBRARY_LABEL, Place, SEARCH_LABEL,
};
use egui::Modifiers;
use egui_kittest::kittest::{NodeT, Queryable};
use matgraph::GraphId;

/// The page with the Library window open.
fn library() -> H {
    let mut harness = page();
    harness.get_by_label(LIBRARY_LABEL).click();
    harness.run();
    harness.run();
    assert!(harness.state().workbench.material.browser.open);
    harness
}

fn card_shows(harness: &H, label: &str) -> bool {
    harness.query_by_label(label).is_some()
}

fn stone(harness: &H) -> GraphId {
    harness.state().workbench.material.graph.unwrap()
}

fn graph_names(harness: &H) -> Vec<String> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .iter()
        .map(|(_, g)| g.name().to_string())
        .collect()
}

#[test]
fn the_library_lists_everything_and_finds_by_name_and_tags() {
    let mut harness = library();
    assert!(card_shows(&harness, "Stone, graph in this project"));
    assert!(card_shows(&harness, "Bricks, starter graph"));
    assert!(card_shows(&harness, "Planks, starter graph"));
    // Every word typed must be in the name or tags.
    harness.get_by_label(SEARCH_LABEL).click();
    harness.run();
    harness.get_by_label(SEARCH_LABEL).type_text("stone floor");
    harness.run();
    assert!(card_shows(&harness, "Stone tiles, starter graph"));
    assert!(card_shows(&harness, "Cobbles, starter graph"));
    assert!(!card_shows(&harness, "Bricks, starter graph"), "no floor");
    assert!(
        !card_shows(&harness, "Stone, graph in this project"),
        "no tags"
    );
    harness
        .state_mut()
        .workbench
        .material()
        .browser
        .query
        .clear();
    harness.run();
    // Clicking a tag shows only cards with it; clicking again stops.
    harness.get_by_label("wood").click();
    harness.run();
    assert!(card_shows(&harness, "Planks, starter graph"));
    assert!(!card_shows(&harness, "Bricks, starter graph"));
    harness.get_by_label("wood").click();
    harness.run();
    assert!(card_shows(&harness, "Bricks, starter graph"));
    // Only this project's.
    harness.state_mut().workbench.material().browser.from = Some(Place::Project);
    harness.run();
    assert!(card_shows(&harness, "Stone, graph in this project"));
    assert!(!card_shows(&harness, "Bricks, starter graph"));
    // Nothing found says what to try.
    harness.state_mut().workbench.material().browser.query = "zzz".into();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Nothing here matches")
            .is_some()
    );
}

#[test]
fn a_click_chooses_ctrl_adds_shift_takes_away() {
    let mut harness = library();
    harness.get_by_label("Bricks, starter graph").click();
    harness.run();
    let chosen = |h: &H| h.state().workbench.material.browser.chosen.clone();
    assert_eq!(chosen(&harness), [ItemId::Starter("bricks")].into());
    harness
        .get_by_label("Planks, starter graph")
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert_eq!(chosen(&harness).len(), 2);
    assert_eq!(
        harness.state().workbench.material.browser.focus,
        Some(ItemId::Starter("planks"))
    );
    harness
        .get_by_label("Bricks, starter graph")
        .click_modifiers(Modifiers::SHIFT);
    harness.run();
    assert_eq!(chosen(&harness), [ItemId::Starter("planks")].into());
    // A starter's tags are fixed, and it can't go in a pack.
    assert!(harness.query_by_label(ADD_TAG_LABEL).is_none());
    assert!(
        harness
            .get_by_label(EXPORT_PACK)
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn tags_on_the_project_s_graph_are_one_undo_step_each() {
    let mut harness = library();
    harness.get_by_label("Stone, graph in this project").click();
    harness.run();
    harness.get_by_label(ADD_TAG_LABEL).click();
    harness.run();
    harness.get_by_label(ADD_TAG_LABEL).type_text("Wall, rough");
    harness.run();
    harness.get_by_label("Add").click();
    harness.run();
    let tags = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .graphs()
            .get(stone(h))
            .unwrap()
            .tags()
            .to_vec()
    };
    assert_eq!(tags(&harness), ["rough", "wall"]);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change the graph's tags")
    );
    // The card shows them, and the tag is clickable now.
    assert!(harness.query_by_label("wall").is_some());
    harness.get_by_label("Take off the tag rough").click();
    harness.run();
    assert_eq!(tags(&harness), ["wall"]);
    undo(&mut harness);
    assert_eq!(tags(&harness), ["rough", "wall"]);
    undo(&mut harness);
    assert!(tags(&harness).is_empty());
}

#[test]
fn a_graph_kept_in_your_library_opens_as_a_copy_in_any_project() {
    let mut harness = library();
    harness.get_by_label("Stone, graph in this project").click();
    harness.run();
    harness.get_by_label("Keep in your library").click();
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.material.library_graphs.len(), 1);
    assert!(card_shows(&harness, "Stone, graph in your library"));
    // Kept again: replaced, not doubled.
    harness.get_by_label("Keep in your library").click();
    harness.run();
    assert_eq!(harness.state().workbench.material.library_graphs.len(), 1);
    // Kept between sessions as graph files' text.
    let kept = harness.state().workbench.graph_library();
    assert_eq!(kept.len(), 1);
    let other = Workbench::new(super::tests::info(), None).with_graph_library(vec![
        kept[0].clone(),
        kept[0].clone(),
        "junk".into(),
    ]);
    assert_eq!(other.material.library_graphs.len(), 1, "one per name");
    // Opening it from the library makes a copy here, one undo step.
    harness.get_by_label("Stone, graph in your library").click();
    harness.run();
    harness.get_by_label("Open a copy in this project").click();
    harness.run();
    harness.run();
    assert_eq!(graph_names(&harness), ["Stone", "Stone 2"]);
    let bench = &harness.state().workbench;
    assert_eq!(
        bench.document().undo_label(),
        Some("Open a copy of Stone 2")
    );
    assert!(
        !bench.material.browser.open,
        "the window closes on the copy"
    );
    undo(&mut harness);
    assert_eq!(graph_names(&harness), ["Stone"]);
    // Removing it from the library leaves the project alone.
    let bench = &mut harness.state_mut().workbench;
    let mut note = None;
    let mut document = bench.document.clone();
    bench.material().set_item_tags(
        &mut document,
        &ItemId::LibraryGraph("Stone".into()),
        vec!["Kept".into()],
        &mut note,
    );
    assert_eq!(
        bench.material().library_graphs[0].graph.tags(),
        ["kept"],
        "library tags change at once"
    );
}

#[test]
fn double_clicking_a_starter_opens_a_copy() {
    let mut harness = super::material_mode_tests::quick_page();
    harness.get_by_label(LIBRARY_LABEL).click();
    harness.run();
    let at = harness
        .get_by_label("Cobbles, starter graph")
        .rect()
        .center();
    for _ in 0..60 {
        harness.step();
    }
    harness.event(egui::Event::PointerMoved(at));
    harness.step();
    // Two clicks in quick succession.
    for pressed in [true, false, true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    harness.run();
    assert_eq!(graph_names(&harness), ["Stone", "Cobbles"]);
    assert!(!harness.state().workbench.material.browser.open);
}

#[test]
fn a_pack_of_the_chosen_cards_is_written_and_brought_into_a_library() {
    let mut harness = library();
    let own = with_own_box(&mut harness);
    let kind = super::material_mode_tests::graph(&harness)
        .get(own)
        .unwrap()
        .kind
        .clone();
    harness.state_mut().workbench.material().browser.open = true;
    harness.run();
    harness.get_by_label("Stone, graph in this project").click();
    harness.run();
    let own_card = format!("{}, own box in this project", "My box");
    harness
        .get_by_label(&own_card)
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    // A starter chosen too is left out of the pack.
    harness
        .get_by_label("Bricks, starter graph")
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    harness.get_by_label(EXPORT_PACK).click();
    harness.run();
    let bench = &mut harness.state_mut().workbench;
    let mut actions = Vec::new();
    bench.carry_out_box_file_asks(&mut actions);
    assert!(
        actions.is_empty(),
        "the frame already turned the ask into an action"
    );
    let (name, text) = bench.pack_file().expect("a pack was asked for");
    assert_eq!(name, "Material pack");
    let pack = fileio::pack_file::read_pack(text.as_bytes()).unwrap();
    assert_eq!(pack.graphs.len(), 1);
    assert_eq!(pack.boxes.len(), 1);
    assert_eq!(pack.boxes[0].main.id, kind);
    assert_eq!(pack.graphs[0].own.len(), 1, "the graph brings its own box");
    // Brought into another library: everything at once, and the window
    // shows your library.
    let mut other = Workbench::new(super::tests::info(), None);
    other.bring_into_library(std::path::Path::new("set.matpack"), text.as_bytes());
    assert_eq!(other.material.library_graphs.len(), 1);
    assert_eq!(other.material.library.len(), 1);
    assert!(other.material.browser.open);
    assert_eq!(other.material.browser.from, Some(Place::Library));
    assert!(
        other
            .note()
            .unwrap_or_default()
            .contains("Brought 1 graphs")
    );
    // Again: replaced, not doubled.
    other.bring_into_library(std::path::Path::new("set.matpack"), text.as_bytes());
    assert_eq!(other.material.library_graphs.len(), 1);
    assert_eq!(other.material.library.len(), 1);
    // A graph file and a box file come in by their extensions.
    let graph = fileio::graph_file::write_graph_file(&pack.graphs[0]);
    let mut third = Workbench::new(super::tests::info(), None);
    third.bring_into_library(std::path::Path::new("stone.MATGRAPH"), graph.as_bytes());
    assert_eq!(third.material.library_graphs.len(), 1);
    let own = fileio::own_box_file::write_box_file(&pack.boxes[0]);
    third.bring_into_library(std::path::Path::new("box.matbox"), own.as_bytes());
    assert_eq!(third.material.library.len(), 1);
    // A damaged pack brings in nothing and says why.
    let mut fourth = Workbench::new(super::tests::info(), None);
    fourth.bring_into_library(std::path::Path::new("bad.matpack"), &text.as_bytes()[..40]);
    assert!(fourth.material.library_graphs.is_empty());
    assert!(
        fourth
            .note()
            .unwrap_or_default()
            .contains("Couldn't import")
    );
}

#[test]
fn an_own_box_is_placed_from_its_card_and_its_tags_are_an_undo_step() {
    let mut harness = library();
    let own = with_own_box(&mut harness);
    let kind = super::material_mode_tests::graph(&harness)
        .get(own)
        .unwrap()
        .kind
        .clone();
    harness.state_mut().workbench.material().browser.open = true;
    harness.run();
    let card = "My box, own box in this project";
    harness.get_by_label(card).click();
    harness.run();
    let placed = |h: &H| {
        super::material_mode_tests::graph(h)
            .boxes()
            .filter(|(_, b)| b.kind == kind)
            .count()
    };
    assert_eq!(placed(&harness), 1);
    harness.get_by_label("Place it").click();
    harness.run();
    assert_eq!(placed(&harness), 2);
    let bench = &mut harness.state_mut().workbench;
    let mut document = bench.document.clone();
    let mut note = None;
    bench.material().set_item_tags(
        &mut document,
        &ItemId::ProjectBox(kind.clone()),
        vec!["Lumpy".into()],
        &mut note,
    );
    assert_eq!(note, None);
    assert_eq!(
        document.scene().graphs().own(&kind).unwrap().tags,
        ["lumpy"]
    );
    assert_eq!(document.undo_label(), Some("Change your own box's tags"));
}

#[test]
fn card_pictures_are_made_once_and_never_while_scrolling_or_searching() {
    let mut harness = library();
    for _ in 0..4 {
        harness.run();
    }
    let runs = |h: &H| h.state().workbench.material.browser.pictures.runs;
    // One graph in the project; starters and own boxes bring their own.
    assert_eq!(runs(&harness), 1);
    assert!(
        !harness
            .state()
            .workbench
            .material
            .browser
            .pictures
            .waiting()
    );
    // Searching hides the card and brings it back: nothing made again.
    harness.state_mut().workbench.material().browser.query = "bricks".into();
    harness.run();
    harness
        .state_mut()
        .workbench
        .material()
        .browser
        .query
        .clear();
    for _ in 0..3 {
        harness.run();
    }
    assert_eq!(runs(&harness), 1);
    // Scrolling the cards makes nothing.
    harness.hover_at(
        harness
            .get_by_label("Bricks, starter graph")
            .rect()
            .center(),
    );
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -200.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    for _ in 0..3 {
        harness.run();
    }
    assert_eq!(runs(&harness), 1);
    // A change to the graph makes its picture again, once.
    let id = stone(&harness);
    let bench = &mut harness.state_mut().workbench;
    let mut document = bench.document.clone();
    let mut note = None;
    bench.material().set_item_tags(
        &mut document,
        &ItemId::ProjectGraph(id),
        vec!["new".into()],
        &mut note,
    );
    bench.document = document;
    for _ in 0..3 {
        harness.run();
    }
    assert_eq!(runs(&harness), 2);
}
