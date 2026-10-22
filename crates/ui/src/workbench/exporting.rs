//! The workbench's part in exporting and importing
//! ([decision 0071](../../../../docs/decisions/0071-obj-export-and-import.md)):
//! File → Import…, Export… and Export again, and the Export window. The
//! files themselves are written and read by the app (through `fileio`),
//! which reports back.

use super::{Workbench, WorkbenchAction, file_name};
use crate::dialog::{self, Badge, Dialog, Section};
use crate::theme;
use egui::{RichText, Ui};
use fileio::engine::Engine;
use fileio::export::{
    ExportFormat, ExportOptions, ExportPreset, ExportSummary, ExportWhat, FileUnits, UpAxis,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The Export window's choices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportChoice {
    /// The preset the settings below follow, if any (decision 0075).
    pub preset: Option<ExportPreset>,
    /// The kind of file.
    pub format: ExportFormat,
    /// Everything shown, or the selection.
    pub what: ExportWhat,
    /// The file's units.
    pub units: FileUnits,
    /// The file's up axis.
    pub up: UpAxis,
    /// Whether polygons are cut into triangles.
    pub triangulate: bool,
}

impl Default for ExportChoice {
    /// Godot's settings, everything shown.
    fn default() -> Self {
        Self::default_with(ExportWhat::Everything).with_preset(ExportPreset::Godot)
    }
}

impl ExportChoice {
    fn default_with(what: ExportWhat) -> Self {
        Self {
            preset: None,
            format: ExportFormat::default(),
            what,
            units: FileUnits::default(),
            up: UpAxis::default(),
            triangulate: false,
        }
    }

    /// These choices with `preset`'s settings (what is exported stays).
    pub fn with_preset(self, preset: ExportPreset) -> Self {
        let settings = preset.settings();
        Self {
            preset: Some(preset),
            format: settings.format,
            units: settings.units,
            up: settings.up,
            triangulate: settings.triangulate,
            what: self.what,
        }
    }

    /// Whether these choices write the file `preset` would (for glTF, the
    /// format alone decides units, axes and triangles).
    pub fn follows(&self, preset: ExportPreset) -> bool {
        let options = |format: ExportFormat, units, up, triangulate| {
            format.options(&ExportOptions {
                units,
                up,
                triangulate,
                ..ExportOptions::default()
            })
        };
        let settings = preset.settings();
        self.format == settings.format
            && options(self.format, self.units, self.up, self.triangulate)
                == options(
                    settings.format,
                    settings.units,
                    settings.up,
                    settings.triangulate,
                )
    }
}

/// The export settings the app remembers between projects and launches
/// ([decision 0122](../../../../docs/decisions/0122-solis-preset-and-remembered-export-settings.md)):
/// the last export's Made for and file settings, so a new project exports
/// the same way without setting it up again. What goes out (everything
/// or the selection) isn't remembered: it belongs to the moment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    /// The preset, if any.
    pub preset: Option<ExportPreset>,
    /// The kind of file (used without a preset).
    pub format: ExportFormat,
    /// The file's units (used without a preset).
    pub units: FileUnits,
    /// The file's up axis (used without a preset).
    pub up: UpAxis,
    /// Whether polygons are cut into triangles (used without a preset).
    pub triangulate: bool,
}

impl Default for ExportSettings {
    /// Godot's, as on the first launch.
    fn default() -> Self {
        Self::of(ExportChoice::default())
    }
}

impl ExportSettings {
    /// The settings of `choice`.
    pub fn of(choice: ExportChoice) -> Self {
        Self {
            preset: choice.preset,
            format: choice.format,
            units: choice.units,
            up: choice.up,
            triangulate: choice.triangulate,
        }
    }

    /// The Export window's choices with these settings, everything shown.
    /// A preset's own settings win over the rest (a damaged settings file
    /// can hold anything).
    pub fn choice(self) -> ExportChoice {
        let custom = ExportChoice {
            preset: None,
            format: self.format,
            what: ExportWhat::Everything,
            units: self.units,
            up: self.up,
            triangulate: self.triangulate,
        };
        match self.preset {
            Some(preset) => custom.with_preset(preset),
            None => custom,
        }
    }
}

/// An export into the project's game project folder the app is asked to
/// make (decision 0076).
#[derive(Debug, Clone, PartialEq)]
pub struct TargetRequest {
    /// The folder.
    pub folder: PathBuf,
    /// The file's name, without its extension: the project's name.
    pub name: String,
    /// The settings: the engine's preset.
    pub preset: ExportPreset,
    /// What is exported, with the shading seen on screen.
    pub options: ExportOptions,
}

