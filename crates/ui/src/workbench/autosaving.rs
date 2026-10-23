//! Autosave and recovery from the workbench's side: the app's autosave
//! settings and folder, the messages it sends back after each autosave,
//! and asking whether to recover work left by a crash.

use super::{Workbench, WorkbenchAction};
use crate::autosave::{AutosaveSettings, RecoveryAnswer, RecoveryOffer, Tick};
use scene::Document;
use std::path::PathBuf;

impl Workbench {
    /// The workbench with remembered autosave settings restored.
    #[must_use]
    pub fn with_autosave(mut self, settings: AutosaveSettings) -> Self {
        self.autosaver.settings = settings.clamped();
        self
    }

    /// The autosave settings, for remembering between sessions.
    pub fn autosave_settings(&self) -> AutosaveSettings {
        self.autosaver.settings
    }

    /// Where the app keeps backup copies, to show in Settings.
    pub fn set_autosave_folder(&mut self, folder: Option<PathBuf>) {
        self.autosave_folder = folder;
    }

    /// Work left behind by a crash: the workbench asks what to do with it
    /// (one at a time, newest first).
    pub fn offer_recovery(&mut self, offers: Vec<RecoveryOffer>) {
        self.recovery
            .offers
            .extend(offers.into_iter().filter(|o| !o.copies.is_empty()));
    }

    /// Whether the question about work left by a crash is showing.
    pub fn is_asking_to_recover(&self) -> bool {
        !self.recovery.offers.is_empty()
    }

    /// The app wrote the backup copy it was asked for.
    pub fn autosaved(&mut self) {
        self.autosaver.finished(true);
    }

    /// Writing a backup copy failed: says why (once), and tries again
    /// after the next interval.
    pub fn autosave_failed(&mut self, reason: &str) {
        self.autosaver.finished(false);
        self.note = Some(crate::trf!("Couldn't save a backup copy: {}.", reason));
    }

    /// The app brought back `document` from a backup copy, for the
    /// project at `project` (if it had been saved): it replaces the
    /// current scene as unsaved work, so saving keeps it.
    pub fn recovered(&mut self, document: Document, project: Option<PathBuf>, when: &str) {
        self.replace_document(document);
        // Unsaved: the project file (if any) doesn't have this work.
        self.file.saved_revision = u64::MAX;
        self.file.path = project;
        // The app already kept it as this run's newest backup copy, and
        // the timer starts from it (`replace_document`).
        self.note = Some(crate::trf!(
            "Restored your work from {}. Save it to keep it.",
            when
        ));
        self.viewport.frame(crate::Frame::All);
    }

    /// Restoring a backup copy failed: says why. The copies are kept.
    pub fn recovery_failed(&mut self, reason: &str) {
        self.note = Some(crate::trf!("Couldn't restore the backup copy: {}.", reason));
    }

    /// Asks about work left by a crash, and when it is due, has a backup
    /// copy made.
    pub(super) fn autosave_and_recovery(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if let Some((offer, answer)) = self.recovery.show(ctx) {
            let folder = offer.folder.clone();
            match answer {
                RecoveryAnswer::Restore(copy) => actions.push(WorkbenchAction::Restore {
                    folder,
                    copy,
                    project: offer.project,
                }),
                RecoveryAnswer::OpenSaved(path) => {
                    actions.push(WorkbenchAction::Open { path: Some(path) });
                    actions.push(WorkbenchAction::DiscardBackups { folder });
                }
                RecoveryAnswer::Discard => actions.push(WorkbenchAction::DiscardBackups { folder }),
            }
        }
        let now = ctx.input(|i| i.time);
        let revision = self.document.revision();
        match self.autosaver.tick(now, revision) {
            Tick::Idle => {}
            Tick::Wait(seconds) => {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(seconds));
            }
            Tick::Save => {
                self.autosaver.started(revision, now);
                let scene = self.document.scene().clone();
                actions.push(WorkbenchAction::Autosave {
                    document: Box::new(Document::from_scene(scene, self.document.selection())),
                    keep: self.autosaver.settings.copies as usize,
                });
            }
        }
    }
}
