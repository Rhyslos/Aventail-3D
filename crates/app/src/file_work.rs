//! Saving and exporting on another thread ([decision 0163](../../../docs/decisions/0163-saving-and-exporting-in-the-background.md)),
//! as backup copies already are ([`crate::backups`]). The work gets a copy
//! of the document (quick: shapes are shared, and what the scene has
//! worked out comes along), so the window keeps drawing and the user can
//! keep working; changes made meanwhile simply aren't in that save. One
//! piece of work at a time: the workbench holds back another save or
//! export until it is done, and closing the window waits for it.

use crate::project::{self, SaveOutcome};
use std::path::PathBuf;
use std::thread::JoinHandle;
use ui::{ExportRequest, FileWork, KitRequest, TargetRequest, Workbench};

/// What a piece of file work came to, for the workbench.
enum Done {
    Saved {
        path: PathBuf,
        /// The document's revision when the copy was taken.
        revision: u64,
        outcome: SaveOutcome,
    },
    Exported {
        request: Box<ExportRequest>,
        path: PathBuf,
        result: Result<fileio::export::ExportSummary, String>,
    },
    ToGame(ui::TargetOutcome),
    Kit(ui::KitOutcome),
}

impl Done {
    /// Whether it worked (a failure keeps the window open, so it is seen).
    fn worked(&self) -> bool {
        match self {
            Self::Saved { outcome, .. } => outcome.result.is_ok(),
            Self::Exported { result, .. } => result.is_ok(),
            Self::ToGame(outcome) => outcome.is_ok(),
            Self::Kit(outcome) => outcome.is_ok(),
        }
    }

    /// Tells the workbench how it went.
    fn report(self, workbench: &mut Workbench) {
        match self {
            Self::Saved {
                path,
                revision,
                outcome,
            } => project::report_saved(workbench, path, revision, outcome),
            Self::Exported {
                request,
                path,
                result,
            } => project::report_exported(workbench, &request, path, result),
            Self::ToGame(outcome) => workbench.target_exported(outcome),
            Self::Kit(outcome) => workbench.kit_exported(outcome),
        }
    }
}

/// What the window does once the work under way is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AfterWork {
    /// Nothing.
    #[default]
    Stay,
    /// Try closing again: the window's close button was pressed, so the
    /// workbench asks about unsaved changes (made during the save) first.
    AskToClose,
    /// Close: unsaved changes were already dealt with.
    Close,
}

/// The save or export under way, if any.
#[derive(Default)]
pub struct FileJobs {
    running: Option<JoinHandle<Done>>,
    after: AfterWork,
}

/// A copy of the workbench's document for work on another thread.
fn copy_of(workbench: &Workbench) -> scene::Document {
    let document = workbench.document();
    scene::Document::from_scene(document.scene().copy_with_results(), document.selection())
}

impl FileJobs {
    /// Whether a save or export is under way.
    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// Starts `job` on another thread, the workbench saying it does `work`;
    /// refused (the workbench says why) while other work is under way.
    fn start(
        &mut self,
        workbench: &mut Workbench,
        work: FileWork,
        job: impl FnOnce() -> Done + Send + 'static,
    ) {
        if self.busy() {
            workbench.file_work_refused();
            return;
        }
        workbench.file_work_started(work);
        self.running = Some(std::thread::spawn(job));
    }

    /// Saves the workbench's document to `path` on another thread (and
    /// exports it into its game project folder afterwards, if it does that
    /// on every save, decision 0076).
    pub fn save(&mut self, workbench: &mut Workbench, path: PathBuf) {
        let export = workbench.save_export_request(&path);
        let document = copy_of(workbench);
        let revision = workbench.document().revision();
        let work = FileWork::Saving(path.clone());
        self.start(workbench, work, move || {
            let outcome = project::save_work(&document, &path, export.as_ref());
            Done::Saved {
                path,
                revision,
                outcome,
            }
        });
    }

