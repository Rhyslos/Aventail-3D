//! Working a graph out on a model, for the painter (Phase 12, milestone 5,
//! [decision 0261](../../../docs/decisions/0261-graphs-in-the-painter.md)).
//!
//! In the Material mode a graph makes one seamless square that repeats.
//! In the painter it is laid on a model, and the boxes in the **From the
//! model** group bring in the model's own baked maps (its curvature,
//! occlusion, thickness, which way it faces, where on it), so a graph can
//! put moss in the crevices or lighten the edges of the model being
//! painted.
//!
//! The model's maps cover its whole texture square, laid out by its UVs;
//! they don't repeat. So a graph runs in two places at once:
//!
//! - boxes that nothing from the model feeds make the material's repeat,
//!   as always, at [`OnModel::tile_size`];
//! - a box from the model, and every box it feeds, works on the model's
//!   texture square at the texture's size. Where such a box takes a
//!   repeat from the first kind, the repeat is laid across the square
//!   [`OnModel::repeats`] times first ([`Value::repeated`]), the way the
//!   Material mode's preview lays it, so the two agree.
//!
//! Lengths in centimetres stay right in both: a box on the model's square
//! is told the square covers `repeats` times the graph's real-world size.

use crate::graph::Graph;
use crate::image::{Grey, Size};
use crate::registry::{Group, Registry};
use crate::value::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// One of the model's maps a graph can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModelMap {
    /// Outer edges light, creases dark, flat 0.5.
    Curvature,
    /// How open the surface is: 1 open to the sky, darker in crevices.
    Occlusion,
    /// How thin the model is there: 1 thin, 0 thick.
    Thickness,
    /// How much the surface faces up: 1 straight up, 0 sideways or down.
    FacingUp,
    /// How high up the model: 0 at its bottom, 1 at its top.
    Up,
    /// Where across the model, left to right: 0 to 1.
    Across,
    /// Where from front to back: 0 to 1.
    FrontToBack,
}

impl ModelMap {
    /// Every map, in order.
    pub const ALL: [ModelMap; 7] = [
        ModelMap::Curvature,
        ModelMap::Occlusion,
        ModelMap::Thickness,
        ModelMap::FacingUp,
        ModelMap::Up,
        ModelMap::Across,
        ModelMap::FrontToBack,
    ];

    /// The grey a texel no polygon covers takes, where nothing tells:
    /// flat, open, middling.
    pub fn neutral(self) -> f32 {
        match self {
            ModelMap::Curvature | ModelMap::Thickness => 0.5,
            ModelMap::Occlusion => 1.0,
            ModelMap::FacingUp => 0.0,
            ModelMap::Up | ModelMap::Across | ModelMap::FrontToBack => 0.5,
        }
    }
}

/// The model's maps, each a grey picture over its whole texture square
/// (row 0 the top of the texture), all one size.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelMaps {
    size: Size,
    key: u64,
    maps: BTreeMap<ModelMap, Arc<Grey>>,
}

impl ModelMaps {
    /// No maps yet, at `size`. `key` tells these maps apart from others
    /// (the model's shape and UVs and the texture size, hashed by the
    /// caller): boxes using them are worked out again when it changes.
    pub fn new(size: Size, key: u64) -> Self {
        Self {
            size,
            key,
            maps: BTreeMap::new(),
        }
    }

    /// Adds map `map`; refused (`false`) when it isn't this size.
    pub fn insert(&mut self, map: ModelMap, image: Grey) -> bool {
        if image.size() != self.size.get() {
            return false;
        }
        self.maps.insert(map, Arc::new(image));
        true
    }

    /// Map `map`, if it was made.
    pub fn get(&self, map: ModelMap) -> Option<&Arc<Grey>> {
        self.maps.get(&map)
    }

    /// Their size: the texture's.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The key given when made.
    pub fn key(&self) -> u64 {
        self.key
    }

    /// How much memory they take, in bytes.
    pub fn memory_size(&self) -> usize {
        self.maps.values().map(|m| m.memory_size()).sum()
    }
}

/// Where a graph is worked out in the painter: the model's maps, and how
/// many times the material's repeat goes across its texture square.
#[derive(Debug, Clone)]
pub struct OnModel {
    /// The model's maps.
    pub maps: Arc<ModelMaps>,
    /// Repeats across the texture square at the graph's real-world size:
    /// from the model's size, its UVs and the graph's real-world size
    /// (any number above 0).
    pub repeats: f32,
    /// The layer's Tiling ([decision 0270](../../../docs/decisions/0270-tiling-and-moving-colours.md)):
    /// how many times more often the repeat is laid across the square (1
    /// as made). It makes the repeating parts smaller without changing
    /// the graph; the boxes following the model keep the model's own
    /// size, so their lengths stay real centimetres on it.
    pub tiling: f32,
}

