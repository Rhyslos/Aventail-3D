//! Assemblies on the shape canvas (decision 0274), used through the real
//! interface: plain and linked copies with Ctrl / Alt + C and V, a
//! linked copy's points edited changing every copy, groups made with
//! Alt + G and moved as one, flipping, the order, colours and heights,
//! pasting onto another Shape box's canvas, and undo for each.

use super::material_mode_tests::{drag, graph, undo};
use super::material_shape_tests::{
    SHAPE, at, canvas_page, click, double_click, drawing, note, press_key, shape, with_square,
};
use egui::{Event, Modifiers};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, GraphEdit, GraphId};
use shapes::{Change, PieceId};

type H = super::material_mode_tests::H;

/// Presses `key` with `modifiers` with the pointer over the canvas at
/// `spot` (a canvas place).
fn shortcut(harness: &mut H, modifiers: Modifiers, key: egui::Key, spot: Vec2) {
    let pos = at(harness, spot);
    harness.event(Event::PointerMoved(pos));
    harness.run();
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// The square selected with Q and a click.
fn square_selected(harness: &mut H) {
    with_square(harness);
    press_key(harness, egui::Key::Q);
    let middle = at(harness, Vec2::splat(0.5));
    click(harness, middle, Modifiers::NONE);
    assert_eq!(shape(harness).selected.len(), 1);
}

#[test]
fn ctrl_c_and_v_paste_a_plain_copy_at_the_pointer() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    shortcut(
        &mut harness,
        Modifiers::COMMAND,
        egui::Key::C,
        Vec2::splat(0.5),
    );
    assert!(note(&harness).contains("Copied 1 pieces"));
    shortcut(
        &mut harness,
        Modifiers::COMMAND,
        egui::Key::V,
        Vec2::new(0.2, 0.8),
    );
    let d = drawing(&harness);
    assert_eq!(d.pieces().len(), 2);
    let copy = &d.pieces()[1];
    assert!(copy.placed().middle().distance(Vec2::new(0.2, 0.8)) < 1e-4);
    assert_eq!(d.linked_count(copy.id), 1, "a plain copy");
    assert_eq!(shape(&mut harness).selected, [copy.id].into());
    undo(&mut harness);
    assert_eq!(drawing(&harness).pieces().len(), 1);
}

#[test]
fn a_linked_copys_points_edited_change_every_copy() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    shortcut(&mut harness, Modifiers::ALT, egui::Key::C, Vec2::splat(0.5));
    shortcut(
        &mut harness,
        Modifiers::ALT,
        egui::Key::V,
        Vec2::new(0.25, 0.25),
    );
    let d = drawing(&harness);
    let (first, copy) = (d.pieces()[0].id, d.pieces()[1].id);
    assert_eq!(d.linked_count(first), 2);
    assert!(note(&harness).contains("linked copies"));
    // Turn the copy a quarter, so its points aren't where the first's are.
    let middle = d.piece(copy).unwrap().placed().middle();
    let turned = d
        .with_change(
            &[copy],
            &Change::turned(middle, std::f32::consts::FRAC_PI_2),
        )
        .unwrap();
    let workbench = &mut harness.state_mut().workbench;
    workbench
        .material
        .set_drawing(&mut workbench.document, turned, "Turn", None)
        .unwrap();
    harness.run();
    // Double-click the copy and drag one of its corners out.
    let spot = at(&mut harness, Vec2::new(0.25, 0.25));
    double_click(&mut harness, spot);
    assert_eq!(shape(&mut harness).editing, Some(copy));
    let corner = drawing(&harness).piece(copy).unwrap().placed().knots()[0].at;
    let from = at(&mut harness, corner);
    let to = at(&mut harness, corner + Vec2::new(-0.05, -0.05));
    drag(&mut harness, from, to, Modifiers::NONE);
    let d = drawing(&harness);
    let (a, b) = (d.piece(first).unwrap(), d.piece(copy).unwrap());
    assert_eq!(a.path, b.path, "one shape");
    assert_ne!(a.place, b.place, "each in its own place");
    assert!(
        b.placed().knots()[0]
            .at
            .distance(corner + Vec2::splat(-0.05))
            < 0.01
    );
    // The first changed too: it isn't the square it was.
    let square = shapes::Basic::Square.path(Vec2::splat(0.5), 0.2);
    assert_ne!(a.path, square);
    undo(&mut harness);
    assert_eq!(drawing(&harness).piece(first).unwrap().path, square);
}

