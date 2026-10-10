//! The two palettes around the viewport, as in Cinema 4D: the command
//! palette along the top (tools and primitives) and the mode palette down
//! the left (what kind of element you work on).
//!
//! Commands that don't exist yet are shown disabled, with a tooltip saying
//! which phase brings them, so the layout is complete from the start.

use crate::icons::{self, Icon};
use egui::{Color32, Response, Sense, Ui, Vec2, WidgetInfo, WidgetType};

/// Size of a palette button, in logical pixels.
pub const BUTTON_SIZE: f32 = 32.0;
/// Size of the icon inside a palette button.
const ICON_SIZE: f32 = 18.0;

/// What kind of element the tools work on (Cinema 4D's modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Whole objects, with their children.
    #[default]
    Model,
    /// Whole objects, including moving their axis (pivot).
    Object,
    /// Points (vertices).
    Points,
    /// Edges.
    Edges,
    /// Polygons (faces).
    Polygons,
    /// UV coordinates, in the UV Editor and on the model.
    Uv,
}

impl Mode {
    /// Every mode, in palette order.
    pub const ALL: [Mode; 6] = [
        Self::Model,
        Self::Object,
        Self::Points,
        Self::Edges,
        Self::Polygons,
        Self::Uv,
    ];

    /// The mode's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::Object => "Object",
            Self::Points => "Points",
            Self::Edges => "Edges",
            Self::Polygons => "Polygons",
            Self::Uv => "UV",
        }
    }

    /// What the mode is for, in plain words.
    pub fn description(self) -> &'static str {
        match self {
            Self::Model => "Work on whole objects",
            Self::Object => "Work on whole objects, including their axis (pivot)",
            Self::Points => "Work on points (vertices)",
            Self::Edges => "Work on edges",
            Self::Polygons => "Work on polygons (faces)",
            Self::Uv => "Work on UVs, in the UV Editor and on the model",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Model => Icon::Cube,
            Self::Object => Icon::ObjectMode,
            Self::Points => Icon::Points,
            Self::Edges => Icon::Edges,
            Self::Polygons => Icon::Polygons,
            Self::Uv => Icon::Uv,
        }
    }
}

/// How a palette button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Not available yet: drawn faint, cannot be clicked.
    Unavailable,
    /// Available, not active.
    Normal,
    /// The active tool or mode.
    Active,
}

/// The accent colour for the active mode, as in Cinema 4D.
pub const ACCENT: Color32 = Color32::from_rgb(242, 163, 58);

/// A square palette button with an icon. `label` names it for tooltips
/// and screen readers; `note` is added to the tooltip (for example when
/// the command arrives).
pub fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    note: &str,
    state: ButtonState,
) -> Response {
    let enabled = state != ButtonState::Unavailable;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(BUTTON_SIZE), sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        let painter = ui.painter();
        let background = match state {
            ButtonState::Active => Some(ACCENT.gamma_multiply(0.22)),
            _ if enabled && response.hovered() => Some(visuals.widgets.hovered.weak_bg_fill),
            _ => None,
        };
        if let Some(fill) = background {
            painter.rect_filled(rect, 5.0, fill);
        }
        let color = match state {
            ButtonState::Active => ACCENT,
            ButtonState::Normal => visuals.widgets.inactive.fg_stroke.color,
            ButtonState::Unavailable => visuals.weak_text_color().gamma_multiply(0.7),
        };
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE));
        icons::paint(painter, icon_rect, icon, color);
    }
    let tip = if note.is_empty() {
        label.to_string()
    } else {
        format!("{label}\n{note}")
    };
    response.on_hover_text(tip)
}

/// A thin divider between groups of palette buttons.
fn divider(ui: &mut Ui, vertical_bar: bool) {
    let length = BUTTON_SIZE * 0.7;
    let size = if vertical_bar {
        Vec2::new(9.0, length)
    } else {
        Vec2::new(length, 9.0)
    };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    if vertical_bar {
        ui.painter().vline(rect.center().x, rect.y_range(), stroke);
    } else {
        ui.painter().hline(rect.x_range(), rect.center().y, stroke);
    }
}

/// One command on the top palette that is not built yet.
struct Planned {
    icon: Icon,
    label: &'static str,
    arrives: &'static str,
}

const fn planned(icon: Icon, label: &'static str, arrives: &'static str) -> Planned {
    Planned {
        icon,
        label,
        arrives,
    }
}

