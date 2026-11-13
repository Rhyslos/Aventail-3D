//! Edges, varied copies and veins on the shape canvas (decision 0275),
//! used through the real interface: an edge picked in the pieces list
//! changing linked copies and the canvas's outline, "Each copy differs"
//! and ↻ New look, a line's vein rows, and undo for each.

use super::material_mode_tests::undo;
use super::material_shape_tests::{
    SHAPE, at, canvas_page, click, drawing, press_key, shape, with_square,
};
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{GraphEdit, GraphId};
use shapes::{Change, Drawing, EdgeKind, Knot, Path, PieceKind};
use std::sync::Arc;

type H = super::material_mode_tests::H;

/// The square and a linked copy of it to the right, the square selected.
fn square_and_linked_copy(harness: &mut H) {
    with_square(harness);
    let d = drawing(harness);
    let first = d.pieces()[0].id;
    let (d, _) = d
        .with_pasted(
            &d.copied(&[first]),
            true,
            &Change::moved(Vec2::new(0.3, 0.0)),
        )
        .unwrap();
    set(harness, d);
    press_key(harness, egui::Key::Q);
    let middle = at(harness, Vec2::splat(0.5));
    click(harness, middle, Modifiers::NONE);
    assert_eq!(shape(harness).selected, [first].into());
}

fn set(harness: &mut H, d: Drawing) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit: GraphEdit::SetDrawing {
                node: SHAPE,
                drawing: Arc::new(d),
            },
        })
        .unwrap();
    harness.run();
}

#[test]
fn picking_an_edge_roughens_every_linked_copy_on_the_canvas() {
    let mut harness = canvas_page();
    square_and_linked_copy(&mut harness);
    let before = drawing(&harness);
    let smooth_points = shape(&mut harness).outlines_now(&before)[0][0]
        .knots()
        .len();
    harness.get_by_label("Serrated").click();
    harness.run();
    let d = drawing(&harness);
    assert!(d.pieces().iter().all(|p| p.edge.kind == EdgeKind::Serrated));
    // The canvas shows the teeth at once.
    let outlines = shape(&mut harness).outlines_now(&d);
    assert!(outlines[0][0].knots().len() > smooth_points * 4);
    assert!(outlines[1][0].knots().len() > smooth_points * 4);
    // Its sliders and the seed's ↻ show; ↻ rolls new teeth.
    assert!(harness.query_by_label("Amount").is_some());
    assert!(harness.query_by_label("Tooth size").is_some());
    let seed = d.pieces()[0].edge.seed;
    harness.get_by_label("↻").click();
    harness.run();
    let rolled = drawing(&harness);
    assert_ne!(rolled.pieces()[0].edge.seed, seed);
    assert_eq!(rolled.pieces()[1].edge.seed, rolled.pieces()[0].edge.seed);
    undo(&mut harness);
    assert_eq!(drawing(&harness).pieces()[0].edge.seed, seed);
    undo(&mut harness);
    assert!(
        drawing(&harness)
            .pieces()
            .iter()
            .all(|p| p.edge.kind == EdgeKind::Smooth)
    );
}

#[test]
fn each_copy_differs_makes_linked_copies_come_out_their_own_way() {
    let mut harness = canvas_page();
    square_and_linked_copy(&mut harness);
    harness.get_by_label("Jagged").click();
    harness.run();
    harness.get_by_label("Each copy differs").click();
    harness.run();
    let d = drawing(&harness);
    assert!(d.pieces().iter().all(|p| p.vary.on), "shared by the link");
    assert_eq!(d.pieces()[0].path, d.pieces()[1].path, "still one shape");
    let drawn: Vec<Path> = d
        .pieces()
        .iter()
        .map(|p| d.drawn_paths(p)[0].clone())
        .collect();
    let moved_back = drawn[1].changed(&Change::moved(Vec2::new(-0.3, 0.0)));
    assert_ne!(moved_back.knots().len(), 0);
    assert!(
        moved_back
            .knots()
            .iter()
            .zip(drawn[0].knots())
            .any(|(a, b)| a.at.distance(b.at) > 1e-3),
        "the copy differs"
    );
    // ↻ New look gives the selected piece a new seed.
    let seed = d.pieces()[0].seed;
    harness.get_by_label("↻ New look").click();
    harness.run();
    let d = drawing(&harness);
    assert_ne!(d.pieces()[0].seed, seed);
    undo(&mut harness);
    assert_eq!(drawing(&harness).pieces()[0].seed, seed);
    undo(&mut harness);
    assert!(drawing(&harness).pieces().iter().all(|p| !p.vary.on));
}

#[test]
fn a_chosen_line_shows_its_vein_and_the_box_grooves_along_it() {
    let mut harness = canvas_page();
    let vein = Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.3)),
            Knot::corner(Vec2::new(0.5, 0.7)),
        ],
        false,
    );
    let d = Drawing::new()
        .with_piece(
            "Leaf",
            shapes::Basic::Circle.path(Vec2::splat(0.5), 0.3),
            PieceKind::Add,
        )
        .unwrap()
        .with_piece("Midrib", vein, PieceKind::Line)
        .unwrap();
    set(&mut harness, d);
    harness.get_by_label_contains("Midrib").click();
    harness.run();
    assert!(harness.query_by_label("Vein").is_some());
    assert!(harness.query_by_label("Depth").is_some());
    assert!(harness.query_by_label("Shade").is_some());
    assert!(
        harness.query_by_label("Serrated").is_none(),
        "lines have no edge"
    );
    // The box's Height has the groove.
    let d = drawing(&harness);
    let look = matgraph::boxes::drawn_shape::Look {
        puff: 0.0,
        top: 1.0,
        outline: 1.0,
    };
    let made = matgraph::boxes::drawn_shape::drawn(&d, matgraph::Size::nearest(256), look, 1);
    assert!(made.height.at(128, 128) < 0.8);
    assert!((made.height.at(80, 128) - 1.0).abs() < 1e-5);
}
