//! The app window's contents: menu bar, palettes, docked panels, status
//! bar and the About box.

use crate::attributes::{AttributesPanel, Shared};
use crate::drawing::{DrawingCache, SceneSummary};
use crate::layout::{default_layout, fit_coordinates, hide_viewport_tab, restore_or_default};
use crate::objects::{self, DocRequest, ObjectManager};
use crate::palette::{Mode, PaletteCommand, PaletteState, command_palette, mode_palette};
use crate::panel::Panel;
use crate::panels::{about_contents, coordinates_contents, menu_line_with};
use crate::shape_picker::ShapePicker;
use crate::theme::{self, Accent};
use crate::units::Units;
use crate::viewport::{ViewOptions, ViewportPanel, ViewportRenderer};
use egui::{Align2, Id, Key, KeyboardShortcut, Modifiers, Ui, WidgetText};
use egui_dock::{DockArea, DockState, TabViewer};
use scene::{Command, Document, ObjectKind, PrimitiveKind};

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

/// Layouts shown on the right of the menu bar: the standard one, and
/// arrangements for particular kinds of work that arrive later.
const LAYOUTS: [(&str, Option<&str>); 3] = [
    ("Standard", None),
    ("Modelling", Some("A layout for modelling arrives later")),
    ("UV Edit", Some("A layout for UV work arrives in Phase 6")),
];

/// Something the user asked for that the program around the workbench
/// must carry out (the workbench never touches files or the window itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchAction {
    /// Close the window.
    Quit,
}

/// Menu entries that exist in the design but not yet in the app, with the
/// phase that brings them.
const COMING: [(&str, &[(&str, &str)]); 4] = [
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
];

/// The whole interface around the scene.
pub struct Workbench {
    info: AppInfo,
    dock: DockState<Panel>,
    /// The standard layout still needs Coordinates fitted to its content,
    /// which can only happen once the panels' sizes are known.
    fit_pending: bool,
    mode: Mode,
    /// The scene being worked on, with its undo history and selection.
    document: Document,
    objects: ObjectManager,
    attributes: AttributesPanel,
    /// Metres or centimetres, for every length shown or typed.
    units: Units,
    /// The `B` shape list.
    shape_picker: ShapePicker,
    /// What the viewport draws, kept up to date with the document.
    drawing: DrawingCache,
    viewport: ViewportPanel,
    accent: Accent,
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
            document: Document::new(),
            objects: ObjectManager::default(),
            attributes: AttributesPanel::default(),
            units: Units::default(),
            shape_picker: ShapePicker::default(),
            drawing: DrawingCache::default(),
            viewport: ViewportPanel::default(),
            accent: Accent::default(),
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

    /// The scene being worked on.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The scene being worked on, for changing it in tests.
    #[cfg(test)]
    pub(crate) fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The shape list's state, for tests.
    #[cfg(test)]
    pub(crate) fn shape_picker(&self) -> &ShapePicker {
        &self.shape_picker
    }

    /// The Object Manager's state, for tests.
    #[cfg(test)]
    pub(crate) fn object_manager(&self) -> &ObjectManager {
        &self.objects
    }

    /// The 3D viewport panel.
    pub fn viewport(&self) -> &ViewportPanel {
        &self.viewport
    }

    /// The 3D viewport panel, for changing it in tests.
    #[cfg(test)]
    pub(crate) fn viewport_mut(&mut self) -> &mut ViewportPanel {
        &mut self.viewport
    }

    /// The viewport's display choices, for saving between sessions.
    pub fn view_options(&self) -> ViewOptions {
        self.viewport.options()
    }

    /// Metres or centimetres, for saving between sessions.
    pub fn units(&self) -> Units {
        self.units
    }

    /// The workbench with a remembered choice of units restored.
    #[must_use]
    pub fn with_units(mut self, units: Units) -> Self {
        self.units = units;
        self
    }

    /// The accent colour, for saving between sessions.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// The workbench with a remembered accent colour restored.
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Changes the accent colour (Window → Accent colour).
    pub fn set_accent(&mut self, accent: Accent) {
        self.accent = accent;
    }

