//! The workbench's part in exporting and importing
//! ([decision 0071](../../../../docs/decisions/0071-obj-export-and-import.md)):
//! File → Import…, Export… and Export again, and the Export window. The
//! files themselves are written and read by the app (through `fileio`),
//! which reports back.

use super::{Workbench, WorkbenchAction, file_name};
use crate::theme;
use egui::{RichText, Ui};
use fileio::engine::Engine;
use fileio::export::{
    ExportFormat, ExportOptions, ExportPreset, ExportSummary, ExportWhat, FileUnits, UpAxis,
};
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

/// How an export into the game project folder went: the file written and
/// what it held, or why not.
pub type TargetOutcome = Result<(PathBuf, ExportSummary), String>;

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
    /// The engine the game project folder belongs to, and that project's
    /// own folder, worked out for the folder named (only when it changes).
    detected: Option<(String, Option<(Engine, PathBuf)>)>,
}

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

    /// The shading exports use: as the viewport shows it.
    fn export_hardness(&self) -> mesh::Hardness {
        match self.viewport.options().auto_hard_angle() {
            Some(angle) => mesh::Hardness::ByAngle(angle),
            None => mesh::Hardness::Marked,
        }
    }

    /// The export into the game project folder, if the project has one:
    /// everything shown, with the engine's preset (Godot's when the folder
    /// isn't in a known engine's project), named after the project saved
    /// at `saving_to` (or the project's own file, or "Untitled").
    pub fn target_request(&mut self, saving_to: Option<&Path>) -> Option<TargetRequest> {
        let folder = PathBuf::from(&self.document.scene().export_target()?.folder);
        let preset = self
            .detected_engine()
            .map_or(ExportPreset::Godot, |(engine, _)| engine.preset());
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
            None => "Game project folder set. It isn't inside a Godot, Unity or Unreal Engine \
                     project, but exports still go there. Every save now exports there too."
                .to_string(),
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
            },
            path,
            suggested_name: format!("{stem}.{}", choice.format.extension()),
        }))
    }

    /// Draws the Export window, if open.
    pub(super) fn export_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if !self.exporting.open {
            return;
        }
        let mut open = true;
        let mut choice = self.exporting.choice;
        let mut go = false;
        let selected = !self.document.selection().is_empty();
        egui::Window::new("Export")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(360.0);
                let heading = |ui: &mut Ui, text: &str| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(text).strong().color(theme::TEXT_STRONG));
                };
                heading(ui, "Made for");
                ui.horizontal_wrapped(|ui| {
                    for preset in ExportPreset::ALL {
                        let response = ui
                            .radio(choice.preset == Some(preset), preset.name())
                            .on_hover_text(preset.description());
                        if response.clicked() {
                            choice = choice.with_preset(preset);
                        }
                    }
                    if ui
                        .radio(choice.preset.is_none(), "Your own settings")
                        .on_hover_text("Choose the kind of file and how below")
                        .clicked()
                    {
                        choice.preset = None;
                    }
                });
                if let Some(preset) = choice.preset {
                    ui.label(
                        RichText::new(preset.description())
                            .small()
                            .color(theme::TEXT_WEAK),
                    );
                }
                if choice.preset == Some(ExportPreset::SubstancePainter)
                    && ui
                        .button("Check for baking…")
                        .on_hover_text(
                            "List what would spoil the bake, in plain words, with one-click \
                             fixes",
                        )
                        .clicked()
                {
                    self.baking.open = true;
                }
                heading(ui, "Kind of file");
                for format in ExportFormat::ALL {
                    ui.radio_value(&mut choice.format, format, format_label(format));
                }
                heading(ui, "What");
                ui.radio_value(&mut choice.what, ExportWhat::Everything, "Everything shown");
                ui.add_enabled_ui(selected, |ui| {
                    ui.radio_value(
                        &mut choice.what,
                        ExportWhat::Selected,
                        "The selected objects",
                    )
                    .on_disabled_hover_text("Select some objects first");
                });
                heading(ui, "Size and direction");
                let fixed = choice.format.fixes_space();
                if fixed {
                    ui.label(
                        RichText::new(
                            "glTF is always metres, Y up and triangles: every program reads it \
                             that way.",
                        )
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                }
                ui.add_enabled_ui(!fixed, |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut choice.units, FileUnits::Metres, "Metres")
                            .on_hover_text(
                                "1 unit in the file is 1 m: Godot, Unity, Substance Painter",
                            );
                        ui.radio_value(&mut choice.units, FileUnits::Centimetres, "Centimetres")
                            .on_hover_text("1 unit in the file is 1 cm: Unreal Engine");
                    });
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut choice.up, UpAxis::Y, "Y up")
                            .on_hover_text(
                                "Godot, Unity, Unreal's OBJ import and Substance Painter",
                            );
                        ui.radio_value(&mut choice.up, UpAxis::Z, "Z up")
                            .on_hover_text("As in this app; Blender and 3ds Max's own axes");
                    });
                    ui.checkbox(&mut choice.triangulate, "Cut polygons into triangles")
                        .on_hover_text(
                            "Cut every polygon into triangles the same way the viewport does, so a \
                         bake in Substance Painter matches the game exactly",
                        );
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Shading is exported as you see it: with Automatic hard edges on (Mesh \
                         menu), by angle; otherwise by the edges you marked hard.",
                    )
                    .small()
                    .color(theme::TEXT_WEAK),
                );
                heading(ui, "Game project folder");
                self.target_rows(ui, actions);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Export…").clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.exporting.open = false;
                    }
                });
            });
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
        if !open {
            self.exporting.open = false;
        }
        if go {
            self.exporting.open = false;
            actions.push(self.export_action(choice, None));
        }
    }

    /// The Export window's game project folder rows: the folder and its
    /// engine, Export on every save, and choosing or clearing it.
    fn target_rows(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let target = self.document.scene().export_target().cloned();
        let weak = |ui: &mut Ui, text: String| {
            ui.label(RichText::new(text).small().color(theme::TEXT_WEAK));
        };
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
        match &target {
            None => weak(
                ui,
                "Export straight into a Godot, Unity or Unreal Engine project, and again on \
                 every save."
                    .to_string(),
            ),
            Some(target) => {
                ui.label(&target.folder);
                let engine = self.detected_engine();
                weak(
                    ui,
                    match engine {
                        Some((engine, project)) => format!(
                            "In the {} project {}: exported as {} with its settings.",
                            engine.name(),
                            file_name(&project),
                            engine.preset().settings().format.name()
                        ),
                        None => "Not inside a Godot, Unity or Unreal Engine project: exported \
                                 with Godot's settings."
                            .to_string(),
                    },
                );
                let mut on_save = target.on_save;
                if ui
                    .checkbox(&mut on_save, "Export on every save")
                    .on_hover_text(
                        "Each time you save the project, export it into this folder too, named \
                         after the project, so the engine picks up the change",
                    )
                    .changed()
                {
                    self.run_target(Some(scene::ExportTarget {
                        on_save,
                        ..target.clone()
                    }));
                }
            }
        }
        ui.horizontal(|ui| {
            if ui
                .button("Choose folder…")
                .on_hover_text("Pick the folder inside your game project the models go in")
                .clicked()
            {
                actions.push(WorkbenchAction::ChooseExportFolder {
                    start: start.clone(),
                });
            }
            if target.is_some()
                && ui
                    .button("Clear")
                    .on_hover_text("Stop exporting into a game project")
                    .clicked()
            {
                self.run_target(None);
            }
        });
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

/// A format as the Export window lists it.
fn format_label(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Glb => {
            "glTF (.glb): for Godot and Substance Painter, each object at its pivot"
        }
        ExportFormat::Obj => "OBJ (.obj): plain, read by almost every 3D program",
    }
}
