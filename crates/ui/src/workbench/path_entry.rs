//! Typing a path when the system's file window can't open. On Linux the
//! Open and Save windows come from the desktop (its "portal", or the
//! zenity program); on a desktop without either, asking for one gives
//! nothing back at once, and Save As, Open, Export… and Choose folder
//! would do nothing without a word. The app then tells the workbench
//! ([`Workbench::file_window_missing`]), which says so in the status bar
//! and opens this small window to type the path in instead.

use super::exporting::{ExportRequest, KitRequest};
use super::{Workbench, WorkbenchAction};
use crate::dialog;
use crate::text::tr;
use crate::theme;
use egui::{Key, Modifiers, RichText};
use std::path::{Path, PathBuf};

/// What the file window was asked for, and so what the typed path is for.
#[derive(Debug, Clone, PartialEq)]
pub enum FilePurpose {
    /// Where to save the project (Save As, or the first Save), as a
    /// project file ending in `extension` (added when left out).
    SaveProject {
        /// The project file's extension, without its dot.
        extension: String,
    },
    /// Which project to open.
    OpenProject,
    /// Which OBJ file to import.
    Import,
    /// Which image to import as a paint layer on the material's texture
    /// set (decision 0177).
    PaintImage {
        /// The material.
        material: String,
    },
    /// Which palette file to import (decision 0183).
    ImportPalette,
    /// Where to write the project's palette number `index`.
    ExportPalette {
        /// Which palette.
        index: usize,
    },
    /// Where to export as the request says.
    Export(Box<ExportRequest>),
    /// Which folder to export the kit's pieces into.
    ExportKit(Box<KitRequest>),
    /// Which folder of a game project to export into on every save.
    GameFolder,
}

impl FilePurpose {
    /// The window's title.
    fn title(&self) -> &'static str {
        match self {
            Self::SaveProject { .. } => tr("Save project as"),
            Self::OpenProject => tr("Open project"),
            Self::Import => tr("Import"),
            Self::PaintImage { .. } => tr("Import an image"),
            Self::ImportPalette => tr("Import a palette"),
            Self::ExportPalette { .. } => tr("Export the palette to"),
            Self::Export(_) => tr("Export to"),
            Self::ExportKit(_) => tr("Export kit into"),
            Self::GameFolder => tr("Game project folder"),
        }
    }

    /// The main button's name.
    fn button(&self) -> &'static str {
        match self {
            Self::SaveProject { .. } => tr("Save"),
            Self::OpenProject => tr("Open"),
            Self::Import | Self::PaintImage { .. } | Self::ImportPalette => tr("Import"),
            Self::Export(_) | Self::ExportKit(_) | Self::ExportPalette { .. } => tr("Export"),
            Self::GameFolder => tr("Choose folder"),
        }
    }

    /// Whether the path names a folder (not a file).
    fn wants_folder(&self) -> bool {
        matches!(self, Self::ExportKit(_) | Self::GameFolder)
    }

    /// Whether the path must already exist (opening, importing, a folder).
    fn must_exist(&self) -> bool {
        !matches!(
            self,
            Self::SaveProject { .. } | Self::Export(_) | Self::ExportPalette { .. }
        )
    }

    /// The extension a file written for it ends in, if it writes one.
    fn extension(&self) -> Option<&str> {
        match self {
            Self::SaveProject { extension } => Some(extension),
            Self::Export(request) => Some(request.format.extension()),
            Self::ExportPalette { .. } => Some("gpl"),
            _ => None,
        }
    }
}

/// The path window while it is open.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PathEntry {
    purpose: FilePurpose,
    /// What is typed.
    text: String,
    /// The box still needs the keyboard (on its first frame).
    focus: bool,
}

/// The name of the path box, for screen readers and tests.
pub(crate) const PATH_BOX_LABEL: &str = crate::text!("Path");

/// `text` as a path, with a `~` at its start standing for the home
/// folder (`home`).
fn expanded(text: &str, home: Option<&Path>) -> PathBuf {
    let text = text.trim();
    match (text.strip_prefix('~'), home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
            home.join(rest.trim_start_matches(['/', '\\']))
        }
        _ => PathBuf::from(text),
    }
}

