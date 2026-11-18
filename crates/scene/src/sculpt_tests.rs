//! Tests for Sculpt layers (decision 0304): the command, where sculpting
//! and later edits go, undo, a changed shape under a sculpt, the cache
//! taking a stroke's shape, and the undo memory a stroke keeps.

use crate::stack::Rebuilt;
use crate::*;
use glam::Vec3;
use mesh::Primitive;
use sculpt::{BrushKind, Dab, Falloff, LiveSculpt};
use std::sync::Arc;

/// A document with one standard sphere (selected), and its id.
fn with_a_sphere() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

/// One Draw stroke on `id` at its top, as the interface makes it: on a
/// live sculpt of its sculpt target. Returns the live sculpt.
fn stroke(doc: &mut Document, id: ObjectId, along: f32) -> LiveSculpt {
    let target = doc.scene().sculpt_target(id).unwrap();
    let mut live = LiveSculpt::new(Arc::clone(&target.input), target.sculpt.as_ref());
    let hit = live
        .hit(Vec3::new(along, 0.0, 500.0), Vec3::NEG_Z)
        .expect("the sphere's top");
    for i in 0..5 {
        live.dab(&Dab {
            kind: BrushKind::Draw,
            centre: hit.point + Vec3::X * i as f32,
            radius: 20.0,
            strength: 1.0,
            falloff: Falloff::Smooth,
            invert: false,
        });
    }
    let sculpt = live.finish().unwrap();
    doc.execute(Command::Sculpt { id, sculpt }).unwrap();
    live
}

fn layers(doc: &Document, id: ObjectId) -> Vec<(String, bool)> {
    doc.scene()
        .object(id)
        .unwrap()
        .stack
        .layers()
        .iter()
        .map(|l| (l.name.clone(), l.is_sculpt()))
        .collect()
}

fn shape(doc: &Document, id: ObjectId) -> Mesh {
    (*doc.scene().evaluated(id).unwrap().mesh).clone()
}

#[test]
fn a_stroke_adds_a_sculpt_layer_and_the_next_replaces_its_sculpt() {
    let (mut doc, id) = with_a_sphere();
    let plain = shape(&doc, id);
    let live = stroke(&mut doc, id, 0.0);
    assert_eq!(layers(&doc, id), vec![("Sculpt".to_string(), true)]);
    assert_eq!(&shape(&doc, id), live.mesh());
    assert_ne!(shape(&doc, id), plain);
    let after_one = shape(&doc, id);
    stroke(&mut doc, id, 10.0);
    assert_eq!(layers(&doc, id).len(), 1, "the same layer");
    let after_two = shape(&doc, id);
    // Each stroke one undo step, exactly back and forth.
    doc.undo();
    assert_eq!(shape(&doc, id), after_one);
    doc.undo();
    assert_eq!(shape(&doc, id), plain);
    assert!(layers(&doc, id).is_empty());
    doc.redo();
    doc.redo();
    assert_eq!(shape(&doc, id), after_two);
}

#[test]
fn edits_after_sculpting_go_into_a_new_layer_and_so_does_sculpting_after_them() {
    let (mut doc, id) = with_a_sphere();
    stroke(&mut doc, id, 0.0);
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: vec![VertexId::from_raw(0)],
            offset: Vec3::Z,
            anchors: None,
            keep_texture: false,
        },
    })
    .unwrap();
    assert_eq!(
        layers(&doc, id),
        vec![("Sculpt".to_string(), true), ("Edits".to_string(), false)]
    );
    stroke(&mut doc, id, 0.0);
    assert_eq!(
        layers(&doc, id),
        vec![
            ("Sculpt".to_string(), true),
            ("Edits".to_string(), false),
            ("Sculpt 2".to_string(), true)
        ]
    );
    // The sculpt target is the top layer, working on the edits under it.
    let target = doc.scene().sculpt_target(id).unwrap();
    assert_eq!(target.layer, Some(2));
    // Merging a Sculpt layer is refused, in plain words.
    let merged = doc.execute(Command::ChangeLayer {
        id,
        index: 1,
        change: LayerChange::MergeDown,
    });
    assert!(merged.unwrap_err().to_string().contains("Sculpt layer"));
}

#[test]
fn a_switched_off_sculpt_layer_takes_no_more_sculpting() {
    let (mut doc, id) = with_a_sphere();
    stroke(&mut doc, id, 0.0);
    doc.execute(Command::ChangeLayer {
        id,
        index: 0,
        change: LayerChange::SetEnabled(false),
    })
    .unwrap();
    assert_eq!(doc.scene().sculpt_target(id).unwrap().layer, None);
    stroke(&mut doc, id, 0.0);
    assert_eq!(layers(&doc, id).len(), 2);
}