impl OnModel {
    /// The model's maps, with the repeat laid `repeats` times across its
    /// square at Tiling 1.
    pub fn new(maps: Arc<ModelMaps>, repeats: f32) -> Self {
        Self {
            maps,
            repeats,
            tiling: 1.0,
        }
    }

    /// The same with Tiling `tiling`.
    pub fn with_tiling(mut self, tiling: f32) -> Self {
        self.tiling = tiling;
        self
    }

    /// Repeats across the texture square as laid: `repeats` times
    /// `tiling`, each kept to a usable number (real and above 0; 1 if
    /// not).
    pub fn repeats(&self) -> f32 {
        self.at_size() * usable(self.tiling)
    }

    /// Repeats across the square at the graph's real-world size, Tiling
    /// aside: kept usable as [`Self::repeats`].
    fn at_size(&self) -> f32 {
        usable(self.repeats)
    }

    /// The size the material's repeat is made at: about as many pixels as
    /// one repeat covers on the texture, a power of two, never more than
    /// twice the texture (a repeat bigger than the texture shows only a
    /// part) and never more than 2048.
    pub fn tile_size(&self) -> Size {
        let texture = self.maps.size.get();
        let wanted = (texture as f32 / self.repeats()).round().max(1.0) as u32;
        let most = (texture * 2).min(2048);
        Size::nearest(wanted.clamp(Size::MIN, most.max(Size::MIN)))
    }

    /// How many centimetres the texture square covers on the model, for a
    /// graph whose repeat covers `real_size_cm`. Tiling doesn't change
    /// it: the model is as big as it was.
    pub fn square_cm(&self, real_size_cm: f32) -> f32 {
        real_size_cm * self.at_size()
    }
}

/// `n` if real and above 0, else 1.
fn usable(n: f32) -> f32 {
    if n.is_finite() && n > 0.0 { n } else { 1.0 }
}

/// Whether box kind `kind` brings in the model's maps.
pub fn is_from_model(registry: &Registry, kind: &str) -> bool {
    registry
        .get(kind)
        .is_some_and(|t| t.info().group == Group::Model)
}

/// Whether `graph` uses the model at all: then it follows the model it is
/// painted on (a "smart material").
pub fn uses_model(registry: &Registry, graph: &Graph) -> bool {
    graph.boxes().any(|(_, b)| is_from_model(registry, &b.kind))
}

/// The model's maps `graph` needs, from its boxes in the From the model
/// group.
pub fn maps_needed(registry: &Registry, graph: &Graph) -> BTreeSet<ModelMap> {
    graph
        .boxes()
        .filter(|(_, b)| is_from_model(registry, &b.kind))
        .flat_map(|(_, b)| crate::boxes::from_model::maps_of(&b.kind).iter().copied())
        .collect()
}

impl Value {
    /// This value repeated `repeats` times across a picture `size` pixels
    /// across, each pixel blended between the four nearest of the repeat
    /// (wrapping round), on up to `threads` cores: a material's repeat
    /// laid across a model's texture square.
    pub fn repeated(&self, repeats: f32, size: Size, threads: usize) -> Value {
        let step = repeats / size.get() as f32;
        let place = move |x: u32, y: u32| ((x as f32 + 0.5) * step, (y as f32 + 0.5) * step);
        let grey = |g: &Grey| -> Arc<Grey> {
            Arc::new(crate::parallel::fill(size, threads, |x, y| {
                let (u, v) = place(x, y);
                g.sample(u, v)
            }))
        };
        let rgb = |c: &crate::image::Rgb| -> Arc<crate::image::Rgb> {
            Arc::new(crate::parallel::fill(size, threads, |x, y| {
                let (u, v) = place(x, y);
                c.sample_rgb(u, v)
            }))
        };
        match self {
            Value::Shape(g) => Value::Shape(grey(g)),
            Value::Height(g) => Value::Height(grey(g)),
            Value::Colour(c) => Value::Colour(rgb(c)),
            Value::Material(m) => Value::Material(Arc::new(crate::value::MaterialMaps {
                colour: rgb(&m.colour),
                height: grey(&m.height),
                roughness: grey(&m.roughness),
                metal: grey(&m.metal),
                normal: rgb(&m.normal),
                occlusion: grey(&m.occlusion),
                curvature: grey(&m.curvature),
            })),
        }
    }
}

#[cfg(test)]
mod tests;
