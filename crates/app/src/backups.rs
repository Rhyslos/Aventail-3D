//! Backup copies for the workbench ([decision 0036](../../../docs/decisions/0036-autosave.md)):
//! where they go, writing them on another thread, and restoring or
//! discarding what a crash left behind. The workbench decides when; this
//! does the file work (`fileio::autosave`) and reports back.

use crate::branding;
use fileio::autosave::{self, Leftover, Session};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::SystemTime;
use ui::{RecoveryCopy, RecoveryOffer, Workbench};

/// Where backup copies go: the app's own folder, so project folders stay
/// tidy and unsaved scenes are covered too. On Windows that's the *local*
/// application data (`%LOCALAPPDATA%`), not the roaming one: copies of
/// heavy scenes are large and shouldn't travel between computers. On
/// macOS and Linux, the same folder as the app's other data.
pub fn folder() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .map(|p| p.join(branding::APP_ID))
    } else {
        eframe::storage_dir(branding::APP_ID)
    };
    base.map(|b| b.join("autosave"))
}

/// The running app's backups.
#[derive(Debug, Default)]
pub struct Backups {
    /// This run's session; `None` if no folder could be made (backups are
    /// then off, and the workbench is told why).
    session: Option<Session>,
    /// What earlier runs left behind, until restored or discarded.
    leftovers: Vec<Leftover>,
    /// A copy being written.
    writing: Option<JoinHandle<std::io::Result<PathBuf>>>,
    /// The project the session has noted.
    project: Option<PathBuf>,
}

/// What a leftover looks like to the workbench.
fn offer(leftover: &Leftover) -> RecoveryOffer {
    RecoveryOffer {
        folder: leftover.folder.clone(),
        project: leftover.project.clone(),
        copies: leftover
            .copies
            .iter()
            .map(|c| RecoveryCopy {
                path: c.path.clone(),
                saved: c.saved,
            })
            .collect(),
    }
}

impl Backups {
    /// Starts backups in `folder`: finds what earlier runs left behind
    /// (offering it to the workbench) and starts this run's session.
    pub fn start(folder: Option<&Path>, workbench: &mut Workbench) -> Self {
        let Some(folder) = folder else {
            return Self::default();
        };
        let leftovers = autosave::leftovers(folder);
        workbench.offer_recovery(leftovers.iter().map(offer).collect());
        workbench.set_autosave_folder(Some(folder.to_path_buf()));
        let session = match Session::start(folder) {
            Ok(session) => Some(session),
            Err(error) => {
                workbench.autosave_failed(&format!(
                    "the backup folder {} can't be used ({error})",
                    folder.display()
                ));
                None
            }
        };
        Self {
            session,
            leftovers,
            writing: None,
            project: None,
        }
    }

    /// Writes a backup copy of `document` on another thread, keeping the
    /// newest `keep`.
    pub fn write(&mut self, document: scene::Document, keep: usize, workbench: &mut Workbench) {
        let Some(session) = &self.session else {
            workbench.autosave_failed("there is no folder for backup copies");
            return;
        };
        let folder = session.folder().to_path_buf();
        self.writing = Some(std::thread::spawn(move || {
            let bytes = fileio::project::to_bytes(&document).map_err(std::io::Error::other)?;
            autosave::write_copy_in(&folder, &bytes, keep)
        }));
    }

    /// Reports a finished copy to the workbench, and notes the project the
    /// workbench has open (for recovery to name it). Called every frame.
    pub fn update(&mut self, workbench: &mut Workbench) {
        if self.writing.as_ref().is_some_and(JoinHandle::is_finished)
            && let Some(handle) = self.writing.take()
        {
            match handle.join() {
                Ok(Ok(_)) => workbench.autosaved(),
                Ok(Err(error)) => workbench.autosave_failed(&error.to_string()),
                Err(_) => workbench.autosave_failed("the backup stopped unexpectedly"),
            }
        }
        let project = workbench.project_path().map(Path::to_path_buf);
        if project != self.project
            && let Some(session) = &self.session
        {
            let _ = session.set_project(project.as_deref());
            self.project = project;
        }
    }

