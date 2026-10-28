//! The app window's contents: menu bar, palettes, docked panels, status
//! bar and the About box.

use crate::attributes::AttributesPanel;
use crate::autosave::{Autosaver, RecoveryQuestion};
use crate::components::{self, ComponentTracker};
use crate::coordinates::Interaction;
use crate::drawing::{DrawingCache, SceneSummary};
use crate::layout::{hide_viewport_tab, restore_or_default};
use crate::objects::{DocRequest, ObjectManager};
use crate::palette::{Mode, PaletteCommand, PaletteState, command_palette, mode_palette};
use crate::panel::Panel;
use crate::panels::about_contents;
use crate::project::ProjectFile;
use crate::selecting::StrokeMemory;
use crate::settings::{SETTINGS_SHORTCUT, SettingsWindow};
use crate::shape_picker::ShapePicker;
use crate::text::tr;
use crate::theme::{self, Accent};
use crate::tool_menu::ToolMenu;
use crate::transforming::Tool;
use crate::undo_memory::UndoMemory;
use crate::units::Units;
use crate::viewport::{ViewOptions, ViewportPanel, ViewportRenderer};
use egui::{Key, Ui};
use egui_dock::{DockArea, DockState};
use scene::{Command, Document, ObjectKind};
use selection::SelectRequest;

/// How quickly two clicks must follow each other to be a double-click, in
/// seconds: Windows' standard.
pub const DOUBLE_CLICK_SECONDS: f64 = 0.5;
pub(crate) use selection::VISIBLE_ONLY_HELP;
use std::path::PathBuf;
mod autosaving;
mod bars;
pub(crate) mod file_work;
mod files;
mod heavy_shape;
mod layouts;
mod levels;
mod mesh_menu;
mod preferences;
mod requests;
mod tabs;
mod updating;
mod uv_menu;
pub use bars::navigation_hint;
pub use file_work::FileWork;
pub use files::DISCARD_SHORTCUT;
use files::file_name;
pub(crate) use layouts::LayoutChoice;
pub(crate) use levels::STOP_EDITING_LEVEL;
pub use path_entry::FilePurpose;
use tabs::PanelViewer;
pub use updating::UpdateOffer;
pub(crate) use updating::{SKIP_HELP, SKIP_VERSION};
// The tests reach these through `super::`, as when they were defined or
// imported here.
#[cfg(test)]
use crate::autosave::AutosaveSettings;
#[cfg(test)]
pub(crate) use bars::{
    DEVELOPER_MENU, EDIT_LAYERS_ENTRY, LEVELS_ENTRY, MANUAL_ENTRY, SEARCH_ENTRY, SHOW_HIDE_MENU,
    UV_EDITOR_ENTRY,
};
#[cfg(test)]
use egui::Modifiers;
#[cfg(test)]
pub(crate) use mesh_menu::{
    MESH_BUILD_MENU, MESH_CUT_MENU, MESH_OBJECT_MENU, MESH_REMOVE_MENU, MESH_SHADING_MENU,
};
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
pub(crate) use uv_menu::{
    UV_EDITOR_UV_MENU, UV_LAY_OUT_MENU, UV_PIECES_MENU, UV_SEAMS_MENU, UV_SETTINGS_MENU,
    UV_TURN_MENU,
};

/// Facts about the program, shown in the window and the About box.
/// The app passes these in from its branding, so this crate never names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// Full product name, such as "Aventail 3D".
    pub name: String,
    /// Version, such as "0.0.1".
    pub version: String,
    /// The build number, when the release workflow made this build
    /// (decision 0169). Shown in Help → About, not in the title.
    pub build: Option<u32>,
    /// Where the user manual is read online (Help → User manual).
    pub manual_url: String,
}

impl AppInfo {
    /// The version with the build number, when there is one: "0.0.1
    /// (build 3)", for Help → About and the splash screen.
    pub fn full_version(&self) -> String {
        match self.build {
            Some(build) => crate::trf!("{} (build {})", self.version, build),
            None => self.version.clone(),
        }
    }
}

