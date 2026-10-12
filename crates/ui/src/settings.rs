//! The Settings window: every app-wide preference in one place, in
//! sections (Interface, Viewport, Undo, Autosave). Opened from Edit → Settings… or
//! with `Ctrl + ,`.
//!
//! Switches you flip while working stay where you use them too (metres or
//! centimetres beside the Shape title, the navigation picker in the
//! viewport's corner, the grid button); they show the same values here.

use crate::autosave::{AutosaveSettings, COPIES, MINUTES};
use crate::selecting::SelectKeys;
use crate::theme::{self, Accent};
use crate::tool_menu::ToolMenuStyle;
use crate::transforming::SnapSettings;
use crate::undo_memory::{UndoMemory, format_bytes};
use crate::units::Units;
use crate::viewport::GridDetail;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Ui, vec2};
use tools::{NavigationSettings, Scheme};

/// Opens the Settings window.
pub const SETTINGS_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma);

/// The window's title.
pub(crate) const SETTINGS_TITLE: &str = "Settings";

/// A part of the Settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Section {
    /// How the app looks, and the units lengths are shown in.
    #[default]
    Interface,
    /// How the mouse moves the view, and the grid.
    Viewport,
    /// How much the undo history may keep.
    Undo,
    /// Backup copies made in the background.
    Autosave,
}

impl Section {
    pub(crate) const ALL: [Section; 4] =
        [Self::Interface, Self::Viewport, Self::Undo, Self::Autosave];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Viewport => "Viewport",
            Self::Undo => "Undo",
            Self::Autosave => "Autosave",
        }
    }
}

/// What the Settings window shows and changes: the workbench's own
/// values, borrowed for the frame.
pub(crate) struct Values<'a> {
    pub(crate) accent: &'a mut Accent,
    pub(crate) units: &'a mut Units,
    pub(crate) navigation: &'a mut NavigationSettings,
    pub(crate) grid: &'a mut GridDetail,
    /// Whether box, lasso and brush select only what can be seen.
    pub(crate) select_visible_only: &'a mut bool,
    /// Which keys add to and take from the selection.
    pub(crate) select_keys: &'a mut SelectKeys,
    /// How moving, rotating and scaling snap.
    pub(crate) snap: &'a mut SnapSettings,
    pub(crate) undo_memory: &'a mut UndoMemory,
    /// What the undo history uses now: bytes and steps.
    pub(crate) history: (usize, usize),
    /// How backup copies are made.
    pub(crate) autosave: &'a mut AutosaveSettings,
    /// Where they go, if known.
    pub(crate) autosave_folder: Option<&'a std::path::Path>,
    /// How the tool menu (E) looks.
    pub(crate) tool_menu: &'a mut ToolMenuStyle,
}

/// The Settings window's state.
#[derive(Debug, Clone, Default)]
pub(crate) struct SettingsWindow {
    pub(crate) open: bool,
    pub(crate) section: Section,
}

impl SettingsWindow {
    /// Draws the window, if open.
    pub(crate) fn show(&mut self, ctx: &egui::Context, values: &mut Values<'_>) {
        if !self.open {
            return;
        }
        let mut open = true;
        egui::Window::new(SETTINGS_TITLE)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size(vec2(560.0, 420.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(120.0);
                        for section in Section::ALL {
                            let response =
                                ui.selectable_label(self.section == section, section.name());
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    true,
                                    self.section == section,
                                    format!("Settings section: {}", section.name()),
                                )
                            });
                            if response.clicked() {
                                self.section = section;
                            }
                        }
                        ui.set_min_height(360.0);
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                            if ui
                                .button("Restore standard")
                                .on_hover_text(
                                    "Put every setting back as it was when the app was new \
                                 (the panel layout is kept)",
                                )
                                .clicked()
                            {
                                restore_standard(values);
                            }
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(400.0);
                        ui.set_min_height(360.0);
                        match self.section {
                            Section::Interface => interface(ui, values),
                            Section::Viewport => viewport(ui, values),
                            Section::Undo => undo(ui, values),
                            Section::Autosave => autosave(ui, values),
                        }
                    });
                });
            });
        if !open {
            self.open = false;
        }
    }
}

