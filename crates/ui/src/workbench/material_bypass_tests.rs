//! Bypass and reroute dots in the Material mode (decision 0278), used
//! through the real interface: a box's switch, its right-click menu and
//! the Attributes tick bypass it (each one undo step); the boxes after a
//! bypass that can pass nothing say why and offer to switch it back on;
//! clicking a wire adds a reroute dot, dragging moves it, Delete removes
//! it; dots and bypass are copied with boxes, saved and undone.

use super::material_mode_tests::{
    BLEND, H, LEVELS, NOISE, box_at, click, drag, graph, page, plug, point_at_canvas, undo,
};
use super::*;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use matgraph::{BoxId, PlugRef};

/// The wire Levels → Blend's bottom, after `connected`.
fn into_blend() -> PlugRef {
    PlugRef::new(BLEND, "bottom")
}

/// `page` with Noise → Levels → Blend's bottom wired.
fn connected() -> H {
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
    harness
}

fn note(harness: &H) -> String {
    harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string()
}

fn bypassed(harness: &H, id: BoxId) -> bool {
    graph(harness).get(id).unwrap().bypassed
}

fn switch_of(harness: &mut H, id: BoxId) -> Pos2 {
    let (rect, _) = box_at(harness, id);
    crate::material::bypassing::switch_rect(rect).center()
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

fn wire_at(harness: &mut H, to: &PlugRef, stretch: usize) -> Pos2 {
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    bench
        .material()
        .wire_on_screen(&document, to, stretch)
        .expect("the wire is drawn")
}

fn dots_at(harness: &mut H, to: &PlugRef) -> Vec<Pos2> {
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    bench.material().dots_on_screen(&document, to)
}

#[test]
fn the_switch_on_a_box_s_title_bypasses_it_in_one_undo_step() {
    let mut harness = connected();
    let steps = harness.state().workbench.document().undo_steps();
    let at = switch_of(&mut harness, LEVELS);
    click(&mut harness, at, Modifiers::NONE);
    assert!(bypassed(&harness, LEVELS));
    assert!(
        note(&harness).contains("Levels is bypassed"),
        "{}",
        note(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    // Its picture is Noise's now, passed straight through.
    harness.run();
    let pictures = &harness.state_mut().workbench.material().pictures;
    let shown = pictures.shown().expect("worked out");
    assert_eq!(
        shown.evaluation.picture(LEVELS),
        shown.evaluation.picture(NOISE)
    );
    // Clicked again: back on.
    let at = switch_of(&mut harness, LEVELS);
    click(&mut harness, at, Modifiers::NONE);
    assert!(!bypassed(&harness, LEVELS));
    click(&mut harness, at, Modifiers::NONE);
    assert!(bypassed(&harness, LEVELS));
    undo(&mut harness);
    assert!(!bypassed(&harness, LEVELS));
}

#[test]
fn a_box_s_right_click_menu_bypasses_and_switches_it_back_on() {
    let mut harness = connected();
    let (levels, _) = box_at(&mut harness, LEVELS);
    right_click(&mut harness, levels.left_top() + egui::vec2(20.0, 6.0));
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .menu
            .is_some()
    );
    // The canvas's search didn't open instead.
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .search
            .is_none()
    );
    harness
        .get_by_label(crate::material::bypassing::BYPASS_LABEL)
        .click();
    harness.run();
    assert!(bypassed(&harness, LEVELS));
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .menu
            .is_none()
    );
    right_click(&mut harness, levels.left_top() + egui::vec2(20.0, 6.0));
    harness
        .get_by_label(crate::material::bypassing::SWITCH_ON_LABEL)
        .click();
    harness.run();
    assert!(!bypassed(&harness, LEVELS));
    // Esc closes the menu without doing anything.
    right_click(&mut harness, levels.left_top() + egui::vec2(20.0, 6.0));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        harness
            .state_mut()
            .workbench
            .material()
            .canvas
            .menu
            .is_none()
    );
    assert!(!bypassed(&harness, LEVELS));
}

#[test]
fn the_attributes_tick_bypasses_the_selected_box() {
    let mut harness = connected();
    harness.state_mut().workbench.material().canvas.selected = [LEVELS].into();
    harness.run();
    harness
        .get_by_label(crate::material::bypassing::BYPASSED_LABEL)
        .click();
    harness.run();
    assert!(bypassed(&harness, LEVELS));
    assert!(
        harness
            .query_all_by_label_contains("its settings do nothing until it is switched back on")
            .next()
            .is_some()
    );
}

#[test]
fn a_bypass_that_passes_nothing_is_explained_and_offers_to_switch_it_on() {
    // Noise makes a height from nothing: bypassed, Levels (fed by it)
    // can't run, and says why.
    let mut harness = connected();
    let at = switch_of(&mut harness, NOISE);
    click(&mut harness, at, Modifiers::NONE);
    assert!(bypassed(&harness, NOISE));
    harness.run();
    let problems = {
        let bench = &mut harness.state_mut().workbench;
        let g = graph_now(bench);
        bench.material().problems(&g)
    };
    let why = problems.get(&LEVELS).expect("Levels can't run");
    assert!(why.reason.contains("Noise is bypassed"), "{}", why.reason);
    assert_eq!(why.fixes[0], matgraph::Fix::StopBypass(NOISE));
    // Attributes offers the fix as a button: it switches Noise on.
    harness.state_mut().workbench.material().canvas.selected = [LEVELS].into();
    harness.run();
    harness.get_by_label("Switch Noise back on").click();
    harness.run();
    assert!(!bypassed(&harness, NOISE));
}

