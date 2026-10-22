//! The two palettes around the viewport: the command
//! palette along the top (tools and primitives) and the mode palette down
//! the left (what kind of element you work on).
//!
//! Commands that don't exist yet are shown disabled, with a tooltip saying
//! which phase brings them, so the layout is complete from the start.

use crate::commands::CommandId;
use crate::icons::{self, Icon};
use crate::selecting::SelectTool;
use crate::theme::{self, tint};
use crate::transforming::{Axes, Tool};
use crate::viewport::ViewOptions;
use egui::{Color32, Response, RichText, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, vec2};
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

    /// The five modes people see (decision 0126): the Object mode is the
    /// Model mode with Edit pivot on, switched on the top palette and in
    /// the Tools menu.
    pub const SHOWN: [Mode; 5] = [
        Self::Model,
        Self::Points,
        Self::Edges,
        Self::Polygons,
        Self::Uv,
    ];

    /// The mode's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::Object => "Model (Edit pivot)",
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
            Self::Object => {
                "Work on whole objects' pivots: the gizmo moves and turns the pivot, not the shape"
            }
            Self::Points => "Work on points (vertices)",
            Self::Edges => "Work on edges",
            Self::Polygons => "Work on polygons (faces)",
            Self::Uv => "Work on UVs, in the UV Editor and on the model",
        }
    }

    /// The key that switches to this mode (and, pressed again, back to
    /// the Model mode), if it has one. See `MODE_KEYS`.
    pub const fn key(self) -> Option<egui::Key> {
        match self {
            Self::Points => Some(egui::Key::Num1),
            Self::Edges => Some(egui::Key::Num2),
            Self::Polygons => Some(egui::Key::Num3),
            Self::Model | Self::Object | Self::Uv => None,
        }
    }

    /// The tooltip: what the mode is for, and its key.
    fn tooltip(self) -> String {
        match self.key() {
            Some(key) => format!("{} ({})", self.description(), key.name()),
            None => self.description().to_string(),
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
        | Icon::Cone
        | Icon::ShapeList => tint::SHAPE,
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
    /// Switch to the next select tool (as `Q` does), or back to
    /// selecting from a transform tool.
    NextSelectTool,
    /// Use a transform tool (move, rotate or scale).
    Tool(Tool),
    /// Lock or free an axis (0 X, 1 Y, 2 Z).
    Lock(usize),
    /// Switch between world and object axes.
    ToggleAxes,
    /// Turn snapping on or off.
    ToggleSnapping,
    /// Turn the selected objects' live mirror on or off.
    ToggleSymmetry,
    /// Turn Edit pivot on or off (decision 0126).
    ToggleEditPivot,
    /// Open command search.
    Search,
}

/// The Edit pivot switch's name (decision 0126).
pub const EDIT_PIVOT_LABEL: &str = "Edit pivot";
/// The command palette's search button.
pub const SEARCH_LABEL: &str = "Command search";

/// One command on the top palette: one that works, or one that is not
/// built yet (saying when it arrives).
struct Planned {
    icon: Icon,
    label: &'static str,
    /// For a working command, what it does; otherwise when it arrives.
    arrives: &'static str,
    command: Option<PaletteCommand>,
    /// The command list's entry, when it has one: its key is added to
    /// the tooltip.
    listed: Option<CommandId>,
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
        listed: None,
    }
}