/// A kit export the app is asked to make (decision 0078): each piece as
/// its own file.
#[derive(Debug, Clone, PartialEq)]
pub struct KitRequest {
    /// The pieces: the selected objects (not those inside another).
    pub pieces: Vec<scene::ObjectId>,
    /// The kind of file.
    pub format: ExportFormat,
    /// How (with the shading seen on screen).
    pub options: ExportOptions,
    /// Where: the game project folder, or ask when `None`.
    pub folder: Option<PathBuf>,
}

/// How a kit export went: each piece's name and its file and what it
/// held, or why not; or why nothing was exported.
pub type KitOutcome = Result<Vec<(String, TargetOutcome)>, String>;

/// How an export into the game project folder went: the file written and
/// what it held, or why not.
pub type TargetOutcome = Result<(PathBuf, ExportSummary), String>;

/// "1 piece", "3 pieces".
fn counted_pieces(n: usize) -> String {
    if n == 1 {
        "1 piece".to_string()
    } else {
        format!("{n} pieces")
    }
}

/// The status bar's words for `outcome`.
pub(crate) fn target_note(outcome: &TargetOutcome) -> String {
    match outcome {
        Ok((path, summary)) => {
            let place = path.parent().map(file_name).unwrap_or_default();
            format!(
                "Exported {} ({} triangles) into {place}.",
                file_name(path),
                summary.triangles
            )
        }
        Err(why) => format!("Couldn't export into the game project: {why}."),
    }
}

/// The Export window and the last export.
#[derive(Debug, Clone, Default)]
pub(crate) struct Exporting {
    /// Whether the Export window is showing.
    pub(crate) open: bool,
    /// Its choices.
    pub(crate) choice: ExportChoice,
    /// The last export: where, and how (for Export again).
    pub(crate) last: Option<(PathBuf, ExportChoice)>,
    /// The last export's settings, remembered by the app (decision 0122).
    pub(crate) remembered: ExportSettings,
    /// The engine the game project folder belongs to, and that project's
    /// own folder, worked out for the folder named (only when it changes).
    detected: Option<(String, Option<(Engine, PathBuf)>)>,
    /// The section the window shows.
    pub(crate) section: ExportSection,
    /// What the export would hold, and what it was worked out for.
    summary: Option<(SummaryKey, ExportSummary)>,
}

/// The Export window's sections (decision 0087). Materials, Mesh, Rig and
/// Animations join them as those features arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ExportSection {
    /// Made for, what goes out and the file's settings.
    #[default]
    General,
    /// The game project folder: exports into it, and on every save.
    GameProject,
}

/// What an export summary was worked out for.
#[derive(Debug, Clone, PartialEq)]
struct SummaryKey {
    revision: u64,
    selection: Vec<scene::ObjectId>,
    what: ExportWhat,
}

/// The Export window's title.
pub(crate) const EXPORT_TITLE: &str = "Export";

/// The note at the foot of the Export window's side list.
const SECTIONS_NOTE: &str = "Only sections with something to set are listed. Materials, Mesh, \
                             Rig and Animations arrive with their features.";

/// An export the app is asked to make.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportRequest {
    /// The preset it follows, if any.
    pub preset: Option<ExportPreset>,
    /// The kind of file.
    pub format: ExportFormat,
    /// How the scene is exported (with the shading seen on screen).
    pub options: ExportOptions,
    /// Where, or ask first when `None`.
    pub path: Option<PathBuf>,
    /// The file name to offer when asking: the project's name with the
    /// format's extension.
    pub suggested_name: String,
}

impl Workbench {
    /// File → Import…, Export… and Export again.
    pub(super) fn export_entries(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if ui
            .button("Import…")
            .on_hover_text(
                "Bring shapes in from an OBJ file, each object as a plain mesh, added to the \
                 scene",
            )
            .clicked()
        {
            actions.push(WorkbenchAction::Import { path: None });
            ui.close();
        }
        if ui
            .button("Export…")
            .on_hover_text(
                "Write the scene (or the selection) to a file for a game engine or Substance \
                 Painter: pick the kind of file and how",
            )
            .clicked()
        {
            self.exporting.open = true;
            ui.close();
        }
        let again = self.exporting.last.clone();
        let response = ui
            .add_enabled(again.is_some(), egui::Button::new("Export again"))
            .on_disabled_hover_text("Export once first; this then repeats it");
        let response = match &again {
            Some((path, _)) => {
                response.on_hover_text(format!("Export to {} again, the same way", path.display()))
            }
            None => response,
        };
        if response.clicked()
            && let Some((path, choice)) = again
        {
            actions.push(self.export_action(choice, Some(path)));
            ui.close();
        }
        if ui
            .button("Export kit…")
            .on_hover_text(
                "Export each selected object (with what's inside it) as a file of its own, \
                 named after it, its pivot at the file's origin: modular pieces for a game \
                 engine. Uses the Export window's settings",
            )
            .clicked()
        {
            match self.kit_request() {
                Some(request) => actions.push(WorkbenchAction::ExportKit(Box::new(request))),
                None => {
                    self.note = Some("Export kit: select the pieces to export first.".into());
                }
            }
            ui.close();
        }
        let target = self.target_request(None);
        let response = ui
            .add_enabled(
                target.is_some(),
                egui::Button::new("Export to game project"),
            )
            .on_disabled_hover_text(
                "Choose a game project folder in the Export window first; this then exports \
                 straight into it",
            );
        let response = match &target {
            Some(request) => response.on_hover_text(format!(
                "Export into {} now, named after the project",
                request.folder.display()
            )),
            None => response,
        };
        if response.clicked()
            && let Some(request) = target
        {
            actions.push(WorkbenchAction::ExportToTarget(Box::new(request)));
            ui.close();
        }
    }

