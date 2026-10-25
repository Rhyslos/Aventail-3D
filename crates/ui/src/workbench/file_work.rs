//! Saving and exporting on another thread ([decision 0163](../../../../docs/decisions/0163-saving-and-exporting-in-the-background.md)):
//! what the workbench knows while the app does that work. The status bar
//! says what is being done, with a turning wheel; another save, export,
//! open or new project waits until it is done (the user is told so); the
//! app keeps the window open until it is finished.

use super::{Workbench, WorkbenchAction};
use crate::project::Pending;
use crate::text::tr;
use std::path::{Path, PathBuf};

/// The file work the app is doing on another thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileWork {
    /// Saving the project to this file (and exporting it into its game
    /// project folder afterwards, if it does that on every save).
    Saving(PathBuf),
    /// Exporting to this file.
    Exporting(PathBuf),
    /// Exporting into the project's game project folder.
    ExportingToGame,
    /// Exporting kit pieces, each as a file of its own.
    ExportingKit,
}

impl FileWork {
    /// What the status bar says while it is done.
    fn doing(&self) -> String {
        match self {
            Self::Saving(path) => crate::trf!("Saving {}…", file_name(path)),
            Self::Exporting(path) => crate::trf!("Exporting {}…", file_name(path)),
            Self::ExportingToGame => tr("Exporting into the game project…").to_string(),
            Self::ExportingKit => tr("Exporting the kit…").to_string(),
        }
    }
}

/// A file's name, for notes.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl Workbench {
    /// The app has started `work` on another thread: the status bar says
    /// so until it reports back (with [`Workbench::saved_at`],
    /// [`Workbench::save_failed`], [`Workbench::exported`],
    /// [`Workbench::export_failed`], [`Workbench::target_exported`] or
    /// [`Workbench::kit_exported`]).
    pub fn file_work_started(&mut self, work: FileWork) {
        self.note = Some(work.doing());
        self.file.busy = Some(work);
    }

    /// The file work under way, if any.
    pub fn file_work(&self) -> Option<&FileWork> {
        self.file.busy.as_ref()
    }

    /// The work under way is over (it reported back).
    pub(super) fn file_work_done(&mut self) {
        self.file.busy = None;
    }

    /// The window will close once the work under way is done: the status
    /// bar says so.
    pub fn closing_after_file_work(&mut self) {
        if let Some(work) = &self.file.busy {
            self.note = Some(crate::trf!(
                "{} The app closes when it's done.",
                work.doing()
            ));
        }
    }

    /// Whether starting `next` must wait for the work under way: a new
    /// project, or opening one, while saving (the save would be reported
    /// on the wrong project). Says so.
    pub(super) fn waits_for_file_work(&mut self, next: &Pending) -> bool {
        let saving = matches!(self.file.busy, Some(FileWork::Saving(_)));
        if saving && matches!(next, Pending::New | Pending::Open(_)) {
            self.say_still_busy();
            return true;
        }
        false
    }

    /// Takes out of `actions` the file work that must wait for the work
    /// under way (one save or export at a time; nothing replaces the
    /// project while it is saved), saying so. Closing is left for the app,
    /// which waits.
    pub(super) fn hold_file_actions(&mut self, actions: &mut Vec<WorkbenchAction>) {
        if self.file.busy.is_none() {
            return;
        }
        let before = actions.len();
        actions.retain(|action| {
            !matches!(
                action,
                WorkbenchAction::Save { .. }
                    | WorkbenchAction::Open { .. }
                    | WorkbenchAction::Restore { .. }
                    | WorkbenchAction::Export(_)
                    | WorkbenchAction::ExportToTarget(_)
                    | WorkbenchAction::ExportKit(_)
                    | WorkbenchAction::Import { .. }
            )
        });
        if actions.len() != before {
            // Whatever waited for a save that won't happen now doesn't
            // happen either.
            self.file.after_save = None;
            self.say_still_busy();
        }
    }

    /// The app refused to start more file work while some is under way:
    /// says so, and whatever waited for a save doesn't happen.
    pub fn file_work_refused(&mut self) {
        self.file.after_save = None;
        self.say_still_busy();
    }

    /// The status bar says the work under way must finish first.
    fn say_still_busy(&mut self) {
        if let Some(work) = &self.file.busy {
            self.note = Some(crate::trf!("{} Try again when it's done.", work.doing()));
        }
    }
}
