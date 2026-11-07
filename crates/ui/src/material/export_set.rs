//! The Material mode's ways to use a graph's material outside the canvas
//! (Phase 12, milestone 6, [decision 0262](../../../../docs/decisions/0262-baked-export.md)):
//! **Give to the selected shapes** (the workbench does it, `giving`), and
//! **Export textures…**: the material as a seamless texture set of
//! picture files, made for the engine the Export window is set up for, at
//! the texture budget's size.
//!
//! The export is worked out and written on another thread; the export
//! check above the button says, in plain words, what won't come out as
//! the preview shows it (boxes that follow the model, animated boxes) and
//! what stops it (no Material out, boxes from a newer app).

use super::MaterialEditor;
use crate::text::tr;
use crate::theme;
use crate::workbench::ExportSettings;
use egui::Ui;
use fileio::export::graph_textures::{SetOptions, file_names, texture_set, write_set};
use fileio::export::{ExportPreset, TextureFormat};
use matgraph::{Evaluator, Graph, Registry, Size};
use std::path::PathBuf;
use std::thread::JoinHandle;

/// The button exporting a graph's texture set.
pub(crate) const EXPORT_TEXTURES: &str = crate::text!("Export textures…");
/// The button giving a graph's material to the selected shapes.
pub(crate) const GIVE_TO_SELECTED: &str = crate::text!("Give to the selected shapes");

/// An export of a texture set under way.
struct Job {
    /// The graph's name.
    name: String,
    /// The folder.
    folder: PathBuf,
    handle: JoinHandle<Result<Vec<PathBuf>, String>>,
}

/// Texture set exports: the one under way, and how the last went.
#[derive(Default)]
pub(crate) struct SetExport {
    job: Option<Job>,
    /// What the last export said.
    pub(crate) last: Option<String>,
}

impl std::fmt::Debug for SetExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetExport")
            .field("busy", &self.job.is_some())
            .field("last", &self.last)
            .finish()
    }
}

/// How `graph`'s texture set goes out for the export set up as `export`:
/// the engine's packing and normal map, PNG unless TGA is chosen, and a
/// Godot material for Godot.
pub(crate) fn set_options(graph: &Graph, export: ExportSettings) -> SetOptions {
    let choice = export.choice();
    SetOptions {
        stem: graph.name().to_string(),
        format: choice.textures,
        packing: choice.packing,
        green_down: choice.green_down,
        godot: choice.preset == Some(ExportPreset::Godot),
    }
}

/// What an export of `graph` on its own should say first, in plain words:
/// why it can't go (no Material out, boxes from a newer app), then what
/// won't come out as the preview shows it. The first is `true` when it
/// can't go at all.
pub(crate) fn checks(registry: &Registry, graph: &Graph) -> (bool, Vec<String>) {
    let mut out = Vec::new();
    let mut stops = false;
    if graph.first_of(matgraph::eval::MATERIAL_OUT).is_none() {
        stops = true;
        out.push(
            tr(
                "This graph has no Material out box, so it makes no material to export yet. Add \
                 one (Finish group) and plug the material into it.",
            )
            .to_string(),
        );
    }
    for note in matgraph::bake_notes(registry, graph, false) {
        stops |= note.stops();
        out.push(note.describe(registry, graph));
    }
    (stops, out)
}

impl SetExport {
    /// Whether an export is under way.
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Starts exporting `graph`'s material at `size` pixels into `folder`
    /// as `options` say, on another thread.
    pub(crate) fn start(
        &mut self,
        folder: PathBuf,
        graph: Graph,
        registry: std::sync::Arc<Registry>,
        size: u32,
        options: SetOptions,
    ) {
        if self.job.is_some() {
            return;
        }
        let name = graph.name().to_string();
        let into = folder.clone();
        let handle = std::thread::spawn(move || {
            let size = Size::new(size).map_err(|r| r.reason)?;
            let maps = Evaluator::new()
                .material(&registry, &graph, size)
                .map_err(|r| r.reason)?;
            let files = texture_set(&maps, &options)?;
            write_set(&into, &options.stem, &files, options.godot)
        });
        self.job = Some(Job {
            name,
            folder,
            handle,
        });
        self.last = None;
    }

    /// Takes a finished export: what to say about it, once.
    pub(crate) fn collect(&mut self) -> Option<String> {
        if !self.job.as_ref().is_some_and(|j| j.handle.is_finished()) {
            return None;
        }
        let job = self.job.take()?;
        let said = match job.handle.join() {
            Ok(Ok(files)) => crate::trf!(
                "Exported {}'s textures ({} files) into {}.",
                job.name,
                files.len(),
                job.folder.display()
            ),
            Ok(Err(why)) => crate::trf!("Couldn't export {}'s textures: {}", job.name, why),
            Err(_) => crate::trf!(
                "Couldn't export {}'s textures: something went wrong.",
                job.name
            ),
        };
        self.last = Some(said.clone());
        Some(said)
    }

    /// Waits for the export under way (for tests).
    #[cfg(test)]
    pub(crate) fn finish(&mut self) -> Option<String> {
        while self.job.as_ref().is_some_and(|j| !j.handle.is_finished()) {
            std::thread::yield_now();
        }
        self.collect()
    }
}

impl MaterialEditor {
    /// The preview panel's last part: give the material to the selected
    /// shapes, or export it as a texture set, with the export check.
    pub(crate) fn use_rows(
        &mut self,
        ui: &mut Ui,
        graph: &Graph,
        size: u32,
        export: ExportSettings,
        shapes_selected: bool,
    ) {
        ui.add_space(8.0);
        crate::dialog::heading(ui, tr("Use it"));
        let give = ui
            .add_enabled(shapes_selected, egui::Button::new(tr(GIVE_TO_SELECTED)))
            .on_hover_text(tr(
                "Give this material to the selected shapes, without painting: the viewport \
                 shows it at its real-world size, it follows the graph as you change it, and \
                 exporting the model bakes it into textures. Ctrl + Z takes it back",
            ))
            .on_disabled_hover_text(tr(
                "Select one or more shapes first (in Objects, or in the viewport)",
            ));
        if give.clicked() {
            self.give_asked = true;
        }
        ui.add_space(4.0);
        let (stops, checks) = checks(self.registry(), graph);
        for check in &checks {
            ui.label(egui::RichText::new(check).color(theme::WARNING).size(12.0));
        }
        let options = set_options(graph, export);
        let busy = self.export_set.busy();
        let button = ui
            .add_enabled(!stops && !busy, egui::Button::new(tr(EXPORT_TEXTURES)))
            .on_hover_text(tr(
                "Write this material as seamless picture files, to tile in other apps and \
                 engines: pick a folder",
            ))
            .on_disabled_hover_text(if busy {
                tr("Exporting…")
            } else {
                tr("The export check above says why")
            });
        if button.clicked() {
            self.export_asked = true;
        }
        let made_for = export.preset.map_or_else(
            || tr("your own settings").to_string(),
            |p| p.name().to_string(),
        );
        let format = if options.format == TextureFormat::Tga {
            tr("TGA")
        } else {
            tr("PNG")
        };
        crate::dialog::help(
            ui,
            crate::trf!(
                "{} × {} {} files made for {} (the Export window's Made for): {}.",
                size,
                size,
                format,
                made_for,
                file_names(&options).join(", ")
            ),
        );
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("Exporting…"));
            });
        } else if let Some(last) = &self.export_set.last {
            crate::dialog::help(ui, last);
        }
    }
}
