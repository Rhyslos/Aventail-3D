//! [`super::Command::Paste`]: objects copied in another project pasted
//! into this one, with their materials' texture sets, graphs and own
//! boxes ([decision 0268](../../../../docs/decisions/0268-paste-between-tabs.md)),
//! and the colour palettes their boxes name that the project lacks
//! ([decision 0281](../../../../docs/decisions/0281-palette-map.md)).
//!
//! Everything is planned first ([`Clipping::plan`]); then own boxes,
//! graphs and texture sets go in, then the objects. Each object is added
//! at the end of the top level once the objects it reads (an instance's
//! original, cutters, modifiers' sources, a sweep's cross-section) are
//! in, so what it names is always there and checked as any new object's
//! is; then each goes inside its parent, in order, and the levels of
//! detail made by hand find their shapes.

use super::Run;
use crate::clipping::{Clipping, material_names};
use crate::object::{Material, Object, ObjectId, ObjectKind};
use crate::tree::{Edit, Place, SceneError};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// An id no object has: what names an object that didn't come along.
const NOTHING: ObjectId = ObjectId(u64::MAX);

/// The clipping's objects (by place in it) in an order where each comes
/// after the objects it reads; any loop (never in a sound project) is
/// left in the clipping's order at the end.
fn reading_order(clip: &Clipping) -> Vec<usize> {
    let place: BTreeMap<ObjectId, usize> = clip
        .objects()
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id, i))
        .collect();
    let needs: Vec<BTreeSet<usize>> = clip
        .objects()
        .iter()
        .map(|c| {
            let mut ids = reads_or_shows(&c.object);
            ids.retain(|id| *id != c.id);
            ids.iter().filter_map(|id| place.get(id).copied()).collect()
        })
        .collect();
    let mut done = vec![false; needs.len()];
    let mut out = Vec::with_capacity(needs.len());
    loop {
        let before = out.len();
        for i in 0..needs.len() {
            if !done[i] && needs[i].iter().all(|n| done[*n]) {
                done[i] = true;
                out.push(i);
            }
        }
        if out.len() == before {
            break;
        }
    }
    out.extend((0..needs.len()).filter(|i| !done[*i]));
    out
}

/// The objects `object` reads or shows: its original, for an instance,
/// its booleans' cutters, its modifiers' sources and its sweep's
/// cross-section.
fn reads_or_shows(object: &Object) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = object
        .booleans
        .iter()
        .flat_map(|b| b.cutters.iter().copied())
        .collect();
    out.extend(object.modifiers.iter().flat_map(|m| m.kind.sources()));
    if let Some(profile) = object.kind.curve().and_then(|c| c.solid.profile_object()) {
        out.push(profile);
    }
    if let ObjectKind::Instance(source) = object.kind {
        out.push(source);
    }
    out
}

/// `object` naming the pasted objects (`new`) instead of the ones it was
/// copied with; anything else it named, [`NOTHING`] (or nothing).
fn renamed(object: &Object, new: &BTreeMap<ObjectId, ObjectId>) -> Object {
    let to = |id: ObjectId| new.get(&id).copied().unwrap_or(NOTHING);
    let mut out = object.clone();
    if let ObjectKind::Instance(source) = out.kind {
        out.kind = ObjectKind::Instance(to(source));
    }
    if let ObjectKind::Curve(curve) = &mut out.kind
        && let crate::CurveSolid::Sweep(sweep) = &mut curve.solid
        && let crate::SweepProfile::Curve(id) = &mut sweep.profile
    {
        *id = to(*id);
    }
    for boolean in &mut out.booleans {
        boolean.cutters.retain(|c| new.contains_key(c));
        for c in &mut boolean.cutters {
            *c = to(*c);
        }
    }
    out.booleans.retain(|b| !b.cutters.is_empty());
    for m in &mut out.modifiers {
        m.kind = m.kind.with_sources_renamed(to);
    }
    // Found once everything is in its place.
    for level in &mut out.lods.levels {
        level.hand = None;
    }
    out
}

