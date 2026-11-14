//! A Seam blend layer's settings ([decision 0293](../../../../docs/decisions/0293-blend-seams.md)):
//! how many seams it blends, how wide, and the buttons that mark every
//! seam that shows or unmark them all. The same rows show in the Paint
//! layers panel (for the chosen Seam blend layer) and in the Paint tab
//! while the Blend seams tool (9) is in use.

use crate::dialog::help;
use crate::text::tr;
use egui::{RichText, Ui};
use paint::seam_blend::{MAX_WIDTH, MIN_WIDTH};

/// The button marking every seam whose sides don't match.
pub(crate) const SHOWING_LABEL: &str = crate::text!("Every seam that shows");

/// The button unmarking every seam.
pub(crate) const UNMARK_LABEL: &str = crate::text!("Unmark all");

/// The width slider's name.
pub(crate) const WIDTH_LABEL: &str = crate::text!("Blend width");

/// What the rows show: the layer's seams and width, and how its making
/// stands.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct SeamView {
    /// Whether there is a Seam blend layer yet (the tool makes one on the
    /// first seam clicked).
    pub(crate) layer: bool,
    /// How many seam edges are marked.
    pub(crate) marked: usize,
    /// How many of them are no longer on the model.
    pub(crate) missing: usize,
    /// Its width, in texels each side.
    pub(crate) width: f32,
    /// Whether it is being made now.
    pub(crate) making: bool,
}

/// What the rows asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SeamAsk {
    /// A new width (while its slider is dragged, every frame).
    Width(f32),
    /// Mark every seam that shows.
    Showing,
    /// Unmark every seam.
    Unmark,
}

/// The rows for `view`; what was asked. `dragging` is set while the
/// width is being dragged or typed.
pub(crate) fn show(ui: &mut Ui, view: &SeamView, dragging: &mut bool) -> Option<SeamAsk> {
    let mut asked = None;
    if view.layer {
        let text = match view.marked {
            0 => tr(
                "No seams marked yet: click one on the model or in the UV Editor with Blend \
                 seams (9).",
            )
            .to_string(),
            1 => tr("Blends 1 seam edge.").to_string(),
            n => crate::trf!("Blends {} seam edges.", n),
        };
        ui.label(text);
        if view.missing > 0 {
            ui.label(
                RichText::new(crate::trf!(
                    "{} of them are no longer on the model (changed since), so they're left out.",
                    view.missing
                ))
                .color(crate::theme::WARNING),
            );
        }
        if view.making {
            help(ui, tr("Blending…"));
        }
        let mut width = view.width;
        let slider = crate::fitting::side_row(ui, tr(WIDTH_LABEL), |ui| {
            ui.add(
                egui::Slider::new(&mut width, MIN_WIDTH..=MAX_WIDTH)
                    .suffix(tr(" px"))
                    .max_decimals(0),
            )
            .on_hover_text(tr(
                "How far from each seam the blend reaches, in texture pixels on each side: \
                 wider hides a bigger jump, narrower keeps more of each side's detail",
            ))
        });
        if slider.dragged() || slider.has_focus() {
            *dragging = true;
        }
        if slider.changed() {
            asked = Some(SeamAsk::Width(width));
        }
    } else {
        help(
            ui,
            tr(
                "Click a seam on the model or in the UV Editor: a Seam blend layer is made on \
                 top to hold the blend.",
            ),
        );
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr(SHOWING_LABEL))
            .on_hover_text(tr(
                "Mark every seam of this material whose two sides don't match (painted \
                 differently), on the shapes shown",
            ))
            .clicked()
        {
            asked = Some(SeamAsk::Showing);
        }
        if ui
            .add_enabled(view.marked > 0, egui::Button::new(tr(UNMARK_LABEL)))
            .on_hover_text(tr("Blend no seams: the layer turns see-through"))
            .clicked()
        {
            asked = Some(SeamAsk::Unmark);
        }
    });
    asked
}
