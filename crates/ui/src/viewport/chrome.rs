//! Everything drawn around and over the 3D image: the viewport's own menu
//! line along the top (with buttons you can drag to navigate), and the
//! quiet labels over the image (view name, camera, axis marker, grid
//! spacing).

use super::{FOUR_VIEWS_KEY, VIEW_KEYS, View, ViewOptions, ViewportPanel, cursor_for};
use crate::icons::{self, Icon};
use crate::text::tr;
use crate::theme::{self, backdrop};
use egui::{
    Align, Align2, CursorIcon, FontId, Layout, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2,
};
use glam::Vec2;
use tools::{DragKind, ViewKind};

/// Height of the viewport's menu line, in logical pixels.
pub(super) const HEADER_HEIGHT: f32 = 26.0;

/// Accessible names of the menu line's navigation buttons.
pub(crate) const PAN_BUTTON: &str = crate::text!("Drag to pan the view");
/// See [`PAN_BUTTON`].
pub(crate) const ZOOM_BUTTON: &str = crate::text!("Drag to zoom the view");
/// See [`PAN_BUTTON`].
pub(crate) const ORBIT_BUTTON: &str = crate::text!("Drag to orbit the view");

/// Accessible name of the button that switches between one view and
/// four, when one view is showing.
pub(crate) const FOUR_VIEWS_BUTTON: &str = crate::text!("Show four views");
/// See [`FOUR_VIEWS_BUTTON`]; when four views are showing.
pub(crate) const MAXIMISE_BUTTON: &str = crate::text!("Maximise this view");