/// Something the user asked for that the program around the workbench
/// must carry out (the workbench never touches files or the window itself).
#[derive(Debug, Clone, PartialEq)]
pub enum WorkbenchAction {
    /// Close the window (unsaved changes have been dealt with).
    Quit,
    /// Save the document to `path`, or ask where first (Save As, or a
    /// project never saved) when `None`. Report back with
    /// [`Workbench::saved`], [`Workbench::save_failed`] or
    /// [`Workbench::save_cancelled`].
    Save {
        /// Where to save, if already known.
        path: Option<PathBuf>,
    },
    /// Open the project at `path`, or ask which first when `None`. Report
    /// back with [`Workbench::opened`] or [`Workbench::open_failed`].
    Open {
        /// Which project, if already known (File → Open recent).
        path: Option<PathBuf>,
    },
    /// Save a backup copy of `document` in the background, keeping the
    /// newest `keep` copies. Report back with [`Workbench::autosaved`] or
    /// [`Workbench::autosave_failed`].
    Autosave {
        /// A copy of the document as it is now.
        document: Box<Document>,
        /// How many copies to keep.
        keep: usize,
    },
    /// Bring back the backup copy `copy` from the backups `folder` left by
    /// a crash, for the project at `project` (if it had been saved). Report
    /// back with [`Workbench::recovered`] or [`Workbench::recovery_failed`];
    /// once restored, the folder can go.
    Restore {
        /// The leftover backups.
        folder: PathBuf,
        /// The copy chosen.
        copy: PathBuf,
        /// The project it belongs to.
        project: Option<PathBuf>,
    },
    /// Export the scene as `request` says, asking where first when it has
    /// no path. Report back with [`Workbench::exported`] or
    /// [`Workbench::export_failed`].
    Export(Box<ExportRequest>),
    /// Ask which folder of a game project to export into (decision 0076).
    /// Report back with [`Workbench::export_folder_chosen`].
    ChooseExportFolder {
        /// Where the folder picker starts, if anywhere.
        start: Option<PathBuf>,
    },
    /// Export into the project's game project folder as `request` says.
    /// Report back with [`Workbench::target_exported`].
    ExportToTarget(Box<TargetRequest>),
    /// Export each piece as its own file as `request` says, asking for a
    /// folder first when it has none (decision 0078). Report back with
    /// [`Workbench::kit_exported`].
    ExportKit(Box<KitRequest>),
    /// Import the file at `path`, or ask which first when `None`. Report
    /// back with [`Workbench::imported`] or [`Workbench::import_failed`].
    Import {
        /// Which file, if already known.
        path: Option<PathBuf>,
    },
    /// Import the image at `path` (or ask which first, when `None`) as a
    /// paint layer on the texture set of the material `material`
    /// (decision 0177). Report back with [`Workbench::import_paint_image`]
    /// or [`Workbench::import_failed`].
    ImportPaintImage {
        /// The material whose texture set gets the layer.
        material: String,
        /// Which image file, if already known.
        path: Option<PathBuf>,
    },
    /// Delete the backups `folder` left by a crash.
    DiscardBackups {
        /// The leftover backups.
        folder: PathBuf,
    },
    /// Install the update offered ([`Workbench::offer_update`], decision
    /// 0169): `now`, by closing the app (unsaved changes asked about as
    /// for any close), installing and opening it again; otherwise when the
    /// app is next closed.
    InstallUpdate {
        /// Restart now, rather than at the next close.
        now: bool,
    },
    /// Don't install the update offered, and don't offer that build again
    /// (Skip this version, decision 0173; [`Workbench::skipped_update`]).
    SkipUpdate,
}

/// The keys that switch to the Points, Edges and Polygons modes. Common in
/// 3D programs, and free here (Q is the Select tool, decision 0019); the
/// same key again goes back to the Model mode.
pub const MODE_KEYS: [(Key, Mode); 3] = [
    (Key::Num1, Mode::Points),
    (Key::Num2, Mode::Edges),
    (Key::Num3, Mode::Polygons),
];

/// Steps straight back to the Model mode from wherever editing has got to
/// ([decision 0057](../../../docs/decisions/0057-after-phase-4-testing.md)):
/// a Slit line is finished, a modelling tool is put away and the Points,
/// Edges or Polygons mode is left, so a click selects objects again.
pub(crate) const BACK_KEY: Key = Key::Tab;

/// The menus between Modes and View, in order: Select, Tools, Mesh and
/// UV, each filled from its own module.
const MIDDLE_MENUS: [&str; 4] = [
    crate::text!("Select"),
    crate::text!("Tools"),
    crate::text!("Mesh"),
    crate::text!("UV"),
];

