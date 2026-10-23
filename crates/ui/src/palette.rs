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
    worded_button(ui, icon, tint, label, note, state, None)
}

/// The size of the words beside a palette icon (decision 0126).
const WORD_SIZE: f32 = 13.0;

/// An icon button with `word` written beside the icon, when given (the
/// top palette's labels, where there's room; decision 0126).
fn worded_button(
    ui: &mut Ui,
    icon: Icon,
    tint: Color32,
    label: &str,
    note: &str,
    state: ButtonState,
    word: Option<&str>,
) -> Response {
    let enabled = state != ButtonState::Unavailable;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let galley = word.map(|w| {
        ui.painter().layout_no_wrap(
            w.to_string(),
            egui::FontId::proportional(WORD_SIZE),
            Color32::PLACEHOLDER,
        )
    });
    let width = BUTTON_SIZE + galley.as_ref().map_or(0.0, |g| g.size().x + 8.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON_SIZE), sense);
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
        let square = egui::Rect::from_min_size(rect.min, Vec2::splat(BUTTON_SIZE));
        let icon_rect = egui::Rect::from_center_size(square.center(), Vec2::splat(ICON_SIZE));
        icons::paint(painter, icon_rect, icon, color);
        if let Some(galley) = galley {
            let text = match state {
                ButtonState::Active => Color32::WHITE,
                ButtonState::Unavailable => dimmed(theme::TEXT),
                ButtonState::Normal if response.hovered() => theme::TEXT_STRONG,
                ButtonState::Normal => theme::TEXT,
            };
            let at = egui::pos2(square.max.x - 2.0, rect.center().y - galley.size().y / 2.0);
            painter.galley(at, galley, text);
        }
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
        | Icon::Icosphere
        | Icon::Torus
        | Icon::Capsule
        | Icon::Dome
        | Icon::Elbow
        | Icon::Sector
        | Icon::Gear
        | Icon::HexGrid
        | Icon::HexSphere
        | Icon::Stairs
        | Icon::Arch
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
    /// Select with this tool (from the select tool list).
    SetSelectTool(SelectTool),
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
    /// Apply the selected objects' mirrors (decision 0126).
    ApplyMirror,
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

/// From this much room across, the top palette writes words beside its
/// most used icons (decision 0126): the tools, Snap and Symmetry.
pub(crate) const WORDS_FROM_WIDTH: f32 = 1150.0;

/// The word beside a palette command's icon, when there's room.
fn word_for(command: PaletteCommand, select_tool: SelectTool) -> Option<&'static str> {
    Some(match command {
        PaletteCommand::NextSelectTool => select_tool.name(),
        PaletteCommand::Tool(Tool::Move) => "Move",
        PaletteCommand::Tool(Tool::Rotate) => "Rotate",
        PaletteCommand::Tool(Tool::Scale) => "Scale",
        PaletteCommand::ToggleSnapping => "Snap",
        PaletteCommand::ToggleSymmetry => "Symmetry",
        _ => return None,
    })
}

/// The accessible name of the arrow that opens the select tool list.
pub(crate) const SELECT_TOOL_MENU_LABEL: &str = "Select tools";

/// The small arrow beside the select tool button, and the list it opens
/// (decision 0126): box, lasso and paint select, the one in use ticked.
/// Returns the tool picked, if any.
fn select_tool_menu(ui: &mut Ui, current: SelectTool, in_use: bool) -> Option<SelectTool> {
    let (rect, response) = ui.allocate_exact_size(vec2(14.0, BUTTON_SIZE), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, SELECT_TOOL_MENU_LABEL));
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
        .on_hover_text("Pick how a drag selects: box, lasso or paint (Q steps through them)");
    let mut picked = None;
    egui::Popup::from_toggle_button_response(&response)
        .gap(4.0)
        .width(190.0)
        .show(|ui| {
            for tool in SelectTool::ALL {
                let ticked = in_use && tool == current;
                let text = if ticked {
                    format!("✓ {}", tool.name())
                } else {
                    format!("   {}", tool.name())
                };
                if ui.button(text).on_hover_text(tool.description()).clicked() {
                    picked = Some(tool);
                    ui.close();
                }
            }
        });
    picked
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
    let words = ui.available_width() >= WORDS_FROM_WIDTH;
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
                    | PaletteCommand::ApplyMirror
                    | PaletteCommand::SetSelectTool(_)
                    | PaletteCommand::Search => (true, command.arrives.to_string()),
                };
                let button_state = if active {
                    ButtonState::Active
                } else if available {
                    ButtonState::Normal
                } else {
                    ButtonState::Unavailable
                };
                let word = words.then(|| word_for(action, state.select_tool)).flatten();
                if worded_button(ui, icon, icon_tint(icon), label, &note, button_state, word)
                    .clicked()
                {
                    clicked = Some(action);
                }
                if matches!(action, PaletteCommand::NextSelectTool)
                    && let Some(tool) =
                        select_tool_menu(ui, state.select_tool, state.tool == Tool::Select)
                {
                    clicked = Some(PaletteCommand::SetSelectTool(tool));
                }
                if matches!(action, PaletteCommand::ToggleSymmetry) {
                    match symmetry_menu(ui, symmetry, state.symmetry, units) {
                        SymmetryClick::Toggle => clicked = Some(PaletteCommand::ToggleSymmetry),
                        SymmetryClick::Apply => clicked = Some(PaletteCommand::ApplyMirror),
                        SymmetryClick::Nothing => {}
                    }
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
) -> SymmetryClick {
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
        .map_or(SymmetryClick::Nothing, |shown| shown.inner)
}