/// Puts every setting back to its standard value.
fn restore_standard(values: &mut Values<'_>) {
    *values.accent = Accent::default();
    *values.units = Units::default();
    *values.navigation = NavigationSettings::default();
    *values.grid = GridDetail::default();
    *values.select_visible_only = false;
    *values.select_keys = SelectKeys::default();
    *values.snap = SnapSettings::default();
    *values.undo_memory = UndoMemory::default();
    *values.autosave = AutosaveSettings::default();
    *values.tool_menu = ToolMenuStyle::default();
}

/// A heading for a group of settings.
fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).strong().color(theme::TEXT_STRONG));
}

/// A short explanation under a heading.
fn explain(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
    ui.add_space(2.0);
}

fn interface(ui: &mut Ui, values: &mut Values<'_>) {
    heading(ui, "Accent colour");
    explain(
        ui,
        "The one colour that marks what's active: the current mode, the chosen option, \
         the selection.",
    );
    for accent in Accent::ALL {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (swatch, _) = ui.allocate_exact_size(vec2(16.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(swatch, 3.0, accent.colors().fill);
            if ui.radio(*values.accent == accent, accent.name()).clicked() {
                *values.accent = accent;
            }
        });
    }
    ui.add_space(8.0);
    heading(ui, "Lengths");
    explain(
        ui,
        "How sizes are shown and typed. Only the display changes, never the shapes. \
         Also switched beside the Shape title in the Attributes panel.",
    );
    for units in Units::ALL {
        if ui
            .radio(
                *values.units == units,
                format!("{} ({})", units.name(), units.symbol()),
            )
            .clicked()
        {
            *values.units = units;
        }
    }
    ui.add_space(8.0);
    heading(ui, "Tool menu (E)");
    explain(
        ui,
        "Press E for the tools of what you're working on. Both looks hold the same \
         tools in the same order; tap E to step through them, Enter or Space to use one.",
    );
    for style in ToolMenuStyle::ALL {
        if ui
            .radio(*values.tool_menu == style, style.name())
            .on_hover_text(style.description())
            .clicked()
        {
            *values.tool_menu = style;
        }
    }
}

fn viewport(ui: &mut Ui, values: &mut Values<'_>) {
    heading(ui, "Navigation controls");
    explain(
        ui,
        "How the mouse moves the view. Also chosen from the button in the viewport's \
         bottom-right corner.",
    );
    for scheme in Scheme::ALL {
        if ui
            .radio(values.navigation.scheme == scheme, scheme.name())
            .on_hover_text(scheme.summary())
            .clicked()
        {
            values.navigation.scheme = scheme;
        }
    }
    ui.add_space(4.0);
    ui.checkbox(
        &mut values.navigation.alt_everywhere,
        "Alt + mouse works in every scheme",
    );
    ui.checkbox(
        &mut values.navigation.around_cursor,
        "Orbit and zoom around the point under the mouse",
    );
    ui.checkbox(
        &mut values.navigation.invert_orbit,
        "Invert orbit direction",
    );
    ui.add_space(8.0);
    heading(ui, "Grid lines");
    explain(
        ui,
        "Also stepped through with the grid button on the left palette.",
    );
    for level in GridDetail::ALL {
        if ui
            .radio(*values.grid == level, level.label())
            .on_hover_text(level.explanation())
            .clicked()
        {
            *values.grid = level;
        }
    }
    ui.add_space(8.0);
    heading(ui, "Selecting");
    explain(
        ui,
        "Also switched with the button above the grid button on the left palette.",
    );
    ui.checkbox(values.select_visible_only, "Select only visible")
        .on_hover_text(crate::workbench::VISIBLE_ONLY_HELP);
    ui.add_space(4.0);
    explain(
        ui,
        "Selection keys: which key adds to the selection and which takes away, \
         for clicks, boxes, lassos, the brush and double-clicks alike. A plain \
         click or drag always starts a new selection; Alt + double-click selects a ring.",
    );
    for keys in SelectKeys::ALL {
        if ui
            .radio(*values.select_keys == keys, keys.name())
            .on_hover_text(keys.summary())
            .clicked()
        {
            *values.select_keys = keys;
        }
    }
    explain(ui, values.select_keys.summary());
    ui.add_space(8.0);
    heading(ui, "Snapping");
    explain(
        ui,
        "While moving, rotating and scaling. The snapping button on the top palette \
         turns it on and off.",
    );
    let snap = &mut *values.snap;
    ui.checkbox(&mut snap.on, "Snapping on");
    ui.checkbox(
        &mut snap.ctrl_flips,
        "Holding Ctrl while dragging flips snapping for that drag",
    )
    .on_hover_text("Off: only the button turns snapping on and off.");
    ui.label("Moves snap to:");
    ui.horizontal(|ui| {
        ui.checkbox(&mut snap.points, "Points");
        ui.checkbox(&mut snap.edges, "Edges");
        ui.checkbox(&mut snap.polygons, "Polygons");
        ui.checkbox(&mut snap.grid, "Grid lines").on_hover_text(
            "The lines shown: every 1 m, 50 cm or 10 cm, as the grid button is set.",
        );
    });
    ui.horizontal(|ui| {
        ui.label("Turns snap every");
        ui.add(
            egui::DragValue::new(&mut snap.angle)
                .range(1.0..=90.0)
                .speed(0.5)
                .suffix("°"),
        );
        ui.label("and scaling every");
        let mut percent = snap.scale * 100.0;
        if ui
            .add(
                egui::DragValue::new(&mut percent)
                    .range(1.0..=100.0)
                    .speed(0.5)
                    .suffix("%"),
            )
            .changed()
        {
            snap.scale = percent / 100.0;
        }
    });
}

