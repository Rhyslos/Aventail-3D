//! Tests for milestone 16's graph edits: bypassing a box, and reroute
//! dots (adding, moving, removing, following the wire and its boxes, and
//! surviving a rebuild from parts), with a fuzz run of random edits.

use super::*;
use crate::refusal::Refusal;
use glam::Vec2;

fn registry() -> &'static Registry {
    Registry::standard()
}

fn add(graph: &mut Graph, kind: &str) -> BoxId {
    let id = BoxId(graph.next_id());
    let at = Vec2::new(id.0 as f32 * 220.0, 0.0);
    let edit = GraphEdit::AddBox {
        kind: kind.into(),
        at,
    };
    graph.apply(registry(), &edit).unwrap();
    id
}

fn apply(graph: &mut Graph, edit: GraphEdit) -> Result<(), Refusal> {
    graph.apply(registry(), &edit)
}

/// Noise → Levels → Material from height, with two dots on the second
/// wire.
fn bent() -> (Graph, PlugRef) {
    let mut g = Graph::new("Bent");
    let noise = add(&mut g, "noise");
    let levels = add(&mut g, "levels");
    let mfh = add(&mut g, "material_from_height");
    let to = PlugRef::new(mfh, "height");
    for edit in [
        GraphEdit::Connect {
            from: PlugRef::new(noise, "height"),
            to: PlugRef::new(levels, "in"),
        },
        GraphEdit::Connect {
            from: PlugRef::new(levels, "out"),
            to: to.clone(),
        },
        GraphEdit::SetBends {
            to: to.clone(),
            points: vec![Vec2::new(500.0, 200.0), Vec2::new(600.0, 200.0)],
        },
    ] {
        apply(&mut g, edit).unwrap();
    }
    (g, to)
}

#[test]
fn a_box_is_bypassed_and_switched_back_on() {
    let (mut g, _) = bent();
    let levels = BoxId(2);
    apply(
        &mut g,
        GraphEdit::Bypass {
            node: levels,
            on: true,
        },
    )
    .unwrap();
    assert!(g.get(levels).unwrap().bypassed);
    // Kept through a rebuild from parts, and copied with the box.
    let back = Graph::from_parts(g.to_parts(), registry()).unwrap();
    assert_eq!(back, g);
    assert!(
        g.get(levels)
            .unwrap()
            .copy_edits(BoxId(9))
            .contains(&GraphEdit::Bypass {
                node: BoxId(9),
                on: true
            })
    );
    apply(
        &mut g,
        GraphEdit::Bypass {
            node: levels,
            on: false,
        },
    )
    .unwrap();
    assert!(!g.get(levels).unwrap().bypassed);
    assert!(g.get(levels).unwrap().copy_edits(BoxId(9)).is_empty());
}

#[test]
fn material_out_and_missing_boxes_cant_be_bypassed() {
    let mut g = Graph::new("End");
    let out = add(&mut g, "material_out");
    let why = apply(
        &mut g,
        GraphEdit::Bypass {
            node: out,
            on: true,
        },
    )
    .unwrap_err();
    assert!(
        why.reason.contains("where the graph ends"),
        "{}",
        why.reason
    );
    assert!(!g.get(out).unwrap().bypassed);
    let gone = apply(
        &mut g,
        GraphEdit::Bypass {
            node: BoxId(77),
            on: true,
        },
    );
    assert!(gone.is_err());
}

#[test]
fn dots_are_added_moved_and_removed() {
    let (mut g, to) = bent();
    assert_eq!(g.bends(&to).len(), 2);
    assert!(g.bends(&PlugRef::new(BoxId(2), "in")).is_empty());
    // Moved: the whole list again.
    let moved = vec![Vec2::new(520.0, 260.0), Vec2::new(600.0, 200.0)];
    apply(
        &mut g,
        GraphEdit::SetBends {
            to: to.clone(),
            points: moved.clone(),
        },
    )
    .unwrap();
    assert_eq!(g.bends(&to), moved.as_slice());
    // None left: the wire is straight and keeps nothing.
    apply(
        &mut g,
        GraphEdit::SetBends {
            to: to.clone(),
            points: Vec::new(),
        },
    )
    .unwrap();
    assert!(g.bends(&to).is_empty());
    assert_eq!(g.all_bends().count(), 0);
}

