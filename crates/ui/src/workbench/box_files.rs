//! Box files for your own boxes (decision 0263), and packs and graphs
//! for the library browser (decision 0292): what writing one writes,
//! what reading one does, and your library (own boxes and graphs) kept
//! with the app's settings. The app shows the file windows and reads and
//! writes the files.

use super::{Workbench, WorkbenchAction};
use crate::material::library::{LibraryAsk, LibraryGraph, MAX_LIBRARY_GRAPHS, Place};
use crate::material::own_boxes::BoxFileAsk;
use crate::text::tr;
use fileio::graph_file::{GRAPH_EXTENSION, read_graph_file, write_graph_file};
use fileio::own_box_file::{BOX_EXTENSION, BoxFile, read_box_file, write_box_file};
use fileio::pack_file::{read_pack, write_pack};
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
        match self.material.library_file_asked.take() {
            Some(LibraryAsk::SavePack(pack, name)) => {
                self.pack_out = Some((name, write_pack(&pack)));
                actions.push(WorkbenchAction::SavePack { path: None });
            }
            Some(LibraryAsk::BringIn) => {
                actions.push(WorkbenchAction::BringIntoLibrary { path: None });
            }
            None => {}
        }
    }

    /// The pack being written: a name for the file and its text.
    pub fn pack_file(&self) -> Option<(String, String)> {
        self.pack_out.clone()
    }

    /// Says the pack was written to `path`, or why it wasn't.
    pub fn pack_saved(&mut self, path: &Path, result: Result<(), String>) {
        let file = super::files::file_name(path);
        self.note = Some(match result {
            Ok(()) => crate::trf!("Saved the pack {}.", file),
            Err(why) => crate::trf!("Couldn't save the pack {}: {}.", file, why),
        });
    }

    /// Brings the file read from `path` into your library: a box file
    /// or a graph file by its extension, anything else read as a pack.
    /// Everything in it goes in or nothing does; the library browser
    /// then shows your library.
    pub fn bring_into_library(&mut self, path: &Path, bytes: &[u8]) {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if extension.as_deref() == Some(BOX_EXTENSION) {
            return self.load_box_file(path, bytes);
        }
        let pack = if extension.as_deref() == Some(GRAPH_EXTENSION) {
            read_graph_file(bytes).map(|graph| fileio::pack_file::Pack {
                graphs: vec![graph],
                boxes: Vec::new(),
            })
        } else {
            read_pack(bytes)
        };
        let pack = match pack {
            Ok(pack) => pack,
            Err(why) => return self.import_failed(path, &why),
        };
        let library = &self.material;
        let new_graphs = pack
            .graphs
            .iter()
            .filter(|g| library.library_graph(g.graph.name()).is_none())
            .count();
        let new_boxes = pack
            .boxes
            .iter()
            .filter(|b| !library.library.iter().any(|f| f.main.id == b.main.id))
            .count();
        if library.library_graphs.len() + new_graphs > MAX_LIBRARY_GRAPHS
            || library.library.len() + new_boxes > matgraph::own::MAX_OWN_BOXES
        {
            return self.import_failed(path, tr("your library hasn't room for all of it"));
        }
        let (graphs, boxes) = (pack.graphs.len(), pack.boxes.len());
        for graph in pack.graphs {
            let _ = self.material.keep_graph(graph);
        }
        for own in pack.boxes {
            let _ = self.material.keep_box(own);
        }
        let browser = &mut self.material.browser;
        browser.open = true;
        browser.from = Some(Place::Library);
        self.note = Some(crate::trf!(
            "Brought {} graphs and {} own boxes into your library: they are in the Library \
             window, in every project.",
            graphs,
            boxes
        ));
    }

    /// The graphs in your library, each as a graph file's text, for
    /// saving between sessions.
    pub fn graph_library(&self) -> Vec<String> {
        self.material
            .library_graphs
            .iter()
            .map(|g| write_graph_file(&g.file))
            .collect()
    }

    /// Gives the workbench the graphs kept in your library last session;
    /// one that can't be read any more, or a second by the same name, is
    /// left out.
    pub fn with_graph_library(mut self, kept: Vec<String>) -> Self {
        let mut graphs: Vec<std::sync::Arc<LibraryGraph>> = Vec::new();
        for file in kept
            .iter()
            .filter_map(|t| read_graph_file(t.as_bytes()).ok())
        {
            if graphs.len() < MAX_LIBRARY_GRAPHS
                && !graphs.iter().any(|g| g.name() == file.graph.name())
            {
                graphs.push(LibraryGraph::new(file));
            }
        }
        self.material.library_graphs = graphs;
        self
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
