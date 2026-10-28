//! Mixing a texture set's layers into the one image a surface shows.
//!
//! A [`Composite`] keeps the mixed image and what each of its tiles was
//! made from, so after a change only the tiles whose layers changed are
//! mixed again, and the views upload only those.
//!
//! Layers mix bottom first, each in its blend mode with its opacity
//! ([`crate::blend`]). A layer clipped to the one below shows only where
//! that layer (the nearest unclipped one under it) is painted. A group
//! mixes its own layers first, then lays the result on as one layer.

use crate::blend::{Blend, over};
use crate::image::{Image, TILE_BYTES, Tile};
use crate::set::{Content, Layer, TextureSet};
use std::sync::Arc;

/// What one layer gave one tile: its pixels (kept, so the pointer stays a
/// sound identity), and the settings it was mixed with.
#[derive(Clone, PartialEq)]
struct Input {
    source: Source,
    /// The mask's tile, base and inversion, if it has a mask.
    mask: Option<(Option<Tile>, bool, bool)>,
    visible: bool,
    opacity: u32,
    blend: Blend,
    clipped: bool,
}

/// Where a layer's pixels in one tile come from.
#[derive(Clone, PartialEq)]
enum Source {
    /// Painted pixels, or none.
    Tile(Option<Tile>),
    /// One colour everywhere.
    Fill([u8; 4]),
    /// A group's layers, bottom first.
    Group(Vec<Input>),
}

impl Input {
    fn of(layer: &Layer, index: usize) -> Self {
        Self {
            source: match &layer.content {
                Content::Paint(image) => Source::Tile(image.tile(index).cloned()),
                Content::Fill(color) => Source::Fill(*color),
                Content::Group(layers) => {
                    Source::Group(layers.iter().map(|l| Self::of(l, index)).collect())
                }
            },
            mask: layer
                .mask
                .as_ref()
                .map(|m| (m.image.tile(index).cloned(), m.white, m.inverted)),
            visible: layer.visible,
            opacity: layer.opacity.to_bits(),
            blend: layer.blend,
            clipped: layer.clipped,
        }
    }

    fn same(&self, other: &Self) -> bool {
        let same_source = match (&self.source, &other.source) {
            (Source::Tile(Some(a)), Source::Tile(Some(b))) => Arc::ptr_eq(a, b),
            (Source::Tile(None), Source::Tile(None)) => true,
            (Source::Fill(a), Source::Fill(b)) => a == b,
            (Source::Group(a), Source::Group(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.same(b))
            }
            _ => false,
        };
        let same_mask = match (&self.mask, &other.mask) {
            (None, None) => true,
            (Some((a, aw, ai)), Some((b, bw, bi))) => {
                aw == bw
                    && ai == bi
                    && match (a, b) {
                        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                        (None, None) => true,
                        _ => false,
                    }
            }
            _ => false,
        };
        same_source
            && same_mask
            && self.visible == other.visible
            && self.opacity == other.opacity
            && self.blend == other.blend
            && self.clipped == other.clipped
    }

    fn opacity(&self) -> f32 {
        f32::from_bits(self.opacity).clamp(0.0, 1.0)
    }

    /// Its pixels in the tile, mixed for a group and shaped by its mask;
    /// `None` if empty.
    fn pixels(&self) -> Option<Tile> {
        let pixels = self.unmasked()?;
        let Some((mask, white, inverted)) = &self.mask else {
            return Some(pixels);
        };
        let mut out = pixels.to_vec();
        for (i, px) in out.chunks_exact_mut(4).enumerate() {
            let at = mask.as_ref().map_or([0; 4], |m| {
                [m[i * 4], m[i * 4 + 1], m[i * 4 + 2], m[i * 4 + 3]]
            });
            let shown = crate::set::amount(at, *white, *inverted);
            px[3] = (px[3] as f32 * shown).round() as u8;
        }
        out.iter()
            .skip(3)
            .step_by(4)
            .any(|a| *a != 0)
            .then(|| Arc::new(out))
    }