/// What was clicked in the Symmetry menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymmetryClick {
    Nothing,
    /// On: turn the selected objects' mirror on or off.
    Toggle,
    /// Apply: make the mirrored half part of the shape (decision 0126).
    Apply,
}

/// The Symmetry menu's Apply button.
pub(crate) const APPLY_MIRROR_LABEL: &str = "Apply mirror";

/// The Symmetry menu's contents: On (for the selected objects), the axes,
/// where the mirror is and the weld distance, and Apply mirror. Returns
/// what was clicked.
pub(crate) fn symmetry_settings(
    ui: &mut Ui,
    symmetry: &mut crate::Symmetry,
    on: bool,
    units: crate::units::Units,
) -> SymmetryClick {
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
    let applied = ui
        .add_enabled(on, egui::Button::new(APPLY_MIRROR_LABEL))
        .on_hover_text(
            "Make the mirrored half part of the shape for real, as Blender's Apply: the mirror \
             is gone, both halves stay, and each half can be changed on its own from then on",
        )
        .on_disabled_hover_text("Turn symmetry on first: there's no mirror to apply")
        .clicked();
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
    if applied {
        SymmetryClick::Apply
    } else if toggled {
        SymmetryClick::Toggle
    } else {
        SymmetryClick::Nothing
    }
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
    /// Edit pivot switched on or off.
    EditPivot,
}

/// Accessible name of the mode palette's isolate button.
pub const ISOLATE_LABEL: &str = "Isolate";

/// The mode palette down the left side, with the viewport's display
/// toggles at the bottom. Returns what was picked this frame, if
/// anything; toggles change `options` directly.
pub fn mode_palette(ui: &mut Ui, current: Mode, options: &mut ViewOptions) -> Option<ModePick> {
    let mut picked = None;
    let mut names: Vec<Named> = Vec::new();
    let mut named = |response: &Response, name: String, active: bool, act: Act| {
        names.push(Named {
            rect: response.rect,
            name,
            active,
            act,
        });
    };
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
            let response = tinted_button(ui, mode.icon(), tint, &label, &mode.tooltip(), state);
            if response.clicked() {
                picked = Some(ModePick::Mode(mode));
            }
            named(
                &response,
                mode.name().to_string(),
                state == ButtonState::Active,
                Act::Pick(ModePick::Mode(mode)),
            );
            if mode == Mode::Model {
                // Edit pivot sits under the Model mode it belongs to, where
                // the Object mode was (the project lead's choice).
                let (pivot_state, pivot_note) = if current == Mode::Object {
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
                let response = icon_button(
                    ui,
                    Icon::ObjectMode,
                    EDIT_PIVOT_LABEL,
                    pivot_note,
                    pivot_state,
                );
                if response.clicked() {
                    picked = Some(ModePick::EditPivot);
                }
                named(
                    &response,
                    EDIT_PIVOT_LABEL.to_string(),
                    pivot_state == ButtonState::Active,
                    Act::Pick(ModePick::EditPivot),
                );
            }
        }
        divider(ui, false);
        let response = icon_button(
            ui,
            Icon::Solo,
            ISOLATE_LABEL,
            "Isolate: hide every other object, or in the Points, Edges and \
             Polygons modes every other polygon (Ctrl + H). Alt + H shows everything again.",
            ButtonState::Normal,
        );
        if response.clicked() {
            picked = Some(ModePick::Isolate);
        }
        named(
            &response,
            ISOLATE_LABEL.to_string(),
            false,
            Act::Pick(ModePick::Isolate),
        );
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
        let response = icon_button(ui, Icon::VisibleOnly, VISIBLE_ONLY_LABEL, &note, visible);
        if response.clicked() {
            options.select_visible_only = !options.select_visible_only;
        }
        named(
            &response,
            VISIBLE_ONLY_LABEL.to_string(),
            options.select_visible_only,
            Act::VisibleOnly,
        );
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
        let response = icon_button(ui, Icon::GridDetail, GRID_DETAIL_LABEL, &note, state);
        if response.clicked() {
            options.grid_detail = detail.next();
        }
        named(
            &response,
            GRID_DETAIL_LABEL.to_string(),
            state == ButtonState::Active,
            Act::GridDetail,
        );
    });
    // Any click on the palette (a mode, or a switch at the bottom).
    let acted = ui.input(|i| i.pointer.any_click()) && ui.rect_contains_pointer(ui.max_rect());
    match slide_out_names(ui, &names, acted) {
        Some(Act::Pick(pick)) => picked = Some(pick),
        Some(Act::VisibleOnly) => options.select_visible_only = !options.select_visible_only,
        Some(Act::GridDetail) => options.grid_detail = options.grid_detail.next(),
        None => {}
    }
    picked
}