    /// The engine the game project folder belongs to, if any, and that
    /// project's folder (worked out again only when the folder changes).
    pub(crate) fn detected_engine(&mut self) -> Option<(Engine, PathBuf)> {
        let folder = self.document.scene().export_target()?.folder.clone();
        match &self.exporting.detected {
            Some((known, found)) if *known == folder => found.clone(),
            _ => {
                let found = fileio::engine::detect(Path::new(&folder));
                self.exporting.detected = Some((folder, found.clone()));
                found
            }
        }
    }

    /// The shading exports use: each object's own smoothing, as the
    /// viewport shows it (decision 0126).
    fn export_hardness(&self) -> Option<mesh::Hardness> {
        None
    }

    /// The preset for a game project folder the app can't recognise (a
    /// project of another engine): the Export window's Made for when it is
    /// a game engine, otherwise Godot's.
    pub(crate) fn fallback_preset(&self) -> ExportPreset {
        self.exporting
            .choice
            .preset
            .filter(|p| p.is_engine())
            .unwrap_or(ExportPreset::Godot)
    }

    /// The export settings the app remembers (decision 0122).
    pub fn export_settings(&self) -> ExportSettings {
        self.exporting.remembered
    }

    /// The workbench with remembered export settings: the Export window
    /// starts with them.
    #[must_use]
    pub fn with_export_settings(mut self, settings: ExportSettings) -> Self {
        self.exporting.remembered = settings;
        self.exporting.choice = settings.choice();
        self
    }

    /// The export into the game project folder, if the project has one:
    /// everything shown, with the engine's preset (when the folder isn't
    /// in a project the app can recognise, Made for's engine, or Godot's),
    /// named after the project saved
    /// at `saving_to` (or the project's own file, or "Untitled").
    pub fn target_request(&mut self, saving_to: Option<&Path>) -> Option<TargetRequest> {
        let folder = PathBuf::from(&self.document.scene().export_target()?.folder);
        let preset = self
            .detected_engine()
            .map_or(self.fallback_preset(), |(engine, _)| engine.preset());
        let name = saving_to
            .or(self.file.path.as_deref())
            .and_then(Path::file_stem)
            .map_or_else(
                || "Untitled".to_string(),
                |s| s.to_string_lossy().into_owned(),
            );
        Some(TargetRequest {
            folder,
            name,
            preset,
            options: ExportOptions {
                what: ExportWhat::Everything,
                hardness: self.export_hardness(),
                ..ExportOptions::default()
            },
        })
    }

    /// The export to make after saving to `path`, if the project exports
    /// into its game project folder on every save.
    pub fn save_export_request(&mut self, path: &Path) -> Option<TargetRequest> {
        if !self.document.scene().export_target()?.on_save {
            return None;
        }
        self.target_request(Some(path))
    }

    /// The kit export of the selected objects (none inside another), with
    /// the Export window's settings, into the game project folder if the
    /// project has one; `None` with nothing selected.
    pub fn kit_request(&mut self) -> Option<KitRequest> {
        let selection = self.document.selection();
        let scene = self.document.scene();
        let pieces: Vec<scene::ObjectId> = selection
            .iter()
            .copied()
            .filter(|id| {
                !selection
                    .iter()
                    .any(|s| s != id && scene.is_inside(*id, *s))
            })
            .collect();
        if pieces.is_empty() {
            return None;
        }
        let folder = scene.export_target().map(|t| PathBuf::from(&t.folder));
        let choice = self.exporting.choice;
        self.exporting.remembered = ExportSettings::of(choice);
        let WorkbenchAction::Export(request) = self.export_action(choice, None) else {
            return None;
        };
        Some(KitRequest {
            pieces,
            format: request.format,
            options: ExportOptions {
                what: ExportWhat::Everything,
                ..request.options
            },
            folder,
        })
    }