    /// The workbench with remembered display choices restored.
    #[must_use]
    pub fn with_view_options(mut self, options: ViewOptions) -> Self {
        *self.viewport.options_mut() = options.migrated();
        self
    }

    /// Draws the whole window and returns what the user asked for.
    /// `renderer` draws the 3D viewport.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = Vec::new();
        if theme::accent(ui.ctx()) != self.accent {
            theme::set_accent(ui.ctx(), self.accent);
        }
        if ui.input_mut(|i| i.consume_shortcut(&QUIT_SHORTCUT)) {
            actions.push(WorkbenchAction::Quit);
        }
        let mut requests = Vec::new();
        // The shape list goes first: while it is open, Esc closes it
        // rather than deselecting.
        let typing = ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming();
        if let Some(kind) = self.shape_picker.keys(ui, typing) {
            requests.push(create(kind));
        }
        self.document_keys(ui, &mut requests);

        egui::Panel::top("menu_bar")
            .frame(theme::menu_bar_frame())
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| self.menu_bar(ui, &mut actions, &mut requests));
            });
        let palette_state = PaletteState {
            undo: self.document.undo_label().map(str::to_string),
            redo: self.document.redo_label().map(str::to_string),
        };
        egui::Panel::top("command_palette")
            .frame(theme::palette_frame())
            .exact_size(40.0)
            .show(ui, |ui| {
                if let Some(command) = command_palette(ui, &palette_state) {
                    requests.push(match command {
                        PaletteCommand::Undo => DocRequest::Undo,
                        PaletteCommand::Redo => DocRequest::Redo,
                        PaletteCommand::CreateGroup => {
                            DocRequest::Run(Command::Create(ObjectKind::Group))
                        }
                        PaletteCommand::CreateShape(kind) => {
                            DocRequest::Run(Command::Create(ObjectKind::Primitive(kind.standard())))
                        }
                    });
                }
            });
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
                if let Some(mode) = mode_palette(ui, self.mode, self.viewport.options_mut()) {
                    self.mode = mode;
                }
            });

        let summary: SceneSummary = self.drawing.update(&self.document).clone();
        hide_viewport_tab(&mut self.dock);
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .hidable_tab_bars(true)
                .tab_context_menus(false)
                .show_close_buttons(false)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(false)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        viewport: &mut self.viewport,
                        renderer,
                        summary: &summary,
                        document: &mut self.document,
                        objects: &mut self.objects,
                        attributes: &mut self.attributes,
                        units: &mut self.units,
                        requests: &mut requests,
                    },
                );
        });
        if self.fit_pending && fit_coordinates(&mut self.dock) {
            self.fit_pending = false;
            ui.ctx().request_repaint();
        }

        if let Some(kind) = self.shape_picker.show(ui.ctx()) {
            requests.push(create(kind));
        }
        if !requests.is_empty() {
            self.carry_out(requests);
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

    /// Keys that change the document: undo, redo, delete, deselect, group
    /// and ungroup. Ignored while typing in a text box, which has its own.
    fn document_keys(&mut self, ui: &Ui, out: &mut Vec<DocRequest>) {
        // (While a name is typed, Esc means "keep the old name", even on
        // the frame egui has already taken the keyboard away.)
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        let popup_open = egui::Popup::is_any_open(ui.ctx());
        let selection = self.document.selection();
        let can_ungroup = objects::has_group(&self.document, &selection);
        let mut note = None;
        ui.input_mut(|i| {
            // Ctrl+Shift+Z first: Ctrl+Z would match it too.
            if i.consume_shortcut(&objects::REDO_SHORTCUT_2)
                || i.consume_shortcut(&objects::REDO_SHORTCUT)
            {
                out.push(DocRequest::Redo);
            } else if i.consume_shortcut(&objects::UNDO_SHORTCUT) {
                out.push(DocRequest::Undo);
            }
            if i.consume_shortcut(&objects::UNGROUP_SHORTCUT) && !selection.is_empty() {
                if can_ungroup {
                    out.push(DocRequest::Run(Command::Ungroup(selection.clone())));
                } else {
                    note = Some("Nothing to ungroup: only groups can be ungrouped.".to_string());
                }
            }
            if i.consume_shortcut(&objects::GROUP_SHORTCUT) && !selection.is_empty() {
                out.push(DocRequest::Run(Command::Group(selection.clone())));
            }
            let delete = i.consume_shortcut(&objects::DELETE_SHORTCUT)
                | i.consume_key(Modifiers::NONE, Key::Backspace);
            if delete && !selection.is_empty() {
                out.push(DocRequest::Run(Command::Delete(selection.clone())));
            }
        });
        if note.is_some() {
            self.note = note;
        }
        // Esc closes an open menu first; otherwise it deselects.
        if !popup_open && ui.input_mut(|i| i.consume_shortcut(&objects::DESELECT_SHORTCUT)) {
            self.document.clear_selection();
        }
    }

    /// Carries out what the menus, keys and panels asked for this frame,
    /// saying in the status bar what happened when it isn't plain to see.
    fn carry_out(&mut self, requests: Vec<DocRequest>) {
        for request in requests {
            self.note = match request {
                DocRequest::Run(command) => {
                    let what = command.describe(self.document.scene());
                    match self.document.execute(command) {
                        Ok(()) => None,
                        Err(why) => Some(format!("Couldn't {}: {why}.", what.to_lowercase())),
                    }
                }
                DocRequest::RunMerging(command, key) => {
                    let what = command.describe(self.document.scene());
                    match self.document.execute_merging(command, key) {
                        Ok(()) => None,
                        Err(why) => Some(format!("Couldn't {}: {why}.", what.to_lowercase())),
                    }
                }
                DocRequest::Undo => Some(match self.document.undo() {
                    Some(what) => format!("Undone: {what}."),
                    None => "Nothing to undo.".into(),
                }),
                DocRequest::Redo => Some(match self.document.redo() {
                    Some(what) => format!("Redone: {what}."),
                    None => "Nothing to redo.".into(),
                }),
            };
        }
    }

    fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            let selected = match self.document.selection().len() {
                0 => String::new(),
                1 => " 1 object selected.".into(),
                n => format!(" {n} objects selected."),
            };
            ui.label(format!(
                "{} mode: {}.{selected}",
                self.mode.name(),
                self.mode.description().to_lowercase()
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let note = match self.note.as_deref() {
                    Some(note) => note.to_string(),
                    None => navigation_hint(self.viewport.options().navigation.scheme),
                };
                ui.label(egui::RichText::new(note).weak());
            });
        });
    }

    fn menu_bar(
        &mut self,
        ui: &mut Ui,
        actions: &mut Vec<WorkbenchAction>,
        requests: &mut Vec<DocRequest>,
    ) {
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
            objects::undo_entries(ui, &self.document, requests);
            ui.separator();
            objects::selection_entries(ui, &mut self.document, &mut self.objects, requests);
        });
        // Menu order as in DESIGN.md: File, Edit, Create, Modes, Select,
        // Tools, Mesh, UV, View, Window, Help.
        ui.menu_button("Create", |ui| {
            for kind in PrimitiveKind::ALL {
                if ui
                    .button(kind.name())
                    .on_hover_text(kind.description())
                    .clicked()
                {
                    requests.push(create(kind));
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button("Group")
                .on_hover_text("An empty object to hold and organise other objects")
                .clicked()
            {
                requests.push(DocRequest::Run(Command::Create(ObjectKind::Group)));
                ui.close();
            }
            ui.separator();
            ui.label(
                egui::RichText::new("Tip: press B for the shape list")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        });
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
        coming_menus(ui, &COMING);
        ui.menu_button("View", |ui| {
            if ui
                .button("Reset view")
                .on_hover_text("Go back to the opening camera view")
                .clicked()
            {
                self.viewport.reset_view();
                ui.close();
            }
            crate::viewport::chrome::frame_entries(ui, &mut self.viewport);
            ui.separator();
            crate::viewport::chrome::grid_entries(ui, &mut self.viewport.options_mut().grid_detail);
            ui.separator();
            if ui.button("Reset panel layout").clicked() {
                self.reset_layout();
                ui.close();
            }
        });
        ui.menu_button("Window", |ui| {
            ui.menu_button("Accent colour", |ui| {
                for accent in Accent::ALL {
                    if accent_choice(ui, accent, self.accent).clicked() {
                        self.accent = accent;
                        ui.close();
                    }
                }
            });
            ui.add_enabled(false, egui::Button::new("UV Editor panel"))
                .on_disabled_hover_text("Arrives in Phase 6");
        });
        ui.menu_button("Help", |ui| {
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
        layout_tabs(ui);
    }
}

/// The request that creates a primitive of `kind` with standard settings.
fn create(kind: PrimitiveKind) -> DocRequest {
    DocRequest::Run(Command::Create(ObjectKind::Primitive(kind.standard())))
}

/// The status bar's standing hint: the navigation controls in use.
pub fn navigation_hint(scheme: tools::Scheme) -> String {
    format!("{}: {}", scheme.name(), scheme.summary())
}

/// One accent colour in the Window menu: a tick on the one in use, a
/// swatch, and its name. The swatch sits inside the row, before the name,
/// so it never spills out of the menu.
fn accent_choice(ui: &mut Ui, accent: Accent, current: Accent) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (swatch, _) = ui.allocate_exact_size(egui::vec2(16.0, 10.0), egui::Sense::hover());
        ui.painter().rect_filled(swatch, 3.0, accent.colors().fill);
        ui.add(egui::RadioButton::new(accent == current, accent.name()))
    })
    .inner
}

