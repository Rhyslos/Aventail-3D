//! The emergency copy: when the app crashes, the work as it was is saved
//! into this run's backup folder before the app goes, so at most a moment
//! of work is lost instead of the minutes since the last backup copy. The
//! next start offers it back like any other backup copy
//! ([`crate::backups`], `fileio::autosave`).
//!
//! A crash in Rust is a *panic*. A panic hook (a function the program
//! runs whenever a panic happens, on whichever thread) writes the copy.
//! The hook can't reach the workbench: the panic usually happens in the
//! middle of the workbench's own frame, perhaps halfway through an edit.
//! So the app keeps a recent *snapshot* of the document instead:
//! [`Snapshots::update`] copies the document after a frame in which it
//! changed (between frames it is always whole, never mid-edit), at most
//! every [`SNAPSHOT_EVERY`]. Copying is cheap: shapes are shared, not
//! duplicated. The snapshot sits behind a lock that the hook only *tries*
//! to take, a few times, so a lock that is held or broken never makes the
//! hook wait for long. The file is written on a thread of its own, which
//! the hook waits for at most [`HOOK_WAIT`]; a failure anywhere is
//! ignored, so the hook never panics itself.

use fileio::autosave;
use scene::Document;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

/// The shortest time between two snapshots of a document that keeps
/// changing (a drag changes it every frame). A crash loses at most about
/// this much work.
pub const SNAPSHOT_EVERY: Duration = Duration::from_millis(500);
/// The longest the panic hook waits for the emergency copy to be written.
/// Ordinary scenes take a fraction of a second; a heavy one may not make
/// it, and the backup copies from the timer are still there.
pub const HOOK_WAIT: Duration = Duration::from_secs(10);
/// How many times the hook tries the snapshot's lock, and how long it
/// waits between tries: a tenth of a second in all.
const LOCK_TRIES: u32 = 20;
const LOCK_PAUSE: Duration = Duration::from_millis(5);

/// What the hook needs, shared between the app and the hook.
#[derive(Debug, Default)]
struct Shared {
    slot: Mutex<Slot>,
    /// An emergency copy is being written: a second panic (perhaps in the
    /// writing itself) doesn't start another.
    writing: AtomicBool,
}

/// The snapshot and where it goes.
#[derive(Debug, Default)]
struct Slot {
    /// This run's backup folder (its session folder); `None` while the
    /// app starts, after it closed normally, or with backups unavailable.
    folder: Option<PathBuf>,
    /// The document as of the last snapshot; `None` when nothing is
    /// unsaved (then there is nothing to rescue).
    document: Option<Arc<Document>>,
}

/// The emergency copy's shared state. Clones share it: the app keeps one
/// and the panic hook another.
#[derive(Debug, Clone, Default)]
pub struct Emergency {
    shared: Arc<Shared>,
}

/// Locks `slot`, getting past a lock left broken by a panic: the slot is
/// only ever replaced whole, so what it holds is always usable.
fn lock(slot: &Mutex<Slot>) -> MutexGuard<'_, Slot> {
    slot.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Sets `writing` back to false when dropped, even if the writing
/// panicked.
struct Done<'a>(&'a AtomicBool);

impl Drop for Done<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Emergency {
    /// Nothing to rescue yet, and nowhere to put it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the backup folder the emergency copy goes into (this run's
    /// session folder), or `None` for nowhere.
    pub fn set_folder(&self, folder: Option<PathBuf>) {
        lock(&self.shared.slot).folder = folder;
    }

    /// Keeps `document` as the snapshot to rescue, or none.
    pub fn keep(&self, document: Option<Document>) {
        let new = document.map(Arc::new);
        let old = std::mem::replace(&mut lock(&self.shared.slot).document, new);
        // Let go of the old one outside the lock.
        drop(old);
    }

    /// Forgets the folder and the snapshot: the app is closing normally,
    /// and its backup folder is about to be deleted.
    pub fn forget(&self) {
        let mut slot = lock(&self.shared.slot);
        slot.folder = None;
        let old = slot.document.take();
        drop(slot);
        drop(old);
    }

    /// Makes the program write the emergency copy whenever it panics,
    /// after the usual panic message. Call once, when the app starts.
    pub fn install(&self) {
        let previous = std::panic::take_hook();
        let emergency = self.clone();
        std::panic::set_hook(Box::new(move |info| {
            previous(info);
            emergency.on_panic();
        }));
    }

