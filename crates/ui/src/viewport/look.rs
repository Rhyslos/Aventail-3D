//! The look row in the viewport's bottom-left corner, beside the axis
//! marker: a button naming the shading in use (Shaded, Shaded with wires,
//! Wireframe), which opens the list to choose from
//! ([decision 0063](../../../../docs/decisions/0063-wireframe.md)).

use super::{Shading, ViewportPanel};
use crate::theme;
use egui::{
    FontId, Frame, Margin, Popup, PopupCloseBehavior, Rect, RectAlign, Sense, Stroke, Ui,
    WidgetInfo, WidgetType, pos2, vec2,
};

/// Accessible name of the shading button, and the heading of its list and
/// of the Display menu's choices.
pub(crate) const SHADING_LABEL: &str = "Shading";

/// How far the row starts from the view's left edge: past the axis
/// marker.
pub(super) const ROW_LEFT: f32 = 82.0;
/// The row's buttons' height, as the navigation picker's.
const HEIGHT: f32 = 26.0;
/// The shading list's width, fixed so it never changes size as the mouse
/// moves over it (a tooltip appearing used to make it narrower).
const LIST_WIDTH: f32 = 180.0;

/// Draws the shading button at the bottom left of `rect` (past the axis
/// marker), and its list when open.
pub(super) fn show(ui: &mut Ui, rect: Rect, panel: &mut ViewportPanel) {
    let shading = panel.options.shading;
    let font = FontId::proportional(12.0);
    let name = ui
        .painter()
        .layout_no_wrap(shading.label().to_string(), font, theme::TEXT);
    let size = vec2(name.size().x + 44.0, HEIGHT);
    let button = Rect::from_min_size(
        pos2(rect.left() + ROW_LEFT, rect.bottom() - HEIGHT - 8.0),
        size,
    );
    let response = ui.interact(button, ui.id().with("shading_button"), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, SHADING_LABEL));
    let painter = ui.painter();
    let fill = if response.hovered() {
        theme::HOVER
    } else {
        theme::PALETTE
    };
    painter.rect(
        button,
        5.0,
        fill,
        Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    swatch(painter, button.left_center() + vec2(14.0, 0.0), shading);
    painter.galley(
        pos2(
            button.left() + 26.0,
            button.center().y - name.size().y / 2.0,
        ),
        name,
        theme::TEXT,
    );
    // A small chevron pointing up: the list opens upwards.
    let tip = pos2(button.right() - 11.0, button.center().y - 2.0);
    painter.line(
        vec![tip + vec2(-3.5, 3.0), tip, tip + vec2(3.5, 3.0)],
        Stroke::new(1.5, theme::TEXT_WEAK),
    );
    let response = response.on_hover_text("How shapes are drawn: shaded, with wires, or wireframe");
    Popup::from_toggle_button_response(&response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::TOP_START)
        .gap(6.0)
        .width(LIST_WIDTH)
        .frame(
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(6.0)
                .inner_margin(Margin::symmetric(8, 8))
                .shadow(ui.visuals().popup_shadow),
        )
        .show(|ui| {
            ui.set_width(LIST_WIDTH - 16.0);
            choices(ui, &mut panel.options.shading);
            // Cavity's switch, in the same list (decision 0066); it keeps
            // the list open, so it can be tried on and off.
            ui.separator();
            super::chrome::cavity_entry(ui, &mut panel.options);
        });
}

/// The shading choices, under a heading: in the button's list and the
/// Display menu.
pub(crate) fn choices(ui: &mut Ui, shading: &mut Shading) {
    ui.label(egui::RichText::new(SHADING_LABEL).color(theme::TEXT_WEAK));
    for look in Shading::ALL {
        if ui
            .add(
                egui::Button::selectable(*shading == look, look.label())
                    .wrap_mode(egui::TextWrapMode::Extend),
            )
            .on_hover_text(look.explanation())
            .clicked()
        {
            *shading = look;
            ui.close();
        }
    }
}

/// A small square showing the look: filled for shaded, filled and crossed
/// for shaded with wires, an outline crossed for wireframe.
fn swatch(painter: &egui::Painter, centre: egui::Pos2, shading: Shading) {
    let square = Rect::from_center_size(centre, vec2(11.0, 11.0));
    let line = Stroke::new(1.2, theme::TEXT);
    if shading.shows_faces() {
        painter.rect_filled(square, 2.0, theme::TEXT_WEAK);
    }
    painter.rect_stroke(square, 2.0, line, egui::StrokeKind::Inside);
    if shading != Shading::Shaded {
        let wire = Stroke::new(1.0, theme::TEXT_STRONG);
        painter.line_segment([square.left_top(), square.right_bottom()], wire);
        painter.line_segment([square.center_top(), square.center_bottom()], wire);
        painter.line_segment([square.left_center(), square.right_center()], wire);
    }
}