/// The layout tabs on the right of the menu bar. Only the standard layout
/// exists so far; the others are shown greyed out with when they arrive.
fn layout_tabs(ui: &mut Ui) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.add_space(6.0);
        for (name, coming) in LAYOUTS.iter().rev() {
            match coming {
                None => {
                    let accent = theme::accent(ui.ctx()).colors();
                    let response = ui.add(
                        egui::Label::new(egui::RichText::new(*name).color(theme::TEXT_STRONG))
                            .selectable(false),
                    );
                    let rect = response.rect;
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom() + 3.0,
                        egui::Stroke::new(2.0, accent.light),
                    );
                    response.on_hover_text("The layout in use");
                }
                Some(when) => {
                    ui.add_enabled(false, egui::Label::new(*name).selectable(false))
                        .on_disabled_hover_text(*when);
                }
            }
        }
    });
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
    summary: &'a SceneSummary,
    document: &'a mut Document,
    objects: &'a mut ObjectManager,
    attributes: &'a mut AttributesPanel,
    units: &'a mut Units,
    requests: &'a mut Vec<DocRequest>,
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
            self.viewport.show(ui, self.renderer, self.summary);
            return;
        }
        match tab {
            Panel::Attributes => menu_line_with(ui, |ui| {
                let mut shared = Shared {
                    clipboard: &mut self.objects.clipboard,
                    units: self.units,
                };
                self.attributes
                    .menus(ui, self.document, &mut shared, self.requests);
            }),
            Panel::Objects => menu_line_with(ui, |ui| {
                objects::menus(ui, self.document, self.objects, self.requests);
            }),
            Panel::Viewport | Panel::Coordinates => {}
        }
        egui::Frame::new()
            .inner_margin(theme::panel_margin())
            .show(ui, |ui| match tab {
                Panel::Viewport => {}
                Panel::Attributes => {
                    self.attributes
                        .show(ui, self.document, self.units, self.requests);
                }
                Panel::Objects => self.objects.show(ui, self.document, self.requests),
                Panel::Coordinates => coordinates_contents(ui, *self.units),
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
mod attribute_tests;
#[cfg(test)]
mod object_tests;
#[cfg(test)]
mod shape_picker_tests;
#[cfg(test)]
mod tests;