    /// The app made a kit export: says how it went.
    pub fn kit_exported(&mut self, outcome: KitOutcome) {
        self.note = Some(match outcome {
            Err(why) => format!("Couldn't export the kit: {why}."),
            Ok(pieces) => {
                let done: Vec<&(String, TargetOutcome)> =
                    pieces.iter().filter(|(_, r)| r.is_ok()).collect();
                let place = done
                    .first()
                    .and_then(|(_, r)| r.as_ref().ok())
                    .and_then(|(p, _)| p.parent())
                    .map(file_name)
                    .unwrap_or_default();
                let mut note = match done.len() {
                    0 => "No pieces exported.".to_string(),
                    n if n == pieces.len() => {
                        format!("Exported {} into {place}.", counted_pieces(n))
                    }
                    n => format!(
                        "Exported {n} of {} into {place}.",
                        counted_pieces(pieces.len())
                    ),
                };
                for (name, result) in &pieces {
                    if let Err(why) = result {
                        note.push_str(&format!(" {name}: {why}."));
                    }
                }
                note
            }
        });
    }

    /// The app exported into the game project folder (File → Export to
    /// game project): says how it went.
    pub fn target_exported(&mut self, outcome: TargetOutcome) {
        self.note = Some(target_note(&outcome));
    }

    /// The user chose `folder` as the project's game project folder:
    /// exports go there, and on every save. The Export window's Made for
    /// follows the folder's engine.
    pub fn export_folder_chosen(&mut self, folder: PathBuf) {
        let target = scene::ExportTarget {
            folder: folder.display().to_string(),
            on_save: true,
        };
        if let Err(why) = self
            .document
            .execute(scene::Command::SetExportTarget(Some(target)))
        {
            self.note = Some(format!("Couldn't use that folder: {why}."));
            return;
        }
        let engine = self.detected_engine();
        if let Some((engine, _)) = &engine {
            self.exporting.choice = self.exporting.choice.with_preset(engine.preset());
        }
        self.note = Some(match engine {
            Some((engine, project)) => format!(
                "Game project folder set: {} project {}. Every save now exports there too.",
                engine.name(),
                file_name(&project)
            ),
            None => format!(
                "Game project folder set. It isn't inside a Godot, Unity, Unreal Engine or Solis \
                 project, so exports go there with {}'s settings. Every save now exports \
                 there too.",
                self.fallback_preset().name()
            ),
        });
    }

    /// The export of `choice`, to `path` (or asking where).
    fn export_action(&self, choice: ExportChoice, path: Option<PathBuf>) -> WorkbenchAction {
        let hardness = self.export_hardness();
        let stem = self
            .file
            .path
            .as_deref()
            .and_then(Path::file_stem)
            .map_or_else(
                || "Untitled".to_string(),
                |s| s.to_string_lossy().into_owned(),
            );
        WorkbenchAction::Export(Box::new(ExportRequest {
            preset: choice.preset,
            format: choice.format,
            options: ExportOptions {
                what: choice.what,
                units: choice.units,
                up: choice.up,
                hardness,
                triangulate: choice.triangulate,
                godot_names: choice.preset == Some(ExportPreset::Godot),
            },
            path,
            suggested_name: format!("{stem}.{}", choice.format.extension()),
        }))
    }

    /// What the export with the window's choices would hold (worked out
    /// again only when the scene, the selection or what to export
    /// changes).
    fn export_summary(&mut self) -> ExportSummary {
        let key = SummaryKey {
            revision: self.document.revision(),
            selection: self.document.selection(),
            what: self.exporting.choice.what,
        };
        if let Some((known, summary)) = &self.exporting.summary
            && *known == key
        {
            return *summary;
        }
        let summary = fileio::export::summary(
            &self.document,
            &ExportOptions {
                what: key.what,
                ..ExportOptions::default()
            },
        );
        self.exporting.summary = Some((key, summary));
        summary
    }

