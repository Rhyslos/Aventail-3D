//! The shape canvas (decision 0273), used through the real interface:
//! opening it from a Shape box, drawing with the pen (corners, curves,
//! closing on the first point), cut-outs with Alt or the switch, an open
//! line refused with ways to make it work, the mirror line, basic shapes,
//! moving, editing points, Tab back and Esc only cancelling, the
//! reference image, and undo for each.

use super::material_mode_tests::{H, box_at, drag, graph, quick_page, undo};
use super::*;
use crate::material::look::COLUMN;
use crate::material::shape_canvas::{self, ShapeTool};
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, GraphEdit, GraphId};
use shapes::{Basic, Drawing, PieceKind};
use std::sync::Arc;

/// The Shape box added to the page's graph.
pub(super) const SHAPE: BoxId = BoxId(6);

/// The page with a Shape box added beside its boxes, not yet opened.
pub(super) fn with_shape_box() -> H {
    let mut harness = quick_page();
    let edit = GraphEdit::AddBox {
        kind: matgraph::boxes::drawn_shape::DRAWN_SHAPE.into(),
        at: Vec2::new(4.0 * COLUMN, 0.0),
    };
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
    assert!(graph(&harness).get(SHAPE).is_some());
    harness
}

/// The page with its Shape box's canvas open.
pub(super) fn canvas_page() -> H {
    let mut harness = with_shape_box();
    harness
        .state_mut()
        .workbench
        .material()
        .open_shape_canvas(SHAPE);
    harness.run();
    harness.run();
    harness
}

pub(super) fn drawing(harness: &H) -> Arc<Drawing> {
    graph(harness)
        .get(SHAPE)
        .unwrap()
        .drawing
        .clone()
        .unwrap_or_default()
}

pub(super) fn shape(harness: &mut H) -> &mut shape_canvas::ShapeCanvas {
    harness
        .state_mut()
        .workbench
        .material()
        .shape
        .as_mut()
        .unwrap()
}

fn is_open(harness: &mut H) -> bool {
    harness.state_mut().workbench.material().shape.is_some()
}

/// Where the canvas place `p` is on screen.
pub(super) fn at(harness: &mut H, p: Vec2) -> Pos2 {
    let s = shape(harness);
    s.view.on_screen(s.area.unwrap(), p)
}

/// Points at a spot of the canvas with nothing on it, so its keys go to
/// it.
pub(super) fn point_at_shape_canvas(harness: &mut H) {
    let spot = at(harness, Vec2::new(0.97, 0.97));
    harness.event(Event::PointerMoved(spot));
    harness.run();
}

pub(super) fn press_key(harness: &mut H, key: egui::Key) {
    point_at_shape_canvas(harness);
    harness.key_press(key);
    harness.run();
}

/// A click at `pos`, then a pause long enough that the next one is
/// never a double-click.
pub(super) fn click(harness: &mut H, pos: Pos2, modifiers: Modifiers) {
    super::material_mode_tests::click(harness, pos, modifiers);
    for _ in 0..30 {
        harness.step();
    }
}

pub(super) fn double_click(harness: &mut H, pos: Pos2) {
    // Long enough after any click before not to make a triple click.
    for _ in 0..70 {
        harness.step();
    }
    harness.event(Event::PointerMoved(pos));
    harness.step();
    for pressed in [true, false, true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

/// Clicks with the pen at each place, then on the first to close.
fn pen_triangle(harness: &mut H, corners: [Vec2; 3], modifiers: Modifiers) {
    press_key(harness, egui::Key::P);
    for (i, c) in corners.iter().enumerate() {
        let pos = at(harness, *c);
        click(
            harness,
            pos,
            if i == 0 { modifiers } else { Modifiers::NONE },
        );
    }
    let first = at(harness, corners[0]);
    click(harness, first, Modifiers::NONE);
}

const TRIANGLE: [Vec2; 3] = [
    Vec2::new(0.3, 0.3),
    Vec2::new(0.7, 0.3),
    Vec2::new(0.5, 0.7),
];

pub(super) fn note(harness: &H) -> String {
    harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string()
}

/// The page's Shape box with a square drawn on it (as one undo step).
pub(super) fn with_square(harness: &mut H) {
    let square = Drawing::new()
        .with_piece(
            "Square",
            Basic::Square.path(Vec2::splat(0.5), 0.2),
            PieceKind::Add,
        )
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: GraphEdit::SetDrawing {
                node: SHAPE,
                drawing: Arc::new(square),
            },
        })
        .unwrap();
    harness.run();
}

