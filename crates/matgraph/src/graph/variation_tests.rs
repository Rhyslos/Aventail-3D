//! Tests for variations and the dice (milestone 18): saving, using,
//! updating, renaming and deleting variations, what they hold, the graph
//! with a variation for the painter, rebuilding from parts, and the dice
//! rolling every seed the same way for the same roll.

use super::*;
use crate::own::{Proposal, fresh_id, registry_with};
use crate::settings::SettingValue;
use glam::Vec2;
use std::sync::Arc;

fn registry() -> &'static Registry {
    Registry::standard()
}

fn add(graph: &mut Graph, kind: &str) -> BoxId {
    let id = BoxId(graph.next_id());
    let at = Vec2::new(id.0 as f32 * 200.0, 0.0);
    let edit = GraphEdit::AddBox {
        kind: kind.into(),
        at,
    };
    graph.apply(registry(), &edit).unwrap();
    id
}

fn set(graph: &mut Graph, node: BoxId, key: &str, value: SettingValue) {
    let edit = GraphEdit::SetSetting {
        node,
        key: key.into(),
        value,
    };
    graph.apply(registry(), &edit).unwrap();
}

fn setting(graph: &Graph, node: BoxId, key: &str) -> SettingValue {
    let b = graph.get(node).unwrap();
    let spec = registry()
        .get(&b.kind)
        .unwrap()
        .info()
        .setting(key)
        .unwrap();
    b.settings.get(key).unwrap_or_else(|| spec.standard())
}

/// Noise into Levels, Levels' Low shown in the painter; Tiles beside.
struct Moss {
    graph: Graph,
    noise: BoxId,
    tiles: BoxId,
    levels: BoxId,
}

fn moss() -> Moss {
    let mut graph = Graph::new("Moss");
    let noise = add(&mut graph, "noise");
    let tiles = add(&mut graph, "tiles");
    let levels = add(&mut graph, "levels");
    let wire = GraphEdit::Connect {
        from: PlugRef::new(noise, "height"),
        to: PlugRef::new(levels, "in"),
    };
    graph.apply(registry(), &wire).unwrap();
    let show = GraphEdit::ShowInPainter {
        node: levels,
        key: "low".into(),
        shown: true,
    };
    graph.apply(registry(), &show).unwrap();
    Moss {
        graph,
        noise,
        tiles,
        levels,
    }
}

/// Saves the graph as it is as variation `name`; its id.
fn save(graph: &mut Graph, name: &str) -> VariationId {
    let id = VariationId(graph.next_id());
    let variation = graph.variation_now(registry(), name);
    graph
        .apply(registry(), &GraphEdit::AddVariation(variation))
        .unwrap();
    id
}

fn using(graph: &mut Graph, id: VariationId) {
    for edit in graph.variation_edits(registry(), id) {
        graph.apply(registry(), &edit).unwrap();
    }
}

#[test]
fn a_variation_holds_shown_settings_every_seed_and_the_moment() {
    let Moss {
        mut graph,
        noise,
        tiles,
        levels,
    } = moss();
    set(&mut graph, levels, "low", SettingValue::Number(0.3));
    set(&mut graph, noise, "seed", SettingValue::Whole(5));
    graph.apply(registry(), &GraphEdit::SetSeed(9)).unwrap();
    graph.apply(registry(), &GraphEdit::SetMoment(2.5)).unwrap();
    let v = graph.variation_now(registry(), "Clean");
    assert_eq!(v.name, "Clean");
    assert_eq!(v.value(levels, "low"), Some(SettingValue::Number(0.3)));
    assert_eq!(v.value(noise, "seed"), Some(SettingValue::Whole(5)));
    // A seed left at its standard is held too, so using it puts it back.
    assert_eq!(v.value(tiles, "seed"), Some(SettingValue::Whole(0)));
    // Settings neither shown nor seeds aren't.
    assert_eq!(v.value(levels, "high"), None);
    assert_eq!((v.seed, v.moment), (9, 2.5));
}