    /// Draws the Export window, if open
    /// ([decision 0087](../../../../docs/decisions/0087-one-look-for-every-window.md)):
    /// Made for in the title bar, a side list of sections (General and
    /// Game project; Materials, Mesh, Rig and Animations join it with
    /// their features), and Export… in the footer.
    pub(super) fn export_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if !self.exporting.open {
            return;
        }
        let mut choice = self.exporting.choice;
        let mut section = self.exporting.section;
        let selected = !self.document.selection().is_empty();
        let summary = self.export_summary();
        let target = self.document.scene().export_target().cloned();
        let engine = self.detected_engine();
        let into = self.target_request(None);
        let fallback = self.fallback_preset();
        let nothing = summary.objects == 0;
        let mut go = false;
        let mut cancel = false;
        let mut bake = false;
        let mut target_change: Option<Option<scene::ExportTarget>> = None;
        let mut choose_folder = false;
        let mut go_to_game_project = false;
        let sections = [
            Section {
                id: ExportSection::General,
                name: "General",
                badge: None,
                apart: false,
                label: None,
            },
            Section {
                id: ExportSection::GameProject,
                name: "Game project",
                badge: Some(match &target {
                    Some(t) if t.on_save => Badge::On,
                    _ => Badge::Off,
                }),
                apart: true,
                label: None,
            },
        ];
        let outcome = Dialog::new(EXPORT_TITLE, 700.0)
            .body_height(490.0)
            .show(ctx, |d| {
                d.header(true, |ui| {
                    ui.label(RichText::new("Made for").color(theme::TEXT_WEAK));
                    made_for(ui, &mut choice);
                });
                d.sections(
                    &mut section,
                    &sections,
                    Some(SECTIONS_NOTE),
                    |ui, shown| match shown {
                        ExportSection::General => {
                            let open_game_project = general(
                                ui,
                                &mut choice,
                                selected,
                                summary,
                                GameProjectView {
                                    target: target.as_ref(),
                                    engine: engine.as_ref(),
                                    into: into.as_ref(),
                                },
                                &mut bake,
                            );
                            go_to_game_project |= open_game_project;
                        }
                        ExportSection::GameProject => game_project(
                            ui,
                            target.as_ref(),
                            engine.as_ref(),
                            fallback,
                            &mut target_change,
                            &mut choose_folder,
                        ),
                    },
                );
                d.footer(
                    nothing.then_some("Nothing to export: nothing shown is a shape."),
                    |ui| {
                        let response =
                            ui.add_enabled_ui(!nothing, |ui| dialog::main_button(ui, "Export…"));
                        if response.inner.clicked() {
                            go = true;
                        }
                        if dialog::plain_button(ui, "Cancel").clicked() {
                            cancel = true;
                        }
                    },
                );
            });
        if go_to_game_project {
            section = ExportSection::GameProject;
        }
        let (enter, escape) = dialog::enter_or_escape(ctx);
        if enter && !nothing {
            go = true;
        }
        if outcome.closed || escape {
            cancel = true;
        }
        if !selected && choice.what == ExportWhat::Selected {
            choice.what = ExportWhat::Everything;
        }
        // Changing a setting by hand leaves the preset.
        if let Some(preset) = choice.preset
            && !choice.follows(preset)
        {
            choice.preset = None;
        }
        self.exporting.choice = choice;
        self.exporting.section = section;
        if bake {
            self.baking.open = true;
        }
        if let Some(change) = target_change {
            self.run_target(change);
        }
        if choose_folder {
            let start = target
                .as_ref()
                .map(|t| PathBuf::from(&t.folder))
                .or_else(|| {
                    self.file
                        .path
                        .as_deref()
                        .and_then(Path::parent)
                        .map(Path::to_path_buf)
                });
            actions.push(WorkbenchAction::ChooseExportFolder { start });
        }
        if cancel {
            self.exporting.open = false;
        }
        if go {
            self.exporting.open = false;
            actions.push(self.export_action(choice, None));
        }
    }

    fn run_target(&mut self, target: Option<scene::ExportTarget>) {
        if let Err(why) = self
            .document
            .execute(scene::Command::SetExportTarget(target))
        {
            self.note = Some(format!("Couldn't change the game project folder: {why}."));
        }
    }

    /// The app exported to `path`, as asked by an export of `request`.
    pub fn exported(&mut self, request: &ExportRequest, path: PathBuf, summary: ExportSummary) {
        let choice = ExportChoice {
            preset: request.preset,
            format: request.format,
            what: request.options.what,
            units: request.options.units,
            up: request.options.up,
            triangulate: request.options.triangulate,
        };
        let objects = match summary.objects {
            1 => "1 object".to_string(),
            n => format!("{n} objects"),
        };
        let made_for = request
            .preset
            .map(|p| format!(" for {}", p.name()))
            .unwrap_or_default();
        let materials = match summary.materials {
            0 | 1 => String::new(),
            n => format!(", {n} materials"),
        };
        self.note = Some(format!(
            "Exported {objects}{made_for} ({} triangles{materials}) to {}. File → Export again \
             repeats it.",
            summary.triangles,
            file_name(&path)
        ));
        self.exporting.last = Some((path, choice));
        self.exporting.remembered = ExportSettings::of(choice);
    }

    /// Exporting failed: says why.
    pub fn export_failed(&mut self, reason: &str) {
        self.note = Some(format!("Couldn't export: {reason}."));
    }

    /// The app read `objects` from the file at `path`: they are added to
    /// the scene as plain meshes, in one undo step, and selected.
    pub fn imported(&mut self, path: &Path, import: fileio::obj::ObjImport) {
        let objects: Vec<scene::Object> = import
            .objects
            .into_iter()
            .map(|o| {
                let mut object = scene::Object::new(scene::ObjectKind::Mesh(Arc::new(o.mesh)));
                object.name = o.name;
                object.materials = o.materials;
                object
            })
            .collect();
        let count = objects.len();
        let label = format!("Import {}", file_name(path));
        match self
            .document
            .execute_labelled(scene::Command::InsertMany(objects), label, None)
        {
            Ok(()) => {
                let objects = match count {
                    1 => "1 object".to_string(),
                    n => format!("{n} objects"),
                };
                let mut note = format!("Imported {objects} from {}.", file_name(path));
                let left_out = import.skipped_polygons;
                if left_out > 0 {
                    note.push_str(&format!(
                        " {left_out} broken polygon{} left out.",
                        if left_out == 1 { " was" } else { "s were" }
                    ));
                }
                note.push_str(" Ctrl + Z takes it back.");
                self.note = Some(note);
                self.viewport.frame(crate::Frame::Selected);
            }
            Err(why) => self.note = Some(format!("Couldn't import: {why}.")),
        }
    }

    /// Importing failed: says why.
    pub fn import_failed(&mut self, path: &Path, reason: &str) {
        self.note = Some(format!("Couldn't import {}: {reason}.", file_name(path)));
    }
}

