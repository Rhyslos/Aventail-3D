//! Realism channels ([decision 0207](../../../docs/decisions/0207-realism-channels.md)):
//! besides its colour, a layer can change how rough or shiny the surface
//! is (**roughness**), whether it is metal (**metal**), and what light it
//! gives off (**glow**), as Substance Painter's channels do.
//!
//! Each layer holds a part for each channel it changes: painted pixels
//! for a paint layer, one value for a fill layer. A channel's layers are
//! mixed like the colour's ([`channel_view`] turns a set into one whose
//! layers hold that channel, for [`crate::Composite`]): in the order of
//! the layers, by their opacity, masks, clipping and visibility, always
//! with the Normal blend. Where no layer changes a channel, its standard
//! value shows ([`Channel::standard`]).
//!
//! Roughness and metal are kept as grey (the value in red, green and
//! blue alike; 0 smooth or not metal, 255 rough or metal), glow as a
//! colour (sRGB). Height (decision 0211) is grey too: 128 level, lighter
//! raised, darker sunk.

use crate::image::Image;
use crate::set::{Content, Layer, TextureSet};

/// How deep the height channel's full range is, in centimetres of the
/// surface (decision 0211): from 0 (most sunk) to 255 (most raised) is
/// this far, so 50% to 100% raises a spot by half of it.
pub const HEIGHT_DEPTH: f32 = 0.5;

/// A realism channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Channel {
    /// How rough the surface is: rough surfaces spread reflections out,
    /// smooth ones are shiny.
    Roughness,
    /// Whether the surface is metal: metal reflects in its own colour.
    Metal,
    /// Light the surface gives off, as a colour.
    Glow,
    /// How high the surface stands (decision 0211): bumps, cracks and
    /// grooves, shown by light and sent out as a normal map.
    Height,
}

impl Channel {
    /// Every channel, in the order they're listed.
    pub const ALL: [Channel; 4] = [
        Channel::Roughness,
        Channel::Metal,
        Channel::Glow,
        Channel::Height,
    ];

    /// Its number, for saving.
    pub fn index(self) -> u8 {
        self as u8
    }

    /// The channel numbered `index`.
    pub fn from_index(index: u8) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }

    /// What shows where no layer changes it: fairly rough, not metal, no
    /// glow.
    pub fn standard(self) -> [u8; 4] {
        match self {
            Channel::Roughness => [204, 204, 204, 255],
            Channel::Metal | Channel::Glow => [0, 0, 0, 255],
            // Level: neither raised nor sunk.
            Channel::Height => [128, 128, 128, 255],
        }
    }

    /// Whether its value is a grey (one number) rather than a colour.
    pub fn is_grey(self) -> bool {
        self != Channel::Glow
    }
}

/// A layer's part of a channel.
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelPart {
    /// Painted pixels (a paint layer's).
    Paint(Image),
    /// One value everywhere (a fill layer's), with its alpha.
    Fill([u8; 4]),
}

impl ChannelPart {
    /// Its pixels, if painted.
    pub fn image(&self) -> Option<&Image> {
        match self {
            ChannelPart::Paint(image) => Some(image),
            ChannelPart::Fill(_) => None,
        }
    }

    /// Its pixels to change, if painted.
    pub fn image_mut(&mut self) -> Option<&mut Image> {
        match self {
            ChannelPart::Paint(image) => Some(image),
            ChannelPart::Fill(_) => None,
        }
    }
}

/// The channels a layer changes, and how.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Channels {
    /// Its roughness, if it changes it.
    pub roughness: Option<ChannelPart>,
    /// Its metal, if it changes it.
    pub metal: Option<ChannelPart>,
    /// Its glow, if it changes it.
    pub glow: Option<ChannelPart>,
    /// Its height, if it changes it (decision 0211).
    pub height: Option<ChannelPart>,
}

impl Channels {
    /// Its part of `channel`, if it changes it.
    pub fn get(&self, channel: Channel) -> Option<&ChannelPart> {
        match channel {
            Channel::Roughness => self.roughness.as_ref(),
            Channel::Metal => self.metal.as_ref(),
            Channel::Glow => self.glow.as_ref(),
            Channel::Height => self.height.as_ref(),
        }
    }

    /// Its part of `channel`, to change or set.
    pub fn get_mut(&mut self, channel: Channel) -> &mut Option<ChannelPart> {
        match channel {
            Channel::Roughness => &mut self.roughness,
            Channel::Metal => &mut self.metal,
            Channel::Glow => &mut self.glow,
            Channel::Height => &mut self.height,
        }
    }

    /// Whether it changes no channel.
    pub fn is_empty(&self) -> bool {
        Channel::ALL.iter().all(|c| self.get(*c).is_none())
    }

    /// Every painted image, to change (to move or resize with the
    /// colour's).
    pub fn images_mut(&mut self) -> impl Iterator<Item = &mut Image> {
        [
            &mut self.roughness,
            &mut self.metal,
            &mut self.glow,
            &mut self.height,
        ]
        .into_iter()
        .filter_map(|p| p.as_mut().and_then(ChannelPart::image_mut))
    }

    /// Every painted image.
    pub fn images(&self) -> impl Iterator<Item = &Image> {
        Channel::ALL
            .into_iter()
            .filter_map(|c| self.get(c).and_then(ChannelPart::image))
    }

    /// Roughly how many bytes its images take.
    pub fn memory_size(&self) -> usize {
        self.images().map(Image::memory_size).sum()
    }
}

