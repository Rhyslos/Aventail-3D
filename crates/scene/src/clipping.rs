//! Objects copied out of one project to paste into another: copy in one
//! tab, paste in another
//! ([decision 0268](../../../docs/decisions/0268-paste-between-tabs.md)).
//!
//! A [`Clipping`] holds its own copy of everything the objects need, so
//! it can be pasted after the project it came from has changed, gone to
//! sleep or closed:
//!
//! - **the objects**, with everything inside them, as they were when
//!   copied: their shapes and recorded edits (UV sets and all), their
//!   booleans, modifiers, live mirrors, levels of detail and materials;
//! - **the objects they need that weren't chosen**: the cutters of their
//!   booleans, the cages, curves and shapes their modifiers read, the
//!   cross-sections of their sweeps (pasted too, as they were: a cutter
//!   stays hidden);
//! - **their materials' texture sets** (the layers, paint and masks),
//! - **the material graphs** those layers' graph fills run, and
//! - **the boxes of your own** those graphs use, with the own boxes
//!   inside them.
//!
//! An instance whose original wasn't copied can't show it in another
//! project, so it becomes a copy of its original's shape (as Bake into
//! shape would leave it, but with the edits still editable).
//!
//! Pasting ([`crate::Command::Paste`]) is one undo step, planned first
//! by [`Clipping::plan`] so nothing clashes with what the project has:
//! objects get the project's next ids (and a numbered name when theirs
//! is taken); a material whose name the project uses for something else
//! gets a numbered name; graphs and own boxes the project already has
//! the same of are reused, others are added with new ids where theirs are
//! taken (`matgraph::own::bringing`).

use crate::object::{Material, Object, ObjectId, ObjectKind, Transform};
use crate::tree::{Scene, SceneError};
use matgraph::own::bringing::{self, Brought};
use matgraph::{Graph, GraphId, OwnBox, Refusal};
use paint::TextureSet;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use words::trf;

/// One object of a [`Clipping`].
#[derive(Debug, Clone, PartialEq)]
pub struct Clipped {
    /// Its id where it came from (other objects of the clipping name it).
    pub id: ObjectId,
    /// Its parent where it came from, when that was copied too; `None`
    /// for an object at the clipping's top.
    pub parent: Option<ObjectId>,
    /// The object. One at the top is placed where it was in the world.
    pub object: Object,
    /// Whether it is one of the objects chosen (at the top of what was
    /// copied); `false` for what came along inside one, or because one
    /// needs it.
    pub chosen: bool,
}

/// Objects copied out of a project with everything they need, to paste
/// into another (see the module's words).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clipping {
    objects: Vec<Clipped>,
    sets: BTreeMap<String, Arc<TextureSet>>,
    graphs: BTreeMap<GraphId, Arc<Graph>>,
    own: Vec<Arc<OwnBox>>,
}

/// The objects `object` reads, by id: its booleans' cutters, its
/// modifiers' sources and its sweep's cross-section.
fn reads(object: &Object) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = object
        .booleans
        .iter()
        .flat_map(|b| b.cutters.iter().copied())
        .collect();
    out.extend(object.modifiers.iter().flat_map(|m| m.kind.sources()));
    if let Some(profile) = object.kind.curve().and_then(|c| c.solid.profile_object()) {
        out.push(profile);
    }
    out
}

/// `id` and everything inside it, parents first.
fn with_inside(scene: &Scene, id: ObjectId, out: &mut Vec<ObjectId>) {
    out.push(id);
    for child in scene.children(Some(id)) {
        with_inside(scene, *child, out);
    }
}

/// `object`, an instance of `source`, as a copy of `source`'s shape: its
/// own name, place, materials and settings, the original's shape, edits,
/// booleans, modifiers and mirror.
fn standalone(object: &Object, source: &Object) -> Object {
    Object {
        kind: source.kind.clone(),
        stack: source.stack.clone(),
        booleans: source.booleans.clone(),
        modifiers: source.modifiers.clone(),
        mirror: source.mirror,
        hardness: source.hardness,
        ..object.clone()
    }
}

