//! Box files for your own boxes (decision 0263): what writing one
//! writes, what reading one does, and your library kept with the app's
//! settings. The app shows the file windows and reads and writes the
//! files.

use super::{Workbench, WorkbenchAction};
use crate::material::own_boxes::BoxFileAsk;
use crate::text::tr;
use fileio::own_box_file::{BoxFile, read_box_file, write_box_file};
use std::path::Path;

impl Workbench {
    /// Turns what the Material mode asked for into actions for the app.
    pub(super) fn carry_out_box_file_asks(&mut self, actions: &mut Vec<WorkbenchAction>) {
        match self.material.box_file_asked.take() {
            Some(BoxFileAsk::Save(file)) => {
                self.box_file_out = Some(file);
                actions.push(WorkbenchAction::SaveBoxFile { path: None });
            }
            Some(BoxFileAsk::Load) => actions.push(WorkbenchAction::LoadBoxFile { path: None }),
            None => {}
        }
    }

    /// The box file being written: a name for the file and its text.
    pub fn box_file(&self) -> Option<(String, String)> {
        let file = self.box_file_out.as_ref()?;
        Some((file.main.name.clone(), write_box_file(file)))
    }

    /// Says the box file was written to `path`, or why it wasn't.
    pub fn box_file_saved(&mut self, path: &Path, result: Result<(), String>) {
        let file = super::files::file_name(path);
        self.note = Some(match result {
            Ok(()) => crate::trf!("Saved the box file {}.", file),
            Err(why) => crate::trf!("Couldn't save the box file {}: {}.", file, why),
        });
    }

    /// Adds the box in the box file read from `path` to your library
    /// (replacing one with the same id), to place from the shelf.
    pub fn load_box_file(&mut self, path: &Path, bytes: &[u8]) {
        let file = match read_box_file(bytes) {
            Ok(file) => file,
            Err(why) => return self.import_failed(path, &why),
        };
        let name = file.main.name.clone();
        let library = &mut self.material.library;
        match library.iter().position(|f| f.main.id == file.main.id) {
            Some(at) => library[at] = file,
            None if library.len() >= matgraph::own::MAX_OWN_BOXES => {
                return self.import_failed(path, tr("your library is full"));
            }
            None => library.push(file),
        }
        self.note = Some(crate::trf!(
            "Added {} to your library: it is on the shelf in the Material mode; click it to \
             place it.",
            name
        ));
    }

    /// Your library of own boxes, each as a box file's text, for saving
    /// between sessions.
    pub fn own_library(&self) -> Vec<String> {
        self.material.library.iter().map(write_box_file).collect()
    }

    /// Gives the workbench the library kept last session; a box that
    /// can't be read any more is left out.
    pub fn with_own_library(mut self, kept: Vec<String>) -> Self {
        self.material.library = kept
            .iter()
            .filter_map(|text| read_box_file(text.as_bytes()).ok())
            .take(matgraph::own::MAX_OWN_BOXES)
            .collect::<Vec<BoxFile>>();
        self
    }
}
