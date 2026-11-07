//! The Material mode and the node canvas (decision 0258), used through the
//! real interface: connecting by dragging between dots, a wire refused
//! with its note and a fix, the box search, comment frames, removing,
//! copying, the preview and Attributes, and undo for each.

use super::tests::{fresh, info};
use super::*;
use crate::material::look::COLUMN;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, Registry};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The boxes of the graph `page` makes.
const NOISE: BoxId = BoxId(1);
const LEVELS: BoxId = BoxId(2);
const FILL: BoxId = BoxId(3);
const BLEND: BoxId = BoxId(4);
const TILES: BoxId = BoxId(5);

/// The Material mode showing a graph of Noise and Levels on top, Colour
/// fill and Blend under them, and Tiles under those, unconnected.
fn page() -> H {
    let mut workbench = Workbench::new(info(), None);
    let mut graph = Graph::new("Stone");
    for (kind, x, y) in [
        ("noise", 0.0, 0.0),
        ("levels", 2.0 * COLUMN, 0.0),
        ("colour_fill", 0.0, 260.0),
        ("blend", 2.0 * COLUMN, 260.0),
        ("tiles", 0.0, 520.0),
    ] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, y),
        };
        graph.apply(Registry::standard(), &edit).unwrap();
    }
    workbench
        .document_mut()
        .execute(scene::Command::AddGraph(graph))
        .unwrap();
    workbench.mode = Mode::Material;
    let mut harness = super::tests::harness_for(workbench);
    harness.run();
    harness.run();
    harness
}

fn graph(harness: &H) -> std::sync::Arc<Graph> {
    let bench = &harness.state().workbench;
    bench
        .document()
        .scene()
        .graphs()
        .get(GraphId(1))
        .unwrap()
        .clone()
}

fn plug(harness: &mut H, id: BoxId, key: &str, output: bool) -> Pos2 {
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    bench
        .material()
        .plug_on_screen(&document, &PlugRef::new(id, key), output)
        .unwrap_or_else(|| panic!("no plug {key} on {id:?}"))
}

fn box_at(harness: &mut H, id: BoxId) -> (egui::Rect, egui::Rect) {
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    bench.material().box_on_screen(&document, id).unwrap()
}

fn canvas(harness: &mut H) -> egui::Rect {
    let bench = &mut harness.state_mut().workbench;
    let m = bench.material();
    m.canvas.rect.or(m.canvas.last_rect).unwrap()
}

/// A spot on the canvas with nothing on it.
fn empty_spot(harness: &mut H) -> Pos2 {
    canvas(harness).right_bottom() - egui::vec2(15.0, 15.0)
}

/// A drag with the left button from `from` to `to`, in small steps,
/// holding `modifiers`.
fn drag(harness: &mut H, from: Pos2, to: Pos2, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    harness.step();
    harness.run();
}

fn click(harness: &mut H, at: Pos2, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        harness.step();
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

/// Points at the canvas, so its keys go to it.
fn point_at_canvas(harness: &mut H) {
    let at = empty_spot(harness);
    harness.event(Event::PointerMoved(at));
    harness.run();
}

fn undo(harness: &mut H) {
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();
}

#[test]
fn the_material_mode_has_its_own_layout_and_starts_a_graph() {
    let mut harness = fresh();
    harness.get_by_label("Material mode").click();
    harness.run();
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.mode, Mode::Material);
    assert_eq!(bench.layout_choice(), LayoutChoice::Material);
    assert!(bench.dock.find_tab(&Panel::MaterialPreview).is_some());
    harness
        .get_all_by_label(crate::material::NEW_GRAPH_LABEL)
        .next()
        .unwrap()
        .click();
    harness.run();
    harness.run();
    let bench = &harness.state().workbench;
    let graphs = bench.document().scene().graphs();
    assert_eq!(graphs.len(), 1);
    let (_, made) = graphs.iter().next().unwrap();
    assert!(
        made.first_of("material_out").is_some(),
        "starts with Material out"
    );
    assert!(
        harness
            .query_by_label(crate::material::CANVAS_LABEL)
            .is_some()
    );
    // Leaving the mode puts the layout back.
    harness.get_by_label("Model mode").click();
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workbench.layout_choice(),
        LayoutChoice::Standard
    );
    // The layout tab goes there too.
    harness.get_by_label("Material").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Material);
}

#[test]
fn dragging_between_dots_connects_and_undo_takes_it_back() {
    let mut harness = page();
    let from = plug(&mut harness, NOISE, "height", true);
    let to = plug(&mut harness, LEVELS, "in", false);
    drag(&mut harness, from, to, Modifiers::NONE);
    let g = graph(&harness);
    assert_eq!(g.wires().len(), 1);
    assert_eq!(g.wires()[0].from, PlugRef::new(NOISE, "height"));
    // Wires run left to right.
    assert!(g.get(LEVELS).unwrap().at.x > g.get(NOISE).unwrap().at.x);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Connect boxes")
    );
    undo(&mut harness);
    assert!(graph(&harness).wires().is_empty());
}

