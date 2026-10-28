//! The paint brush's settings in the Attributes panel, in Paint mode
//! ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)):
//! like Substance Painter's Properties panel, the settings of the tool in
//! use, above the selection's own.

use super::{AttributesPanel, label, section, stepper};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use paint::{Brush, BrushKind};

/// The brush's settings, kept between strokes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BrushSettings {
    /// The brush.
    pub(crate) brush: Brush,
    /// Whether its size is measured on the surface (centimetres) instead
    /// of on screen (points): the same real size on every object.
    pub(crate) on_surface: bool,
    /// Its radius on the surface, in centimetres, when measured there.
    pub(crate) surface_radius: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            brush: Brush::default(),
            on_surface: false,
            surface_radius: 5.0,
        }
    }
}

/// The smallest and largest radius on the surface, in centimetres.
pub(crate) const SURFACE_RADIUS: (f32, f32) = (0.05, 500.0);

/// What the brush is painting on, for the panel to show: the material
/// and the layer, and how big a texture pixel is there.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BrushTarget {
    /// The material whose texture set is painted.
    pub(crate) material: String,
    /// The layer painted into.
    pub(crate) layer: String,
    /// How wide one texture pixel is on the surface under the brush, in
    /// centimetres, if known.
    pub(crate) texel: Option<f32>,
    /// The brush's radius on the surface there, in centimetres.
    pub(crate) radius: Option<f32>,
}

impl AttributesPanel {
    /// The Paint brush section: what it paints with and on.
    pub(super) fn brush_section(&mut self, ui: &mut Ui) {
        let settings = &mut self.brush;
        section(
            ui,
            match settings.brush.kind {
                BrushKind::Paint => tr("Paint brush"),
                BrushKind::Erase => tr("Eraser"),
            },
        );
        egui::Grid::new("attributes_brush")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 6.0))
            .show(ui, |ui| {
                label(
                    ui,
                    tr("Size"),
                    tr("The brush's radius. [ and ] change it, as does Ctrl + right drag (sideways)"),
                );
                ui.horizontal(|ui| {
                    if settings.on_surface {
                        stepper::number(
                            ui,
                            &mut settings.surface_radius,
                            SURFACE_RADIUS,
                            (0.5, 2),
                            tr("Brush size on the surface"),
                        );
                        ui.label(RichText::new(tr("cm")).color(theme::TEXT_WEAK));
                    } else {
                        stepper::number(
                            ui,
                            &mut settings.brush.radius,
                            (
                                *paint::brush::RADIUS_RANGE.start(),
                                *paint::brush::RADIUS_RANGE.end(),
                            ),
                            (1.0, 0),
                            tr("Brush size on screen"),
                        );
                        ui.label(RichText::new(tr("px")).color(theme::TEXT_WEAK));
                    }
                });
                ui.end_row();
                label(
                    ui,
                    tr("Measured"),
                    tr("On screen: the brush stays the same size as you zoom. On the surface: \
                        the same real size on every object, in centimetres"),
                );
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.on_surface, false, tr("On screen"));
                    ui.selectable_value(&mut settings.on_surface, true, tr("On the surface"));
                });
                ui.end_row();
                label(
                    ui,
                    tr("Flow"),
                    tr("How much paint one stroke lays on. Within a stroke paint doesn't pile \
                        up; stroke again to build it up. Ctrl + left drag (sideways) changes it"),
                );
                stepper::percent(ui, &mut settings.brush.flow, (0.0, 1.0), tr("Brush flow"));
                ui.end_row();
                label(
                    ui,
                    tr("Hardness"),
                    tr("How much of the brush paints at full strength: 100% has a crisp edge, \
                        0% fades from the middle. Ctrl + right drag (up and down) changes it"),
                );
                stepper::percent(
                    ui,
                    &mut settings.brush.hardness,
                    (0.0, 1.0),
                    tr("Brush hardness"),
                );
                ui.end_row();
                label(
                    ui,
                    tr("Spacing"),
                    tr("The gap between the brush's dabs along a stroke, as a share of its \
                        width: small for smooth lines"),
                );
                stepper::percent(
                    ui,
                    &mut settings.brush.spacing,
                    (0.01, 4.0),
                    tr("Brush spacing"),
                );
                ui.end_row();
                if settings.brush.kind == BrushKind::Paint {
                    label(ui, tr("Colour"), tr("The colour the brush paints"));
                    let [r, g, b, _] = settings.brush.color;
                    let mut rgb = [r, g, b];
                    let response = egui::color_picker::color_edit_button_srgb(ui, &mut rgb);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::ColorButton,
                            true,
                            tr("Brush colour"),
                        )
                    });
                    settings.brush.color = [rgb[0], rgb[1], rgb[2], 255];
                    ui.end_row();
                }
                label(
                    ui,
                    tr("Paint through"),
                    tr("Paint the parts hidden behind others and facing away too"),
                );
                ui.checkbox(&mut settings.brush.through, "");
                ui.end_row();
            });
        settings.brush = settings.brush.clamped();
        let target = &self.brush_target;
        if !target.material.is_empty() {
            ui.label(
                RichText::new(crate::trf!(
                    "Painting on {} → {}",
                    target.material,
                    target.layer
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        }
        if let (Some(texel), Some(radius)) = (target.texel, target.radius)
            && radius < texel
        {
            ui.label(
                RichText::new(tr(
                    "The brush is smaller than a texture pixel here: its detail won't show. \
                     Make the brush bigger, or the texture size (UV → Texture) larger.",
                ))
                .color(theme::WARNING),
            );
        }
        ui.label(
            RichText::new(tr(
                "1 paint, 2 erase, [ and ] size, Ctrl + drag size, hardness and flow, Esc leave",
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
    }
}
