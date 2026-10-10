//! Everything drawn around and over the 3D image: the viewport's own menu
//! line along the top (with buttons you can drag to navigate), and the
//! quiet labels over the image (view name, camera, axis marker, grid
//! spacing).

use super::{ViewportPanel, cursor_for};
use crate::icons::{self, Icon};
use crate::theme::{self, backdrop, tint};
use egui::{
    Align, Align2, CursorIcon, FontId, Layout, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2,
};
use glam::{Vec2, Vec3};
use tools::DragKind;

/// Height of the viewport's menu line, in logical pixels.
pub(super) const HEADER_HEIGHT: f32 = 26.0;

/// Accessible names of the menu line's navigation buttons.
pub(crate) const PAN_BUTTON: &str = "Drag to pan the view";
/// See [`PAN_BUTTON`].
pub(crate) const ZOOM_BUTTON: &str = "Drag to zoom the view";
/// See [`PAN_BUTTON`].
pub(crate) const ORBIT_BUTTON: &str = "Drag to orbit the view";

/// What arrives with the rest of Phase 1, for greyed-out menu entries.
const LATER_IN_PHASE_1: &str = "Arrives later in Phase 1";

/// The menu line along the top of the viewport: its menus on the left and
/// the navigation buttons on the right. `size` is the 3D image's size, so
/// dragging a button moves the view as much as dragging in the image.
pub(super) fn header(ui: &mut Ui, rect: Rect, panel: &mut ViewportPanel, size: Vec2) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme::VIEWPORT_HEADER);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    let inner = rect.shrink2(vec2(6.0, 0.0));
    let mut ui = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center))
            .id_salt("viewport_header"),
    );
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.spacing_mut().button_padding = vec2(7.0, 2.0);
    let (menu_icon, _) = ui.allocate_exact_size(vec2(22.0, 20.0), Sense::hover());
    icons::paint(
        ui.painter(),
        Rect::from_center_size(menu_icon.center(), vec2(15.0, 15.0)),
        Icon::Menu,
        theme::TEXT_WEAK,
    );
    // Inside a menu bar, so the menus look like plain words, as on the
    // main menu bar, rather than buttons.
    egui::MenuBar::new().ui(&mut ui, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        menus(ui, panel);
    });
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let maximise = icon_button(ui, Icon::Maximise, "Maximise this view", Sense::hover());
        maximise.on_hover_text(
            "Maximise this view\nArrives with the four-view layout, later in Phase 1",
        );
        for (icon, label, kind) in [
            (Icon::Rotate, ORBIT_BUTTON, DragKind::Orbit),
            (Icon::Dolly, ZOOM_BUTTON, DragKind::Dolly),
            (Icon::Move, PAN_BUTTON, DragKind::Pan),
        ] {
            let response = icon_button(ui, icon, label, Sense::drag());
            if response.dragged() {
                panel.button_drag(kind, response.drag_delta(), size);
                ui.ctx().set_cursor_icon(cursor_for(kind));
            } else if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
            response.on_hover_text(label);
        }
    });
}

/// The viewport's menus: View, Cameras, Display, Options and Panel.
fn menus(ui: &mut Ui, panel: &mut ViewportPanel) {
    ui.menu_button("View", |ui| {
        if ui
            .button("Reset view")
            .on_hover_text("Go back to the opening camera view")
            .clicked()
        {
            panel.reset_view();
            ui.close();
        }
        ui.separator();
        for item in ["Frame all", "Frame selected"] {
            ui.add_enabled(false, egui::Button::new(item))
                .on_disabled_hover_text(LATER_IN_PHASE_1);
        }
    });
    ui.menu_button("Cameras", |ui| {
        let _ = ui.radio(true, "Perspective");
        for item in ["Top", "Front", "Right"] {
            ui.add_enabled(false, egui::RadioButton::new(false, item))
                .on_disabled_hover_text(LATER_IN_PHASE_1);
        }
    });
    ui.menu_button("Display", |ui| {
        ui.checkbox(
            &mut panel.options.fine_grid,
            crate::palette::GRID_DETAIL_LABEL,
        )
        .on_hover_text("Show the grid's 10 cm lines as well as the 1 m lines");
    });
    ui.menu_button("Options", |ui| {
        if ui
            .button(format!("{}…", super::PICKER_LABEL))
            .on_hover_text("Choose which mouse buttons and keys move the view")
            .clicked()
        {
            panel.open_picker = true;
            ui.close();
        }
        let navigation = &mut panel.options.navigation;
        ui.checkbox(
            &mut navigation.around_cursor,
            super::picker::AROUND_CURSOR_LABEL,
        );
        ui.checkbox(&mut navigation.invert_orbit, "Invert orbit direction");
        ui.checkbox(
            &mut navigation.alt_everywhere,
            "Alt + mouse works in every scheme",
        );
    });
    ui.menu_button("Panel", |ui| {
        ui.add_enabled(false, egui::Button::new("Four views"))
            .on_disabled_hover_text(LATER_IN_PHASE_1);
    });
}

/// A small icon button on the viewport's menu line.
fn icon_button(ui: &mut Ui, icon: Icon, label: &str, sense: Sense) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 22.0), sense);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let color = if response.hovered() || response.dragged() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
        theme::TEXT_STRONG
    } else {
        theme::TEXT_WEAK
    };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(15.0, 15.0)),
        icon,
        color,
    );
    response
}

