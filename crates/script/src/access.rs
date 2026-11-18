//! What a script may reach outside the project, and reaching it safely
//! (choice 5; [decision 0307](../../../docs/decisions/0307-keeping-sharing-and-permissions.md)).
//!
//! A script reaches files only through `Project:readText`, `writeText`
//! and `exportShapes`, and only in the one folder its approved
//! permission names ([`Access`]): the project's folder, or a folder the
//! person chose. Every path is checked here before anything is read:
//!
//! - it's **inside the folder**: written from inside it (`models/chair.glb`),
//!   never a whole path from the top of the computer (`/etc`, `C:\`),
//!   and never climbing out with `..`;
//! - it goes through **no link** (a symbolic link or junction), which
//!   could lead out of the folder, and what's found is checked again to
//!   be inside it;
//! - its names are ones every computer can keep: not too long, no
//!   characters or names Windows refuses.
//!
//! **Writing waits for the end** ([`Writes`]): what a script writes is
//! held until the run ends well, then each file is written whole or not
//! at all (`fileio::project::safe_write`). A run that fails, is stopped or
//! runs out of time writes nothing; a script reading a file it has
//! written reads what it wrote.

use std::path::{Path, PathBuf};
use words::{tr, trf};

/// The most bytes `Project:readText` reads from one file.
pub const MAX_READ_BYTES: u64 = 16 << 20;
/// The most files one run may write (an export's files beside it
/// counted).
pub const MAX_WRITES: usize = 1_000;
/// The most bytes one run may write, all its files together: they wait
/// in the app's memory until the run ends.
pub const MAX_WRITE_BYTES: usize = 256 << 20;
/// The longest path a script may give, in bytes.
pub const MAX_PATH_BYTES: usize = 1_024;
/// The longest name of one file or folder in a path, in bytes.
pub const MAX_NAME_BYTES: usize = 255;
/// The most folders deep a path may go.
pub const MAX_DEPTH: usize = 32;

/// What a run may reach outside the project: worked out by the app from
/// the script's header and what the person approved. The standard is
/// nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Access {
    /// The folder it may reach, when its permission names one and there
    /// is one (an unsaved project has no folder).
    pub folder: Option<PathBuf>,
    /// Whether it may read files there.
    pub read: bool,
    /// Whether it may write files there.
    pub write: bool,
    /// Whether the folder is one the person chose (not the project's).
    pub chosen: bool,
}

/// What a run has written, waiting for the run to end well.
#[derive(Debug, Default)]
pub(crate) struct Writes {
    /// Each file: where it goes, the path the script gave, and its
    /// bytes, in the order written.
    files: Vec<(PathBuf, String, Vec<u8>)>,
    bytes: usize,
}

impl Writes {
    /// Holds `bytes` for `path`, in place of anything written there
    /// before in this run.
    fn hold(&mut self, path: PathBuf, given: &str, bytes: Vec<u8>) -> Result<(), String> {
        let earlier = self
            .files
            .iter()
            .position(|(p, _, _)| *p == path)
            .map(|at| self.files.remove(at).2.len())
            .unwrap_or(0);
        let total = self.bytes - earlier + bytes.len();
        if total > MAX_WRITE_BYTES {
            return Err(trf!(
                "a script may write at most {} MB in one run",
                MAX_WRITE_BYTES >> 20
            ));
        }
        if self.files.len() >= MAX_WRITES {
            return Err(trf!(
                "a script may write at most {} files in one run",
                MAX_WRITES
            ));
        }
        self.bytes = total;
        self.files.push((path, given.to_string(), bytes));
        Ok(())
    }

    /// What was written to `path` in this run, if anything.
    fn held(&self, path: &Path) -> Option<&[u8]> {
        self.files
            .iter()
            .find(|(p, _, _)| p == path)
            .map(|(_, _, b)| b.as_slice())
    }

    /// Whether nothing was written.
    pub(crate) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Writes every file held, each whole or not at all, checking each
    /// place again first (something may have changed on disk while the
    /// script ran). Gives back how many were written, or the first that
    /// couldn't be (with the ones before it written) and why.
    pub(crate) fn write_out(self, access: &Access) -> Result<usize, String> {
        let Some(root) = access.folder.as_deref().filter(|_| access.write) else {
            return if self.files.is_empty() {
                Ok(0)
            } else {
                Err(tr("it isn't allowed to write files").to_string())
            };
        };
        let mut written = 0;
        for (_, given, bytes) in self.files {
            let shown = given.clone();
            let checked = place(root, &given).map_err(|why| trf!("{}: {}", shown, why))?;
            if let Some(parent) = checked.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| trf!("{}: its folder couldn't be made ({})", shown, e))?;
            }
            // Made folders are checked once more for links.
            let checked = place(root, &given).map_err(|why| trf!("{}: {}", shown, why))?;
            fileio::project::safe_write(&checked, &bytes)
                .map_err(|e| trf!("{} couldn't be written ({})", shown, e))?;
            written += 1;
        }
        Ok(written)
    }
}

/// Why a script may not do what it tried, in words that say what to do.
fn not_allowed(call: &str, path: &str, access: &Access, write: bool) -> String {
    let allowed = if write { access.write } else { access.read };
    if !allowed {
        return if write {
            trf!(
                "{} can't write {}: this script isn't allowed to write files. A script asks for it with a line at its top (-- asks: {}), and you allow it when it first runs.",
                call,
                path,
                tr(crate::Ask::WriteProjectFolder.words())
            )
        } else {
            trf!(
                "{} can't read {}: this script isn't allowed to read files. A script asks for it with a line at its top (-- asks: {}), and you allow it when it first runs.",
                call,
                path,
                tr(crate::Ask::ReadProjectFolder.words())
            )
        };
    }
    trf!(
        "{} can't reach {}: the project hasn't been saved yet, so it has no folder. Save the project, then run the script again.",
        call,
        path
    )
}