/// The Made for drop-down in the Export window's title bar: each preset,
/// with its kind of file on the right, then Custom settings.
fn made_for(ui: &mut Ui, choice: &mut ExportChoice) {
    let (name, hint) = match choice.preset {
        Some(preset) => (preset.name(), preset.settings().format.name()),
        None => (CUSTOM_SETTINGS, choice.format.name()),
    };
    let response = dialog::drop_down(ui, "Made for", name, hint, 230.0, |ui| {
        for preset in ExportPreset::ALL {
            if dialog::drop_down_entry(
                ui,
                choice.preset == Some(preset),
                preset.name(),
                preset.settings().format.name(),
            )
            .on_hover_text(preset.description())
            .clicked()
            {
                *choice = choice.with_preset(preset);
            }
        }
        ui.separator();
        if dialog::drop_down_entry(ui, choice.preset.is_none(), CUSTOM_SETTINGS, "any program")
            .on_hover_text("Pick the kind of file and how it's set up yourself")
            .clicked()
        {
            choice.preset = None;
        }
    });
    response.on_hover_text("Where the file is going: this picks the right settings for it");
}

/// The name of the Made for choice without a preset.
pub(crate) const CUSTOM_SETTINGS: &str = "Custom settings";

/// What the General section shows about the game project folder.
struct GameProjectView<'a> {
    target: Option<&'a scene::ExportTarget>,
    engine: Option<&'a (Engine, PathBuf)>,
    into: Option<&'a TargetRequest>,
}

