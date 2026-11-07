//! Your own boxes (decision 0263), used through the real interface:
//! Alt + G and its window, going inside by double-click and back out with
//! Esc, sliders chosen inside, Shift + G, linked and plain copies, the
//! shelf's library, box files, refusals, and undo for each.

use super::material_mode_tests::{
    H, LEVELS, NOISE, box_at, drag, graph, page, plug, point_at_canvas, quick_page, undo,
};
use super::*;
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use matgraph::{BoxId, GraphEdit, GraphId, SettingValue};

/// The page's graph with Noise wired into Levels, and Levels made into
/// your own box through the window: its placed box's id.
fn with_own_box(harness: &mut H) -> BoxId {
    let a = plug(harness, NOISE, "height", true);
    let b = plug(harness, LEVELS, "in", false);
    drag(harness, a, b, Modifiers::NONE);
    harness.state_mut().workbench.material().canvas.selected = [LEVELS].into();
    point_at_canvas(harness);
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::G);
    harness.run();
    assert!(harness.state_mut().workbench.material().making.is_some());
    let placed = BoxId(graph(harness).next_id());
    harness
        .get_by_label(crate::material::own_boxes::MAKE_IT)
        .click();
    harness.run();
    harness.run();
    placed
}

fn note(harness: &H) -> String {
    harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string()
}

fn own_count(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .own_count()
}

#[test]
fn alt_g_makes_your_own_box_in_one_undo_step() {
    let mut harness = page();
    let placed = with_own_box(&mut harness);
    let g = graph(&harness);
    let kind = g.get(placed).expect("the own box is placed").kind.clone();
    assert!(matgraph::own::is_own(&kind));
    assert!(g.get(LEVELS).is_none());
    assert_eq!(own_count(&harness), 1);
    assert!(
        note(&harness).contains("your own box"),
        "{}",
        note(&harness)
    );
    // Its picture was taken.
    let bench = &harness.state().workbench;
    let own = bench.document().scene().graphs().own(&kind).unwrap();
    assert!(own.picture.is_some());
    assert_eq!(own.inputs.len(), 1);
    undo(&mut harness);
    assert!(graph(&harness).get(LEVELS).is_some());
    assert_eq!(own_count(&harness), 0);
}

#[test]
fn alt_g_on_material_out_says_why() {
    let mut harness = page();
    let bench = &mut harness.state_mut().workbench;
    bench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: GraphEdit::AddBox {
                kind: "material_out".into(),
                at: glam::Vec2::new(900.0, 0.0),
            },
        })
        .unwrap();
    bench.material().canvas.selected = [BoxId(6)].into();
    harness.run();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::G);
    harness.run();
    assert!(
        note(&harness).contains("Material out"),
        "{}",
        note(&harness)
    );
    assert!(harness.state_mut().workbench.material().making.is_none());
    // Nothing selected says what to do.
    harness
        .state_mut()
        .workbench
        .material()
        .canvas
        .selected
        .clear();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::G);
    harness.run();
    assert!(
        note(&harness).contains("Select the boxes"),
        "{}",
        note(&harness)
    );
}

#[test]
fn double_click_goes_inside_and_esc_comes_back_out() {
    let mut harness = quick_page();
    let placed = with_own_box(&mut harness);
    let (rect, _) = box_at(&mut harness, placed);
    let at = rect.left_top() + egui::vec2(20.0, 6.0);
    // Long enough after Make it's click not to make a triple click.
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
    let bench = &mut harness.state_mut().workbench;
    assert_eq!(bench.material().inside.len(), 1, "inside the box");
    let document = bench.document.clone();
    let shown = bench.material().current(&document).unwrap().clone();
    assert!(
        shown.get(LEVELS).is_some(),
        "the canvas shows Levels inside"
    );
    assert!(
        harness
            .query_all_by_label_contains(crate::material::own_boxes::BACK_OUT)
            .next()
            .is_some(),
        "the way back out is shown"
    );
    // Levels inside gets the pictures from Noise outside.
    harness
        .state_mut()
        .workbench
        .material()
        .pictures
        .settle(30.0);
    harness.run();
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    let graph_inside = bench.material().current(&document).unwrap().clone();
    let problems = bench.material().problems(&graph_inside);
    assert!(problems.is_empty(), "{problems:?}");
    point_at_canvas(&mut harness);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state_mut().workbench.material().inside.is_empty());
}

