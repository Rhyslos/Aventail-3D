//! Saving and opening projects for the workbench: the native Open and
//! Save dialogs, and the `fileio` calls. The workbench says what it wants
//! ([`ui::WorkbenchAction`]); this carries it out and reports back.
//! Saving and exporting are done in two halves: the work itself, on a copy
//! of the document, which [`crate::file_work`] runs on another thread
//! (decision 0163), and the report to the workbench afterwards.

use crate::branding::{APP_NAME, PROJECT_EXTENSION};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use ui::{FilePurpose, Workbench};
use words::{tr, trf};

/// What a system file window gave back.
#[derive(Debug, Clone, PartialEq)]
pub enum Asked<T> {
    /// The person chose this.
    Chosen(T),
    /// The person closed the window without choosing.
    Closed,
    /// No window came: nothing came back at once (see [`NO_WINDOW_WITHIN`]).
    NoWindow,
}

/// How soon a file window that gives nothing back counts as never shown.
/// The windows only say "nothing chosen", whether the person closed the
/// window or it couldn't open (on Linux it needs the desktop's file portal
/// or the zenity program); but nobody sees a window and closes it within
/// half a second, while one that can't open comes back at once.
pub const NO_WINDOW_WITHIN: Duration = Duration::from_millis(500);

/// What a file window that gave back `result` after `took` means.
pub fn asked<T>(result: Option<T>, took: Duration) -> Asked<T> {
    match result {
        Some(chosen) => Asked::Chosen(chosen),
        None if took < NO_WINDOW_WITHIN => Asked::NoWindow,
        None => Asked::Closed,
    }
}

/// Shows a file window with `show` and says what came of it.
fn ask<T>(show: impl FnOnce() -> Option<T>) -> Asked<T> {
    let start = Instant::now();
    let result = show();
    asked(result, start.elapsed())
}

/// The home folder, to start typing a path from when there's no project
/// folder.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Where typing a path starts: `folder` (or the home folder) and `name`.
fn typed_start(folder: Option<&Path>, name: Option<&str>) -> Option<PathBuf> {
    let folder = folder.map(Path::to_path_buf).or_else(home)?;
    Some(match name {
        Some(name) => folder.join(name),
        None => folder,
    })
}

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
    trf!("{} project", APP_NAME)
}

/// The project file name to offer: the project's own, or Untitled.
fn project_name(current: Option<&Path>) -> String {
    current.and_then(Path::file_name).map_or_else(
        || trf!("Untitled.{}", PROJECT_EXTENSION),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Asks where to save, starting in the project's folder with its name.
fn ask_where_to_save(current: Option<&Path>) -> Asked<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(tr("Save project"))
        .add_filter(filter_name(), &[PROJECT_EXTENSION])
        .set_file_name(project_name(current));
    if let Some(folder) = current.and_then(Path::parent) {
        dialog = dialog.set_directory(folder);
    }
    match ask(|| dialog.save_file()) {
        Asked::Chosen(path) => Asked::Chosen(with_extension(path)),
        other => other,
    }
}

/// Asks which project to open.
fn ask_which_to_open(current: Option<&Path>) -> Asked<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .set_title(tr("Open project"))
        .add_filter(filter_name(), &[PROJECT_EXTENSION]);
    if let Some(folder) = current.and_then(Path::parent) {
        dialog = dialog.set_directory(folder);
    }
    ask(|| dialog.pick_file())
}

/// Where to save the workbench's document: `path`, or where the user
/// chooses; `None` if they close the dialog (and the workbench told), or
/// if it couldn't open (the workbench then asks for the path itself).
pub fn where_to_save(workbench: &mut Workbench, path: Option<PathBuf>) -> Option<PathBuf> {
    if path.is_some() {
        return path;
    }
    let current = workbench.project_path().map(Path::to_path_buf);
    match ask_where_to_save(current.as_deref()) {
        Asked::Chosen(path) => Some(path),
        Asked::Closed => {
            workbench.save_cancelled();
            None
        }
        Asked::NoWindow => {
            let name = project_name(current.as_deref());
            let start = typed_start(current.as_deref().and_then(Path::parent), Some(&name));
            workbench.file_window_missing(
                FilePurpose::SaveProject {
                    extension: PROJECT_EXTENSION.to_string(),
                },
                start,
            );
            None
        }
    }
}

/// How a save went: whether the project was saved, and how the export
/// into its game project folder that follows every save went, if it has
/// one (decision 0076).
#[derive(Debug)]
pub struct SaveOutcome {
    /// Saved, or why not.
    pub result: Result<(), String>,
    /// The export after it, if any (only once saved).
    pub export: Option<ui::TargetOutcome>,
}

