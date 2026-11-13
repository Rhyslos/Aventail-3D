//! Tests for working out graphs that move over time (milestone 15): the
//! same moment always gives the same pictures, boxes that don't move are
//! kept in the cache as the time goes on, the graph's moment is what is
//! baked, and an own box holding a moving box moves too.

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
            at: Vec2::new(id.0 as f32 * 200.0, 0.0),
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

/// Noise → Scroll → Material from height → Material out, and Clouds
/// beside it: the boxes' ids.
struct Lava {
    graph: Graph,
    noise: BoxId,
    scroll: BoxId,
    mat: BoxId,
    clouds: BoxId,
}

fn lava() -> Lava {
    let mut g = Graph::new("Lava");
    let noise = add(&mut g, "noise");
    let scroll = add(&mut g, "scroll");
    let mat = add(&mut g, "material_from_height");
    let out = add(&mut g, "material_out");
    let clouds = add(&mut g, "clouds");
    wire(&mut g, (noise, "height"), (scroll, "in"));
    wire(&mut g, (scroll, "out"), (mat, "height"));
    wire(&mut g, (mat, "material"), (out, "material"));
    Lava {
        graph: g,
        noise,
        scroll,
        mat,
        clouds,
    }
}

fn size() -> Size {
    Size::new(32).unwrap()
}

#[test]
fn the_same_moment_always_gives_the_same_material() {
    let l = lava();
    let mut a = Evaluator::with_threads(1);
    let mut b = Evaluator::with_threads(3);
    for t in [0.0, 0.25, 1.7, 3.99] {
        a.set_time(Some(t));
        b.set_time(Some(t));
        let first = a.material(registry(), &l.graph, size()).unwrap();
        let second = b.material(registry(), &l.graph, size()).unwrap();
        assert_eq!(first, second, "at {t} s, on any number of cores");
    }
    a.set_time(Some(0.5));
    let early = a.material(registry(), &l.graph, size()).unwrap();
    a.set_time(Some(1.5));
    let later = a.material(registry(), &l.graph, size()).unwrap();
    assert_ne!(early, later, "it moves");
}

#[test]
fn boxes_that_don_t_move_are_kept_as_the_time_goes_on() {
    let l = lava();
    let mut e = Evaluator::with_threads(2);
    e.set_time(Some(0.0));
    let first = e.run(registry(), &l.graph, size(), &[]);
    assert_eq!(first.made(), 5);
    e.set_time(Some(0.1));
    let next = e.run(registry(), &l.graph, size(), &[]);
    // Noise and Clouds come from the cache; Scroll and what it feeds are
    // worked out again.
    assert_eq!(next.reused(), 2);
    assert_eq!(next.made(), 3);
    assert_eq!(first.key(l.noise), next.key(l.noise));
    assert_eq!(first.key(l.clouds), next.key(l.clouds));
    assert_ne!(first.key(l.scroll), next.key(l.scroll));
    assert_ne!(first.key(l.mat), next.key(l.mat));
    // Back to a moment already seen: all from the cache.
    e.set_time(Some(0.0));
    let again = e.run(registry(), &l.graph, size(), &[]);
    assert_eq!(again.made(), 0);
}

#[test]
fn without_a_time_set_the_graphs_moment_is_used() {
    let mut l = lava();
    l.graph
        .apply(registry(), &GraphEdit::SetMoment(1.25))
        .unwrap();
    let mut e = Evaluator::with_threads(2);
    assert_eq!(e.time(), None);
    let baked = e.material(registry(), &l.graph, size()).unwrap();
    e.set_time(Some(1.25));
    assert_eq!(e.material(registry(), &l.graph, size()).unwrap(), baked);
    e.set_time(Some(0.0));
    assert_ne!(e.material(registry(), &l.graph, size()).unwrap(), baked);
    // Times out of range are kept in it.
    e.set_time(Some(f32::INFINITY));
    assert_eq!(e.time(), Some(0.0));
    e.set_time(Some(-3.0));
    assert_eq!(e.time(), Some(0.0));
}

#[test]
fn the_speed_follows_the_real_world_size() {
    let mut l = lava();
    let out = l.graph.first_of(MATERIAL_OUT).unwrap();
    let mut e = Evaluator::with_threads(2);
    e.set_time(Some(0.5));
    let before = e.run(registry(), &l.graph, size(), &[]);
    l.graph
        .apply(
            registry(),
            &GraphEdit::SetSetting {
                node: out,
                key: REAL_SIZE.into(),
                value: SettingValue::Number(300.0),
            },
        )
        .unwrap();
    let after = e.run(registry(), &l.graph, size(), &[]);
    assert_ne!(before.key(l.scroll), after.key(l.scroll));
    assert_eq!(before.key(l.noise), after.key(l.noise));
}

#[test]
fn an_own_box_holding_a_moving_box_moves_too() {
    let l = lava();
    let selection = [l.scroll].into_iter().collect();
    let mut proposal = crate::own::Proposal::of(&l.graph, registry(), &selection).unwrap();
    proposal.name = "Belt".into();
    let id = crate::own::fresh_id("Belt", |_| false);
    let (own, edits) = proposal.make(&l.graph, registry(), id, None).unwrap();
    let own = std::sync::Arc::new(own);
    let with_own = crate::own::registry_with([&own]);
    assert!(with_own.get(&own.id).unwrap().info().animated);
    let mut grouped = l.graph.clone();
    for edit in &edits {
        grouped.apply(&with_own, edit).unwrap();
    }
    assert!(grouped.is_animated(&with_own));
    let mut e = Evaluator::with_threads(2);
    let mut plain = Evaluator::with_threads(2);
    for t in [0.0, 0.8, 2.2] {
        e.set_time(Some(t));
        plain.set_time(Some(t));
        assert_eq!(
            e.material(&with_own, &grouped, size()).unwrap(),
            plain.material(registry(), &l.graph, size()).unwrap(),
            "grouping keeps the picture at {t} s"
        );
    }
}

#[test]
fn a_graph_without_moving_boxes_is_not_animated() {
    let mut g = Graph::new("Still");
    add(&mut g, "noise");
    assert!(!g.is_animated(registry()));
    add(&mut g, "pulse");
    assert!(g.is_animated(registry()));
}
