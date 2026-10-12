//! Backup copies saved in the background, and finding them again after a
//! crash ([decision 0036](../../../docs/decisions/0036-autosave.md)).
//!
//! Every running copy of the app has its own *session* folder inside the
//! autosave folder, holding:
//! - `lock`: a file the app keeps locked while it runs. The system lets go
//!   of the lock when the app ends in any way, crash included, so a
//!   session folder whose lock can be taken belongs to an app that is no
//!   longer running;
//! - `project.txt`: the project the session had open, if it was saved
//!   somewhere (so recovery can say which project it is);
//! - `copy-N.avtl`: the backup copies, ordinary project files, the
//!   highest number the newest. Only the newest few are kept.
//!
//! When the app closes normally it deletes its session folder: whatever
//! was unsaved was either saved or deliberately thrown away. A folder
//! left behind with copies in it means the app didn't close normally.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The name of the file a running session keeps locked.
const LOCK: &str = "lock";
/// The file naming the session's project.
const PROJECT: &str = "project.txt";
/// The start of a backup copy's file name.
const COPY_PREFIX: &str = "copy-";
/// A backup copy's file extension.
const COPY_EXTENSION: &str = "avtl";

/// This running app's session: its folder, kept locked while it runs.
#[derive(Debug)]
pub struct Session {
    folder: PathBuf,
    /// Held open (and locked) for as long as the session lives.
    _lock: File,
}

/// One backup copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copy {
    /// Its file.
    pub path: PathBuf,
    /// When it was saved.
    pub saved: SystemTime,
}

/// A session left behind by an app that didn't close normally, with its
/// backup copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftover {
    /// Its folder.
    pub folder: PathBuf,
    /// The project it had open, if that had been saved somewhere.
    pub project: Option<PathBuf>,
    /// Its copies, newest first.
    pub copies: Vec<Copy>,
}

impl Session {
    /// Starts a session in a new folder inside `root` (made if needed).
    pub fn start(root: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        // The process number keeps two apps started in the same
        // millisecond apart; a counter, two sessions in one process.
        let pid = std::process::id();
        let mut n = 0u32;
        let folder = loop {
            let candidate = root.join(format!("session-{stamp}-{pid}-{n}"));
            match std::fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists && n < 1000 => n += 1,
                Err(e) => return Err(e),
            }
        };
        let lock = File::create(folder.join(LOCK))?;
        lock.try_lock().map_err(io::Error::other)?;
        Ok(Self {
            folder,
            _lock: lock,
        })
    }

    /// The session's folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Notes which project the session has open (`None` for one never
    /// saved).
    pub fn set_project(&self, project: Option<&Path>) -> io::Result<()> {
        let text = project.map_or_else(String::new, |p| p.to_string_lossy().into_owned());
        std::fs::write(self.folder.join(PROJECT), text)
    }

    /// Writes `bytes` (a project file) as the newest backup copy, then
    /// deletes all but the newest `keep` copies (at least one is kept).
    /// Returns the new copy's file.
    pub fn write(&self, bytes: &[u8], keep: usize) -> io::Result<PathBuf> {
        write_copy_in(&self.folder, bytes, keep)
    }

    /// Makes `copy` (from a leftover session) this session's newest
    /// backup, so restored work is safe from a second crash straight away.
    /// It keeps the time it was first saved, so a later recovery still
    /// says how old the work is.
    pub fn adopt(&self, copy: &Path, keep: usize) -> io::Result<PathBuf> {
        let bytes = std::fs::read(copy)?;
        let adopted = self.write(&bytes, keep)?;
        if let Ok(saved) = std::fs::metadata(copy).and_then(|m| m.modified()) {
            File::options()
                .write(true)
                .open(&adopted)
                .and_then(|f| f.set_modified(saved))?;
        }
        Ok(adopted)
    }

    /// Ends the session normally: its folder and copies are deleted.
    pub fn end(self) -> io::Result<()> {
        let Self { folder, _lock } = self;
        drop(_lock);
        std::fs::remove_dir_all(folder)
    }
}

/// Writes `bytes` as the next copy in a session's `folder` and deletes
/// the oldest beyond `keep` ([`Session::write`], for writing from another
/// thread, which only needs the folder).
pub fn write_copy_in(folder: &Path, bytes: &[u8], keep: usize) -> io::Result<PathBuf> {
    let numbers = copy_numbers(folder)?;
    let next = numbers.last().map_or(1, |n| n + 1);
    let path = folder.join(format!("{COPY_PREFIX}{next}.{COPY_EXTENSION}"));
    // Written beside it first, so a copy is never half-written.
    let temporary = path.with_extension("tmp");
    {
        let mut file = File::create(&temporary)?;
        io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, &path)?;
    let keep = keep.max(1);
    let mut all = numbers;
    all.push(next);
    if all.len() > keep {
        for old in &all[..all.len() - keep] {
            let _ =
                std::fs::remove_file(folder.join(format!("{COPY_PREFIX}{old}.{COPY_EXTENSION}")));
        }
    }
    Ok(path)
}

/// The numbers of the copies in `folder`, lowest first.
fn copy_numbers(folder: &Path) -> io::Result<Vec<u64>> {
    let mut numbers: Vec<u64> = std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let number = name
                .strip_prefix(COPY_PREFIX)?
                .strip_suffix(&format!(".{COPY_EXTENSION}"))?;
            number.parse().ok()
        })
        .collect();
    numbers.sort_unstable();
    Ok(numbers)
}

