//! What each panel shows. For now, most panels are placeholders that say
//! what will appear in them and when.

use crate::icons::{self, Icon};
use crate::theme::{self, tint};
use crate::workbench::AppInfo;
use egui::{Align2, Color32, FontId, Mesh, Rect, RichText, Sense, Ui};

/// Height of a panel's own menu line, in logical pixels.
pub(crate) const MENU_LINE_HEIGHT: f32 = 26.0;

/// A panel's own menu line, along its top: a menu icon, the menus that
/// `add_menus` draws, and a search icon on the right, as in the approved
/// mockup.
pub(crate) fn menu_line_with(ui: &mut Ui, add_menus: impl FnOnce(&mut Ui)) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), MENU_LINE_HEIGHT),
        Sense::hover(),
    );
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, Color32::from_rgb(0x1e, 0x1e, 0x1e)),
    );
    let mut line = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(6.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    line.spacing_mut().item_spacing.x = 0.0;
    line.spacing_mut().button_padding = egui::vec2(7.0, 2.0);
    let (icon, _) = line.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::hover());
    icons::paint(
        line.painter(),
        Rect::from_center_size(icon.center(), egui::vec2(14.0, 14.0)),
        Icon::Menu,
        theme::TEXT_WEAK,
    );
    egui::MenuBar::new().ui(&mut line, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        add_menus(ui);
    });
    line.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let (search, response) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::hover());
        icons::paint(
            ui.painter(),
            Rect::from_center_size(search.center(), egui::vec2(13.0, 13.0)),
            Icon::Search,
            theme::TEXT_WEAK,
        );
        response.on_hover_text("Search arrives in Phase 2");
    });
}

/// A heading and an explanation, centred, for panels with nothing in them.
pub(crate) fn placeholder(ui: &mut Ui, heading: &str, text: &str) {
    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(heading).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(text).weak());
    });
}

/// A rectangle filled with a top-to-bottom colour gradient.
pub(crate) fn vertical_gradient(rect: Rect, top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The Coordinates panel: typed position, size and rotation of the
/// selection, laid out as in the approved mockup. The axis letters are
/// coloured (X red, Y blue, Z green; rotation's H, P and B in grey).
///
/// Until something can be selected (Phase 3), the fields show zeros and
/// can't be edited; hovering one says why.
pub(crate) fn coordinates_contents(ui: &mut Ui, units: crate::units::Units) {
    let position = [
        ("X", tint::AXIS_X),
        ("Y", tint::AXIS_Y),
        ("Z", tint::AXIS_Z),
    ];
    let rotation = [
        ("H", theme::TEXT_WEAK),
        ("P", theme::TEXT_WEAK),
        ("B", theme::TEXT_WEAK),
    ];
    let groups = [
        ("Position", position, units.symbol()),
        ("Size", position, units.symbol()),
        ("Rotation", rotation, "°"),
    ];
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        for (title, axes, unit) in groups {
            ui.vertical(|ui| {
                ui.set_width(COORDINATE_GROUP_WIDTH);
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(RichText::new(title).small().color(theme::TEXT_WEAK));
                for (axis, color) in axes {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (letter, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, FIELD_HEIGHT), Sense::hover());
                        ui.painter().text(
                            letter.center(),
                            Align2::CENTER_CENTER,
                            axis,
                            FontId::proportional(13.0),
                            color,
                        );
                        coordinate_field(ui, &format!("{title} {axis}"), &format!("0 {unit}"));
                    });
                }
            });
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let why = "Select something to type its coordinates";
                ui.add_enabled(
                    false,
                    egui::Button::new("Object (Rel)  ⏷")
                        .fill(theme::HEADER)
                        .min_size(egui::vec2(120.0, FIELD_HEIGHT)),
                )
                .on_disabled_hover_text(
                    "Whether values are relative to the object or the world. Arrives in Phase 3.",
                );
                ui.add_enabled(
                    false,
                    egui::Button::new("Apply").min_size(egui::vec2(120.0, FIELD_HEIGHT)),
                )
                .on_disabled_hover_text(why);
            });
        });
    });
}

/// Width of one group (Position, Size or Rotation) in the Coordinates panel.
const COORDINATE_GROUP_WIDTH: f32 = 200.0;
/// Height of a value field.
const FIELD_HEIGHT: f32 = 22.0;

/// A value field, drawn as in the mockup: dark, rounded, filling the rest
/// of the row. Read-only until there is a selection to edit.
fn coordinate_field(ui: &mut Ui, label: &str, value: &str) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, FIELD_HEIGHT), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, false, label));
    let painter = ui.painter();
    painter.rect(
        rect,
        4.0,
        theme::FIELD,
        egui::Stroke::new(1.0, theme::FIELD_BORDER),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        value,
        FontId::proportional(13.0),
        theme::TEXT_STRONG.gamma_multiply(0.85),
    );
    response.on_hover_text("Select something to type its coordinates");
}

/// The About box.
pub(crate) fn about_contents(ui: &mut Ui, info: &AppInfo) {
    ui.vertical_centered(|ui| {
        ui.heading(&info.name);
        ui.label(format!("Version {}", info.version));
        ui.add_space(8.0);
        ui.label("A calm, tool-rich 3D modeller for game assets.");
        ui.label(format!("Part of the {} family of tools.", info.family));
        ui.add_space(8.0);
        ui.label("Created by Rhyslos, built with AI assistance (Claude by Anthropic).");
        ui.label("Free software under the GNU General Public License v3.0 or later.");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn gradient_runs_from_top_colour_to_bottom_colour() {
        let (top, bottom) = (Color32::from_gray(80), Color32::from_gray(40));
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        let mesh = vertical_gradient(rect, top, bottom);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        for v in &mesh.vertices {
            let expected = if v.pos.y == 0.0 { top } else { bottom };
            assert_eq!(v.color, expected);
        }
    }
}
