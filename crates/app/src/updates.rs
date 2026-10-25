//! Updates ([decision 0169](../../../docs/decisions/0169-installer-and-updates.md)):
//! an installed copy on Windows looks for a newer build on the project's
//! GitHub Releases page when it starts and every few hours, downloads its
//! setup program in the background, and the workbench offers it (a bar
//! with Restart now and Later). The setup program runs when the app
//! closes, quietly, and opens the app again after Restart now.
//!
//! - Every green change to `main` makes a release tagged `build-<number>`,
//!   with the setup program (`<APP_ID>-setup.exe`) attached
//!   (`.github/workflows/release.yml`). Builds made that way carry their
//!   number (`APP_BUILD` at compile time, [`build_number`]).
//! - Only a copy installed by that setup program updates itself (its
//!   uninstaller sits beside it), with a build number, on Windows, and with
//!   Settings → Interface → Look for new versions on ([`why_not`]). A copy
//!   built by hand or run from a zip never changes itself.
//! - The asking and downloading is done by `curl`, part of Windows since
//!   Windows 10 (1803): nothing to add to the app, and the system's own
//!   certificates. A download goes to a `.part` file first and counts only
//!   once it has the size GitHub gave, so a cut-off download is never run.
//!
//! The work that decides (reading GitHub's answer, comparing builds,
//! naming files) is plain functions, tested on every system; only the
//! fetching and the installing touch the outside world.

use crate::branding;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use words::{tr, trf};

/// How long after starting the first look happens: the window first.
const FIRST_LOOK_AFTER: Duration = Duration::from_secs(10);
/// How often a running app looks again.
const LOOK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// The longest an answer from GitHub may take.
const ASK_TIMEOUT_SECONDS: u32 = 30;
/// The longest a download may take.
const DOWNLOAD_TIMEOUT_SECONDS: u32 = 30 * 60;

/// This build's number, when it was built by the release workflow.
pub fn build_number() -> Option<u32> {
    parse_build(option_env!("APP_BUILD")?)
}

/// A build number written as text (`"412"`), or `None`.
fn parse_build(text: &str) -> Option<u32> {
    text.trim().parse().ok().filter(|n| *n > 0)
}

/// The setup program's file name on the Releases page.
pub fn installer_name() -> String {
    format!("{}-setup.exe", branding::APP_ID)
}

/// The newest release, as GitHub describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Its build number (from its tag, `build-412`).
    pub build: u32,
    /// Its page, which lists what changed.
    pub page: String,
    /// Where its setup program downloads from.
    pub installer_url: String,
    /// The setup program's size, in bytes.
    pub size: u64,
}

/// Reads GitHub's answer about the latest release, or says what's wrong
/// with it.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| tr("GitHub's answer couldn't be read").to_string())?;
    let tag = value["tag_name"].as_str().unwrap_or_default();
    let build = tag
        .strip_prefix("build-")
        .and_then(parse_build)
        .ok_or_else(|| trf!("the latest release ({}) has no build number", tag))?;
    let page = value["html_url"].as_str().unwrap_or_default().to_string();
    let wanted = installer_name();
    let asset = value["assets"]
        .as_array()
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| a["name"].as_str() == Some(wanted.as_str()))
        })
        .ok_or_else(|| trf!("build {} has no setup program yet", build))?;
    let installer_url = asset["browser_download_url"]
        .as_str()
        .filter(|u| u.starts_with("https://"))
        .ok_or_else(|| trf!("build {} has no download address", build))?
        .to_string();
    let size = asset["size"]
        .as_u64()
        .filter(|s| *s > 0)
        .ok_or_else(|| trf!("build {}'s setup program is empty", build))?;
    Ok(Release {
        build,
        page,
        installer_url,
        size,
    })
}

