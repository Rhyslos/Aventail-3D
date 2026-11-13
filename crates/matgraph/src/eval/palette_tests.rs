//! Tests for boxes colouring with the project's palettes (milestone 19):
//! the registry carries them to every box, changing a palette makes only
//! the boxes using it again, an own box holding a Palette map colours
//! with the outer project's palettes, and the palettes a graph names are
//! found through own boxes.

use super::*;
use crate::graph::GraphEdit;
use crate::palettes::{Palette, Palettes};
use crate::settings::SettingValue;
use glam::Vec2;

fn palettes(colours: Vec<[u8; 3]>) -> Palettes {
    Palettes::new([
        Palette {
            name: "Game".into(),
            colours,
        },
        Palette {
            name: "Moss".into(),
            colours: vec![[40, 90, 30], [120, 170, 60]],
        },
    ])
}

fn registry_with(p: Palettes) -> Registry {
    Registry::standard().clone().with_palettes(p)
}

fn add(g: &mut Graph, registry: &Registry, kind: &str) -> BoxId {
    let id = BoxId(g.next_id());
    g.apply(
        registry,
        &GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(id.0 as f32 * 200.0, 0.0),
        },
    )
    .unwrap();
    id
}

fn wire(g: &mut Graph, registry: &Registry, from: (BoxId, &str), to: (BoxId, &str)) {
    g.apply(
        registry,
        &GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        },
    )
    .unwrap();
}

/// Noise → Palette map, and Clouds beside it.
struct Painted {
    graph: Graph,
    noise: BoxId,
    map: BoxId,
    clouds: BoxId,
}

fn painted(registry: &Registry) -> Painted {
    let mut g = Graph::new("Painted");
    let noise = add(&mut g, registry, "noise");
    let map = add(&mut g, registry, "palette_map");
    let clouds = add(&mut g, registry, "clouds");
    wire(&mut g, registry, (noise, "height"), (map, "in"));
    Painted {
        graph: g,
        noise,
        map,
        clouds,
    }
}

fn size() -> Size {
    Size::new(32).unwrap()
}

#[test]
fn a_palette_change_makes_only_its_boxes_again() {
    let first = registry_with(palettes(vec![[0, 0, 0], [255, 255, 255]]));
    let p = painted(&first);
    let mut e = Evaluator::with_threads(2);
    let before = e.run(&first, &p.graph, size(), &[]);
    assert!(before.outcome(p.map).unwrap().is_ok());
    let changed = registry_with(palettes(vec![[200, 0, 0], [0, 0, 200]]));
    let after = e.run(&changed, &p.graph, size(), &[]);
    assert_ne!(before.key(p.map), after.key(p.map));
    assert_eq!(before.key(p.noise), after.key(p.noise));
    assert_eq!(before.key(p.clouds), after.key(p.clouds));
    assert_ne!(before.picture(p.map), after.picture(p.map));
    // Every pixel is one of the new swatches.
    let Some(Value::Colour(c)) = after.picture(p.map) else {
        panic!("no colour");
    };
    let red = [200.0 / 255.0, 0.0, 0.0];
    let blue = [0.0, 0.0, 200.0 / 255.0];
    assert!(c.pixels().iter().all(|px| *px == red || *px == blue));
    // The same palettes again: found in the cache.
    let again = e.run(&changed, &p.graph, size(), &[]);
    assert_eq!(again.made(), 0);
}

#[test]
fn without_the_projects_palettes_the_box_says_why() {
    let p = painted(&registry_with(palettes(vec![[1, 2, 3]])));
    let mut e = Evaluator::with_threads(1);
    let out = e.run(Registry::standard(), &p.graph, size(), &[]);
    let why = out.outcome(p.map).unwrap().as_ref().unwrap_err();
    assert_eq!(why.fixes, [crate::Fix::AddPalettes]);
    // A named palette the project doesn't have.
    let mut g = p.graph.clone();
    let registry = registry_with(palettes(vec![[1, 2, 3]]));
    g.apply(
        &registry,
        &GraphEdit::SetSetting {
            node: p.map,
            key: "palette".into(),
            value: SettingValue::Palette("Lava".into()),
        },
    )
    .unwrap();
    let out = e.run(&registry, &g, size(), &[]);
    let why = out.outcome(p.map).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("Lava"));
    assert_eq!(why.fixes.len(), 2);
    assert_eq!(
        crate::palettes::names_used(&registry, &g),
        ["Lava".to_string()].into()
    );
    // The first palette isn't a name to bring along.
    assert!(crate::palettes::names_used(&registry, &p.graph).is_empty());
}