    /// Exports the workbench's document to `path` as `request` says, on
    /// another thread.
    pub fn export(
        &mut self,
        workbench: &mut Workbench,
        request: Box<ExportRequest>,
        path: PathBuf,
    ) {
        let document = copy_of(workbench);
        let work = FileWork::Exporting(path.clone());
        self.start(workbench, work, move || {
            let result = project::export_work(&document, &request, &path);
            Done::Exported {
                request,
                path,
                result,
            }
        });
    }

    /// Exports the workbench's document into its game project folder as
    /// `request` says, on another thread.
    pub fn export_to_game(&mut self, workbench: &mut Workbench, request: Box<TargetRequest>) {
        let document = copy_of(workbench);
        self.start(workbench, FileWork::ExportingToGame, move || {
            Done::ToGame(project::export_into_work(&document, &request))
        });
    }

    /// Exports the kit pieces of `request` into `folder`, on another
    /// thread.
    pub fn export_kit(
        &mut self,
        workbench: &mut Workbench,
        request: Box<KitRequest>,
        folder: PathBuf,
    ) {
        let document = copy_of(workbench);
        self.start(workbench, FileWork::ExportingKit, move || {
            Done::Kit(project::kit_work(&document, &request, &folder))
        });
    }

    /// The window is to close once the work under way is done: `agreed`
    /// when unsaved changes were already dealt with (File → Quit), else
    /// the workbench asks about them then (the window's close button).
    pub fn close_when_done(&mut self, workbench: &mut Workbench, agreed: bool) {
        self.after = match (self.after, agreed) {
            (AfterWork::Close, _) | (_, true) => AfterWork::Close,
            _ => AfterWork::AskToClose,
        };
        workbench.closing_after_file_work();
    }

    /// Reports finished work to the workbench, and says what the window
    /// does now. Called every frame.
    pub fn update(&mut self, workbench: &mut Workbench) -> AfterWork {
        if !self.running.as_ref().is_some_and(JoinHandle::is_finished) {
            return AfterWork::Stay;
        }
        let after = std::mem::take(&mut self.after);
        match self.running.take().map(JoinHandle::join) {
            Some(Ok(done)) => {
                let worked = done.worked();
                done.report(workbench);
                if worked { after } else { AfterWork::Stay }
            }
            Some(Err(_)) => {
                stopped(workbench);
                AfterWork::Stay
            }
            None => AfterWork::Stay,
        }
    }

    /// Waits for the work under way, if any, and reports it: the app is
    /// closing.
    pub fn finish(&mut self, workbench: &mut Workbench) {
        if let Some(handle) = self.running.take() {
            match handle.join() {
                Ok(done) => done.report(workbench),
                Err(_) => stopped(workbench),
            }
        }
    }
}

