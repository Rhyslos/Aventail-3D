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
        }
        self.material.on = on;
        if on {
            self.material.begin_frame(ctx, &self.document);
        }
    }

    /// The Material mode's canvas, for tests.
    #[cfg(test)]
    pub(crate) fn material(&mut self) -> &mut crate::material::MaterialEditor {
        &mut self.material
    }
}