/// Saves `document` to `path`, then exports it as `export` says if the
/// save succeeded. The work of a save: slow for heavy projects (the file
/// is opened again to check it), so run away from the window's thread.
pub fn save_work(
    document: &scene::Document,
    path: &Path,
    export: Option<&ui::TargetRequest>,
) -> SaveOutcome {
    match fileio::project::save(path, document) {
        Ok(()) => SaveOutcome {
            result: Ok(()),
            export: export.map(|request| export_into_work(document, request)),
        },
        Err(error) => SaveOutcome {
            result: Err(error.to_string()),
            export: None,
        },
    }
}

/// Tells the workbench how the save of its document as it was at
/// `revision` to `path` went.
pub fn report_saved(workbench: &mut Workbench, path: PathBuf, revision: u64, outcome: SaveOutcome) {
    match outcome.result {
        Ok(()) => workbench.saved_at(path, revision, outcome.export),
        Err(why) => workbench.save_failed(&why),
    }
}

/// Saves the workbench's document to `path` here, on this thread, and
/// reports back (the window saves on another thread, [`crate::file_work`]).
#[cfg(test)]
pub fn save_to(workbench: &mut Workbench, path: PathBuf) {
    let export = workbench.save_export_request(&path);
    let revision = workbench.document().revision();
    let outcome = save_work(workbench.document(), &path, export.as_ref());
    report_saved(workbench, path, revision, outcome);
}

/// Exports `document` into its game project folder as `request` says.
pub fn export_into_work(
    document: &scene::Document,
    request: &ui::TargetRequest,
) -> ui::TargetOutcome {
    fileio::engine::export_into(
        &request.folder,
        &request.name,
        document,
        request.preset,
        &request.options,
        &generator(),
        APP_NAME,
    )
    .map_err(|e| e.to_string())
}

/// The folder to export the kit pieces of `request` into: its own, or one
/// the user chooses (decision 0078); `None` if they close the dialog.
pub fn kit_folder(workbench: &mut Workbench, request: &ui::KitRequest) -> Option<PathBuf> {
    if let Some(folder) = &request.folder {
        return Some(folder.clone());
    }
    let mut dialog = rfd::FileDialog::new().set_title(tr("Export kit into"));
    let project_folder = workbench
        .project_path()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    if let Some(folder) = &project_folder {
        dialog = dialog.set_directory(folder);
    }
    match ask(|| dialog.pick_folder()) {
        Asked::Chosen(folder) => Some(folder),
        Asked::Closed => None,
        Asked::NoWindow => {
            workbench.file_window_missing(
                FilePurpose::ExportKit(Box::new(request.clone())),
                typed_start(project_folder.as_deref(), None),
            );
            None
        }
    }
}

/// Exports each piece of `request` into `folder`, from `document`.
pub fn kit_work(
    document: &scene::Document,
    request: &ui::KitRequest,
    folder: &Path,
) -> ui::KitOutcome {
    fileio::engine::export_kit(
        folder,
        document,
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
    let mut dialog = rfd::FileDialog::new().set_title(tr("Game project folder"));
    let start = start.filter(|s| s.is_dir());
    if let Some(start) = &start {
        dialog = dialog.set_directory(start);
    }
    match ask(|| dialog.pick_folder()) {
        Asked::Chosen(folder) => workbench.export_folder_chosen(folder),
        Asked::Closed => {}
        Asked::NoWindow => {
            workbench
                .file_window_missing(FilePurpose::GameFolder, typed_start(start.as_deref(), None));
        }
    }
}

/// Opens a project: `path`, or the one the user chooses.
pub fn open(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => {
            let current = workbench.project_path().map(Path::to_path_buf);
            match ask_which_to_open(current.as_deref()) {
                Asked::Chosen(path) => path,
                Asked::Closed => return,
                Asked::NoWindow => {
                    let folder = current.as_deref().and_then(Path::parent);
                    workbench
                        .file_window_missing(FilePurpose::OpenProject, typed_start(folder, None));
                    return;
                }
            }
        }
    };
    open_from(workbench, path);
}

/// Opens the project at `path` into the workbench and reports back.
pub fn open_from(workbench: &mut Workbench, path: PathBuf) {
    match fileio::project::open_noting(&path) {
        Ok((document, notes)) => workbench.opened_noting(document, path, &notes),
        Err(error) => workbench.open_failed(&path, &error.to_string()),
    }
}

