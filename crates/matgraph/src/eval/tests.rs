//! Tests for working graphs out: end to end, caching, seeds, cores, and
//! boxes that can't run.

use super::*;
use crate::graph::GraphEdit;
use crate::settings::SettingValue;
use glam::Vec2;

fn registry() -> &'static Registry {
    Registry::standard()
}

fn add(g: &mut Graph, kind: &str) -> BoxId {
    let id = BoxId(g.next_id());
    g.apply(
        registry(),
        &GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::ZERO,
        },
    )
    .unwrap();
    id
}

fn wire(g: &mut Graph, from: (BoxId, &str), to: (BoxId, &str)) {
    g.apply(
        registry(),
        &GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        },
    )
    .unwrap();
}

fn set(g: &mut Graph, node: BoxId, key: &str, value: SettingValue) {
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node,
            key: key.into(),
            value,
        },
    )
    .unwrap();
}

/// Tiles and noise blended, made into a material: the boxes' ids.
struct Stone {
    graph: Graph,
    tiles: BoxId,
    noise: BoxId,
    blend: BoxId,
    mat: BoxId,
    out: BoxId,
}

fn stone() -> Stone {
    let mut g = Graph::new("Stone");
    let (tiles, noise, blend) = (
        add(&mut g, "tiles"),
        add(&mut g, "noise"),
        add(&mut g, "blend"),
    );
    let (mat, out, fill) = (
        add(&mut g, "material_from_height"),
        add(&mut g, "material_out"),
        add(&mut g, "colour_fill"),
    );
    wire(&mut g, (tiles, "shape"), (blend, "bottom"));
    wire(&mut g, (noise, "height"), (blend, "top"));
    wire(&mut g, (tiles, "shape"), (blend, "where"));
    wire(&mut g, (blend, "out"), (mat, "height"));
    wire(&mut g, (fill, "colour"), (mat, "colour"));
    wire(&mut g, (mat, "material"), (out, "material"));
    set(&mut g, blend, "amount", SettingValue::Number(0.3));
    set(&mut g, tiles, "gap", SettingValue::Number(0.2));
    Stone {
        graph: g,
        tiles,
        noise,
        blend,
        mat,
        out,
    }
}

fn size(n: u32) -> Size {
    Size::new(n).unwrap()
}

#[test]
fn a_graph_makes_a_whole_material() {
    let s = stone();
    let maps = Evaluator::with_threads(2)
        .material(registry(), &s.graph, size(64))
        .unwrap();
    assert_eq!(maps.size(), 64);
    let h = maps.height.pixels();
    assert!(h.iter().all(|v| (0.0..=1.0).contains(v)));
    // The gaps are low, the tiles high with noise on them.
    assert!(h.contains(&0.0) && h.iter().any(|v| *v > 0.6));
    assert!(
        maps.normal
            .pixels()
            .iter()
            .all(|n| (glam::Vec3::from(*n).length() - 1.0).abs() < 1e-4)
    );
    assert_eq!(maps.colour.at(3, 3), [0.5; 3]);
}

#[test]
fn an_empty_graph_runs_nothing_and_says_it_makes_no_material() {
    let g = Graph::new("Empty");
    let mut e = Evaluator::with_threads(1);
    let run = e.run(registry(), &g, size(16), &[]);
    assert_eq!((run.made(), run.reused()), (0, 0));
    let refusal = e.material(registry(), &g, size(16)).unwrap_err();
    assert_eq!(
        refusal.fixes,
        vec![Fix::AddBox {
            kind: MATERIAL_OUT.into()
        }]
    );
}