/// The calm backdrop and a plain-language reason, when there is no image.
pub(super) fn no_image(ui: &mut Ui, rect: Rect, reason: &str) {
    let horizon = rect.top() + rect.height() * 0.4;
    let sky = Rect::from_min_max(rect.min, pos2(rect.right(), horizon));
    let ground = Rect::from_min_max(pos2(rect.left(), horizon), rect.max);
    let painter = ui.painter_at(rect);
    painter.add(crate::panels::vertical_gradient(
        sky,
        backdrop::SKY_TOP,
        backdrop::HORIZON,
    ));
    painter.rect_filled(ground, 0.0, backdrop::GROUND);
    // A real label (not just painted text), so screen readers and tests
    // can read the reason. Not selectable, so dragging across it still
    // moves the camera underneath.
    ui.put(
        Rect::from_center_size(rect.center(), vec2(rect.width(), 40.0)),
        egui::Label::new(
            egui::RichText::new(format!("The 3D view can't be shown: {reason}."))
                .color(theme::TEXT),
        )
        .selectable(false),
    );
}

/// Quiet labels over the image: the view's name, the camera, an axis
/// marker, and the grid spacing.
pub(super) fn overlays(ui: &Ui, rect: Rect, panel: &ViewportPanel) {
    let painter = ui.painter_at(rect);
    let small = FontId::proportional(12.0);
    painter.text(
        rect.left_top() + vec2(10.0, 8.0),
        Align2::LEFT_TOP,
        "Perspective",
        small.clone(),
        theme::TEXT_STRONG,
    );
    painter.text(
        pos2(rect.center().x, rect.top() + 8.0),
        Align2::CENTER_TOP,
        "Default camera",
        small.clone(),
        theme::TEXT_WEAK,
    );
    let spacing = if panel.options.fine_grid {
        "10 cm"
    } else {
        "1 m"
    };
    painter.text(
        rect.left_bottom() + vec2(10.0, -10.0),
        Align2::LEFT_BOTTOM,
        format!("Grid spacing: {spacing}"),
        small,
        theme::TEXT_WEAK,
    );
    axis_marker(&painter, rect.right_top() + vec2(-36.0, 36.0), panel);
    if matches!(panel.drag, Some(DragKind::Orbit | DragKind::Dolly)) {
        pivot_marker(ui, &painter, rect, panel);
    }
}

/// A small ring and dot on the point the view turns around or zooms
/// towards, shown while orbiting or zooming with a drag.
fn pivot_marker(ui: &Ui, painter: &egui::Painter, rect: Rect, panel: &ViewportPanel) {
    let size = Vec2::new(rect.width(), rect.height());
    let Some(at) = panel.camera().project(panel.navigator().pivot(), size) else {
        return;
    };
    let centre = rect.min + vec2(at.x, at.y);
    if !rect.contains(centre) {
        return;
    }
    let accent = theme::accent(ui.ctx()).colors();
    let shadow = Stroke::new(3.5, egui::Color32::from_black_alpha(110));
    painter.circle_stroke(centre, 7.0, shadow);
    painter.circle_stroke(centre, 7.0, Stroke::new(1.5, accent.light));
    painter.circle_filled(centre, 2.0, accent.light);
}

/// Screen directions of the world axes as the camera sees them, `length`
/// long, farthest first so nearer axes are drawn on top.
pub(super) fn axis_directions(
    camera: &tools::Camera,
    length: f32,
) -> Vec<(&'static str, egui::Color32, egui::Vec2)> {
    let (right, up, forward) = (camera.right(), camera.up(), camera.forward());
    let mut axes: Vec<_> = [
        ("X", tint::AXIS_X, Vec3::X),
        ("Y", tint::AXIS_Y, Vec3::Y),
        ("Z", tint::AXIS_Z, Vec3::Z),
    ]
    .into_iter()
    .map(|(name, color, dir)| {
        let screen = vec2(dir.dot(right), -dir.dot(up)) * length;
        (name, color, screen, dir.dot(forward))
    })
    .collect();
    axes.sort_by(|a, b| b.3.total_cmp(&a.3));
    axes.into_iter()
        .map(|(name, color, screen, _)| (name, color, screen))
        .collect()
}

/// A small marker showing which way X, Y and Z point from this view.
fn axis_marker(painter: &egui::Painter, centre: egui::Pos2, panel: &ViewportPanel) {
    for (name, color, dir) in axis_directions(panel.camera(), 18.0) {
        painter.line_segment([centre, centre + dir], Stroke::new(1.6, color));
        // An axis pointing straight at or away from you is just a dot;
        // its letter then sits on the centre.
        let away = if dir.length() > 1.0 {
            dir.normalized() * 7.0
        } else {
            egui::Vec2::ZERO
        };
        let label_at = centre + dir + away;
        painter.text(
            label_at,
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(10.0),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axis_marker_matches_the_opening_view() {
        let axes = axis_directions(&tools::Camera::default(), 10.0);
        let get = |n| axes.iter().find(|(name, _, _)| *name == n).unwrap().2;
        assert!(get("X").x > 0.0, "X points right");
        assert!(get("Z").y < 0.0, "Z points up the screen");
        assert!(get("Z").x.abs() < 0.01, "Z stays upright");
        for (_, _, d) in &axes {
            assert!(d.length() <= 10.0 + 1e-3);
        }
    }

    #[test]
    fn the_axis_marker_draws_far_axes_first() {
        // Looking along +Y, the Y axis points away and is drawn first.
        let camera = tools::Camera::looking_at(Vec3::new(0.0, -500.0, 0.0), Vec3::ZERO);
        let axes = axis_directions(&camera, 10.0);
        assert_eq!(axes[0].0, "Y");
        assert!(axes[0].2.length() < 0.01, "pointing straight away: a dot");
    }
}