/// The whole interface around the scene.
pub struct Workbench {
    info: AppInfo,
    dock: DockState<Panel>,
    /// The layouts not in use, each kept as it was left (decisions 0096
    /// and 0181).
    other_docks: layouts::KeptLayouts,
    /// Which layout `dock` is.
    layout_choice: LayoutChoice,
    /// The UV Editor panel (decision 0096).
    uv_editor: crate::uv_editor::UvEditor,
    /// The paint brush in Paint mode (decision 0179).
    brushing: brushing::Brushing,
    /// The Paint layers panel (decision 0181).
    paint_layers: crate::paint_layers::PaintLayersPanel,
    /// Whether the tool menu shows the UV ring: chosen when it opens, by
    /// whether the mouse is over the UV Editor (decision 0097).
    uv_menu: bool,
    /// How the UV tools behave (decision 0099).
    uv_settings: unwrapping::UvSettings,
    /// The standard layout still needs Coordinates fitted to its content,
    /// which can only happen once the panels' sizes are known.
    fit_pending: bool,
    /// Whether Coordinates has the height it was fitted to, so a change
    /// of window size fits it again.
    coordinates_fitted: bool,
    /// Coordinates' height on the first frame it was known (a saved band
    /// far taller than needed is slimmed then).
    coordinates_checked: Option<f32>,
    /// Coordinates' height last frame, to tell a drag from a resize.
    coordinates_last: Option<f32>,
    /// The window's size last frame.
    last_screen: Option<egui::Vec2>,
    mode: Mode,
    /// The scene being worked on, with its undo history and selection.
    document: Document,
    objects: ObjectManager,
    /// The Edit layers panel (decision 0091).
    layers: crate::layers::LayersPanel,
    attributes: AttributesPanel,
    /// The project's units (decision 0068), for every length shown or
    /// typed this frame: read from the document as the frame starts; a
    /// change made in the panels or Settings becomes an undoable step as
    /// it ends.
    units: Units,
    /// The units a new project starts with: a preference, remembered
    /// between sessions.
    new_units: Units,
    /// How much memory the undo history may use.
    undo_memory: UndoMemory,
    /// The Settings window.
    settings: SettingsWindow,
    /// Where the project is saved, whether it has unsaved changes, and
    /// the recent projects.
    file: ProjectFile,
    /// Actions to hand to the app on the next frame (what was waiting for
    /// a save to finish).
    queued: Vec<WorkbenchAction>,
    /// How many forgotten undo steps the status bar has already told
    /// about (it says so once, when forgetting starts).
    forgotten_told: bool,
    /// The `B` shape list.
    shape_picker: ShapePicker,
    /// The tool menu (E).
    tool_menu: ToolMenu,
    /// Command search, opened with Space (decision 0126).
    search: crate::command_search::CommandSearch,
    /// What the viewport draws, kept up to date with the document.
    drawing: DrawingCache,
    /// Polygons hidden from view (Shift + H, Ctrl + H; decision 0055).
    hidden: crate::drawing::Hidden,
    /// The objects copied for pasting (Ctrl + C, Alt + C; decision 0055).
    copied: Vec<scene::ObjectId>,
    /// The Points, Edges and Polygons modes: the element under the mouse
    /// and what is drawn over the edited objects.
    components: ComponentTracker,
    /// Selection commands from keys and the Select menu, carried out once
    /// the frame's drawing is known.
    select_requests: Vec<SelectRequest>,
    /// What a flipping brush stroke has flipped so far.
    stroke_memory: StrokeMemory,
    /// The tool in use: selecting, or a transform gizmo.
    tool: Tool,
    /// A change typed or dragged in the Coordinates panel, in progress.
    coordinates_session: Option<transform::CoordinatesSession>,
    /// A gizmo drag in progress.
    gizmo_session: Option<crate::transforming::Session>,
    /// A modelling tool's drag (extrude, inset) in progress.
    modelling_session: Option<modelling::ModellingSession>,
    /// A drag editing UVs on the model, if one is going on (decision 0103).
    uv_model_drag: Option<uv_on_model::UvModelDrag>,
    /// A loop cut being placed (decision 0047).
    loop_cut_session: Option<loop_cut::LoopCutSession>,
    /// A drag of the Attributes panel's Crease or Bevel weight field
    /// (decisions 0143 and 0149).
    crease_drag: Option<creases::CreaseDrag>,
    /// A slit's line being drawn (decision 0049).
    slit_session: Option<slit::SlitSession>,
    /// A curve being drawn with Draw curve (decision 0138).
    curve_session: Option<curve_drawing::CurveSession>,
    /// The mode last checked for selected curves, which have no edges or
    /// polygons of their own (decision 0138).
    curve_mode_noted: Mode,
    /// A Bridge or Weld drag in progress (decision 0051).
    pair_session: Option<joining::PairSession>,
    /// What the next click on an object is for, while one is awaited:
    /// Pivot to object (decision 0068), Snap to or Align (decision 0093).
    target_pick: Option<aligning::TargetUse>,
    /// The Align window, while open (decision 0093).
    align: Option<aligning::AlignSession>,
    /// The Align window's choices, for the next time.
    align_settings: aligning::AlignSettings,
    /// The quad menu (decision 0094).
    quad_menu: crate::quad_menu::QuadMenu,
    /// Whether the right button last went down while something was under
    /// way (so letting it go doesn't open the quad menu).
    right_press_busy: bool,
    /// The Export window and the last export (decision 0071).
    exporting: exporting::Exporting,
    /// The bake check window (decision 0077).
    baking: baking::Baking,
    /// The question asked before dissolving across a sharp corner.
    dissolve_question: Option<cleaning::DissolveQuestion>,
    /// The change the gizmo drag last showed, so an unchanged frame does
    /// nothing.
    gizmo_shown: Option<tools::Change>,
    /// How many transform changes have been made (each gets its own undo
    /// merge key).
    sessions: u64,
    /// The undo merge key of the kit grid being changed in the Create
    /// menu's Modular kit list, so a drag is one step (decision 0160).
    kit_key: u64,
    viewport: ViewportPanel,
    accent: Accent,
    about_open: bool,
    /// The window to type a path in when the system's file window can't
    /// open, while open.
    path_entry: Option<path_entry::PathEntry>,
    /// The keyboard shortcut viewer (Keys on the menu bar; decision 0056).
    shortcuts: shortcuts::ShortcutViewer,
    /// The keyboard guessed from the system language, drawn in the Keys
    /// window until one is picked there.
    system_keyboard: crate::KeyboardLayout,
    /// A message for the status bar, such as "Panel layout reset".
    note: Option<String>,
    /// A newer build offered, and what the last check found (decision
    /// 0169).
    updating: updating::Updating,
    /// Whether anything was selected last frame, to notice the selection
    /// becoming empty in an element mode (decision 0175).
    selected_before: bool,
    /// When the next backup copy is due.
    autosaver: Autosaver,
    /// Where backup copies go, to show in Settings.
    autosave_folder: Option<PathBuf>,
    /// The question asked after a crash.
    recovery: RecoveryQuestion,
}

