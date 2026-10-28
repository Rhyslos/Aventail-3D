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

    /// Whether it is what `layer` gives tile `index` now: the same tiles
    /// (by pointer, so a sound identity) and settings as [`Self::of`]
    /// would give, without making it (most tiles don't change, and a big
    /// set has tens of thousands of them).
    fn matches(&self, layer: &Layer, index: usize) -> bool {
        let same_tile = |a: &Option<Tile>, b: Option<&Tile>| match (a, b) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let same_source = match (&self.source, &layer.content) {
            (Source::Tile(a), Content::Paint(image)) => same_tile(a, image.tile(index)),
            (Source::Fill(a), Content::Fill(b)) => a == b,
            (Source::Group(a), Content::Group(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.matches(b, index))
            }
            _ => false,
        };
        let same_mask = match (&self.mask, &layer.mask) {
            (None, None) => true,
            (Some((a, aw, ai)), Some(m)) => {
                *aw == m.white && *ai == m.inverted && same_tile(a, m.image.tile(index))
            }
            _ => false,
        };
        same_source
            && same_mask
            && self.visible == layer.visible
            && self.opacity == layer.opacity.to_bits()
            && self.blend == layer.blend
            && self.clipped == layer.clipped
    }

    /// Whether it adds nothing to a mix: hidden, see-through or with no
    /// pixels here.
    fn adds_nothing(&self) -> bool {
        !self.visible || self.opacity() <= 0.0 || matches!(self.source, Source::Tile(None))
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
    /// Per tile, the mix of the layers under the one that last changed
    /// there, and how many layers that is: painting on one layer then
    /// mixes only from it up (decision 0188).
    under: Vec<Option<(usize, MixState)>>,
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
            under: Vec::new(),
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
            self.under.clear();
        }
        let count = self.image.tile_count();
        let fresh = self.inputs.len() != count;
        if fresh {
            self.inputs = vec![Vec::new(); count];
            self.under = vec![None; count];
        }
        let mut changed = Vec::new();
        let mut work = Vec::new();
        for index in 0..count {
            let old = &self.inputs[index];
            // The first layer that changed here (all of them when fresh).
            let first = if fresh {
                0
            } else {
                old.iter()
                    .zip(&set.layers)
                    .position(|(a, b)| !a.matches(b, index))
                    .unwrap_or(old.len().min(set.layers.len()))
            };
            if !fresh && old.len() == set.layers.len() && first == old.len() {
                continue;
            }
            let now: Vec<Input> = set.layers.iter().map(|l| Input::of(l, index)).collect();
            changed.push(index);
            // When the layers from the first that changed up added
            // nothing, the tile shown was the mix of those under it: a
            // stroke on an empty part of a layer needn't mix them again.
            let shown = (!fresh && first > 0 && old[first..].iter().all(Input::adds_nothing))
                .then(|| self.image.tile(index).cloned());
            work.push(Work {
                index,
                first,
                under: self.under[index].take(),
                shown,
                inputs: now,
            });
        }
        let mixed = mix_all(&work);
        for (work, (tile, under)) in work.into_iter().zip(mixed) {
            self.image.set_tile(work.index, tile);
            self.inputs[work.index] = work.inputs;
            self.under[work.index] = under;
        }
        changed
    }
}

/// A tile mixed, and the mix kept of the layers under the first that
/// changed.
type Mixed = (Option<Tile>, Option<(usize, MixState)>);

/// One tile to mix again: its layers now, the first of them that
/// changed, and the mix kept of those under it from last time.
struct Work {
    index: usize,
    first: usize,
    under: Option<(usize, MixState)>,
    /// The tile as shown before, when it was just the mix of the layers
    /// under the first that changed.
    shown: Option<Option<Tile>>,
    inputs: Vec<Input>,
}

