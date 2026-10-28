//! Making gradient layers ([decision 0194](../../../../docs/decisions/0194-gradient-fills.md)):
//! a gradient layer's pixels are made from its settings on the shapes
//! using its material. The Paint layers panel asks for it when a gradient
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
            if !remake(&mut made.layers, &sources) || made == *set {
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

/// Makes every gradient layer in `layers` (groups included) on
/// `sources`. Whether there was one.
fn remake(layers: &mut [Layer], sources: &[bake::Source<'_>]) -> bool {
    let mut any = false;
    for layer in layers {
        if let Some(gradient) = layer.gradient
            && let Some(image) = bake::gradient(sources, &gradient)
        {
            layer.content = Content::Paint(image);
            any = true;
        }
        if let Content::Group(children) = &mut layer.content {
            any |= remake(children, sources);
        }
    }
    any
}

fn has_gradient(layers: &[Layer]) -> bool {
    layers
        .iter()
        .any(|l| l.gradient.is_some() || l.children().is_some_and(|c| has_gradient(c)))
}