/// The home folder, from the environment.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// `path` ending in `.extension`: added if the name typed left it out.
fn with_extension(path: PathBuf, extension: &str) -> PathBuf {
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

/// What `text` means for `purpose`: the path to use (with the file's
/// extension added when left out), or why it can't be used, in a
/// sentence.
pub(crate) fn checked(
    purpose: &FilePurpose,
    text: &str,
    home: Option<&Path>,
) -> Result<PathBuf, String> {
    if text.trim().is_empty() {
        return Err(tr("Type a path first.").to_string());
    }
    let path = expanded(text, home);
    if purpose.wants_folder() {
        return if path.is_dir() {
            Ok(path)
        } else {
            Err(tr("There's no folder there.").to_string())
        };
    }
    if path.is_dir() {
        return Err(tr("That's a folder: add the file's name after it.").to_string());
    }
    if purpose.must_exist() {
        return if path.is_file() {
            Ok(path)
        } else {
            Err(tr("There's no file there.").to_string())
        };
    }
    let path = match purpose.extension() {
        Some(extension) => with_extension(path, extension),
        None => path,
    };
    match path.parent() {
        Some(folder) if folder.as_os_str().is_empty() || folder.is_dir() => Ok(path),
        Some(folder) => Err(crate::trf!(
            "The folder {} doesn't exist.",
            folder.display()
        )),
        None => Err(tr("Type the file's name too.").to_string()),
    }
}

/// The note when saving to `path` would replace a file.
fn replaced(path: &Path) -> String {
    crate::trf!("{} is there already: it will be replaced.", path.display())
}

impl Workbench {
    /// The app asked the system for a file window for `purpose` and none
    /// came (it gave nothing back at once). Says so in the status bar and
    /// opens a small window to type the path in instead, starting with
    /// `start` (a folder, or a suggested file). Whatever was waiting for
    /// a save (closing, say) still waits for this one.
    pub fn file_window_missing(&mut self, purpose: FilePurpose, start: Option<PathBuf>) {
        self.note = Some(
            tr(
                "The system's file window didn't open (on Linux it needs the desktop's file \
                 portal or the zenity program). Type the path in the window instead.",
            )
            .to_string(),
        );
        self.path_entry = Some(PathEntry {
            purpose,
            text: start.map(|p| p.display().to_string()).unwrap_or_default(),
            focus: true,
        });
    }

    /// Whether the path window is open.
    pub fn is_typing_a_path(&self) -> bool {
        self.path_entry.is_some()
    }

    /// The path window, while open: a box to type the path in, why it
    /// can't be used yet, and the button that uses it.
    pub(super) fn path_window(&mut self, ctx: &egui::Context, actions: &mut Vec<WorkbenchAction>) {
        let Some(entry) = self.path_entry.as_mut() else {
            return;
        };
        let home = home();
        let check = checked(&entry.purpose, &entry.text, home.as_deref());
        let mut go = false;
        let mut cancel = false;
        let title = entry.purpose.title();
        let button = entry.purpose.button();
        dialog::Dialog::new(title, 520.0)
            .id("type_a_path")
            .modal()
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(tr(
                        "The system's file window couldn't open, so type the full path here. \
                         ~ stands for your home folder.",
                    ));
                    ui.add_space(6.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut entry.text)
                            .desired_width(f32::INFINITY)
                            .hint_text(tr("/home/you/Models/crate")),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::TextEdit,
                            true,
                            tr(PATH_BOX_LABEL),
                        )
                    });
                    if entry.focus {
                        response.request_focus();
                        entry.focus = false;
                    }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        go = true;
                        // Not usable yet: keep typing in the box.
                        if check.is_err() {
                            response.request_focus();
                        }
                    }
                    ui.add_space(4.0);
                    match &check {
                        Ok(path) if path.is_file() && !entry.purpose.must_exist() => {
                            ui.label(
                                RichText::new(replaced(path))
                                    .font(dialog::help_font())
                                    .color(theme::WARNING),
                            );
                        }
                        Ok(path) => {
                            dialog::help(ui, path.display().to_string());
                        }
                        Err(why) => {
                            dialog::help(ui, why.clone());
                        }
                    }
                });
                d.footer(None, |ui| {
                    let ready = check.is_ok();
                    if ui
                        .add_enabled_ui(ready, |ui| dialog::main_button(ui, button))
                        .inner
                        .clicked()
                    {
                        go = true;
                    }
                    if dialog::plain_button(ui, tr("Cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            cancel = true;
        }
        if cancel {
            if matches!(entry.purpose, FilePurpose::SaveProject { .. }) {
                self.save_cancelled();
            }
            self.path_entry = None;
            return;
        }
        let Ok(path) = check else {
            return;
        };
        if !go {
            return;
        }
        let Some(entry) = self.path_entry.take() else {
            return;
        };
        match entry.purpose {
            FilePurpose::SaveProject { .. } => {
                actions.push(WorkbenchAction::Save { path: Some(path) });
            }
            FilePurpose::OpenProject => actions.push(WorkbenchAction::Open { path: Some(path) }),
            FilePurpose::Import => actions.push(WorkbenchAction::Import { path: Some(path) }),
            FilePurpose::ImportPalette => {
                actions.push(WorkbenchAction::ImportPalette { path: Some(path) });
            }
            FilePurpose::ExportPalette { index } => {
                actions.push(WorkbenchAction::ExportPalette {
                    index,
                    path: Some(path),
                });
            }
            FilePurpose::PaintImage { material } => {
                actions.push(WorkbenchAction::ImportPaintImage {
                    material,
                    path: Some(path),
                });
            }
            FilePurpose::Export(mut request) => {
                request.path = Some(path);
                actions.push(WorkbenchAction::Export(request));
            }
            FilePurpose::ExportKit(mut request) => {
                request.folder = Some(path);
                actions.push(WorkbenchAction::ExportKit(request));
            }
            FilePurpose::GameFolder => self.export_folder_chosen(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("path-entry-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_home_folder_mark_is_expanded() {
        let home = Path::new("/home/someone");
        assert_eq!(
            expanded("~/Models/crate", Some(home)),
            PathBuf::from("/home/someone/Models/crate")
        );
        assert_eq!(expanded("~", Some(home)), PathBuf::from("/home/someone"));
        // Only a ~ of its own: "~crate" is a name.
        assert_eq!(expanded("~crate", Some(home)), PathBuf::from("~crate"));
        assert_eq!(expanded("  /a/b  ", Some(home)), PathBuf::from("/a/b"));
        assert_eq!(expanded("~/x", None), PathBuf::from("~/x"));
    }

    #[test]
    fn a_save_gets_its_extension_and_needs_its_folder() {
        let at = folder("save");
        let save = FilePurpose::SaveProject {
            extension: "avtl".into(),
        };
        let typed = at.join("crate").display().to_string();
        assert_eq!(checked(&save, &typed, None), Ok(at.join("crate.avtl")));
        let typed = at.join("crate.AVTL").display().to_string();
        assert_eq!(checked(&save, &typed, None), Ok(at.join("crate.AVTL")));
        let missing = at.join("no such folder").join("crate");
        let why = checked(&save, &missing.display().to_string(), None).unwrap_err();
        assert!(why.contains("doesn't exist"), "{why}");
        let why = checked(&save, &at.display().to_string(), None).unwrap_err();
        assert!(why.contains("folder"), "{why}");
        assert!(checked(&save, "   ", None).is_err());
        let _ = std::fs::remove_dir_all(&at);
    }

    #[test]
    fn opening_needs_a_file_and_a_folder_needs_a_folder() {
        let at = folder("open");
        let file = at.join("crate.avtl");
        std::fs::write(&file, "x").unwrap();
        let typed = file.display().to_string();
        assert_eq!(
            checked(&FilePurpose::OpenProject, &typed, None),
            Ok(file.clone())
        );
        assert!(
            checked(
                &FilePurpose::Import,
                &at.join("gone.obj").display().to_string(),
                None
            )
            .is_err()
        );
        assert!(checked(&FilePurpose::GameFolder, &typed, None).is_err());
        assert_eq!(
            checked(&FilePurpose::GameFolder, &at.display().to_string(), None),
            Ok(at.clone())
        );
        let _ = std::fs::remove_dir_all(&at);
    }
}
