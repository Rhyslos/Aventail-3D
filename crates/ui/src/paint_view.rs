//! What the views show of the project's painting ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! each texture set's layers mixed into one image, kept up to date tile by
//! tile, and a key naming each texture set for the GPU.

use paint::{Composite, Image, PaintStore, TextureSet};
use std::collections::HashMap;
use std::sync::Arc;

/// A texture set's mixed image, as the views show it.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintedTexture {
    /// Its key for the GPU ([`texture_key`]).
    pub key: u64,
    /// The material it belongs to.
    pub material: String,
    /// Its layers mixed. Unchanged tiles are shared with the last image,
    /// so a renderer can upload only the tiles that aren't.
    pub image: Image,
}

/// The key naming the texture set of material `material`: the same name
/// always gives the same key, and never 0 (which means "no texture").
pub fn texture_key(material: &str) -> u64 {
    // FNV-1a: small, and the same on every run.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in material.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash.max(1)
}

/// A baked map laid over a texture set's paint in the views, as a guide
/// (decision 0193).
#[derive(Debug, Clone)]
pub(crate) struct Guide {
    /// The map.
    pub(crate) image: Image,
    /// How strongly it shows, 0 to 1.
    pub(crate) opacity: f32,
    /// How it mixes with the paint.
    pub(crate) blend: paint::Blend,
}

/// The guides shown, by material, and a number that goes up whenever
/// they change (so the views know to show them again).
#[derive(Debug, Clone, Default)]
pub(crate) struct Guides {
    pub(crate) revision: u64,
    pub(crate) by_material: HashMap<String, Guide>,
}

/// A texture set's mix as the views get it.
#[derive(Debug)]
struct Shown {
    set: Arc<TextureSet>,
    composite: Composite,
    /// With its guide laid over, and the guides' revision it was made for.
    guided: Option<(u64, Image)>,
}

/// Every texture set's mix, made again only where its layers changed.
#[derive(Debug, Default)]
pub(crate) struct Composites {
    sets: HashMap<String, Shown>,
}

impl Composites {
    /// The mixed images of `paint`'s texture sets, updated from what they
    /// were, with `guides` laid over.
    pub(crate) fn update(&mut self, paint: &PaintStore, guides: &Guides) -> Vec<PaintedTexture> {
        self.sets
            .retain(|material, _| paint.set(material).is_some());
        let mut out = Vec::new();
        for (material, set) in paint.sets() {
            let mut changed: Option<Vec<usize>> = Some(Vec::new());
            let shown = match self.sets.get_mut(material) {
                Some(shown) => {
                    if !Arc::ptr_eq(&shown.set, set) {
                        changed = Some(shown.composite.update(set));
                        shown.set = Arc::clone(set);
                    }
                    shown
                }
                None => {
                    let Some(composite) = Composite::of(set) else {
                        continue;
                    };
                    changed = None;
                    self.sets.entry(material.to_string()).or_insert(Shown {
                        set: Arc::clone(set),
                        composite,
                        guided: None,
                    })
                }
            };
            let image = match guides.by_material.get(material) {
                Some(guide) if guide.image.size() == shown.composite.image().size() => {
                    let base = shown.composite.image();
                    let fresh = !matches!(&shown.guided, Some((r, _)) if *r == guides.revision);
                    let tiles: Vec<usize> = match (&changed, fresh) {
                        (Some(tiles), false) => tiles.clone(),
                        _ => (0..base.tile_count()).collect(),
                    };
                    let mut guided = match shown.guided.take() {
                        Some((_, image)) if !fresh => image,
                        _ => base.clone(),
                    };
                    for index in tiles {
                        guided.set_tile(index, guide_tile(base, guide, index));
                    }
                    shown.guided = Some((guides.revision, guided.clone()));
                    guided
                }
                _ => {
                    shown.guided = None;
                    shown.composite.image().clone()
                }
            };
            out.push(PaintedTexture {
                key: texture_key(material),
                material: material.to_string(),
                image,
            });
        }
        out
    }
}