/// `set` with each layer holding its part of `channel` in place of its
/// colour, mixed by the Normal blend: what [`crate::Composite`] mixes to
/// show that channel. Layers that don't change it are see-through.
/// Images are shared, not copied.
pub fn channel_view(set: &TextureSet, channel: Channel) -> TextureSet {
    fn view(layer: &Layer, channel: Channel, blank: &Image) -> Layer {
        let content = match &layer.content {
            Content::Group(children) => Content::Group(
                children
                    .iter()
                    .map(|child| view(child, channel, blank))
                    .collect(),
            ),
            _ => match layer.channels.get(channel) {
                Some(ChannelPart::Paint(image)) => Content::Paint(image.clone()),
                Some(ChannelPart::Fill(value)) => Content::Fill(*value),
                None => Content::Paint(blank.clone()),
            },
        };
        Layer {
            name: layer.name.clone(),
            visible: layer.visible,
            opacity: layer.opacity,
            blend: crate::Blend::Normal,
            clipped: layer.clipped,
            expanded: layer.expanded,
            content,
            mask: layer.mask.clone(),
            strokes: Vec::new(),
            gradient: None,
            lighting: None,
            material: None,
            channels: Channels::default(),
        }
    }
    // A see-through image (no tiles), shared by every layer that doesn't
    // change the channel.
    let Some(blank) = Image::new(set.size) else {
        return TextureSet {
            size: set.size,
            layers: Vec::new(),
            realism: false,
        };
    };
    TextureSet {
        size: set.size,
        layers: set
            .layers
            .iter()
            .map(|layer| view(layer, channel, &blank))
            .collect(),
        realism: false,
    }
}

/// A channel's mixed pixel over its standard value: the grey (or glow
/// colour) shown, fully opaque.
pub fn over_standard(channel: Channel, mixed: [u8; 4]) -> [u8; 4] {
    let base = channel.standard();
    let a = f32::from(mixed[3]) / 255.0;
    let mut out = [255u8; 4];
    for k in 0..3 {
        let v = f32::from(base[k]) * (1.0 - a) + f32::from(mixed[k]) * a;
        out[k] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Whether any layer of `layers` (or their groups) changes a channel.
pub fn any_channels(layers: &[Layer]) -> bool {
    layers.iter().any(|layer| {
        !layer.channels.is_empty() || layer.children().is_some_and(|c| any_channels(c))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Composite;

    #[test]
    fn channels_are_numbered_both_ways() {
        for channel in Channel::ALL {
            assert_eq!(Channel::from_index(channel.index()), Some(channel));
        }
        assert_eq!(Channel::from_index(9), None);
    }

    #[test]
    fn a_channel_mixes_its_layers_parts_by_opacity_and_ignores_blend() {
        let mut set = TextureSet::new(64).unwrap();
        let mut base = Layer::fill("Base", [10, 20, 30, 255]);
        base.channels.roughness = Some(ChannelPart::Fill([200, 200, 200, 255]));
        let mut top = Layer::fill("Top", [0; 4]);
        top.channels.roughness = Some(ChannelPart::Fill([0, 0, 0, 255]));
        top.opacity = 0.5;
        top.blend = crate::Blend::Multiply;
        let plain = Layer::fill("No roughness", [1, 2, 3, 255]);
        set.layers = vec![base, top, plain];
        let view = channel_view(&set, Channel::Roughness);
        let mixed = Composite::of(&view).unwrap();
        let pixel = mixed.image().pixel(5, 5);
        assert_eq!(pixel[3], 255);
        assert!((i32::from(pixel[0]) - 100).abs() <= 1, "{pixel:?}");
        // Metal: nothing changes it, so it is see-through.
        let metal = Composite::of(&channel_view(&set, Channel::Metal)).unwrap();
        assert_eq!(metal.image().pixel(5, 5)[3], 0);
        assert_eq!(over_standard(Channel::Metal, [9, 9, 9, 0]), [0, 0, 0, 255]);
        assert_eq!(
            over_standard(Channel::Roughness, [0, 0, 0, 0]),
            Channel::Roughness.standard()
        );
    }

    #[test]
    fn a_painted_part_shows_only_where_painted_and_hidden_layers_dont() {
        let mut set = TextureSet::new(64).unwrap();
        let mut image = Image::new(64).unwrap();
        image.set_pixel(1, 1, [255, 255, 255, 255]);
        let mut layer = Layer::paint("Metal", Image::new(64).unwrap());
        layer.channels.metal = Some(ChannelPart::Paint(image));
        set.layers.push(layer);
        let mixed = Composite::of(&channel_view(&set, Channel::Metal)).unwrap();
        assert_eq!(mixed.image().pixel(1, 1), [255, 255, 255, 255]);
        assert_eq!(mixed.image().pixel(2, 2)[3], 0);
        set.layers[0].visible = false;
        let hidden = Composite::of(&channel_view(&set, Channel::Metal)).unwrap();
        assert_eq!(hidden.image().pixel(1, 1)[3], 0);
        assert!(any_channels(&set.layers));
    }

    #[test]
    fn groups_keep_their_layers_parts() {
        let mut inner = Layer::fill("Glow", [0; 4]);
        inner.channels.glow = Some(ChannelPart::Fill([255, 128, 0, 255]));
        let mut set = TextureSet::new(64).unwrap();
        set.layers.push(Layer::group("Lamp", vec![inner]));
        let mixed = Composite::of(&channel_view(&set, Channel::Glow)).unwrap();
        assert_eq!(mixed.image().pixel(0, 0), [255, 128, 0, 255]);
    }
}