/// The Export window's General section: what goes out, the file's
/// settings, a word on the game project folder, and the numbers. Gives
/// back whether "Game project" was clicked to go there.
fn general(
    ui: &mut Ui,
    choice: &mut ExportChoice,
    selected: bool,
    summary: ExportSummary,
    game: GameProjectView<'_>,
    bake: &mut bool,
) -> bool {
    dialog::heading(ui, "General");
    dialog::help(
        ui,
        match choice.preset {
            Some(preset) => format!("{}: {}.", preset.name(), preset.description()),
            None => "Any program: pick the kind of file and how it's set up below.".to_string(),
        },
    );
    ui.add_space(4.0);
    dialog::row(ui, "Objects", |ui| {
        let selected_help = if selected {
            "The selected objects, with what's inside them"
        } else {
            "Select some objects first"
        };
        dialog::segmented(
            ui,
            &mut choice.what,
            &[
                (
                    ExportWhat::Everything,
                    "Everything shown",
                    "Every object you can see in the viewport",
                ),
                (ExportWhat::Selected, "Selected only", selected_help),
            ],
        );
    });
    dialog::row(ui, "Includes", |ui| {
        ui.add_enabled_ui(false, |ui| {
            dialog::chip(ui, &mut true, "Mesh");
            dialog::chip(ui, &mut true, "Materials");
        })
        .response
        .on_disabled_hover_text(
            "Shapes and their materials always go out. Skeletons and animations will be \
             listed here once the app has them, to leave out when you don't need them.",
        );
    });
    match choice.preset {
        Some(preset) => {
            dialog::row(ui, "File settings", |ui| {
                dialog::locked(ui, &settings_line(preset))
                    .on_hover_text("Set by Made for. Choose Custom settings to change them.");
            });
            if preset == ExportPreset::SubstancePainter {
                dialog::row(ui, "Before baking", |ui| {
                    if ui
                        .button("Check for baking…")
                        .on_hover_text(
                            "List what would spoil the bake, in plain words, with one-click \
                             fixes",
                        )
                        .clicked()
                    {
                        *bake = true;
                    }
                });
            }
        }
        None => custom_settings(ui, choice),
    }
    dialog::help(
        ui,
        "Shading goes out as you see it: with Automatic hard edges on (Mesh menu), by angle; \
         otherwise by the edges you marked hard.",
    );
    dialog::divider(ui);
    let go_there = game_project_note(ui, game);
    dialog::divider(ui);
    dialog::heading(ui, "What goes out");
    ui.add_space(2.0);
    dialog::tiles(
        ui,
        &[
            (
                thousands(summary.objects),
                plural(summary.objects, "object", "objects"),
            ),
            (
                thousands(summary.triangles),
                plural(summary.triangles, "triangle", "triangles"),
            ),
            (
                thousands(summary.materials),
                plural(summary.materials, "material", "materials"),
            ),
        ],
    );
    go_there
}

/// A preset's file settings in one line: "glTF · metres · Y up ·
/// triangles".
pub(crate) fn settings_line(preset: ExportPreset) -> String {
    let settings = preset.settings();
    let options = settings.format.options(&ExportOptions {
        units: settings.units,
        up: settings.up,
        triangulate: settings.triangulate,
        ..ExportOptions::default()
    });
    format!(
        "{} · {} · {} · {}",
        settings.format.name(),
        match options.units {
            FileUnits::Metres => "metres",
            FileUnits::Centimetres => "centimetres",
        },
        match options.up {
            UpAxis::Y => "Y up",
            UpAxis::Z => "Z up",
        },
        if options.triangulate {
            "triangles"
        } else {
            "polygons kept"
        }
    )
}

/// The file's settings, to choose by hand (Custom settings).
fn custom_settings(ui: &mut Ui, choice: &mut ExportChoice) {
    dialog::row(ui, "File type", |ui| {
        dialog::segmented(
            ui,
            &mut choice.format,
            &[
                (
                    ExportFormat::Glb,
                    "glTF (.glb)",
                    "For Godot, Unity, Unreal Engine and Substance Painter: each object at its \
                     pivot",
                ),
                (
                    ExportFormat::Obj,
                    "OBJ (.obj)",
                    "Plain, read by almost every 3D program",
                ),
            ],
        );
    });
    let fixed = choice.format.fixes_space();
    ui.add_enabled_ui(!fixed, |ui| {
        dialog::row(ui, "Units", |ui| {
            dialog::segmented(
                ui,
                &mut choice.units,
                &[
                    (
                        FileUnits::Metres,
                        "Metres",
                        "1 unit in the file is 1 m: Godot, Unity, Substance Painter",
                    ),
                    (
                        FileUnits::Centimetres,
                        "Centimetres",
                        "1 unit in the file is 1 cm: Unreal Engine",
                    ),
                ],
            );
        });
        dialog::row(ui, "Up axis", |ui| {
            dialog::segmented(
                ui,
                &mut choice.up,
                &[
                    (
                        UpAxis::Y,
                        "Y up",
                        "Godot, Unity, Unreal's OBJ import and Substance Painter",
                    ),
                    (
                        UpAxis::Z,
                        "Z up",
                        "As in this app; Blender and 3ds Max's own axes",
                    ),
                ],
            );
        });
        dialog::row(ui, "Triangles", |ui| {
            ui.checkbox(&mut choice.triangulate, "Cut polygons into triangles")
                .on_hover_text(
                    "Cut every polygon into triangles the same way the viewport does, so a \
                     bake in Substance Painter matches the game exactly",
                );
        });
    });
    if fixed {
        dialog::help(
            ui,
            "glTF is always metres, Y up and triangles: every program reads it that way.",
        );
    }
}