impl Work {
    /// The tile mixed, and the mix of the layers under the first that
    /// changed, to keep for next time (none when all of them changed).
    fn mix(&self) -> Mixed {
        let first = self.first.min(self.inputs.len());
        if first == 0 {
            return (mix(&self.inputs), None);
        }
        // From what was kept, if it is of layers that haven't changed.
        let mut state = match (&self.under, &self.shown) {
            (_, Some(shown)) => MixState::shown(shown.as_ref(), &self.inputs[..first]),
            (Some((kept, state)), None) if *kept <= first => {
                let mut state = state.clone();
                state.lay(&self.inputs[*kept..first]);
                state
            }
            (_, None) => {
                let mut state = MixState::new();
                state.lay(&self.inputs[..first]);
                state
            }
        };
        let under = state.clone();
        state.lay(&self.inputs[first..]);
        (state.finish(), Some((first, under)))
    }
}

/// Fewer tiles than this are mixed on one thread: starting threads costs
/// more than it saves.
const TILES_A_THREAD: usize = 16;

/// Mixes each `(index, layers)` tile, in order, spread over the
/// processor's cores when there are many (decision 0188): the first
/// mix of a 4K texture set of twenty layers is tens of millions of
/// pixels.
fn mix_all(work: &[Work]) -> Vec<Mixed> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(work.len() / TILES_A_THREAD)
        .max(1);
    if threads == 1 {
        return work.iter().map(Work::mix).collect();
    }
    let per = work.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let parts: Vec<_> = work
            .chunks(per)
            .map(|part| scope.spawn(move || part.iter().map(Work::mix).collect::<Vec<_>>()))
            .collect();
        parts
            .into_iter()
            // A panic while mixing is a bug; mix that part here instead
            // of losing it.
            .zip(work.chunks(per))
            .flat_map(|(part, work)| {
                part.join()
                    .unwrap_or_else(|_| work.iter().map(Work::mix).collect())
            })
            .collect()
    })
}

/// `layers` (bottom first) mixed into one image `size` pixels across, as
/// the surface would show them: what Merge down keeps.
pub fn flatten(layers: &[Layer], size: u32) -> Option<Image> {
    let mut image = Image::new(size)?;
    let work: Vec<Work> = (0..image.tile_count())
        .map(|index| Work {
            index,
            first: 0,
            under: None,
            shown: None,
            inputs: layers.iter().map(|l| Input::of(l, index)).collect(),
        })
        .collect();
    for (work, (tile, _)) in work.iter().zip(mix_all(&work)) {
        image.set_tile(work.index, tile);
    }
    Some(image)
}

/// The pixel `set` shows at texel (`x`, `y`), mixing only the tile that
/// holds it: what colour picking reads (decision 0183). Transparent
/// outside the texture.
pub fn pixel_at(set: &TextureSet, x: u32, y: u32) -> [u8; 4] {
    if x >= set.size || y >= set.size {
        return [0; 4];
    }
    let across = crate::image::tiles_across(set.size);
    let index = ((y / crate::TILE) * across + x / crate::TILE) as usize;
    let inputs: Vec<Input> = set.layers.iter().map(|l| Input::of(l, index)).collect();
    let Some(tile) = mix(&inputs) else {
        return [0; 4];
    };
    let at = (((y % crate::TILE) * crate::TILE + x % crate::TILE) * 4) as usize;
    [tile[at], tile[at + 1], tile[at + 2], tile[at + 3]]
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
    let mut state = MixState::new();
    state.lay(inputs);
    state.finish()
}

/// A tile being mixed, layer by layer from the bottom.
#[derive(Clone)]
struct MixState {
    out: Vec<u8>,
    /// The pixels clipped layers show through (the nearest unclipped
    /// layer below), and its opacity.
    base: Option<Tile>,
    base_opacity: f32,
}

impl MixState {
    /// Nothing mixed yet.
    fn new() -> Self {
        Self {
            out: vec![0u8; TILE_BYTES],
            base: None,
            base_opacity: 0.0,
        }
    }

