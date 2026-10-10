//! The app window's contents: menu bar, palettes, docked panels, status
//! bar and the About box.

use crate::layout::{default_layout, fit_coordinates, restore_or_default};
use crate::palette::{Mode, command_palette, mode_palette};
use crate::panel::Panel;
use crate::panels::{about_contents, attributes_contents, coordinates_contents, objects_contents};
use crate::theme;
use crate::viewport::{ViewportPanel, ViewportRenderer};
use egui::{Align2, Id, Key, KeyboardShortcut, Modifiers, Ui, WidgetText};
use egui_dock::{DockArea, DockState, TabViewer};

/// Facts about the program, shown in the window and the About box.
/// The app passes these in from its branding, so this crate never names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// Full product name, such as "Aventail 3D".
    pub name: String,
    /// Version, such as "0.0.1".
    pub version: String,
    /// The family of tools the app belongs to, such as "Rhylib".
    pub family: String,
}

/// Keyboard shortcut for File → Quit (Ctrl+Q, or Cmd+Q on macOS).
pub const QUIT_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Q);

/// Something the user asked for that the program around the workbench
/// must carry out (the workbench never touches files or the window itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchAction {
    /// Close the window.
    Quit,
}

/// Menu entries that exist in the design but not yet in the app, with the
/// phase that brings them.
const COMING: [(&str, &[(&str, &str)]); 6] = [
    (
        "Create",
        &[
            ("Cube, cylinder, plane, sphere and more", "Phase 2"),
            ("Null (empty group)", "Phase 2"),
        ],
    ),
    (
        "Select",
        &[
            ("Live selection, rectangle and lasso", "Phase 3"),
            ("Loop, ring, grow and shrink", "Phase 3"),
        ],
    ),
    (
        "Tools",
        &[
            ("Move, rotate and scale", "Phase 3"),
            ("Snapping", "Phase 3"),
        ],
    ),
    (
        "Mesh",
        &[
            ("Extrude, inset, bevel", "Phase 4"),
            ("Loop cut, knife, bridge, weld", "Phase 4"),
        ],
    ),
    (
        "UV",
        &[
            ("Seams, unwrap, relax", "Phase 6"),
            ("Pack and texel density", "Phase 6"),
        ],
    ),
    ("Window", &[("UV Editor panel", "Phase 6")]),
];

/// The whole interface around the scene.
pub struct Workbench {
    info: AppInfo,
    dock: DockState<Panel>,
    /// The standard layout still needs Coordinates fitted to its content,
    /// which can only happen once the panels' sizes are known.
    fit_pending: bool,
    mode: Mode,
    viewport: ViewportPanel,
    about_open: bool,
    /// A message for the status bar, such as "Panel layout reset".
    note: Option<String>,
}