/// The name of the Panel menu's entry for the four-view layout.
pub(crate) const FOUR_VIEWS_ENTRY: &str = crate::text!("Four views");

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
    egui::MenuBar::new()
        .config(crate::dialog::menu_config())
        .ui(&mut ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            menus(ui, panel);
        });
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let four = panel.four_views();
        let (label, explanation) = if four {
            (
                tr(MAXIMISE_BUTTON),
                tr("Show only the active view (middle click)"),
            )
        } else {
            (
                tr(FOUR_VIEWS_BUTTON),
                tr("Show the perspective, top, front and right views at once (middle click)"),
            )
        };
        let layout = icon_button(ui, Icon::Maximise, label, Sense::click());
        if layout.clicked() {
            panel.set_four_views(!four);
        }
        layout.on_hover_text(format!("{label}\n{explanation}"));
        for (icon, label, kind) in [
            (Icon::Rotate, tr(ORBIT_BUTTON), DragKind::Orbit),
            (Icon::Dolly, tr(ZOOM_BUTTON), DragKind::Dolly),
            (Icon::Move, tr(PAN_BUTTON), DragKind::Pan),
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
    crate::dialog::menu(ui, tr("View"), |ui| {
        if ui
            .button(tr("Reset view"))
            .on_hover_text(tr("Go back to the opening camera view"))
            .clicked()
        {
            panel.reset_view();
            ui.close();
        }
        ui.separator();
        frame_entries(ui, panel);
    });
    crate::dialog::menu(ui, tr("Cameras"), |ui| {
        let active = panel.active_kind();
        for kind in MENU_KINDS {
            let key = if kind == ViewKind::Orthographic {
                super::PROJECTION_KEY.name()
            } else {
                VIEW_KEYS
                    .iter()
                    .find(|(_, k)| *k == kind)
                    .map(|(key, _)| key.name())
                    .unwrap_or_default()
            };
            if ui
                .add(egui::Button::selectable(kind == active, kind.name()).shortcut_text(key))
                .on_hover_text(camera_explanation(kind))
                .clicked()
            {
                // Between perspective and orthographic the view keeps
                // looking where it looked (as P does); other kinds start
                // at their opening camera.
                let flat = panel.camera().is_flat();
                let switch = match kind {
                    ViewKind::Perspective => flat,
                    ViewKind::Orthographic => !flat,
                    _ => false,
                };
                if switch {
                    panel.toggle_projection();
                } else {
                    panel.set_kind(kind);
                }
                ui.close();
            }
        }
    });
    crate::dialog::menu(ui, tr("Display"), |ui| {
        super::look::choices(ui, &mut panel.options.shading);
        ui.separator();
        ui.label(egui::RichText::new(tr("Surface")).color(crate::theme::TEXT_WEAK));
        for surface in super::Surface::ALL {
            if ui
                .add(egui::Button::selectable(
                    panel.options.surface == surface,
                    surface.label(),
                ))
                .on_hover_text(surface.explanation())
                .clicked()
            {
                panel.options.surface = surface;
                ui.close();
            }
        }
        if panel.options.surface == super::Surface::Painted {
            ui.separator();
            super::look::preview_choices(ui, &mut panel.options.paint_preview);
        }
        ui.separator();
        grid_entries(ui, &mut panel.options.grid_detail);
        ui.separator();
        // What is drawn over the surfaces: cavity, hard edges, normals.
        cavity_entry(ui, &mut panel.options);
        let mut hard = !panel.options.hide_hard_edges;
        if ui
            .checkbox(&mut hard, tr(HARD_EDGES_ENTRY))
            .on_hover_text(tr(
                "Mark hard edges, where the shading breaks into a crisp edge, in a colour of \
                 their own while editing points, edges or polygons",
            ))
            .changed()
        {
            panel.options.hide_hard_edges = !hard;
        }
        ui.checkbox(&mut panel.options.show_seams, tr(SEAMS_ENTRY))
            .on_hover_text(tr(
                "Mark seams, where the surface is cut open for its UVs, in orange while \
                 editing points, edges or polygons (the UV mode always shows them)",
            ));
        ui.checkbox(&mut panel.options.show_normals, tr(NORMALS_ENTRY))
            .on_hover_text(tr(
                "Show which way each polygon of the selected objects faces, as a short line \
                 from its middle. Lines pointing into a shape show flipped polygons.",
            ));
        ui.checkbox(&mut panel.options.show_collision, tr(COLLISION_ENTRY))
            .on_hover_text(tr(COLLISION_HELP));
        ui.checkbox(&mut panel.options.show_triangles, tr(TRIANGLES_ENTRY))
            .on_hover_text(tr(TRIANGLES_HELP));
        ui.separator();
        ui.checkbox(&mut panel.options.flat_from_dice, tr(FLAT_FROM_DICE_ENTRY))
            .on_hover_text(tr(FLAT_FROM_DICE_HELP));
        settings_entry(ui, panel);
    });
    crate::dialog::menu(ui, tr("Options"), |ui| {
        if ui
            .button(format!("{}…", tr(super::PICKER_LABEL)))
            .on_hover_text(tr("Choose which mouse buttons and keys move the view"))
            .clicked()
        {
            panel.open_picker = true;
            ui.close();
        }
        let navigation = &mut panel.options.navigation;
        ui.checkbox(
            &mut navigation.around_cursor,
            tr(super::picker::AROUND_CURSOR_LABEL),
        );
        ui.checkbox(&mut navigation.invert_orbit, tr("Invert orbit direction"));
        ui.checkbox(&mut navigation.show_pivot, tr(SHOW_PIVOT_LABEL))
            .on_hover_text(tr(SHOW_PIVOT_HELP));
        ui.checkbox(
            &mut navigation.alt_everywhere,
            tr("Alt + mouse works in every scheme"),
        );
        settings_entry(ui, panel);
    });
    crate::dialog::menu(ui, tr("Panel"), |ui| {
        let four = panel.four_views();
        if ui
            .add(
                egui::Button::selectable(four, tr(FOUR_VIEWS_ENTRY))
                    .shortcut_text(FOUR_VIEWS_KEY.name()),
            )
            .on_hover_text(tr(
                "Show the perspective, top, front and right views at once; \
                 click again (or middle click a view) to show only the active one",
            ))
            .clicked()
        {
            panel.set_four_views(!four);
            ui.close();
        }
    });
}

/// The entry at the foot of the Display and Options menus that opens the
/// Settings window, where every one of these settings also lives
/// (decision 0126).
pub(crate) const SETTINGS_ENTRY: &str = crate::text!("All settings…");

fn settings_entry(ui: &mut Ui, panel: &mut ViewportPanel) {
    ui.separator();
    let keys = ui
        .ctx()
        .format_shortcut(&crate::settings::SETTINGS_SHORTCUT);
    if ui
        .add(egui::Button::new(tr(SETTINGS_ENTRY)).shortcut_text(keys))
        .on_hover_text(tr(
            "Open the Settings window, where every setting has its home",
        ))
        .clicked()
    {
        panel.open_settings = true;
        ui.close();
    }
}

