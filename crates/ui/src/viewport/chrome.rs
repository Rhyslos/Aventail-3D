//! Everything drawn around and over the 3D image: the viewport's own menu
//! line along the top (with buttons you can drag to navigate), and the
//! quiet labels over the image (view name, camera, axis marker, grid
//! spacing).

use super::{FOUR_VIEWS_KEY, VIEW_KEYS, View, ViewOptions, ViewportPanel, cursor_for};
use crate::icons::{self, Icon};
use crate::theme::{self, backdrop, tint};
use egui::{
    Align, Align2, CursorIcon, FontId, Layout, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2,
};
use glam::{Vec2, Vec3};
use tools::{DragKind, ViewKind};

/// Height of the viewport's menu line, in logical pixels.
pub(super) const HEADER_HEIGHT: f32 = 26.0;

/// Accessible names of the menu line's navigation buttons.
pub(crate) const PAN_BUTTON: &str = "Drag to pan the view";
/// See [`PAN_BUTTON`].
pub(crate) const ZOOM_BUTTON: &str = "Drag to zoom the view";
/// See [`PAN_BUTTON`].
pub(crate) const ORBIT_BUTTON: &str = "Drag to orbit the view";

/// Accessible name of the button that switches between one view and
/// four, when one view is showing.
pub(crate) const FOUR_VIEWS_BUTTON: &str = "Show four views";
/// See [`FOUR_VIEWS_BUTTON`]; when four views are showing.
pub(crate) const MAXIMISE_BUTTON: &str = "Maximise this view";

/// The name of the Panel menu's entry for the four-view layout.
pub(crate) const FOUR_VIEWS_ENTRY: &str = "Four views";

/// The menu line along the top of the viewport: its menus on the left and
/// the navigation buttons on the right, which act on the active view.
pub(super) fn header(ui: &mut Ui, rect: Rect, panel: &mut ViewportPanel) {
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
        let four = panel.four_views();
        let (label, explanation) = if four {
            (MAXIMISE_BUTTON, "Show only the active view (middle click)")
        } else {
            (
                FOUR_VIEWS_BUTTON,
                "Show the perspective, top, front and right views at once (middle click)",
            )
        };
        let layout = icon_button(ui, Icon::Maximise, label, Sense::click());
        if layout.clicked() {
            panel.set_four_views(!four);
        }
        layout.on_hover_text(format!("{label}\n{explanation}"));
        for (icon, label, kind) in [
            (Icon::Rotate, ORBIT_BUTTON, DragKind::Orbit),
            (Icon::Dolly, ZOOM_BUTTON, DragKind::Dolly),
            (Icon::Move, PAN_BUTTON, DragKind::Pan),
        ] {
            let response = icon_button(ui, icon, label, Sense::drag());
            if response.dragged() {
                panel.button_drag(kind, response.drag_delta());
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
        frame_entries(ui, panel);
    });
    ui.menu_button("Cameras", |ui| {
        let active = panel.active_kind();
        for kind in ViewKind::ALL {
            let key = VIEW_KEYS
                .iter()
                .find(|(_, k)| *k == kind)
                .map(|(key, _)| key.name())
                .unwrap_or_default();
            if ui
                .add(egui::Button::selectable(kind == active, kind.name()).shortcut_text(key))
                .on_hover_text(camera_explanation(kind))
                .clicked()
            {
                panel.set_kind(kind);
                ui.close();
            }
        }
    });
    ui.menu_button("Display", |ui| {
        grid_entries(ui, &mut panel.options.grid_detail);
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
        let four = panel.four_views();
        if ui
            .add(
                egui::Button::selectable(four, FOUR_VIEWS_ENTRY)
                    .shortcut_text(FOUR_VIEWS_KEY.name()),
            )
            .on_hover_text(
                "Show the perspective, top, front and right views at once; \
                 click again (or middle click a view) to show only the active one",
            )
            .clicked()
        {
            panel.set_four_views(!four);
            ui.close();
        }
    });
}

/// What the Cameras menu's tooltip says about a kind of view.
fn camera_explanation(kind: ViewKind) -> &'static str {
    match kind {
        ViewKind::Perspective => "An ordinary 3D view: far things look smaller",
        ViewKind::Top => "A flat view looking straight down: no perspective, true sizes",
        ViewKind::Front => "A flat view from the front: no perspective, true sizes",
        ViewKind::Right => "A flat view from the right: no perspective, true sizes",
    }
}

