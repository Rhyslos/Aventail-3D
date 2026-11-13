//! The Material mode's part in the workbench (decision 0258): its layout
//! while in the mode, and the canvas's graph kept up to date each frame.

use super::{LayoutChoice, Workbench};
use crate::palette::Mode;

impl Workbench {
    /// The Material mode on or off, each frame before the panels: the
    /// Material layout while in it (and the one before back after), and
    /// the graph shown sent to be worked out.
    pub(super) fn update_material_mode(&mut self, ctx: &egui::Context) {
        let on = self.mode == Mode::Material;
        if on && !self.material.on && self.layout_choice != LayoutChoice::Material {
            self.material.layout_before = Some(self.layout_choice);
            self.swap_layout(LayoutChoice::Material);
        }
        if !on && self.material.on {
            let before = self.material.layout_before.take();
            if self.layout_choice == LayoutChoice::Material {
                let back = before.unwrap_or(match self.mode {
                    Mode::Uv => LayoutChoice::UvEdit,
                    Mode::Paint => LayoutChoice::Paint,
                    _ => LayoutChoice::Standard,
                });
                self.swap_layout(back);
            }
            self.material.cancel_drag(&mut self.document);
            self.material.shape = None;
            self.material.paint = None;
        }
        self.material.on = on;
        if on {
            self.material.begin_frame(ctx, &self.document);
        }
    }

    /// Puts `reference` under the open shape canvas (decision 0273), as
    /// one undo step; says why not when no canvas is open.
    pub fn set_shape_reference(&mut self, reference: shapes::Reference) {
        let Some(drawing) = self.material.shape_drawing(&self.document) else {
            self.note = Some(
                crate::text::tr(
                    "Open a Shape box's canvas first: the reference image goes under it.",
                )
                .into(),
            );
            return;
        };
        let name = reference.name.clone();
        let changed = drawing.with_reference(Some(std::sync::Arc::new(reference)));
        let label = crate::text::tr("Bring in a reference image");
        self.note = Some(
            match self
                .material
                .set_drawing(&mut self.document, changed, label, None)
            {
                Ok(()) => crate::trf!(
                    "{} is under the canvas: draw over it, and change its opacity on the bar. It                      never changes the shape.",
                    name
                ),
                Err(why) => why.reason,
            },
        );
    }

    /// Puts `photo` on the Image box that asked for it (decision 0283),
    /// as one undo step, and says so.
    pub fn set_box_photo(&mut self, photo: matgraph::Photo) {
        self.note = Some(self.material.put_photo(&mut self.document, photo));
    }

    /// The Material mode's canvas, for tests.
    #[cfg(test)]
    pub(crate) fn material(&mut self) -> &mut crate::material::MaterialEditor {
        &mut self.material
    }
}
