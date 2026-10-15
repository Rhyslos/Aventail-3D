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

/// Saves the workbench's document to `path` and reports back. A project
/// that exports into a game project on every save does that too, once
/// the save has succeeded (decision 0076).
pub fn save_to(workbench: &mut Workbench, path: PathBuf) {
    let export = workbench.save_export_request(&path);
    match fileio::project::save(&path, workbench.document()) {
        Ok(()) => {
            let outcome = export.map(|request| export_into(workbench, &request));
            workbench.saved_with_export(path, outcome);
        }
        Err(error) => workbench.save_failed(&error.to_string()),
    }
}

/// Exports the workbench's document into its game project folder as
/// `request` says.
pub fn export_into(workbench: &Workbench, request: &ui::TargetRequest) -> ui::TargetOutcome {
    fileio::engine::export_into(
        &request.folder,
        &request.name,
        workbench.document(),
        request.preset,
        &request.options,
        &generator(),
        APP_NAME,
    )
    .map_err(|e| e.to_string())
}

/// Exports each piece of `request` as its own file: into its folder, or
/// one the user chooses (decision 0078). Reports back.
pub fn export_kit(workbench: &mut Workbench, request: &ui::KitRequest) {
    let folder = match &request.folder {
        Some(folder) => folder.clone(),
        None => {
            let mut dialog = rfd::FileDialog::new().set_title("Export kit into");
            if let Some(folder) = workbench.project_path().and_then(Path::parent) {
                dialog = dialog.set_directory(folder);
            }
            match dialog.pick_folder() {
                Some(folder) => folder,
                None => return,
            }
        }
    };
    let outcome = kit_into(workbench, request, &folder);
    workbench.kit_exported(outcome);
}

/// Exports each piece of `request` into `folder`.
fn kit_into(workbench: &Workbench, request: &ui::KitRequest, folder: &Path) -> ui::KitOutcome {
    fileio::engine::export_kit(
        folder,
        workbench.document(),
        &request.pieces,
        request.format,
        &request.options,
        &generator(),
        APP_NAME,
    )
    .map(|pieces| {
        pieces
            .into_iter()
            .map(|(name, result)| (name, result.map_err(|e| e.to_string())))
            .collect()
    })
    .map_err(|e| e.to_string())
}

/// Asks which folder of a game project to export into, starting at
/// `start`, and tells the workbench.
pub fn choose_export_folder(workbench: &mut Workbench, start: Option<PathBuf>) {
    let mut dialog = rfd::FileDialog::new().set_title("Game project folder");
    if let Some(start) = start.filter(|s| s.is_dir()) {
        dialog = dialog.set_directory(start);
    }
    if let Some(folder) = dialog.pick_folder() {
        workbench.export_folder_chosen(folder);
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

/// The name the app writes into exported files: "Aventail 3D 0.0.1".
fn generator() -> String {
    format!("{APP_NAME} {}", env!("CARGO_PKG_VERSION"))
}

/// `path` ending in `.extension`: added if the name typed left it out.
fn with_file_extension(path: PathBuf, extension: &str) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(extension))
    {
        return path;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(extension);
    path.with_file_name(name)
}

/// Exports the workbench's document as `request` says (asking where
/// first when it has no path) and reports back.
pub fn export(workbench: &mut Workbench, request: &ui::ExportRequest) {
    let extension = request.format.extension();
    let path = match &request.path {
        Some(path) => path.clone(),
        None => {
            let mut dialog = rfd::FileDialog::new()
                .set_title("Export")
                .add_filter(request.format.name(), &[extension])
                .set_file_name(&request.suggested_name);
            if let Some(folder) = workbench.project_path().and_then(Path::parent) {
                dialog = dialog.set_directory(folder);
            }
            match dialog.save_file() {
                Some(path) => with_file_extension(path, extension),
                None => return,
            }
        }
    };
    export_to(workbench, request, path);
}

/// Exports the workbench's document to `path` and reports back.
pub fn export_to(workbench: &mut Workbench, request: &ui::ExportRequest, path: PathBuf) {
    match fileio::export::export_to(
        &path,
        workbench.document(),
        request.format,
        &request.options,
        &generator(),
    ) {
        Ok(summary) => workbench.exported(request, path, summary),
        Err(error) => workbench.export_failed(&error.to_string()),
    }
}

/// Imports a file into the workbench's scene: `path`, or the one the user
/// chooses.
pub fn import(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => {
            let mut dialog = rfd::FileDialog::new()
                .set_title("Import")
                .add_filter("OBJ", &["obj"]);
            if let Some(folder) = workbench.project_path().and_then(Path::parent) {
                dialog = dialog.set_directory(folder);
            }
            match dialog.pick_file() {
                Some(path) => path,
                None => return,
            }
        }
    };
    import_from(workbench, &path);
}