#[test]
fn using_a_variation_sets_its_values_back_and_marks_it_in_use() {
    let Moss {
        mut graph,
        noise,
        levels,
        ..
    } = moss();
    set(&mut graph, levels, "low", SettingValue::Number(0.1));
    let clean = save(&mut graph, "Clean");
    assert_eq!(graph.variation_in_use(registry()), Some(clean));
    assert!(graph.variation_edits(registry(), clean).is_empty());
    set(&mut graph, levels, "low", SettingValue::Number(0.6));
    set(&mut graph, noise, "seed", SettingValue::Whole(77));
    graph.apply(registry(), &GraphEdit::SetSeed(3)).unwrap();
    let mossy = save(&mut graph, "Mossy");
    assert_ne!(clean, mossy);
    assert_eq!(graph.variation_in_use(registry()), Some(mossy));
    using(&mut graph, clean);
    assert_eq!(setting(&graph, levels, "low"), SettingValue::Number(0.1));
    assert_eq!(setting(&graph, noise, "seed"), SettingValue::Whole(0));
    assert_eq!(graph.seed(), 0);
    assert_eq!(graph.variation_in_use(registry()), Some(clean));
    using(&mut graph, mossy);
    assert_eq!(setting(&graph, levels, "low"), SettingValue::Number(0.6));
    assert_eq!(graph.seed(), 3);
    // A change nothing saved: no variation is in use.
    set(&mut graph, levels, "low", SettingValue::Number(0.45));
    assert_eq!(graph.variation_in_use(registry()), None);
    // Settings outside variations are left alone.
    set(&mut graph, levels, "high", SettingValue::Number(0.8));
    using(&mut graph, clean);
    assert_eq!(setting(&graph, levels, "high"), SettingValue::Number(0.8));
}