/// The name the app writes into exported files: "Aventail 3D 0.0.1".
fn generator() -> String {
    // Written into files: the same in every language.
    format!("{} {}", APP_NAME, env!("CARGO_PKG_VERSION"))
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

/// Where to export as `request` says: its path, or where the user
/// chooses; `None` if they close the dialog.
pub fn export_path(workbench: &mut Workbench, request: &ui::ExportRequest) -> Option<PathBuf> {
    if let Some(path) = &request.path {
        return Some(path.clone());
    }
    let extension = request.format.extension();
    let mut dialog = rfd::FileDialog::new()
        .set_title(tr("Export"))
        .add_filter(request.format.name(), &[extension])
        .set_file_name(&request.suggested_name);
    let folder = workbench
        .project_path()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    if let Some(folder) = &folder {
        dialog = dialog.set_directory(folder);
    }
    match ask(|| dialog.save_file()) {
        Asked::Chosen(path) => Some(with_file_extension(path, extension)),
        Asked::Closed => None,
        Asked::NoWindow => {
            workbench.file_window_missing(
                FilePurpose::Export(Box::new(request.clone())),
                typed_start(folder.as_deref(), Some(&request.suggested_name)),
            );
            None
        }
    }
}

/// Exports `document` to `path` as `request` says: the work of an export,
/// slow for heavy scenes, so run away from the window's thread.
pub fn export_work(
    document: &scene::Document,
    request: &ui::ExportRequest,
    path: &Path,
) -> Result<fileio::export::ExportSummary, String> {
    fileio::export::export_to(
        path,
        document,
        request.format,
        &request.options,
        &generator(),
    )
    .map_err(|e| e.to_string())
}

/// Tells the workbench how the export of `request` to `path` went.
pub fn report_exported(
    workbench: &mut Workbench,
    request: &ui::ExportRequest,
    path: PathBuf,
    result: Result<fileio::export::ExportSummary, String>,
) {
    match result {
        Ok(summary) => workbench.exported(request, path, summary),
        Err(why) => workbench.export_failed(&why),
    }
}

/// Exports the workbench's document to `path` here, on this thread, and
/// reports back (the window exports on another thread).
#[cfg(test)]
pub fn export_to(workbench: &mut Workbench, request: &ui::ExportRequest, path: PathBuf) {
    let result = export_work(workbench.document(), request, &path);
    report_exported(workbench, request, path, result);
}

/// Imports a file into the workbench's scene: `path`, or the one the user
/// chooses.
pub fn import(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => {
            let mut dialog = rfd::FileDialog::new()
                .set_title(tr("Import"))
                .add_filter("OBJ", &["obj"]);
            let folder = workbench
                .project_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            if let Some(folder) = &folder {
                dialog = dialog.set_directory(folder);
            }
            match ask(|| dialog.pick_file()) {
                Asked::Chosen(path) => path,
                Asked::Closed => return,
                Asked::NoWindow => {
                    workbench.file_window_missing(
                        FilePurpose::Import,
                        typed_start(folder.as_deref(), None),
                    );
                    return;
                }
            }
        }
    };
    import_from(workbench, &path);
}

/// Imports an image as a paint layer on the texture set of `material`
/// (decision 0177): `path`, or the one the user chooses.
pub fn import_paint_image(workbench: &mut Workbench, material: &str, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => {
            let mut dialog = rfd::FileDialog::new()
                .set_title(tr("Import an image"))
                .add_filter(tr("Images"), &fileio::image_file::IMAGE_EXTENSIONS);
            let folder = workbench
                .project_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            if let Some(folder) = &folder {
                dialog = dialog.set_directory(folder);
            }
            match ask(|| dialog.pick_file()) {
                Asked::Chosen(path) => path,
                Asked::Closed => return,
                Asked::NoWindow => {
                    workbench.file_window_missing(
                        FilePurpose::PaintImage {
                            material: material.to_string(),
                        },
                        typed_start(folder.as_deref(), None),
                    );
                    return;
                }
            }
        }
    };
    match fileio::image_file::open_image(&path) {
        Ok(image) => workbench.import_paint_image(&path, material, &image),
        Err(why) => workbench.import_failed(&path, &why),
    }
}

/// Reads a picture for projection painting (decision 0186): `path`, or
/// the one the user chooses.
pub fn choose_stencil(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => {
            let mut dialog = rfd::FileDialog::new()
                .set_title(tr("Choose a picture to project"))
                .add_filter(tr("Images"), &fileio::image_file::IMAGE_EXTENSIONS);
            let folder = workbench
                .project_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            if let Some(folder) = &folder {
                dialog = dialog.set_directory(folder);
            }
            match ask(|| dialog.pick_file()) {
                Asked::Chosen(path) => path,
                Asked::Closed => return,
                Asked::NoWindow => {
                    workbench.file_window_missing(
                        FilePurpose::Stencil,
                        typed_start(folder.as_deref(), None),
                    );
                    return;
                }
            }
        }
    };
    match fileio::image_file::open_image(&path) {
        Ok(image) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            workbench.set_stencil(&name, &image);
        }
        Err(why) => workbench.import_failed(&path, &why),
    }
}