impl Workbench {
    /// A workbench with the saved panel layout, if it is usable, or the
    /// standard one.
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        let had_saved = saved_layout.is_some();
        let (dock, note) = restore_or_default(saved_layout);
        // A saved layout keeps the user's own sizes; a new or replaced one
        // is fitted on its first frame. A saved band far taller than
        // Coordinates needs (the three-row band before decision 0126's
        // slim one) is slimmed once too: see `keep_coordinates_fitted`.
        let fit_pending = !had_saved || note.is_some();
        Self {
            info,
            dock,
            other_docks: layouts::KeptLayouts::default(),
            layout_choice: LayoutChoice::Standard,
            uv_editor: crate::uv_editor::UvEditor::default(),
            brushing: brushing::Brushing::default(),
            paint_layers: Default::default(),
            uv_menu: false,
            uv_settings: unwrapping::UvSettings::default(),
            fit_pending,
            coordinates_fitted: false,
            coordinates_checked: None,
            coordinates_last: None,
            last_screen: None,
            mode: Mode::default(),
            document: Document::new(),
            objects: ObjectManager::default(),
            layers: crate::layers::LayersPanel::default(),
            attributes: AttributesPanel::default(),
            units: Units::default(),
            new_units: Units::default(),
            undo_memory: UndoMemory::default(),
            forgotten_told: false,
            settings: SettingsWindow::default(),
            file: ProjectFile::default(),
            queued: Vec::new(),
            shape_picker: ShapePicker::default(),
            tool_menu: ToolMenu::default(),
            search: crate::command_search::CommandSearch::default(),
            drawing: DrawingCache::default(),
            hidden: crate::drawing::Hidden::default(),
            copied: Vec::new(),
            components: ComponentTracker::default(),
            select_requests: Vec::new(),
            stroke_memory: StrokeMemory::default(),
            tool: Tool::default(),
            gizmo_session: None,
            modelling_session: None,
            uv_model_drag: None,
            loop_cut_session: None,
            crease_drag: None,
            slit_session: None,
            curve_session: None,
            curve_mode_noted: Mode::Model,
            pair_session: None,
            target_pick: None,
            align: None,
            align_settings: aligning::AlignSettings::default(),
            quad_menu: crate::quad_menu::QuadMenu::default(),
            right_press_busy: false,
            exporting: exporting::Exporting::default(),
            baking: baking::Baking::default(),
            dissolve_question: None,
            gizmo_shown: None,
            coordinates_session: None,
            sessions: 0,
            kit_key: 0,
            viewport: ViewportPanel::default(),
            accent: Accent::default(),
            about_open: false,
            path_entry: None,
            shortcuts: shortcuts::ShortcutViewer::default(),
            system_keyboard: crate::KeyboardLayout::default(),
            note,
            updating: updating::Updating::default(),
            selected_before: false,
            autosaver: Autosaver::default(),
            autosave_folder: None,
            recovery: RecoveryQuestion::default(),
        }
    }

    /// The text for the window's title bar.
    pub fn window_title(&self) -> String {
        format!("{} {}", self.info.name, self.info.version)
    }

    /// The current mode (Model, Object, Points and so on).
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Puts a message in the status bar, for the app around the
    /// workbench (an update that couldn't be downloaded, decision 0173).
    pub fn set_note(&mut self, note: String) {
        self.note = Some(note);
    }

    /// The status bar's message, if any.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// Whether the About box is showing.
    pub fn is_about_open(&self) -> bool {
        self.about_open
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

    /// The Objects panel's state, for tests.
    #[cfg(test)]
    pub(crate) fn object_manager(&self) -> &ObjectManager {
        &self.objects
    }

    /// Whether a name (an object's or a layer's) is being typed, so keys
    /// go to the text box.
    pub(crate) fn renaming(&self) -> bool {
        self.objects.is_renaming() || self.layers.is_renaming()
    }

    /// The Edit layers panel, for tests.
    #[cfg(test)]
    pub(crate) fn layers_panel(&self) -> &crate::layers::LayersPanel {
        &self.layers
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

    // ------------------------------------------------- the project file ---

    /// Draws the whole window and returns what the user asked for.
    /// `renderer` draws the 3D viewport.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = Vec::new();
        // On a Mac, texts say Cmd for Ctrl (decision 0133).
        crate::text::use_mac_keys(ui.ctx().os() == egui::os::OperatingSystem::Mac);
        if theme::accent(ui.ctx()) != self.accent {
            theme::set_accent(ui.ctx(), self.accent);
        }
        // Double-clicks as forgiving as Windows' standard (egui's is 0.3 s).
        if ui.ctx().options(|o| o.input_options.max_double_click_delay) != DOUBLE_CLICK_SECONDS {
            ui.ctx().options_mut(|o| {
                o.input_options.max_double_click_delay = DOUBLE_CLICK_SECONDS;
            });
        }
        actions.append(&mut self.queued);
        // Remember the keys held for a press, for its click a frame later.
        let _ = crate::selecting::click_modifiers(ui.ctx());
        self.units = self.units();
        let units_shown = self.units;
        if self.file.asking.is_none() {
            self.file_keys(ui, &mut actions);
        }
        if ui.input_mut(|i| i.consume_shortcut(&SETTINGS_SHORTCUT)) {
            self.settings.open = !self.settings.open;
        }
        let mut requests = Vec::new();
        // The shape list goes first: while it is open, Esc closes it
        // rather than deselecting.
        // While "Save changes?" (or the dissolve question) is asked, keys
        // answer it, not the scene.
        if self.file.asking.is_none() && self.dissolve_question.is_none() {
            // A window open (Settings, Keys, Export…) takes the keys too: the
            // tool menu and shape list don't open over it (decision 0126).
            let typing = ui.ctx().memory(|m| m.focused().is_some())
                || self.objects.is_renaming()
                || self.window_open();
            if let Some(shape) = self.shape_picker.keys(ui, typing) {
                self.create_shape(shape, &mut requests);
            }
            // The tool menu next: while it is open, Esc closes it, and
            // Enter or Space use its tool. Over the UV Editor it holds the
            // UV tools.
            if !self.tool_menu.is_open() {
                self.uv_menu = self.uv_editor.hovered(ui.ctx())
                    && crate::uv_editor::element_kind(self.mode).is_some();
            }
            let entries = self.tool_menu_entries();
            self.tool_menu_keys(ui, typing, &entries, &mut requests);
            self.search_keys(ui, typing);
            self.showing_keys(ui);
            self.copying_keys(ui);
            // Before the document's keys: in Paint mode Esc leaves the
            // mode instead of deselecting.
            self.paint_keys(ui);
            self.document_keys(ui, &mut requests);
            self.mode_keys(ui);
            self.select_keys(ui);
            self.transform_keys(ui);
            self.modelling_keys(ui);
        }

        egui::Panel::top("menu_bar")
            .frame(theme::menu_bar_frame())
            .show(ui, |ui| {
                egui::MenuBar::new()
                    .config(crate::dialog::menu_config())
                    .ui(ui, |ui| self.menu_bar(ui, &mut actions, &mut requests));
            });
        let palette_state = PaletteState {
            undo: self.document.undo_label().map(str::to_string),
            redo: self.document.redo_label().map(str::to_string),
            select_tool: self.viewport.select_tool(),
            tool: self.tool,
            locked: self.viewport.options().locked,
            axes: self.viewport.options().axes,
            snapping: self.viewport.options().snap.on,
            symmetry: self.selected_mirror().is_some(),
            edit_pivot: self.mode == Mode::Object,
        };
        // The Symmetry menu changes a copy of the selected objects' mirror
        // settings (or those kept for the next mirror), kept if it
        // changed.
        let symmetry_before = self
            .selected_mirror()
            .map_or(self.viewport.options().symmetry, crate::Symmetry::of);
        let mut symmetry = symmetry_before;
        let units = self.units;
        egui::Panel::top("command_palette")
            .frame(theme::palette_frame())
            .exact_size(40.0)
            .show(ui, |ui| {
                if let Some(command) = command_palette(ui, &palette_state, &mut symmetry, units) {
                    requests.push(match command {
                        PaletteCommand::SetSelectTool(tool) => {
                            self.set_tool(Tool::Select);
                            self.set_select_tool(tool);
                            return;
                        }
                        PaletteCommand::NextSelectTool => {
                            if self.tool == Tool::Select {
                                self.next_select_tool();
                            } else {
                                self.set_tool(Tool::Select);
                            }
                            return;
                        }
                        PaletteCommand::Tool(tool) => {
                            self.set_tool(tool);
                            return;
                        }
                        PaletteCommand::Lock(axis) => {
                            self.toggle_lock(axis);
                            return;
                        }
                        PaletteCommand::ToggleAxes => {
                            self.toggle_axes();
                            return;
                        }
                        PaletteCommand::ToggleSnapping => {
                            self.toggle_snapping();
                            return;
                        }
                        PaletteCommand::ToggleSymmetry => {
                            self.toggle_symmetry();
                            return;
                        }
                        PaletteCommand::ToggleEditPivot => {
                            self.toggle_edit_pivot();
                            return;
                        }
                        PaletteCommand::ApplyMirror => {
                            self.apply_mirror();
                            return;
                        }
                        PaletteCommand::Search => {
                            self.search.open();
                            return;
                        }
                        PaletteCommand::Undo => DocRequest::Undo,
                        PaletteCommand::Redo => DocRequest::Redo,
                        PaletteCommand::CreateGroup => {
                            DocRequest::Run(Command::Create(ObjectKind::Group))
                        }
                        PaletteCommand::CreateShape(kind) => self.new_primitive(kind),
                    });
                }
            });
        if symmetry != symmetry_before {
            self.change_symmetry(symmetry);
        }
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
                match mode_palette(ui, self.mode, self.viewport.options_mut()) {
                    // With a modelling tool in use, picking a mode puts it
                    // away, as the mode keys do.
                    Some(crate::palette::ModePick::Mode(mode)) if self.tool.has_settings() => {
                        self.press_mode_key(mode);
                    }
                    Some(crate::palette::ModePick::Mode(mode)) => self.mode = mode,
                    Some(crate::palette::ModePick::Isolate) => {
                        self.hide(showing::Hiding::Others);
                    }
                    Some(crate::palette::ModePick::EditPivot) => self.toggle_edit_pivot(),
                    None => {}
                }
            });

        self.note_curve_modes();
        let editing = components::kind_for(self.mode);
        let mut summary: SceneSummary = self
            .drawing
            .update(&self.document, editing, &self.hidden)
            .clone();
        if self.drawing.busy() || self.components.busy() {
            // A heavy boolean or modifier is being worked out: look again
            // shortly.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        summary.overlay = self.components.overlay();
        self.components.freeze(
            self.gizmo_session.is_some()
                || self.coordinates_session.is_some()
                || self.modelling_session.is_some()
                || self.loop_cut_session.is_some(),
        );
        self.components.prepare(&summary.drawing);
        self.update_paint_mode(&summary.drawing);
        self.place_gizmo(&summary.drawing);
        self.place_drag_tool();
        let coordinates = self.coordinates_view(&summary.drawing);
        let mut coordinates_done = Interaction::default();
        self.attributes.tool = self.tool;
        self.attributes.track_last_edit(&self.document);
        self.attributes.polygons_selected = self.polygons_selected_for_attributes();
        self.attributes.selected_edges = self.edge_values_for_attributes();
        self.attributes.curve.points_mode = self.mode == Mode::Points;
        self.note_level_banner();
        self.note_right_press(ui.ctx());
        // Over the UV Editor, H and Z frame there (keys stay the same,
        // decision 0095).
        self.uv_editor.take_keys(ui.ctx());
        hide_viewport_tab(&mut self.dock);
        let mut modifier_pick = None;
        // The UV Editor's own UV menu button (issue #115): its menu is the
        // main UV menu, drawn here where the workbench can fill it.
        let mut uv_menu_button: Option<egui::Response> = None;
        let view_options = self.viewport.options();
        // What the viewport has under the mouse, for the UV Editor to
        // light up (decision 0103).
        let viewport_hover = self.viewport.pointer().and(self.components.hovered());
        // What explaining choices in the UV Editor needs (decision 0116).
        let explain_suggest = self.suggest_options();
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        // No modifiers palette in the UV mode: nothing on it works on UVs
        // (decision 0176).
        let palette_host = (!matches!(self.mode, Mode::Uv | Mode::Paint))
            .then(|| crate::layout::modifiers_palette_host(&self.dock));
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
                        layers: &mut self.layers,
                        modifier_pick: &mut modifier_pick,
                        uv_editor: &mut self.uv_editor,
                        paint_layers: &mut self.paint_layers,
                        note: &mut self.note,
                        mode: self.mode,
                        navigation: view_options.navigation,
                        select_keys: view_options.select_keys,
                        tool: self.tool,
                        snap: view_options.snap,
                        surface: view_options.surface,
                        high_contrast: view_options.high_contrast,
                        explain: crate::uv_editor::explain::ExplainContext {
                            suggest: explain_suggest,
                        },
                        texture_groups: crate::uv_editor::checking::TextureGroups {
                            per_material: self.uv_settings.per_material,
                            per_object: self.uv_settings.per_object,
                        },
                        viewport_hover,
                        attributes: &mut self.attributes,
                        units: &mut self.units,
                        requests: &mut requests,
                        coordinates,
                        coordinates_done: &mut coordinates_done,
                        uv_menu_button: &mut uv_menu_button,
                        uv_settings: &mut self.uv_settings,
                        palette_host,
                    },
                );
        });
        if let Some(button) = uv_menu_button {
            egui::Popup::menu(&button)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_min_width(crate::dialog::MENU_WIDTH);
                    self.uv_menu_entries(ui, &mut requests);
                });
        }
        // A viewport menu's "All settings…" (decision 0126).
        if self.viewport.take_open_settings() {
            self.settings.open = true;
        }
        match modifier_pick {
            Some(crate::layers::ModifierPick::AddLayer) => self.add_edit_layer(&mut requests),
            Some(crate::layers::ModifierPick::Boolean(kind)) => {
                self.run_boolean(kind, &mut requests);
            }
            Some(crate::layers::ModifierPick::Modifier(crate::commands::CommandId::GameBevel)) => {
                self.run_game_bevel();
            }
            Some(crate::layers::ModifierPick::Modifier(command)) => {
                self.run_modifier(command, &mut requests);
            }
            None => {}
        }
        if let Some(slot) = self.attributes.assign.take() {
            self.assign_material(slot);
        }
        if let Some(material) = self.attributes.import_image.take() {
            actions.push(WorkbenchAction::ImportPaintImage {
                material,
                path: None,
            });
        }
        if std::mem::take(&mut self.attributes.show_levels) {
            crate::layout::show_levels(&mut self.dock);
        }
        if self.viewport.take_banner_click() {
            self.stop_editing_level(&mut requests);
        }
        if let Some((what, value, gesture)) = self.attributes.edge_value_asked.take() {
            self.set_edge_value_from_field(what, value, gesture);
        }
        if let Some(command) = self.attributes.curve.asked.take() {
            self.run_command(command, ui.ctx(), &mut requests);
        }
        // What is under the mouse now that the viewport knows where it is;
        // when that changes, draw again so the highlight follows at once.
        self.carry_out_selection(&summary.drawing);
        self.carry_out_brush(&summary.drawing);
        self.leave_empty_element_mode();
        self.carry_out_gizmo(ui, &summary.drawing);
        let keys = ui.input(|i| i.modifiers);
        self.carry_out_tool_drag(&summary.drawing, keys);
        self.show_loop_cut_preview(&summary.drawing, keys.command);
        self.show_slit(&summary.drawing);
        self.show_curve(&summary.drawing);
        self.carry_out_coordinates(coordinates_done, &summary.drawing);
        let shown = summary.overlay.version;
        let options = self.viewport.options();
        self.components
            .set_outside_hover(self.uv_editor.under_mouse(ui.ctx()));
        let overlay = self.components.update(
            self.mode,
            &summary.drawing,
            &self.document,
            self.viewport.pointer(),
            crate::components::Guides {
                hard_edges: !options.hide_hard_edges,
                normals: options.show_normals,
                seams: options.show_seams || self.mode == Mode::Uv,
                collision: options.show_collision,
                triangles: options.show_triangles,
            },
        );
        if overlay.version != shown {
            ui.ctx().request_repaint();
        }
        // Levels comes back beside Attributes when a shape with levels is
        // selected (decision 0165).
        self.keep_levels_panel();
        // Edit layers stays out of the way until an object with layers is
        // selected (decision 0126).
        if self.dock.find_tab(&Panel::EditLayers).is_none() {
            let scene = self.document.scene();
            let layered = self.document.selection().iter().any(|id| {
                scene
                    .object(*id)
                    .is_some_and(|o| o.stack.layers().len() > 1)
            });
            if layered {
                crate::layout::add_beside_objects(&mut self.dock, Panel::EditLayers);
            }
        }
        self.keep_coordinates_fitted(ui.ctx());

        if let Some(shape) = self.shape_picker.show(ui.ctx()) {
            self.create_shape(shape, &mut requests);
        }
        let entries = self.tool_menu_entries();
        self.tool_menu_show(ui.ctx(), &entries, &mut requests);
        self.open_quad_menu();
        self.quad_menu_show(ui.ctx(), &mut requests);
        self.search_window(ui.ctx(), &mut actions, &mut requests);
        if !requests.is_empty() {
            self.carry_out(requests);
            ui.ctx().request_repaint();
        }

        self.shortcut_window(ui.ctx());
        if self.about_open {
            let mut done = false;
            let outcome =
                crate::dialog::Dialog::new(crate::trf!("About {}", self.info.name), 460.0)
                    .id("about")
                    .show(ui.ctx(), |d| {
                        d.header(true, |_| {});
                        d.body(|ui| about_contents(ui, &self.info));
                        d.footer(None, |ui| {
                            done = crate::dialog::main_button(ui, tr("Close")).clicked();
                        });
                    });
            let (enter, escape) = crate::dialog::enter_or_escape(ui.ctx());
            if outcome.closed || done || enter || escape {
                self.about_open = false;
            }
        }

        // The bake check first: opened from the Export window, it sits on
        // top of it, so Esc closes it first.
        self.settings_window(ui.ctx());
        self.bake_window(ui.ctx());
        self.export_window(ui.ctx(), &mut actions);
        self.keep_units(units_shown);
        self.align_window(ui.ctx());
        // While Pivot to object, Snap to or Align waits for its click, the
        // pointer says so.
        if self.target_pick.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        self.dissolve_question_window(ui.ctx());
        self.update_window(ui.ctx(), &mut actions);
        self.unsaved_question(ui.ctx(), &mut actions);
        self.path_window(ui.ctx(), &mut actions);
        self.autosave_and_recovery(ui.ctx(), &mut actions);
        // One save or export at a time (decision 0163).
        self.hold_file_actions(&mut actions);

        actions
    }

    /// Whether one of the app's windows is open (Export, Check for baking,
    /// Settings, the keys, About): Esc closes it rather than selecting
    /// nothing.
    fn window_open(&self) -> bool {
        self.update_window_shown()
            || self.exporting.open
            || self.baking.open
            || self.settings.open
            || self.shortcuts.open
            || self.about_open
            || self.align.is_some()
            || self.quad_menu.is_open()
            || self.search.open
            || self.path_entry.is_some()
    }
}