#[test]
fn odd_dots_are_refused_with_reasons() {
    let (mut g, to) = bent();
    let before = g.clone();
    let cases = [
        (
            PlugRef::new(BoxId(2), "nope"),
            vec![Vec2::ZERO],
            "isn't there",
        ),
        (
            to.clone(),
            vec![Vec2::ZERO; MAX_BENDS + 1],
            "reroute dots at most",
        ),
        (to.clone(), vec![Vec2::new(f32::NAN, 0.0)], "too far"),
        (to.clone(), vec![Vec2::new(MAX_PLACE * 2.0, 0.0)], "too far"),
    ];
    for (to, points, words) in cases {
        let why = apply(&mut g, GraphEdit::SetBends { to, points }).unwrap_err();
        assert!(why.reason.contains(words), "{}", why.reason);
        assert_eq!(g, before, "nothing changed");
    }
}

#[test]
fn dots_go_with_their_wire() {
    // Disconnected.
    let (mut g, to) = bent();
    apply(&mut g, GraphEdit::Disconnect(to.clone())).unwrap();
    assert!(g.bends(&to).is_empty());
    // A box at either end removed.
    for gone in [BoxId(2), BoxId(3)] {
        let (mut g, to) = bent();
        apply(&mut g, GraphEdit::RemoveBoxes(vec![gone])).unwrap();
        assert!(g.bends(&to).is_empty());
    }
    // Replaced by a wire from another output.
    let (mut g, to) = bent();
    apply(
        &mut g,
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(1), "height"),
            to: to.clone(),
        },
    )
    .unwrap();
    assert!(g.bends(&to).is_empty());
    // Connected again from the same output: the same wire, kept.
    let (mut g, to) = bent();
    apply(
        &mut g,
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(2), "out"),
            to: to.clone(),
        },
    )
    .unwrap();
    assert_eq!(g.bends(&to).len(), 2);
    // Dissolving Levels makes a new wire Noise → Material from height,
    // without the old wire's dots.
    let (mut g, to) = bent();
    apply(&mut g, GraphEdit::DissolveBox(BoxId(2))).unwrap();
    assert!(g.wire_into(&to).is_some());
    assert!(g.bends(&to).is_empty());
}

#[test]
fn dots_move_when_both_their_boxes_do() {
    let (mut g, to) = bent();
    let by = Vec2::new(40.0, -30.0);
    let before: Vec<Vec2> = g.bends(&to).to_vec();
    // One end only: the dots stay.
    apply(
        &mut g,
        GraphEdit::MoveBoxes {
            ids: vec![BoxId(3)],
            by,
        },
    )
    .unwrap();
    assert_eq!(g.bends(&to), before.as_slice());
    // Both ends: they come along.
    apply(
        &mut g,
        GraphEdit::MoveBoxes {
            ids: vec![BoxId(2), BoxId(3)],
            by,
        },
    )
    .unwrap();
    let after: Vec<Vec2> = before.iter().map(|p| *p + by).collect();
    assert_eq!(g.bends(&to), after.as_slice());
}