/// The General section's word on the game project folder: whether every
/// save exports into it, and where. Gives back whether "Game project" was
/// clicked.
fn game_project_note(ui: &mut Ui, game: GameProjectView<'_>) -> bool {
    let on = game.target.is_some_and(|t| t.on_save);
    dialog::heading_with_pill(ui, "Game project", on);
    let engine_name = game.engine.map(|(engine, _)| engine.name());
    match (game.target, game.into) {
        (Some(target), Some(into)) => {
            if target.on_save {
                dialog::help(
                    ui,
                    format!(
                        "Every time you save the project, the model is also exported into \
                         your game project, so {} picks up the change by itself. It goes to:",
                        engine_name.unwrap_or("the game engine")
                    ),
                );
            } else {
                dialog::help(
                    ui,
                    "Exporting on every save is off. File → Export to game project exports \
                     into it when you choose. It goes to:",
                );
            }
            let file = format!(
                "{}.{}",
                into.name,
                into.preset.settings().format.extension()
            );
            let place = match game.engine {
                Some((engine, project)) => {
                    let inside = into
                        .folder
                        .strip_prefix(project)
                        .ok()
                        .map(|rest| {
                            rest.components()
                                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let mut parts = vec![format!("{} ({})", file_name(project), engine.name())];
                    parts.extend(inside);
                    parts.push(file);
                    parts.join(" › ")
                }
                None => into.folder.join(file).display().to_string(),
            };
            dialog::place(ui, &place).on_hover_text(into.folder.display().to_string());
        }
        _ => {
            dialog::help(
                ui,
                "Pick a folder inside your Godot, Unity, Unreal Engine or Solis project, and every \
                 save exports the model into it too, so the engine picks up the change by \
                 itself.",
            );
        }
    }
    let mut clicked = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let words = if game.target.is_some() {
            "Change the folder or switch it off under "
        } else {
            "Set it up under "
        };
        ui.label(
            RichText::new(words)
                .font(dialog::help_font())
                .color(theme::TEXT_WEAK),
        );
        clicked = ui
            .link(RichText::new("Game project").font(dialog::help_font()))
            .clicked();
        ui.label(
            RichText::new(" in the list.")
                .font(dialog::help_font())
                .color(theme::TEXT_WEAK),
        );
    });
    clicked
}

/// The Export window's Game project section: the folder and its engine,
/// Export on every save, and choosing or clearing it. A change is put in
/// `change`; `choose` asks for a folder.
fn game_project(
    ui: &mut Ui,
    target: Option<&scene::ExportTarget>,
    engine: Option<&(Engine, PathBuf)>,
    fallback: ExportPreset,
    change: &mut Option<Option<scene::ExportTarget>>,
    choose: &mut bool,
) {
    dialog::heading_with_pill(ui, "Game project", target.is_some_and(|t| t.on_save));
    dialog::help(
        ui,
        "Export straight into a Godot, Unity, Unreal Engine or Solis project: pick the folder inside \
         it that your models go in. Every save then exports the model there too, named after \
         the project, so the engine picks up the change by itself.",
    );
    ui.add_space(4.0);
    match target {
        None => {
            dialog::row(ui, "Folder", |ui| {
                ui.label(RichText::new("None chosen").color(theme::TEXT_WEAK));
            });
        }
        Some(target) => {
            dialog::row(ui, "Folder", |ui| {
                dialog::place(ui, &target.folder);
            });
            dialog::row(ui, "Engine", |ui| {
                ui.label(match engine {
                    Some((engine, project)) => format!(
                        "In the {} project {}: exported as {} with its settings.",
                        engine.name(),
                        file_name(project),
                        engine.preset().settings().format.name()
                    ),
                    None => format!(
                        "Not inside a Godot, Unity, Unreal Engine or Solis project: exported with \
                         {}'s settings (Made for picks which).",
                        fallback.name()
                    ),
                });
            });
            dialog::row(ui, "On save", |ui| {
                let mut on_save = target.on_save;
                if ui
                    .checkbox(&mut on_save, "Export on every save")
                    .on_hover_text(
                        "Each time you save the project, export it into this folder too, named \
                         after the project, so the engine picks up the change",
                    )
                    .changed()
                {
                    *change = Some(Some(scene::ExportTarget {
                        on_save,
                        ..target.clone()
                    }));
                }
            });
        }
    }
    ui.add_space(4.0);
    dialog::row(ui, "", |ui| {
        if ui
            .button("Choose folder…")
            .on_hover_text("Pick the folder inside your game project the models go in")
            .clicked()
        {
            *choose = true;
        }
        if target.is_some()
            && ui
                .button("Clear")
                .on_hover_text("Stop exporting into a game project")
                .clicked()
        {
            *change = Some(None);
        }
    });
}

/// `n` with a comma between each three digits from the right: "1,284".
pub(crate) fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `one` for 1, `many` otherwise.
fn plural(n: usize, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 { one } else { many }
}