/// Why this copy doesn't update itself, or `None` when it does: it must
/// be on Windows, carry a build number, have been installed by the setup
/// program (its uninstaller beside `exe`) and have updates switched on.
pub fn why_not(build: Option<u32>, exe: &Path, windows: bool, wanted: bool) -> Option<String> {
    if !wanted {
        return Some(tr("Looking for new versions is off.").to_string());
    }
    if !windows {
        return Some(
            tr(
                "Only the Windows version updates itself so far: download new versions from the \
                Releases page.",
            )
            .to_string(),
        );
    }
    if build.is_none() {
        return Some(
            tr(
                "This copy was built by hand, without a build number, so it doesn't update \
                itself.",
            )
            .to_string(),
        );
    }
    if !installed(exe) {
        return Some(
            tr(
                "This copy isn't installed, so it doesn't update itself: run the setup program \
                from the Releases page once, and the installed app keeps itself up to date.",
            )
            .to_string(),
        );
    }
    None
}

/// Whether `exe` was put where it is by the setup program: its uninstaller
/// (`unins000.exe` and so on) is beside it.
fn installed(exe: &Path) -> bool {
    let Some(folder) = exe.parent() else {
        return false;
    };
    std::fs::read_dir(folder).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            name.starts_with("unins") && name.ends_with(".exe")
        })
    })
}

/// Where downloaded setup programs wait: beside the backup copies, in the
/// app's own local folder.
pub fn folder() -> Option<PathBuf> {
    crate::backups::folder().and_then(|f| f.parent().map(|p| p.join("updates")))
}

/// Where build `build`'s setup program goes in `folder`.
fn installer_path(folder: &Path, build: u32) -> PathBuf {
    folder.join(format!("{}-setup-{build}.exe", branding::APP_ID))
}

/// Asks the internet for text and files.
pub trait Fetch: Send {
    /// The text at `url`.
    fn text(&self, url: &str) -> Result<String, String>;
    /// Saves what is at `url` as `to`.
    fn file(&self, url: &str, to: &Path) -> Result<(), String>;
}

/// [`Fetch`] through the system's `curl` (in Windows since Windows 10).
#[derive(Debug, Clone, Copy, Default)]
pub struct Curl;

impl Curl {
    fn command(timeout: u32) -> std::process::Command {
        let mut command = std::process::Command::new("curl");
        command.args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-time",
            &timeout.to_string(),
            "--user-agent",
            &format!("{}-updater", branding::APP_ID),
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window flashing up.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
    }

    fn run(mut command: std::process::Command) -> Result<Vec<u8>, String> {
        let output = command
            .output()
            .map_err(|e| trf!("curl couldn't be started: {}", e))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(curl_failure(&String::from_utf8_lossy(&output.stderr)))
        }
    }
}

/// What curl's complaint means, in plain words where it is a common one.
fn curl_failure(complaint: &str) -> String {
    let complaint = complaint.trim().trim_start_matches("curl: ");
    if complaint.contains("error: 404") {
        tr("no version has been published yet").to_string()
    } else if complaint.contains("resolve host") || complaint.contains("connect to") {
        tr("no internet connection").to_string()
    } else if complaint.contains("error: 403") || complaint.contains("error: 429") {
        tr("GitHub asked to wait a while").to_string()
    } else {
        complaint.to_string()
    }
}

impl Fetch for Curl {
    fn text(&self, url: &str) -> Result<String, String> {
        let mut command = Self::command(ASK_TIMEOUT_SECONDS);
        command.args(["--header", "Accept: application/vnd.github+json", url]);
        let bytes = Self::run(command)?;
        String::from_utf8(bytes).map_err(|_| tr("GitHub's answer couldn't be read").to_string())
    }

    fn file(&self, url: &str, to: &Path) -> Result<(), String> {
        let mut command = Self::command(DOWNLOAD_TIMEOUT_SECONDS);
        command.arg("--output").arg(to).arg(url);
        Self::run(command).map(|_| ())
    }
}

/// What one look found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// Nothing newer than this build (the newest one's number).
    UpToDate(u32),
    /// A newer build, downloaded to `installer`.
    Ready {
        /// The release.
        release: Release,
        /// Its setup program, ready to run.
        installer: PathBuf,
    },
    /// The look didn't work (no internet, GitHub busy): why.
    Failed(String),
}

