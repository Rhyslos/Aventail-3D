//! Tests for precise placement in the UV Editor (decision 0237).

use super::*;

fn snapping(settings: UvSnap) -> UvSnapping {
    UvSnapping {
        settings,
        pixels: 1024,
        reach: 0.02,
        moving: vec![Vec2::new(0.1, 0.1), Vec2::new(0.2, 0.1)],
        points: vec![Vec2::new(0.505, 0.3)],
        segments: vec![[Vec2::new(0.0, 0.6), Vec2::new(1.0, 0.6)]],
        rows: vec![0.25, 0.5],
    }
}

#[test]
fn the_standard_grid_lands_the_middle_on_tenths() {
    let s = snapping(UvSnap::default());
    let by = s.snap_move(Vec2::new(0.15, 0.1), Vec2::new(0.23, 0.04), None);
    let to = Vec2::new(0.15, 0.1) + by;
    assert!((to - Vec2::new(0.4, 0.1)).length() < 1e-5, "{to}");
    // A grid of your own.
    let fine = snapping(UvSnap {
        grid_size: 0.25,
        ..UvSnap::default()
    });
    let to = Vec2::new(0.15, 0.1) + fine.snap_move(Vec2::new(0.15, 0.1), Vec2::new(0.2, 0.0), None);
    assert!((to.x - 0.25).abs() < 1e-5);
    // Pixels, with the grid off.
    let pixels = snapping(UvSnap {
        grid: false,
        pixels: true,
        ..UvSnap::default()
    });
    let to =
        Vec2::new(0.1, 0.1) + pixels.snap_move(Vec2::new(0.1, 0.1), Vec2::new(0.00031, 0.0), None);
    let px = to.x * 1024.0;
    assert!((px - px.round()).abs() < 1e-3, "{px}");
    // Nothing chosen: the move as dragged.
    let none = snapping(UvSnap {
        grid: false,
        ..UvSnap::default()
    });
    assert_eq!(
        none.snap_move(Vec2::ZERO, Vec2::new(0.123, 0.0), None),
        Vec2::new(0.123, 0.0)
    );
}

#[test]
fn corners_edges_and_trim_rows_win_when_near() {
    let corners = snapping(UvSnap {
        corners: true,
        ..UvSnap::default()
    });
    // The second moving corner (0.2, 0.1) moved by (0.3, 0.19) is
    // (0.5, 0.29): the corner (0.505, 0.3) is near.
    let by = corners.snap_move(Vec2::ZERO, Vec2::new(0.3, 0.19), None);
    assert!(((Vec2::new(0.2, 0.1) + by) - Vec2::new(0.505, 0.3)).length() < 1e-5);
    let edges = snapping(UvSnap {
        edges: true,
        grid: false,
        ..UvSnap::default()
    });
    let by = edges.snap_move(Vec2::ZERO, Vec2::new(0.0, 0.49), None);
    assert!(((Vec2::new(0.1, 0.1) + by).y - 0.6).abs() < 1e-5, "{by}");
    let trim = snapping(UvSnap {
        trim: true,
        grid: false,
        ..UvSnap::default()
    });
    let by = trim.snap_move(Vec2::ZERO, Vec2::new(0.0, 0.14), None);
    assert!(((0.1 + by.y) - 0.25).abs() < 1e-5, "{by}");
    // Far away, the grid takes over.
    let by = corners.snap_move(Vec2::new(0.15, 0.1), Vec2::new(0.0, 0.7), None);
    let to = Vec2::new(0.15, 0.1) + by;
    assert!((to.y - 0.8).abs() < 1e-5, "{to}");
}

#[test]
fn a_move_kept_to_one_way_stays_that_way() {
    let s = snapping(UvSnap {
        corners: true,
        ..UvSnap::default()
    });
    let by = s.snap_move(Vec2::ZERO, Vec2::new(0.3, 0.19), Some(0));
    assert_eq!(by.y, 0.0);
    let by = s.snap_move(Vec2::ZERO, Vec2::new(0.3, 0.19), Some(1));
    assert_eq!(by.x, 0.0);
}

#[test]
fn a_broken_grid_size_is_kept_in_range() {
    let s = snapping(UvSnap {
        grid_size: f32::NAN,
        ..UvSnap::default()
    });
    assert!(
        s.snap_move(Vec2::ZERO, Vec2::new(0.3, 0.0), None)
            .is_finite()
    );
    let tiny = snapping(UvSnap {
        grid_size: 0.0,
        ..UvSnap::default()
    });
    assert!(
        tiny.snap_move(Vec2::ZERO, Vec2::new(0.3, 0.0), None)
            .is_finite()
    );
}