#[test]
fn sliders_chosen_inside_show_on_every_placed_box() {
    let mut harness = page();
    let placed = with_own_box(&mut harness);
    let kind = graph(&harness).get(placed).unwrap().kind.clone();
    let bench = &mut harness.state_mut().workbench;
    bench.material().open_inside(kind.clone(), Some(placed));
    bench.material().canvas.selected = [LEVELS].into();
    harness.run();
    harness.run();
    let label = format!("{} Darkest", crate::material::own_boxes::SLIDER_SWITCH);
    harness.get_by_label(&label).click();
    harness.run();
    let bench = &harness.state().workbench;
    let own = bench.document().scene().graphs().own(&kind).unwrap();
    assert_eq!(own.sliders.len(), 1);
    // Outside, the placed box has it as a setting of its own.
    let registry = bench.document().scene().graphs().registry().clone();
    let info = registry.get(&kind).unwrap().info().clone();
    assert_eq!(info.settings[0].name.show(), "Darkest");
    undo(&mut harness);
    let bench = &harness.state().workbench;
    assert!(
        bench
            .document()
            .scene()
            .graphs()
            .own(&kind)
            .unwrap()
            .sliders
            .is_empty()
    );
}

#[test]
fn shift_g_opens_it_back_up() {
    let mut harness = page();
    let placed = with_own_box(&mut harness);
    harness.state_mut().workbench.material().canvas.selected = [placed].into();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::SHIFT, egui::Key::G);
    harness.run();
    let g = graph(&harness);
    assert!(g.get(placed).is_none());
    assert!(g.boxes().any(|(_, b)| b.kind == "levels"));
    assert_eq!(g.wires().len(), 1, "Noise still feeds Levels");
    // The box stays on the shelf.
    assert_eq!(own_count(&harness), 1);
    // Shift + G on a box of the app's own says why not.
    harness.state_mut().workbench.material().canvas.selected = [NOISE].into();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::SHIFT, egui::Key::G);
    harness.run();
    assert!(note(&harness).contains("Alt + G"), "{}", note(&harness));
}

#[test]
fn alt_v_pastes_linked_copies_and_ctrl_v_copies_of_their_own() {
    let mut harness = page();
    let placed = with_own_box(&mut harness);
    let kind = graph(&harness).get(placed).unwrap().kind.clone();
    harness.state_mut().workbench.material().canvas.selected = [placed].into();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::C);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::V);
    harness.run();
    let g = graph(&harness);
    assert_eq!(
        g.boxes().filter(|(_, b)| b.kind == kind).count(),
        2,
        "linked"
    );
    assert_eq!(own_count(&harness), 1);
    assert!(note(&harness).contains("linked"), "{}", note(&harness));
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::V);
    harness.run();
    assert_eq!(own_count(&harness), 2, "a new box of its own");
    let bench = &harness.state().workbench;
    let names: Vec<String> = bench
        .document()
        .scene()
        .graphs()
        .own_boxes()
        .map(|o| o.name.clone())
        .collect();
    assert!(names.iter().any(|n| n.ends_with("copy")), "{names:?}");
    undo(&mut harness);
    assert_eq!(own_count(&harness), 1);
}

