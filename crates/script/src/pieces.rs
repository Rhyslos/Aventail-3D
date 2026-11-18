//! Running part of a script (plan, milestone 3): only the lines selected
//! in the editor (Ctrl + Enter), or what was typed on the console line.
//! Both become a [`ScriptFile`] for [`crate::Run::start`], so they run
//! exactly as a whole script does: on a copy of the scene, in a fresh
//! sandbox, landing as one undo step.

use crate::{API_VERSION, Language, Names, ScriptFile};

/// The console's file name for LustScript, as mistakes show it.
pub const CONSOLE_LUSTSCRIPT: &str = "console.lsc";
/// The console's file name for Lua.
pub const CONSOLE_LUA: &str = "console.lua";

/// Only lines `first` to `last` of `file` (counting from 1), ready to run
/// on their own. Every other line is left blank, so a mistake is still
/// shown at its own line, except the lines a part can't run without:
/// the header comments at the top (its name and scripting version) and,
/// in LustScript, the `fetch` lines. A LustScript part that doesn't end
/// with `end` gets one.
pub fn only_lines(file: &ScriptFile, first: usize, last: usize) -> ScriptFile {
    let lustscript = file.language() == Some(Language::LustScript);
    let mut in_header = true;
    let mut ended = false;
    let mut kept: Vec<&str> = Vec::new();
    for (n, line) in file.text.lines().enumerate() {
        let n = n + 1;
        let trimmed = line.trim();
        in_header &= trimmed.is_empty() || trimmed.starts_with("--");
        let chosen = (first..=last).contains(&n);
        let needed = in_header || lustscript && trimmed.starts_with("fetch ");
        if chosen || needed {
            kept.push(line);
            if chosen && !trimmed.is_empty() && !trimmed.starts_with("--") {
                ended = trimmed == "end";
            }
        } else {
            kept.push("");
        }
    }
    // Lines past the selection aren't needed at all.
    while kept.last().is_some_and(|l| l.is_empty()) {
        kept.pop();
    }
    let mut text = kept.join("\n");
    text.push('\n');
    if lustscript && !ended {
        text.push_str("end\n");
    }
    ScriptFile {
        name: file.name.clone(),
        text,
    }
}

/// What was typed on the console line as a script in `language`, and how
/// many lines were put before it (to take off the lines of its mistakes).
/// LustScript gets everything the app offers fetched, and its `end`; both
/// say the scripting version, so no note is given about it.
pub fn console_script(typed: &str, language: Language, names: &Names) -> (ScriptFile, u32) {
    let typed = typed.trim_end();
    match language {
        Language::LustScript => {
            let mut text = format!(
                "-- api: {API_VERSION}\n{}\n{typed}\n",
                crate::describe::fetch_everything(names)
            );
            let ends = typed.lines().next_back().is_some_and(|l| l.trim() == "end");
            if !ends {
                text.push_str("end\n");
            }
            (
                ScriptFile {
                    name: CONSOLE_LUSTSCRIPT.into(),
                    text,
                },
                2,
            )
        }
        Language::Lua => (
            ScriptFile {
                name: CONSOLE_LUA.into(),
                text: format!("-- api: {API_VERSION}\n{typed}\n"),
            },
            1,
        ),
    }
}

#[cfg(test)]
#[path = "pieces_tests.rs"]
mod tests;