fn undo(ui: &mut Ui, values: &mut Values<'_>) {
    heading(ui, "Undo memory");
    explain(
        ui,
        "Undo keeps steps until they fill this much memory, then forgets the oldest first. \
         There is no limit on the number of steps, and the newest step is always kept.",
    );
    let (bytes, steps) = values.history;
    ui.label(format!(
        "In use: {} of {} ({} {})",
        format_bytes(bytes),
        values.undo_memory.short(),
        steps,
        if steps == 1 { "step" } else { "steps" }
    ));
    ui.add_space(4.0);
    for choice in UndoMemory::ALL {
        if ui
            .radio(*values.undo_memory == choice, choice.label())
            .on_hover_text(choice.explanation())
            .clicked()
        {
            *values.undo_memory = choice;
        }
    }
}

fn autosave(ui: &mut Ui, values: &mut Values<'_>) {
    heading(ui, "Backup copies");
    explain(
        ui,
        "While you work, the app saves a backup copy of your scene in the background, only \
         when something changed. If the app ever closes without saving, it offers the copies \
         the next time it starts. They never replace your project file: saving does that.",
    );
    let settings = &mut *values.autosave;
    ui.checkbox(&mut settings.on, "Save backup copies in the background");
    ui.add_enabled_ui(settings.on, |ui| {
        ui.horizontal(|ui| {
            ui.label("Every");
            let unit = if settings.minutes == 1 {
                " minute"
            } else {
                " minutes"
            };
            ui.add(
                egui::DragValue::new(&mut settings.minutes)
                    .range(MINUTES)
                    .speed(0.2)
                    .suffix(unit),
            )
            .on_hover_text("Counted from the first change since the last copy.");
        });
        ui.horizontal(|ui| {
            ui.label("Keep the newest");
            let unit = if settings.copies == 1 {
                " copy"
            } else {
                " copies"
            };
            ui.add(
                egui::DragValue::new(&mut settings.copies)
                    .range(COPIES)
                    .speed(0.2)
                    .suffix(unit),
            )
            .on_hover_text(
                "More copies let you go further back after a crash. A heavy scene takes \
                 tens of MB per copy.",
            );
        });
    });
    ui.add_space(8.0);
    heading(ui, "Where they go");
    match values.autosave_folder {
        Some(folder) => {
            explain(
                ui,
                "The app's own folder, so your project folders stay tidy. Copies are deleted \
                 when the app closes normally.",
            );
            ui.add(
                egui::Label::new(RichText::new(folder.display().to_string()).monospace())
                    .selectable(true),
            );
        }
        None => {
            explain(
                ui,
                "No folder for backup copies could be found on this computer.",
            );
        }
    }
}