/// The request that creates a primitive of `kind` with standard settings.
#[cfg(test)]
mod attribute_tests;
mod automation;
#[cfg(test)]
mod automation_tests;
mod baking;
#[cfg(test)]
mod baking_tests;
#[cfg(test)]
mod basics_tests;
#[cfg(test)]
mod boolean_tests;
mod brushing;
#[cfg(test)]
mod brushing_tests;
mod cleaning;
#[cfg(test)]
mod cleaning_tests;
mod command;
mod copying;
mod creases;
#[cfg(test)]
mod creases_tests;
pub(crate) mod exporting;
#[cfg(test)]
mod exporting_tests;
#[cfg(test)]
mod extrude_tests;
#[cfg(test)]
mod file_work_tests;
mod game_bevel;
#[cfg(test)]
mod game_bevel_tests;
mod hardness;
#[cfg(test)]
mod hardness_tests;
mod joining;
#[cfg(test)]
mod joining_tests;
mod kit;
#[cfg(test)]
mod kit_tests;
#[cfg(test)]
mod last_edit_tests;
mod loop_cut;
#[cfg(test)]
mod loop_cut_tests;
#[cfg(test)]
mod material_tests;
mod materials;
#[cfg(test)]
mod paint_layers_tests;
mod painting;
#[cfg(test)]
mod painting_tests;
#[cfg(test)]
mod palette_label_tests;
#[cfg(test)]
mod palette_place_tests;
#[cfg(test)]
mod reference_images;
#[cfg(test)]
mod seam_preference_tests;
mod seam_preferences;
mod seams;
#[cfg(test)]
mod seams_tests;
mod unwrapping;
#[cfg(test)]
mod uv_brushing_tests;
mod uv_on_model;
#[cfg(test)]
mod uv_on_model_tests;
pub use unwrapping::{UnwrapWay, UvSettings};
#[cfg(test)]
mod checking_tests;
#[cfg(test)]
mod connected_tests;
#[cfg(test)]
mod density_tests;
#[cfg(test)]
mod pack_tests;
mod piece_settings;
#[cfg(test)]
mod piece_settings_tests;
#[cfg(test)]
mod pinning_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod trim_tests;
#[cfg(test)]
mod unwrapping_tests;
pub use exporting::{
    ExportChoice, ExportRequest, ExportSettings, KitOutcome, KitRequest, TargetOutcome,
    TargetRequest,
};
mod aligning;
#[cfg(test)]
mod aligning_tests;
#[cfg(test)]
mod array_tests;
#[cfg(test)]
mod axis_deform_tests;
#[cfg(test)]
mod bevel_upgrade_tests;
mod cage_editing;
#[cfg(test)]
mod collision_tests;
mod curve_commands;
mod curve_drawing;
mod curve_editing;
#[cfg(test)]
mod curve_tests;
#[cfg(test)]
mod decimate_tests;
#[cfg(test)]
mod displace_tests;
#[cfg(test)]
mod id_clash_tests;
#[cfg(test)]
mod lattice_tests;
#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod live_bevel_tests;
#[cfg(test)]
mod lod_tests;
mod modelling;
#[cfg(test)]
mod modelling_tests;
#[cfg(test)]
mod modifier_list_tests;
#[cfg(test)]
mod modifier_tests;
mod parts;
mod path_entry;
mod pivots;
#[cfg(test)]
mod pivots_tests;
mod quad;
#[cfg(test)]
mod quad_tests;
#[cfg(test)]
mod quadify_tests;
mod quadifying;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod scatter_tests;
mod searching;
mod selection;
#[cfg(test)]
mod shell_tests;
mod shortcuts;
pub(crate) mod showing;
mod slit;
#[cfg(test)]
mod slit_tests;
#[cfg(test)]
mod spline_tests;
mod status_line;
#[cfg(test)]
mod struts_tests;
#[cfg(test)]
mod symmetry_tests;
mod transform;
#[cfg(test)]
mod triangulate_tests;
#[cfg(test)]
mod updating_tests;
#[cfg(test)]
mod uv_editor_tests;
#[cfg(test)]
mod uv_hand_tests;
#[cfg(test)]
mod weighted_tests;
mod welding_distance;
#[cfg(test)]
mod welding_distance_tests;

#[cfg(test)]
mod autosave_tests;
#[cfg(test)]
mod heavy_check;
#[cfg(test)]
mod mode_tests;
#[cfg(test)]
mod object_tests;
#[cfg(test)]
mod project_tests;
#[cfg(test)]
mod random_action_tests;
#[cfg(test)]
mod select_tests;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod shape_picker_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_menu_tests;
#[cfg(test)]
mod transform_tests;
#[cfg(test)]
mod workflow_tests;
#[cfg(test)]
mod wrap_tests;
