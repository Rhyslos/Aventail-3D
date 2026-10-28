//! Texture sets and their layers ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)).
//!
//! A **texture set** is the textures one material is painted with. It
//! belongs to a material by name: materials with the same name are one
//! material in an export, across objects, so they share one texture set.
//! Its **layers** are mixed bottom first into what the surface shows.

use crate::image::Image;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The most layers a texture set may have.
pub const MAX_LAYERS: usize = 256;

/// The most characters in a layer's name.
pub const MAX_NAME: usize = 200;

/// How a layer mixes onto the layers under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blend {
    /// Over what is under it, by its own coverage.
    #[default]
    Normal,
}

/// What a layer holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// Painted pixels.
    Paint(Image),
}

/// One dab of a stroke as painted, kept to paint it again later (at
/// another texture size, or after the shape changed): in the shape's own
/// space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabRecord {
    /// Its middle on the surface.
    pub at: glam::Vec3,
    /// The way towards the camera there (unit length).
    pub toward: glam::Vec3,
    /// Its radius on the surface.
    pub radius: f32,
}

/// One stroke as painted (decision 0179): the brush and its dabs.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeRecord {
    /// Paint or erase.
    pub kind: crate::BrushKind,
    /// Its colour (sRGB, with alpha).
    pub color: [u8; 4],
    /// Its hardness, 0 to 1.
    pub hardness: f32,
    /// Its flow, 0 to 1.
    pub flow: f32,
    /// Whether it painted hidden parts too.
    pub through: bool,
    /// Its dabs, in order.
    pub dabs: Vec<DabRecord>,
}

impl StrokeRecord {
    /// Whether its numbers are real and in range.
    pub fn is_valid(&self) -> bool {
        (0.0..=1.0).contains(&self.hardness)
            && (0.0..=1.0).contains(&self.flow)
            && self.dabs.iter().all(|d| {
                d.at.is_finite() && d.toward.is_finite() && d.radius.is_finite() && d.radius >= 0.0
            })
    }
}

/// One layer of a texture set.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Its name, as the Paint layers panel shows it.
    pub name: String,
    /// Whether it is mixed in (a hidden layer keeps its pixels).
    pub visible: bool,
    /// How much of it shows, 0 to 1.
    pub opacity: f32,
    /// How it mixes onto the layers under it.
    pub blend: Blend,
    /// What it holds.
    pub content: Content,
    /// The strokes painted on it, oldest first (decision 0179), kept to
    /// paint them again later. Shared, so copying a layer is cheap.
    pub strokes: Vec<std::sync::Arc<StrokeRecord>>,
}

impl Layer {
    /// A visible paint layer called `name` holding `image`, fully opaque.
    pub fn paint(name: impl Into<String>, image: Image) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: Blend::Normal,
            content: Content::Paint(image),
            strokes: Vec::new(),
        }
    }

    /// Its painted pixels, for a paint layer.
    pub fn image(&self) -> Option<&Image> {
        match &self.content {
            Content::Paint(image) => Some(image),
        }
    }

    /// Its painted pixels to change, for a paint layer.
    pub fn image_mut(&mut self) -> Option<&mut Image> {
        match &mut self.content {
            Content::Paint(image) => Some(image),
        }
    }

    /// Whether its numbers are real and in range and its name is
    /// reasonable.
    pub fn is_valid(&self) -> bool {
        self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
            && self.name.chars().count() <= MAX_NAME
            && self.strokes.iter().all(|s| s.is_valid())
    }

    /// Roughly how many bytes it takes.
    pub fn memory_size(&self) -> usize {
        size_of::<Self>()
            + self.name.len()
            + self
                .strokes
                .iter()
                .map(|s| size_of::<StrokeRecord>() + s.dabs.len() * size_of::<DabRecord>())
                .sum::<usize>()
            + match &self.content {
                Content::Paint(image) => image.memory_size(),
            }
    }
}

/// The textures one material is painted with.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureSet {
    /// Pixels across its textures.
    pub size: u32,
    /// Its layers, bottom first.
    pub layers: Vec<Layer>,
}

