//! Scripting (Phase 14; [decision 0299](../../../docs/decisions/0299-scripting-plan.md),
//! [decision 0301](../../../docs/decisions/0301-script-runner.md), plan in
//! `docs/plans/phase-14.md`).
//!
//! Scripts are written in **LustScript** (`.lsc`), turned into Lua by the
//! Lust compiler held here as a library, or in plain **Lua** (`.lua`).
//! Both run in **Luau** through `mlua`, in a sandbox that reaches nothing
//! outside the project, with a time and a memory limit, on a thread of
//! their own, against a copy of the scene: the app never waits for a
//! script, and Stop works at any moment.
//!
//! - [`read_header`]: what a script says about itself, read without
//!   running it.
//! - [`check`]: its mistakes, each with its file, line and column.
//! - [`Run::start`]: runs it against a [`Snapshot`]; what it says comes
//!   back as [`Event`]s, and what it changed as a [`Landing`]: the
//!   changes to carry out on the real document as one undo step.
//! - [`tokens`]: the pieces of a script's text, for colouring it.
//! - [`only_lines`] and [`console_script`]: part of a script, or the
//!   console line, made ready to run.
//! - [`library`]: your script library on disk.
//! - [`describe`]: every call scripts can make, listed once, and the
//!   description files and the manual's reference page made from it.
//!
//! The app's name never appears here: the scripting folder's name and
//! who made the description files come in [`Names`], from the app's
//! branding. Scripts change the scene only through the same commands and
//! operations as the app's tools (decision 0302).

mod changes;
mod compile;
pub mod describe;
mod header;
pub mod library;
mod natives;
mod output;
mod pieces;
mod problem;
mod run;
mod sandbox;
mod tokens;
mod turns;
mod values;

pub use changes::{MAX_BUILD, MAX_BUILT, MAX_CHANGES, MAX_CORNERS};
pub use compile::check;
pub use describe::{API_VERSION, descriptions, fetch_everything, reference_page};
pub use header::{Ask, Header, read_header};
pub use natives::MAX_CHUNK;
pub use pieces::{CONSOLE_LUA, CONSOLE_LUSTSCRIPT, console_script, only_lines};
pub use tokens::{Token, TokenKind, tokens};
pub use output::{Ending, Event, Landing, Line, MAX_LINE_BYTES, MAX_LINES};
pub use problem::Problem;
pub use run::{Mode, Run, Snapshot, ending_words};
pub use turns::CHECK_LIMIT;
pub use values::MAX_LIST;

use std::time::Duration;

/// The names scripting takes from the app's branding (never written in
/// the code, `RENAMING.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    /// The scripting folder's name, which is also Lua's global for the
    /// app's calls: `aventail` (`fetch "aventail/scene"`, `aventail.Scene`).
    pub folder: String,
    /// Who made the description files, for their first line: the app's
    /// name and version.
    pub made_by: String,
}

/// How long a run may take, and how much memory it may use (choice 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The longest a run may take.
    pub time: Duration,
    /// The most memory a run may use, in bytes.
    pub memory: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            time: Duration::from_secs(10),
            memory: 256 << 20,
        }
    }
}

/// The language a script is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// LustScript (`.lsc`).
    LustScript,
    /// Lua (`.lua`).
    Lua,
}

/// The most bytes a script file may hold.
pub const MAX_SCRIPT_BYTES: u64 = 1 << 20;

/// A script: its file's name (`bevel.lsc`) and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFile {
    /// The file's name, without its folder: mistakes are shown at it.
    pub name: String,
    /// Its text.
    pub text: String,
}

impl ScriptFile {
    /// The language its name's ending says.
    pub fn language(&self) -> Option<Language> {
        let (_, ending) = self.name.rsplit_once('.')?;
        if ending.eq_ignore_ascii_case("lsc") {
            Some(Language::LustScript)
        } else if ending.eq_ignore_ascii_case("lua") {
            Some(Language::Lua)
        } else {
            None
        }
    }

    /// Reads a script file from disk, within [`MAX_SCRIPT_BYTES`], with a
    /// plain reason when it can't be.
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        use std::io::Read as _;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let file = Self {
            name: name.clone(),
            text: String::new(),
        };
        if file.language().is_none() {
            return Err(words::trf!(
                "{} isn't a script: scripts end in .lsc (LustScript) or .lua (Lua).",
                name
            ));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .and_then(|f| f.take(MAX_SCRIPT_BYTES + 1).read_to_end(&mut bytes))
            .map_err(|e| words::trf!("{} couldn't be read ({}).", name, e))?;
        if bytes.len() as u64 > MAX_SCRIPT_BYTES {
            return Err(words::trf!(
                "{} is too big for a script: a script may be at most {} KB.",
                name,
                MAX_SCRIPT_BYTES >> 10
            ));
        }
        let text = String::from_utf8(bytes).map_err(|_| {
            words::trf!("{} isn't text a script can hold (it must be UTF-8).", name)
        })?;
        Ok(Self { name, text })
    }
}
