//! Backup copies in the background, and the question after a crash
//! ([decision 0036](../../../docs/decisions/0036-autosave.md)).
//!
//! The workbench decides *when* a backup is due and asks the app to save
//! one (it never touches files itself); the app writes it on another
//! thread with `fileio::autosave`. When the app starts after a crash, it
//! hands the workbench what was left behind, and the workbench asks what
//! to do with it.

use egui::{Key, Modifiers};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// The fewest and most minutes between backups.
pub const MINUTES: std::ops::RangeInclusive<u32> = 1..=30;
/// The fewest and most backup copies kept.
pub const COPIES: std::ops::RangeInclusive<u32> = 1..=20;

/// How backups are made: chosen in Settings → Autosave, remembered by the
/// app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutosaveSettings {
    /// Whether backups are made at all.
    pub on: bool,
    /// Minutes between backups (only when something changed).
    pub minutes: u32,
    /// How many copies are kept, newest first.
    pub copies: u32,
}

impl Default for AutosaveSettings {
    /// On, every 5 minutes, keeping 5 copies (the project lead's choice,
    /// 2026-10-12).
    fn default() -> Self {
        Self {
            on: true,
            minutes: 5,
            copies: 5,
        }
    }
}

impl AutosaveSettings {
    /// The settings within their limits (a damaged settings file can hold
    /// anything).
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            on: self.on,
            minutes: self.minutes.clamp(*MINUTES.start(), *MINUTES.end()),
            copies: self.copies.clamp(*COPIES.start(), *COPIES.end()),
        }
    }

    /// Seconds between backups.
    fn interval(self) -> f64 {
        f64::from(self.clamped().minutes) * 60.0
    }
}

/// What the timer says this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Tick {
    /// Nothing to back up (no changes, or backups are off).
    Idle,
    /// Changes are waiting: a backup is due in this many seconds.
    Wait(f64),
    /// A backup is due now.
    Save,
}

/// Keeps track of when the next backup is due. Backups run on their own
/// clock, separate from saving: every interval after the last backup, if
/// anything changed since that backup, whether or not the project was
/// saved in between (the project lead's choice, 2026-10-12).
#[derive(Debug, Clone, Default)]
pub(crate) struct Autosaver {
    pub(crate) settings: AutosaveSettings,
    /// When the last backup was made (or the document was opened).
    since: Option<f64>,
    /// The document's revision in the last backup (or when it was
    /// opened: an unchanged document needs no backup).
    backed_up: Option<u64>,
    /// `backed_up` before the backup being written, to go back to if it
    /// fails.
    previous: Option<u64>,
    /// A backup is being written.
    writing: bool,
}

impl Autosaver {
    /// Whether a backup is due at `now` (seconds) for a document at
    /// `revision`.
    pub(crate) fn tick(&mut self, now: f64, revision: u64) -> Tick {
        let backed_up = *self.backed_up.get_or_insert(revision);
        let since = *self.since.get_or_insert(now);
        if !self.settings.on || revision == backed_up {
            return Tick::Idle;
        }
        let due = since + self.settings.interval();
        if self.writing {
            Tick::Wait((due - now).max(1.0))
        } else if now >= due {
            Tick::Save
        } else {
            Tick::Wait(due - now)
        }
    }

    /// A backup of `revision` was handed to the app to write at `now`.
    pub(crate) fn started(&mut self, revision: u64, now: f64) {
        self.previous = self.backed_up;
        self.backed_up = Some(revision);
        self.since = Some(now);
        self.writing = true;
    }

    /// The app finished writing it, or failed (then the changes are
    /// tried again after the next interval).
    pub(crate) fn finished(&mut self, ok: bool) {
        self.writing = false;
        if !ok {
            self.backed_up = self.previous;
        }
    }

    /// A different document is open: the clock starts again, and the
    /// document as opened needs no backup.
    pub(crate) fn restart(&mut self) {
        self.since = None;
        self.backed_up = None;
        self.previous = None;
    }
}

/// A backup copy offered for recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCopy {
    /// Its file.
    pub path: PathBuf,
    /// When it was saved.
    pub saved: SystemTime,
}

