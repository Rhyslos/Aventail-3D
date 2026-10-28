//! Presets and starting layouts ([decision 0206](../../../../docs/decisions/0206-presets-and-starting-layouts.md)).
//!
//! A material with no layers yet offers two **starting layouts**: a
//! hand-painted one (a base colour, light and shade, edge highlights and
//! a layer to paint on) and a realistic one (a base colour, dirt in the
//! cavities, worn edges, dust on what faces up and a layer to paint on).
//! The Presets row saves the chosen layer as a **style** or the whole
//! material as a **material preset**, and loads either (or brushes) from
//! a preset file (`fileio::preset`).

use crate::text::tr;
use egui::Ui;
use paint::{Blend, Layer, SmartKind, SmartMask, TextureSet};
use scene::Document;

/// What a preset file to save holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetKind {
    /// The person's own brushes.
    Brushes,
    /// The chosen paint layer or group, as a recipe.
    Style,
    /// The shown material's colour and layers, as a recipe.
    Material,
}

impl PresetKind {
    /// What the file is called when nothing better is known.
    pub(crate) fn fallback_name(self) -> &'static str {
        match self {
            Self::Brushes => tr("Brushes"),
            Self::Style => tr("Style"),
            Self::Material => tr("Material"),
        }
    }
}

/// What the Presets row or the brush bar asked for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PresetAsk {
    /// Save a preset file of this kind.
    Save(PresetKind),
    /// Load a preset file (whatever it holds).
    Load,
}

/// A starting layout. The realistic one uses the realism channels
/// (decision 0207).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Start {
    /// For stylised, hand-painted textures.
    HandPainted,
    /// For realistic textures.
    Realistic,
}

impl Start {
    /// The undo step's name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::HandPainted => crate::text!("Start a hand-painted layout"),
            Self::Realistic => crate::text!("Start a realistic layout"),
        }
    }
}

/// The smart mask of the hand-painted layout's edge highlight (and the
/// panel's "+ Edge highlight").
pub(crate) const EDGE_HIGHLIGHT: SmartMask = SmartMask {
    kind: SmartKind::Edges,
    amount: 0.12,
    softness: 0.35,
    noise: 0.15,
    noise_size: 0.05,
    seed: 1,
};

/// The light and shade layer the panel and the hand-painted layout add.
pub(crate) fn light_and_shade(size: u32) -> Option<Layer> {
    let mut layer = Layer::paint(tr("Light and shade"), paint::Image::new(size)?);
    layer.lighting = Some(paint::Lighting::default());
    // Laid over the colours below rather than covering them.
    layer.blend = Blend::Overlay;
    layer.opacity = 0.6;
    Some(layer)
}

/// The edge highlight layer the panel and the hand-painted layout add.
pub(crate) fn edge_highlight(size: u32) -> Option<Layer> {
    let mut mask = paint::Mask::new(size, false)?;
    mask.smart = Some(EDGE_HIGHLIGHT);
    let mut layer = Layer::fill(tr("Edge highlight"), [255, 246, 225, 255]);
    layer.mask = Some(mask);
    // Lightens what's below rather than covering it.
    layer.blend = Blend::Screen;
    layer.opacity = 0.7;
    Some(layer)
}

/// A realism mask's ready-made layer (decision 0209): a fill of the
/// usual colour, roughness and metal for dirt, worn bare metal or rust,
/// shown through its smart mask.
pub(crate) fn realism_mask_layer(kind: SmartKind, size: u32) -> Option<Layer> {
    let (name, colour, roughness, metal, opacity) = match kind {
        SmartKind::Dirt => (tr("Dirt"), [62, 50, 38, 255], 0.95, 0.0, 0.75),
        SmartKind::Wear => (tr("Worn metal"), [176, 176, 182, 255], 0.35, 1.0, 1.0),
        SmartKind::Rust => (tr("Rust"), [118, 52, 24, 255], 0.9, 0.0, 0.9),
        _ => return None,
    };
    let mut layer = masked_fill(
        size,
        name,
        colour,
        Blend::Normal,
        opacity,
        SmartMask::of(kind),
    )?;
    layer.channels.roughness = Some(grey(roughness));
    layer.channels.metal = Some(grey(metal));
    Some(layer)
}

/// A fill layer shown through a smart mask.
fn masked_fill(
    size: u32,
    name: &str,
    colour: [u8; 4],
    blend: Blend,
    opacity: f32,
    smart: SmartMask,
) -> Option<Layer> {
    let mut mask = paint::Mask::new(size, false)?;
    mask.smart = Some(smart);
    let mut layer = Layer::fill(name, colour);
    layer.mask = Some(mask);
    layer.blend = blend;
    layer.opacity = opacity;
    Some(layer)
}