/// The work stopped unexpectedly (a crash on its thread): says so.
fn stopped(workbench: &mut Workbench) {
    let why = words::tr("it stopped unexpectedly");
    match workbench.file_work() {
        Some(FileWork::Saving(_)) => workbench.save_failed(why),
        Some(FileWork::Exporting(_)) => workbench.export_failed(why),
        Some(FileWork::ExportingToGame) => workbench.target_exported(Err(why.to_string())),
        Some(FileWork::ExportingKit) | None => workbench.kit_exported(Err(why.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workbench_with_a_cube() -> Workbench {
        let mut workbench = Workbench::new(crate::app_info(), None);
        let mut doc = scene::Document::new();
        doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        workbench.opened(doc, PathBuf::from("crate.avtl"));
        workbench
    }

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("app-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// Runs frames until the work is done; what the window does then.
    fn wait(jobs: &mut FileJobs, workbench: &mut Workbench) -> AfterWork {
        for _ in 0..2_000 {
            if !jobs.busy() {
                return AfterWork::Stay;
            }
            let after = jobs.update(workbench);
            if !jobs.busy() {
                return after;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the work never finished");
    }

    /// A change to the scene, as the user could make it: a triangle
    /// imported.
    fn a_change(workbench: &mut Workbench) {
        let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let import = fileio::obj::read_obj(
            text,
            fileio::export::FileUnits::Metres,
            fileio::export::UpAxis::Y,
        )
        .unwrap();
        workbench.imported(std::path::Path::new("triangle.obj"), import);
    }

    #[test]
    fn a_save_on_another_thread_is_reported_when_done() {
        let folder = folder("save-thread");
        let path = folder.join("crate.avtl");
        let mut workbench = workbench_with_a_cube();
        let mut jobs = FileJobs::default();
        jobs.save(&mut workbench, path.clone());
        assert!(jobs.busy());
        assert_eq!(workbench.file_work(), Some(&FileWork::Saving(path.clone())));
        // A second save waits: refused, and said so.
        jobs.save(&mut workbench, folder.join("other.avtl"));
        assert_eq!(wait(&mut jobs, &mut workbench), AfterWork::Stay);
        assert_eq!(workbench.file_work(), None);
        assert!(!workbench.has_unsaved_changes());
        assert_eq!(workbench.project_path(), Some(path.as_path()));
        assert!(fileio::project::open(&path).is_ok());
        assert!(!folder.join("other.avtl").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn changes_made_while_saving_stay_unsaved() {
        let folder = folder("save-edit");
        let path = folder.join("crate.avtl");
        let mut workbench = workbench_with_a_cube();
        let mut jobs = FileJobs::default();
        let revision = workbench.document().revision();
        jobs.save(&mut workbench, path.clone());
        // Reported as saved at the revision it copied, after a change.
        let handle = jobs.running.take().unwrap();
        let done = handle.join().ok().unwrap();
        a_change(&mut workbench);
        assert!(workbench.document().revision() > revision);
        done.report(&mut workbench);
        assert!(
            workbench.has_unsaved_changes(),
            "the triangle isn't saved yet"
        );
        let saved = fileio::project::open(&path).unwrap();
        assert_eq!(saved.scene().walk().len(), 1, "only the cube");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_failed_save_is_reported_and_keeps_the_window_open() {
        let mut workbench = workbench_with_a_cube();
        let mut jobs = FileJobs::default();
        let nowhere = std::env::temp_dir()
            .join(format!("no-such-folder-{}", std::process::id()))
            .join("x.avtl");
        a_change(&mut workbench);
        jobs.save(&mut workbench, nowhere);
        assert!(workbench.note().unwrap().starts_with("Saving x.avtl"));
        jobs.close_when_done(&mut workbench, true);
        assert!(workbench.note().unwrap().contains("closes when it's done"));
        assert_eq!(wait(&mut jobs, &mut workbench), AfterWork::Stay);
        assert!(workbench.has_unsaved_changes());
        assert!(workbench.note().unwrap().starts_with("Couldn't save"));
        assert_eq!(workbench.file_work(), None);
    }

    #[test]
    fn closing_waits_for_an_export_then_closes() {
        let folder = folder("export-thread");
        let mut workbench = workbench_with_a_cube();
        let mut jobs = FileJobs::default();
        let request = Box::new(ExportRequest {
            preset: None,
            format: fileio::export::ExportFormat::Glb,
            options: fileio::export::ExportOptions::default(),
            path: None,
            suggested_name: "crate.glb".into(),
        });
        let path = folder.join("crate.glb");
        jobs.export(&mut workbench, request, path.clone());
        assert_eq!(
            workbench.file_work(),
            Some(&FileWork::Exporting(path.clone()))
        );
        jobs.close_when_done(&mut workbench, false);
        assert_eq!(wait(&mut jobs, &mut workbench), AfterWork::AskToClose);
        assert!(path.exists());
        // The workbench says it was exported.
        assert_eq!(workbench.file_work(), None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn finishing_waits_for_the_work() {
        let folder = folder("finish");
        let path = folder.join("crate.avtl");
        let mut workbench = workbench_with_a_cube();
        let mut jobs = FileJobs::default();
        jobs.save(&mut workbench, path.clone());
        jobs.finish(&mut workbench);
        assert!(!jobs.busy());
        assert!(path.exists());
        assert!(!workbench.has_unsaved_changes());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
