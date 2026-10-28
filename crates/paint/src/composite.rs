//! Mixing a texture set's layers into the one image a surface shows.
//!
//! A [`Composite`] keeps the mixed image and what each of its tiles was
//! made from, so after a change only the tiles whose layers changed are
//! mixed again, and the views upload only those.

use crate::image::{Image, TILE_BYTES, Tile};
use crate::set::{Blend, Content, Layer, TextureSet};
use std::sync::Arc;

/// What one layer gave one tile: the tile itself (kept, so the pointer
/// stays a sound identity), and the settings it was mixed with.
#[derive(Clone, PartialEq)]
struct Input {
    tile: Option<Tile>,
    visible: bool,
    opacity: u32,
    blend: Blend,
}

impl Input {
    fn of(layer: &Layer, index: usize) -> Self {
        Self {
            tile: match &layer.content {
                Content::Paint(image) => image.tile(index).cloned(),
            },
            visible: layer.visible,
            opacity: layer.opacity.to_bits(),
            blend: layer.blend,
        }
    }

    fn same(&self, other: &Self) -> bool {
        let same_tile = match (&self.tile, &other.tile) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        same_tile
            && self.visible == other.visible
            && self.opacity == other.opacity
            && self.blend == other.blend
    }
}

/// A texture set's layers mixed into one image, kept up to date tile by
/// tile.
#[derive(Clone)]
pub struct Composite {
    image: Image,
    inputs: Vec<Vec<Input>>,
}

impl std::fmt::Debug for Composite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Composite")
            .field("image", &self.image)
            .finish()
    }
}

impl Composite {
    /// The mix of `set`'s layers.
    pub fn of(set: &TextureSet) -> Option<Self> {
        let mut composite = Self {
            image: Image::new(set.size)?,
            inputs: Vec::new(),
        };
        composite.update(set);
        Some(composite)
    }

    /// The mixed image.
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// Mixes again the tiles whose layers changed in `set`, and returns
    /// their indices. A set of a different size starts afresh (every tile
    /// returned).
    pub fn update(&mut self, set: &TextureSet) -> Vec<usize> {
        if set.size != self.image.size() {
            if let Some(image) = Image::new(set.size) {
                self.image = image;
                self.inputs.clear();
            }
        }
        let count = self.image.tile_count();
        let fresh = self.inputs.len() != count;
        if fresh {
            self.inputs = vec![Vec::new(); count];
        }
        let mut changed = Vec::new();
        for index in 0..count {
            let now: Vec<Input> = set.layers.iter().map(|l| Input::of(l, index)).collect();
            let old = &self.inputs[index];
            let same =
                !fresh && old.len() == now.len() && old.iter().zip(&now).all(|(a, b)| a.same(b));
            if same {
                continue;
            }
            self.image.set_tile(index, mix(&now));
            self.inputs[index] = now;
            changed.push(index);
        }
        changed
    }
}

/// Mixes one tile's layers, bottom first. `None` if nothing shows.
fn mix(inputs: &[Input]) -> Option<Tile> {
    let showing: Vec<&Input> = inputs
        .iter()
        .filter(|i| i.visible && i.tile.is_some() && f32::from_bits(i.opacity) > 0.0)
        .collect();
    match showing.as_slice() {
        [] => return None,
        // One fully opaque normal layer: its tile as it is.
        [only] if f32::from_bits(only.opacity) >= 1.0 && only.blend == Blend::Normal => {
            return only.tile.clone();
        }
        _ => {}
    }
    let mut out = vec![0u8; TILE_BYTES];
    for input in showing {
        let Some(tile) = &input.tile else { continue };
        let opacity = f32::from_bits(input.opacity).clamp(0.0, 1.0);
        for (dst, src) in out.chunks_exact_mut(4).zip(tile.chunks_exact(4)) {
            over(dst, src, opacity, input.blend);
        }
    }
    out.iter()
        .skip(3)
        .step_by(4)
        .any(|a| *a != 0)
        .then(|| Arc::new(out))
}

