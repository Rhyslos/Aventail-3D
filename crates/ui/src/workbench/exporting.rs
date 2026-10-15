//! The workbench's part in exporting and importing
//! ([decision 0071](../../../../docs/decisions/0071-obj-export-and-import.md)):
//! File → Import…, Export… and Export again, and the Export window. The
//! files themselves are written and read by the app (through `fileio`),
//! which reports back.

use super::{Workbench, WorkbenchAction, file_name};
use crate::theme;
use egui::{RichText, Ui};
use fileio::export::{ExportFormat, ExportOptions, ExportSummary, ExportWhat, FileUnits, UpAxis};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The Export window's choices.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ExportChoice {
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

/// The Export window and the last export.
#[derive(Debug, Clone, Default)]
pub(crate) struct Exporting {
    /// Whether the Export window is showing.
    pub(crate) open: bool,
    /// Its choices.
    pub(crate) choice: ExportChoice,
    /// The last export: where, and how (for Export again).
    pub(crate) last: Option<(PathBuf, ExportChoice)>,
}

/// An export the app is asked to make.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportRequest {
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
    }

    /// The export of `choice`, to `path` (or asking where).
    fn export_action(&self, choice: ExportChoice, path: Option<PathBuf>) -> WorkbenchAction {
        let hardness = match self.viewport.options().auto_hard_angle() {
            Some(angle) => mesh::Hardness::ByAngle(angle),
            None => mesh::Hardness::Marked,
        };
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
                        .on_hover_text("Godot, Unity, Unreal's OBJ import and Substance Painter");
                    ui.radio_value(&mut choice.up, UpAxis::Z, "Z up")
                        .on_hover_text("As in this app; Blender and 3ds Max's own axes");
                });
                ui.checkbox(&mut choice.triangulate, "Cut polygons into triangles")
                    .on_hover_text(
                        "Cut every polygon into triangles the same way the viewport does, so a \
                         bake in Substance Painter matches the game exactly",
                    );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Shading is exported as you see it: with Automatic hard edges on (Mesh \
                         menu), by angle; otherwise by the edges you marked hard.",
                    )
                    .small()
                    .color(theme::TEXT_WEAK),
                );
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
        self.exporting.choice = choice;
        if !open {
            self.exporting.open = false;
        }
        if go {
            self.exporting.open = false;
            actions.push(self.export_action(choice, None));
        }
    }

    /// The app exported to `path`, as asked by an export of `request`.
    pub fn exported(&mut self, request: &ExportRequest, path: PathBuf, summary: ExportSummary) {
        let choice = ExportChoice {
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
        self.note = Some(format!(
            "Exported {objects} ({} triangles) to {}. File → Export again repeats it.",
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
        ExportFormat::Obj => "OBJ (.obj): plain, read by almost every 3D program",
    }
}