#[test]
fn groups_instances_and_broken_sculpts_are_refused() {
    let (mut doc, id) = with_a_sphere();
    let sculpt = sculpt::Sculpt::new(&shape(&doc, id));
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    assert!(
        doc.execute(Command::Sculpt {
            id: group,
            sculpt: sculpt.clone()
        })
        .is_err()
    );
    assert!(doc.scene().sculpt_target(group).is_none());
    doc.execute(Command::Duplicate {
        ids: vec![id],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert!(
        doc.execute(Command::Sculpt {
            id: instance,
            sculpt: sculpt.clone()
        })
        .is_err()
    );
    assert!(doc.scene().sculpt_target(instance).is_none());
    // A sculpt can't hold moves that aren't numbers.
    let mut moves = sculpt.moves().clone();
    moves.set_chunk(0, vec![Vec3::NAN; sculpt.len()]);
    assert!(sculpt.with_moves(moves).is_none());
    // A sculpt made for another shape is fitted, not refused.
    let other = sculpt::Sculpt::new(&PrimitiveKind::Cube.standard().mesh());
    assert!(doc.execute(Command::Sculpt { id, sculpt: other }).is_ok());
}

#[test]
fn a_changed_shape_under_the_sculpt_is_fitted_with_a_warning_and_undo_brings_it_back() {
    let (mut doc, id) = with_a_sphere();
    stroke(&mut doc, id, 0.0);
    let sculpted = shape(&doc, id);
    let top = |m: &Mesh| {
        m.vertices()
            .map(|(_, p)| p.z)
            .fold(f32::NEG_INFINITY, f32::max)
    };
    // A bigger sphere (same points): the detail rides along, no warning.
    let mut bigger = PrimitiveKind::Sphere.standard();
    if let Primitive::Sphere(p) = &mut bigger {
        p.radius = 60.0;
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: bigger,
    })
    .unwrap();
    let evaluated = doc.scene().evaluated(id).unwrap();
    assert!(evaluated.warnings.is_empty());
    assert!(top(&evaluated.mesh) > top(&sculpted));
    // More sides: renumbered points, fitted by position, with a warning.
    let mut finer = PrimitiveKind::Sphere.standard();
    if let Primitive::Sphere(p) = &mut finer {
        p.sides = 48;
        p.rings = 24;
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: finer,
    })
    .unwrap();
    let evaluated = doc.scene().evaluated(id).unwrap();
    assert!(evaluated.warnings.iter().any(|w| w.found_again));
    assert!(
        top(&evaluated.mesh) > 50.0 + 1.0,
        "the bump is kept: {}",
        top(&evaluated.mesh)
    );
    doc.undo();
    doc.undo();
    assert_eq!(shape(&doc, id), sculpted);
}

#[test]
fn the_cache_takes_a_strokes_shape_without_working_it_out_again() {
    let (mut doc, id) = with_a_sphere();
    stroke(&mut doc, id, 0.0);
    let _ = doc.scene().evaluated(id);
    let mut live = stroke(&mut doc, id, 10.0);
    let shown = live.shape();
    assert!(doc.scene().prime_sculpt(id, Arc::clone(&shown)));
    let evaluated = doc.scene().evaluated(id).unwrap();
    assert!(Arc::ptr_eq(&evaluated.mesh, &shown));
    assert_eq!(doc.scene().last_rebuilt(id), Some(Rebuilt::Nothing));
    // And it is what working it out gives.
    let fresh = doc.scene().clone();
    assert_eq!(*fresh.evaluated(id).unwrap().mesh, *shown);
}

