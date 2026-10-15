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
    ui.menu_button("Display", |ui| {
        super::look::choices(ui, &mut panel.options.shading);
        ui.separator();
        grid_entries(ui, &mut panel.options.grid_detail);
        ui.separator();
        // What is drawn over the surfaces: cavity, hard edges, normals.
        cavity_entry(ui, &mut panel.options);
        let mut hard = !panel.options.hide_hard_edges;
        if ui
            .checkbox(&mut hard, HARD_EDGES_ENTRY)
            .on_hover_text(
                "Mark hard edges, where the shading breaks into a crisp edge, in a colour of \
                 their own while editing points, edges or polygons",
            )
            .changed()
        {
            panel.options.hide_hard_edges = !hard;
        }
        ui.checkbox(&mut panel.options.show_normals, NORMALS_ENTRY)
            .on_hover_text(
                "Show which way each polygon of the selected objects faces, as a short line \
                 from its middle. Lines pointing into a shape show flipped polygons.",
            );
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
        ui.checkbox(&mut navigation.show_pivot, SHOW_PIVOT_LABEL)
            .on_hover_text(SHOW_PIVOT_HELP);
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

/// The Display menu's switch for marking hard edges.
pub(crate) const HARD_EDGES_ENTRY: &str = "Hard edges";
/// The Mesh menu's switch for shading by angle (decisions 0064 and 0065:
/// moved there from the Display menu).
pub(crate) const AUTO_HARD_EDGES_ENTRY: &str = "Automatic hard edges";
/// The Display menu's and the look row's switch for cavity (decision 0065).
pub(crate) const CAVITY_ENTRY: &str = "Cavity";
/// What cavity does, for its tooltips.
pub(crate) const CAVITY_EXPLANATION: &str = "Brighten ridges and darken creases on the \
     surfaces, so edges and the shape's form read clearly at a glance. Only how it looks: \
     nothing about the shape changes.";

/// The Cavity switch, in the Display menu.
pub(crate) fn cavity_entry(ui: &mut Ui, options: &mut ViewOptions) {
    let mut cavity = !options.hide_cavity;
    if ui
        .checkbox(&mut cavity, CAVITY_ENTRY)
        .on_hover_text(CAVITY_EXPLANATION)
        .changed()
    {
        options.hide_cavity = !cavity;
    }
}

/// The Automatic hard edges switch, in the main Mesh menu: shade every
/// shape by angle after every change (decision 0064).
pub(crate) fn auto_hard_edges_entry(ui: &mut Ui, options: &mut ViewOptions) {
    let mut auto = !options.hard_edges_by_hand;
    if ui
        .checkbox(&mut auto, AUTO_HARD_EDGES_ENTRY)
        .on_hover_text(format!(
            "Shade every shape by angle: edges where polygons meet more sharply than {}° are \
             crisp, the rest smooth, after every change. A cylinder's sides stay smooth and \
             its caps crisp, however you extrude it. Switch it off to shade by the hard and \
             soft edges you set yourself.",
            super::AUTO_HARD_ANGLE
        ))
        .changed()
    {
        options.hard_edges_by_hand = !auto;
    }
}
/// The Display menu's switch for showing normals.
pub(crate) const NORMALS_ENTRY: &str = "Normals";

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
        ViewKind::Perspective => "An ordinary 3D view: far things look smaller",
        ViewKind::Top => "A flat view looking straight down: no perspective, true sizes",
        ViewKind::Front => "A flat view from the front: no perspective, true sizes",
        ViewKind::Right => "A flat view from the right: no perspective, true sizes",
        ViewKind::Orthographic => {
            "No perspective, from any direction: orbit it freely; parallel lines stay parallel"
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

/// A quiet note at the bottom of a view while a heavy shape is made ready
/// to draw in the background.
pub(super) fn preparing(ui: &mut Ui, rect: Rect) {
    ui.put(
        Rect::from_center_size(
            rect.center_bottom() - vec2(0.0, 34.0),
            vec2(rect.width(), 20.0),
        ),
        egui::Label::new(
            egui::RichText::new("Preparing a heavy shape for drawing…").color(theme::TEXT_WEAK),
        )
        .selectable(false),
    );
}

/// Where the axis marker's middle sits, from a view's bottom-left corner:
/// in the corner, beside the look row ([decision 0063](../../../../docs/decisions/0063-wireframe.md)).
pub(super) const AXIS_MARKER_AT: egui::Vec2 = vec2(40.0, -40.0);

/// Quiet labels over a view's image: its name, the camera, an axis
/// marker in the bottom-left corner, and the grid spacing above it.
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
        rect.left_bottom() + vec2(10.0, -82.0),
        Align2::LEFT_BOTTOM,
        format!("Grid spacing: {spacing}"),
        small,
        theme::TEXT_WEAK,
    );
    axis_marker(&painter, rect.left_bottom() + AXIS_MARKER_AT, view.camera());
    if shows_pivot_marker(view.drag, options) {
        pivot_marker(ui, &painter, rect, view);
    }
}

/// The name of the switch for the pivot marker, in the Options menu and
/// Settings.
pub(crate) const SHOW_PIVOT_LABEL: &str = "Show the turning point while orbiting";
/// What it does.
pub(crate) const SHOW_PIVOT_HELP: &str = "While you orbit, a small ring marks the point the view turns around \
     (and, when zooming with a drag, the point it moves towards)";

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

/// The marker's axis letters: each axis's name, its direction in the world,
/// and where its letter sits, for a marker around `centre`. Farthest first,
/// as drawn, so nearer letters are on top.
pub(super) fn axis_letters(
    camera: &tools::Camera,
    centre: egui::Pos2,
) -> Vec<(&'static str, Vec3, egui::Pos2)> {
    axis_directions(camera, AXIS_LENGTH)
        .into_iter()
        .map(|(name, _, dir)| {
            let axis = match name {
                "X" => Vec3::X,
                "Y" => Vec3::Y,
                _ => Vec3::Z,
            };
            (name, axis, centre + dir + letter_offset(dir))
        })
        .collect()
}

/// How far an axis letter sits past its line's end: a little further
/// along it, or on the centre for an axis pointing straight at or away
/// from the view.
fn letter_offset(dir: egui::Vec2) -> egui::Vec2 {
    if dir.length() > 1.0 {
        dir.normalized() * LETTER_RADIUS
    } else {
        egui::Vec2::ZERO
    }
}

/// The radius of the round button each axis letter sits in, in points.
const LETTER_RADIUS: f32 = 8.0;

/// How long the marker's axis lines are, in points.
const AXIS_LENGTH: f32 = 22.0;

/// Accessible name of an axis letter on the marker, which turns the view
/// to look along it when clicked (decision 0066).
pub(crate) fn look_along_label(axis: &str) -> String {
    format!("Look along {axis}")
}

/// The marker's letters as buttons, in a perspective view: returns the
/// axis clicked, if any. A hovered letter gets a ring.
pub(super) fn axis_clicks(
    ui: &Ui,
    rect: Rect,
    slot: usize,
    camera: &tools::Camera,
) -> Option<Vec3> {
    let mut clicked = None;
    for (name, axis, at) in axis_letters(camera, rect.left_bottom() + AXIS_MARKER_AT) {
        let hit = Rect::from_center_size(at, vec2(LETTER_RADIUS, LETTER_RADIUS) * 2.0);
        let response = ui.interact(
            hit,
            ui.id().with(("look_along", slot, name)),
            Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, look_along_label(name))
        });
        if response.hovered() {
            ui.painter()
                .circle_stroke(at, LETTER_RADIUS, Stroke::new(1.2, theme::TEXT_STRONG));
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if response
            .on_hover_text(format!(
                "Look at the scene from the {name} side (click again: from the other side)"
            ))
            .clicked()
        {
            clicked = Some(axis);
        }
    }
    clicked
}

/// A small marker showing which way X, Y and Z point from this view.
fn axis_marker(painter: &egui::Painter, centre: egui::Pos2, camera: &tools::Camera) {
    for (name, color, dir) in axis_directions(camera, AXIS_LENGTH) {
        painter.line_segment([centre, centre + dir], Stroke::new(1.6, color));
        // An axis pointing straight at or away from you is just a dot;
        // its letter then sits on the centre.
        let label_at = centre + dir + letter_offset(dir);
        // Each letter in a soft round button of its colour: click it to
        // look along the axis (perspective views, decision 0066).
        painter.circle_filled(label_at, LETTER_RADIUS, color.gamma_multiply(0.22));
        painter.circle_stroke(
            label_at,
            LETTER_RADIUS,
            Stroke::new(1.0, color.gamma_multiply(0.6)),
        );
        painter.text(
            label_at,
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(11.0),
            color,
        );
    }
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