#[test]
fn an_own_box_colours_with_the_outer_projects_palettes() {
    let registry = registry_with(palettes(vec![[10, 20, 30], [200, 210, 220]]));
    let mut p = painted(&registry);
    p.graph
        .apply(
            &registry,
            &GraphEdit::SetSetting {
                node: p.map,
                key: "palette".into(),
                value: SettingValue::Palette("Moss".into()),
            },
        )
        .unwrap();
    let selection = [p.map].into_iter().collect();
    let mut proposal = crate::own::Proposal::of(&p.graph, &registry, &selection).unwrap();
    proposal.name = "Mossy".into();
    // The palette becomes one of its sliders, starting at Moss.
    for slider in &mut proposal.sliders {
        slider.chosen = slider.setting == "palette";
    }
    assert!(proposal.sliders.iter().any(|s| s.chosen));
    let id = crate::own::fresh_id("Mossy", |_| false);
    let (own, edits) = proposal.make(&p.graph, &registry, id, None).unwrap();
    let own = std::sync::Arc::new(own);
    let with_own = crate::own::registry_with([&own]).with_palettes(registry.palettes().clone());
    let kind = with_own.get(&own.id).unwrap();
    assert!(kind.uses_palettes());
    let slider = kind
        .info()
        .settings
        .iter()
        .find(|s| s.key != "seed")
        .unwrap();
    assert_eq!(
        slider.standard(),
        SettingValue::Palette("Moss".into()),
        "the slider starts at the palette chosen inside"
    );
    let mut grouped = p.graph.clone();
    for edit in &edits {
        grouped.apply(&with_own, edit).unwrap();
    }
    assert!(crate::palettes::uses_palettes(&with_own, &grouped));
    assert!(!crate::palettes::uses_palettes(
        &with_own,
        &Graph::new("Empty")
    ));
    assert_eq!(
        crate::palettes::names_used(&with_own, &grouped),
        ["Moss".to_string()].into()
    );
    let mut e = Evaluator::with_threads(2);
    let grouped_out = e.run(&with_own, &grouped, size(), &[]);
    let plain_out = e.run(&registry, &p.graph, size(), &[]);
    let placed = grouped
        .boxes()
        .find(|(_, b)| b.kind == own.id)
        .map(|(id, _)| id)
        .unwrap();
    let Some(Value::Colour(inside)) = grouped_out.picture(placed) else {
        panic!("the own box didn't run");
    };
    let Some(Value::Colour(plain)) = plain_out.picture(p.map) else {
        panic!("the palette map didn't run");
    };
    assert_eq!(inside, plain, "grouping keeps the picture");
    // Changing the outer palettes changes the own box.
    let moss = Palettes::new([Palette {
        name: "Moss".into(),
        colours: vec![[0, 0, 0]],
    }]);
    let changed = crate::own::registry_with([&own]).with_palettes(moss);
    let after = e.run(&changed, &grouped, size(), &[]);
    assert_ne!(grouped_out.key(placed), after.key(placed));
    let Some(Value::Colour(black)) = after.picture(placed) else {
        panic!("the own box didn't run");
    };
    assert!(black.pixels().iter().all(|px| *px == [0.0; 3]));
}

#[test]
fn the_graphs_store_keeps_its_palettes_through_own_box_changes() {
    let mut store = crate::GraphStore::new();
    let p = palettes(vec![[1, 1, 1]]);
    store.set_palettes(p.clone());
    assert_eq!(*store.registry().palettes(), p);
    let r = registry_with(p.clone());
    let g = painted(&r);
    let selection = [g.noise].into_iter().collect();
    let mut proposal = crate::own::Proposal::of(&g.graph, &r, &selection).unwrap();
    proposal.name = "Bumps".into();
    let id = crate::own::fresh_id("Bumps", |_| false);
    let (own, _) = proposal.make(&g.graph, &r, id.clone(), None).unwrap();
    store.put_own(&id, Some(std::sync::Arc::new(own)));
    assert_eq!(*store.registry().palettes(), p);
    assert!(store.registry().get(&id).is_some());
    let before = store.registry().clone();
    store.set_palettes(p.clone());
    assert!(
        std::sync::Arc::ptr_eq(&before, store.registry()),
        "the same palettes leave the registry alone"
    );
    store.set_palettes(Palettes::default());
    assert!(store.registry().palettes().is_empty());
    assert!(store.registry().get(&id).is_some());
}