/// One look: asks GitHub for the latest release and, when it is newer
/// than `current`, downloads its setup program into `folder` (or finds it
/// there whole from an earlier look).
pub fn look(fetch: &dyn Fetch, current: u32, folder: &Path) -> Found {
    let release = match fetch
        .text(branding::RELEASES_API)
        .and_then(|t| parse_release(&t))
    {
        Ok(release) => release,
        Err(why) => return Found::Failed(why),
    };
    if release.build <= current {
        return Found::UpToDate(release.build);
    }
    let installer = installer_path(folder, release.build);
    let whole = |path: &Path| std::fs::metadata(path).is_ok_and(|m| m.len() == release.size);
    if !whole(&installer) {
        if let Err(e) = std::fs::create_dir_all(folder) {
            return Found::Failed(trf!("the download folder couldn't be made: {}", e));
        }
        let part = installer.with_extension("part");
        if let Err(why) = fetch.file(&release.installer_url, &part) {
            let _ = std::fs::remove_file(&part);
            return Found::Failed(why);
        }
        if !whole(&part) {
            let _ = std::fs::remove_file(&part);
            return Found::Failed(tr("the download was cut off").to_string());
        }
        if let Err(e) = std::fs::rename(&part, &installer) {
            return Found::Failed(trf!("the download couldn't be kept: {}", e));
        }
    }
    forget_older(folder, release.build);
    Found::Ready { release, installer }
}

/// Removes setup programs for builds older than `build` from `folder`.
fn forget_older(folder: &Path, build: u32) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let start = format!("{}-setup-", branding::APP_ID);
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let older = name
            .strip_prefix(&start)
            .and_then(|rest| rest.split('.').next())
            .and_then(parse_build)
            .is_some_and(|b| b < build);
        if older {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Looks in the background: soon after starting, then every few hours,
/// until a newer build is ready.
pub struct Updater {
    found: Receiver<Found>,
    /// Dropping it stops the looking.
    _stop: Sender<()>,
}

impl Updater {
    /// Starts looking with `fetch`, for builds newer than `current`,
    /// downloading into `folder`.
    pub fn start(fetch: Box<dyn Fetch>, current: u32, folder: PathBuf) -> Self {
        Self::start_with(fetch, current, folder, FIRST_LOOK_AFTER, LOOK_EVERY)
    }

    fn start_with(
        fetch: Box<dyn Fetch>,
        current: u32,
        folder: PathBuf,
        first: Duration,
        every: Duration,
    ) -> Self {
        let (found_tx, found) = mpsc::channel();
        let (stop, stop_rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            let mut wait = first;
            loop {
                match stop_rx.recv_timeout(wait) {
                    Err(RecvTimeoutError::Timeout) => {}
                    // Stopped, or the app has gone.
                    _ => return,
                }
                let result = look(fetch.as_ref(), current, &folder);
                let ready = matches!(result, Found::Ready { .. });
                if found_tx.send(result).is_err() || ready {
                    return;
                }
                wait = every;
            }
        });
        Self { found, _stop: stop }
    }

    /// What the looking has found since last asked, newest last.
    pub fn poll(&self) -> Vec<Found> {
        self.found.try_iter().collect()
    }
}

/// A setup program to run when the app has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingInstall {
    /// The setup program.
    pub installer: PathBuf,
    /// Open the app again afterwards (Restart now).
    pub reopen: bool,
}

impl PendingInstall {
    /// The setup program's arguments: quiet, closing anything in the way,
    /// and opening the app again only after Restart now (`/NOLAUNCH` is
    /// read by the setup script, `installer/windows/setup.iss`).
    pub fn arguments(&self) -> Vec<&'static str> {
        let mut arguments = vec![
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/CLOSEAPPLICATIONS",
        ];
        if !self.reopen {
            arguments.push("/NOLAUNCH");
        }
        arguments
    }

    /// Starts the setup program, which carries on after the app has gone.
    pub fn run(&self) -> std::io::Result<()> {
        std::process::Command::new(&self.installer)
            .args(self.arguments())
            .spawn()
            .map(|_| ())
    }
}

/// The install agreed to, run once the window has closed.
static PENDING: std::sync::Mutex<Option<PendingInstall>> = std::sync::Mutex::new(None);

/// Runs `install` when the app closes (Restart now, or Later).
pub fn set_pending(install: PendingInstall) {
    *PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(install);
}

/// The install agreed to, if any (taken: it runs once).
pub fn take_pending() -> Option<PendingInstall> {
    PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod tests;