#[test]
fn dragging_a_wires_end_away_disconnects_it() {
    let mut harness = page();
    let from = plug(&mut harness, NOISE, "height", true);
    let to = plug(&mut harness, LEVELS, "in", false);
    drag(&mut harness, from, to, Modifiers::NONE);
    assert_eq!(graph(&harness).wires().len(), 1);
    let to = plug(&mut harness, LEVELS, "in", false);
    let away = empty_spot(&mut harness);
    drag(&mut harness, to, away, Modifiers::NONE);
    assert!(graph(&harness).wires().is_empty());
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .search
            .is_none()
    );
}

#[test]
fn a_wire_that_wont_fit_turns_red_says_why_and_offers_a_fix() {
    let mut harness = page();
    let from = plug(&mut harness, FILL, "colour", true);
    let to = plug(&mut harness, BLEND, "where", false);
    // Over the plug, before letting go: the note, and the status bar.
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(from + (to - from) * (i as f32 / 8.0)));
        harness.step();
    }
    let status = harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string();
    assert!(status.contains("no inside or outside"), "{status}");
    assert!(
        harness
            .query_by_label_contains("no inside or outside")
            .is_some()
    );
    // Letting go cancels the wire; the note stays with its fixes.
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.run();
    assert!(graph(&harness).wires().is_empty());
    let fix = harness.get_by_label("Connect Tiles instead");
    fix.click();
    harness.run();
    let g = graph(&harness);
    assert_eq!(g.wires().len(), 1);
    assert_eq!(g.wires()[0].from, PlugRef::new(TILES, "shape"));
    assert_eq!(g.wires()[0].to, PlugRef::new(BLEND, "where"));
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .note
            .is_none()
    );
    undo(&mut harness);
    assert!(graph(&harness).wires().is_empty());
}

#[test]
fn a_wire_dropped_on_empty_canvas_opens_the_search_for_what_fits() {
    let mut harness = page();
    let from = plug(&mut harness, NOISE, "height", true);
    let spot = empty_spot(&mut harness);
    drag(&mut harness, from, spot, Modifiers::NONE);
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .search
            .is_some()
    );
    assert!(
        harness
            .query_by_label(crate::material::COMES_NEXT)
            .is_some()
    );
    harness.get_all_by_label("Levels").last().unwrap().click();
    harness.run();
    let g = graph(&harness);
    assert_eq!(g.box_count(), 6);
    let new = BoxId(6);
    assert_eq!(g.get(new).unwrap().kind, "levels");
    assert_eq!(
        g.wire_into(&PlugRef::new(new, "in")).unwrap().from.node,
        NOISE
    );
    undo(&mut harness);
    assert_eq!(graph(&harness).box_count(), 5, "one step");
}

#[test]
fn space_and_right_click_open_the_box_search_not_the_command_search() {
    let mut harness = page();
    point_at_canvas(&mut harness);
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .search
            .is_some()
    );
    assert!(!harness.state().workbench.search.open);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .search
            .is_none()
    );
    assert_eq!(harness.state().workbench.mode, Mode::Material);
    let spot = empty_spot(&mut harness);
    harness.event(Event::PointerMoved(spot));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: spot,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .search
            .is_some()
    );
    // Typing narrows it; Enter adds the box.
    for c in ["n", "o", "i", "s"] {
        harness.event(Event::Text(c.to_string()));
        harness.run();
    }
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(graph(&harness).get(BoxId(6)).unwrap().kind, "noise");
}

