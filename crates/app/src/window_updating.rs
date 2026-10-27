//! The window's part in updates (decisions [0169](../../../docs/decisions/0169-installer-and-updates.md),
//! [0172](../../../docs/decisions/0172-update-popup-and-splash-check.md) and
//! [0173](../../../docs/decisions/0173-update-question-on-the-splash-screen.md)):
//! starting and stopping the looking as the setting says, the splash
//! screen's wait and question, and handing what is found to the
//! workbench.
//!
//! - **On the splash screen** the first look runs while it says *Looking
//!   for a new version…*. A newer build that isn't skipped is asked about
//!   there, before the app opens: **Update now** downloads it (the bar
//!   filling) and installs it, opening the app again on the new version;
//!   **Open without updating** opens the app, with **Skip this version**
//!   not asking about that build again.
//! - **While the app runs**, a newer build found by a later look is
//!   downloaded at once and offered in the workbench's window (Restart
//!   now, Later, Skip this version).

use crate::updates::{self, Found, PendingInstall, Release, Updater};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use ui::Workbench;
use words::{tr, trf};

/// The longest the splash screen waits for the first look's answer; the
/// look itself gives up a little sooner (`updates::ASK_TIMEOUT_SECONDS`).
pub(crate) const SPLASH_WAITS_FOR_LOOK: Duration =
    Duration::from_secs(updates::ASK_TIMEOUT_SECONDS as u64 + 2);

/// What the splash screen shows for updates.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SplashUpdate {
    /// Nothing to wait for: the app may open.
    Done,
    /// Waiting, saying this, with how far a download has got.
    Waiting(String, Option<f32>),
    /// A newer build was found: ask about it.
    Asking(Release),
}

/// Updates while the app starts and runs.
#[derive(Default)]
pub(crate) struct Updating {
    /// Looking in the background, while that is wanted and possible.
    updater: Option<Updater>,
    /// Whether looking was wanted when last seen, to notice the setting
    /// change.
    wanted: Option<bool>,
    /// The setup program downloaded for a newer build.
    pub(crate) ready: Option<PathBuf>,
    /// When the first look began, while the splash screen waits for it;
    /// `None` once it is answered or given up on.
    pub(crate) first_look: Option<Instant>,
    /// A newer build being downloaded, the file it grows in and since when.
    pub(crate) downloading: Option<(Release, PathBuf, Instant)>,
    /// The newer build the splash screen asks about.
    pub(crate) question: Option<Release>,
    /// The splash screen's Skip this version box.
    pub(crate) skip_box: bool,
    /// Update now was chosen on the splash screen: download, install and
    /// open again.
    pub(crate) updating_now: Option<Release>,
    /// A build declined for this run (Open without updating): not offered
    /// again until the next start.
    declined: u32,
}

impl Updating {
    /// Starts or stops looking as the setting says, and hands what was
    /// found to the workbench. Returns whether to look again soon.
    pub(crate) fn update(&mut self, workbench: &mut Workbench) -> bool {
        let wanted = workbench.checks_for_updates();
        if self.wanted != Some(wanted) {
            self.wanted = Some(wanted);
            self.updater = None;
            let exe = std::env::current_exe().unwrap_or_default();
            let build = updates::build_number();
            match (
                updates::why_not(build, &exe, cfg!(windows), wanted),
                build,
                updates::folder(),
            ) {
                (Some(why), _, _) => workbench.set_update_status(why),
                _ if self.ready.is_some() => {}
                (None, Some(build), Some(folder)) => {
                    self.updater = Some(Updater::start(Box::new(updates::Curl), build, folder));
                    self.first_look = Some(Instant::now());
                    workbench.set_update_status(tr("Looking for a new version…").to_string());
                }
                (None, _, None) => workbench.set_update_status(
                    tr("There's no folder to download new versions into.").to_string(),
                ),
                (None, None, _) => {}
            }
        }
        let Some(updater) = &self.updater else {
            return false;
        };
        updater.set_quiet(workbench.skipped_update().unwrap_or(0).max(self.declined));
        for found in updater.poll() {
            if !matches!(found, Found::Downloading { .. }) {
                self.first_look = None;
                self.downloading = None;
            }
            match found {
                Found::Downloading { release, part } => {
                    workbench.set_update_status(trf!("Downloading build {}…", release.build));
                    self.downloading = Some((release, part, Instant::now()));
                }
                Found::Newer(release) => {
                    workbench.set_update_status(trf!("Build {} is available.", release.build));
                    self.question = Some(release);
                }
                Found::UpToDate(newest) => {
                    workbench
                        .set_update_status(trf!("Up to date: build {} is the newest.", newest));
                }
                Found::Skipped(build) if workbench.skipped_update() == Some(build) => {
                    workbench.set_update_status(trf!(
                        "Build {} is the newest, and you skipped it; you'll be asked when a \
                         newer one comes out.",
                        build
                    ));
                }
                Found::Skipped(build) => workbench.set_update_status(trf!(
                    "Build {} is the newest; you chose to open without it, so it's offered again \
                     at the next start.",
                    build
                )),
                Found::Failed(why) => {
                    if let Some(release) = self.updating_now.take() {
                        workbench.set_note(trf!(
                            "Build {} couldn't be downloaded ({}); the app opened as it was. It \
                             asks again at the next start.",
                            release.build,
                            why
                        ));
                    }
                    workbench.set_update_status(trf!(
                        "Couldn't look for a new version ({}). It tries again in a few hours.",
                        why
                    ));
                }
                Found::Ready { release, installer } => {
                    workbench.set_update_status(trf!(
                        "Build {} is downloaded and ready to install.",
                        release.build
                    ));
                    if self.updating_now.is_some() {
                        // Update now on the splash screen: install, open again.
                        updates::set_pending(PendingInstall {
                            installer: installer.clone(),
                            reopen: true,
                        });
                    } else {
                        workbench.offer_update(ui::UpdateOffer {
                            build: release.build,
                            notes_url: release.page,
                        });
                    }
                    self.ready = Some(installer);
                }
            }
        }
        self.ready.is_none()
    }

