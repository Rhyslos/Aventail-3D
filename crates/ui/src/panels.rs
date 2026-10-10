//! What each panel shows. For now, most panels are placeholders that say
//! what will appear in them and when.

use crate::workbench::AppInfo;
use egui::{Align2, Color32, FontId, Mesh, Rect, RichText, Sense, Ui};

/// Top colour of the viewport's background gradient.
const VIEWPORT_TOP: Color32 = Color32::from_rgb(92, 95, 100);
/// Bottom colour of the viewport's background gradient.
const VIEWPORT_BOTTOM: Color32 = Color32::from_rgb(57, 59, 63);

/// A heading and an explanation, centred, for panels with nothing in them.
pub(crate) fn placeholder(ui: &mut Ui, heading: &str, text: &str) {
    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(heading).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(text).weak());
    });
}

/// The viewport: a calm gradient for now. The 3D view arrives in Phase 1.
pub(crate) fn viewport_contents(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.add(vertical_gradient(rect, VIEWPORT_TOP, VIEWPORT_BOTTOM));
    let text = Color32::from_rgb(233, 233, 233);
    painter.text(
        rect.left_top() + egui::vec2(14.0, 10.0),
        Align2::LEFT_TOP,
        "Perspective",
        FontId::proportional(13.0),
        text,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "The 3D view arrives in Phase 1",
        FontId::proportional(14.0),
        text.gamma_multiply(0.6),
    );
}

/// A rectangle filled with a top-to-bottom colour gradient.
fn vertical_gradient(rect: Rect, top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The Attribute Manager: settings of the selected object or active tool.
pub(crate) fn attributes_contents(ui: &mut Ui) {
    placeholder(
        ui,
        "Nothing selected",
        "Select an object or pick a tool, and its settings show up here.",
    );
}

/// The Object Manager: the tree of objects in the scene.
pub(crate) fn objects_contents(ui: &mut Ui) {
    placeholder(
        ui,
        "No objects yet",
        "Objects you create appear here as a tree. Primitives arrive in Phase 2.",
    );
}

/// The Coordinates panel: typed position, size and rotation of the
/// selection. Shown with zeros and greyed out until there is a selection.
pub(crate) fn coordinates_contents(ui: &mut Ui) {
    let groups = [
        ("Position", ["X", "Y", "Z"], "cm"),
        ("Size", ["X", "Y", "Z"], "cm"),
        ("Rotation", ["H", "P", "B"], "°"),
    ];
    ui.add_space(4.0);
    ui.horizontal_top(|ui| {
        for (title, axes, unit) in groups {
            ui.vertical(|ui| {
                ui.set_width(170.0);
                ui.label(RichText::new(title).weak());
                for axis in axes {
                    ui.horizontal(|ui| {
                        ui.label(axis);
                        let mut value = 0.0_f64;
                        ui.add_enabled(
                            false,
                            egui::DragValue::new(&mut value).suffix(format!(" {unit}")),
                        )
                        .on_disabled_hover_text("Select something to type its coordinates");
                    });
                }
            });
            ui.add_space(12.0);
        }
        ui.add_enabled(false, egui::Button::new("Apply"))
            .on_disabled_hover_text("Select something to type its coordinates");
    });
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

/// The viewport gradient's colour halfway down, for tests that look at
/// what was drawn.
pub fn viewport_middle_colour() -> Color32 {
    VIEWPORT_TOP.lerp_to_gamma(VIEWPORT_BOTTOM, 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn gradient_runs_from_top_colour_to_bottom_colour() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 50.0));
        let mesh = vertical_gradient(rect, VIEWPORT_TOP, VIEWPORT_BOTTOM);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        for v in &mesh.vertices {
            let expected = if v.pos.y == 0.0 {
                VIEWPORT_TOP
            } else {
                VIEWPORT_BOTTOM
            };
            assert_eq!(v.color, expected);
        }
    }

    #[test]
    fn viewport_is_a_calm_mid_grey() {
        // Not black, not white, and nearly neutral: the calm look from the
        // design (DESIGN.md §5.2).
        let c = viewport_middle_colour();
        let (r, g, b) = (c.r() as i32, c.g() as i32, c.b() as i32);
        assert!((50..=110).contains(&r));
        assert!((r - b).abs() < 12 && (g - b).abs() < 12);
    }
}