/// The largest palette file read, in bytes.
const MAX_PALETTE_FILE: u64 = 1 << 20;

/// Reads a GIMP palette file into the project's palettes (decision
/// 0183): `path`, or the one the user chooses.
pub fn import_palette(workbench: &mut Workbench, path: Option<PathBuf>) {
    let path = match path {
        Some(path) => path,
        None => {
            let mut dialog = rfd::FileDialog::new()
                .set_title(tr("Import a palette"))
                .add_filter(tr("GIMP palettes"), &["gpl"]);
            let folder = workbench
                .project_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            if let Some(folder) = &folder {
                dialog = dialog.set_directory(folder);
            }
            match ask(|| dialog.pick_file()) {
                Asked::Chosen(path) => path,
                Asked::Closed => return,
                Asked::NoWindow => {
                    workbench.file_window_missing(
                        FilePurpose::ImportPalette,
                        typed_start(folder.as_deref(), None),
                    );
                    return;
                }
            }
        }
    };
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > MAX_PALETTE_FILE {
        return workbench.import_failed(&path, tr("it is too large for a palette"));
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            workbench.import_palette(&path, &text);
        }
        Err(why) => workbench.import_failed(&path, &why.to_string()),
    }
}

/// Writes the project's palette number `index` as a GIMP palette file
/// (decision 0183): to `path`, or where the user chooses.
pub fn export_palette(workbench: &mut Workbench, index: usize, path: Option<PathBuf>) {
    let Some((name, text)) = workbench.palette_file(index) else {
        return;
    };
    let path = match path {
        Some(path) => path,
        None => {
            let file: String = name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == ' ' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let file = format!("{}.gpl", file.trim());
            let mut dialog = rfd::FileDialog::new()
                .set_title(tr("Export the palette"))
                .add_filter(tr("GIMP palettes"), &["gpl"])
                .set_file_name(file.clone());
            let folder = workbench
                .project_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf);
            if let Some(folder) = &folder {
                dialog = dialog.set_directory(folder);
            }
            match ask(|| dialog.save_file()) {
                Asked::Chosen(path) => path,
                Asked::Closed => return,
                Asked::NoWindow => {
                    workbench.file_window_missing(
                        FilePurpose::ExportPalette { index },
                        typed_start(folder.as_deref(), Some(&file)),
                    );
                    return;
                }
            }
        }
    };
    let result = std::fs::write(&path, text).map_err(|why| why.to_string());
    workbench.palette_exported(&path, result);
}

/// Imports the OBJ file at `path` and reports back. Too large a file is
/// refused before it is read.
pub fn import_from(workbench: &mut Workbench, path: &Path) {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size > fileio::limits::MAX_UNPACKED {
        return workbench.import_failed(path, ui::text::tr(fileio::limits::TOO_MUCH));
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
    fn a_file_window_that_gives_nothing_at_once_never_showed() {
        // Nothing chosen at once: no window came (no portal, no zenity).
        assert_eq!(
            asked::<PathBuf>(None, Duration::from_millis(3)),
            Asked::NoWindow
        );
        // Nothing chosen after a while: the person closed it.
        assert_eq!(
            asked::<PathBuf>(None, Duration::from_secs(2)),
            Asked::Closed
        );
        // A choice is a choice, however quick.
        let path = PathBuf::from("/w/crate.avtl");
        assert_eq!(
            asked(Some(path.clone()), Duration::ZERO),
            Asked::Chosen(path)
        );
    }

    #[test]
    fn typing_starts_in_the_folder_given_with_the_name() {
        assert_eq!(
            typed_start(Some(Path::new("/w")), Some("crate.glb")),
            Some(PathBuf::from("/w/crate.glb"))
        );
        assert_eq!(
            typed_start(Some(Path::new("/w")), None),
            Some(PathBuf::from("/w"))
        );
        assert_eq!(project_name(None), format!("Untitled.{PROJECT_EXTENSION}"));
        assert_eq!(
            project_name(Some(Path::new("/w/barrel.avtl"))),
            "barrel.avtl"
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
        let outcome = kit_work(first.document(), &kit, &game.join("models")).unwrap();
        assert_eq!(outcome.len(), 1);
        assert!(game.join("models/Cube.glb").exists());

        // Nothing to export: nothing written.
        let mut empty = workbench();
        export_to(&mut empty, &request, folder.join("empty.obj"));
        assert!(!folder.join("empty.obj").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