/// Work left behind by an app that didn't close normally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryOffer {
    /// The folder of backups it left (handed back to the app to restore
    /// from or discard).
    pub folder: PathBuf,
    /// The project it had open, if that had been saved somewhere.
    pub project: Option<PathBuf>,
    /// Its copies, newest first.
    pub copies: Vec<RecoveryCopy>,
    /// Whether a copy was left out because restoring it closed the app.
    pub left_out: bool,
}

/// "3 minutes ago", for when a copy was saved.
pub fn ago(saved: SystemTime, now: SystemTime) -> String {
    let seconds = now.duration_since(saved).map_or(0, |d| d.as_secs());
    let (n, unit) = match seconds {
        0..60 => return "just now".to_string(),
        60..3_600 => (seconds / 60, "minute"),
        3_600..86_400 => (seconds / 3_600, "hour"),
        _ => (seconds / 86_400, "day"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

/// What the recovery question was answered with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecoveryAnswer {
    /// Bring this copy back.
    Restore(PathBuf),
    /// Open the project as it was last saved, and let the copies go.
    OpenSaved(PathBuf),
    /// Let the copies go.
    Discard,
}

/// The question asked after a crash, one leftover at a time.
#[derive(Debug, Clone, Default)]
pub(crate) struct RecoveryQuestion {
    pub(crate) offers: Vec<RecoveryOffer>,
    /// Which copy of the first offer is chosen.
    chosen: usize,
}

/// The window's title (also how tests find it).
pub(crate) const RECOVERY_TITLE: &str = "Recover unsaved work?";

impl RecoveryQuestion {
    /// Asks about the first offer, if any. Returns the offer and the
    /// answer once one is given.
    pub(crate) fn show(&mut self, ctx: &egui::Context) -> Option<(RecoveryOffer, RecoveryAnswer)> {
        let offer = self.offers.first()?.clone();
        let now = SystemTime::now();
        let mut answer = None;
        let chosen = &mut self.chosen;
        crate::dialog::Dialog::new(RECOVERY_TITLE, 480.0)
            .id("recover_unsaved_work")
            .modal()
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    let what = offer.project.as_ref().map_or_else(
                        || "an unsaved scene".to_string(),
                        |p| {
                            p.file_name().map_or_else(
                                || p.display().to_string(),
                                |n| n.to_string_lossy().into(),
                            )
                        },
                    );
                    ui.label(format!(
                        "The app closed without saving your work on {what}. Backup copies were \
                         kept:"
                    ));
                    ui.add_space(2.0);
                    for (i, copy) in offer.copies.iter().enumerate() {
                        let mut text = format!("Saved {}", ago(copy.saved, now));
                        if i == 0 {
                            text.push_str(" (newest)");
                        }
                        if ui.radio(*chosen == i, text).clicked() {
                            *chosen = i;
                        }
                    }
                    if offer.left_out {
                        crate::dialog::help(
                            ui,
                            "One newer copy is left out: restoring it closed the app last time.",
                        );
                    }
                    crate::dialog::help(ui, "Enter: restore the chosen copy");
                });
                d.footer(None, |ui| {
                    if crate::dialog::main_button(ui, "Restore")
                        .on_hover_text(
                            "Bring the chosen copy back. It opens as unsaved work: save it to \
                             keep it.",
                        )
                        .clicked()
                    {
                        answer = offer
                            .copies
                            .get(*chosen)
                            .map(|c| RecoveryAnswer::Restore(c.path.clone()));
                    }
                    if let Some(project) = &offer.project
                        && crate::dialog::plain_button(ui, "Open as saved")
                            .on_hover_text(
                                "Open the project as you last saved it, and delete the backup \
                                 copies.",
                            )
                            .clicked()
                    {
                        answer = Some(RecoveryAnswer::OpenSaved(project.clone()));
                    }
                    if crate::dialog::plain_button(ui, "Discard")
                        .on_hover_text("Delete the backup copies for good and start afresh.")
                        .clicked()
                    {
                        answer = Some(RecoveryAnswer::Discard);
                    }
                });
            });
        if answer.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            answer = offer
                .copies
                .get(self.chosen)
                .map(|c| RecoveryAnswer::Restore(c.path.clone()));
        }
        let answer = answer?;
        self.offers.remove(0);
        self.chosen = 0;
        Some((offer, answer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn backups_run_on_their_own_clock_when_something_changed() {
        let mut timer = Autosaver::default();
        // Opened at 0 with revision 4: nothing to back up.
        assert_eq!(timer.tick(0.0, 4), Tick::Idle);
        assert_eq!(timer.tick(1_000.0, 4), Tick::Idle);
        // A change after a long quiet spell: due at once.
        assert_eq!(timer.tick(1_001.0, 5), Tick::Save);
        timer.started(5, 1_001.0);
        // While it is written, nothing more is started.
        assert!(matches!(timer.tick(1_002.0, 6), Tick::Wait(_)));
        timer.finished(true);
        // Busy work: the next one is due an interval after the last.
        assert_eq!(timer.tick(1_100.0, 9), Tick::Wait(201.0));
        assert_eq!(timer.tick(1_301.0, 9), Tick::Save);
        timer.started(9, 1_301.0);
        timer.finished(true);
        // Backed up and unchanged since: idle.
        assert_eq!(timer.tick(5_000.0, 9), Tick::Idle);
    }

    #[test]
    fn saving_does_not_reset_the_clock() {
        // The timer doesn't know about saving at all: a change saved with
        // Ctrl + S still gets its backup on time.
        let mut timer = Autosaver::default();
        timer.tick(0.0, 1);
        assert_eq!(timer.tick(10.0, 2), Tick::Wait(290.0));
        // (saved here, revision 2 is in the project file)
        assert_eq!(timer.tick(300.0, 2), Tick::Save);
    }

    #[test]
    fn backups_can_be_switched_off_and_failures_try_again() {
        let mut timer = Autosaver {
            settings: AutosaveSettings {
                on: false,
                ..AutosaveSettings::default()
            },
            ..Autosaver::default()
        };
        assert_eq!(timer.tick(0.0, 0), Tick::Idle);
        assert_eq!(timer.tick(10.0, 5), Tick::Idle, "off");
        timer.settings.on = true;
        timer.settings.minutes = 1;
        assert_eq!(timer.tick(60.0, 5), Tick::Save);
        timer.started(5, 60.0);
        timer.finished(false);
        assert_eq!(timer.tick(61.0, 5), Tick::Wait(59.0), "tries again");
        assert_eq!(timer.tick(120.0, 5), Tick::Save);
        // Another document: its state when opened needs no backup.
        timer.restart();
        assert_eq!(timer.tick(130.0, 7), Tick::Idle);
        assert_eq!(timer.tick(140.0, 8), Tick::Wait(50.0));
    }

    #[test]
    fn settings_stay_within_their_limits() {
        let wild = AutosaveSettings {
            on: true,
            minutes: 0,
            copies: 500,
        }
        .clamped();
        assert_eq!((wild.minutes, wild.copies), (1, 20));
        let standard = AutosaveSettings::default();
        assert_eq!(
            (standard.on, standard.minutes, standard.copies),
            (true, 5, 5)
        );
        // Old settings files without these fields read as the standard.
        let read: AutosaveSettings = ron::from_str("(on: false)").unwrap();
        assert_eq!(read.minutes, 5);
        assert!(!read.on);
    }

    #[test]
    fn times_read_naturally() {
        let now = SystemTime::now();
        let before = |s| now - Duration::from_secs(s);
        assert_eq!(ago(before(20), now), "just now");
        assert_eq!(ago(before(60), now), "1 minute ago");
        assert_eq!(ago(before(7 * 60 + 5), now), "7 minutes ago");
        assert_eq!(ago(before(2 * 3_600), now), "2 hours ago");
        assert_eq!(ago(before(3 * 86_400), now), "3 days ago");
        assert_eq!(
            ago(now + Duration::from_secs(9), now),
            "just now",
            "clock skew"
        );
    }
}