    /// Restores `copy` from the leftover backups in `folder`: the
    /// workbench gets the work back as unsaved, the copy becomes this
    /// run's newest backup straight away, and the leftover is deleted.
    pub fn restore(
        &mut self,
        folder: &Path,
        copy: &Path,
        project: Option<PathBuf>,
        keep: usize,
        workbench: &mut Workbench,
    ) {
        let when = std::fs::metadata(copy)
            .and_then(|m| m.modified())
            .map_or_else(
                |_| "a backup copy".to_string(),
                |saved| ui::ago(saved, SystemTime::now()),
            );
        match fileio::project::open(copy) {
            Ok(document) => {
                if let Some(session) = &self.session {
                    let _ = session.adopt(copy, keep);
                }
                workbench.recovered(document, project, &when);
                self.discard(folder);
            }
            Err(error) => workbench.recovery_failed(&error.to_string()),
        }
    }

    /// Deletes the leftover backups in `folder`.
    pub fn discard(&mut self, folder: &Path) {
        if let Some(at) = self.leftovers.iter().position(|l| l.folder == folder) {
            let leftover = self.leftovers.remove(at);
            let _ = autosave::discard(&leftover);
        }
    }

    /// The app is closing normally: this run's copies are deleted (the
    /// work was saved, or deliberately thrown away). Waits for a copy
    /// being written first.
    pub fn end(&mut self) {
        if let Some(handle) = self.writing.take() {
            let _ = handle.join();
        }
        if let Some(session) = self.session.take() {
            let _ = session.end();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("app-backups-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    fn workbench() -> Workbench {
        Workbench::new(crate::app_info(), None)
    }

    fn wait_for_copy(backups: &mut Backups, workbench: &mut Workbench) {
        let start = std::time::Instant::now();
        while backups.writing.is_some() {
            backups.update(workbench);
            assert!(start.elapsed().as_secs() < 30, "the copy was never written");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn the_folder_is_the_apps_own() {
        if let Some(folder) = folder() {
            assert!(folder.ends_with("autosave"));
            assert!(folder.to_string_lossy().contains(branding::APP_ID));
        }
    }

    #[test]
    fn a_crash_leaves_copies_that_the_next_run_offers_and_restores() {
        let root = temporary("crash");
        // First run: work, a backup copy, then a crash (no `end`).
        let mut first_bench = workbench();
        let mut first = Backups::start(Some(&root), &mut first_bench);
        let mut doc = scene::Document::new();
        doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        first.write(doc.clone(), 5, &mut first_bench);
        wait_for_copy(&mut first, &mut first_bench);
        drop(first);

        // Second run: the work is offered.
        let mut bench = workbench();
        let mut second = Backups::start(Some(&root), &mut bench);
        assert!(bench.is_asking_to_recover());
        assert_eq!(second.leftovers.len(), 1);
        let leftover = second.leftovers[0].clone();
        second.restore(
            &leftover.folder,
            &leftover.copies[0].path,
            Some(PathBuf::from("/work/crate.avtl")),
            5,
            &mut bench,
        );
        assert_eq!(
            bench.document().scene().children(None).len(),
            1,
            "the cube is back"
        );
        assert!(bench.has_unsaved_changes());
        assert_eq!(bench.project_path(), Some(Path::new("/work/crate.avtl")));
        assert!(!leftover.folder.exists(), "the leftover is gone");
        // The restored work is this run's newest backup already.
        let own = second.session.as_ref().unwrap().folder().to_path_buf();
        assert!(own.join("copy-1.avtl").exists());
        // A normal end tidies this run's folder away.
        second.end();
        assert!(!own.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discarding_and_a_missing_folder_are_handled() {
        let root = temporary("discard");
        let mut bench = workbench();
        let mut first = Backups::start(Some(&root), &mut bench);
        first.write(scene::Document::new(), 5, &mut bench);
        wait_for_copy(&mut first, &mut bench);
        drop(first);
        let mut bench = workbench();
        let mut second = Backups::start(Some(&root), &mut bench);
        let folder = second.leftovers[0].folder.clone();
        second.discard(&folder);
        assert!(!folder.exists());
        // A damaged copy is reported, and kept.
        second.restore(&folder, &root.join("missing.avtl"), None, 5, &mut bench);
        second.end();
        // No folder at all: backups are off, and writing says why.
        let mut none = Backups::start(None, &mut bench);
        none.write(scene::Document::new(), 5, &mut bench);
        none.update(&mut bench);
        none.end();
        let _ = std::fs::remove_dir_all(&root);
    }
}