#[test]
fn c_frames_the_selection_and_its_title_moves_what_is_inside() {
    let mut harness = page();
    let (noise, _) = box_at(&mut harness, NOISE);
    let (levels, _) = box_at(&mut harness, LEVELS);
    click(
        &mut harness,
        noise.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    click(
        &mut harness,
        levels.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::COMMAND,
    );
    assert_eq!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .selected
            .len(),
        2
    );
    point_at_canvas(&mut harness);
    harness.key_press(egui::Key::C);
    harness.run();
    let g = graph(&harness);
    let (frame, made) = g.frames().next().expect("a frame");
    assert_eq!(made.title, "Comment");
    let mut inside = g.boxes_in(frame);
    inside.sort();
    assert_eq!(inside, vec![NOISE, LEVELS]);
    // Its title is ready to type.
    for c in ["R", "o", "c", "k"] {
        harness.event(Event::Text(c.to_string()));
        harness.run();
    }
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    let renamed = graph(&harness).frame(frame).unwrap().title.clone();
    assert!(renamed.ends_with("Rock"), "{renamed}");
    // Drag the title: the frame and its boxes move together.
    let before = graph(&harness);
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    let (_, title) = bench.material().frame_on_screen(&document, frame).unwrap();
    let zoom = bench.material().canvas.view.zoom;
    let grab = title.left_center() + egui::vec2(30.0, 0.0);
    // Two columns to the right.
    let by = egui::vec2(2.0 * COLUMN * zoom, 0.0);
    drag(&mut harness, grab, grab + by, Modifiers::NONE);
    let after = graph(&harness);
    let moved = |id: BoxId| after.get(id).unwrap().at - before.get(id).unwrap().at;
    assert!(
        (moved(NOISE).x - 2.0 * COLUMN).abs() < 1.0,
        "{:?}",
        moved(NOISE)
    );
    assert!((moved(NOISE) - moved(LEVELS)).length() < 1e-3);
    assert_eq!(moved(FILL), Vec2::ZERO, "boxes outside stay");
    assert_eq!(moved(BLEND), Vec2::ZERO);
    let frame_moved = after.frame(frame).unwrap().min - before.frame(frame).unwrap().min;
    assert!(frame_moved.x > COLUMN);
    // One step takes it all back.
    undo(&mut harness);
    let back = graph(&harness);
    assert_eq!(back.get(NOISE).unwrap().at, before.get(NOISE).unwrap().at);
    assert_eq!(
        back.frame(frame).unwrap().min,
        before.frame(frame).unwrap().min
    );
}

#[test]
fn backspace_keeps_the_wire_through_and_delete_removes() {
    let mut harness = page();
    for (from, to) in [
        ((NOISE, "height"), (LEVELS, "in")),
        ((LEVELS, "out"), (BLEND, "bottom")),
    ] {
        let a = plug(&mut harness, from.0, from.1, true);
        let b = plug(&mut harness, to.0, to.1, false);
        drag(&mut harness, a, b, Modifiers::NONE);
    }
    assert_eq!(graph(&harness).wires().len(), 2);
    let (levels, _) = box_at(&mut harness, LEVELS);
    click(
        &mut harness,
        levels.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    point_at_canvas(&mut harness);
    harness.key_press(egui::Key::Backspace);
    harness.run();
    let g = graph(&harness);
    assert!(g.get(LEVELS).is_none());
    let through = g.wire_into(&PlugRef::new(BLEND, "bottom")).unwrap();
    assert_eq!(through.from.node, NOISE);
    // No object was deleted instead.
    let (fill, _) = box_at(&mut harness, FILL);
    click(
        &mut harness,
        fill.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    point_at_canvas(&mut harness);
    harness.key_press(egui::Key::Delete);
    harness.run();
    assert!(graph(&harness).get(FILL).is_none());
    undo(&mut harness);
    assert!(graph(&harness).get(FILL).is_some());
}

#[test]
fn copying_and_pasting_boxes_keeps_their_wires_and_settings() {
    let mut harness = page();
    let a = plug(&mut harness, NOISE, "height", true);
    let b = plug(&mut harness, LEVELS, "in", false);
    drag(&mut harness, a, b, Modifiers::NONE);
    let set = GraphEdit::SetSetting {
        node: LEVELS,
        key: "high".into(),
        value: matgraph::SettingValue::Number(0.5),
    };
    let bench = &mut harness.state_mut().workbench;
    bench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: set,
        })
        .unwrap();
    bench.material().canvas.selected = [NOISE, LEVELS].into();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::C);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::V);
    harness.run();
    let g = graph(&harness);
    assert_eq!(g.box_count(), 7);
    assert_eq!(g.wires().len(), 2);
    let pasted = g
        .boxes()
        .find(|(id, b)| *id > TILES && b.kind == "levels")
        .unwrap();
    assert_eq!(
        pasted.1.settings.get("high"),
        Some(matgraph::SettingValue::Number(0.5))
    );
    // Alt + V pastes plain copies and says so.
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::V);
    harness.run();
    assert_eq!(graph(&harness).box_count(), 9);
    let note = harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string();
    assert!(note.contains("linked copies"), "{note}");
}