#[test]
fn a_stroke_keeps_only_the_chunks_it_touched_in_the_undo_history() {
    let mut doc = Document::new();
    let ball = mesh::samples::heavy_ball(200_000, 100.0);
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(ball))))
        .unwrap();
    let id = doc.selection()[0];
    let before = doc.history_memory();
    let target = doc.scene().sculpt_target(id).unwrap();
    let mut live = LiveSculpt::new(Arc::clone(&target.input), None);
    let hit = live.hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y).unwrap();
    live.dab(&Dab {
        kind: BrushKind::Draw,
        centre: hit.point,
        radius: 5.0,
        strength: 1.0,
        falloff: Falloff::Smooth,
        invert: false,
    });
    let first = live.finish().unwrap();
    // The first stroke keeps the whole layer (where the points were).
    doc.execute(Command::Sculpt { id, sculpt: first }).unwrap();
    let after_first = doc.history_memory();
    live.dab(&Dab {
        kind: BrushKind::Draw,
        centre: hit.point,
        radius: 5.0,
        strength: 1.0,
        falloff: Falloff::Smooth,
        invert: false,
    });
    let second = live.finish().unwrap();
    doc.execute(Command::Sculpt { id, sculpt: second }).unwrap();
    let grown = doc.history_memory() - after_first;
    let whole = after_first - before;
    assert!(grown > 0);
    // The chunks the stroke touched, before and after: never the whole.
    let chunk = sculpt::CHUNK * std::mem::size_of::<Vec3>();
    assert!(grown <= 2 * 2 * chunk + 4096, "{grown}");
    assert!(grown * 3 < whole, "{grown} of {whole}");
}

#[test]
fn after_an_undo_the_shape_coming_in_is_found_without_working_the_sculpt_out() {
    let (mut doc, id) = with_a_sphere();
    stroke(&mut doc, id, 0.0);
    stroke(&mut doc, id, 10.0);
    let _ = doc.scene().evaluated(id);
    doc.undo();
    let before = doc.scene().last_rebuilt(id);
    let target = doc.scene().sculpt_target(id).unwrap();
    assert_eq!(doc.scene().last_rebuilt(id), before, "nothing worked out");
    assert_eq!(target.layer, Some(0));
    // The live sculpt follows the undo, and the cache takes its shape.
    let mut live = LiveSculpt::new(Arc::clone(&target.input), target.sculpt.as_ref());
    assert!(doc.scene().prime_sculpt(id, live.shape()));
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh, live.shape());
    assert_eq!(doc.scene().last_rebuilt(id), Some(Rebuilt::Nothing));
}

/// A long session (decision 0313): a thousand strokes all over a shape
/// keep the undo history within its memory budget, the oldest steps
/// forgotten as it fills, and the last strokes still undo and redo to
/// exactly the shapes they made.
#[test]
fn a_thousand_strokes_stay_within_the_undo_budget() {
    const BUDGET: usize = 24_000_000;
    let mut doc = Document::new();
    doc.set_history_limit(HistoryLimit::Bytes(BUDGET));
    let ball = mesh::samples::heavy_ball(60_000, 100.0);
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(ball))))
        .unwrap();
    let id = doc.selection()[0];
    let target = doc.scene().sculpt_target(id).unwrap();
    // One live sculpt for the whole session, as the interface keeps it.
    let mut live = LiveSculpt::new(Arc::clone(&target.input), None);
    let mut most = 0;
    let mut shapes = Vec::new();
    for n in 0..1000u32 {
        // Strokes walk round the ball, a few dabs each.
        let turn = n as f32 * 0.37;
        let from = Vec3::new(turn.cos() * 500.0, turn.sin() * 500.0, 60.0 + (n % 7) as f32 * 12.0);
        let Some(hit) = live.hit(from, Vec3::new(0.0, 0.0, 100.0) - from) else {
            continue;
        };
        for i in 0..3 {
            live.dab(&Dab {
                kind: if n % 5 == 4 { BrushKind::Smooth } else { BrushKind::Draw },
                centre: hit.point + Vec3::Z * i as f32,
                radius: 12.0,
                strength: 0.5,
                falloff: Falloff::Smooth,
                invert: n % 2 == 1,
            });
        }
        let Some(sculpt) = live.finish() else {
            continue;
        };
        doc.execute(Command::Sculpt { id, sculpt }).unwrap();
        most = most.max(doc.history_memory());
        assert!(doc.history_memory() <= BUDGET, "stroke {n}: {}", doc.history_memory());
        if n >= 995 {
            shapes.push(shape(&doc, id));
        }
    }
    assert!(most > BUDGET / 2, "the budget was reached: {most}");
    assert!(doc.forgotten_steps() > 0, "old steps were forgotten");
    // The last strokes undo to exactly the shapes before them, and redo.
    let last = shapes.pop().unwrap();
    for expected in shapes.iter().rev() {
        assert!(doc.undo().is_some());
        assert_eq!(&shape(&doc, id), expected);
    }
    for _ in 0..shapes.len() {
        assert!(doc.redo().is_some());
    }
    assert_eq!(shape(&doc, id), last);
}