#[test]
fn double_clicking_a_shape_box_opens_its_canvas_and_tab_goes_back() {
    let mut harness = with_shape_box();
    let (rect, _) = box_at(&mut harness, SHAPE);
    double_click(&mut harness, rect.left_top() + egui::vec2(20.0, 6.0));
    harness.run();
    assert!(is_open(&mut harness), "the canvas opened");
    assert!(
        harness
            .query_by_label(shape_canvas::SHAPE_CANVAS_LABEL)
            .is_some()
    );
    // Esc has nothing to cancel: the canvas stays.
    press_key(&mut harness, egui::Key::Escape);
    assert!(is_open(&mut harness));
    press_key(&mut harness, egui::Key::Tab);
    assert!(!is_open(&mut harness), "Tab goes back to the graph");
    assert_eq!(harness.state().workbench.mode(), Mode::Material);
    assert_eq!(note(&harness), "Back to the graph.");
    // Attributes' button opens it too.
    harness.state_mut().workbench.material().canvas.selected = [SHAPE].into();
    harness.run();
    harness.get_by_label(shape_canvas::OPEN_LABEL).click();
    harness.run();
    assert!(is_open(&mut harness));
    harness.get_by_label(shape_canvas::BACK_LABEL).click();
    harness.run();
    assert!(!is_open(&mut harness));
}

#[test]
fn the_pen_draws_a_shape_closed_on_its_first_point_and_undo_takes_it_back() {
    let mut harness = canvas_page();
    pen_triangle(&mut harness, TRIANGLE, Modifiers::NONE);
    let d = drawing(&harness);
    assert_eq!(d.pieces().len(), 1, "{}", note(&harness));
    let piece = &d.pieces()[0];
    assert_eq!(piece.kind, PieceKind::Add);
    assert!(piece.path.is_closed());
    assert_eq!(piece.path.knots().len(), 3);
    for (k, c) in piece.path.knots().iter().zip(TRIANGLE) {
        assert!(k.at.distance(c) < 0.01, "{k:?} {c}");
        assert!(k.is_corner());
    }
    assert!(shape(&mut harness).pen.is_none());
    assert!(shape(&mut harness).selected.contains(&piece.id));
    // The box gives the shape.
    let mask = shapes::raster::mask(&d, 64);
    assert!(mask.iter().sum::<f32>() > 64.0 * 64.0 * 0.05);
    undo(&mut harness);
    assert!(drawing(&harness).is_empty());
}

#[test]
fn dragging_with_the_pen_pulls_out_a_curve() {
    let mut harness = canvas_page();
    press_key(&mut harness, egui::Key::P);
    let a = at(&mut harness, TRIANGLE[0]);
    click(&mut harness, a, Modifiers::NONE);
    let b = at(&mut harness, TRIANGLE[1]);
    let pulled = at(&mut harness, TRIANGLE[1] + Vec2::new(0.0, 0.1));
    drag(&mut harness, b, pulled, Modifiers::NONE);
    let c = at(&mut harness, TRIANGLE[2]);
    click(&mut harness, c, Modifiers::NONE);
    click(&mut harness, a, Modifiers::NONE);
    let d = drawing(&harness);
    let k = d.pieces()[0].path.knots()[1];
    assert!(!k.is_corner(), "a curve");
    assert!(k.out.distance(Vec2::new(0.0, 0.1)) < 0.01, "{k:?}");
    assert!(k.inn.distance(Vec2::new(0.0, -0.1)) < 0.01);
}

#[test]
fn alt_or_the_cut_switch_makes_a_cut_out() {
    let mut harness = canvas_page();
    pen_triangle(&mut harness, TRIANGLE, Modifiers::ALT);
    assert_eq!(drawing(&harness).pieces()[0].kind, PieceKind::Cut);
    harness.get_by_label("Cut").click();
    harness.run();
    assert!(shape(&mut harness).cut);
    let moved = TRIANGLE.map(|c| c + Vec2::new(0.0, 0.05));
    pen_triangle(&mut harness, moved, Modifiers::NONE);
    assert_eq!(drawing(&harness).pieces()[1].kind, PieceKind::Cut);
    // Alt with the switch on Cut adds.
    let lower = TRIANGLE.map(|c| c + Vec2::new(0.0, 0.1));
    pen_triangle(&mut harness, lower, Modifiers::ALT);
    assert_eq!(drawing(&harness).pieces()[2].kind, PieceKind::Add);
}