/// The names of the materials `object`'s polygons use: its own, or the
/// standard one while it has none. None for a group or a cage.
pub(crate) fn material_names(object: &Object) -> Vec<String> {
    if !object.kind.has_shape() {
        return Vec::new();
    }
    if object.materials.is_empty() {
        vec![Material::standard().name]
    } else {
        object.materials.iter().map(|m| m.name.clone()).collect()
    }
}

/// Every graph fill's graph in `set`'s layers.
fn graphs_of(set: &TextureSet) -> BTreeSet<GraphId> {
    fn walk(layers: &[paint::Layer], out: &mut BTreeSet<GraphId>) {
        for layer in layers {
            if let Some(fill) = &layer.graph {
                out.insert(fill.graph);
            }
            if let Some(children) = layer.children() {
                walk(children, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&set.layers, &mut out);
    out
}

/// `set` with its graph fills running the graphs `ids` gives them (a
/// graph not listed stays); the same set when nothing changes.
fn with_graphs_renamed(set: &Arc<TextureSet>, ids: &BTreeMap<GraphId, GraphId>) -> Arc<TextureSet> {
    fn walk(layers: &mut [paint::Layer], ids: &BTreeMap<GraphId, GraphId>) {
        for layer in layers {
            if let Some(fill) = &mut layer.graph
                && let Some(new) = ids.get(&fill.graph)
            {
                fill.graph = *new;
            }
            if let Some(children) = layer.children_mut() {
                walk(children, ids);
            }
        }
    }
    if graphs_of(set)
        .iter()
        .all(|g| ids.get(g).is_none_or(|n| n == g))
    {
        return set.clone();
    }
    let mut changed = (**set).clone();
    walk(&mut changed.layers, ids);
    Arc::new(changed)
}

/// Whether two materials' texture sets are the same (both none counts).
fn same_set(a: Option<&Arc<TextureSet>>, b: Option<&Arc<TextureSet>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
        _ => false,
    }
}

impl Clipping {
    /// Copies `ids` (with everything inside them) out of `scene`, with
    /// everything they need. `None` when none of them is there.
    pub fn capture(scene: &Scene, ids: &[ObjectId]) -> Option<Self> {
        let present: Vec<ObjectId> = ids.iter().copied().filter(|i| scene.contains(*i)).collect();
        let tops = crate::command::top_level(scene, &present);
        if tops.is_empty() {
            return None;
        }
        let mut order = Vec::new();
        for top in &tops {
            with_inside(scene, *top, &mut order);
        }
        let mut wanted: BTreeSet<ObjectId> = order.iter().copied().collect();
        // What they read, and what that reads, until nothing new comes.
        loop {
            let objects = Self::objects_of(scene, &wanted);
            let more: BTreeSet<ObjectId> = objects
                .values()
                .flat_map(reads)
                .filter(|id| !wanted.contains(id) && scene.contains(*id))
                .collect();
            if more.is_empty() {
                break;
            }
            for id in more {
                let mut inside = Vec::new();
                with_inside(scene, id, &mut inside);
                wanted.extend(inside);
            }
        }
        let mut objects = Self::objects_of(scene, &wanted);
        let chosen: BTreeSet<ObjectId> = tops.iter().copied().collect();
        let mut clipped = Vec::with_capacity(wanted.len());
        for (id, _) in scene.walk() {
            let Some(mut object) = objects.remove(&id) else {
                continue;
            };
            let parent = scene.parent(id).filter(|p| wanted.contains(p));
            if parent.is_none() && scene.parent(id).is_some() {
                // Placed where it was in the world.
                let world = scene.world_matrix(id);
                if let Some(t) = Transform::from_matrix(world) {
                    object.transform = t;
                }
            }
            clipped.push(Clipped {
                id,
                parent,
                object,
                chosen: chosen.contains(&id),
            });
        }
        let names: BTreeSet<String> = clipped
            .iter()
            .flat_map(|c| material_names(&c.object))
            .collect();
        let sets: BTreeMap<String, Arc<TextureSet>> = names
            .into_iter()
            .filter_map(|n| scene.paint().set(&n).map(|s| (n, s.clone())))
            .collect();
        let graphs: BTreeMap<GraphId, Arc<Graph>> = sets
            .values()
            .flat_map(|s| graphs_of(s))
            .filter_map(|g| scene.graphs().get(g).map(|graph| (g, graph.clone())))
            .collect();
        let own = bringing::needed(
            scene.graphs(),
            graphs
                .values()
                .flat_map(|g| g.boxes().map(|(_, b)| b.kind.as_str())),
        );
        Some(Self {
            objects: clipped,
            sets,
            graphs,
            own,
        })
    }

    /// The objects `ids` of `scene`, each instance whose original isn't
    /// among them made a copy of its original's shape.
    fn objects_of(scene: &Scene, ids: &BTreeSet<ObjectId>) -> BTreeMap<ObjectId, Object> {
        ids.iter()
            .filter_map(|id| {
                let object = scene.object(*id)?;
                let object = match object.kind {
                    ObjectKind::Instance(source) if !ids.contains(&source) => scene
                        .object(source)
                        .map_or_else(|| object.clone(), |s| standalone(object, s)),
                    _ => object.clone(),
                };
                Some((*id, object))
            })
            .collect()
    }

    /// Every object, parents before their children.
    pub fn objects(&self) -> &[Clipped] {
        &self.objects
    }

    /// How many objects were chosen (at the top of what was copied).
    pub fn chosen_count(&self) -> usize {
        self.objects.iter().filter(|c| c.chosen).count()
    }

    /// How many objects at the top came along because a chosen one needs
    /// them (a boolean's cutter, a modifier's cage...).
    pub fn needed_count(&self) -> usize {
        self.objects
            .iter()
            .filter(|c| c.parent.is_none() && !c.chosen)
            .count()
    }

    /// The texture sets of the objects' materials, by material name.
    pub fn sets(&self) -> &BTreeMap<String, Arc<TextureSet>> {
        &self.sets
    }

    /// The material graphs their layers run, by id where they came from.
    pub fn graphs(&self) -> &BTreeMap<GraphId, Arc<Graph>> {
        &self.graphs
    }

    /// The boxes of your own those graphs use.
    pub fn own_boxes(&self) -> &[Arc<OwnBox>] {
        &self.own
    }

    /// How pasting it into `scene` fits it in, or why it can't: the own
    /// boxes and graphs to add or reuse, and each material's name there.
    pub fn plan(&self, scene: &Scene) -> Result<PastePlan, SceneError> {
        let refuse = |why: String| SceneError::Graph(Refusal::new(why));
        let store = scene.graphs();
        let own = bringing::bring_in(store, &self.own)
            .map_err(|why| refuse(trf!("Couldn't bring in the boxes of your own: {}.", why)))?;
        let mut known: Vec<Arc<OwnBox>> = store.own_boxes().cloned().collect();
        known.extend(own.add.iter().cloned().map(Arc::new));
        let registry = matgraph::own::registry_with(&known);
        // Graphs: the same one the project has, or a new one with the
        // project's next id.
        let mut graphs = Vec::new();
        let mut next = store.next_id().0;
        let mut names: BTreeSet<String> = store.iter().map(|(_, g)| g.name().to_string()).collect();
        for (from, graph) in &self.graphs {
            let mut parts = graph.to_parts();
            bringing::rename_kinds(&mut parts, &own.ids);
            let same = store.iter().find(|(_, g)| {
                let mut theirs = g.to_parts();
                bringing::same_or_numbered(&theirs.name, &parts.name) && {
                    theirs.name.clone_from(&parts.name);
                    theirs == parts
                }
            });
            if let Some((id, _)) = same {
                graphs.push(PlannedGraph {
                    from: *from,
                    to: id,
                    add: None,
                });
                continue;
            }
            parts.name = bringing::unused_name(&parts.name, |n| names.contains(n));
            names.insert(parts.name.clone());
            let made = Graph::from_parts(parts, &registry).map_err(|why| {
                refuse(trf!(
                    "The material graph {} can't be brought in: {}.",
                    graph.name(),
                    why
                ))
            })?;
            graphs.push(PlannedGraph {
                from: *from,
                to: GraphId(next),
                add: Some(made),
            });
            next = next.saturating_add(1);
        }
        let graph_ids: BTreeMap<GraphId, GraphId> = graphs.iter().map(|g| (g.from, g.to)).collect();
        // Materials: the same name while it means the same there.
        let mut used: BTreeSet<String> = scene
            .walk()
            .iter()
            .filter_map(|(id, _)| scene.object(*id))
            .flat_map(material_names)
            .collect();
        used.extend(scene.paint().sets().map(|(n, _)| n.to_string()));
        let wanted: BTreeSet<String> = self
            .objects
            .iter()
            .flat_map(|c| material_names(&c.object))
            .collect();
        let mut materials = Vec::new();
        let mut picked: BTreeSet<String> = BTreeSet::new();
        for name in wanted {
            let set = self
                .sets
                .get(&name)
                .map(|s| with_graphs_renamed(s, &graph_ids));
            if set.as_ref().is_some_and(|s| !s.is_valid()) {
                return Err(SceneError::Paint(words::text!(
                    "a texture set's size or layers are out of range"
                )));
            }
            let fits = |n: &str| same_set(scene.paint().set(n), set.as_ref());
            let free = !used.contains(&name) && !picked.contains(&name);
            let to = if free || fits(&name) {
                name.clone()
            } else if let Some(n) = used
                .iter()
                .find(|n| bringing::same_or_numbered(n, &name) && fits(n) && !picked.contains(*n))
            {
                n.clone()
            } else {
                bringing::unused_name(&name, |n| used.contains(n) || picked.contains(n))
            };
            picked.insert(to.clone());
            let put = set.filter(|s| !same_set(scene.paint().set(&to), Some(s)));
            materials.push(PlannedMaterial {
                from: name,
                to,
                set: put,
            });
        }
        Ok(PastePlan {
            own,
            graphs,
            materials,
        })
    }
}

/// One material graph of a paste.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedGraph {
    /// Its id where it came from.
    pub from: GraphId,
    /// Its id in the project.
    pub to: GraphId,
    /// The graph to add, or `None` when the project has the same one.
    pub add: Option<Graph>,
}

/// One material of a paste.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedMaterial {
    /// Its name where it came from.
    pub from: String,
    /// Its name in the project.
    pub to: String,
    /// The texture set to give it there, or `None` when it has none or
    /// the project has the same one.
    pub set: Option<Arc<TextureSet>>,
}

/// How a [`Clipping`] fits into a project ([`Clipping::plan`]).
#[derive(Debug, Clone, PartialEq)]
pub struct PastePlan {
    /// The boxes of your own to add, and each one's id there.
    pub own: Brought,
    /// The material graphs.
    pub graphs: Vec<PlannedGraph>,
    /// The materials.
    pub materials: Vec<PlannedMaterial>,
}

impl PastePlan {
    /// The materials given another name, as (where they came from, in
    /// the project).
    pub fn renamed_materials(&self) -> Vec<(&str, &str)> {
        self.materials
            .iter()
            .filter(|m| m.from != m.to)
            .map(|m| (m.from.as_str(), m.to.as_str()))
            .collect()
    }

    /// How many material graphs are added.
    pub fn graphs_added(&self) -> usize {
        self.graphs.iter().filter(|g| g.add.is_some()).count()
    }

    /// How many texture sets are added.
    pub fn sets_added(&self) -> usize {
        self.materials.iter().filter(|m| m.set.is_some()).count()
    }

    /// The name material `name` (where it came from) has in the project.
    pub fn material_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.materials
            .iter()
            .find(|m| m.from == name)
            .map_or(name, |m| m.to.as_str())
    }
}
