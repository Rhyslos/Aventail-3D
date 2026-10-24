//! The workbench's part in exporting and importing
//! ([decision 0071](../../../../docs/decisions/0071-obj-export-and-import.md)):
//! File → Import…, Export… and Export again, and the Export window. The
//! files themselves are written and read by the app (through `fileio`),
//! which reports back.

use super::{Workbench, WorkbenchAction, file_name};
use crate::text::tr;
use egui::Ui;
use fileio::engine::Engine;
use fileio::export::{
    ExportFormat, ExportOptions, ExportPreset, ExportSummary, ExportWhat, FileUnits, UpAxis,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod window;

// The tests reach this through `super::exporting::`, as when it was
// defined here.
#[cfg(test)]
pub(crate) use window::settings_line;

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
    /// Whether each object's levels of detail go out beside it (decision
    /// 0155). Not a preset's setting: kept when Made for changes.
    pub lod_levels: bool,
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
            lod_levels: false,
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
            lod_levels: self.lod_levels,
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
    /// Whether levels of detail go out (decision 0155).
    pub lod_levels: bool,
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
            lod_levels: choice.lod_levels,
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
            lod_levels: self.lod_levels,
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
        tr("1 piece").to_string()
    } else {
        crate::trf!("{} pieces", n)
    }
}