/// Mixes pixel `src` (straight alpha) onto `dst` with `opacity`.
fn over(dst: &mut [u8], src: &[u8], opacity: f32, blend: Blend) {
    let sa = src[3] as f32 / 255.0 * opacity;
    if sa <= 0.0 {
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    for c in 0..3 {
        let s = match blend {
            Blend::Normal => src[c] as f32,
        };
        let d = dst[c] as f32;
        let value = (s * sa + d * da * (1.0 - sa)) / out_a;
        dst[c] = value.round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(size: u32, rgba: [u8; 4]) -> Image {
        let pixels: Vec<u8> = (0..size * size).flat_map(|_| rgba).collect();
        Image::from_rgba(size, size, size, &pixels).unwrap()
    }

    #[test]
    fn one_opaque_layer_shows_as_it_is_and_shares_its_tiles() {
        let mut set = TextureSet::new(128).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(128, [200, 100, 50, 255])));
        let composite = Composite::of(&set).unwrap();
        assert_eq!(composite.image().pixel(5, 5), [200, 100, 50, 255]);
        let layer = set.layers[0].image().unwrap();
        assert!(Arc::ptr_eq(
            composite.image().tile(0).unwrap(),
            layer.tile(0).unwrap()
        ));
    }

    #[test]
    fn layers_mix_bottom_first_with_their_opacity() {
        let mut set = TextureSet::new(64).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(64, [0, 0, 0, 255])));
        let mut top = Layer::paint("Top", filled(64, [255, 255, 255, 255]));
        top.opacity = 0.5;
        set.layers.push(top);
        let composite = Composite::of(&set).unwrap();
        let [r, _, _, a] = composite.image().pixel(0, 0);
        assert!((r as i32 - 128).abs() <= 1, "{r}");
        assert_eq!(a, 255);
    }

    #[test]
    fn hidden_layers_and_empty_sets_show_nothing() {
        let mut set = TextureSet::new(64).unwrap();
        assert!(Composite::of(&set).unwrap().image().is_empty());
        let mut layer = Layer::paint("Base", filled(64, [9, 9, 9, 255]));
        layer.visible = false;
        set.layers.push(layer);
        assert!(Composite::of(&set).unwrap().image().is_empty());
    }

    #[test]
    fn only_changed_tiles_are_mixed_again() {
        let mut set = TextureSet::new(128).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(128, [1, 2, 3, 255])));
        let mut composite = Composite::of(&set).unwrap();
        assert!(composite.update(&set).is_empty(), "nothing changed");
        if let Some(image) = set.layers[0].image_mut() {
            image.set_pixel(100, 10, [255, 0, 0, 255]);
        }
        assert_eq!(composite.update(&set), vec![1]);
        assert_eq!(composite.image().pixel(100, 10), [255, 0, 0, 255]);
        set.layers[0].visible = false;
        assert_eq!(composite.update(&set).len(), 4, "every tile changed");
        assert!(composite.image().is_empty());
    }

    #[test]
    fn a_new_size_starts_afresh() {
        let mut set = TextureSet::new(64).unwrap();
        let mut composite = Composite::of(&set).unwrap();
        set.size = 128;
        assert_eq!(composite.update(&set).len(), 4);
        assert_eq!(composite.image().size(), 128);
    }

    #[test]
    fn a_half_transparent_layer_over_nothing_keeps_its_colour() {
        let mut set = TextureSet::new(64).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(64, [200, 50, 10, 128])));
        let mut faint = Layer::paint("Faint", filled(64, [200, 50, 10, 128]));
        faint.opacity = 0.5;
        set.layers.push(faint);
        let [r, g, b, a] = Composite::of(&set).unwrap().image().pixel(1, 1);
        assert_eq!([r, g, b], [200, 50, 10]);
        assert!(a > 128);
    }
}
