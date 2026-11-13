//! Bringing your own boxes from one project into another: pasting
//! objects, or boxes on the Material canvas, copied in another tab
//! ([decision 0268](../../../../docs/decisions/0268-paste-between-tabs.md)).
//!
//! Two projects can each have an own box with the same id: the id is
//! made from what was being made when the box was made, so a box copied
//! back and forth keeps it, but either project may have changed its
//! inside since. What comes in therefore never takes the place of what
//! the project has:
//!
//! - **The same box** (the same inside, plugs, sliders, words and
//!   picture) already in the project is used as it is, under its id
//!   there, so pasting the same thing twice doesn't fill the project
//!   with copies. Its name may differ only by a number ("Rocky 2"), as
//!   given by an earlier paste.
//! - **Another box with the same id** keeps its id; the one coming in
//!   gets a new one (and a numbered name when the name is taken too).
//! - Otherwise it comes in as it is, id and all.
//!
//! The boxes inside each box follow: a box holding another holds the one
//! it was given in the project.

use super::{OwnBox, OwnParts, fresh_id, registry_with};
use crate::graph::GraphParts;
use crate::store::GraphStore;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use words::trf;

/// What bringing own boxes into a project needs done.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Brought {
    /// The own boxes to add to the project, each after the boxes it
    /// holds: with their ids and the kinds inside already as the project
    /// will have them.
    pub add: Vec<OwnBox>,
    /// Each box's id where it came from, and its id in the project.
    pub ids: BTreeMap<String, String>,
    /// How many were the same as one the project has (used as they are).
    pub reused: usize,
    /// How many got a new id because the project has another box with
    /// theirs.
    pub new_ids: usize,
}

impl Brought {
    /// The id own box `id` (from where it came) has in the project; any
    /// other kind of box is left as it is.
    pub fn id_of<'a>(&'a self, id: &'a str) -> &'a str {
        self.ids.get(id).map_or(id, String::as_str)
    }
}

/// Every own box of `store` that boxes of `kinds` need: those kinds that
/// are own boxes, and the own boxes inside them, through every level.
/// Kinds the store hasn't are left out.
pub fn needed<'k>(
    store: &GraphStore,
    kinds: impl IntoIterator<Item = &'k str>,
) -> Vec<Arc<OwnBox>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut todo: Vec<String> = kinds
        .into_iter()
        .filter(|k| super::is_own(k))
        .map(str::to_string)
        .collect();
    while let Some(id) = todo.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(own) = store.own(&id) {
            todo.extend(own.uses());
            out.push(own.clone());
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Renames the own boxes inside `parts` by `ids` (where it came from to
/// the project); other kinds stay.
pub fn rename_kinds(parts: &mut GraphParts, ids: &BTreeMap<String, String>) {
    for (_, b) in &mut parts.boxes {
        if let Some(new) = ids.get(&b.kind) {
            b.kind.clone_from(new);
        }
    }
}

/// Whether `name` is `base`, or `base` with a number after it ("Rocky
/// 2"), as a paste names what it brings when the name is taken.
pub fn same_or_numbered(name: &str, base: &str) -> bool {
    name == base
        || name.strip_prefix(base).is_some_and(|rest| {
            rest.starts_with(' ')
                && !rest.trim().is_empty()
                && rest.trim().bytes().all(|b| b.is_ascii_digit())
        })
}

/// `base`, or `base` numbered ("Rocky 2"), not taken.
pub fn unused_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..10_000)
        .map(|n| trf!("{} {}", base, n))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_string())
}

/// What bringing `boxes` (with every own box they hold, from another
/// project) into the project whose graphs are `store` needs done, or why
/// it can't be: a box that is damaged, or holds one that isn't given.
pub fn bring_in(store: &GraphStore, boxes: &[Arc<OwnBox>]) -> Result<Brought, String> {
    let mut brought = Brought::default();
    let mut taken_ids: BTreeSet<String> = store.own_boxes().map(|o| o.id.clone()).collect();
    let mut taken_names: BTreeSet<String> = store.own_boxes().map(|o| o.name.clone()).collect();
    let mut known: Vec<Arc<OwnBox>> = store.own_boxes().cloned().collect();
    for own in in_order(boxes) {
        if let Some(missing) = own
            .uses()
            .into_iter()
            .find(|u| !brought.ids.contains_key(u) && store.own(u).is_none())
        {
            return Err(trf!(
                "{} holds a box of your own that didn't come with it ({})",
                own.name,
                missing
            ));
        }
        let mut parts = own.to_parts();
        rename_kinds(&mut parts.inner, &brought.ids);
        if let Some(same) = same_box(store, &parts) {
            brought.ids.insert(own.id.clone(), same);
            brought.reused += 1;
            continue;
        }
        if taken_ids.contains(&parts.id) {
            // Made from the old id, so the same box gets the same new id.
            let seed = [own.id.as_str(), "/pasted"].concat();
            parts.id = fresh_id(&seed, |i| taken_ids.contains(i));
            brought.new_ids += 1;
        }
        if taken_names.contains(&parts.name) {
            parts.name = unused_name(&parts.name, |n| taken_names.contains(n));
        }
        let registry = registry_with(&known);
        let made = OwnBox::from_parts(parts, &registry)
            .map_err(|why| trf!("{} can't be brought in: {}", own.name, why))?;
        taken_ids.insert(made.id.clone());
        taken_names.insert(made.name.clone());
        brought.ids.insert(own.id.clone(), made.id.clone());
        known.push(Arc::new(made.clone()));
        brought.add.push(made);
    }
    Ok(brought)
}

/// The id of an own box of `store` that is the same as `parts` (its id
/// and tags aside, and its name aside from a number after it), the one
/// with `parts`' id first.
fn same_box(store: &GraphStore, parts: &OwnParts) -> Option<String> {
    let same = |o: &OwnBox| {
        let mut theirs = o.to_parts();
        if !same_or_numbered(&theirs.name, &parts.name) {
            return false;
        }
        theirs.id.clone_from(&parts.id);
        theirs.name.clone_from(&parts.name);
        // Tags are only words to find it by (decision 0292).
        theirs.tags.clone_from(&parts.tags);
        theirs == *parts
    };
    if let Some(o) = store.own(&parts.id).filter(|o| same(o)) {
        return Some(o.id.clone());
    }
    store.own_boxes().find(|o| same(o)).map(|o| o.id.clone())
}

/// `boxes` with every box after the boxes it holds (those among
/// `boxes`), otherwise by id; a loop (never in a sound project) is left
/// in id order at the end.
fn in_order(boxes: &[Arc<OwnBox>]) -> Vec<Arc<OwnBox>> {
    let mut waiting: BTreeMap<String, Arc<OwnBox>> =
        boxes.iter().map(|o| (o.id.clone(), o.clone())).collect();
    let mut out = Vec::with_capacity(waiting.len());
    loop {
        let ready: Vec<String> = waiting
            .values()
            .filter(|o| o.uses().iter().all(|u| !waiting.contains_key(u)))
            .map(|o| o.id.clone())
            .collect();
        if ready.is_empty() {
            break;
        }
        for id in ready {
            out.extend(waiting.remove(&id));
        }
    }
    out.extend(waiting.into_values());
    out
}