/// The sessions in `root` whose apps are no longer running, with their
/// copies. Sessions that ended without a copy (nothing to recover) are
/// tidied away; sessions still running are left alone.
pub fn leftovers(root: &Path) -> Vec<Leftover> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let folder = entry.path();
        if !folder.is_dir() || !entry.file_name().to_string_lossy().starts_with("session-") {
            continue;
        }
        // Still running: its app holds the lock.
        let lock = File::options().write(true).open(folder.join(LOCK));
        if let Ok(lock) = &lock
            && lock.try_lock().is_err()
        {
            continue;
        }
        drop(lock);
        let numbers = copy_numbers(&folder).unwrap_or_default();
        if numbers.is_empty() {
            let _ = std::fs::remove_dir_all(&folder);
            continue;
        }
        let copies = numbers
            .iter()
            .rev()
            .filter_map(|n| {
                let path = folder.join(format!("{COPY_PREFIX}{n}.{COPY_EXTENSION}"));
                let saved = std::fs::metadata(&path).ok()?.modified().ok()?;
                Some(Copy { path, saved })
            })
            .collect();
        let project = std::fs::read_to_string(folder.join(PROJECT))
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .map(PathBuf::from);
        found.push(Leftover {
            folder,
            project,
            copies,
        });
    }
    // Newest first, by their newest copy.
    found.sort_by_key(|l| std::cmp::Reverse(l.copies.first().map(|c| c.saved)));
    found
}

/// Deletes a leftover session and its copies (after they were restored
/// or deliberately thrown away).
pub fn discard(leftover: &Leftover) -> io::Result<()> {
    std::fs::remove_dir_all(&leftover.folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty folder for one test.
    fn root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("autosave-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn copies_are_numbered_and_only_the_newest_are_kept() {
        let root = root("keep");
        let session = Session::start(&root).unwrap();
        for i in 0..7u8 {
            session.write(&[i], 3).unwrap();
        }
        assert_eq!(copy_numbers(session.folder()).unwrap(), vec![5, 6, 7]);
        assert_eq!(
            std::fs::read(session.folder().join("copy-7.avtl")).unwrap(),
            vec![6]
        );
        // Keeping none still keeps one.
        session.write(&[9], 0).unwrap();
        assert_eq!(copy_numbers(session.folder()).unwrap(), vec![8]);
        let folder = session.folder().to_path_buf();
        session.end().unwrap();
        assert!(!folder.exists(), "a normal end tidies up");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_running_session_is_not_a_leftover_but_a_dead_one_is() {
        let root = root("dead");
        let running = Session::start(&root).unwrap();
        running.write(b"one", 5).unwrap();
        running
            .set_project(Some(Path::new("/work/crate.avtl")))
            .unwrap();
        assert!(leftovers(&root).is_empty(), "still running");
        // A crash: the lock goes away but the folder stays.
        let folder = running.folder().to_path_buf();
        drop(running);
        let found = leftovers(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].folder, folder);
        assert_eq!(found[0].project, Some(PathBuf::from("/work/crate.avtl")));
        assert_eq!(found[0].copies.len(), 1);
        // Discarding deletes it.
        discard(&found[0]).unwrap();
        assert!(leftovers(&root).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn leftovers_list_copies_newest_first_and_untitled_has_no_project() {
        let root = root("order");
        let session = Session::start(&root).unwrap();
        session.write(b"a", 5).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        session.write(b"b", 5).unwrap();
        session.set_project(None).unwrap();
        drop(session);
        let found = leftovers(&root);
        assert_eq!(found[0].project, None);
        let copies = &found[0].copies;
        assert_eq!(std::fs::read(&copies[0].path).unwrap(), b"b");
        assert!(copies[0].saved >= copies[1].saved);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_dead_sessions_are_tidied_and_strangers_ignored() {
        let root = root("tidy");
        let empty = Session::start(&root).unwrap();
        let folder = empty.folder().to_path_buf();
        drop(empty);
        std::fs::write(root.join("notes.txt"), "not a session").unwrap();
        std::fs::create_dir_all(root.join("something-else")).unwrap();
        assert!(leftovers(&root).is_empty());
        assert!(!folder.exists(), "nothing to recover: removed");
        assert!(root.join("notes.txt").exists() && root.join("something-else").exists());
        // A missing root has no leftovers.
        assert!(leftovers(&root.join("missing")).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn restored_copies_are_adopted_and_two_sessions_stay_apart() {
        let root = root("adopt");
        let old = Session::start(&root).unwrap();
        let copy = old.write(b"work", 5).unwrap();
        drop(old);
        let found = leftovers(&root);
        let new = Session::start(&root).unwrap();
        let other = Session::start(&root).unwrap();
        assert_ne!(new.folder(), other.folder());
        let old_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        File::options()
            .write(true)
            .open(&copy)
            .unwrap()
            .set_modified(old_time)
            .unwrap();
        let adopted = new.adopt(&copy, 5).unwrap();
        assert_eq!(std::fs::read(&adopted).unwrap(), b"work");
        let adopted_time = std::fs::metadata(&adopted).unwrap().modified().unwrap();
        assert_eq!(adopted_time, old_time, "keeps when it was first saved");
        discard(&found[0]).unwrap();
        assert!(
            leftovers(&root).is_empty(),
            "the two live sessions aren't leftovers"
        );
        new.end().unwrap();
        other.end().unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
