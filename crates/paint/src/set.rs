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

/// The deepest groups may nest.
pub const MAX_DEPTH: usize = 8;

pub use crate::blend::Blend;

/// What a layer holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// Painted pixels.
    Paint(Image),
    /// One colour over the whole texture (a fill layer), sRGB with alpha:
    /// shaped by clipping it to the layer below (and by masks, later).
    Fill([u8; 4]),
    /// A group: its layers, bottom first, mixed together and then laid on
    /// as one layer.
    Group(Vec<Layer>),
}

/// One dab of a stroke as painted, kept to paint it again later (at
/// another texture size, or after the shape changed): in the shape's own
/// space.
///
/// A dab painted flat in the UV Editor (decision 0180) has no way towards
/// a camera: its `toward` is zero, `at` holds its place on the texture
/// (U, V, 0) and `radius` is in texture widths. See [`DabRecord::flat`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DabRecord {
    /// Its middle on the surface.
    pub at: glam::Vec3,
    /// The way towards the camera there (unit length), or zero for a dab
    /// painted flat on the texture.
    pub toward: glam::Vec3,
    /// Its radius on the surface (in texture widths for a flat dab).
    pub radius: f32,
}

impl DabRecord {
    /// A dab painted flat on the texture at `uv`, `radius` texture widths
    /// across.
    pub fn flat(uv: glam::Vec2, radius: f32) -> Self {
        Self {
            at: uv.extend(0.0),
            toward: glam::Vec3::ZERO,
            radius,
        }
    }

    /// Where it was on the texture, if it was painted flat there.
    pub fn flat_uv(&self) -> Option<glam::Vec2> {
        (self.toward == glam::Vec3::ZERO).then(|| self.at.truncate())
    }
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
    /// Whether it shows only where the layer below it (the nearest
    /// unclipped one) is painted: clipped to the layer below.
    pub clipped: bool,
    /// For a group, whether the Paint layers panel shows its layers.
    pub expanded: bool,
    /// What it holds.
    pub content: Content,
    /// The strokes painted on it, oldest first (decision 0179), kept to
    /// paint them again later. Shared, so copying a layer is cheap.
    pub strokes: Vec<std::sync::Arc<StrokeRecord>>,
}

impl Layer {
    /// A visible paint layer called `name` holding `image`, fully opaque.
    pub fn paint(name: impl Into<String>, image: Image) -> Self {
        Self::holding(name, Content::Paint(image))
    }

    /// A visible fill layer called `name` of colour `color`.
    pub fn fill(name: impl Into<String>, color: [u8; 4]) -> Self {
        Self::holding(name, Content::Fill(color))
    }

    /// A visible, open group called `name` holding `layers` (bottom
    /// first).
    pub fn group(name: impl Into<String>, layers: Vec<Layer>) -> Self {
        Self::holding(name, Content::Group(layers))
    }

    fn holding(name: impl Into<String>, content: Content) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: Blend::Normal,
            clipped: false,
            expanded: true,
            content,
            strokes: Vec::new(),
        }
    }

    /// Its painted pixels, for a paint layer.
    pub fn image(&self) -> Option<&Image> {
        match &self.content {
            Content::Paint(image) => Some(image),
            _ => None,
        }
    }

    /// Its painted pixels to change, for a paint layer.
    pub fn image_mut(&mut self) -> Option<&mut Image> {
        match &mut self.content {
            Content::Paint(image) => Some(image),
            _ => None,
        }
    }

    /// Its layers, for a group (bottom first).
    pub fn children(&self) -> Option<&Vec<Layer>> {
        match &self.content {
            Content::Group(layers) => Some(layers),
            _ => None,
        }
    }

    /// Its layers to change, for a group.
    pub fn children_mut(&mut self) -> Option<&mut Vec<Layer>> {
        match &mut self.content {
            Content::Group(layers) => Some(layers),
            _ => None,
        }
    }

    /// How many layers it is: one, and a group's layers too.
    pub fn count(&self) -> usize {
        1 + self
            .children()
            .map_or(0, |c| c.iter().map(Layer::count).sum())
    }

    /// How many levels of groups it holds (0 for a layer that isn't a
    /// group).
    pub fn depth(&self) -> usize {
        self.children()
            .map_or(0, |c| 1 + c.iter().map(Layer::depth).max().unwrap_or(0))
    }

    /// Whether its numbers are real and in range and its name is
    /// reasonable (its layers too, for a group).
    pub fn is_valid(&self) -> bool {
        self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
            && self.name.chars().count() <= MAX_NAME
            && self.strokes.iter().all(|s| s.is_valid())
            && self
                .children()
                .is_none_or(|c| c.iter().all(Layer::is_valid))
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
                Content::Fill(_) => 0,
                Content::Group(layers) => layers.iter().map(Layer::memory_size).sum(),
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

    /// Whether it is sound: a size in range, not too many layers or
    /// groups too deep, each valid and with images of the set's size.
    pub fn is_valid(&self) -> bool {
        fn sized(layer: &Layer, size: u32) -> bool {
            layer.image().is_none_or(|i| i.size() == size)
                && layer
                    .children()
                    .is_none_or(|c| c.iter().all(|l| sized(l, size)))
        }
        crate::image::SIZES.contains(&self.size)
            && self.layers.iter().map(Layer::count).sum::<usize>() <= MAX_LAYERS
            && self.layers.iter().all(|layer| {
                layer.is_valid() && layer.depth() < MAX_DEPTH && sized(layer, self.size)
            })
    }

    /// Roughly how many bytes it takes.
    pub fn memory_size(&self) -> usize {
        size_of::<Self>() + self.layers.iter().map(Layer::memory_size).sum::<usize>()
    }

    /// A name for a new layer: `base`, or `base 2`, `base 3`... if taken.
    pub fn free_name(&self, base: &str) -> String {
        let listed = self.listed(true);
        let taken = |name: &str| {
            listed
                .iter()
                .any(|(path, _)| self.layer(path).is_some_and(|l| l.name == name))
        };
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
    fn a_flat_dab_keeps_its_place_on_the_texture() {
        let dab = DabRecord::flat(glam::Vec2::new(0.25, 0.75), 0.01);
        assert_eq!(dab.flat_uv(), Some(glam::Vec2::new(0.25, 0.75)));
        let on_model = DabRecord {
            at: glam::Vec3::ONE,
            toward: glam::Vec3::Z,
            radius: 0.1,
        };
        assert_eq!(on_model.flat_uv(), None);
    }

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
