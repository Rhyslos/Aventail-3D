//! Sculpt mode's settings and where they are shown ([decision 0304](../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! the brushes' settings (each brush keeps its own size and strength), the
//! sculpt's look, the Sculpt tab beside Attributes, the shelf of brushes
//! under the view, and the brush bar floating over the view's top (in the
//! paint brush bar's look, sharing its number bars).

use crate::paint_bar::{Shown, fill_field, group, weak};
use crate::text::tr;
use crate::theme;
use egui::{Pos2, Rect, RichText, Ui, WidgetInfo, WidgetType};
use sculpt::{BrushKind, BrushSettings, Falloff};

/// Accessible name of the floating sculpt brush bar.
pub(crate) const BAR_LABEL: &str = crate::text!("Sculpt brush bar");
/// Accessible names of its numbers.
pub(crate) const SIZE_LABEL: &str = crate::text!("Sculpt brush size");
pub(crate) const STRENGTH_LABEL: &str = crate::text!("Sculpt brush strength");
pub(crate) const SPACING_LABEL: &str = crate::text!("Sculpt brush spacing");

/// How the sculpt is shown in Sculpt mode (decision 0304): a matcap of
/// the app's own, or the app's usual lit look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SculptLook {
    /// Warm clay: the standard.
    #[default]
    Clay,
    /// Red wax, glossy: shows small detail.
    RedWax,
    /// Pale stone, matte.
    PaleStone,
    /// The app's usual lit look, with the objects' colours.
    Lit,
}

impl SculptLook {
    /// Every look, in menu order.
    pub(crate) const ALL: [Self; 4] = [Self::Clay, Self::RedWax, Self::PaleStone, Self::Lit];

    /// Its name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Clay => tr("Clay"),
            Self::RedWax => tr("Red wax"),
            Self::PaleStone => tr("Pale stone"),
            Self::Lit => tr("Lit"),
        }
    }

    /// The matcap the renderer draws (`render::SurfaceDraw::Matcap`), or
    /// `None` for the usual look.
    pub(crate) fn matcap(self) -> Option<u8> {
        match self {
            Self::Clay => Some(0),
            Self::RedWax => Some(1),
            Self::PaleStone => Some(2),
            Self::Lit => None,
        }
    }
}

/// Sculpt mode's settings.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SculptSettings {
    /// The brush chosen (Smooth is also held with Shift).
    pub(crate) brush: BrushKind,
    /// Draw's settings.
    pub(crate) draw: BrushSettings,
    /// Smooth's settings.
    pub(crate) smooth: BrushSettings,
    /// How the sculpt is shown.
    pub(crate) look: SculptLook,
    /// Whether creases are darkened and ridges lit (cavity).
    pub(crate) cavity: bool,
    /// What is being sculpted, in plain words, set by the workbench each
    /// frame for the Sculpt tab.
    pub(crate) status: String,
}

impl Default for SculptSettings {
    fn default() -> Self {
        Self {
            brush: BrushKind::Draw,
            draw: BrushSettings::standard(BrushKind::Draw),
            smooth: BrushSettings::standard(BrushKind::Smooth),
            look: SculptLook::Clay,
            cavity: true,
            status: String::new(),
        }
    }
}

impl SculptSettings {
    /// Brush `kind`'s settings.
    pub(crate) fn of(&self, kind: BrushKind) -> BrushSettings {
        match kind {
            BrushKind::Draw => self.draw,
            BrushKind::Smooth => self.smooth,
        }
    }

    /// Brush `kind`'s settings, to change.
    pub(crate) fn of_mut(&mut self, kind: BrushKind) -> &mut BrushSettings {
        match kind {
            BrushKind::Draw => &mut self.draw,
            BrushKind::Smooth => &mut self.smooth,
        }
    }

    /// Scales the chosen brush's size by `factor` ([ and ], Ctrl + right
    /// drag).
    pub(crate) fn scale_size(&mut self, factor: f32) {
        let brush = self.of_mut(self.brush);
        brush.radius *= factor;
        *brush = brush.clamped();
    }

    /// Adds `amount` to the chosen brush's strength (Ctrl + right drag up
    /// and down).
    pub(crate) fn add_strength(&mut self, amount: f32) {
        let brush = self.of_mut(self.brush);
        brush.strength += amount;
        *brush = brush.clamped();
    }
}

/// Size, Strength and Spacing of `settings`, as bars filled to their
/// value.
fn numbers(ui: &mut Ui, settings: &mut BrushSettings) {
    weak(ui, tr("Size"));
    let (low, high) = sculpt::brush::RADIUS_RANGE;
    fill_field(
        ui,
        tr(SIZE_LABEL),
        &mut settings.radius,
        (low, high),
        Shown {
            scale: 1.0,
            suffix: " px",
            decimals: 0,
            eased: true,
            size: true,
        },
    )
    .on_hover_text(tr(
        "The brush's radius on screen: drag sideways or click to type. [ and ] change it, as \
         does Ctrl + right drag (sideways)",
    ));
    let percent = Shown {
        scale: 100.0,
        suffix: "%",
        decimals: 0,
        eased: false,
        size: false,
    };
    weak(ui, tr("Strength"));
    fill_field(
        ui,
        tr(STRENGTH_LABEL),
        &mut settings.strength,
        (0.0, 1.0),
        percent,
    )
    .on_hover_text(tr(
        "How much each dab changes the surface. Ctrl + right drag (up and down) changes it",
    ));
    weak(ui, tr("Spacing"));
    let (low, high) = sculpt::brush::SPACING_RANGE;
    fill_field(
        ui,
        tr(SPACING_LABEL),
        &mut settings.spacing,
        (low, high),
        percent,
    )
    .on_hover_text(tr(
        "The gap between the brush's dabs along a stroke, as a share of its radius: small for \
         smooth strokes",
    ));
    *settings = settings.clamped();
}