    /// The state after laying `under` on, when their mix is `shown`
    /// already: only what clipped layers above would show through is
    /// worked out.
    fn shown(shown: Option<&Tile>, under: &[Input]) -> Self {
        let mut state = Self::new();
        if let Some(tile) = shown {
            state.out.copy_from_slice(tile);
        }
        if let Some(input) = under.iter().rev().find(|i| !i.clipped) {
            let shows = input.visible && input.opacity() > 0.0;
            state.base = if shows { input.pixels() } else { None };
            state.base_opacity = input.opacity();
        }
        state
    }

    /// Lays `inputs` on, bottom first.
    fn lay(&mut self, inputs: &[Input]) {
        for input in inputs {
            let shows = input.visible && input.opacity() > 0.0;
            if !input.clipped {
                self.base = if shows { input.pixels() } else { None };
                self.base_opacity = input.opacity();
                if let Some(pixels) = &self.base {
                    for (dst, src) in self.out.chunks_exact_mut(4).zip(pixels.chunks_exact(4)) {
                        over(dst, src, self.base_opacity, input.blend);
                    }
                }
                continue;
            }
            let Some(under) = self.base.as_ref().filter(|_| shows) else {
                continue;
            };
            let Some(pixels) = input.pixels() else {
                continue;
            };
            let opacity = input.opacity() * self.base_opacity;
            for ((dst, src), below) in self
                .out
                .chunks_exact_mut(4)
                .zip(pixels.chunks_exact(4))
                .zip(under.chunks_exact(4))
            {
                over(dst, src, opacity * below[3] as f32 / 255.0, input.blend);
            }
        }
    }

    /// The tile mixed, or `None` if nothing shows in it.
    fn finish(self) -> Option<Tile> {
        self.out
            .iter()
            .skip(3)
            .step_by(4)
            .any(|a| *a != 0)
            .then(|| Arc::new(self.out))
    }
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

    #[test]
    fn a_pixel_is_read_as_the_whole_mix_shows_it() {
        let mut set = TextureSet::new(128).unwrap();
        set.layers
            .push(Layer::paint("Base", filled(128, [0, 0, 0, 255])));
        let mut top = Image::new(128).unwrap();
        top.set_pixel(100, 70, [255, 255, 255, 255]);
        set.layers.push(Layer::paint("Top", top));
        let whole = Composite::of(&set).unwrap();
        for (x, y) in [(100, 70), (0, 0), (127, 127)] {
            assert_eq!(pixel_at(&set, x, y), whole.image().pixel(x, y));
        }
        assert_eq!(pixel_at(&set, 128, 0), [0; 4]);
    }

    #[test]
    fn many_tiles_mixed_on_several_threads_come_out_as_on_one() {
        // 1024 px: 256 tiles, enough to share out.
        let mut set = TextureSet::new(1024).unwrap();
        let mut under = filled(1024, [10, 200, 30, 255]);
        under.set_pixel(700, 900, [255, 0, 0, 255]);
        set.layers.push(Layer::paint("Under", under));
        let mut over_it = Layer::paint("Over", filled(1024, [90, 20, 250, 120]));
        over_it.blend = Blend::Overlay;
        set.layers.push(over_it);
        set.layers.push(Layer::fill("Tint", [0, 0, 255, 40]));
        let work: Vec<Work> = (0..256)
            .map(|index| Work {
                index,
                first: 0,
                under: None,
                shown: None,
                inputs: set.layers.iter().map(|l| Input::of(l, index)).collect(),
            })
            .collect();
        let together = mix_all(&work);
        assert_eq!(together.len(), 256);
        for (work, (tile, _)) in work.iter().zip(&together) {
            assert_eq!(tile.as_deref(), mix(&work.inputs).as_deref());
        }
        let composite = Composite::of(&set).unwrap();
        assert_ne!(
            composite.image().pixel(700, 900),
            composite.image().pixel(0, 0)
        );
    }

