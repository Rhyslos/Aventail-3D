//! Finding boxes and the minimap in the Material mode (decision 0279),
//! used through the real interface: typing in the canvas bar's field
//! selects and frames the first match, next and previous (and Enter) go
//! along the matches, the list goes to one, a box's menu finds others
//! like it, the command search puts the cursor in the field; the minimap
//! shows on a graph too big for the screen, a click or a drag on it moves
//! the view there without touching the graph, and the View menu switches
//! it off.

use super::material_mode_tests::{
    BLEND, H, LEVELS, NOISE, TILES, box_at, canvas, click, drag, graph, page,
};
use super::*;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Frame, GraphEdit, GraphId};

fn find(harness: &mut H, words: &str) {
    harness
        .get_by_label(crate::material::finding::FIND_LABEL)
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::A);
    harness
        .get_by_label(crate::material::finding::FIND_LABEL)
        .type_text(words);
    harness.run();
    harness.run();
}

fn selected(harness: &mut H) -> Vec<BoxId> {
    let m = harness.state_mut().workbench.material();
    m.canvas.selected.iter().copied().collect()
}

/// Whether box `id` is drawn whole on the canvas.
fn on_screen(harness: &mut H, id: BoxId) -> bool {
    let (rect, _) = box_at(harness, id);
    canvas(harness).contains_rect(rect)
}

/// The canvas point in the middle of the canvas.
fn middle(harness: &mut H) -> Vec2 {
    let rect = canvas(harness);
    let m = harness.state_mut().workbench.material();
    m.canvas.view.on_canvas(rect, rect.center())
}

fn edit(harness: &mut H, edit: GraphEdit) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit,
        })
        .unwrap();
    harness.run();
}

/// Moves the view so the graph's top left is far off the bottom right:
/// only part of the graph shows, and the minimap appears.
fn look_away(harness: &mut H) {
    let m = harness.state_mut().workbench.material();
    m.canvas.view = crate::material::view::View {
        pan: Vec2::new(900.0, 600.0),
        zoom: 1.0,
    };
    harness.run();
    harness.run();
}

fn map(harness: &mut H) -> Option<crate::material::minimap::MapPlace> {
    harness.state_mut().workbench.material().canvas.map
}

#[test]
fn typing_in_the_bar_selects_and_frames_the_first_match() {
    let mut harness = page();
    look_away(&mut harness);
    assert!(!on_screen(&mut harness, NOISE));
    let steps = harness.state().workbench.document().undo_steps();
    find(&mut harness, "noise");
    assert_eq!(selected(&mut harness), [NOISE]);
    assert!(on_screen(&mut harness, NOISE), "the canvas frames it");
    let (rect, _) = box_at(&mut harness, NOISE);
    assert!((rect.center() - canvas(&mut harness).center()).length() < 2.0);
    // Finding changes nothing in the graph.
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    // The count and the list.
    assert!(harness.query_by_label("1 of 1").is_some());
    assert!(
        harness
            .query_by_label(crate::material::finding::MATCHES_LABEL)
            .is_some()
    );
}

#[test]
fn next_previous_and_enter_go_along_the_matches_in_reading_order() {
    let mut harness = page();
    // Frames named after what they hold, so "stone" finds a frame too.
    edit(
        &mut harness,
        GraphEdit::AddFrame(Frame::new(
            "Stone pattern",
            Vec2::new(-20.0, 480.0),
            Vec2::new(200.0, 800.0),
        )),
    );
    // Blend's mode set to Multiply: found by its choice.
    edit(
        &mut harness,
        GraphEdit::SetSetting {
            node: BLEND,
            key: "mode".into(),
            value: matgraph::SettingValue::Choice(3),
        },
    );
    find(&mut harness, "l");
    // Levels, Colour fill, Blend (its name), Tiles, the frame: "l" is in
    // Levels, Colour fill, Blend, Tiles; reading order is left to right.
    let found: Vec<_> = harness
        .state_mut()
        .workbench
        .material()
        .canvas
        .finding
        .found
        .iter()
        .map(|f| f.thing)
        .collect();
    use matgraph::FoundThing::Box as B;
    assert_eq!(found[0], B(BoxId(3)), "Colour fill is left, above Tiles");
    assert!(found.contains(&B(TILES)));
    assert!(found.contains(&B(LEVELS)));
    let first = selected(&mut harness);
    harness
        .get_by_label(crate::material::finding::NEXT_LABEL)
        .click();
    harness.run();
    let second = selected(&mut harness);
    assert_ne!(first, second);
    harness
        .get_by_label(crate::material::finding::PREVIOUS_LABEL)
        .click();
    harness.run();
    assert_eq!(selected(&mut harness), first);
    // Previous from the first wraps to the last.
    harness
        .get_by_label(crate::material::finding::PREVIOUS_LABEL)
        .click();
    harness.run();
    let last = harness.state_mut().workbench.material().canvas.finding.at;
    assert_eq!(last, Some(found.len() - 1));
    // Enter in the field goes on, wrapping back to the first.
    harness
        .get_by_label(crate::material::finding::FIND_LABEL)
        .click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(selected(&mut harness), first);
    // A choice: "multiply" finds Blend; a frame's title finds the frame.
    find(&mut harness, "multiply");
    assert_eq!(selected(&mut harness), [BLEND]);
    find(&mut harness, "stone pattern");
    let m = harness.state_mut().workbench.material();
    assert!(m.canvas.selected.is_empty());
    assert!(m.canvas.frame.is_some(), "the frame is selected");
}