/// Tile `index` of `base` with `guide` laid over.
fn guide_tile(base: &Image, guide: &Guide, index: usize) -> Option<paint::Tile> {
    let Some(over) = guide.image.tile(index) else {
        return base.tile(index).cloned();
    };
    let mut pixels = match base.tile(index) {
        Some(tile) => tile.to_vec(),
        None => vec![0; paint::TILE_BYTES],
    };
    for (dst, src) in pixels.chunks_exact_mut(4).zip(over.chunks_exact(4)) {
        // Where the guide covers the surface (unpainted parts get it over
        // the material's own colour).
        if src[3] == 0 {
            continue;
        }
        paint::blend::over(dst, src, guide.opacity.clamp(0.0, 1.0), guide.blend);
    }
    Some(Arc::new(pixels))
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint::Layer;

    #[test]
    fn keys_are_steady_and_never_zero() {
        assert_eq!(texture_key("Wood"), texture_key("Wood"));
        assert_ne!(texture_key("Wood"), texture_key("Metal"));
        assert_ne!(texture_key(""), 0);
    }

    #[test]
    fn composites_follow_the_store_and_share_unchanged_tiles() {
        let mut image = Image::new(128).unwrap();
        image.set_pixel(0, 0, [255; 4]);
        let mut set = TextureSet::new(128).unwrap();
        set.layers.push(Layer::paint("Paint", image));
        let mut store = PaintStore::default();
        store.put("Wood", Some(Arc::new(set.clone())));
        let mut composites = Composites::default();
        let first = composites.update(&store, &Guides::default());
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].image.pixel(0, 0), [255; 4]);
        // A change to one tile: the other tiles are the same tiles.
        set.layers[0]
            .image_mut()
            .unwrap()
            .set_pixel(100, 100, [9; 4]);
        store.put("Wood", Some(Arc::new(set)));
        let second = composites.update(&store, &Guides::default());
        let (a, b) = (&first[0].image, &second[0].image);
        assert!(Arc::ptr_eq(a.tile(0).unwrap(), b.tile(0).unwrap()));
        assert_eq!(b.pixel(100, 100), [9; 4]);
        // Taken away: gone.
        store.put("Wood", None);
        assert!(composites.update(&store, &Guides::default()).is_empty());
    }

    #[test]
    fn a_guide_is_laid_over_the_paint_and_follows_it() {
        let mut image = Image::new(128).unwrap();
        image.set_pixel(0, 0, [200, 200, 200, 255]);
        let mut set = TextureSet::new(128).unwrap();
        set.layers.push(Layer::paint("Paint", image));
        let mut store = PaintStore::default();
        store.put("Wood", Some(Arc::new(set.clone())));
        let mut dark = Image::new(128).unwrap();
        dark.set_pixel(0, 0, [100, 100, 100, 255]);
        dark.set_pixel(1, 0, [100, 100, 100, 255]);
        let mut guides = Guides {
            revision: 1,
            ..Guides::default()
        };
        guides.by_material.insert(
            "Wood".into(),
            Guide {
                image: dark,
                opacity: 1.0,
                blend: paint::Blend::Multiply,
            },
        );
        let mut composites = Composites::default();
        let shown = composites.update(&store, &guides);
        // Multiplied: 200 × 100 / 255.
        assert_eq!(shown[0].image.pixel(0, 0)[0], 78);
        // Unpainted under it: the guide itself, over the material.
        assert_eq!(shown[0].image.pixel(1, 0), [100, 100, 100, 255]);
        // Painting again: the guide stays over the new paint.
        set.layers[0]
            .image_mut()
            .unwrap()
            .set_pixel(0, 0, [255, 255, 255, 255]);
        store.put("Wood", Some(Arc::new(set)));
        let shown = composites.update(&store, &guides);
        assert_eq!(shown[0].image.pixel(0, 0)[0], 100);
        // No guide: the paint alone.
        let shown = composites.update(
            &store,
            &Guides {
                revision: 2,
                ..Guides::default()
            },
        );
        assert_eq!(shown[0].image.pixel(0, 0)[0], 255);
    }
}