/// The Display menu's switch for marking hard edges.
pub(crate) const HARD_EDGES_ENTRY: &str = crate::text!("Show hard edges");
/// The Mesh menu's switch for the selected objects' smoothing by angle
/// (decision 0126: once Automatic hard edges, for every shape).
pub(crate) const AUTO_HARD_EDGES_ENTRY: &str = crate::text!("Smooth by angle");
/// The Display menu's and the look row's switch for cavity (decision 0065).
pub(crate) const CAVITY_ENTRY: &str = crate::text!("Cavity");
/// What cavity does, for its tooltips.
pub(crate) const CAVITY_EXPLANATION: &str = crate::text!(
    "Brighten ridges and darken creases on the \
     surfaces, so edges and the shape's form read clearly at a glance. Only how it looks: \
     nothing about the shape changes."
);

/// The name of the view dice's orthographic switch, in the Display menu
/// (decision 0084).
pub(crate) const FLAT_FROM_DICE_ENTRY: &str =
    crate::text!("Switch to orthographic when clicking the dice");
/// What it does.
const FLAT_FROM_DICE_HELP: &str = crate::text!(
    "Clicking the view dice in the bottom-left corner (or snapping to it with Ctrl while \
     orbiting) also makes the view flat, with no perspective, so lines line up. Orbit away \
     and it goes back to perspective."
);

/// The Cavity switch, in the Display menu.
pub(crate) fn cavity_entry(ui: &mut Ui, options: &mut ViewOptions) {
    let mut cavity = !options.hide_cavity;
    if ui
        .checkbox(&mut cavity, tr(CAVITY_ENTRY))
        .on_hover_text(tr(CAVITY_EXPLANATION))
        .changed()
    {
        options.hide_cavity = !cavity;
    }
}

/// The Display menu's switch for showing normals.
pub(crate) const NORMALS_ENTRY: &str = crate::text!("Show normals");
/// The Display menu's switch for outlining every object's collision
/// shapes (decision 0151).
pub(crate) const COLLISION_ENTRY: &str = crate::text!("Show collision");
/// What it does.
pub(crate) const COLLISION_HELP: &str = crate::text!(
    "Outline every object's collision shapes (Wrapped, Wrapped in parts, A box), as a game \
     engine will make them. Off, only the selected objects' are outlined."
);
/// The Display menu's entry for the triangles the export makes
/// (decision 0161).
pub(crate) const TRIANGLES_ENTRY: &str = crate::text!("Show triangles");
/// What it does.
pub(crate) const TRIANGLES_HELP: &str = crate::text!(
    "Draw faint lines where every shown shape's polygons are cut into triangles, exactly as \
     the export cuts them for a game engine: see a long thin triangle or a bent polygon's \
     fold before the game does."
);
/// The Display menu's entry for seams (decision 0096).
pub(crate) const SEAMS_ENTRY: &str = crate::text!("Show seams");

/// The kinds of view the Cameras menu offers, in order.
const MENU_KINDS: [ViewKind; 5] = [
    ViewKind::Perspective,
    ViewKind::Orthographic,
    ViewKind::Top,
    ViewKind::Front,
    ViewKind::Right,
];

/// What the Cameras menu's tooltip says about a kind of view.
fn camera_explanation(kind: ViewKind) -> &'static str {
    match kind {
        ViewKind::Perspective => tr("An ordinary 3D view: far things look smaller"),
        ViewKind::Top => tr("A flat view looking straight down: no perspective, true sizes"),
        ViewKind::Front => tr("A flat view from the front: no perspective, true sizes"),
        ViewKind::Right => tr("A flat view from the right: no perspective, true sizes"),
        ViewKind::Orthographic => {
            tr("No perspective, from any direction: orbit it freely; parallel lines stay parallel")
        }
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
    ui.label(egui::RichText::new(tr(crate::palette::GRID_DETAIL_LABEL)).color(theme::TEXT_WEAK));
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
            egui::RichText::new(crate::trf!("The 3D view can't be shown: {}.", reason))
                .color(theme::TEXT),
        )
        .selectable(false),
    );
}