    /// What the panic hook does: writes the emergency copy and says where
    /// it went. Never panics.
    fn on_panic(&self) {
        if let Some(path) = self.write_now(HOOK_WAIT) {
            // Ignored if the error output is gone: nothing more to do.
            let _ = writeln!(
                std::io::stderr(),
                "Your unsaved work was saved as an emergency copy ({}). The next start offers \
                 it back.",
                path.display()
            );
        }
    }

    /// The folder and snapshot, if the lock can be had within a tenth of
    /// a second and there is something to write.
    fn take(&self) -> Option<(PathBuf, Arc<Document>)> {
        for _ in 0..LOCK_TRIES {
            let slot = match self.shared.slot.try_lock() {
                Ok(slot) => slot,
                Err(TryLockError::Poisoned(broken)) => broken.into_inner(),
                Err(TryLockError::WouldBlock) => {
                    std::thread::sleep(LOCK_PAUSE);
                    continue;
                }
            };
            return Some((slot.folder.clone()?, slot.document.clone()?));
        }
        None
    }

    /// Writes the emergency copy now, waiting at most `wait` for it.
    /// Returns its file, or `None` if there was nothing to write, nowhere
    /// to write it, the lock couldn't be had, one is being written
    /// already, or it failed or took too long.
    pub fn write_now(&self, wait: Duration) -> Option<PathBuf> {
        if self.shared.writing.swap(true, Ordering::SeqCst) {
            return None;
        }
        let Some((folder, document)) = self.take() else {
            self.shared.writing.store(false, Ordering::SeqCst);
            return None;
        };
        let (send, receive) = std::sync::mpsc::channel();
        let shared = Arc::clone(&self.shared);
        let started = std::thread::Builder::new()
            .name("emergency copy".into())
            .spawn(move || {
                // Done once written, or if the writing panics.
                let done = Done(&shared.writing);
                let written = write(&folder, &document);
                drop(done);
                let _ = send.send(written);
            });
        if started.is_err() {
            self.shared.writing.store(false, Ordering::SeqCst);
            return None;
        }
        receive.recv_timeout(wait).ok().flatten()
    }
}

/// Writes `document` as the emergency copy in `folder`.
fn write(folder: &Path, document: &Document) -> Option<PathBuf> {
    let bytes = fileio::project::to_bytes(document).ok()?;
    autosave::write_emergency_in(folder, &bytes).ok()
}

/// Keeps the emergency snapshot up to date from the running app, frame by
/// frame.
#[derive(Debug)]
pub struct Snapshots {
    emergency: Emergency,
    /// The revision of the document last kept, and whether it was kept
    /// (unsaved) or cleared (saved); `None` before the first frame.
    kept: Option<(u64, bool)>,
    /// When the last snapshot was taken.
    taken: Option<Instant>,
}

impl Snapshots {
    /// Keeps snapshots for `emergency`.
    pub fn new(emergency: Emergency) -> Self {
        Self {
            emergency,
            kept: None,
            taken: None,
        }
    }

    /// The shared state, for setting the folder.
    pub fn emergency(&self) -> &Emergency {
        &self.emergency
    }

    /// Called after every frame with the document and whether it has
    /// unsaved changes. Takes a snapshot if it changed, unless one was
    /// taken less than [`SNAPSHOT_EVERY`] ago: then it returns how long
    /// until one is due, for the app to look again then (a frame may not
    /// otherwise come while the user pauses).
    pub fn update(&mut self, document: &Document, unsaved: bool, now: Instant) -> Option<Duration> {
        let revision = document.revision();
        if self.kept == Some((revision, unsaved)) {
            return None;
        }
        if !unsaved {
            // Saved (or nothing done yet): nothing to rescue.
            self.emergency.keep(None);
            self.kept = Some((revision, false));
            return None;
        }
        if let Some(taken) = self.taken {
            let since = now.saturating_duration_since(taken);
            if since < SNAPSHOT_EVERY {
                return Some(SNAPSHOT_EVERY - since);
            }
        }
        let scene = document.scene().clone();
        self.emergency
            .keep(Some(Document::from_scene(scene, document.selection())));
        self.kept = Some((revision, true));
        self.taken = Some(now);
        None
    }
}

#[cfg(test)]
mod tests;
