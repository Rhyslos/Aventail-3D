//! Mistakes in plain words, with the place in the script they're at
//! (plan, "Mistakes in plain words"). Lust's own messages are already
//! written this way; a Lua error is turned back into the script's file and
//! line (adapted from Solis's `lua_problem`).

use crate::header::capped;

/// A mistake in a script: where it is, and what's wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The file, as the script's own name (`bevel.lsc`) or the app's
    /// description file it's in (`aventail/scene.lsc`); empty when it's
    /// about no file.
    pub file: String,
    /// Its line, from 1 (0 when it isn't at a line).
    pub line: u32,
    /// Its column, from 1 (0 when not known).
    pub column: u32,
    /// What's wrong, in plain words.
    pub message: String,
}

/// The most characters kept of a mistake's message: a script's `error`
/// can throw any text.
pub(crate) const MAX_MESSAGE_CHARS: usize = 1_000;

impl Problem {
    /// A mistake about `file` as a whole.
    pub(crate) fn whole(file: &str, message: String) -> Self {
        Self {
            file: file.to_string(),
            line: 0,
            column: 0,
            message,
        }
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            _ if self.file.is_empty() => write!(f, "{}", self.message),
            (0, _) => write!(f, "{}: {}", self.file, self.message),
            (line, 0) => write!(f, "{}:{line}  {}", self.file, self.message),
            (line, column) => write!(f, "{}:{line}:{column}  {}", self.file, self.message),
        }
    }
}

impl From<lust::Problem> for Problem {
    fn from(p: lust::Problem) -> Self {
        Self {
            file: p.file,
            line: p.line,
            column: p.column,
            message: capped(&p.message, MAX_MESSAGE_CHARS),
        }
    }
}

/// A Lua error's place and message: the script's own file and line when
/// the error names one, or the first line of the script among the calls
/// that led to it (an error raised by one of the app's calls names no
/// place itself). `file` is the script's name.
pub(crate) fn lua_problem(file: &str, err: &mlua::Error) -> Problem {
    let text = err.to_string();
    let first = first_line(&text);
    let mine = |path: &str| path == file;
    let (line, message) = match place(first) {
        Some((path, line, message)) if mine(path) => (line, message),
        Some((_, _, message)) => (called_from(&text, &mine).unwrap_or(0), message),
        None => (called_from(&text, &mine).unwrap_or(0), first),
    };
    Problem {
        file: file.to_string(),
        line,
        column: 0,
        message: capped(message, MAX_MESSAGE_CHARS),
    }
}

/// A Lust chunk's error, located in the LustScript: where the error says
/// it is, or the first line of the script among the calls that led to it.
pub(crate) fn lust_problem(chunk: &lust::LuaChunk, entry: &str, err: &mlua::Error) -> Problem {
    let text = err.to_string();
    let located = chunk.locate(&text);
    let message = capped(&located.message, MAX_MESSAGE_CHARS);
    let message = message
        .strip_prefix("runtime error: ")
        .or_else(|| message.strip_prefix("callback error: "))
        .unwrap_or(&message)
        .to_string();
    if let Some(line) = located.line {
        return Problem {
            file: located.file,
            line,
            column: located.column.unwrap_or(0),
            message,
        };
    }
    // The calls that led to it, innermost first, each `lust:812: in …`:
    // the first that lands in the script (not Lust's own runtime).
    for line in text.lines().skip(1) {
        let Some(at) = line.find("lust:") else {
            continue;
        };
        let found = chunk.locate(&format!("{}: here", &line[at..]));
        if let Some(n) = found.line {
            return Problem {
                file: found.file,
                line: n,
                column: found.column.unwrap_or(0),
                message,
            };
        }
    }
    Problem::whole(entry, message)
}

/// An error's first line, without the library's own words before it.
fn first_line(text: &str) -> &str {
    let mut first = text.lines().next().unwrap_or("").trim();
    for prefix in ["runtime error: ", "syntax error: ", "callback error: "] {
        first = first.strip_prefix(prefix).unwrap_or(first);
    }
    first
}

/// The first line naming the script among the calls that led to an
/// error (each `bevel.lua:5: in function …`).
fn called_from(text: &str, mine: &dyn Fn(&str) -> bool) -> Option<u32> {
    text.lines()
        .skip(1)
        .filter_map(|line| place(line.trim()))
        .find(|(path, _, _)| mine(path))
        .map(|(_, line, _)| line)
}

/// The place a line of an error starts with, and what follows it:
/// `bevel.lua:5: attempt to …` is `bevel.lua`, 5 and the rest.
fn place(line: &str) -> Option<(&str, u32, &str)> {
    line.match_indices(':').find_map(|(i, _)| {
        let rest = &line[i + 1..];
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let after = rest[digits..].strip_prefix(": ").or_else(|| {
            // The end of a traceback line: `bevel.lua:5` alone.
            rest[digits..].is_empty().then_some("")
        })?;
        Some((&line[..i], rest[..digits].parse().ok()?, after))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_read_from_error_lines() {
        assert_eq!(
            place("tidy.lua:5: attempt to call a nil value"),
            Some(("tidy.lua", 5, "attempt to call a nil value"))
        );
        assert_eq!(place("no place here"), None);
        assert_eq!(place("tidy.lua:x: no"), None);
    }

    #[test]
    fn problems_are_shown_with_their_place() {
        let p = Problem {
            file: "tidy.lua".into(),
            line: 3,
            column: 0,
            message: "oops".into(),
        };
        assert_eq!(p.to_string(), "tidy.lua:3  oops");
        let p = Problem::whole("", "just words".into());
        assert_eq!(p.to_string(), "just words");
    }
}