fn graph_now(bench: &mut Workbench) -> std::sync::Arc<matgraph::Graph> {
    let document = bench.document.clone();
    bench.material().current(&document).unwrap().clone()
}

#[test]
fn clicking_a_wire_adds_a_dot_that_drags_and_deletes() {
    let mut harness = connected();
    let to = into_blend();
    let steps = harness.state().workbench.document().undo_steps();
    let at = wire_at(&mut harness, &to, 0);
    click(&mut harness, at, Modifiers::NONE);
    assert_eq!(graph(&harness).bends(&to).len(), 1, "{}", note(&harness));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    // Selected: ringed, and Delete would remove it rather than a box.
    assert_eq!(
        harness.state_mut().workbench.material().canvas.dot,
        Some((to.clone(), 0))
    );
    // Dragged down: the wire bends round there, one undo step.
    let dot = dots_at(&mut harness, &to)[0];
    let was = graph(&harness).bends(&to)[0];
    drag(
        &mut harness,
        dot,
        dot + egui::vec2(0.0, 120.0),
        Modifiers::NONE,
    );
    let now = graph(&harness).bends(&to)[0];
    assert!(now.y > was.y + 60.0, "{was} → {now}");
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 2);
    // A second dot on the stretch after it goes after it.
    let at = wire_at(&mut harness, &to, 1);
    click(&mut harness, at, Modifiers::NONE);
    let dots = graph(&harness).bends(&to).to_vec();
    assert_eq!(dots.len(), 2);
    assert!(dots[1].x >= dots[0].x);
    // Delete removes the selected dot, not a box.
    let boxes = graph(&harness).box_count();
    point_at_canvas(&mut harness);
    harness.key_press(egui::Key::Delete);
    harness.run();
    assert_eq!(graph(&harness).bends(&to).len(), 1);
    assert_eq!(graph(&harness).box_count(), boxes);
    // Undo brings it back; undo again takes the move back.
    undo(&mut harness);
    assert_eq!(graph(&harness).bends(&to).len(), 2);
    undo(&mut harness);
    undo(&mut harness);
    assert_eq!(graph(&harness).bends(&to), [was]);
}

#[test]
fn a_dot_dragged_behind_its_output_is_put_back_between_the_ends() {
    let mut harness = connected();
    let to = into_blend();
    let at = wire_at(&mut harness, &to, 0);
    click(&mut harness, at, Modifiers::NONE);
    let dot = dots_at(&mut harness, &to)[0];
    let levels = plug(&mut harness, LEVELS, "out", true);
    drag(
        &mut harness,
        dot,
        Pos2::new(levels.x - 200.0, dot.y),
        Modifiers::NONE,
    );
    let g = graph(&harness);
    let x = g.bends(&to)[0].x;
    let start = g.get(LEVELS).unwrap().at.x + crate::material::look::BOX_WIDTH;
    let end = g.get(BLEND).unwrap().at.x;
    assert!(x > start && x < end, "{start} < {x} < {end}");
}

#[test]
fn dots_and_bypass_are_copied_with_boxes_and_saved() {
    let mut harness = connected();
    let to = into_blend();
    let at = wire_at(&mut harness, &to, 0);
    click(&mut harness, at, Modifiers::NONE);
    let at = switch_of(&mut harness, LEVELS);
    click(&mut harness, at, Modifiers::NONE);
    harness.state_mut().workbench.material().canvas.selected = [LEVELS, BLEND].into();
    point_at_canvas(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::C);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::V);
    harness.run();
    let g = graph(&harness);
    let pasted_levels = g
        .boxes()
        .find(|(id, b)| *id > BoxId(5) && b.kind == "levels")
        .map(|(id, b)| (id, b.bypassed))
        .unwrap();
    assert!(pasted_levels.1, "the copy is bypassed too");
    let pasted_blend = g
        .boxes()
        .find(|(id, b)| *id > BoxId(5) && b.kind == "blend")
        .unwrap()
        .0;
    let pasted_to = PlugRef::new(pasted_blend, "bottom");
    assert_eq!(
        g.bends(&pasted_to).len(),
        1,
        "the copied wire's dot came too"
    );
    // Saved and opened again: the same graph.
    let bytes = fileio::project::to_bytes(harness.state().workbench.document()).unwrap();
    let back = fileio::project::from_bytes(&bytes).unwrap();
    assert_eq!(
        back.scene().graphs(),
        harness.state().workbench.document().scene().graphs()
    );
}

#[test]
fn moving_both_boxes_takes_the_dots_along() {
    let mut harness = connected();
    let to = into_blend();
    let at = wire_at(&mut harness, &to, 0);
    click(&mut harness, at, Modifiers::NONE);
    let was = graph(&harness).bends(&to)[0];
    harness.state_mut().workbench.material().canvas.selected = [LEVELS, BLEND].into();
    let (levels, _) = box_at(&mut harness, LEVELS);
    let grab = levels.left_top() + egui::vec2(20.0, 6.0);
    drag(
        &mut harness,
        grab,
        grab + egui::vec2(0.0, 150.0),
        Modifiers::NONE,
    );
    let now = graph(&harness).bends(&to)[0];
    assert!(now.y > was.y + 80.0, "{was} → {now}");
}
