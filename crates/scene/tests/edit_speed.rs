//! The edit-speed check: how long the history, bevel and finding edits
//! again take on big shapes (the "fast edits" milestone, decision 0042).
//!
//! Not part of the normal test run. `tools/heavy/heavy-check.sh` runs it
//! with the rest of the heavy-mesh check, in release mode.

// A test that prints its table.
#![allow(clippy::unwrap_used, clippy::print_stdout)]

use mesh::{Anchors, EdgeKey, ElementKind, FaceId, Primitive, SphereParams, Topology, VertexId};
use scene::{BevelShape, Command, Document, Grouping, ObjectKind, Operation};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn polygons() -> usize {
    std::env::var("HEAVY_POLYGONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000_000)
}

fn sphere(sides: u32, rings: u32) -> mesh::Mesh {
    Primitive::Sphere(SphereParams {
        radius: 100.0,
        sides,
        rings,
    })
    .mesh()
}

fn row(what: &str, took: Duration, target_ms: u64) -> bool {
    let met = took <= Duration::from_millis(target_ms);
    println!(
        "{what:<58} {:>9.2}ms {:>8}ms  {}",
        took.as_secs_f64() * 1000.0,
        target_ms,
        if met { "ok" } else { "SLOW" }
    );
    met
}

fn timed<T>(work: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let out = work();
    (out, start.elapsed())
}

/// A document with `mesh` as its one object and `history` edits recorded,
/// and the object's id.
fn with_history(mesh: mesh::Mesh, history: &[Operation]) -> (Document, scene::ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(mesh))))
        .unwrap();
    let id = doc.selection()[0];
    for operation in history {
        doc.execute(Command::Record {
            id,
            operation: operation.clone(),
        })
        .unwrap();
    }
    (doc, id)
}

fn lift(point: u32, by: f32) -> Operation {
    Operation::MovePoints {
        points: vec![VertexId::from_raw(point)],
        offset: glam::Vec3::Z * by,
        anchors: None,
    }
}

/// The slowest of ten "drag frames": the last edit changes and the shape
/// is worked out again, as when dragging or changing the Last edit panel.
fn drag_frames(
    doc: &mut Document,
    id: scene::ObjectId,
    last: impl Fn(usize) -> Operation,
) -> Duration {
    let index = doc.scene().object(id).unwrap().stack.operation_count() - 1;
    let _ = doc.scene().evaluated(id);
    let mut slowest = Duration::ZERO;
    for frame in 0..10 {
        let ((), took) = timed(|| {
            doc.execute(Command::ReplaceOperation {
                id,
                layer: 0,
                index,
                operation: last(frame),
            })
            .unwrap();
            let _ = doc.scene().evaluated(id);
        });
        slowest = slowest.max(took);
    }
    slowest
}

#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn edit_speed_check() {
    let polygons = polygons();
    let mut all_met = true;
    println!();
    println!("Edit-speed check");

    // A 3,000-polygon prop with 200 edits in its history: extrudes and
    // moves, as modelling goes.
    let extrude = |face: u32, distance: f32| Operation::ExtrudePolygons {
        faces: vec![FaceId::from_raw(face)],
        distance,
        grouping: Grouping::Together,
        anchors: None,
    };
    let prop = sphere(64, 48);
    let history: Vec<Operation> = (0..200)
        .map(|i| {
            if i % 2 == 0 {
                extrude(i * 13 % 3000, 1.0)
            } else {
                lift(i % 500, 0.1)
            }
        })
        .collect();
    let (mut doc, id) = with_history(prop, &history);
    let slowest = drag_frames(&mut doc, id, |f| lift(7, 1.0 + f as f32));
    all_met &= row(
        "Drag after 200 edits, 3k-polygon prop (slowest frame)",
        slowest,
        16,
    );

    // The heavy shape with five extrudes in its history.
    let ball = mesh::samples::heavy_ball(polygons, 100.0);
    let history: Vec<Operation> = (0..5).map(|i| extrude(i * 1000, 1.0)).collect();
    let (mut doc, id) = with_history(ball.clone(), &history);
    let slowest = drag_frames(&mut doc, id, |f| extrude(4000, 1.0 + f as f32));
    all_met &= row(
        "Drag an extrude after 5 extrudes, heavy shape (slowest)",
        slowest,
        1_000,
    );

    // Bevelling every edge and every point of a 32,000-polygon sphere.
    let dense = sphere(256, 128);
    let edges: Vec<EdgeKey> = dense.topology().edges().collect();
    let shape = BevelShape {
        width: 0.5,
        segments: 2,
        profile: 1.0,
    };
    let (_, took) = timed(|| {
        let mut m = dense.clone();
        mesh::edit::bevel_edges(&mut m, &edges, shape)
    });
    all_met &= row("Bevel every edge of a 32k-polygon sphere", took, 5_000);
    let points: Vec<VertexId> = dense.vertices().map(|(v, _)| v).collect();
    let (_, took) = timed(|| {
        let mut m = dense.clone();
        mesh::edit::bevel_points(&mut m, &points, 0.5)
    });
    all_met &= row("Bevel every point of a 32k-polygon sphere", took, 5_000);

    // Finding 1,000 anchored points again on the heavy shape.
    let some: Vec<Vec<VertexId>> = ball
        .vertices()
        .step_by((ball.vertex_count() / 1000).max(1))
        .map(|(v, _)| vec![v])
        .collect();
    let anchors = Anchors::record(&ball, ElementKind::Points, &some).unwrap();
    let topology = Topology::new(&ball);
    let ((found, _), took) = timed(|| anchors.find(&ball, &topology));
    assert_eq!(found.len(), some.len());
    all_met &= row("Find 1,000 edited points again, heavy shape", took, 2_000);
    println!();
    assert!(all_met, "some steps missed their targets (see the table)");
}
