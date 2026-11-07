//! The starter graphs (decision 0264), used through the real interface:
//! opening one from the canvas bar, from the empty canvas and from the
//! shelf, a second copy of the same one, and undo.

use super::material_mode_tests::{H, page, undo};
use super::tests::fresh;
use crate::material::starters::{START_FROM_LABEL, STARTERS_LABEL};
use egui_kittest::kittest::Queryable;
use fileio::starter_graphs::STARTERS;

fn graph_names(harness: &H) -> Vec<String> {
    let bench = &harness.state().workbench;
    bench
        .document()
        .scene()
        .graphs()
        .iter()
        .map(|(_, g)| g.name().to_string())
        .collect()
}

fn shown_name(harness: &H) -> Option<String> {
    let bench = &harness.state().workbench;
    let id = bench.material.graph?;
    Some(
        bench
            .document()
            .scene()
            .graphs()
            .get(id)?
            .name()
            .to_string(),
    )
}

/// Clicks the card of starter `name` in the Starter graphs window.
fn choose(harness: &mut H, name: &str) {
    harness
        .get_by_label(&format!("Open a copy of {name}"))
        .click();
    harness.run();
    harness.run();
}

#[test]
fn a_starter_opens_as_a_copy_from_the_canvas_bar_and_undo_takes_it_away() {
    let mut harness = page();
    harness
        .get_all_by_label(STARTERS_LABEL)
        .next()
        .unwrap()
        .click();
    harness.run();
    assert!(harness.state().workbench.material.starters_open);
    // Every starter has a card in the window.
    for starter in &STARTERS {
        assert!(
            harness
                .query_all_by_label(&format!("Open a copy of {}", starter.name))
                .next()
                .is_some(),
            "{}",
            starter.name
        );
    }
    let before = harness
        .state()
        .workbench
        .document()
        .undo_label()
        .map(str::to_string);
    choose(&mut harness, "Stone tiles");
    assert_eq!(graph_names(&harness), ["Stone", "Stone tiles"]);
    assert_eq!(shown_name(&harness).as_deref(), Some("Stone tiles"));
    assert!(
        !harness.state().workbench.material.starters_open,
        "the window closes"
    );
    let bench = &harness.state().workbench;
    assert_eq!(
        bench.document().undo_label(),
        Some("Open the starter graph Stone tiles")
    );
    let note = bench.note.clone().unwrap_or_default();
    assert!(note.contains("yours to change"), "{note}");
    // It is the starter, with its frames and notes, and works out.
    let id = bench.material.graph.unwrap();
    let graph = bench.document().scene().graphs().get(id).unwrap().clone();
    assert!(graph.frames().all(|(_, f)| !f.note.is_empty()));
    assert!(graph.frames().count() >= 3);
    // Opening it again makes a second copy with its own name.
    harness
        .get_all_by_label(STARTERS_LABEL)
        .next()
        .unwrap()
        .click();
    harness.run();
    choose(&mut harness, "Stone tiles");
    assert_eq!(
        graph_names(&harness),
        ["Stone", "Stone tiles", "Stone tiles 2"]
    );
    // Undo takes each copy away in one step.
    undo(&mut harness);
    undo(&mut harness);
    harness.run();
    assert_eq!(graph_names(&harness), ["Stone"]);
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .undo_label()
            .map(str::to_string),
        before
    );
}

#[test]
fn the_empty_canvas_offers_the_starters() {
    let mut harness = fresh();
    harness.get_by_label("Material mode").click();
    harness.run();
    harness.run();
    harness.get_by_label(START_FROM_LABEL).click();
    harness.run();
    choose(&mut harness, "Bricks");
    assert_eq!(graph_names(&harness), ["Bricks"]);
    assert_eq!(shown_name(&harness).as_deref(), Some("Bricks"));
    assert!(
        harness
            .query_by_label(crate::material::CANVAS_LABEL)
            .is_some()
    );
}

#[test]
fn the_shelf_opens_a_starter_with_one_click() {
    let mut harness = page();
    let card = harness
        .get_all_by_label("Starter graph Cobbles")
        .next()
        .expect("the shelf lists the starters");
    card.click();
    harness.run();
    harness.run();
    assert_eq!(graph_names(&harness), ["Stone", "Cobbles"]);
}

#[test]
fn every_starter_is_tidy_and_its_notes_fit_above_its_boxes() {
    let ctx = egui::Context::default();
    crate::theme::apply(&ctx);
    // Fonts are loaded on the first frame.
    let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
    output.textures_delta.clear();
    for starter in &STARTERS {
        let file = starter.load().unwrap();
        let registry = matgraph::own::registry_with(&file.own);
        let graph = file.graph;
        assert!(
            crate::material::tidy::tidy(&graph, &registry).is_empty(),
            "{}: the tidy layout would move things",
            starter.key
        );
        for (id, frame) in graph.frames() {
            let top_box = graph
                .boxes_in(id)
                .iter()
                .map(|b| graph.get(*b).unwrap().at.y)
                .fold(frame.max.y, f32::min);
            let pad = crate::material::look::NOTE_PAD;
            let width = frame.max.x - frame.min.x - 2.0 * pad;
            let galley = ctx.fonts_mut(|f| {
                f.layout(
                    frame.note.clone(),
                    egui::FontId::proportional(crate::material::look::NOTE_SIZE),
                    egui::Color32::WHITE,
                    width,
                )
            });
            let room = top_box - (frame.min.y + crate::material::look::FRAME_TITLE) - pad;
            assert!(
                galley.size().y <= room,
                "{} / {}: the note needs {} but has {room}",
                starter.key,
                frame.title,
                galley.size().y
            );
        }
    }
}