#[test]
fn only_what_changed_is_worked_out_again() {
    let mut s = stone();
    let mut e = Evaluator::with_threads(2);
    let first = e.run(registry(), &s.graph, size(32), &[]);
    assert_eq!((first.made(), first.reused()), (6, 0));
    let again = e.run(registry(), &s.graph, size(32), &[]);
    assert_eq!((again.made(), again.reused()), (0, 6));
    // A setting near the end: only that box and the one after it.
    set(&mut s.graph, s.mat, "roughness", SettingValue::Number(0.2));
    let late = e.run(registry(), &s.graph, size(32), &[]);
    assert_eq!((late.made(), late.reused()), (2, 4));
    // A setting at the start: the noise and everything after it, not the
    // tiles or the colour.
    set(&mut s.graph, s.noise, "scale", SettingValue::Whole(9));
    let early = e.run(registry(), &s.graph, size(32), &[]);
    assert_eq!((early.made(), early.reused()), (4, 2));
    // Moving a box changes nothing worked out.
    s.graph
        .apply(
            registry(),
            &GraphEdit::MoveBoxes {
                ids: vec![s.tiles],
                by: Vec2::ONE,
            },
        )
        .unwrap();
    assert_eq!(e.run(registry(), &s.graph, size(32), &[]).made(), 0);
    // Another size is another result.
    assert_eq!(e.run(registry(), &s.graph, size(16), &[]).made(), 6);
}

#[test]
fn the_same_seed_gives_the_same_textures_and_another_seed_another() {
    let s = stone();
    let picture = |g: &Graph, id: BoxId| {
        Evaluator::with_threads(1)
            .run(registry(), g, size(32), &[])
            .picture(id)
            .cloned()
            .unwrap()
    };
    assert_eq!(picture(&s.graph, s.noise), picture(&s.graph, s.noise));
    let mut reseeded = s.graph.clone();
    reseeded
        .apply(registry(), &GraphEdit::SetSeed(1234))
        .unwrap();
    assert_ne!(picture(&s.graph, s.noise), picture(&reseeded, s.noise));
    // Tiles' shape doesn't use its seed (only its Random output does).
    assert_eq!(picture(&s.graph, s.tiles), picture(&reseeded, s.tiles));
    // Colour fill has no seed: the graph's seed doesn't change it, so its
    // result is found again.
    let mut e = Evaluator::with_threads(1);
    e.run(registry(), &s.graph, size(32), &[]);
    let run = e.run(registry(), &reseeded, size(32), &[]);
    assert!(run.outcome(s.tiles).is_some());
    assert_eq!(run.reused(), 1);
    // The box's own seed too.
    let mut own = s.graph.clone();
    set(&mut own, s.noise, "seed", SettingValue::Whole(3));
    assert_ne!(picture(&s.graph, s.noise), picture(&own, s.noise));
}

#[test]
fn any_number_of_cores_makes_the_same_textures() {
    let s = stone();
    let one = Evaluator::with_threads(1)
        .material(registry(), &s.graph, size(128))
        .unwrap();
    for threads in [2, 3, 16] {
        let many = Evaluator::with_threads(threads)
            .material(registry(), &s.graph, size(128))
            .unwrap();
        assert_eq!(*many, *one, "{threads} cores");
    }
}

#[test]
fn an_empty_needed_input_says_why_and_what_comes_after_waits() {
    let mut s = stone();
    s.graph
        .apply(
            registry(),
            &GraphEdit::Disconnect(PlugRef::new(s.blend, "top")),
        )
        .unwrap();
    let run = Evaluator::with_threads(1).run(registry(), &s.graph, size(16), &[]);
    let Some(Err(why)) = run.outcome(s.blend) else {
        panic!("Blend ran")
    };
    assert!(
        why.reason.contains("Blend") && why.reason.contains("Top"),
        "{}",
        why.reason
    );
    // Noise is in the graph, unplugged: offered (with Tiles' Random, a
    // height too), before boxes to add.
    assert!(why.fixes.contains(&Fix::ConnectInstead {
        from: PlugRef::new(s.noise, "height")
    }));
    assert!(matches!(
        why.fixes.first(),
        Some(Fix::ConnectInstead { .. })
    ));
    assert!(why.fixes.contains(&Fix::AddBox {
        kind: "noise".into()
    }));
    let Some(Err(waits)) = run.outcome(s.mat) else {
        panic!("it ran")
    };
    assert_eq!(waits.fixes, vec![Fix::LookAt(s.blend)]);
    assert!(matches!(run.outcome(s.out), Some(Err(_))));
    // The boxes before it, and unconnected ones, still ran.
    assert!(matches!(run.outcome(s.noise), Some(Ok(_))));
    assert!(matches!(run.outcome(s.tiles), Some(Ok(_))));
    assert_eq!(run.problems().count(), 3);
}