#[test]
fn a_click_in_the_list_goes_to_that_match_and_esc_empties_the_field() {
    let mut harness = page();
    find(&mut harness, "e");
    let rows: Vec<String> = harness
        .state_mut()
        .workbench
        .material()
        .canvas
        .finding
        .found
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert!(rows.len() >= 3, "{rows:?}");
    // The row in the list, just under the field (Attributes may name
    // Levels too).
    let field = harness
        .get_by_label(crate::material::finding::FIND_LABEL)
        .rect();
    harness
        .get_all_by_label("Levels")
        .filter(|n| n.rect().min.y > field.max.y && (n.rect().max.x - field.max.x).abs() < 40.0)
        .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .expect("Levels in the list")
        .click();
    harness.run();
    assert_eq!(selected(&mut harness), [LEVELS]);
    // The list closed; the words stay, marked boxes still ringed.
    assert!(
        harness
            .query_by_label(crate::material::finding::MATCHES_LABEL)
            .is_none()
    );
    harness
        .get_by_label(crate::material::finding::FIND_LABEL)
        .click();
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    let m = harness.state_mut().workbench.material();
    assert!(m.canvas.finding.query.is_empty());
    assert!(m.canvas.finding.marked().0.is_empty());
    // Esc there leaves the selection as it was.
    assert_eq!(selected(&mut harness), [LEVELS]);
}

#[test]
fn nothing_found_says_so() {
    let mut harness = page();
    find(&mut harness, "zebra");
    assert!(harness.query_by_label("No match").is_some());
    assert!(selected(&mut harness).is_empty());
}

#[test]
fn a_box_s_menu_finds_others_like_it() {
    let mut harness = page();
    let added = BoxId(graph(&harness).next_id());
    edit(
        &mut harness,
        GraphEdit::AddBox {
            kind: "noise".into(),
            at: Vec2::new(0.0, 900.0),
        },
    );
    let (noise, _) = box_at(&mut harness, NOISE);
    let at = noise.left_top() + egui::vec2(20.0, 6.0);
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
    harness
        .get_by_label(crate::material::finding::FIND_LIKE_LABEL)
        .click();
    harness.run();
    let m = harness.state_mut().workbench.material();
    assert_eq!(m.canvas.finding.query, "Noise");
    assert_eq!(m.canvas.finding.found.len(), 2);
    assert_eq!(
        selected(&mut harness),
        [NOISE],
        "it starts at the box itself"
    );
    harness
        .get_by_label(crate::material::finding::NEXT_LABEL)
        .click();
    harness.run();
    assert_eq!(selected(&mut harness), [added]);
    assert!(on_screen(&mut harness, added));
}

#[test]
fn the_command_search_puts_the_cursor_in_the_field() {
    let mut harness = page();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries
        .iter()
        .find(|e| e.name == "Find a box in the graph…")
        .expect("in the command search");
    let action = entry.action;
    let ctx = harness.ctx.clone();
    let bench = &mut harness.state_mut().workbench;
    let (mut actions, mut requests) = (Vec::new(), Vec::new());
    bench.run_search(action, &ctx, &mut actions, &mut requests);
    harness.run();
    harness.run();
    // Typing now types into the field.
    harness.event(Event::Text("tiles".into()));
    harness.run();
    harness.run();
    assert_eq!(selected(&mut harness), [TILES]);
}

#[test]
fn the_minimap_shows_only_when_the_graph_doesnt_fit() {
    let mut harness = page();
    assert!(map(&mut harness).is_none(), "the whole graph fits");
    look_away(&mut harness);
    let shown = map(&mut harness).expect("part of the graph is off screen");
    assert!(canvas(&mut harness).contains_rect(shown.rect));
    // Framed whole again (Z with nothing selected): hidden.
    let rect = canvas(&mut harness);
    let bench = &mut harness.state_mut().workbench;
    let g = graph_now(bench);
    bench.material().frame_view(&g, rect, false);
    harness.run();
    harness.run();
    assert!(map(&mut harness).is_none());
}

fn graph_now(bench: &mut Workbench) -> std::sync::Arc<matgraph::Graph> {
    bench
        .document()
        .scene()
        .graphs()
        .get(GraphId(1))
        .unwrap()
        .clone()
}