/// A palette button for a command from the command list: its icon, name
/// and explanation come from there.
const fn listed(id: CommandId, command: PaletteCommand) -> Planned {
    let info = id.info();
    Planned {
        icon: info.icon,
        label: info.name,
        arrives: info.help,
        command: Some(command),
        listed: Some(id),
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
    /// The select tool in use.
    pub select_tool: SelectTool,
    /// The tool in use.
    pub tool: Tool,
    /// The locked axes.
    pub locked: [bool; 3],
    /// World or object axes.
    pub axes: Axes,
    /// Whether snapping is on.
    pub snapping: bool,
    /// Whether the selected objects have a live mirror.
    pub symmetry: bool,
    /// Whether Edit pivot is on (the Object mode).
    pub edit_pivot: bool,
}

/// What the Symmetry switch does, for its tooltips.
pub(crate) const SYMMETRY_HELP: &str = "Model one half: each selected object gets a mirror of its own, \
     a copy of its shape on the other side of its centre (its pivot) that follows every edit, \
     joined to it at the middle. The mirror moves and turns with the object. The arrow beside it \
     chooses X, Y or Z and moves the mirror.";

/// The accessible name of the arrow that opens the Symmetry menu.
pub(crate) const SYMMETRY_MENU_LABEL: &str = "Symmetry settings";

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
        working(
            Icon::BoxSelection,
            "Select tool",
            "The select tool in use",
            PaletteCommand::NextSelectTool,
        ),
        listed(CommandId::Move, PaletteCommand::Tool(Tool::Move)),
        listed(CommandId::Scale, PaletteCommand::Tool(Tool::Scale)),
        listed(CommandId::Rotate, PaletteCommand::Tool(Tool::Rotate)),
    ],
    &[
        shape(Icon::Cube, "Create cube", PrimitiveKind::Cube),
        shape(Icon::Cylinder, "Create cylinder", PrimitiveKind::Cylinder),
        shape(Icon::Plane, "Create plane", PrimitiveKind::Plane),
        shape(Icon::Sphere, "Create sphere", PrimitiveKind::Sphere),
        working(
            CommandId::NewGroup.info().icon,
            "Create group",
            "Create an empty group, to hold and organise other objects",
            PaletteCommand::CreateGroup,
        ),
    ],
    &[
        working(
            Icon::Snap,
            "Snapping",
            "Snap moves to points, edges, polygons and the grid, turns to 15° and scaling to 10% steps. Hold Ctrl while dragging to flip it for that drag.",
            PaletteCommand::ToggleSnapping,
        ),
        working(
            Icon::Symmetry,
            "Symmetry",
            SYMMETRY_HELP,
            PaletteCommand::ToggleSymmetry,
        ),
    ],
];

/// The icon for a select tool.
pub fn select_tool_icon(tool: SelectTool) -> Icon {
    match tool {
        SelectTool::Box => Icon::BoxSelection,
        SelectTool::Lasso => Icon::LassoSelection,
        SelectTool::Brush => Icon::PaintSelect,
    }
}

/// The command palette along the top of the window, with the Symmetry
/// menu beside its switch (changing `symmetry`). Returns the command
/// clicked this frame, if any.
pub fn command_palette(
    ui: &mut Ui,
    state: &PaletteState,
    symmetry: &mut crate::Symmetry,
    units: crate::units::Units,
) -> Option<PaletteCommand> {
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
                let mut icon = command.icon;
                let mut label = command.label;
                let mut active = false;
                let (available, note) = match action {
                    PaletteCommand::NextSelectTool => {
                        let tool = state.select_tool;
                        icon = select_tool_icon(tool);
                        label = tool.name();
                        active = state.tool == Tool::Select;
                        (
                            true,
                            format!(
                                "{}: {}.\nClick or press Q for {}.",
                                tool.name(),
                                tool.description(),
                                tool.next().name().to_lowercase()
                            ),
                        )
                    }
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
                    PaletteCommand::Tool(tool) => {
                        active = state.tool == tool || state.tool == Tool::All;
                        let keys = command
                            .listed
                            .and_then(|id| id.info().shortcut)
                            .map(|keys| format!(" ({})", ui.ctx().format_shortcut(&keys)))
                            .unwrap_or_default();
                        (true, format!("{}{keys}", command.arrives))
                    }
                    PaletteCommand::ToggleSnapping => {
                        active = state.snapping;
                        (true, command.arrives.to_string())
                    }
                    PaletteCommand::ToggleSymmetry => {
                        active = state.symmetry;
                        (true, command.arrives.to_string())
                    }
                    PaletteCommand::Lock(_)
                    | PaletteCommand::ToggleAxes
                    | PaletteCommand::ToggleEditPivot
                    | PaletteCommand::Search => (true, command.arrives.to_string()),
                };
                let button_state = if active {
                    ButtonState::Active
                } else if available {
                    ButtonState::Normal
                } else {
                    ButtonState::Unavailable
                };
                if icon_button(ui, icon, label, &note, button_state).clicked() {
                    clicked = Some(action);
                }
                if matches!(action, PaletteCommand::ToggleSymmetry)
                    && symmetry_menu(ui, symmetry, state.symmetry, units)
                {
                    clicked = Some(PaletteCommand::ToggleSymmetry);
                }
            }
            if index == 1 {
                // The axis locks sit with the transform tools.
                divider(ui, true);
                for (index, (axis, color)) in [
                    ("X", tint::AXIS_X),
                    ("Y", tint::AXIS_Y),
                    ("Z", tint::AXIS_Z),
                ]
                .into_iter()
                .enumerate()
                {
                    if axis_lock(ui, axis, color, state.locked[index]).clicked() {
                        clicked = Some(PaletteCommand::Lock(index));
                    }
                }
                let (axes_state, axes_note) = match state.axes {
                    Axes::World => (
                        ButtonState::Normal,
                        "Now: the gizmo follows the world's axes.\nClick or press L for the object's own axes.",
                    ),
                    Axes::Object => (
                        ButtonState::Active,
                        "Now: the gizmo follows the object's own axes.\nClick or press L for the world's axes.",
                    ),
                };
                if icon_button(ui, Icon::World, AXES_LABEL, axes_note, axes_state).clicked() {
                    clicked = Some(PaletteCommand::ToggleAxes);
                }
                let (pivot_state, pivot_note) = if state.edit_pivot {
                    (
                        ButtonState::Active,
                        "Edit pivot is on: the gizmo moves and turns the selected objects' \
                         pivots, not their shapes.\nClick to turn it off.",
                    )
                } else {
                    (
                        ButtonState::Normal,
                        "Edit pivot: move and turn the selected objects' pivots (the point \
                         they turn and scale around), not their shapes",
                    )
                };
                if icon_button(ui, Icon::ObjectMode, EDIT_PIVOT_LABEL, pivot_note, pivot_state)
                    .clicked()
                {
                    clicked = Some(PaletteCommand::ToggleEditPivot);
                }
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(
                ui,
                Icon::Search,
                SEARCH_LABEL,
                "Find any command by typing part of its name (Space)",
                ButtonState::Normal,
            )
            .clicked()
            {
                clicked = Some(PaletteCommand::Search);
            }
        });
    });
    clicked
}