impl Workbench {
    /// A workbench with the saved panel layout, if it is usable, or the
    /// standard one.
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        let had_saved = saved_layout.is_some();
        let (dock, note) = restore_or_default(saved_layout);
        // A saved layout keeps the user's own sizes; a new or replaced one
        // is fitted on its first frame.
        let fit_pending = !had_saved || note.is_some();
        Self {
            info,
            dock,
            fit_pending,
            mode: Mode::default(),
            viewport: ViewportPanel::default(),
            about_open: false,
            note,
        }
    }

    /// The current panel layout, for saving between sessions.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The text for the window's title bar.
    pub fn window_title(&self) -> String {
        format!("{} {}", self.info.name, self.info.version)
    }

    /// The current mode (Model, Object, Points and so on).
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The status bar's message, if any.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// Whether the About box is showing.
    pub fn is_about_open(&self) -> bool {
        self.about_open
    }

    /// Puts every panel back in its standard place (View → Reset panel layout).
    pub fn reset_layout(&mut self) {
        self.dock = default_layout();
        self.fit_pending = true;
        self.note = Some("Panel layout reset to the standard arrangement.".into());
    }

    /// The 3D viewport panel.
    pub fn viewport(&self) -> &ViewportPanel {
        &self.viewport
    }

    /// Draws the whole window and returns what the user asked for.
    /// `renderer` draws the 3D viewport.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = Vec::new();
        if ui.input_mut(|i| i.consume_shortcut(&QUIT_SHORTCUT)) {
            actions.push(WorkbenchAction::Quit);
        }

        egui::Panel::top("menu_bar")
            .frame(theme::menu_bar_frame())
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| self.menu_bar(ui, &mut actions));
            });
        egui::Panel::top("command_palette")
            .frame(theme::palette_frame())
            .exact_size(40.0)
            .show(ui, command_palette);
        egui::Panel::bottom("status_bar")
            .frame(theme::status_bar_frame())
            .exact_size(24.0)
            .show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("mode_palette")
            .frame(theme::palette_frame())
            .exact_size(44.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(6.0);
                if let Some(mode) = mode_palette(ui, self.mode) {
                    self.mode = mode;
                }
            });

        let style = theme::dock_style(ui.style().as_ref());
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .show_close_buttons(false)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(false)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        viewport: &mut self.viewport,
                        renderer,
                    },
                );
        });
        if self.fit_pending && fit_coordinates(&mut self.dock) {
            self.fit_pending = false;
            ui.ctx().request_repaint();
        }

        let mut about_open = self.about_open;
        egui::Window::new(format!("About {}", self.info.name))
            .open(&mut about_open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| about_contents(ui, &self.info));
        self.about_open = about_open;

        actions
    }

    fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.label(format!(
                "{} mode: {}.",
                self.mode.name(),
                self.mode.description().to_lowercase()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let note = self
                    .note
                    .as_deref()
                    .unwrap_or("Panels can be dragged by their tabs and resized at their edges.");
                ui.label(egui::RichText::new(note).weak());
            });
        });
    }

    fn menu_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("File", |ui| {
            for item in ["New", "Open…", "Save", "Save As…"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Projects arrive in Phase 2");
            }
            ui.add_enabled(false, egui::Button::new("Export…"))
                .on_disabled_hover_text("Export arrives in Phase 5");
            ui.separator();
            let keys = ui.ctx().format_shortcut(&QUIT_SHORTCUT);
            if ui
                .add(egui::Button::new("Quit").shortcut_text(keys))
                .clicked()
            {
                actions.push(WorkbenchAction::Quit);
            }
        });
        ui.menu_button("Edit", |ui| {
            for item in ["Undo", "Redo"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Undo arrives in Phase 2");
            }
        });
        // Menu order as in DESIGN.md: File, Edit, Create, Modes, Select,
        // Tools, Mesh, UV, View, Window, Help.
        coming_menus(ui, &COMING[..1]);
        ui.menu_button("Modes", |ui| {
            for mode in Mode::ALL {
                if ui
                    .radio(self.mode == mode, mode.name())
                    .on_hover_text(mode.description())
                    .clicked()
                {
                    self.mode = mode;
                    ui.close();
                }
            }
        });
        coming_menus(ui, &COMING[1..5]);
        ui.menu_button("View", |ui| {
            if ui.button("Reset panel layout").clicked() {
                self.reset_layout();
                ui.close();
            }
        });
        coming_menus(ui, &COMING[5..]);
        ui.menu_button("Help", |ui| {
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
    }
}

/// Menus whose commands are not built yet: each lists what is coming and
/// when, greyed out.
fn coming_menus(ui: &mut Ui, menus: &[(&str, &[(&str, &str)])]) {
    for (title, items) in menus {
        ui.menu_button(*title, |ui| {
            for (item, phase) in *items {
                ui.add_enabled(false, egui::Button::new(*item))
                    .on_disabled_hover_text(format!("Arrives in {phase}"));
            }
        });
    }
}

/// Draws each panel's contents.
struct PanelViewer<'a> {
    viewport: &'a mut ViewportPanel,
    renderer: &'a mut dyn ViewportRenderer,
}

impl TabViewer for PanelViewer<'_> {
    type Tab = Panel;

    fn id(&mut self, tab: &mut Panel) -> Id {
        Id::new(("panel", *tab))
    }

    fn title(&mut self, tab: &mut Panel) -> WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Panel) {
        if *tab == Panel::Viewport {
            // Edge to edge, with no padding.
            self.viewport.show(ui, self.renderer);
            return;
        }
        egui::Frame::new()
            .inner_margin(theme::panel_margin())
            .show(ui, |ui| match tab {
                Panel::Viewport => {}
                Panel::Attributes => attributes_contents(ui),
                Panel::Objects => objects_contents(ui),
                Panel::Coordinates => coordinates_contents(ui),
            });
    }

    fn is_closeable(&self, _tab: &Panel) -> bool {
        false
    }

    fn on_tab_button(&mut self, tab: &mut Panel, response: &egui::Response) {
        // Name the tab for screen readers (and UI tests); egui_dock paints
        // the title without labelling the button.
        let title = tab.title();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    }

    fn allowed_in_windows(&self, _tab: &mut Panel) -> bool {
        // Panels dock inside the main window only: a floating window could
        // be closed and take its panel with it.
        false
    }

    fn clear_background(&self, tab: &Panel) -> bool {
        // The viewport paints its own background.
        *tab != Panel::Viewport
    }

    fn scroll_bars(&self, tab: &Panel) -> [bool; 2] {
        // The viewport fills its panel exactly.
        if *tab == Panel::Viewport {
            [false, false]
        } else {
            [true, true]
        }
    }
}

#[cfg(test)]
mod tests;