/// A quiet note at the bottom of a view while a heavy shape is made ready
/// to draw in the background.
pub(super) fn preparing(ui: &mut Ui, rect: Rect) {
    ui.put(
        Rect::from_center_size(
            rect.center_bottom() - vec2(0.0, 34.0),
            vec2(rect.width(), 20.0),
        ),
        egui::Label::new(
            egui::RichText::new(tr("Preparing a heavy shape for drawing…")).color(theme::TEXT_WEAK),
        )
        .selectable(false),
    );
}

/// Quiet labels over a view's image: its name, the camera, and the grid
/// spacing in the bottom-left corner, above the view dice (which the
/// viewport draws and listens to itself, decision 0084).
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
            tr("Default camera"),
            small.clone(),
            theme::TEXT_WEAK,
        );
    }
    let spacing = options.grid_detail.spacing_text();
    painter.text(
        rect.left_bottom() + vec2(10.0, -96.0),
        Align2::LEFT_BOTTOM,
        crate::trf!("Grid spacing: {}", spacing),
        small,
        theme::TEXT_WEAK,
    );
    if shows_pivot_marker(view.drag, options) {
        pivot_marker(ui, &painter, rect, view);
    }
}

/// The name of the switch for the pivot marker, in the Options menu and
/// Settings.
pub(crate) const SHOW_PIVOT_LABEL: &str = crate::text!("Show the turning point while orbiting");
/// What it does.
pub(crate) const SHOW_PIVOT_HELP: &str = crate::text!(
    "While you orbit, a small ring marks the point the view turns around \
     (and, when zooming with a drag, the point it moves towards)"
);

/// Whether the pivot marker shows (decision 0080): with its switch on,
/// while orbiting, and while drag-zooming towards the point under the
/// mouse.
pub(super) fn shows_pivot_marker(drag: Option<DragKind>, options: &ViewOptions) -> bool {
    let navigation = options.navigation;
    navigation.show_pivot
        && match drag {
            Some(DragKind::Orbit) => true,
            Some(DragKind::Dolly) => navigation.around_cursor,
            _ => false,
        }
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
    let shadow = Stroke::new(4.5, egui::Color32::from_black_alpha(130));
    painter.circle_stroke(centre, 9.0, shadow);
    painter.circle_stroke(centre, 9.0, Stroke::new(2.0, accent.light));
    painter.circle_filled(centre, 3.5, egui::Color32::from_black_alpha(130));
    painter.circle_filled(centre, 2.5, accent.light);
}

/// A banner across the top of the views (decision 0165: while a level
/// made by hand is edited), with a button to end what it is about.
/// Returns whether the button was clicked.
pub(super) fn banner(ui: &mut Ui, body: Rect, text: &str) -> bool {
    let width = (body.width() - 40.0).clamp(120.0, 620.0);
    let area = Rect::from_min_size(
        egui::pos2(body.center().x - width / 2.0, body.top() + 30.0),
        vec2(width, 60.0),
    );
    let accent = theme::accent(ui.ctx()).colors();
    let mut clicked = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        egui::Frame::new()
            .fill(theme::PANEL.gamma_multiply(0.95))
            .stroke(egui::Stroke::new(1.5, accent.light))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(text).color(theme::TEXT_STRONG));
                    clicked = ui
                        .button(tr(crate::workbench::STOP_EDITING_LEVEL))
                        .on_hover_text(tr(
                            "Select the object again: the viewport shows the model, and its \
                             level goes on as you left it",
                        ))
                        .clicked();
                });
            });
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pivot_marker_shows_while_orbiting_unless_switched_off() {
        let mut options = ViewOptions::default();
        assert!(shows_pivot_marker(Some(DragKind::Orbit), &options));
        assert!(shows_pivot_marker(Some(DragKind::Dolly), &options));
        assert!(!shows_pivot_marker(Some(DragKind::Pan), &options));
        assert!(!shows_pivot_marker(None, &options));
        // Around the middle of the view: still while orbiting, not when
        // zooming (it would just sit in the middle).
        options.navigation.around_cursor = false;
        assert!(shows_pivot_marker(Some(DragKind::Orbit), &options));
        assert!(!shows_pivot_marker(Some(DragKind::Dolly), &options));
        // Switched off: never.
        options.navigation.show_pivot = false;
        assert!(!shows_pivot_marker(Some(DragKind::Orbit), &options));
    }
}