/// Imports the OBJ file at `path` and reports back. Too large a file is
/// refused before it is read.
pub fn import_from(workbench: &mut Workbench, path: &Path) {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > fileio::limits::MAX_UNPACKED {
        return workbench.import_failed(path, fileio::limits::TOO_MUCH);
    }
    let text = match std::fs::read(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => return workbench.import_failed(path, &error.to_string()),
    };
    match fileio::obj::read_obj(
        &text,
        fileio::export::FileUnits::Metres,
        fileio::export::UpAxis::Y,
    ) {
        Ok(mut import) => {
            import.colour_from_files(path.parent().unwrap_or(Path::new(".")));
            workbench.imported(path, import)
        }
        Err(error) => workbench.import_failed(path, &error.to_string()),
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

    #[test]
    fn exporting_and_importing_go_through_the_workbench() {
        let folder = std::env::temp_dir().join(format!("app-export-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut first = workbench();
        let mut doc = scene::Document::new();
        doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        first.opened(doc, folder.join("crate.avtl"));
        let request = ui::ExportRequest {
            preset: None,
            format: fileio::export::ExportFormat::Obj,
            options: fileio::export::ExportOptions::default(),
            path: None,
            suggested_name: "Untitled.obj".into(),
        };
        let path = folder.join("crate.obj");
        export_to(&mut first, &request, path.clone());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(&format!("# {APP_NAME} ")));
        // Its materials beside it.
        assert!(text.contains("mtllib crate.mtl"));
        assert!(folder.join("crate.mtl").exists());
        assert_eq!(with_file_extension(folder.join("crate"), "obj"), path);

        let mut second = workbench();
        import_from(&mut second, &path);
        assert_eq!(second.document().scene().children(None).len(), 1);
        import_from(&mut second, &folder.join("missing.obj"));
        assert_eq!(second.document().scene().children(None).len(), 1);

        // A game project folder: every save exports there too, named after
        // the project.
        let game = folder.join("game");
        std::fs::create_dir_all(game.join("models")).unwrap();
        std::fs::write(game.join("project.godot"), "").unwrap();
        first.export_folder_chosen(game.join("models"));
        save_to(&mut first, folder.join("crate.avtl"));
        let glb = game.join("models/crate.glb");
        assert!(glb.exists());
        assert!(fileio::engine::may_replace(&glb, APP_NAME));
        // Someone else's file of that name is left alone, and the save
        // still happens.
        std::fs::write(game.join("models/barrel.glb"), "theirs").unwrap();
        save_to(&mut first, folder.join("barrel.avtl"));
        assert!(folder.join("barrel.avtl").exists());
        assert_eq!(
            std::fs::read_to_string(game.join("models/barrel.glb")).unwrap(),
            "theirs"
        );

        // A kit: each selected piece its own file, into the game folder.
        let kit = first.kit_request().expect("the cube is selected");
        assert_eq!(kit.folder.as_deref(), Some(game.join("models").as_path()));
        let outcome = kit_into(&first, &kit, &game.join("models")).unwrap();
        assert_eq!(outcome.len(), 1);
        assert!(game.join("models/Cube.glb").exists());

        // Nothing to export: nothing written.
        let mut empty = workbench();
        export_to(&mut empty, &request, folder.join("empty.obj"));
        assert!(!folder.join("empty.obj").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