#[test]
fn a_box_from_a_newer_app_refuses_and_offers_to_remove_it() {
    let mut parts = Graph::new("Future").to_parts();
    parts.next_id = 2;
    parts.boxes.push((
        BoxId(1),
        crate::GraphBox {
            kind: "hologram".into(),
            at: Vec2::ZERO,
            settings: crate::Settings::new(),
            shown: Default::default(),
            in_game: Default::default(),
            drawing: None,
            bypassed: false,
        },
    ));
    let g = Graph::from_parts(parts, registry()).unwrap();
    let run = Evaluator::with_threads(1).run(registry(), &g, size(16), &[]);
    let Some(Err(why)) = run.outcome(BoxId(1)) else {
        panic!("it ran")
    };
    assert!(why.reason.contains("newer"), "{}", why.reason);
    assert_eq!(why.fixes, vec![Fix::RemoveBox(BoxId(1))]);
}

#[test]
fn only_the_boxes_asked_for_and_what_feeds_them_run() {
    let s = stone();
    let run = Evaluator::with_threads(1).run(registry(), &s.graph, size(16), &[s.blend]);
    assert_eq!(run.made(), 3);
    assert!(run.outcome(s.mat).is_none());
    assert!(
        run.picture(s.blend)
            .is_some_and(|v| v.kind() == crate::ValueKind::Height)
    );
    // Material out's picture is its material.
    let all = Evaluator::with_threads(1).run(registry(), &s.graph, size(16), &[s.out]);
    assert!(all.picture(s.out).is_some_and(|v| v.material().is_some()));
}

#[test]
fn results_not_on_screen_are_forgotten_past_the_budget() {
    let s = stone();
    let mut e = Evaluator::with_threads(1);
    e.run(registry(), &s.graph, size(32), &[]);
    let at_32 = e.cache().memory();
    assert!(at_32 > 0 && e.cache().len() == 6);
    e.cache_mut().set_budget(0);
    // What the last run used is kept, even over the budget.
    assert_eq!(e.cache().len(), 6);
    e.run(registry(), &s.graph, size(16), &[]);
    assert_eq!(e.cache().len(), 6, "the 32-pixel results are gone");
    assert!(e.cache().memory() < at_32);
    e.cache_mut().clear();
    assert!(e.cache().is_empty());
}

/// How long the stone graph takes at each size (not part of the normal
/// run: `cargo test --release -p matgraph graph_speed -- --ignored
/// --nocapture`).
#[test]
#[ignore = "a timing, run by hand"]
fn graph_speed() {
    let mut s = stone();
    let mut metal = 0.0;
    for n in [256, 1024, 2048] {
        for threads in [1, crate::parallel::cores()] {
            let mut e = Evaluator::with_threads(threads);
            let start = std::time::Instant::now();
            e.run(registry(), &s.graph, size(n), &[]);
            let whole = start.elapsed();
            metal += 0.1;
            set(&mut s.graph, s.mat, "metal", SettingValue::Number(metal));
            let start = std::time::Instant::now();
            e.run(registry(), &s.graph, size(n), &[]);
            println!(
                "{n:>5} px, {threads} cores: whole graph {:.1} ms, last box changed {:.1} ms",
                whole.as_secs_f64() * 1000.0,
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}