impl TextureSet {
    /// An empty texture set `size` pixels across, or `None` for a size
    /// outside [`crate::image::SIZES`].
    pub fn new(size: u32) -> Option<Self> {
        crate::image::SIZES.contains(&size).then(|| Self {
            size,
            layers: Vec::new(),
        })
    }

    /// Whether it is sound: a size in range, not too many layers, each
    /// valid and with images of the set's size.
    pub fn is_valid(&self) -> bool {
        crate::image::SIZES.contains(&self.size)
            && self.layers.len() <= MAX_LAYERS
            && self.layers.iter().all(|layer| {
                layer.is_valid() && layer.image().is_none_or(|i| i.size() == self.size)
            })
    }

    /// Roughly how many bytes it takes.
    pub fn memory_size(&self) -> usize {
        size_of::<Self>() + self.layers.iter().map(Layer::memory_size).sum::<usize>()
    }

    /// A name for a new layer: `base`, or `base 2`, `base 3`... if taken.
    pub fn free_name(&self, base: &str) -> String {
        let taken = |name: &str| self.layers.iter().any(|l| l.name == name);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|name| !taken(name))
            .unwrap_or_else(|| base.to_string())
    }
}

/// Every texture set in a project, by material name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PaintStore {
    sets: BTreeMap<String, Arc<TextureSet>>,
}

impl PaintStore {
    /// The texture set of the material called `material`, if it has one.
    pub fn set(&self, material: &str) -> Option<&Arc<TextureSet>> {
        self.sets.get(material)
    }

    /// Every texture set, by material name.
    pub fn sets(&self) -> impl Iterator<Item = (&str, &Arc<TextureSet>)> {
        self.sets.iter().map(|(name, set)| (name.as_str(), set))
    }

    /// Gives `material` the texture set `set`, or none (`None`). Returns
    /// what it had.
    pub fn put(&mut self, material: &str, set: Option<Arc<TextureSet>>) -> Option<Arc<TextureSet>> {
        match set {
            Some(set) => self.sets.insert(material.to_string(), set),
            None => self.sets.remove(material),
        }
    }

    /// Whether no material has a texture set.
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// Roughly how many bytes it takes.
    pub fn memory_size(&self) -> usize {
        self.sets
            .iter()
            .map(|(name, set)| name.len() + set.memory_size())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_checks_its_layers() {
        let mut set = TextureSet::new(128).unwrap();
        assert!(set.is_valid());
        set.layers
            .push(Layer::paint("Paint", Image::new(128).unwrap()));
        assert!(set.is_valid());
        set.layers
            .push(Layer::paint("Wrong size", Image::new(64).unwrap()));
        assert!(!set.is_valid());
        set.layers.pop();
        set.layers[0].opacity = f32::NAN;
        assert!(!set.is_valid());
        set.layers[0].opacity = 1.5;
        assert!(!set.is_valid());
    }

    #[test]
    fn free_names_count_up() {
        let mut set = TextureSet::new(64).unwrap();
        assert_eq!(set.free_name("Image"), "Image");
        set.layers
            .push(Layer::paint("Image", Image::new(64).unwrap()));
        assert_eq!(set.free_name("Image"), "Image 2");
        set.layers
            .push(Layer::paint("Image 2", Image::new(64).unwrap()));
        assert_eq!(set.free_name("Image"), "Image 3");
    }

    #[test]
    fn the_store_keeps_one_set_per_material() {
        let mut store = PaintStore::default();
        assert!(store.is_empty());
        let set = Arc::new(TextureSet::new(64).unwrap());
        assert!(store.put("Wood", Some(set.clone())).is_none());
        assert!(store.put("Wood", Some(set)).is_some());
        assert_eq!(store.sets().count(), 1);
        assert!(store.set("Wood").is_some());
        assert!(store.put("Wood", None).is_some());
        assert!(store.is_empty());
    }

    #[test]
    fn sizes_outside_the_range_make_no_set() {
        assert!(TextureSet::new(1).is_none());
    }
}