#[test]
fn dots_survive_a_rebuild_and_damaged_ones_are_refused() {
    let (g, to) = bent();
    let parts = g.to_parts();
    assert_eq!(parts.bends.len(), 1);
    assert_eq!(Graph::from_parts(parts.clone(), registry()).unwrap(), g);
    // Dots of a wire that isn't there are dropped.
    let mut loose = parts.clone();
    loose
        .bends
        .push((PlugRef::new(BoxId(1), "in"), vec![Vec2::ZERO]));
    loose
        .bends
        .push((PlugRef::new(BoxId(40), "in"), vec![Vec2::ZERO]));
    assert_eq!(Graph::from_parts(loose, registry()).unwrap(), g);
    // Too many, out of place, or two lists for one wire: damaged.
    let mut damaged = Vec::new();
    for points in [
        vec![Vec2::ZERO; MAX_BENDS + 1],
        vec![Vec2::INFINITY],
        vec![Vec2::new(0.0, -MAX_PLACE * 3.0)],
    ] {
        let mut p = parts.clone();
        p.bends[0].1 = points;
        damaged.push(p);
    }
    let mut twice = parts;
    twice.bends.push((to, vec![Vec2::ONE]));
    damaged.push(twice);
    for p in damaged {
        let why = Graph::from_parts(p, registry()).unwrap_err();
        assert!(why.contains("reroute dots"), "{why}");
    }
}

#[test]
fn random_edits_never_leave_dots_on_wires_that_are_gone() {
    let kinds = [
        "noise",
        "levels",
        "blend",
        "transform",
        "material_from_height",
    ];
    let mut state = 16u64;
    let mut next = move |n: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % n.max(1)
    };
    let mut g = Graph::new("Fuzz");
    let mut most = 0;
    for _ in 0..600 {
        let ids: Vec<BoxId> = g.boxes().map(|(id, _)| id).collect();
        let pick = |n: u64| ids.get(n as usize % ids.len().max(1)).copied();
        let edit = match [0, 0, 2, 2, 2, 3, 3, 4, 5, 6, 6, 7][next(12) as usize] {
            0 => GraphEdit::AddBox {
                kind: kinds[next(kinds.len() as u64) as usize].into(),
                at: Vec2::new(next(2000) as f32, next(2000) as f32),
            },
            2 => match (pick(next(99)), pick(next(99))) {
                (Some(a), Some(b)) => {
                    let info = |id: BoxId| g.info_of(registry(), id).ok();
                    let (Some(fa), Some(fb)) = (info(a), info(b)) else {
                        continue;
                    };
                    let (Some(out), Some(input)) = (
                        fa.outputs.get(next(4) as usize % fa.outputs.len().max(1)),
                        fb.inputs.get(next(4) as usize % fb.inputs.len().max(1)),
                    ) else {
                        continue;
                    };
                    GraphEdit::Connect {
                        from: PlugRef::new(a, out.key.to_string()),
                        to: PlugRef::new(b, input.key.to_string()),
                    }
                }
                _ => continue,
            },
            3 => match g.wires().get(next(99) as usize % g.wires().len().max(1)) {
                Some(w) => GraphEdit::SetBends {
                    to: w.to.clone(),
                    points: (0..next(5))
                        .map(|_| Vec2::new(next(3000) as f32, next(900) as f32))
                        .collect(),
                },
                None => continue,
            },
            4 => match pick(next(99)) {
                Some(id) => GraphEdit::RemoveBoxes(vec![id]),
                None => continue,
            },
            5 => match pick(next(99)) {
                Some(id) => GraphEdit::DissolveBox(id),
                None => continue,
            },
            6 => GraphEdit::MoveBoxes {
                ids: ids.iter().copied().filter(|_| next(2) == 0).collect(),
                by: Vec2::new(next(50) as f32 - 25.0, 3.0),
            },
            _ => match pick(next(99)) {
                Some(node) => GraphEdit::Bypass {
                    node,
                    on: next(2) == 0,
                },
                None => continue,
            },
        };
        let _ = g.apply(registry(), &edit);
        for (to, points) in g.all_bends() {
            assert!(g.wire_into(to).is_some(), "dots without a wire");
            assert!(!points.is_empty() && points.len() <= MAX_BENDS);
        }
        most = most.max(g.all_bends().count());
    }
    assert!(most > 2, "the run made dots on several wires");
    assert_eq!(Graph::from_parts(g.to_parts(), registry()).unwrap(), g);
}
