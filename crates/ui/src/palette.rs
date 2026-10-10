//! The two palettes around the viewport: the command
//! palette along the top (tools and primitives) and the mode palette down
//! the left (what kind of element you work on).
//!
//! Commands that don't exist yet are shown disabled, with a tooltip saying
//! which phase brings them, so the layout is complete from the start.

use crate::icons::{self, Icon};
use crate::theme::{self, tint};
use crate::viewport::ViewOptions;
use egui::{Color32, Response, Sense, Ui, Vec2, WidgetInfo, WidgetType};
use scene::PrimitiveKind;

/// Size of a palette button, in logical pixels.
pub const BUTTON_SIZE: f32 = 32.0;
/// Size of the icon inside a palette button.
const ICON_SIZE: f32 = 18.0;

/// What kind of element the tools work on.
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
    tinted_button(ui, icon, icon_tint(icon), label, note, state)
}

/// Like [`icon_button`], in a given colour instead of the icon's own.
pub fn tinted_button(
    ui: &mut Ui,
    icon: Icon,
    tint: Color32,
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
        let accent = theme::accent(ui.ctx()).colors();
        let background = match state {
            ButtonState::Active => Some(accent.fill),
            _ if enabled && response.hovered() => Some(visuals.widgets.hovered.weak_bg_fill),
            _ => None,
        };
        if let Some(fill) = background {
            painter.rect_filled(rect, 5.0, fill);
        }
        let color = match state {
            ButtonState::Active => Color32::WHITE,
            ButtonState::Normal if response.hovered() => crate::theme::TEXT_STRONG,
            ButtonState::Normal => tint,
            ButtonState::Unavailable => dimmed(tint),
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

/// The colour an icon is drawn in, by the group of commands it belongs to
/// (see `theme::tint`).
pub fn icon_tint(icon: Icon) -> Color32 {
    match icon {
        Icon::Cube
        | Icon::Cylinder
        | Icon::Plane
        | Icon::Sphere
        | Icon::Disc
        | Icon::Tube
        | Icon::Cone => tint::SHAPE,
        _ => tint::ICON,
    }
}

/// A colour as drawn for a command that is not available yet.
fn dimmed(color: Color32) -> Color32 {
    theme::PALETTE.lerp_to_gamma(color, theme::UNAVAILABLE_STRENGTH)
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

/// A command the top palette can carry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    /// Take back the last step.
    Undo,
    /// Do the last undone step again.
    Redo,
    /// Create an empty group.
    CreateGroup,
    /// Create a primitive with standard settings.
    CreateShape(PrimitiveKind),
}

/// One command on the top palette: one that works, or one that is not
/// built yet (saying when it arrives).
struct Planned {
    icon: Icon,
    label: &'static str,
    /// For a working command, what it does; otherwise when it arrives.
    arrives: &'static str,
    command: Option<PaletteCommand>,
}

const fn planned(icon: Icon, label: &'static str, arrives: &'static str) -> Planned {
    Planned {
        icon,
        label,
        arrives,
        command: None,
    }
}

const fn working(
    icon: Icon,
    label: &'static str,
    note: &'static str,
    command: PaletteCommand,
) -> Planned {
    Planned {
        icon,
        label,
        arrives: note,
        command: Some(command),
    }
}

/// A palette button that creates a primitive, explained by its kind.
const fn shape(icon: Icon, label: &'static str, kind: PrimitiveKind) -> Planned {
    working(
        icon,
        label,
        kind.description(),
        PaletteCommand::CreateShape(kind),
    )
}

/// What the palette needs to know to draw its working commands.
#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    /// What Undo would take back, if anything.
    pub undo: Option<String>,
    /// What Redo would do again, if anything.
    pub redo: Option<String>,
}

/// The groups of the command palette, left to right.
const COMMAND_GROUPS: [&[Planned]; 4] = [
    &[
        working(
            Icon::Undo,
            "Undo",
            "Take back the last change",
            PaletteCommand::Undo,
        ),
        working(
            Icon::Redo,
            "Redo",
            "Do the last undone change again",
            PaletteCommand::Redo,
        ),
    ],
    &[
        planned(Icon::LiveSelection, "Live selection", "Arrives in Phase 3"),
        planned(Icon::Move, "Move", "Arrives in Phase 3"),
        planned(Icon::Scale, "Scale", "Arrives in Phase 3"),
        planned(Icon::Rotate, "Rotate", "Arrives in Phase 3"),
    ],
    &[
        shape(Icon::Cube, "Create cube", PrimitiveKind::Cube),
        shape(Icon::Cylinder, "Create cylinder", PrimitiveKind::Cylinder),
        shape(Icon::Plane, "Create plane", PrimitiveKind::Plane),
        shape(Icon::Sphere, "Create sphere", PrimitiveKind::Sphere),
        working(
            Icon::Null,
            "Create group",
            "Create an empty group, to hold and organise other objects",
            PaletteCommand::CreateGroup,
        ),
    ],
    &[
        planned(Icon::Snap, "Snapping", "Arrives in Phase 3"),
        planned(Icon::Symmetry, "Symmetry", "Arrives in Phase 4"),
    ],
];

/// The command palette along the top of the window. Returns the command
/// clicked this frame, if any.
pub fn command_palette(ui: &mut Ui, state: &PaletteState) -> Option<PaletteCommand> {
    let mut clicked = None;
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (index, group) in COMMAND_GROUPS.iter().enumerate() {
            if index > 0 {
                divider(ui, true);
            }
            for command in *group {
                let Some(action) = command.command else {
                    icon_button(
                        ui,
                        command.icon,
                        command.label,
                        command.arrives,
                        ButtonState::Unavailable,
                    );
                    continue;
                };
                let (available, note) = match action {
                    PaletteCommand::Undo => match &state.undo {
                        Some(what) => (true, format!("Undo: {what}")),
                        None => (false, "Nothing to undo".to_string()),
                    },
                    PaletteCommand::Redo => match &state.redo {
                        Some(what) => (true, format!("Redo: {what}")),
                        None => (false, "Nothing to redo".to_string()),
                    },
                    PaletteCommand::CreateGroup | PaletteCommand::CreateShape(_) => {
                        (true, command.arrives.to_string())
                    }
                };
                let button_state = if available {
                    ButtonState::Normal
                } else {
                    ButtonState::Unavailable
                };
                if icon_button(ui, command.icon, command.label, &note, button_state).clicked() {
                    clicked = Some(action);
                }
            }
            if index == 1 {
                // The axis locks sit with the transform tools.
                divider(ui, true);
                for (axis, color) in [
                    ("X", tint::AXIS_X),
                    ("Y", tint::AXIS_Y),
                    ("Z", tint::AXIS_Z),
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
    clicked
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
        dimmed(color),
    );
    response.on_hover_text(format!("{label}\nArrives in Phase 3"));
}

/// Accessible name of the grid detail toggle on the mode palette.
pub const GRID_DETAIL_LABEL: &str = "Fine grid lines";

/// The mode palette down the left side, with the viewport's display
/// toggles at the bottom. Returns the mode picked this frame, if any;
/// toggles change `options` directly.
pub fn mode_palette(ui: &mut Ui, current: Mode, options: &mut ViewOptions) -> Option<Mode> {
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
            // Mode icons are always soft white, even Model's cube.
            let tint = tint::ICON;
            if tinted_button(ui, mode.icon(), tint, &label, mode.description(), state).clicked() {
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
        let (state, note) = if options.fine_grid {
            (
                ButtonState::Active,
                "On: the grid shows 10 cm lines as well as 1 m lines. Click for a calmer grid.",
            )
        } else {
            (
                ButtonState::Normal,
                "Off: the grid shows 1 m lines only. Click to add the 10 cm lines.",
            )
        };
        if icon_button(ui, Icon::GridDetail, GRID_DETAIL_LABEL, note, state).clicked() {
            options.fine_grid = !options.fine_grid;
        }
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
    fn shapes_are_blue_and_everything_else_soft_white() {
        for icon in crate::icons::ALL {
            assert!(tint::ALL.contains(&icon_tint(icon)), "{icon:?}");
        }
        // As in the mockups: shapes are blue, everything else soft white.
        assert_eq!(icon_tint(Icon::Cube), tint::SHAPE);
        assert_eq!(icon_tint(Icon::Sphere), tint::SHAPE);
        assert_eq!(icon_tint(Icon::Move), tint::ICON);
        assert_eq!(icon_tint(Icon::Null), tint::ICON);
    }

    #[test]
    fn model_mode_is_the_default() {
        assert_eq!(Mode::default(), Mode::Model);
    }

    #[test]
    fn every_planned_command_says_when_it_arrives() {
        let planned = COMMAND_GROUPS
            .iter()
            .flat_map(|g| g.iter())
            .filter(|c| c.command.is_none());
        for command in planned {
            assert!(command.arrives.starts_with("Arrives"), "{}", command.label);
        }
    }
}