/// The file a script means by `given`, inside `root`, checked (module
/// docs). It may not exist yet.
pub(crate) fn place(root: &Path, given: &str) -> Result<PathBuf, String> {
    if given.trim().is_empty() {
        return Err(tr("the path is empty").to_string());
    }
    if given.len() > MAX_PATH_BYTES {
        return Err(trf!("the path is longer than {} letters", MAX_PATH_BYTES));
    }
    if given.starts_with(['/', '\\']) || given.contains(':') {
        return Err(tr(
            "a path is written from inside the folder (such as models/chair.glb), not from the top of the computer",
        )
        .to_string());
    }
    let mut names = Vec::new();
    for name in given.split(['/', '\\']) {
        match name {
            "" | "." => continue,
            ".." => {
                return Err(tr("a path can't climb out of the folder with ..").to_string());
            }
            name => {
                check_name(name)?;
                names.push(name);
            }
        }
    }
    if names.is_empty() {
        return Err(tr("the path names no file").to_string());
    }
    if names.len() > MAX_DEPTH {
        return Err(trf!("the path goes more than {} folders deep", MAX_DEPTH));
    }
    let root =
        std::fs::canonicalize(root).map_err(|e| trf!("the folder can't be reached ({})", e))?;
    let mut path = root.clone();
    let last = names.len() - 1;
    let mut missing = false;
    for (n, name) in names.into_iter().enumerate() {
        path.push(name);
        if missing {
            continue;
        }
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(tr(
                    "it goes through a link (a shortcut to somewhere else), which a script may not follow",
                )
                .to_string());
            }
            Ok(meta) if n < last && !meta.is_dir() => {
                return Err(trf!("{} isn't a folder", name));
            }
            Ok(meta) if n == last && meta.is_dir() => {
                return Err(tr("it names a folder, not a file").to_string());
            }
            Ok(_) => {}
            // What isn't there yet is made when the run ends well.
            Err(_) => missing = true,
        }
    }
    // Belt and braces: what's there is inside the folder.
    let mut found = path.as_path();
    while !found.exists() {
        found = found.parent().unwrap_or(&root);
    }
    let real = std::fs::canonicalize(found).unwrap_or_default();
    if !real.starts_with(&root) {
        return Err(tr("it leads out of the folder").to_string());
    }
    Ok(path)
}

/// Whether one name in a path can be kept on every computer.
fn check_name(name: &str) -> Result<(), String> {
    if name.len() > MAX_NAME_BYTES {
        return Err(trf!(
            "a name in the path is longer than {} letters",
            MAX_NAME_BYTES
        ));
    }
    if name
        .chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*'))
        || name.ends_with([' ', '.'])
    {
        return Err(trf!(
            "{} has characters a file name can't have (or ends in a space or a dot)",
            name
        ));
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        return Err(trf!("{} is a name Windows keeps for itself", name));
    }
    Ok(())
}

/// Reads the text at `given` for `call`.
pub(crate) fn read_text(
    access: &Access,
    writes: &Writes,
    call: &str,
    given: &str,
) -> Result<String, String> {
    let root = match (&access.folder, access.read) {
        (Some(root), true) => root,
        _ => return Err(not_allowed(call, given, access, false)),
    };
    let path = place(root, given).map_err(|why| trf!("{} can't read {}: {}.", call, given, why))?;
    if let Some(bytes) = writes.held(&path) {
        return String::from_utf8(bytes.to_vec())
            .map_err(|_| trf!("{} can't read {}: it isn't text.", call, given));
    }
    use std::io::Read as _;
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .and_then(|f| f.take(MAX_READ_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                trf!("{} can't read {}: there's no such file.", call, given)
            }
            _ => trf!("{} can't read {} ({}).", call, given, e),
        })?;
    if bytes.len() as u64 > MAX_READ_BYTES {
        return Err(trf!(
            "{} can't read {}: it's bigger than {} MB, the most a script reads at once.",
            call,
            given,
            MAX_READ_BYTES >> 20
        ));
    }
    String::from_utf8(bytes).map_err(|_| {
        trf!(
            "{} can't read {}: it isn't text (or isn't UTF-8).",
            call,
            given
        )
    })
}

/// Whether `call` may write at `given`, checked before doing work for it
/// (an export).
pub(crate) fn check_write(access: &Access, call: &str, given: &str) -> Result<(), String> {
    let root = match (&access.folder, access.write) {
        (Some(root), true) => root,
        _ => return Err(not_allowed(call, given, access, true)),
    };
    place(root, given)
        .map(|_| ())
        .map_err(|why| trf!("{} can't write {}: {}.", call, given, why))
}

/// Holds `bytes` to write at `given` when the run ends well, for `call`.
/// Gives back where they'll go.
pub(crate) fn write(
    access: &Access,
    writes: &mut Writes,
    call: &str,
    given: &str,
    bytes: Vec<u8>,
) -> Result<PathBuf, String> {
    let root = match (&access.folder, access.write) {
        (Some(root), true) => root,
        _ => return Err(not_allowed(call, given, access, true)),
    };
    let path =
        place(root, given).map_err(|why| trf!("{} can't write {}: {}.", call, given, why))?;
    writes
        .hold(path.clone(), given, bytes)
        .map_err(|why| trf!("{} can't write {}: {}.", call, given, why))?;
    Ok(path)
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