#[test]
fn every_pivot_sits_where_it_says() {
    let (lo, hi) = (Vec2::new(0.2, 0.4), Vec2::new(0.6, 0.8));
    assert_eq!(UvPivot::Middle.spot(lo, hi, None), Vec2::new(0.4, 0.6));
    assert_eq!(UvPivot::EachPiece.spot(lo, hi, None), Vec2::new(0.4, 0.6));
    assert_eq!(UvPivot::BoxCorner(0).spot(lo, hi, None), lo);
    assert_eq!(
        UvPivot::BoxCorner(1).spot(lo, hi, None),
        Vec2::new(0.6, 0.4)
    );
    assert_eq!(UvPivot::BoxCorner(2).spot(lo, hi, None), hi);
    assert_eq!(
        UvPivot::BoxCorner(3).spot(lo, hi, None),
        Vec2::new(0.2, 0.8)
    );
    assert_eq!(UvPivot::TextureMiddle.spot(lo, hi, None), Vec2::splat(0.5));
    assert_eq!(UvPivot::Own.spot(lo, hi, Some(Vec2::ONE)), Vec2::ONE);
    // Without a point placed yet, the middle.
    assert_eq!(UvPivot::Own.spot(lo, hi, None), Vec2::new(0.4, 0.6));
    for p in UvPivot::ALL {
        assert!(!p.name().is_empty());
    }
}

/// A document with a plane two polygons across, cut into two pieces in
/// the texture, every polygon selected.
fn two_pieces() -> (Document, scene::ObjectId) {
    let mut doc = Document::new();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 1],
            ..Default::default()
        }),
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let t = mesh.topology();
    let middle = t.edges().find(|e| t.edge_faces(*e).count() == 2).unwrap();
    let moved = uv::pieces::cut(&mesh, &[middle], 0.05);
    doc.execute(scene::Command::Record {
        id,
        operation: scene::Operation::SetUvs { corners: moved },
    })
    .unwrap();
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    doc.select_elements(
        [(
            id,
            scene::ElementSelection::of(faces.into_iter().map(scene::Element::Polygon)),
        )],
        "Select",
        None,
    );
    (doc, id)
}

fn piece_middles(doc: &Document, id: scene::ObjectId) -> Vec<Vec2> {
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    mesh::uvs::islands(&mesh)
        .iter()
        .map(|p| {
            let (lo, hi) = mesh::uvs::uv_bounds(&mesh, p, &[]).unwrap();
            (lo + hi) / 2.0
        })
        .collect()
}

#[test]
fn each_piece_turns_about_its_own_middle() {
    let (mut doc, id) = two_pieces();
    let (targets, _) = crate::uv_editor::gizmo::uv_targets(&doc, crate::palette::Mode::Uv);
    let pieces = per_piece(&doc, &targets);
    assert_eq!(pieces.len(), 2, "two pieces");
    let before = piece_middles(&doc, id);
    let start = egui::pos2(300.0, 300.0) + egui::vec2(45.0, 45.0);
    let mut drag = crate::uv_editor::gizmo::GizmoDrag::new(
        crate::uv_editor::gizmo::UvHandle::Turn,
        Vec2::splat(0.5),
        targets,
        start,
        77,
    )
    .with_pieces(pieces);
    drag.follow(
        egui::pos2(300.0, 300.0) + egui::vec2(-45.0, 45.0),
        egui::pos2(300.0, 300.0),
        300.0,
        false,
    );
    assert!(drag.show(&mut doc, None));
    let after = piece_middles(&doc, id);
    for (a, b) in before.iter().zip(&after) {
        assert!(a.distance(*b) < 1e-4, "{a} {b}");
    }
}

#[test]
fn gathering_splits_what_moves_from_what_stays() {
    let (mut doc, id) = two_pieces();
    // Select only the first polygon.
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    let first = mesh.faces().next().unwrap().0;
    doc.select_elements(
        [(
            id,
            scene::ElementSelection::of([scene::Element::Polygon(first)]),
        )],
        "Select",
        None,
    );
    let (targets, _) = crate::uv_editor::gizmo::uv_targets(&doc, crate::palette::Mode::Uv);
    let (moving, still, segments) = gather(&doc, &targets);
    assert_eq!(moving.len(), 4);
    assert_eq!(still.len(), 4);
    assert_eq!(segments.len(), 4);
}