#[test]
fn enter_on_an_open_line_says_why_and_offers_to_close_it() {
    let mut harness = canvas_page();
    press_key(&mut harness, egui::Key::P);
    for c in TRIANGLE {
        let pos = at(&mut harness, c);
        click(&mut harness, pos, Modifiers::NONE);
    }
    press_key(&mut harness, egui::Key::Enter);
    assert!(drawing(&harness).is_empty(), "nothing yet");
    assert!(note(&harness).contains("open path"), "{}", note(&harness));
    harness.get_by_label("Close it").click();
    harness.run();
    let d = drawing(&harness);
    assert_eq!(d.pieces().len(), 1);
    assert!(d.pieces()[0].path.is_closed());
    assert_eq!(d.pieces()[0].kind, PieceKind::Add);
    // Kept as a line instead: a vein, drawn in the outline only.
    for c in [Vec2::new(0.2, 0.8), Vec2::new(0.8, 0.85)] {
        let pos = at(&mut harness, c);
        click(&mut harness, pos, Modifiers::NONE);
    }
    press_key(&mut harness, egui::Key::Enter);
    harness.get_by_label("Keep it as a line").click();
    harness.run();
    let d = drawing(&harness);
    assert_eq!(d.pieces()[1].kind, PieceKind::Line);
    assert!(!d.pieces()[1].path.is_closed());
}

#[test]
fn esc_only_cancels_the_pen_line_and_tab_waits_for_it() {
    let mut harness = canvas_page();
    press_key(&mut harness, egui::Key::P);
    for c in &TRIANGLE[..2] {
        let pos = at(&mut harness, *c);
        click(&mut harness, pos, Modifiers::NONE);
    }
    assert_eq!(shape(&mut harness).pen.as_ref().unwrap().knots.len(), 2);
    press_key(&mut harness, egui::Key::Tab);
    assert!(is_open(&mut harness), "Tab waits for the line");
    assert!(note(&harness).contains("Esc"), "{}", note(&harness));
    press_key(&mut harness, egui::Key::Escape);
    assert!(shape(&mut harness).pen.is_none());
    assert!(drawing(&harness).is_empty());
    assert!(is_open(&mut harness), "Esc only cancels");
    assert_eq!(shape(&mut harness).tool, ShapeTool::Pen);
}

#[test]
fn m_turns_the_mirror_line_on_and_a_half_drawn_against_it_comes_out_whole() {
    let mut harness = canvas_page();
    press_key(&mut harness, egui::Key::M);
    assert!(drawing(&harness).mirror().on);
    // A half leaf against the line (points snap onto it).
    let half = [
        Vec2::new(0.501, 0.2),
        Vec2::new(0.3, 0.5),
        Vec2::new(0.499, 0.8),
    ];
    pen_triangle(&mut harness, half, Modifiers::NONE);
    let d = drawing(&harness);
    let piece = &d.pieces()[0];
    assert_eq!(piece.path.knots()[0].at.x, 0.5, "snapped onto the line");
    assert_eq!(d.drawn_paths(piece).len(), 2);
    let mask = shapes::raster::mask(&d, 64);
    let left: f32 = (0..64)
        .flat_map(|y| (0..32).map(move |x| (x, y)))
        .map(|(x, y)| mask[y * 64 + x])
        .sum();
    let right: f32 = (0..64)
        .flat_map(|y| (32..64).map(move |x| (x, y)))
        .map(|(x, y)| mask[y * 64 + x])
        .sum();
    assert!((left - right).abs() < left * 0.05, "{left} {right}");
    // Undo the leaf, then the mirror line.
    undo(&mut harness);
    undo(&mut harness);
    assert!(!drawing(&harness).mirror().on);
}

#[test]
fn the_mirror_line_can_be_moved() {
    let mut harness = canvas_page();
    press_key(&mut harness, egui::Key::M);
    press_key(&mut harness, egui::Key::Q);
    let from = at(&mut harness, Vec2::new(0.5, 0.5));
    let to = at(&mut harness, Vec2::new(0.6, 0.5));
    drag(&mut harness, from, to, Modifiers::NONE);
    assert!((drawing(&harness).mirror().x - 0.6).abs() < 0.01);
    undo(&mut harness);
    assert_eq!(
        drawing(&harness).mirror().x,
        0.5,
        "one undo step for the drag"
    );
}

#[test]
fn b_adds_a_basic_shape_and_w_moves_it_as_one_undo_step() {
    let mut harness = canvas_page();
    press_key(&mut harness, egui::Key::B);
    assert!(shape(&mut harness).list.is_some());
    harness.get_by_label("Circle").click();
    harness.run();
    let d = drawing(&harness);
    assert_eq!(d.pieces().len(), 1);
    assert_eq!(d.pieces()[0].path.knots().len(), 4);
    let before = d.pieces()[0].path.middle();
    press_key(&mut harness, egui::Key::W);
    let from = at(&mut harness, before + Vec2::new(0.05, 0.05));
    let to = at(&mut harness, before + Vec2::new(0.15, 0.05));
    drag(&mut harness, from, to, Modifiers::NONE);
    let after = drawing(&harness).pieces()[0].placed().middle();
    assert!(
        after.distance(before + Vec2::new(0.1, 0.0)) < 0.01,
        "{after}"
    );
    undo(&mut harness);
    let back = drawing(&harness).pieces()[0].placed().middle();
    assert!(back.distance(before) < 1e-5);
    // R turns, S sizes (about the middle).
    press_key(&mut harness, egui::Key::S);
    let from = at(&mut harness, before + Vec2::new(0.1, 0.0));
    let to = at(&mut harness, before + Vec2::new(0.2, 0.0));
    drag(&mut harness, from, to, Modifiers::NONE);
    let d = drawing(&harness);
    let (lo, hi) = d.pieces()[0].placed().bounds().unwrap();
    assert!((hi.x - lo.x) > 0.6, "twice as wide: {}", hi.x - lo.x);
    // The polygon uses its sides.
    press_key(&mut harness, egui::Key::B);
    harness.get_by_label("Polygon").click();
    harness.run();
    assert_eq!(drawing(&harness).pieces()[1].path.knots().len(), 6);
}