#[test]
fn a_box_s_settings_what_it_does_and_what_comes_next_are_in_attributes() {
    let mut harness = page();
    let (levels, _) = box_at(&mut harness, LEVELS);
    click(
        &mut harness,
        levels.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    assert!(
        harness
            .query_by_label(crate::material::WHAT_IT_DOES)
            .is_some()
    );
    assert!(harness.query_by_label("Lightest").is_some(), "its settings");
    // Attributes is longer than the window: scroll down to the button.
    harness.get_by_label("Material from height").scroll_to_me();
    harness.run();
    harness.get_by_label("Material from height").click();
    harness.run();
    let g = graph(&harness);
    let made = g
        .boxes()
        .find(|(_, b)| b.kind == "material_from_height")
        .map(|(id, _)| id)
        .unwrap();
    assert_eq!(
        g.wire_into(&PlugRef::new(made, "height"))
            .unwrap()
            .from
            .node,
        LEVELS
    );
    assert!(g.get(made).unwrap().at.x > g.get(LEVELS).unwrap().at.x);
}

#[test]
fn clicking_a_picture_looks_through_the_box() {
    let mut harness = page();
    let (_, picture) = box_at(&mut harness, NOISE);
    click(&mut harness, picture.center(), Modifiers::NONE);
    assert_eq!(
        harness.state_mut().workbench.material().canvas.looking_at,
        Some(NOISE)
    );
    assert!(
        harness
            .query_by_label_contains("Looking through Noise")
            .is_some()
    );
    harness
        .get_all_by_label("Back to the material")
        .next()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(
        harness.state_mut().workbench.material().canvas.looking_at,
        None
    );
}

#[test]
fn every_box_gets_a_live_picture_and_problems_are_marked() {
    let mut harness = page();
    let bench = &mut harness.state_mut().workbench;
    let pictures = &bench.material().pictures;
    assert!(!pictures.waiting());
    assert!(pictures.value(NOISE).is_some(), "Noise made a picture");
    // Levels has nothing plugged in: it can't run, and says why.
    let document = bench.document.clone();
    let graph = bench.material().current(&document).unwrap().clone();
    let problems = bench.material().problems(&graph);
    assert!(problems[&LEVELS].reason.contains("nothing plugged"));
}

#[test]
fn alt_g_says_your_own_boxes_come_later() {
    let mut harness = page();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, egui::Key::G);
    harness.run();
    let note = harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string();
    assert!(note.contains("your own boxes"), "{note}");
    assert_eq!(graph(&harness).box_count(), 5);
}

#[test]
fn the_keys_window_and_command_search_know_the_canvas() {
    use super::shortcuts::{Situation, bindings};
    let on_canvas: Vec<_> = bindings()
        .into_iter()
        .filter(|b| b.situation == Situation::Material)
        .collect();
    assert!(on_canvas.len() >= 10);
    let mut harness = fresh();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let new = entries
        .iter()
        .find(|e| e.name == "New material graph")
        .expect("in the command search");
    let action = new.action;
    let ctx = harness.ctx.clone();
    let bench = &mut harness.state_mut().workbench;
    let (mut actions, mut requests) = (Vec::new(), Vec::new());
    bench.run_search(action, &ctx, &mut actions, &mut requests);
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.mode, Mode::Material);
    assert_eq!(bench.document().scene().graphs().len(), 1);
}

#[test]
fn a_gradient_is_edited_in_attributes_and_undone() {
    let mut harness = page();
    let map = BoxId(6);
    let add = GraphEdit::AddBox {
        kind: "gradient_map".into(),
        at: Vec2::new(2.0 * COLUMN, 520.0),
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: add,
        })
        .unwrap();
    harness.run();
    let (top, _) = box_at(&mut harness, map);
    click(
        &mut harness,
        top.left_top() + egui::vec2(20.0, 6.0),
        Modifiers::NONE,
    );
    let colours = |harness: &H| match graph(harness).get(map).unwrap().settings.get("colours") {
        Some(matgraph::SettingValue::Gradient(g)) => g.stops().len(),
        _ => 3,
    };
    assert_eq!(colours(&harness), 3);
    harness.get_by_label("Add a colour").click();
    harness.run();
    assert_eq!(colours(&harness), 4);
    harness
        .get_all_by_label("×")
        .next()
        .expect("each colour can be taken out")
        .click();
    harness.run();
    assert_eq!(colours(&harness), 3);
    undo(&mut harness);
    assert_eq!(colours(&harness), 4);
}

#[test]
fn a_settings_switch_shows_it_in_the_painter_and_undo_hides_it() {
    // Decision 0261: Shown in the painter, under a box's settings.
    let mut harness = page();
    harness.state_mut().workbench.material().canvas.selected = [LEVELS].into();
    harness.run();
    harness
        .get_by_label(crate::material::box_rows::SHOWN_IN_PAINTER)
        .scroll_to_me();
    harness.run();
    let switch = harness
        .get_all_by_label_contains("Lightest")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .expect("a switch for Lightest");
    switch.click();
    harness.run();
    let shown = graph(&harness).shown(Registry::standard()).len();
    assert_eq!(shown, 1);
    assert!(graph(&harness).get(LEVELS).unwrap().shown.contains("high"));
    undo(&mut harness);
    assert!(graph(&harness).shown(Registry::standard()).is_empty());
}
