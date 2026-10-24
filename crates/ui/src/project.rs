//! The open project, as the interface sees it: where it is saved, whether
//! it has unsaved changes, the recently used projects, and the question
//! asked before unsaved changes would be lost.
//!
//! The interface never touches files itself: it hands Save and Open to
//! the app as [`crate::WorkbenchAction`]s, and the app reports back
//! ([`crate::Workbench::saved`], [`crate::Workbench::opened`] and so on).

use crate::text::tr;
use egui::{Key, KeyboardShortcut, Modifiers};
use std::path::{Path, PathBuf};

/// Start a new, empty project.
pub const NEW_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
/// Open a project.
pub const OPEN_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
/// Save the project.
pub const SAVE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
/// Save the project under a new name.
pub const SAVE_AS_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);

/// How many recent projects File → Open recent lists.
pub const RECENT_COUNT: usize = 8;

/// What to do once unsaved changes are dealt with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Pending {
    /// Start a new project.
    New,
    /// Open a project: this one, or ask which.
    Open(Option<PathBuf>),
    /// Close the app.
    Quit,
}

impl Pending {
    /// The end of "Save changes to … before …?".
    pub(crate) fn before(&self) -> &'static str {
        match self {
            Self::New => tr("starting a new project"),
            Self::Open(_) => tr("opening another project"),
            Self::Quit => tr("closing"),
        }
    }
}

/// The open project's file state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProjectFile {
    /// Where the project is saved, if it has been.
    pub(crate) path: Option<PathBuf>,
    /// The document revision when last saved or opened.
    pub(crate) saved_revision: u64,
    /// Recently saved or opened projects, newest first.
    pub(crate) recent: Vec<PathBuf>,
    /// Asking whether to save first, before doing this.
    pub(crate) asking: Option<Pending>,
    /// Do this once the save the user asked for has finished.
    pub(crate) after_save: Option<Pending>,
    /// The save or export the app is doing on another thread, if any
    /// (decision 0163).
    pub(crate) busy: Option<crate::workbench::FileWork>,
}

impl ProjectFile {
    /// The project's name for the title bar and questions: its file name,
    /// or "Untitled" before the first save.
    pub(crate) fn name(&self) -> String {
        self.path.as_deref().and_then(Path::file_name).map_or_else(
            || tr("Untitled").to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// Puts `path` at the top of the recent list.
    pub(crate) fn remember(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT_COUNT);
    }
}

/// A recent project's label in the menu: its file name, and its folder.
pub(crate) fn recent_label(path: &Path) -> String {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    match path.parent().and_then(Path::file_name) {
        Some(folder) => crate::trf!("{}  ({})", name, folder.to_string_lossy()),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsaved_project_is_untitled() {
        let mut file = ProjectFile::default();
        assert_eq!(file.name(), "Untitled");
        file.path = Some(PathBuf::from("/work/crate.avtl"));
        assert_eq!(file.name(), "crate.avtl");
    }

    #[test]
    fn recent_projects_are_newest_first_without_repeats_and_capped() {
        let mut file = ProjectFile::default();
        for n in 0..12 {
            file.remember(Path::new(&format!("/p/{n}.avtl")));
        }
        assert_eq!(file.recent.len(), RECENT_COUNT);
        assert_eq!(file.recent[0], PathBuf::from("/p/11.avtl"));
        file.remember(Path::new("/p/8.avtl"));
        assert_eq!(file.recent[0], PathBuf::from("/p/8.avtl"));
        assert_eq!(
            file.recent.iter().filter(|p| p.ends_with("8.avtl")).count(),
            1
        );
    }

    #[test]
    fn recent_labels_show_the_name_and_folder() {
        assert_eq!(
            recent_label(Path::new("/work/props/crate.avtl")),
            "crate.avtl  (props)"
        );
        assert_eq!(recent_label(Path::new("crate.avtl")), "crate.avtl");
    }
}