/// `start`'s layers, bottom first, for a set `size` pixels across over
/// a material coloured `base`. The top one is the layer to paint on.
pub(crate) fn starting_layers(start: Start, size: u32, base: [u8; 4]) -> Option<Vec<Layer>> {
    let mut layers = vec![Layer::fill(tr("Base colour"), base)];
    match start {
        Start::HandPainted => {
            layers.push(light_and_shade(size)?);
            layers.push(edge_highlight(size)?);
        }
        Start::Realistic => {
            // With roughness and metal (decision 0207): a fairly rough,
            // non-metal base.
            layers[0].channels.roughness = Some(grey(0.7));
            layers[0].channels.metal = Some(grey(0.0));
            layers.push(masked_fill(
                size,
                tr("Cavity dirt"),
                [58, 46, 36, 255],
                Blend::Multiply,
                0.55,
                SmartMask::of(SmartKind::Cavities),
            )?);
            // The base colour lightened: paint worn off the edges.
            let worn = base.map(|c| (f32::from(c) + (255.0 - f32::from(c)) * 0.35).round() as u8);
            layers.push(masked_fill(
                size,
                tr("Edge wear"),
                [worn[0], worn[1], worn[2], 255],
                Blend::Normal,
                0.5,
                SmartMask {
                    amount: 0.08,
                    softness: 0.2,
                    noise: 0.5,
                    noise_size: 0.04,
                    ..SmartMask::of(SmartKind::Edges)
                },
            )?);
            layers.push(masked_fill(
                size,
                tr("Dust"),
                [190, 180, 160, 255],
                Blend::Normal,
                0.25,
                SmartMask {
                    noise: 0.4,
                    ..SmartMask::of(SmartKind::FacingUp)
                },
            )?);
        }
    }
    if start == Start::Realistic {
        // Dirt is rough; worn edges are polished smoother; dust is
        // fully rough.
        for (layer, roughness) in layers[1..].iter_mut().zip([0.9, 0.45, 1.0]) {
            layer.channels.roughness = Some(grey(roughness));
        }
    }
    layers.push(Layer::paint(tr("Paint"), paint::Image::new(size)?));
    Some(layers)
}

/// A fill layer's grey channel value, 0 to 1.
fn grey(value: f32) -> paint::ChannelPart {
    let v = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    paint::ChannelPart::Fill([v, v, v, 255])
}

/// The colour of the material `name` (on the first object that has it),
/// or the standard grey.
pub(crate) fn material_colour(doc: &Document, name: &str) -> [f32; 3] {
    let scene = doc.scene();
    scene
        .walk()
        .iter()
        .flat_map(|(id, _)| scene.material_list(*id))
        .find(|m| m.name == name)
        .map_or(scene::Material::standard().color, |m| m.color)
}

/// A colour 0 to 1 as bytes.
pub(crate) fn bytes(colour: [f32; 3]) -> [u8; 4] {
    let c = colour.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    [c[0], c[1], c[2], 255]
}

/// The row shown while a material has no layers: the starting layouts.
pub(crate) fn start_row(ui: &mut Ui) -> Option<Start> {
    let mut chosen = None;
    ui.label(tr("Start with a layout:"));
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr("Hand-painted"))
            .on_hover_text(tr(
                "A base colour, light and shade, highlights along the edges, and a layer to \
                 paint on: a start for stylised, hand-painted textures",
            ))
            .clicked()
        {
            chosen = Some(Start::HandPainted);
        }
        if ui
            .button(tr("Realistic"))
            .on_hover_text(tr(
                "A base colour, dirt in the cavities, worn edges, dust on what faces up, and a \
                 layer to paint on: a start for realistic textures",
            ))
            .clicked()
        {
            chosen = Some(Start::Realistic);
        }
    });
    chosen
}

/// The Presets row: save the chosen layer as a style, save the material,
/// or load a preset file.
pub(crate) fn presets_row(ui: &mut Ui, set: &TextureSet, chosen: bool) -> Option<PresetAsk> {
    let mut asked = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Presets:"));
        if ui
            .add_enabled(chosen, egui::Button::new(tr("Save style…")))
            .on_hover_text(tr(
                "Keep the chosen layer or group in a preset file, to add to other materials \
                 and projects. Fills, gradients, light and shade and smart masks are kept; \
                 painted pixels aren't, as they belong to this model",
            ))
            .clicked()
        {
            asked = Some(PresetAsk::Save(PresetKind::Style));
        }
        if ui
            .add_enabled(
                !set.layers.is_empty(),
                egui::Button::new(tr("Save material…")),
            )
            .on_hover_text(tr(
                "Keep this material's colour and all its layers in a preset file, to use on \
                 other materials and projects (painted pixels aren't kept)",
            ))
            .clicked()
        {
            asked = Some(PresetAsk::Save(PresetKind::Material));
        }
        if ui
            .button(tr("Load preset…"))
            .on_hover_text(tr(
                "Read a preset file: a style is added above the chosen layer, a material \
                 replaces this material's colour and layers, and brushes join your own",
            ))
            .clicked()
        {
            asked = Some(PresetAsk::Load);
        }
    });
    asked
}