/// The status bar's words for `outcome`.
pub(crate) fn target_note(outcome: &TargetOutcome) -> String {
    match outcome {
        Ok((path, summary)) => {
            let place = path.parent().map(file_name).unwrap_or_default();
            crate::trf!(
                "Exported {} ({} triangles) into {}.",
                file_name(path),
                summary.triangles,
                place
            )
        }
        Err(why) => crate::trf!("Couldn't export into the game project: {}.", why),
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
pub(crate) const EXPORT_TITLE: &str = crate::text!("Export");

/// The note at the foot of the Export window's side list.
const SECTIONS_NOTE: &str = crate::text!(
    "Only sections with something to set are listed. Materials, Mesh, \
                             Rig and Animations arrive with their features."
);

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
            .button(tr("Import…"))
            .on_hover_text(tr(
                "Bring shapes in from an OBJ file, each object as a plain mesh, added to the \
                 scene",
            ))
            .clicked()
        {
            actions.push(WorkbenchAction::Import { path: None });
            ui.close();
        }
        if ui
            .button(tr("Export…"))
            .on_hover_text(tr(
                "Write the scene (or the selection) to a file for a game engine or Substance \
                 Painter: pick the kind of file and how",
            ))
            .clicked()
        {
            self.exporting.open = true;
            ui.close();
        }
        let again = self.exporting.last.clone();
        let response = ui
            .add_enabled(again.is_some(), egui::Button::new(tr("Export again")))
            .on_disabled_hover_text(tr("Export once first; this then repeats it"));
        let response = match &again {
            Some((path, _)) => response.on_hover_text(crate::trf!(
                "Export to {} again, the same way",
                path.display()
            )),
            None => response,
        };
        if response.clicked()
            && let Some((path, choice)) = again
        {
            actions.push(self.export_action(choice, Some(path)));
            ui.close();
        }
        if ui
            .button(tr("Export kit…"))
            .on_hover_text(tr(
                "Export each selected object (with what's inside it) as a file of its own, \
                 named after it, its pivot at the file's origin: modular pieces for a game \
                 engine. Uses the Export window's settings",
            ))
            .clicked()
        {
            match self.kit_request() {
                Some(request) => actions.push(WorkbenchAction::ExportKit(Box::new(request))),
                None => {
                    self.note = Some(tr("Export kit: select the pieces to export first.").into());
                }
            }
            ui.close();
        }
        let target = self.target_request(None);
        let response = ui
            .add_enabled(
                target.is_some(),
                egui::Button::new(tr("Export to game project")),
            )
            .on_disabled_hover_text(tr(
                "Choose a game project folder in the Export window first; this then exports \
                 straight into it",
            ));
        let response = match &target {
            Some(request) => response.on_hover_text(crate::trf!(
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
                || tr("Untitled").to_string(),
                |s| s.to_string_lossy().into_owned(),
            );
        Some(TargetRequest {
            folder,
            name,
            preset,
            options: ExportOptions {
                what: ExportWhat::Everything,
                hardness: self.export_hardness(),
                lod_levels: self.exporting.choice.lod_levels,
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
        self.file_work_done();
        self.note = Some(match outcome {
            Err(why) => crate::trf!("Couldn't export the kit: {}.", why),
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
                    0 => tr("No pieces exported.").to_string(),
                    n if n == pieces.len() => {
                        crate::trf!("Exported {} into {}.", counted_pieces(n), place)
                    }
                    n => crate::trf!(
                        "Exported {} of {} into {}.",
                        n,
                        counted_pieces(pieces.len()),
                        place
                    ),
                };
                for (name, result) in &pieces {
                    if let Err(why) = result {
                        note.push_str(&crate::trf!(" {}: {}.", name, why));
                    }
                }
                note
            }
        });
    }

    /// The app exported into the game project folder (File → Export to
    /// game project): says how it went.
    pub fn target_exported(&mut self, outcome: TargetOutcome) {
        self.file_work_done();
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
            self.note = Some(crate::trf!("Couldn't use that folder: {}.", why));
            return;
        }
        let engine = self.detected_engine();
        if let Some((engine, _)) = &engine {
            self.exporting.choice = self.exporting.choice.with_preset(engine.preset());
        }
        self.note = Some(match engine {
            Some((engine, project)) => crate::trf!(
                "Game project folder set: {} project {}. Every save now exports there too.",
                engine.name(),
                file_name(&project)
            ),
            None => crate::trf!(
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
                || tr("Untitled").to_string(),
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
                lod_levels: choice.lod_levels,
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

    fn run_target(&mut self, target: Option<scene::ExportTarget>) {
        if let Err(why) = self
            .document
            .execute(scene::Command::SetExportTarget(target))
        {
            self.note = Some(crate::trf!(
                "Couldn't change the game project folder: {}.",
                why
            ));
        }
    }

    /// The app exported to `path`, as asked by an export of `request`.
    pub fn exported(&mut self, request: &ExportRequest, path: PathBuf, summary: ExportSummary) {
        self.file_work_done();
        let choice = ExportChoice {
            preset: request.preset,
            format: request.format,
            what: request.options.what,
            units: request.options.units,
            up: request.options.up,
            triangulate: request.options.triangulate,
            lod_levels: request.options.lod_levels,
        };
        let objects = match summary.objects {
            1 => tr("1 object").to_string(),
            n => crate::trf!("{} objects", n),
        };
        let made_for = request
            .preset
            .map(|p| crate::trf!(" for {}", p.name()))
            .unwrap_or_default();
        let materials = match summary.materials {
            0 | 1 => String::new(),
            n => crate::trf!(", {} materials", n),
        };
        self.note = Some(crate::trf!(
            "Exported {}{} ({} triangles{}) to {}. File → Export again \
             repeats it.",
            objects,
            made_for,
            summary.triangles,
            materials,
            file_name(&path)
        ));
        self.exporting.last = Some((path, choice));
        self.exporting.remembered = ExportSettings::of(choice);
    }

    /// Exporting failed: says why.
    pub fn export_failed(&mut self, reason: &str) {
        self.file_work_done();
        self.note = Some(crate::trf!("Couldn't export: {}.", reason));
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
        let label = crate::trf!("Import {}", file_name(path));
        match self
            .document
            .execute_labelled(scene::Command::InsertMany(objects), label, None)
        {
            Ok(()) => {
                let objects = match count {
                    1 => tr("1 object").to_string(),
                    n => crate::trf!("{} objects", n),
                };
                let mut note = crate::trf!("Imported {} from {}.", objects, file_name(path));
                let left_out = import.skipped_polygons;
                if left_out > 0 {
                    note.push_str(&crate::trf!(
                        " {} broken polygon{} left out.",
                        left_out,
                        if left_out == 1 {
                            tr(" was")
                        } else {
                            tr("s were")
                        }
                    ));
                }
                note.push_str(tr(" Ctrl + Z takes it back."));
                self.note = Some(note);
                self.viewport.frame(crate::Frame::Selected);
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't import: {}.", why)),
        }
    }

    /// Importing failed: says why.
    pub fn import_failed(&mut self, path: &Path, reason: &str) {
        self.note = Some(crate::trf!(
            "Couldn't import {}: {}.",
            file_name(path),
            reason
        ));
    }
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