#[test]
fn alt_g_groups_pieces_that_then_select_and_move_as_one() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    shortcut(
        &mut harness,
        Modifiers::COMMAND,
        egui::Key::C,
        Vec2::splat(0.5),
    );
    shortcut(
        &mut harness,
        Modifiers::COMMAND,
        egui::Key::V,
        Vec2::new(0.2, 0.2),
    );
    // Select both: the copy is selected; Ctrl + click adds the first.
    let first_spot = at(&mut harness, Vec2::splat(0.55));
    click(&mut harness, first_spot, Modifiers::COMMAND);
    assert_eq!(shape(&mut harness).selected.len(), 2);
    shortcut(
        &mut harness,
        Modifiers::ALT,
        egui::Key::G,
        Vec2::splat(0.95),
    );
    let d = drawing(&harness);
    assert_eq!(d.groups().len(), 1);
    // A click on one piece now selects the whole group.
    let empty = at(&mut harness, Vec2::splat(0.97));
    click(&mut harness, empty, Modifiers::NONE);
    assert!(shape(&mut harness).selected.is_empty());
    click(&mut harness, first_spot, Modifiers::NONE);
    assert_eq!(shape(&mut harness).selected.len(), 2);
    // The list shows the group as a heading.
    assert!(harness.query_by_label_contains("group of 2").is_some());
    // Dragging one moves both, as one undo step.
    let before: Vec<Vec2> = d.pieces().iter().map(|p| p.placed().middle()).collect();
    press_key(&mut harness, egui::Key::W);
    let to = at(&mut harness, Vec2::new(0.65, 0.55));
    drag(&mut harness, first_spot, to, Modifiers::NONE);
    let d = drawing(&harness);
    for (p, was) in d.pieces().iter().zip(&before) {
        assert!(
            p.placed().middle().distance(*was + Vec2::new(0.1, 0.0)) < 0.01,
            "{}",
            p.placed().middle()
        );
    }
    undo(&mut harness);
    // Shift + G takes the group apart.
    click(&mut harness, first_spot, Modifiers::NONE);
    shortcut(
        &mut harness,
        Modifiers::SHIFT,
        egui::Key::G,
        Vec2::splat(0.95),
    );
    assert!(drawing(&harness).groups().is_empty());
    undo(&mut harness);
    assert_eq!(drawing(&harness).groups().len(), 1);
}

#[test]
fn the_flip_buttons_turn_pieces_over() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    harness.get_by_label("Flip ↔").click();
    harness.run();
    let d = drawing(&harness);
    assert!(d.pieces()[0].place.flips());
    assert!(d.pieces()[0].placed().middle().distance(Vec2::splat(0.5)) < 1e-5);
    harness.get_by_label("Flip ↕").click();
    harness.run();
    assert!(
        !drawing(&harness).pieces()[0].place.flips(),
        "flipped twice"
    );
    undo(&mut harness);
    undo(&mut harness);
    assert_eq!(drawing(&harness).pieces()[0].place, Change::IDENTITY);
}