/// The small arrow beside the Symmetry switch, and the menu it opens:
/// which axes to mirror across, where the mirror is, and how near points
/// weld at the seam ([decision 0088](../../../docs/decisions/0088-live-mirror.md)).
/// Returns whether On was clicked.
fn symmetry_menu(
    ui: &mut Ui,
    symmetry: &mut crate::Symmetry,
    on: bool,
    units: crate::units::Units,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(14.0, BUTTON_SIZE), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, SYMMETRY_MENU_LABEL));
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let c = rect.center();
    ui.painter().line(
        vec![
            c + vec2(-3.5, -1.5),
            c + vec2(0.0, 2.0),
            c + vec2(3.5, -1.5),
        ],
        Stroke::new(1.5, theme::TEXT_WEAK),
    );
    let response = response
        .on_hover_text("Symmetry settings: the axes to mirror across, and where the mirror sits");
    egui::Popup::from_toggle_button_response(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .gap(4.0)
        .width(260.0)
        .show(|ui| symmetry_settings(ui, symmetry, on, units))
        .is_some_and(|shown| shown.inner)
}

/// The Symmetry menu's contents: On (for the selected objects), the axes,
/// where the mirror is and the weld distance. Returns whether On was
/// clicked.
pub(crate) fn symmetry_settings(
    ui: &mut Ui,
    symmetry: &mut crate::Symmetry,
    on: bool,
    units: crate::units::Units,
) -> bool {
    ui.set_width(260.0);
    let small = |ui: &mut Ui, text: &str| {
        ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
    };
    ui.label(RichText::new("Symmetry").strong().color(theme::TEXT_STRONG));
    let mut checked = on;
    let toggled = ui
        .checkbox(&mut checked, "On")
        .on_hover_text(SYMMETRY_HELP)
        .clicked();
    small(ui, "For the selected objects: each keeps its own mirror.");
    ui.add_space(4.0);
    ui.label("Mirror across");
    ui.horizontal(|ui| {
        for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
            let response = ui.checkbox(&mut symmetry.axes[axis], name);
            response.widget_info(|| {
                WidgetInfo::selected(
                    WidgetType::Checkbox,
                    true,
                    symmetry.axes[axis],
                    format!("Symmetry {name}"),
                )
            });
        }
    });
    small(
        ui,
        "The object's own axes: X mirrors left and right, Y front and back, Z top and \
         bottom, across the faint red planes in the viewport.",
    );
    if !symmetry.axes.contains(&true) {
        small(ui, "Pick an axis: with none, nothing is mirrored.");
    }
    ui.add_space(6.0);
    ui.label("Move the mirror to");
    for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(name);
            crate::attributes::stepper::length(
                ui,
                &mut symmetry.offset[axis],
                (
                    -scene::LiveMirror::MAX_CENTRE,
                    scene::LiveMirror::MAX_CENTRE,
                ),
                units,
                &format!("Symmetry offset {name}"),
            );
        });
    }
    small(
        ui,
        "Where the mirror stands, from the object's centre (its pivot): 0 unless moved.",
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Weld distance");
        crate::attributes::stepper::length(
            ui,
            &mut symmetry.weld_distance,
            (0.0, scene::LiveMirror::MAX_WELD_DISTANCE),
            units,
            "Weld distance",
        );
    });
    small(
        ui,
        "Points at the middle this close to their mirror become one, so the halves join into one \
         surface. 0 keeps them apart.",
    );
    toggled
}