impl Run<'_> {
    /// [`super::Command::Paste`]: returns the chosen objects' copies.
    pub(super) fn paste(&mut self, clip: &Clipping) -> Result<Option<Vec<ObjectId>>, SceneError> {
        if clip.objects().is_empty() {
            return Ok(None);
        }
        let plan = clip.plan(self.scene)?;
        if !plan.palettes.is_empty() {
            let before = self.scene.paint().palettes().to_vec();
            let mut after = before.clone();
            after.extend(plan.palettes.iter().cloned());
            self.edit(Edit::SetPalettes { before, after })?;
        }
        for own in &plan.own.add {
            self.set_own_box(&own.id, Some(own))?;
        }
        for graph in &plan.graphs {
            if let Some(add) = &graph.add {
                if self.scene.graphs().next_id() != graph.to {
                    return Err(SceneError::History(words::text!(
                        "a material graph's id was taken"
                    )));
                }
                self.add_graph(add)?;
            }
        }
        for material in &plan.materials {
            if let Some(set) = &material.set {
                let before = self.scene.paint().set(&material.to).cloned();
                self.edit(Edit::SetTextureSet {
                    material: material.to.clone(),
                    before,
                    after: Some(set.clone()),
                })?;
            }
        }
        // The objects, each once what it reads is in.
        let mut new: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
        for i in reading_order(clip) {
            let c = &clip.objects()[i];
            let mut object = renamed(&c.object, &new);
            let names = material_names(&c.object);
            if c.object.materials.is_empty() {
                if let Some(standard) = names.first()
                    && plan.material_name(standard) != standard
                {
                    object.materials = vec![Material {
                        name: plan.material_name(standard).to_string(),
                        ..Material::standard()
                    }];
                }
            } else {
                for m in &mut object.materials {
                    m.name = plan.material_name(&m.name).to_string();
                }
            }
            let base = object.name.trim().to_string();
            if base.is_empty() {
                return Err(SceneError::EmptyName);
            }
            object.name = self.scene.unused_name(&base);
            let place = Place {
                parent: None,
                index: self.scene.children(None).len(),
            };
            let id = self.insert(object, place)?;
            new.insert(c.id, id);
        }
        // Each inside its parent, in order; the top ones at the end of
        // the top level, in order.
        let mut tops = Vec::new();
        for c in clip.objects() {
            let id = new[&c.id];
            match c.parent.and_then(|p| new.get(&p)) {
                Some(parent) => {
                    let index = self.scene.children(Some(*parent)).len();
                    self.move_into(&[id], Some(*parent), index)?;
                }
                None => tops.push(id),
            }
        }
        let end = self.scene.children(None).len();
        self.move_into(&tops, None, end)?;
        // Seam blend layers name their seams by object (decision 0293):
        // the pasted texture sets' name the pasted objects.
        for material in &plan.materials {
            let Some(set) = &material.set else {
                continue;
            };
            if let Some(renumbered) = with_seams_renumbered(set, &new) {
                self.edit(Edit::SetTextureSet {
                    material: material.to.clone(),
                    before: Some(set.clone()),
                    after: Some(Arc::new(renumbered)),
                })?;
            }
        }
        // Levels made by hand: their shapes are inside, pasted too.
        for c in clip.objects() {
            if c.object.lods.levels.iter().all(|l| l.hand.is_none()) {
                continue;
            }
            let id = new[&c.id];
            let Some(object) = self.scene.object(id) else {
                continue;
            };
            let mut lods = object.lods.clone();
            for (level, from) in lods.levels.iter_mut().zip(&c.object.lods.levels) {
                level.hand = from.hand.and_then(|h| {
                    new.get(&h.shape).map(|shape| crate::lod::HandLevel {
                        shape: *shape,
                        made_from: h.made_from,
                    })
                });
            }
            self.replace_lods(id, lods)?;
        }
        Ok(Some(
            clip.objects()
                .iter()
                .filter(|c| c.chosen)
                .map(|c| new[&c.id])
                .collect(),
        ))
    }
}

/// `set` with its Seam blend layers' seams on the objects `new` gives
/// for them (others left as they are, counted missing), or `None` when
/// nothing changes.
fn with_seams_renumbered(
    set: &paint::TextureSet,
    new: &BTreeMap<ObjectId, ObjectId>,
) -> Option<paint::TextureSet> {
    fn walk(layers: &mut [paint::Layer], new: &BTreeMap<ObjectId, ObjectId>) -> bool {
        let mut changed = false;
        for layer in layers {
            if let Some(blend) = &mut layer.seam_blend {
                let seams: Vec<paint::SeamId> = blend
                    .seams
                    .iter()
                    .map(|s| paint::SeamId {
                        object: new
                            .get(&ObjectId::from_raw(s.object))
                            .map_or(s.object, |id| id.get()),
                        points: s.points,
                    })
                    .collect();
                if seams != blend.seams {
                    blend.mark(seams, false);
                    blend.made = 0;
                    changed = true;
                }
            }
            if let Some(children) = layer.children_mut() {
                changed |= walk(children, new);
            }
        }
        changed
    }
    let mut out = set.clone();
    walk(&mut out.layers, new).then_some(out)
}