#[test]
fn updating_renaming_and_deleting_a_variation() {
    let Moss {
        mut graph, levels, ..
    } = moss();
    let id = save(&mut graph, "Clean");
    set(&mut graph, levels, "low", SettingValue::Number(0.4));
    // Updated to the graph as it is now, keeping its name.
    let mut updated = graph.variation_now(registry(), "Clean");
    graph
        .apply(
            registry(),
            &GraphEdit::ChangeVariation {
                id,
                variation: updated.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        graph.variation(id).unwrap().value(levels, "low"),
        Some(SettingValue::Number(0.4))
    );
    updated.name = "Weathered".into();
    let rename = GraphEdit::ChangeVariation {
        id,
        variation: updated,
    };
    graph.apply(registry(), &rename).unwrap();
    assert_eq!(graph.variation(id).unwrap().name, "Weathered");
    // A blank name is refused, and changes nothing.
    let mut blank = graph.variation(id).unwrap().clone();
    blank.name = "  ".into();
    let before = graph.clone();
    let why = graph
        .apply(
            registry(),
            &GraphEdit::ChangeVariation {
                id,
                variation: blank,
            },
        )
        .unwrap_err();
    assert!(why.reason.contains("name"), "{}", why.reason);
    assert_eq!(graph, before);
    graph
        .apply(registry(), &GraphEdit::RemoveVariation(id))
        .unwrap();
    assert!(graph.variation(id).is_none());
    // Gone: changing or deleting it again is refused with why.
    let again = graph
        .apply(registry(), &GraphEdit::RemoveVariation(id))
        .unwrap_err();
    assert!(again.reason.contains("isn't in the graph"));
    // Ids are never reused: the next box gets a new one.
    assert!(add(&mut graph, "noise").0 > id.0);
}

#[test]
fn a_variation_skips_boxes_no_longer_there_and_brings_values_into_range() {
    let Moss {
        mut graph,
        noise,
        levels,
        ..
    } = moss();
    let id = save(&mut graph, "Clean");
    graph
        .apply(registry(), &GraphEdit::RemoveBoxes(vec![noise]))
        .unwrap();
    set(&mut graph, levels, "low", SettingValue::Number(0.5));
    // Out of range in the variation (as from an older app's file).
    let mut odd = graph.variation(id).unwrap().clone();
    for v in &mut odd.values {
        if v.key == "low" {
            v.value = SettingValue::Number(7.0);
        }
    }
    let change = GraphEdit::ChangeVariation { id, variation: odd };
    graph.apply(registry(), &change).unwrap();
    using(&mut graph, id);
    assert_eq!(setting(&graph, levels, "low"), SettingValue::Number(1.0));
}

#[test]
fn the_graph_with_a_variation_is_what_a_painter_layer_runs() {
    let Moss {
        mut graph, levels, ..
    } = moss();
    set(&mut graph, levels, "low", SettingValue::Number(0.2));
    let clean = save(&mut graph, "Clean");
    set(&mut graph, levels, "low", SettingValue::Number(0.7));
    let looked = graph.with_variation(registry(), clean);
    assert_eq!(setting(&looked, levels, "low"), SettingValue::Number(0.2));
    // The graph itself is unchanged, and a missing variation changes
    // nothing.
    assert_eq!(setting(&graph, levels, "low"), SettingValue::Number(0.7));
    assert_eq!(graph.with_variation(registry(), VariationId(999)), graph);
    // Without its variations, the look is the same.
    let bare = looked.clone().without_variations();
    assert_eq!(bare.variations().count(), 0);
    assert_eq!(setting(&bare, levels, "low"), SettingValue::Number(0.2));
}

#[test]
fn variations_survive_parts_and_damaged_ones_are_refused() {
    let Moss { mut graph, .. } = moss();
    let a = save(&mut graph, "Clean");
    save(&mut graph, "Mossy");
    graph
        .apply(registry(), &GraphEdit::RemoveVariation(a))
        .unwrap();
    let back = Graph::from_parts(graph.to_parts(), registry()).unwrap();
    assert_eq!(back, graph);
    assert_eq!(
        back.variations()
            .map(|(_, v)| v.name.as_str())
            .collect::<Vec<_>>(),
        ["Mossy"]
    );
    // An id shared with a box, zero, or past the counter.
    for bad in [1, 0, graph.next_id()] {
        let mut parts = graph.to_parts();
        parts.variations[0].0 = VariationId(bad);
        assert!(Graph::from_parts(parts, registry()).is_err(), "{bad}");
    }
    // A blank name, a number that isn't, a moment out of range.
    let damage: [fn(&mut Variation); 3] = [
        |v| v.name.clear(),
        |v| v.values[0].value = SettingValue::Number(f32::NAN),
        |v| v.moment = -1.0,
    ];
    for hurt in damage {
        let mut parts = graph.to_parts();
        hurt(&mut parts.variations[0].1);
        assert!(Graph::from_parts(parts, registry()).is_err());
    }
    let mut parts = graph.to_parts();
    let one = parts.variations[0].1.clone();
    parts.variations = (0..=MAX_VARIATIONS as u64)
        .map(|i| (VariationId(100 + i), one.clone()))
        .collect();
    parts.next_id = 10_000;
    assert!(Graph::from_parts(parts, registry()).is_err());
}

#[test]
fn a_graph_holds_at_most_its_limit_of_variations() {
    let Moss { mut graph, .. } = moss();
    for i in 0..MAX_VARIATIONS {
        save(&mut graph, &format!("Look {i}"));
    }
    let more = graph.variation_now(registry(), "One more");
    let why = graph
        .apply(registry(), &GraphEdit::AddVariation(more))
        .unwrap_err();
    assert!(why.reason.contains("delete one first"), "{}", why.reason);
}

#[test]
fn new_variation_names_count_on() {
    let Moss { mut graph, .. } = moss();
    assert_eq!(graph.new_variation_name(), "Variation 1");
    save(&mut graph, "Variation 1");
    save(&mut graph, "Variation 3");
    assert_eq!(graph.new_variation_name(), "Variation 2");
}

#[test]
fn the_dice_roll_every_seed_the_same_way_for_the_same_roll() {
    let Moss {
        mut graph,
        noise,
        tiles,
        levels,
    } = moss();
    let edits = graph.rolled(registry(), 12345, None);
    // Noise's and Tiles' seeds and the graph's own; nothing of Levels.
    assert_eq!(edits.len(), 3);
    assert!(edits.iter().all(|e| match e {
        GraphEdit::SetSetting { node, value, .. } =>
            *node != levels
                && matches!(value, SettingValue::Whole(n) if (0..=MAX_ROLLED_SEED as i32).contains(n)),
        GraphEdit::SetSeed(n) => *n <= MAX_ROLLED_SEED,
        _ => false,
    }));
    assert_eq!(edits, graph.rolled(registry(), 12345, None));
    assert_ne!(edits, graph.rolled(registry(), 12346, None));
    let before = graph.clone();
    for edit in &edits {
        graph.apply(registry(), edit).unwrap();
    }
    assert_ne!(
        setting(&graph, noise, "seed"),
        setting(&before, noise, "seed")
    );
    assert_ne!(
        setting(&graph, tiles, "seed"),
        setting(&before, tiles, "seed")
    );
    assert_ne!(
        setting(&graph, noise, "seed"),
        setting(&graph, tiles, "seed")
    );
    assert_ne!(graph.seed(), before.seed());
    // Only one box's: the graph's seed stays.
    let one = graph.rolled(registry(), 5, Some(tiles));
    assert_eq!(one.len(), 1);
    assert!(matches!(&one[0], GraphEdit::SetSetting { node, .. } if *node == tiles));
    // The roll number moves on with the graph and the count.
    assert_ne!(graph.roll_from(0), before.roll_from(0));
    assert_ne!(graph.roll_from(0), graph.roll_from(1));
    assert_eq!(graph.roll_from(4), graph.roll_from(4));
}

#[test]
fn rolled_seeds_spread_over_their_range() {
    let Moss { graph, noise, .. } = moss();
    let seeds: std::collections::BTreeSet<i32> = (0..500)
        .filter_map(|roll| {
            graph
                .rolled(registry(), roll, Some(noise))
                .into_iter()
                .find_map(|e| match e {
                    GraphEdit::SetSetting {
                        value: SettingValue::Whole(n),
                        ..
                    } => Some(n),
                    _ => None,
                })
        })
        .collect();
    assert!(seeds.len() > 490, "{}", seeds.len());
    let mean = seeds.iter().map(|n| f64::from(*n)).sum::<f64>() / seeds.len() as f64;
    assert!((mean - 500_000.0).abs() < 60_000.0, "{mean}");
}

#[test]
fn the_dice_roll_your_own_boxes_seeds_through_their_sliders() {
    let Moss {
        graph,
        noise,
        levels,
        ..
    } = moss();
    let selection = [noise, levels].into_iter().collect();
    let mut proposal = Proposal::of(&graph, registry(), &selection).unwrap();
    for s in &mut proposal.sliders {
        s.chosen = s.setting == "seed" || s.setting == "low";
    }
    proposal.name = "Rocky".into();
    let id = fresh_id("Rocky", |_| false);
    let (own, edits) = proposal.make(&graph, registry(), id, None).unwrap();
    let own = Arc::new(own);
    let all = registry_with(std::slice::from_ref(&own));
    let mut after = graph.clone();
    for edit in &edits {
        after.apply(&all, edit).unwrap();
    }
    let placed = after.first_of(&own.id).unwrap();
    let keys = seed_keys(&all, &own.id);
    assert_eq!(keys.len(), 1, "{keys:?}");
    let rolled = after.rolled(&all, 99, Some(placed));
    assert!(matches!(&rolled[..], [GraphEdit::SetSetting { key, .. }] if *key == keys[0]));
    for edit in &rolled {
        after.apply(&all, edit).unwrap();
    }
    // A variation holds it too.
    let v = after.variation_now(&all, "Rolled");
    assert!(v.value(placed, &keys[0]).is_some());
}
