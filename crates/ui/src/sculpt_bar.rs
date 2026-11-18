//! Sculpt mode's floating brush bar ([decision 0304](../../../docs/decisions/0304-sculpt-mode-and-layers.md)),
//! made like Paint mode's ([decision 0311](../../../docs/decisions/0311-sculpt-mode-looks-like-paint-mode.md)):
//! drawn by the same code ([`crate::paint_bar::floating`]) in the same
//! place, its groups in rows as wide as the view allows. The brush as a
//! button with its picture and a list of every brush, the numbers as bars,
//! the switches as small icons (symmetry, the pen, smoothing), and the
//! falloff, stroke and alpha.

use crate::icons::Icon;
use crate::paint_bar::{icon_switch, switch_named};
use crate::sculpt_tab::{SculptSettings, brush_key_text};
use crate::text::tr;
use crate::theme;
use egui::{
    Frame, Margin, Popup, PopupCloseBehavior, Rect, RectAlign, RichText, Stroke, Ui, WidgetInfo,
    WidgetType, vec2,
};
use sculpt::BrushKind;

/// Accessible name of the brush button, which opens the list of brushes.
pub(crate) const BRUSH_LIST_LABEL: &str = crate::text!("Sculpt brush");
/// Accessible names of the switches.
pub(crate) const SYMMETRY_SWITCH: &str = crate::text!("Sculpt symmetry switch");
pub(crate) const PEN_SWITCH: &str = crate::text!("Sculpt pen pressure switch");
pub(crate) const SMOOTHING_SWITCH: &str = crate::text!("Sculpt smoothing switch");

/// The brush list's width.
const LIST_WIDTH: f32 = 200.0;

/// The bar over the top left of the view `area`, its top at `top`: the
/// brush, its numbers, the switches, and its falloff, stroke and alpha.
pub(crate) fn bar(ctx: &egui::Context, area: Rect, top: f32, settings: &mut SculptSettings) {
    let place = ("sculpt_bar", area, top, crate::sculpt_tab::BAR_LABEL);
    crate::paint_bar::floating(ctx, place, &[0, 1, 2, 3], |ui, g| match g {
        0 => brush_list(ui, settings),
        1 => {
            let kind = settings.brush;
            crate::sculpt_tab::numbers(ui, settings.of_mut(kind));
        }
        2 => switches(ui, settings),
        _ => {
            let kind = settings.brush;
            crate::sculpt_tab::falloff_choice(ui, settings.of_mut(kind));
            crate::sculpt_tab::stroke_choice(ui, settings);
            crate::sculpt_tab::alpha_choice(ui, settings);
        }
    });
}

/// The brush: its picture and name, and a list of every brush.
fn brush_list(ui: &mut Ui, settings: &mut SculptSettings) {
    let response = ui
        .add(egui::Button::new(
            RichText::new(format!("      {}  ⏷", tr(settings.brush.name()))).color(theme::TEXT),
        ))
        .on_hover_text(tr(
            "The sculpting brush: click for every brush. 1 to 7 pick the main ones, and the \
             shelf and the tool menu (E) hold them all. Hold Shift to smooth with any brush",
        ));
    response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, tr(BRUSH_LIST_LABEL)));
    let picture = Rect::from_center_size(
        response.rect.left_center() + vec2(14.0, 0.0),
        vec2(16.0, 16.0),
    );
    crate::icons::paint(
        ui.painter(),
        picture,
        Icon::SculptBrush(settings.brush),
        theme::TEXT,
    );
    Popup::from_toggle_button_response(&response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::BOTTOM_START)
        .gap(6.0)
        .width(LIST_WIDTH)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(6, 6))
                .shadow(ui.visuals().popup_shadow),
        )
        .show(|ui| {
            ui.set_width(LIST_WIDTH - 12.0);
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for kind in BrushKind::ALL {
                        let on = settings.brush == kind;
                        let row = ui
                            .add(
                                egui::Button::selectable(on, format!("      {}", tr(kind.name())))
                                    .min_size(vec2(LIST_WIDTH - 24.0, 24.0)),
                            )
                            .on_hover_text(crate::trf!(
                                "{} ({})",
                                tr(kind.explanation()),
                                brush_key_text(kind)
                            ));
                        let ink = if on {
                            egui::Color32::WHITE
                        } else {
                            theme::TEXT
                        };
                        crate::icons::paint(
                            ui.painter(),
                            Rect::from_center_size(
                                row.rect.left_center() + vec2(14.0, 0.0),
                                vec2(16.0, 16.0),
                            ),
                            Icon::SculptBrush(kind),
                            ink,
                        );
                        if row.clicked() {
                            settings.brush = kind;
                            ui.close();
                        }
                    }
                });
        });
}

/// The axes symmetry mirrors across, in words: "X", "X Y", or "" with
/// none.
pub(crate) fn axes_text(axes: [bool; 3]) -> String {
    ["X", "Y", "Z"]
        .iter()
        .zip(axes)
        .filter(|(_, on)| *on)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The Symmetry, Pen and Smoothing switches, as icons, as Paint mode's
/// Mirror, Pen and Smoothing.
fn switches(ui: &mut Ui, settings: &mut SculptSettings) {
    let symmetry = &mut settings.symmetry;
    let axes = axes_text(symmetry.axes);
    let shown = (!axes.is_empty()).then_some(axes.as_str());
    let mirror = icon_switch(ui, Icon::Symmetry, shown, symmetry.on).on_hover_text(tr(
        "Symmetry: sculpt one side and the other follows, across the object's own axis \
         through its pivot. L turns it on and off; the axes and radial copies are in the \
         Sculpt tab",
    ));
    if switch_named(mirror, symmetry.on, SYMMETRY_SWITCH).clicked() {
        symmetry.on = !symmetry.on;
    }
    let pressure = settings.size_pressure.on || settings.strength_pressure.on;
    let pen = icon_switch(ui, Icon::PaintPen, None, pressure).on_hover_text(tr(
        "Pen pressure: press harder for a bigger, stronger brush. The curves and the least \
         size and strength are in the Sculpt tab",
    ));
    if switch_named(pen, pressure, PEN_SWITCH).clicked() {
        settings.size_pressure.on = !pressure;
        settings.strength_pressure.on = !pressure;
    }
    let smooth = icon_switch(ui, Icon::PaintSmoothing, None, settings.smoothing_on)
        .on_hover_text(tr(
            "Smoothing: the brush trails the mouse on a string, for steady strokes. D turns it \
             on and off; how much is in the Sculpt tab",
        ));
    if switch_named(smooth, settings.smoothing_on, SMOOTHING_SWITCH).clicked() {
        settings.smoothing_on = !settings.smoothing_on;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axes_are_named_in_order() {
        assert_eq!(axes_text([true, false, false]), "X");
        assert_eq!(axes_text([true, false, true]), "X Z");
        assert_eq!(axes_text([false; 3]), "");
    }
}
