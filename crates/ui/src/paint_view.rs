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

/// Every texture set's mix, made again only where its layers changed.
#[derive(Debug, Default)]
pub(crate) struct Composites {
    sets: HashMap<String, (Arc<TextureSet>, Composite)>,
}

impl Composites {
    /// The mixed images of `paint`'s texture sets, updated from what they
    /// were.
    pub(crate) fn update(&mut self, paint: &PaintStore) -> Vec<PaintedTexture> {
        self.sets
            .retain(|material, _| paint.set(material).is_some());
        let mut out = Vec::new();
        for (material, set) in paint.sets() {
            let entry = match self.sets.get_mut(material) {
                Some((last, composite)) => {
                    if !Arc::ptr_eq(last, set) {
                        composite.update(set);
                        *last = Arc::clone(set);
                    }
                    Some(&*composite)
                }
                None => Composite::of(set).map(|composite| {
                    &self
                        .sets
                        .entry(material.to_string())
                        .or_insert((Arc::clone(set), composite))
                        .1
                }),
            };
            if let Some(composite) = entry {
                out.push(PaintedTexture {
                    key: texture_key(material),
                    material: material.to_string(),
                    image: composite.image().clone(),
                });
            }
        }
        out
    }
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
        let first = composites.update(&store);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].image.pixel(0, 0), [255; 4]);
        // A change to one tile: the other tiles are the same tiles.
        set.layers[0]
            .image_mut()
            .unwrap()
            .set_pixel(100, 100, [9; 4]);
        store.put("Wood", Some(Arc::new(set)));
        let second = composites.update(&store);
        let (a, b) = (&first[0].image, &second[0].image);
        assert!(Arc::ptr_eq(a.tile(0).unwrap(), b.tile(0).unwrap()));
        assert_eq!(b.pixel(100, 100), [9; 4]);
        // Taken away: gone.
        store.put("Wood", None);
        assert!(composites.update(&store).is_empty());
    }
}
