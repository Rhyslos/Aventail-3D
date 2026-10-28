//! What the views show of the project's painting ([decision 0177](../../../docs/decisions/0177-texture-painter-plan.md)):
//! each texture set's layers mixed into one image, kept up to date tile by
//! tile, and a key naming each texture set for the GPU.

use paint::{Channel, Composite, Image, PaintStore, TextureSet};
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
    /// With the realism channels (decision 0207): roughness in red and
    /// metal in green over their standard values, opaque, and the glow.
    /// Unchanged tiles are shared as for the colour.
    pub realism: Option<RealismMaps>,
}

/// A texture set's realism channels as the views get them (decision
/// 0207).
#[derive(Debug, Clone, PartialEq)]
pub struct RealismMaps {
    /// Roughness (red) and metal (green), opaque.
    pub surface: Image,
    /// The light given off (sRGB, alpha where it glows).
    pub glow: Image,
}

/// The realism channels' mixes of one texture set.
#[derive(Debug)]
struct ChannelMixes {
    roughness: Composite,
    metal: Composite,
    glow: Composite,
    height: Composite,
    surface: Image,
}

impl ChannelMixes {
    fn of(set: &TextureSet) -> Option<Self> {
        let mix = |c| Composite::of(&paint::channel::channel_view(set, c));
        let mut out = Self {
            roughness: mix(Channel::Roughness)?,
            metal: mix(Channel::Metal)?,
            glow: mix(Channel::Glow)?,
            height: mix(Channel::Height)?,
            surface: Image::new(set.size)?,
        };
        for index in 0..out.surface.tile_count() {
            let tile = out.packed(index);
            out.surface.set_tile(index, Some(tile));
        }
        Some(out)
    }

    /// Brought up to date with `set`: only the changed tiles packed again.
    fn update(&mut self, set: &TextureSet) {
        let view = |c| paint::channel::channel_view(set, c);
        let mut changed = self.roughness.update(&view(Channel::Roughness));
        changed.extend(self.metal.update(&view(Channel::Metal)));
        changed.extend(self.height.update(&view(Channel::Height)));
        self.glow.update(&view(Channel::Glow));
        changed.sort_unstable();
        changed.dedup();
        for index in changed {
            let tile = self.packed(index);
            self.surface.set_tile(index, Some(tile));
        }
    }

    fn maps(&self) -> RealismMaps {
        RealismMaps {
            surface: self.surface.clone(),
            glow: self.glow.image().clone(),
        }
    }

    /// Tile `index` of the roughness, metal and height mixes packed into
    /// one: each over its standard value, roughness in red, metal in
    /// green, height in blue (decision 0211).
    fn packed(&self, index: usize) -> paint::Tile {
        let rough = self.roughness.image().tile(index);
        let metal = self.metal.image().tile(index);
        let height = self.height.image().tile(index);
        let mut out = vec![0u8; paint::TILE_BYTES];
        for (i, texel) in out.chunks_exact_mut(4).enumerate() {
            let at = |tile: Option<&paint::Tile>| {
                tile.map_or([0; 4], |t| {
                    [t[i * 4], t[i * 4 + 1], t[i * 4 + 2], t[i * 4 + 3]]
                })
            };
            let r = paint::channel::over_standard(Channel::Roughness, at(rough));
            let m = paint::channel::over_standard(Channel::Metal, at(metal));
            let h = paint::channel::over_standard(Channel::Height, at(height));
            texel.copy_from_slice(&[r[0], m[0], h[0], 255]);
        }
        Arc::new(out)
    }
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
    /// Its realism channels' mixes, while it uses them.
    channels: Option<ChannelMixes>,
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
                        match (&mut shown.channels, uses_realism(set)) {
                            (Some(mixes), true) => mixes.update(set),
                            (None, true) => shown.channels = ChannelMixes::of(set),
                            (_, false) => shown.channels = None,
                        }
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
                        channels: uses_realism(set).then(|| ChannelMixes::of(set)).flatten(),
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
                realism: shown.channels.as_ref().map(ChannelMixes::maps),
            });
        }
        out
    }
}

/// Whether `set`'s realism channels are shown: it uses them, or a layer
/// changes one.
fn uses_realism(set: &TextureSet) -> bool {
    set.realism || paint::channel::any_channels(&set.layers)
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
        assert!(second[0].realism.is_none(), "no realism channels");
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

    #[test]
    fn realism_channels_are_packed_over_their_standard_values() {
        let mut set = TextureSet::new(128).unwrap();
        set.realism = true;
        let mut metal = Image::new(128).unwrap();
        metal.set_pixel(3, 3, [255, 255, 255, 255]);
        let mut layer = Layer::paint("Steel", Image::new(128).unwrap());
        layer.channels.metal = Some(paint::ChannelPart::Paint(metal));
        set.layers.push(layer);
        let mut store = PaintStore::default();
        store.put("Steel", Some(Arc::new(set.clone())));
        let mut composites = Composites::default();
        let shown = composites.update(&store, &Guides::default());
        let maps = shown[0].realism.as_ref().unwrap();
        assert_eq!(maps.surface.pixel(3, 3), [204, 255, 128, 255]);
        assert_eq!(maps.surface.pixel(0, 0), [204, 0, 128, 255]);
        assert_eq!(maps.glow.pixel(0, 0)[3], 0);
        // A change to one tile: the others are shared.
        if let Some(paint::ChannelPart::Paint(image)) = &mut set.layers[0].channels.metal {
            image.set_pixel(100, 100, [128, 128, 128, 255]);
        }
        store.put("Steel", Some(Arc::new(set)));
        let again = composites.update(&store, &Guides::default());
        let after = again[0].realism.as_ref().unwrap();
        assert!(Arc::ptr_eq(
            maps.surface.tile(0).unwrap(),
            after.surface.tile(0).unwrap()
        ));
        assert_eq!(after.surface.pixel(100, 100)[1], 128);
    }
}