    #[test]
    fn the_quick_blends_match_the_full_sums() {
        // Over nothing, and an opaque normal layer: the shortcuts.
        for (under, layer, opacity) in [
            ([0, 0, 0, 0], [200, 100, 50, 128], 1.0),
            ([0, 0, 0, 0], [200, 100, 50, 255], 0.5),
            ([30, 60, 90, 200], [200, 100, 50, 255], 1.0),
        ] {
            let mut quick = under;
            over(&mut quick, &layer, opacity, Blend::Normal);
            // The sums by hand.
            let sa = layer[3] as f32 / 255.0 * opacity;
            let da = under[3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            for c in 0..3 {
                let s = layer[c] as f32 / 255.0;
                let d = under[c] as f32 / 255.0;
                let value = (s * sa + d * da * (1.0 - sa)) / out_a;
                assert_eq!(quick[c], (value * 255.0).round() as u8);
            }
            assert_eq!(quick[3], (out_a * 255.0).round() as u8);
        }
    }

    #[test]
    fn mixing_from_the_kept_layers_under_comes_out_as_mixing_afresh() {
        // Five layers on 128 px; then a run of changes, each checked
        // against a composite made from nothing.
        let mut set = TextureSet::new(128).unwrap();
        for (n, rgba) in [
            [200, 30, 30, 255],
            [30, 200, 30, 140],
            [30, 30, 200, 90],
            [250, 250, 0, 60],
            [0, 0, 0, 200],
        ]
        .into_iter()
        .enumerate()
        {
            let mut layer = Layer::paint(format!("L{n}"), filled(128, rgba));
            layer.blend = [
                Blend::Normal,
                Blend::Multiply,
                Blend::Screen,
                Blend::Overlay,
                Blend::Hue,
            ][n];
            set.layers.push(layer);
        }
        set.layers[3].clipped = true;
        let mut kept = Composite::of(&set).unwrap();
        let check = |kept: &Composite, set: &TextureSet, what: &str| {
            let fresh = Composite::of(set).unwrap();
            for (x, y) in [(0, 0), (70, 3), (127, 127), (64, 64)] {
                assert_eq!(
                    kept.image().pixel(x, y),
                    fresh.image().pixel(x, y),
                    "{what}"
                );
            }
        };
        let paint = |set: &mut TextureSet, layer: usize, rgba: [u8; 4]| {
            let image = set.layers[layer].image_mut().unwrap();
            for y in 0..128 {
                image.set_pixel(70, y, rgba);
            }
        };
        for (layer, rgba, what) in [
            (4, [9, 9, 9, 255], "the top layer"),
            (4, [90, 9, 9, 100], "the top layer again"),
            (2, [9, 200, 9, 255], "a layer in the middle"),
            (4, [9, 9, 200, 30], "the top after the middle"),
            (3, [255, 255, 255, 255], "the clipped layer"),
            (0, [0, 0, 0, 0], "the bottom"),
        ] {
            paint(&mut set, layer, rgba);
            kept.update(&set);
            check(&kept, &set, what);
        }
        set.layers[2].opacity = 0.3;
        kept.update(&set);
        check(&kept, &set, "an opacity");
        set.layers
            .push(Layer::paint("New", filled(128, [1, 2, 3, 77])));
        kept.update(&set);
        check(&kept, &set, "a layer added on top");
        paint(&mut set, 5, [200, 0, 200, 255]);
        kept.update(&set);
        check(&kept, &set, "painting the new layer");
        set.layers.remove(1);
        kept.update(&set);
        check(&kept, &set, "a layer taken out");
        // An empty layer on top, clipped, then painted: mixed from the
        // tile shown.
        let mut empty = Layer::paint("Empty", Image::new(128).unwrap());
        empty.clipped = true;
        set.layers.push(empty);
        kept.update(&set);
        check(&kept, &set, "an empty clipped layer added");
        let top = set.layers.len() - 1;
        paint(&mut set, top, [0, 255, 255, 180]);
        kept.update(&set);
        check(&kept, &set, "the empty clipped layer painted");
        set.layers[top].clipped = false;
        set.layers
            .push(Layer::paint("Empty 2", Image::new(128).unwrap()));
        kept.update(&set);
        let top = set.layers.len() - 1;
        paint(&mut set, top, [255, 0, 255, 255]);
        kept.update(&set);
        check(&kept, &set, "another empty layer painted");
    }
}
