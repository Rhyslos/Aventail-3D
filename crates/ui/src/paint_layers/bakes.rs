//! The Bakes section of the Paint layers panel ([decision 0192](../../../../docs/decisions/0192-the-baker.md)):
//! bake the texture set's maps from the shape, see them small, and know
//! when they are out of date. The workbench does the baking (on another
//! thread) and fills in what's shown here.

use crate::text::tr;
use crate::theme;
use bake::BakeKind;
use egui::{RichText, Ui};

/// The width of each baked map's box: its picture and name.
const THUMB_BOX: f32 = 64.0;

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
    /// Whether Cancel was clicked while baking.
    pub(crate) cancel: bool,
    /// The map shown on the model as a guide (decision 0193), if any.
    pub(crate) guide: Option<BakeKind>,
    /// How strongly the guide shows, 0 to 1.
    pub(crate) guide_opacity: f32,
    /// A map clicked this frame (to show it as a guide, or stop).
    pub(crate) toggled: Option<BakeKind>,
    /// Whether "Add as a layer" was clicked this frame.
    pub(crate) add_layer: bool,
    /// Whether "Use as the layer's mask" was clicked this frame.
    pub(crate) as_mask: bool,
}

/// The guide's standard strength.
pub(crate) const GUIDE_OPACITY: f32 = 0.6;

/// The button that adds the guide's map as a paint layer.
pub(crate) const ADD_LAYER_LABEL: &str = crate::text!("Add as a layer");

/// The button that makes the guide's map the chosen layer's mask.
pub(crate) const AS_MASK_LABEL: &str = crate::text!("Use as the layer's mask");

/// How a map is laid over paint, as a guide or a layer: occlusion
/// darkens (multiply), curvature lightens and darkens around mid-grey
/// (overlay), the others cover.
pub(crate) fn kind_blend(kind: BakeKind) -> paint::Blend {
    match kind {
        BakeKind::Occlusion => paint::Blend::Multiply,
        BakeKind::Curvature => paint::Blend::Overlay,
        _ => paint::Blend::Normal,
    }
}

/// The button that bakes the maps.
pub(crate) const BAKE_LABEL: &str = crate::text!("Bake maps");
/// The button that stops baking.
pub(crate) const BAKE_CANCEL_LABEL: &str = crate::text!("Cancel baking");

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
        self.toggled = None;
        self.add_layer = false;
        self.as_mask = false;
        if self.guide_opacity <= 0.0 {
            self.guide_opacity = GUIDE_OPACITY;
        }
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
                    if self.busy
                        && ui
                            .button(tr(BAKE_CANCEL_LABEL))
                            .on_hover_text(tr("Stop baking; the maps baked before stay"))
                            .clicked()
                    {
                        self.cancel = true;
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
                // Each map in a box of its own size, so the row wraps
                // onto the next line in a narrow panel (a plain column
                // takes the whole width left, and the row never wraps).
                ui.horizontal_wrapped(|ui| {
                    for (kind, texture) in &self.shown {
                        let size = egui::vec2(THUMB_BOX, THUMB_BOX + 18.0);
                        let column = egui::Layout::top_down(egui::Align::Center);
                        ui.allocate_ui_with_layout(size, column, |ui| {
                            ui.set_max_width(THUMB_BOX);
                            let on = self.guide == Some(*kind);
                            let picture = egui::Image::new((*texture, egui::vec2(48.0, 48.0)));
                            let response = ui
                                .add(egui::Button::image(picture).selected(on))
                                .on_hover_text(crate::trf!(
                                    "{}. Click to show it on the model as a guide; click again to \
                                     stop",
                                    kind_help(*kind)
                                ));
                            if response.clicked() {
                                self.toggled = Some(*kind);
                            }
                            ui.label(RichText::new(kind_name(*kind)).small());
                        });
                    }
                });
                if let Some(kind) = self.guide {
                    ui.label(
                        RichText::new(crate::trf!("Shown on the model: {}.", kind_name(kind)))
                            .small(),
                    );
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Strength")).color(theme::TEXT_WEAK))
                            .on_hover_text(tr("How strongly the guide shows over the paint"));
                        ui.add(
                            egui::Slider::new(&mut self.guide_opacity, 0.05..=1.0)
                                .show_value(false),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(tr(ADD_LAYER_LABEL))
                            .on_hover_text(tr(
                                "A new paint layer holding this map, above the chosen layer, \
                                 mixed the way the guide shows it: paint over it, or lower its \
                                 opacity",
                            ))
                            .clicked()
                        {
                            self.add_layer = true;
                        }
                        if ui
                            .button(tr(AS_MASK_LABEL))
                            .on_hover_text(tr(
                                "The chosen layer shows only where this map is light: dirt in \
                                 the dark corners of occlusion (with Invert), wear on the light \
                                 edges of curvature",
                            ))
                            .clicked()
                        {
                            self.as_mask = true;
                        }
                    });
                }
            });
        // What was clicked is carried out by the workbench next frame.
        if self.asked || self.toggled.is_some() || self.add_layer || self.as_mask {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    #[test]
    fn the_baked_maps_wrap_onto_more_rows_in_a_narrow_panel() {
        // Second review: the six maps stayed in one row, off the panel's
        // edge.
        let mut section = BakesSection {
            shown: BakeKind::ALL
                .iter()
                .map(|k| (*k, egui::TextureId::default()))
                .collect(),
            ..Default::default()
        };
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(220.0, 600.0))
            .build_ui(move |ui| section.show(ui));
        harness.run();
        let tops: std::collections::BTreeSet<i32> = BakeKind::ALL
            .iter()
            .map(|k| harness.get_by_label(kind_name(*k)).rect().top() as i32)
            .collect();
        assert!(tops.len() >= 2, "all in one row");
        for k in BakeKind::ALL {
            assert!(harness.get_by_label(kind_name(k)).rect().right() <= 220.0);
        }
    }
}