/// What a name in the slid-out strip does when clicked: the same as its
/// button.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Act {
    Pick(ModePick),
    VisibleOnly,
    GridDetail,
}

/// A mode palette button, for the strip of names beside it.
struct Named {
    rect: egui::Rect,
    name: String,
    active: bool,
    act: Act,
}

/// How long the mouse rests on the mode palette before its names slide
/// out (decision 0126, the project lead's choice: on hover).
pub(crate) const NAMES_AFTER: f64 = 0.4;
/// How wide the strip of names is.
const NAMES_WIDTH: f32 = 170.0;
/// The accessible name of each name in the strip starts with this.
pub(crate) const NAMES_PREFIX: &str = "Palette name: ";

/// The strip that slides out beside the mode palette when the mouse rests
/// on it, with each button's name level with its icon. Clicking a name does
/// what its button does; the strip slides back when the mouse leaves both.
/// After a click (`acted`) it stays away until the mouse has left and
/// rests on the palette again, so it never covers the viewport you're about
/// to drag in. Returns what was clicked.
fn slide_out_names(ui: &Ui, names: &[Named], acted: bool) -> Option<Act> {
    let ctx = ui.ctx();
    let palette = ui.max_rect();
    let id = egui::Id::new("mode_palette_names");
    let now = ctx.input(|i| i.time);
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    // Where the strip was last drawn, and since when the mouse has been
    // over the palette or the strip.
    let (strip, since, dismissed): (Option<egui::Rect>, Option<f64>, bool) =
        ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    let over = pointer.is_some_and(|p| palette.contains(p) || strip.is_some_and(|s| s.contains(p)));
    let since = if over { since.or(Some(now)) } else { None };
    // A click puts the strip away until the mouse leaves.
    let dismissed = over && (dismissed || acted);
    let rested = !dismissed && since.is_some_and(|t| now - t >= NAMES_AFTER);
    if over && !rested {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(NAMES_AFTER));
    }
    let open = ctx.animate_bool_with_time(id.with("open"), rested, 0.12);
    if open <= 0.0 {
        ctx.data_mut(|d| d.insert_temp(id, (None::<egui::Rect>, since, dismissed)));
        return None;
    }
    let width = NAMES_WIDTH * open;
    let area = egui::Rect::from_min_size(
        egui::pos2(palette.max.x, palette.min.y),
        vec2(width, palette.height()),
    );
    let mut clicked = None;
    egui::Area::new(id.with("strip"))
        .order(egui::Order::Foreground)
        .fixed_pos(area.min)
        .show(ctx, |ui| {
            let (strip_rect, _) = ui.allocate_exact_size(area.size(), Sense::hover());
            let painter = ui.painter_at(strip_rect);
            painter.rect_filled(strip_rect, 0.0, theme::PALETTE);
            let accent = theme::accent(ctx).colors();
            for (index, named) in names.iter().enumerate() {
                let row = egui::Rect::from_min_max(
                    egui::pos2(strip_rect.min.x, named.rect.min.y),
                    egui::pos2(strip_rect.max.x, named.rect.max.y),
                );
                let response = ui.interact(row, id.with(("row", index)), Sense::click());
                response.widget_info(|| {
                    WidgetInfo::labeled(
                        WidgetType::Button,
                        true,
                        format!("{NAMES_PREFIX}{}", named.name),
                    )
                });
                let background = if named.active {
                    Some(accent.fill)
                } else if response.hovered() {
                    Some(ui.visuals().widgets.hovered.weak_bg_fill)
                } else {
                    None
                };
                if let Some(fill) = background {
                    painter.rect_filled(row.shrink2(vec2(2.0, 0.0)), 5.0, fill);
                }
                let colour = if named.active {
                    Color32::WHITE
                } else if response.hovered() {
                    theme::TEXT_STRONG
                } else {
                    theme::TEXT
                };
                painter.text(
                    row.left_center() + vec2(8.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &named.name,
                    egui::FontId::proportional(WORD_SIZE),
                    colour,
                );
                if response.clicked() {
                    clicked = Some(named.act);
                }
            }
        });
    let dismissed = dismissed || clicked.is_some();
    ctx.data_mut(|d| d.insert_temp(id, (Some(area), since, dismissed)));
    clicked
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
