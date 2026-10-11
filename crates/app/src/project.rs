//! Saving and opening projects for the workbench: the native Open and
//! Save dialogs, and the `fileio` calls. The workbench says what it wants
//! ([`ui::WorkbenchAction`]); this carries it out and reports back.

use crate::branding::{APP_NAME, PROJECT_EXTENSION};
use std::path::{Path, PathBuf};
use ui::Workbench;

/// `path` ending in `.avtl`: added if the name typed in the Save dialog
/// left it out (other dots in the name are kept).
pub fn with_extension(path: PathBuf) -> PathBuf {
    let has_it = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(PROJECT_EXTENSION));
    if has_it {
        path
    } else {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".");
        name.push(PROJECT_EXTENSION);
        path.with_file_name(name)
    }
}

/// The dialogs' file type filter: "Aventail 3D project (*.avtl)".
fn filter_name() -> String {
    format!("{APP_NAME} project")
}

/// Asks where to save, starting in the project's folder with its name.
fn ask_where_to_save(current: Option<&Path>) -> Option<PathBuf> {
    let name = current.and_then(Path::file_name).map_or_else(
        || format!("Untitled.{PROJECT_EXTENSION}"),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save project")
        .add_filter(filter_name(), &[PROJECT_EXTENSION])
        .set_file_name(name);
    if let Some(folder) = current.and_then(Path::parent) {
        dialog = dialog.set_directory(folder);
    }
    dialog.save_file().map(with_extension)
}

/// Asks which project to open.
fn ask_which_to_open(current: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Open project")
        .add_filter(filter_name(), &[PROJECT_EXTENSION]);
    if let Some(folder) = current.and_then(Path::parent) {
        dialog = dialog.set_directory(folder);
    }
    dialog.pick_file()
}

/// Saves the workbench's document: to `path`, or where the user chooses.
pub fn save(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => match ask_where_to_save(workbench.project_path()) {
            Some(path) => path,
            None => return workbench.save_cancelled(),
        },
    };
    save_to(workbench, path);
}

/// Saves the workbench's document to `path` and reports back.
pub fn save_to(workbench: &mut Workbench, path: PathBuf) {
    match fileio::project::save(&path, workbench.document()) {
        Ok(()) => workbench.saved(path),
        Err(error) => workbench.save_failed(&error.to_string()),
    }
}

/// Opens a project: `path`, or the one the user chooses.
pub fn open(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => match ask_which_to_open(workbench.project_path()) {
            Some(path) => path,
            None => return,
        },
    };
    open_from(workbench, path);
}

/// Opens the project at `path` into the workbench and reports back.
pub fn open_from(workbench: &mut Workbench, path: PathBuf) {
    match fileio::project::open(&path) {
        Ok(document) => workbench.opened(document, path),
        Err(error) => workbench.open_failed(&path, &error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workbench() -> Workbench {
        Workbench::new(crate::app_info(), None)
    }

    #[test]
    fn the_extension_is_added_when_left_out() {
        assert_eq!(
            with_extension(PathBuf::from("/w/crate")),
            PathBuf::from("/w/crate.avtl")
        );
        assert_eq!(
            with_extension(PathBuf::from("/w/crate.v2")),
            PathBuf::from("/w/crate.v2.avtl")
        );
        assert_eq!(
            with_extension(PathBuf::from("/w/crate.AVTL")),
            PathBuf::from("/w/crate.AVTL")
        );
    }

    #[test]
    fn the_filter_uses_the_apps_name() {
        assert!(filter_name().starts_with(APP_NAME));
    }

    #[test]
    fn saving_and_opening_go_through_the_workbench() {
        let folder = std::env::temp_dir().join(format!("app-project-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("crate.avtl");

        let mut first = workbench();
        save_to(&mut first, path.clone());
        assert_eq!(first.project_path(), Some(path.as_path()));
        assert!(!first.has_unsaved_changes());
        assert!(first.title().starts_with("crate.avtl — "));

        let mut second = workbench();
        open_from(&mut second, path.clone());
        assert_eq!(second.project_path(), Some(path.as_path()));
        assert_eq!(second.recent_projects(), std::slice::from_ref(&path));

        let mut third = workbench();
        open_from(&mut third, folder.join("missing.avtl"));
        assert_eq!(third.project_path(), None);

        let mut fourth = workbench();
        save_to(&mut fourth, folder.join("no such folder").join("x.avtl"));
        assert_eq!(fourth.project_path(), None, "a failed save changes nothing");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