/// The groups of the command palette, left to right.
const COMMAND_GROUPS: [&[Planned]; 4] = [
    &[
        planned(Icon::Undo, "Undo", "Arrives in Phase 2"),
        planned(Icon::Redo, "Redo", "Arrives in Phase 2"),
    ],
    &[
        planned(Icon::LiveSelection, "Live selection", "Arrives in Phase 3"),
        planned(Icon::Move, "Move", "Arrives in Phase 3"),
        planned(Icon::Scale, "Scale", "Arrives in Phase 3"),
        planned(Icon::Rotate, "Rotate", "Arrives in Phase 3"),
    ],
    &[
        planned(Icon::Cube, "Cube", "Arrives in Phase 2"),
        planned(Icon::Cylinder, "Cylinder", "Arrives in Phase 2"),
        planned(Icon::Plane, "Plane", "Arrives in Phase 2"),
        planned(Icon::Sphere, "Sphere", "Arrives in Phase 2"),
        planned(Icon::Null, "Null (empty group)", "Arrives in Phase 2"),
    ],
    &[
        planned(Icon::Snap, "Snapping", "Arrives in Phase 3"),
        planned(Icon::Symmetry, "Symmetry", "Arrives in Phase 4"),
    ],
];

/// The command palette along the top of the window.
pub fn command_palette(ui: &mut Ui) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (index, group) in COMMAND_GROUPS.iter().enumerate() {
            if index > 0 {
                divider(ui, true);
            }
            for command in *group {
                icon_button(
                    ui,
                    command.icon,
                    command.label,
                    command.arrives,
                    ButtonState::Unavailable,
                );
            }
            if index == 1 {
                // The axis locks sit with the transform tools, as in Cinema 4D.
                divider(ui, true);
                for (axis, color) in [
                    ("X", Color32::from_rgb(239, 107, 99)),
                    ("Y", Color32::from_rgb(127, 208, 90)),
                    ("Z", Color32::from_rgb(106, 162, 255)),
                ] {
                    axis_lock(ui, axis, color);
                }
                icon_button(
                    ui,
                    Icon::World,
                    "World or object coordinates",
                    "Arrives in Phase 3",
                    ButtonState::Unavailable,
                );
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            icon_button(
                ui,
                Icon::Search,
                "Find a command",
                "Arrives later",
                ButtonState::Unavailable,
            );
        });
    });
}

/// An axis lock button (X, Y or Z), shown but not available yet.
fn axis_lock(ui: &mut Ui, axis: &str, color: Color32) {
    let label = format!("Lock the {axis} axis");
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(BUTTON_SIZE), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, false, &label));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        axis,
        egui::FontId::proportional(14.0),
        color.gamma_multiply(0.5),
    );
    response.on_hover_text(format!("{label}\nArrives in Phase 3"));
}

/// The mode palette down the left side. Returns the mode picked this
/// frame, if any.
pub fn mode_palette(ui: &mut Ui, current: Mode) -> Option<Mode> {
    let mut picked = None;
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (index, mode) in Mode::ALL.into_iter().enumerate() {
            if index == 2 {
                divider(ui, false);
            }
            let state = if mode == current {
                ButtonState::Active
            } else {
                ButtonState::Normal
            };
            let label = format!("{} mode", mode.name());
            if icon_button(ui, mode.icon(), &label, mode.description(), state).clicked() {
                picked = Some(mode);
            }
        }
        divider(ui, false);
        icon_button(
            ui,
            Icon::Workplane,
            "Workplane",
            "Arrives later",
            ButtonState::Unavailable,
        );
        icon_button(
            ui,
            Icon::Solo,
            "Show only the selection",
            "Arrives later",
            ButtonState::Unavailable,
        );
    });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_have_distinct_names_and_icons() {
        let names: Vec<_> = Mode::ALL.iter().map(|m| m.name()).collect();
        assert_eq!(
            names,
            ["Model", "Object", "Points", "Edges", "Polygons", "UV"]
        );
        for (i, a) in Mode::ALL.iter().enumerate() {
            for b in &Mode::ALL[i + 1..] {
                assert_ne!(a.icon(), b.icon());
            }
        }
    }

    #[test]
    fn model_mode_is_the_default() {
        assert_eq!(Mode::default(), Mode::Model);
    }

    #[test]
    fn every_planned_command_says_when_it_arrives() {
        for command in COMMAND_GROUPS.iter().flat_map(|g| g.iter()) {
            assert!(command.arrives.starts_with("Arrives"), "{}", command.label);
        }
    }
}