/// Accessible name of the world or object axes switch.
pub const AXES_LABEL: &str = "World or object axes";

/// An axis lock button (X, Y or Z): filled with the accent colour while
/// the axis is locked.
fn axis_lock(ui: &mut Ui, axis: &str, color: Color32, locked: bool) -> Response {
    let label = format!("Lock the {axis} axis");
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(BUTTON_SIZE), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, locked, &label));
    let painter = ui.painter();
    if locked {
        painter.rect_filled(rect, 5.0, theme::accent(ui.ctx()).colors().fill);
    } else if response.hovered() {
        painter.rect_filled(rect, 5.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        axis,
        egui::FontId::proportional(14.0),
        if locked { Color32::WHITE } else { color },
    );
    let note = if locked {
        format!(
            "{axis} is locked: moving, rotating and scaling leave it alone.\nClick or press Shift+{axis} to free it."
        )
    } else {
        format!(
            "Click or press Shift+{axis} to lock it, so moving, rotating and scaling leave {axis} alone."
        )
    };
    response.on_hover_text(note)
}

/// Accessible name of the "select only visible" switch on the mode
/// palette.
pub const VISIBLE_ONLY_LABEL: &str = "Select only visible";

/// Accessible name of the grid detail toggle on the mode palette.
pub const GRID_DETAIL_LABEL: &str = "Grid lines";

/// What was clicked on the mode palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModePick {
    /// A mode.
    Mode(Mode),
    /// Isolate (as Ctrl + H does).
    Isolate,
}

/// Accessible name of the mode palette's isolate button.
pub const ISOLATE_LABEL: &str = "Isolate";

/// The mode palette down the left side, with the viewport's display
/// toggles at the bottom. Returns what was picked this frame, if
/// anything; toggles change `options` directly.
pub fn mode_palette(ui: &mut Ui, current: Mode, options: &mut ViewOptions) -> Option<ModePick> {
    let mut picked = None;
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (index, mode) in Mode::SHOWN.into_iter().enumerate() {
            if index == 1 {
                divider(ui, false);
            }
            // With Edit pivot on, the Model mode shows as the one in use.
            let shown = if current == Mode::Object {
                Mode::Model
            } else {
                current
            };
            let state = if mode == shown {
                ButtonState::Active
            } else {
                ButtonState::Normal
            };
            let label = format!("{} mode", mode.name());
            // Mode icons are always soft white, even Model's cube.
            let tint = tint::ICON;
            if tinted_button(ui, mode.icon(), tint, &label, &mode.tooltip(), state).clicked() {
                picked = Some(ModePick::Mode(mode));
            }
        }
        divider(ui, false);
        if icon_button(
            ui,
            Icon::Solo,
            ISOLATE_LABEL,
            "Isolate: hide every other object, or in the Points, Edges and \
             Polygons modes every other polygon (Ctrl + H). Alt + H shows everything again.",
            ButtonState::Normal,
        )
        .clicked()
        {
            picked = Some(ModePick::Isolate);
        }
        let visible = if options.select_visible_only {
            ButtonState::Active
        } else {
            ButtonState::Normal
        };
        let note = format!(
            "{}\nNow: {}.",
            crate::workbench::VISIBLE_ONLY_HELP,
            if options.select_visible_only {
                "on, only what you can see"
            } else {
                "off, selecting through"
            }
        );
        if icon_button(ui, Icon::VisibleOnly, VISIBLE_ONLY_LABEL, &note, visible).clicked() {
            options.select_visible_only = !options.select_visible_only;
        }
        let detail = options.grid_detail;
        let state = if detail == crate::viewport::GridDetail::Metre {
            ButtonState::Normal
        } else {
            ButtonState::Active
        };
        let note = format!(
            "Now: {}.\nClick for: {}.",
            detail.explanation(),
            detail.next().explanation().to_lowercase()
        );
        if icon_button(ui, Icon::GridDetail, GRID_DETAIL_LABEL, &note, state).clicked() {
            options.grid_detail = detail.next();
        }
    });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_have_distinct_names_and_icons() {
        let names: Vec<_> = Mode::SHOWN.iter().map(|m| m.name()).collect();
        assert_eq!(names, ["Model", "Points", "Edges", "Polygons", "UV"]);
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
        assert_eq!(icon_tint(Icon::ShapeList), tint::SHAPE);
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
