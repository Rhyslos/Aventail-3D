//! Tests for bringing own boxes from another project: new ones come in
//! as they are, the same ones are reused, another box with the same id
//! keeps it, boxes inside follow, and damaged ones are refused.

use super::bringing::{bring_in, needed, same_or_numbered, unused_name};
use super::*;
use crate::graph::GraphEdit;
use crate::store::GraphStore;
use glam::Vec2;

fn standard() -> &'static Registry {
    Registry::standard()
}

/// An own box called `name` holding a Levels box (its `low` at `low`),
/// and maybe an own box `inside`.
fn own(name: &str, low: f32, inside: Option<&OwnBox>) -> OwnBox {
    let mut known = Vec::new();
    if let Some(i) = inside {
        known.push(Arc::new(i.clone()));
    }
    let registry = registry_with(&known);
    let mut g = Graph::new("inside");
    g.apply(
        &registry,
        &GraphEdit::AddBox {
            kind: "levels".into(),
            at: Vec2::ZERO,
        },
    )
    .unwrap();
    g.apply(
        &registry,
        &GraphEdit::SetSetting {
            node: BoxId(1),
            key: "low".into(),
            value: crate::SettingValue::Number(low),
        },
    )
    .unwrap();
    if let Some(i) = inside {
        g.apply(
            &registry,
            &GraphEdit::AddBox {
                kind: i.id.clone(),
                at: Vec2::new(200.0, 0.0),
            },
        )
        .unwrap();
    }
    OwnBox {
        id: fresh_id(name, |_| false),
        name: name.into(),
        about: String::new(),
        picture: None,
        inner: Arc::new(g),
        inputs: Vec::new(),
        outputs: Vec::new(),
        sliders: Vec::new(),
        tags: Vec::new(),
    }
}

fn store_with(boxes: &[OwnBox]) -> GraphStore {
    GraphStore::new().with_own(boxes.iter().cloned()).unwrap()
}

#[test]
fn a_box_the_project_hasn_t_comes_in_as_it_is() {
    let rocky = own("Rocky", 0.2, None);
    let brought = bring_in(&GraphStore::new(), &[Arc::new(rocky.clone())]).unwrap();
    assert_eq!(brought.add, vec![rocky.clone()]);
    assert_eq!(brought.id_of(&rocky.id), rocky.id);
    assert_eq!((brought.reused, brought.new_ids), (0, 0));
}

#[test]
fn the_same_box_is_reused() {
    let rocky = own("Rocky", 0.2, None);
    let store = store_with(std::slice::from_ref(&rocky));
    let brought = bring_in(&store, &[Arc::new(rocky.clone())]).unwrap();
    assert!(brought.add.is_empty());
    assert_eq!(brought.reused, 1);
    assert_eq!(brought.id_of(&rocky.id), rocky.id);
}

#[test]
fn another_box_with_the_same_id_keeps_it_and_the_new_one_gets_its_own() {
    let theirs = own("Rocky", 0.2, None);
    let mut ours = own("Rocky", 0.5, None);
    ours.id.clone_from(&theirs.id);
    let store = store_with(std::slice::from_ref(&ours));
    let brought = bring_in(&store, &[Arc::new(theirs.clone())]).unwrap();
    assert_eq!(brought.add.len(), 1);
    let added = &brought.add[0];
    assert_ne!(added.id, theirs.id);
    assert!(is_own_id(&added.id));
    assert_eq!(added.name, "Rocky 2");
    assert_eq!(added.inner, theirs.inner);
    assert_eq!(brought.new_ids, 1);
    // Brought again into the project that now has both: the numbered one
    // is the same, so it is reused.
    let store = store_with(&[ours, added.clone()]);
    let again = bring_in(&store, &[Arc::new(theirs.clone())]).unwrap();
    assert!(again.add.is_empty());
    assert_eq!(again.id_of(&theirs.id), added.id);
}

#[test]
fn boxes_inside_follow_the_ids_given() {
    let inner = own("Pebble", 0.1, None);
    let outer = own("Rocky", 0.2, Some(&inner));
    // The project has another Pebble with the same id.
    let mut other = own("Pebble", 0.9, None);
    other.id.clone_from(&inner.id);
    let store = store_with(std::slice::from_ref(&other));
    // Given outer first: inner is still added before it.
    let brought = bring_in(&store, &[Arc::new(outer.clone()), Arc::new(inner.clone())]).unwrap();
    assert_eq!(brought.add.len(), 2);
    let new_inner = brought.id_of(&inner.id).to_string();
    assert_ne!(new_inner, inner.id);
    assert_eq!(brought.add[0].id, new_inner);
    assert!(brought.add[1].uses().contains(&new_inner));
    assert!(!brought.add[1].uses().contains(&inner.id));
    // Every box added can be put in the project, in order.
    let mut store = store;
    for o in &brought.add {
        store.put_own(&o.id, Some(Arc::new(o.clone())));
    }
    for o in &brought.add {
        assert!(o.check(store.registry()).is_ok());
    }
}

#[test]
fn a_box_holding_one_not_given_is_refused() {
    let inner = own("Pebble", 0.1, None);
    let outer = own("Rocky", 0.2, Some(&inner));
    let why = bring_in(&GraphStore::new(), &[Arc::new(outer)]).unwrap_err();
    assert!(why.contains("Rocky"), "{why}");
}

#[test]
fn what_boxes_need_goes_through_every_level() {
    let inner = own("Pebble", 0.1, None);
    let outer = own("Rocky", 0.2, Some(&inner));
    let store = store_with(&[inner.clone(), outer.clone()]);
    let got: Vec<String> = needed(&store, [outer.id.as_str(), "noise", "own:feed"])
        .iter()
        .map(|o| o.id.clone())
        .collect();
    let mut want = vec![inner.id.clone(), outer.id.clone()];
    want.sort();
    assert_eq!(got, want);
    assert!(needed(&store, ["levels"]).is_empty());
}

#[test]
fn numbered_names() {
    assert!(same_or_numbered("Rocky", "Rocky"));
    assert!(same_or_numbered("Rocky 12", "Rocky"));
    assert!(!same_or_numbered("Rocky road", "Rocky"));
    assert!(!same_or_numbered("Rocky ", "Rocky"));
    assert!(!same_or_numbered("Rock", "Rocky"));
    assert_eq!(unused_name("A", |n| n == "A" || n == "A 2"), "A 3");
    assert_eq!(unused_name("B", |_| false), "B");
    let _ = standard();
}

#[test]
fn a_box_differing_only_in_its_tags_is_reused() {
    let rocky = own("Rocky", 0.2, None);
    let store = store_with(std::slice::from_ref(&rocky));
    let mut tagged = rocky.clone();
    tagged.tags = vec!["stone".into()];
    assert!(tagged.same_but_tags(&rocky));
    assert_ne!(tagged, rocky);
    let brought = bring_in(&store, &[Arc::new(tagged)]).unwrap();
    assert!(brought.add.is_empty());
    assert_eq!(brought.reused, 1);
    // Untidy tags make a box that can't be kept.
    let mut untidy = rocky.clone();
    untidy.tags = vec!["B".into()];
    assert!(untidy.check(crate::Registry::standard()).is_err());
}