    /// The person answered the splash screen's question.
    pub(crate) fn answer(&mut self, answer: ui::splash::UpdateAnswer, workbench: &mut Workbench) {
        let Some(release) = self.question.take() else {
            return;
        };
        match answer {
            ui::splash::UpdateAnswer::Update => {
                if let Some(updater) = &self.updater {
                    updater.download(release.clone());
                }
                self.updating_now = Some(release);
            }
            ui::splash::UpdateAnswer::Open if self.skip_box => {
                workbench.skip_update(Some(release.build));
            }
            ui::splash::UpdateAnswer::Open => self.declined = release.build,
        }
    }

    /// Whether the app should close now to install the update chosen on
    /// the splash screen (it is downloaded).
    pub(crate) fn install_now(&self) -> bool {
        self.updating_now.is_some() && self.ready.is_some()
    }

    /// The update the workbench offered is skipped (Skip this version in
    /// its window): nothing installs.
    pub(crate) fn skipped(&mut self) {
        self.ready = None;
        let _ = updates::take_pending();
    }

    /// What the splash screen shows for updates.
    pub(crate) fn splash(&self) -> SplashUpdate {
        if let Some(release) = &self.question {
            return SplashUpdate::Asking(release.clone());
        }
        if let Some(release) = &self.updating_now {
            if self.ready.is_some() {
                return SplashUpdate::Waiting(
                    trf!("Installing build {}…", release.build),
                    Some(1.0),
                );
            }
            let share = self
                .downloading
                .as_ref()
                .map_or(0.0, |(release, part, _)| updates::progress(part, release));
            return SplashUpdate::Waiting(
                trf!(
                    "Downloading build {}… {}%",
                    release.build,
                    (share * 100.0).round() as u32
                ),
                Some(share),
            );
        }
        match self.first_look {
            Some(began) if began.elapsed() <= SPLASH_WAITS_FOR_LOOK => {
                SplashUpdate::Waiting(tr("Looking for a new version…").to_string(), None)
            }
            _ => SplashUpdate::Done,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(build: u32) -> Release {
        Release {
            build,
            page: format!("https://example.com/build-{build}"),
            installer_url: String::new(),
            size: 100,
        }
    }

    #[test]
    fn the_splash_waits_only_while_a_look_is_under_way() {
        let mut updating = Updating::default();
        assert_eq!(updating.splash(), SplashUpdate::Done, "no looking: no wait");
        updating.first_look = Some(Instant::now());
        assert_eq!(
            updating.splash(),
            SplashUpdate::Waiting("Looking for a new version…".into(), None)
        );
        // A look that has run past its time doesn't hold the app back.
        updating.first_look = Instant::now().checked_sub(SPLASH_WAITS_FOR_LOOK * 2);
        assert_eq!(updating.splash(), SplashUpdate::Done);
    }

    #[test]
    fn a_newer_build_is_asked_about_and_update_now_shows_the_download() {
        let mut updating = Updating {
            question: Some(release(7)),
            ..Updating::default()
        };
        assert_eq!(updating.splash(), SplashUpdate::Asking(release(7)));
        // Update now (no updater here: only the splash screen's side).
        updating.updating_now = updating.question.take();
        assert_eq!(
            updating.splash(),
            SplashUpdate::Waiting("Downloading build 7… 0%".into(), Some(0.0))
        );
        let dir = std::env::temp_dir().join(format!("splash-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let part = dir.join("setup.part");
        std::fs::write(&part, [0u8; 25]).unwrap();
        updating.downloading = Some((release(7), part, Instant::now()));
        assert_eq!(
            updating.splash(),
            SplashUpdate::Waiting("Downloading build 7… 25%".into(), Some(0.25))
        );
        assert!(!updating.install_now());
        updating.ready = Some(dir.join("setup.exe"));
        assert!(updating.install_now());
        assert_eq!(
            updating.splash(),
            SplashUpdate::Waiting("Installing build 7…".into(), Some(1.0))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
