//! Making gradient layers ([decision 0194](../../../../docs/decisions/0194-gradient-fills.md)),
//! light and shade ([decision 0196](../../../../docs/decisions/0196-light-and-shade.md))
//! and smart masks ([decision 0195](../../../../docs/decisions/0195-smart-masks.md)):
//! these layers' pixels are made from their settings on the shapes
//! using their material. The Paint layers panel asks for it when one
//! is added or changed, and paint following the model when the shapes or
//! the texture size changed; the made pixels join that change's undo step.

use super::Workbench;
use crate::drawing::SceneDrawing;
use paint::{Content, Layer, TextureSet};

impl Workbench {
    /// Makes the gradient layers of the materials asked for, each frame
    /// after the drawing is made.
    pub(super) fn make_gradients(&mut self, drawing: &SceneDrawing) {
        let asked = std::mem::take(&mut self.paint_layers.gradients_to_make);
        for material in asked {
            let Some(set) = self.document.scene().paint().set(&material).cloned() else {
                continue;
            };
            let shapes = self.material_shapes(&material, drawing);
            let sources: Vec<bake::Source<'_>> = shapes
                .iter()
                .map(|(_, surface, slots)| bake::Source {
                    surface,
                    slots,
                    size: set.size,
                })
                .collect();
            let mut made = (*set).clone();
            let realism = made.realism;
            if !remake(&mut made.layers, &sources, realism) || made == *set {
                continue;
            }
            let command = scene::Command::SetTextureSet {
                material: material.clone(),
                set: Some(made),
            };
            if let Err(why) = self.document.execute_with_last(command) {
                self.note = Some(crate::trf!("Couldn't make the gradient: {}.", why));
            }
        }
    }

    /// Asks for `material`'s gradient layers to be made again, if it has
    /// any.
    pub(super) fn remake_gradients_of(&mut self, material: &str, set: &TextureSet) {
        if has_gradient(&set.layers)
            && !self
                .paint_layers
                .gradients_to_make
                .iter()
                .any(|m| m == material)
        {
            self.paint_layers
                .gradients_to_make
                .push(material.to_string());
        }
    }
}

/// Makes every gradient layer, light-and-shade layer (decision 0196),
/// material layer (decision 0210) and smart mask (decision 0195) in
/// `layers` (groups included) on `sources`. Whether there was one. With
/// `realism`, material layers get their roughness and metal too.
fn remake(layers: &mut [Layer], sources: &[bake::Source<'_>], realism: bool) -> bool {
    let mut any = false;
    for layer in layers {
        if let Some(mask) = &mut layer.mask
            && let Some(smart) = mask.smart
            && let Some(image) = bake::smart_mask(sources, &smart)
        {
            mask.image = image;
            any = true;
        }
        if let Some(lighting) = layer.lighting
            && let Some(image) = bake::lighting(sources, &lighting)
        {
            layer.content = Content::Paint(image);
            any = true;
        }
        if let Some(gradient) = layer.gradient
            && let Some(image) = bake::gradient(sources, &gradient)
        {
            layer.content = Content::Paint(image);
            any = true;
        }
        if let Some(fill) = layer.material
            && let Some(made) = bake::material(sources, &fill)
        {
            layer.content = Content::Paint(made.colour);
            let (roughness, metal) = if realism {
                (
                    Some(paint::ChannelPart::Paint(made.roughness)),
                    Some(paint::ChannelPart::Paint(made.metal)),
                )
            } else {
                (None, None)
            };
            layer.channels.roughness = roughness;
            layer.channels.metal = metal;
            any = true;
        }
        if let Content::Group(children) = &mut layer.content {
            any |= remake(children, sources, realism);
        }
    }
    any
}

fn has_gradient(layers: &[Layer]) -> bool {
    layers.iter().any(|l| {
        l.is_made()
            || l.mask.as_ref().is_some_and(|m| m.smart.is_some())
            || l.children().is_some_and(|c| has_gradient(c))
    })
}