    /// Its pixels in the tile before its mask.
    fn unmasked(&self) -> Option<Tile> {
        match &self.source {
            Source::Tile(tile) => tile.clone(),
            Source::Fill(color) if color[3] > 0 => Some(Arc::new(
                color.iter().copied().cycle().take(TILE_BYTES).collect(),
            )),
            Source::Fill(_) => None,
            Source::Group(inputs) => mix(inputs),
        }
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
        if set.size != self.image.size()
            && let Some(image) = Image::new(set.size)
        {
            self.image = image;
            self.inputs.clear();
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

/// `layers` (bottom first) mixed into one image `size` pixels across, as
/// the surface would show them: what Merge down keeps.
pub fn flatten(layers: &[Layer], size: u32) -> Option<Image> {
    let mut image = Image::new(size)?;
    for index in 0..image.tile_count() {
        let inputs: Vec<Input> = layers.iter().map(|l| Input::of(l, index)).collect();
        image.set_tile(index, mix(&inputs));
    }
    Some(image)
}

/// Mixes one tile's layers, bottom first. `None` if nothing shows.
fn mix(inputs: &[Input]) -> Option<Tile> {
    // One fully opaque normal paint layer: its tile as it is.
    if let [only] = inputs
        && only.visible
        && only.mask.is_none()
        && !only.clipped
        && only.blend == Blend::Normal
        && only.opacity() >= 1.0
        && let Source::Tile(tile) = &only.source
    {
        return tile.clone();
    }
    let mut out = vec![0u8; TILE_BYTES];
    // The pixels clipped layers show through (the nearest unclipped
    // layer below), and its opacity.
    let mut base: Option<Tile> = None;
    let mut base_opacity = 0.0;
    for input in inputs {
        let shows = input.visible && input.opacity() > 0.0;
        if !input.clipped {
            base = if shows { input.pixels() } else { None };
            base_opacity = input.opacity();
            if let Some(pixels) = &base {
                for (dst, src) in out.chunks_exact_mut(4).zip(pixels.chunks_exact(4)) {
                    over(dst, src, base_opacity, input.blend);
                }
            }
            continue;
        }
        let Some(under) = base.as_ref().filter(|_| shows) else {
            continue;
        };
        let Some(pixels) = input.pixels() else {
            continue;
        };
        let opacity = input.opacity() * base_opacity;
        for ((dst, src), below) in out
            .chunks_exact_mut(4)
            .zip(pixels.chunks_exact(4))
            .zip(under.chunks_exact(4))
        {
            over(dst, src, opacity * below[3] as f32 / 255.0, input.blend);
        }
    }
    out.iter()
        .skip(3)
        .step_by(4)
        .any(|a| *a != 0)
        .then(|| Arc::new(out))
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

    #[test]
    fn a_clipped_layer_shows_only_where_the_layer_below_is_painted() {
        let mut set = TextureSet::new(64).unwrap();
        let mut base = Image::new(64).unwrap();
        base.set_pixel(1, 1, [255, 255, 255, 255]);
        set.layers.push(Layer::paint("Base", base));
        let mut tint = Layer::fill("Tint", [255, 0, 0, 255]);
        tint.clipped = true;
        set.layers.push(tint);
        let image = Composite::of(&set).unwrap().image().clone();
        assert_eq!(image.pixel(1, 1), [255, 0, 0, 255]);
        assert_eq!(image.pixel(5, 5)[3], 0, "nothing below there");
        // Unclipped, the fill covers everything.
        set.layers[1].clipped = false;
        let image = Composite::of(&set).unwrap().image().clone();
        assert_eq!(image.pixel(5, 5), [255, 0, 0, 255]);
        // A hidden base hides what is clipped to it.
        set.layers[1].clipped = true;
        set.layers[0].visible = false;
        assert!(Composite::of(&set).unwrap().image().is_empty());
    }

    #[test]
    fn a_group_mixes_its_layers_then_lays_them_on_as_one() {
        let mut set = TextureSet::new(64).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(64, [0, 0, 0, 255])));
        let mut group = Layer::group(
            "Group",
            vec![
                Layer::fill("White", [255, 255, 255, 255]),
                Layer::fill("Red", [255, 0, 0, 255]),
            ],
        );
        group.opacity = 0.5;
        set.layers.push(group);
        let mut composite = Composite::of(&set).unwrap();
        let [r, g, _, a] = composite.image().pixel(0, 0);
        // Half of the group's red over black.
        assert!((r as i32 - 128).abs() <= 1 && g == 0 && a == 255, "{r} {g}");
        // Changing a layer inside the group mixes the tiles again.
        if let Some(children) = set.layers[1].children_mut() {
            children[1].visible = false;
        }
        assert_eq!(composite.update(&set).len(), 1);
        let [r, g, ..] = composite.image().pixel(0, 0);
        assert_eq!(r, g, "white shows through now");
    }

    #[test]
    fn blend_modes_mix_with_what_is_under() {
        let mut set = TextureSet::new(64).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(64, [200, 200, 200, 255])));
        let mut shade = Layer::fill("Shade", [128, 128, 128, 255]);
        shade.blend = Blend::Multiply;
        set.layers.push(shade);
        let [r, ..] = Composite::of(&set).unwrap().image().pixel(0, 0);
        assert!((r as i32 - 100).abs() <= 1, "{r}");
    }

    #[test]
    fn a_mask_shows_the_layer_by_its_lightness() {
        let mut set = TextureSet::new(64).unwrap();
        let mut layer = Layer::fill("Fill", [255, 0, 0, 255]);
        let mut mask = crate::Mask::new(64, false).unwrap();
        mask.image.set_pixel(1, 1, [255, 255, 255, 255]);
        mask.image.set_pixel(2, 2, [128, 128, 128, 255]);
        layer.mask = Some(mask);
        set.layers.push(layer);
        let image = Composite::of(&set).unwrap().image().clone();
        assert_eq!(image.pixel(1, 1), [255, 0, 0, 255], "white shows");
        assert_eq!(image.pixel(5, 5)[3], 0, "the black base hides");
        assert!(
            (image.pixel(2, 2)[3] as i32 - 128).abs() <= 1,
            "grey shows half"
        );
        // Inverted, the other way round; a white base shows all.
        set.layers[0].mask.as_mut().unwrap().inverted = true;
        let image = Composite::of(&set).unwrap().image().clone();
        assert_eq!(image.pixel(1, 1)[3], 0);
        assert_eq!(image.pixel(5, 5)[3], 255);
        set.layers[0].mask = crate::Mask::new(64, true);
        let image = Composite::of(&set).unwrap().image().clone();
        assert_eq!(image.pixel(5, 5)[3], 255);
    }
}