/// A thin accent line around the active view, when four are showing.
pub(super) fn active_border(ui: &Ui, rect: Rect) {
    let accent = theme::accent(ui.ctx()).colors();
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.5, accent.light),
        egui::StrokeKind::Inside,
    );
}

/// The grid detail choices, under a "Grid lines" heading.
pub(crate) fn grid_entries(ui: &mut Ui, detail: &mut super::GridDetail) {
    ui.label(egui::RichText::new(crate::palette::GRID_DETAIL_LABEL).color(theme::TEXT_WEAK));
    for level in super::GridDetail::ALL {
        if ui
            .radio(*detail == level, level.label())
            .on_hover_text(level.explanation())
            .clicked()
        {
            *detail = level;
            ui.close();
        }
    }
}

/// The Frame all and Frame selected menu entries, with their keys.
pub(crate) fn frame_entries(ui: &mut Ui, panel: &mut ViewportPanel) {
    for what in [super::Frame::All, super::Frame::Selected] {
        let keys = ui.ctx().format_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::NONE,
            what.key(),
        ));
        if ui
            .add(egui::Button::new(what.label()).shortcut_text(keys))
            .on_hover_text(what.explanation())
            .clicked()
        {
            panel.frame(what);
            ui.close();
        }
    }
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

/// Quiet labels over a view's image: its name, the camera, an axis
/// marker, and the grid spacing.
pub(super) fn overlays(ui: &Ui, rect: Rect, view: &View, options: &ViewOptions) {
    let painter = ui.painter_at(rect);
    let small = FontId::proportional(12.0);
    let kind = view.kind();
    painter.text(
        rect.left_top() + vec2(10.0, 8.0),
        Align2::LEFT_TOP,
        kind.name(),
        small.clone(),
        theme::TEXT_STRONG,
    );
    if !kind.is_flat() {
        painter.text(
            pos2(rect.center().x, rect.top() + 8.0),
            Align2::CENTER_TOP,
            "Default camera",
            small.clone(),
            theme::TEXT_WEAK,
        );
    }
    let spacing = options.grid_detail.spacing_text();
    painter.text(
        rect.left_bottom() + vec2(10.0, -10.0),
        Align2::LEFT_BOTTOM,
        format!("Grid spacing: {spacing}"),
        small,
        theme::TEXT_WEAK,
    );
    axis_marker(
        &painter,
        rect.right_top() + vec2(-36.0, 36.0),
        view.camera(),
    );
    if shows_pivot_marker(view.drag, options) {
        pivot_marker(ui, &painter, rect, view);
    }
}

/// Whether the pivot marker shows: while orbiting or drag-zooming around
/// the point under the mouse. Around the middle of the view it would just
/// sit there, distracting, so it stays hidden.
pub(super) fn shows_pivot_marker(drag: Option<DragKind>, options: &ViewOptions) -> bool {
    options.navigation.around_cursor && matches!(drag, Some(DragKind::Orbit | DragKind::Dolly))
}

/// A small ring and dot on the point the view turns around or zooms
/// towards, shown while orbiting or zooming with a drag.
fn pivot_marker(ui: &Ui, painter: &egui::Painter, rect: Rect, view: &View) {
    let size = Vec2::new(rect.width(), rect.height());
    let Some(at) = view.camera().project(view.navigator.pivot(), size) else {
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
fn axis_marker(painter: &egui::Painter, centre: egui::Pos2, camera: &tools::Camera) {
    for (name, color, dir) in axis_directions(camera, 18.0) {
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
    fn the_pivot_marker_shows_only_around_the_mouse() {
        let mut options = ViewOptions::default();
        assert!(shows_pivot_marker(Some(DragKind::Orbit), &options));
        assert!(shows_pivot_marker(Some(DragKind::Dolly), &options));
        assert!(!shows_pivot_marker(Some(DragKind::Pan), &options));
        assert!(!shows_pivot_marker(None, &options));
        options.navigation.around_cursor = false;
        assert!(
            !shows_pivot_marker(Some(DragKind::Orbit), &options),
            "hidden when turning around the middle of the view"
        );
    }

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
