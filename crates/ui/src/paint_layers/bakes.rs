//! The Bakes section of the Paint layers panel ([decision 0192](../../../../docs/decisions/0192-the-baker.md)):
//! bake the texture set's maps from the shape, see them small, and know
//! when they are out of date. The workbench does the baking (on another
//! thread) and fills in what's shown here.

use crate::text::tr;
use crate::theme;
use bake::BakeKind;
use egui::{RichText, Ui};

/// How carefully occlusion and thickness are baked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum BakeQuality {
    /// Quick, a little grainy: 8 rays a texel.
    Draft,
    /// 32 rays a texel.
    #[default]
    Good,
    /// Smooth and slow: 128 rays a texel.
    Best,
}

impl BakeQuality {
    /// Every quality, in the order shown.
    pub(crate) const ALL: [BakeQuality; 3] =
        [BakeQuality::Draft, BakeQuality::Good, BakeQuality::Best];

    /// Rays a texel.
    pub(crate) fn rays(self) -> u32 {
        match self {
            BakeQuality::Draft => 8,
            BakeQuality::Good => 32,
            BakeQuality::Best => 128,
        }
    }

    fn name(self) -> &'static str {
        match self {
            BakeQuality::Draft => tr("Draft"),
            BakeQuality::Good => tr("Good"),
            BakeQuality::Best => tr("Best"),
        }
    }
}

/// What the Bakes section shows, filled in by the workbench each frame,
/// and what was asked of it.
#[derive(Debug, Default)]
pub(crate) struct BakesSection {
    /// Each baked map's picture, small.
    pub(crate) shown: Vec<(BakeKind, egui::TextureId)>,
    /// A line on how things stand (baking, out of date, baked).
    pub(crate) status: String,
    /// Whether baking is under way.
    pub(crate) busy: bool,
    /// The quality chosen.
    pub(crate) quality: BakeQuality,
    /// Whether Bake maps was clicked this frame.
    pub(crate) asked: bool,
}

/// The button that bakes the maps.
pub(crate) const BAKE_LABEL: &str = crate::text!("Bake maps");

/// A map's name, as shown.
pub(crate) fn kind_name(kind: BakeKind) -> &'static str {
    match kind {
        BakeKind::Position => tr("Position"),
        BakeKind::Normal => tr("Normals"),
        BakeKind::Occlusion => tr("Occlusion"),
        BakeKind::Curvature => tr("Curvature"),
        BakeKind::Id => tr("ID"),
        BakeKind::Thickness => tr("Thickness"),
    }
}

/// What a map shows, for its tooltip.
pub(crate) fn kind_help(kind: BakeKind) -> &'static str {
    match kind {
        BakeKind::Position => {
            tr("Where each part is on the shape: red across, green front to back, blue up")
        }
        BakeKind::Normal => tr("Which way each part faces: blue up, red and green sideways"),
        BakeKind::Occlusion => {
            tr("How open each part is to the light around it: dark in corners and creases")
        }
        BakeKind::Curvature => {
            tr("Light along outer edges, dark along inner ones: where wear and dirt go")
        }
        BakeKind::Id => tr("A colour for each material slot"),
        BakeKind::Thickness => tr("Light where the shape is thin, dark where thick"),
    }
}

impl BakesSection {
    /// Draws the section.
    pub(crate) fn show(&mut self, ui: &mut Ui) {
        self.asked = false;
        egui::CollapsingHeader::new(tr("Bakes"))
            .id_salt("paint_bakes")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let button = ui
                        .add_enabled(!self.busy, egui::Button::new(tr(BAKE_LABEL)))
                        .on_hover_text(tr(
                            "Work out maps of the shape (occlusion, curvature and others) for the \
                             texture set: guides for painting light, shade and wear",
                        ));
                    if button.clicked() {
                        self.asked = true;
                    }
                    egui::ComboBox::from_id_salt("bake_quality")
                        .selected_text(self.quality.name())
                        .width(64.0)
                        .show_ui(ui, |ui| {
                            for q in BakeQuality::ALL {
                                ui.selectable_value(&mut self.quality, q, q.name());
                            }
                        })
                        .response
                        .on_hover_text(tr(
                            "How carefully occlusion and thickness are worked out: Best is \
                             smoothest and slowest",
                        ));
                });
                if !self.status.is_empty() {
                    ui.label(RichText::new(&self.status).small().color(theme::TEXT_WEAK));
                }
                ui.horizontal_wrapped(|ui| {
                    for (kind, texture) in &self.shown {
                        ui.vertical(|ui| {
                            ui.image((*texture, egui::vec2(48.0, 48.0)))
                                .on_hover_text(kind_help(*kind));
                            ui.label(RichText::new(kind_name(*kind)).small());
                        });
                    }
                });
            });
    }
}