#[test]
fn the_library_places_boxes_in_another_project_and_box_files_round_trip() {
    let mut harness = page();
    let placed = with_own_box(&mut harness);
    let kind = graph(&harness).get(placed).unwrap().kind.clone();
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    let mut status = None;
    bench
        .material()
        .save_to_library(&document, &kind, &mut status);
    assert_eq!(bench.material().library.len(), 1);
    // Kept between sessions as box files' text.
    let kept = bench.own_library();
    assert_eq!(kept.len(), 1);
    let mut other = Workbench::new(info_for_tests(), None).with_own_library(kept.clone());
    assert_eq!(other.material().library.len(), 1);
    // A box file written and read back joins the library.
    bench.material().box_file_asked = Some(crate::material::own_boxes::BoxFileAsk::Save(
        bench.material().library[0].clone(),
    ));
    let mut actions = Vec::new();
    bench.carry_out_box_file_asks(&mut actions);
    assert!(matches!(
        actions[0],
        WorkbenchAction::SaveBoxFile { path: None }
    ));
    let (name, text) = bench.box_file().unwrap();
    assert_eq!(name, "My box");
    other.material().library.clear();
    other.load_box_file(std::path::Path::new("rock.matbox"), text.as_bytes());
    assert_eq!(other.material().library.len(), 1);
    other.load_box_file(std::path::Path::new("junk.matbox"), b"junk");
    assert!(other.note().unwrap_or_default().contains("Couldn't import"));
    // Placing it in a project without it brings it in, in one undo step.
    let mut graph = matgraph::Graph::new("Other");
    graph
        .apply(
            matgraph::Registry::standard(),
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: glam::Vec2::ZERO,
            },
        )
        .unwrap();
    other
        .document_mut()
        .execute(scene::Command::AddGraph(graph))
        .unwrap();
    other.material().show_graph(Some(GraphId(1)));
    let file = other.material().library[0].clone();
    let mut doc = other.document.clone();
    other
        .material()
        .place_own(&mut doc, &file, None, &mut status);
    assert_eq!(doc.scene().graphs().own_count(), 1);
    let g = doc.scene().graphs().get(GraphId(1)).unwrap();
    assert!(g.boxes().any(|(_, b)| b.kind == kind));
    doc.undo();
    assert_eq!(doc.scene().graphs().own_count(), 0);
}

fn info_for_tests() -> crate::AppInfo {
    super::tests::info()
}

#[test]
fn the_shelf_lists_your_own_boxes_in_the_material_mode() {
    let mut harness = page();
    let placed = with_own_box(&mut harness);
    let _ = placed;
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .dock
            .find_tab(&Panel::Brushes)
            .is_some()
    );
    let card = format!("Own box {}", "My box");
    assert!(
        harness.query_by_label(&card).is_some(),
        "the project's card"
    );
    let before = graph(&harness).box_count();
    harness.get_by_label(&card).click();
    harness.run();
    assert_eq!(graph(&harness).box_count(), before + 1, "a click places it");
}

#[test]
fn the_pictures_of_a_graph_with_your_own_box_match_the_boxes_it_holds() {
    let mut harness = page();
    let before = {
        let a = plug(&mut harness, NOISE, "height", true);
        let b = plug(&mut harness, LEVELS, "in", false);
        drag(&mut harness, a, b, Modifiers::NONE);
        harness
            .state_mut()
            .workbench
            .material()
            .pictures
            .settle(30.0);
        harness.run();
        harness
            .state_mut()
            .workbench
            .material()
            .pictures
            .value(LEVELS)
            .cloned()
            .unwrap()
    };
    harness.state_mut().workbench.material().canvas.selected = [LEVELS].into();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::G);
    harness.run();
    let placed = BoxId(graph(&harness).next_id());
    harness
        .get_by_label(crate::material::own_boxes::MAKE_IT)
        .click();
    harness.run();
    harness
        .state_mut()
        .workbench
        .material()
        .pictures
        .settle(30.0);
    harness.run();
    let after = harness
        .state_mut()
        .workbench
        .material()
        .pictures
        .value(placed)
        .cloned()
        .expect("the own box has a picture");
    assert_eq!(after, before);
    let _ = SettingValue::Number(0.0);
}