#[test]
fn double_click_edits_points_and_tab_leaves_editing_before_the_canvas() {
    let mut harness = canvas_page();
    with_square(&mut harness);
    let middle = at(&mut harness, Vec2::splat(0.5));
    double_click(&mut harness, middle);
    let id = drawing(&harness).pieces()[0].id;
    assert_eq!(shape(&mut harness).editing, Some(id));
    // Drag the top left corner out.
    let corner = Vec2::splat(0.3);
    let from = at(&mut harness, corner);
    let to = at(&mut harness, Vec2::splat(0.2));
    drag(&mut harness, from, to, Modifiers::NONE);
    let k = drawing(&harness).pieces()[0].path.knots()[0];
    assert!(k.at.distance(Vec2::splat(0.2)) < 0.01, "{k:?}");
    // Double-click a side: a new point.
    let side = at(&mut harness, Vec2::new(0.7, 0.5));
    double_click(&mut harness, side);
    assert_eq!(drawing(&harness).pieces()[0].path.knots().len(), 5);
    // Delete removes the chosen point.
    press_key(&mut harness, egui::Key::Delete);
    assert_eq!(drawing(&harness).pieces()[0].path.knots().len(), 4);
    undo(&mut harness);
    undo(&mut harness);
    undo(&mut harness);
    let k = drawing(&harness).pieces()[0].path.knots()[0];
    assert!(k.at.distance(corner) < 1e-5);
    press_key(&mut harness, egui::Key::Tab);
    assert!(is_open(&mut harness));
    assert_eq!(shape(&mut harness).editing, None);
    press_key(&mut harness, egui::Key::Tab);
    assert!(!is_open(&mut harness));
}

#[test]
fn delete_removes_the_selected_piece_and_undo_brings_it_back() {
    let mut harness = canvas_page();
    with_square(&mut harness);
    press_key(&mut harness, egui::Key::Q);
    let middle = at(&mut harness, Vec2::splat(0.5));
    click(&mut harness, middle, Modifiers::NONE);
    assert_eq!(shape(&mut harness).selected.len(), 1);
    press_key(&mut harness, egui::Key::Delete);
    assert!(drawing(&harness).is_empty());
    undo(&mut harness);
    assert_eq!(drawing(&harness).pieces().len(), 1);
    // The pieces list chooses too.
    harness.get_by_label_contains("Square").click();
    harness.run();
    assert_eq!(shape(&mut harness).selected.len(), 1);
}

#[test]
fn the_reference_image_goes_under_the_canvas_and_never_changes_the_shape() {
    let mut harness = canvas_page();
    with_square(&mut harness);
    let key = drawing(&harness).key();
    harness.get_by_label("Reference image…").click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::ChooseReference { path: None })
    );
    let reference = shapes::Reference::new(2, 2, vec![200; 16], "leaf.png").unwrap();
    harness.state_mut().workbench.set_shape_reference(reference);
    harness.run();
    let d = drawing(&harness);
    assert_eq!(d.reference().unwrap().name, "leaf.png");
    assert_eq!(d.key(), key);
    assert!(note(&harness).contains("leaf.png"), "{}", note(&harness));
    harness.get_by_label("Remove image").click();
    harness.run();
    assert!(drawing(&harness).reference().is_none());
    undo(&mut harness);
    assert!(drawing(&harness).reference().is_some());
    // With no canvas open, it says where it goes.
    press_key(&mut harness, egui::Key::Tab);
    let again = shapes::Reference::new(1, 1, vec![0; 4], "x.png").unwrap();
    harness.state_mut().workbench.set_shape_reference(again);
    assert!(note(&harness).contains("Shape box"), "{}", note(&harness));
}

#[test]
fn undoing_the_shape_box_closes_its_canvas() {
    let mut harness = canvas_page();
    with_square(&mut harness);
    undo(&mut harness);
    assert!(
        is_open(&mut harness),
        "the square undone, the box still there"
    );
    undo(&mut harness);
    harness.run();
    assert!(!is_open(&mut harness), "the box is gone, so is its canvas");
}
