//! A script's header: the comment lines at its top that say its name,
//! what it does, which scripting version it was written for and what it
//! asks to reach ([decision 0301](../../../docs/decisions/0301-script-runner.md),
//! choice 7 of the plan). They're read as text, without running anything,
//! so a script can be shown before it's trusted. The same lines work in
//! LustScript and Lua, since both write comments with `--`:
//!
//! ```text
//! -- name: Bevel hard edges
//! -- about: Bevels every hard edge of the selected shapes.
//! -- api: 1
//! -- count: segments
//! -- asks: read and write the project folder
//! ```

use words::{tr, trf};

/// The most lines looked at for the header: it's at the top.
pub(crate) const MAX_HEADER_LINES: usize = 200;
/// The most bytes looked at for the header.
pub(crate) const MAX_HEADER_BYTES: usize = 16 << 10;
/// The most characters kept of a name.
pub(crate) const MAX_NAME_CHARS: usize = 80;
/// The most characters kept of what a script is about.
pub(crate) const MAX_ABOUT_CHARS: usize = 400;

/// What a script may ask to reach outside the project (choice 5). Asking
/// is all milestone 1 reads: approving comes with milestone 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Reading files in the project's folder.
    ReadProjectFolder,
    /// Reading and writing files in the project's folder.
    WriteProjectFolder,
    /// Reading and writing a folder the person chooses.
    ChosenFolder,
}

impl Ask {
    /// Every ask, with the words a header writes for it.
    pub const ALL: [(Ask, &'static str); 3] = [
        (
            Ask::ReadProjectFolder,
            words::text!("read the project folder"),
        ),
        (
            Ask::WriteProjectFolder,
            words::text!("read and write the project folder"),
        ),
        (
            Ask::ChosenFolder,
            words::text!("read and write a folder you choose"),
        ),
    ];

    /// The words a header writes for it.
    pub fn words(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(ask, _)| *ask == self)
            .map_or("", |(_, words)| words)
    }
}

/// What a script's header says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    /// Its name in words (`-- name:`), if it gives one.
    pub name: Option<String>,
    /// What it does (`-- about:`).
    pub about: Option<String>,
    /// The scripting version it was written for (`-- api:`).
    pub api: Option<u32>,
    /// What it asks to reach (`-- asks:`, one line each).
    pub asks: Vec<Ask>,
    /// The setting Ctrl + wheel changes (`-- count:`), by its name.
    pub count: Option<String>,
    /// Lines that look meant for the header but can't be read, in plain
    /// words.
    pub notes: Vec<String>,
}

impl Header {
    /// The name to show: the header's, or the file's name without its
    /// ending.
    pub fn shown_name(&self, file: &str) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => file
                .rsplit_once('.')
                .map_or(file, |(stem, _)| stem)
                .to_string(),
        }
    }
}

/// The keys a header line can have.
const KEYS: [&str; 5] = ["name", "about", "api", "count", "asks"];

/// Reads the header at the top of `text`: the comment lines (and blank
/// lines) before anything else. Never runs anything and never fails:
/// what can't be read is left out with a note.
pub fn read_header(text: &str) -> Header {
    let mut header = Header::default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut seen: Vec<&str> = Vec::new();
    let mut bytes = 0;
    for (n, line) in text.lines().enumerate() {
        bytes += line.len() + 1;
        if n >= MAX_HEADER_LINES || bytes > MAX_HEADER_BYTES {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // A block comment (`--[[`) or anything that isn't a comment ends
        // the header.
        let Some(comment) = line.strip_prefix("--") else {
            break;
        };
        if comment.starts_with('[') {
            break;
        }
        let Some((key, value)) = comment.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let Some(key) = KEYS.iter().find(|k| **k == key) else {
            continue;
        };
        let value = value.trim();
        if seen.contains(key) && *key != "asks" {
            header.notes.push(trf!(
                "Line {}: the header already says its {}, so this line is left out.",
                n + 1,
                key
            ));
            continue;
        }
        seen.push(key);
        read_line(&mut header, key, value, n + 1);
    }
    header
}

/// One header line, `key: value`, read into `header`.
fn read_line(header: &mut Header, key: &str, value: &str, line: usize) {
    match key {
        "name" if !value.is_empty() => header.name = Some(capped(value, MAX_NAME_CHARS)),
        "about" if !value.is_empty() => header.about = Some(capped(value, MAX_ABOUT_CHARS)),
        "api" => match value.parse::<u32>() {
            Ok(version) if version >= 1 => header.api = Some(version),
            _ => header.notes.push(trf!(
                "Line {}: the api line wants a whole number from 1, such as -- api: 1.",
                line
            )),
        },
        "count" if is_setting_name(value) => header.count = Some(value.to_string()),
        "count" => header.notes.push(trf!(
            "Line {}: the count line wants the name of one of the script's settings, such as -- count: segments.",
            line
        )),
        "asks" => {
            let wanted = value.to_lowercase();
            match Ask::ALL.iter().find(|(_, words)| *words == wanted) {
                Some((ask, _)) if !header.asks.contains(ask) => header.asks.push(*ask),
                Some(_) => {}
                None => header.notes.push(trf!(
                    "Line {}: a script can ask to {}, to {} or to {}.",
                    line,
                    tr(Ask::ALL[0].1),
                    tr(Ask::ALL[1].1),
                    tr(Ask::ALL[2].1)
                )),
            }
        }
        _ => header.notes.push(trf!("Line {}: the {} line is empty.", line, key)),
    }
}

/// Whether `name` can be a setting's name: a letter, then letters, digits
/// and underscores.
fn is_setting_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `text` cut to at most `most` characters, with "…" if cut.
pub(crate) fn capped(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod tests;