/// The brush bar over the top left of the view `area`, its top at `top`
/// (decision 0304): the brush, its numbers and its falloff.
pub(crate) fn bar(ctx: &egui::Context, area: Rect, top: f32, settings: &mut SculptSettings) {
    let at = Pos2::new(area.left() + crate::paint_bar::INSET, top);
    egui::Area::new(egui::Id::new("sculpt_bar"))
        .order(egui::Order::Middle)
        .fixed_pos(at)
        .constrain_to(area)
        .show(ctx, |ui| {
            let response = ui
                .horizontal_wrapped(|ui| {
                    ui.set_max_width((area.width() - 16.0).max(120.0));
                    group(ui, |ui| brush_choice(ui, settings));
                    let kind = settings.brush;
                    group(ui, |ui| numbers(ui, settings.of_mut(kind)));
                    group(ui, |ui| falloff_choice(ui, settings.of_mut(kind)));
                })
                .response;
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, tr(BAR_LABEL)));
        });
}

/// The brushes as buttons, the chosen one lit.
fn brush_choice(ui: &mut Ui, settings: &mut SculptSettings) {
    for kind in BrushKind::ALL {
        let on = settings.brush == kind;
        let key = match kind {
            BrushKind::Draw => tr("1"),
            BrushKind::Smooth => tr("hold Shift"),
        };
        if ui
            .selectable_label(on, tr(kind.name()))
            .on_hover_text(crate::trf!("{} ({})", tr(kind.explanation()), key))
            .clicked()
        {
            settings.brush = kind;
        }
    }
}

/// The falloff as a drop-down.
fn falloff_choice(ui: &mut Ui, brush: &mut BrushSettings) {
    weak(ui, tr("Falloff"));
    egui::ComboBox::from_id_salt("sculpt_falloff")
        .selected_text(tr(brush.falloff.name()))
        .show_ui(ui, |ui| {
            for f in Falloff::ALL {
                ui.selectable_value(&mut brush.falloff, f, tr(f.name()));
            }
        })
        .response
        .on_hover_text(tr(
            "How the brush's strength fades from its middle to its rim: Smooth for soft forms, \
             Sharp and Spike for creases and points, Flat for an even push",
        ));
}

/// The Sculpt tab beside Attributes (decision 0304): what is sculpted,
/// the brush's settings, and how the sculpt is shown.
pub(crate) fn panel(ui: &mut Ui, settings: &mut SculptSettings) {
    crate::attributes::section(ui, tr("Sculpting"));
    ui.label(RichText::new(settings.status.as_str()).color(theme::TEXT_WEAK));
    ui.add_space(6.0);
    crate::attributes::section(ui, tr("Brush"));
    ui.horizontal_wrapped(|ui| brush_choice(ui, settings));
    ui.label(RichText::new(tr(settings.brush.explanation())).color(theme::TEXT_WEAK));
    let kind = settings.brush;
    ui.horizontal_wrapped(|ui| numbers(ui, settings.of_mut(kind)));
    ui.horizontal(|ui| falloff_choice(ui, settings.of_mut(kind)));
    ui.add_space(6.0);
    crate::attributes::section(ui, tr("Look"));
    ui.horizontal_wrapped(|ui| {
        for look in SculptLook::ALL {
            ui.selectable_value(&mut settings.look, look, look.name())
                .on_hover_text(tr(
                    "How the sculpt is shown while you sculpt: a matcap (a lit material picture \
                     wrapped on the shape) makes forms read clearly. Only the view changes",
                ));
        }
    });
    ui.checkbox(&mut settings.cavity, tr("Cavity"))
        .on_hover_text(tr("Darkens creases and lights ridges, so detail reads clearly"));
}

/// The shelf of brushes under the view in Sculpt mode: a card per brush
/// with what it does.
pub(crate) fn shelf(ui: &mut Ui, settings: &mut SculptSettings) {
    ui.horizontal_wrapped(|ui| {
        for kind in BrushKind::ALL {
            let on = settings.brush == kind;
            let frame = egui::Frame::new()
                .fill(if on {
                    theme::accent(ui.ctx()).colors().fill
                } else {
                    theme::FIELD
                })
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::same(8));
            let response = frame
                .show(ui, |ui| {
                    ui.set_width(220.0);
                    ui.label(RichText::new(tr(kind.name())).strong());
                    ui.label(RichText::new(tr(kind.explanation())).small());
                })
                .response
                .interact(egui::Sense::click());
            response.widget_info(|| {
                WidgetInfo::selected(WidgetType::Button, true, on, tr(kind.name()))
            });
            if response.clicked() {
                settings.brush = kind;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_brush_keeps_its_own_size_and_strength() {
        let mut s = SculptSettings::default();
        s.scale_size(2.0);
        s.add_strength(0.3);
        assert_eq!(s.draw.radius, 100.0);
        assert!((s.draw.strength - 0.8).abs() < 1e-6);
        s.brush = BrushKind::Smooth;
        assert_eq!(s.of(BrushKind::Smooth).radius, 50.0);
        s.scale_size(1e9);
        assert_eq!(s.smooth.radius, sculpt::brush::RADIUS_RANGE.1);
        s.add_strength(-5.0);
        assert_eq!(s.smooth.strength, 0.0);
        assert_eq!(s.draw.radius, 100.0);
    }

    #[test]
    fn every_look_but_lit_is_a_matcap() {
        let matcaps: Vec<_> = SculptLook::ALL.iter().map(|l| l.matcap()).collect();
        assert_eq!(matcaps, vec![Some(0), Some(1), Some(2), None]);
    }
}