#[test]
fn a_click_on_the_minimap_moves_the_view_there_and_nothing_else() {
    let mut harness = page();
    look_away(&mut harness);
    let map_now = map(&mut harness).unwrap();
    let steps = harness.state().workbench.document().undo_steps();
    let before = graph(&harness);
    // Where Tiles is, on the map.
    let tiles = graph(&harness).get(TILES).unwrap().at;
    let at = map_now.on_map(tiles + Vec2::splat(50.0));
    click(&mut harness, at, Modifiers::NONE);
    let wanted = map_now.on_canvas(at);
    assert!((middle(&mut harness) - wanted).length() < 1.0);
    assert!(on_screen(&mut harness, TILES));
    // The canvas under it took no click: nothing selected, nothing moved.
    assert!(selected(&mut harness).is_empty());
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(*graph(&harness), *before);
}

#[test]
fn dragging_on_the_minimap_follows_the_pointer() {
    let mut harness = page();
    look_away(&mut harness);
    let held = map(&mut harness).unwrap();
    let steps = harness.state().workbench.document().undo_steps();
    let from = held.rect.center();
    let to = held.rect.left_top() + egui::vec2(12.0, 12.0);
    drag(&mut harness, from, to, Modifiers::NONE);
    // The map kept its place during the drag: the view's middle is the
    // canvas point under where the pointer let go.
    assert!((middle(&mut harness) - held.on_canvas(to)).length() < 1.0);
    assert!(selected(&mut harness).is_empty());
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .map_drag
            .is_none()
    );
}

#[test]
fn the_canvas_view_menu_switches_the_minimap_off_and_on() {
    let mut harness = page();
    look_away(&mut harness);
    assert!(map(&mut harness).is_some());
    for (click_it, shown) in [(true, false), (true, true)] {
        if click_it {
            canvas_view_menu(&mut harness);
            harness
                .get_by_label(crate::material::minimap::MINIMAP_LABEL)
                .click();
            harness.run();
            harness.key_press(egui::Key::Escape);
            harness.run();
            harness.run();
        }
        assert_eq!(map(&mut harness).is_some(), shown);
    }
}

#[test]
fn the_minimap_inside_your_own_box_maps_the_boxes_inside() {
    let mut harness = page();
    // Noise and Levels joined, then made into your own box and opened.
    let a = super::material_mode_tests::plug(&mut harness, NOISE, "height", true);
    let b = super::material_mode_tests::plug(&mut harness, LEVELS, "in", false);
    drag(&mut harness, a, b, Modifiers::NONE);
    let m = harness.state_mut().workbench.material();
    m.canvas.selected = [NOISE, LEVELS].into();
    let own = {
        let bench = &mut harness.state_mut().workbench;
        let document = bench.document.clone();
        let g = graph_now(bench);
        let registry = document.scene().graphs().registry().clone();
        let mut proposal =
            matgraph::own::Proposal::of(&g, &registry, &[NOISE, LEVELS].into()).unwrap();
        proposal.name = "Rough".into();
        let id = matgraph::own::fresh_id("Rough", |_| false);
        let (own, edits) = proposal.make(&g, &registry, id, None).unwrap();
        let own = std::sync::Arc::new(own);
        bench
            .document_mut()
            .execute(scene::Command::SetOwnBox {
                id: own.id.clone(),
                own: Some((*own).clone()),
            })
            .unwrap();
        for e in edits {
            bench
                .document_mut()
                .execute(scene::Command::EditGraph {
                    id: GraphId(1),
                    edit: e,
                })
                .unwrap();
        }
        own
    };
    harness.run();
    let placed = graph(&harness)
        .boxes()
        .find(|(_, b)| b.kind == own.id)
        .map(|(id, _)| id)
        .unwrap();
    // Opened, then looked away from inside.
    let bench = &mut harness.state_mut().workbench;
    bench.material().canvas.selected = [placed].into();
    bench.material().open_inside(own.id.clone(), Some(placed));
    harness.run();
    look_away(&mut harness);
    let shown = map(&mut harness).expect("a map of the inside");
    // It holds the boxes inside (two), not the outer graph.
    let inner = &own.inner;
    for (_, b) in inner.boxes() {
        assert!(shown.rect.contains(shown.on_map(b.at)));
    }
    let bench = &mut harness.state_mut().workbench;
    assert_eq!(bench.material().inside.len(), 1);
}

/// Opens the canvas bar's own View menu (the one just above the canvas).
fn canvas_view_menu(harness: &mut H) {
    let rect = canvas(harness);
    harness
        .get_all_by_label("View")
        .filter(|n| n.rect().center().x > rect.min.x && n.rect().center().x < rect.max.x)
        .min_by(|a, b| {
            (a.rect().max.y - rect.min.y)
                .abs()
                .total_cmp(&(b.rect().max.y - rect.min.y).abs())
        })
        .unwrap()
        .click();
    harness.run();
}

#[test]
fn the_view_menu_frames_the_whole_graph() {
    let mut harness = page();
    look_away(&mut harness);
    canvas_view_menu(&mut harness);
    harness
        .get_by_label(crate::material::minimap::FRAME_ALL_LABEL)
        .click();
    harness.run();
    harness.run();
    for id in [NOISE, LEVELS, BLEND, TILES] {
        assert!(on_screen(&mut harness, id));
    }
    let _: Pos2 = canvas(&mut harness).center();
}