#[test]
fn up_and_down_change_the_order_and_groups_move_as_one() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    for spot in [Vec2::new(0.2, 0.2), Vec2::new(0.8, 0.8)] {
        shortcut(
            &mut harness,
            Modifiers::COMMAND,
            egui::Key::C,
            Vec2::splat(0.5),
        );
        shortcut(&mut harness, Modifiers::COMMAND, egui::Key::V, spot);
    }
    let ids = |h: &H| drawing(h).pieces().iter().map(|p| p.id).collect::<Vec<_>>();
    let start = ids(&harness);
    assert_eq!(start.len(), 3);
    // The last pasted is selected: Down puts it under the one before.
    harness.get_by_label("Down").click();
    harness.run();
    assert_eq!(ids(&harness), vec![start[0], start[2], start[1]]);
    undo(&mut harness);
    // Group the bottom two; the group moves up past the top piece.
    {
        let s = shape(&mut harness);
        s.selected = [start[0], start[1]].into();
    }
    harness.run();
    harness.get_by_label("Group").click();
    harness.run();
    harness.get_by_label("Group up").click();
    harness.run();
    assert_eq!(ids(&harness), vec![start[2], start[0], start[1]]);
}

#[test]
fn colours_and_heights_belong_to_each_piece_and_show_in_the_box() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    {
        let workbench = &mut harness.state_mut().workbench;
        let (editor, doc) = (&mut workbench.material, &mut workbench.document);
        let mut note = None;
        editor.colour_chosen(doc, Some([0.8, 0.2, 0.1]), None, None, &mut note);
        editor.colour_chosen(doc, None, Some(0.5), None, &mut note);
    }
    harness.run();
    let d = drawing(&harness);
    assert_eq!(d.pieces()[0].colour, [0.8, 0.2, 0.1]);
    assert_eq!(d.pieces()[0].height, 0.5);
    // The box's Colours output has the piece's colour in its middle.
    let registry = matgraph::Registry::standard();
    let g = graph(&harness);
    let result = matgraph::Evaluator::with_threads(1).run(
        registry,
        &g,
        matgraph::Size::nearest(32),
        &[SHAPE],
    );
    let out = result.outcome(SHAPE).unwrap().as_ref().unwrap();
    let colours = out.outputs[3].as_ref().unwrap();
    match colours {
        matgraph::Value::Colour(c) => {
            let [r, g, b] = c.at(16, 16);
            assert!((r - 0.8).abs() < 0.01 && (g - 0.2).abs() < 0.01 && (b - 0.1).abs() < 0.01);
        }
        other => panic!("not a colour: {:?}", other.kind()),
    }
    // The strip shows four pictures, Colours among them.
    assert!(harness.query_by_label("Colours").is_some());
    undo(&mut harness);
    assert_eq!(drawing(&harness).pieces()[0].height, 1.0);
}

#[test]
fn pieces_paste_onto_another_shape_canvas_linked_only_to_each_other() {
    let mut harness = canvas_page();
    square_selected(&mut harness);
    shortcut(&mut harness, Modifiers::ALT, egui::Key::C, Vec2::splat(0.5));
    shortcut(
        &mut harness,
        Modifiers::ALT,
        egui::Key::V,
        Vec2::new(0.2, 0.2),
    );
    let both: Vec<PieceId> = drawing(&harness).pieces().iter().map(|p| p.id).collect();
    shape(&mut harness).selected = both.into_iter().collect();
    shortcut(
        &mut harness,
        Modifiers::COMMAND,
        egui::Key::C,
        Vec2::splat(0.95),
    );
    // A second Shape box, its canvas open.
    let other = BoxId(7);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: GraphEdit::AddBox {
                kind: matgraph::boxes::drawn_shape::DRAWN_SHAPE.into(),
                at: Vec2::new(0.0, 900.0),
            },
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .material()
        .open_shape_canvas(other);
    harness.run();
    harness.run();
    shortcut(&mut harness, Modifiers::ALT, egui::Key::V, Vec2::splat(0.5));
    let pasted = graph(&harness).get(other).unwrap().drawing.clone().unwrap();
    assert_eq!(pasted.pieces().len(), 2);
    assert_eq!(pasted.linked_count(pasted.pieces()[0].id), 2);
    // The first canvas is unchanged.
    assert_eq!(
        graph(&harness)
            .get(SHAPE)
            .unwrap()
            .drawing
            .as_ref()
            .unwrap()
            .pieces()
            .len(),
        2
    );
}
