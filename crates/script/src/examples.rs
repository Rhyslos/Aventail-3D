//! The example scripts (plan, milestone 6; [decision 0309](../../../docs/decisions/0309-event-scripts-and-examples.md)):
//! a handful of LustScript files that show what scripts are for, each
//! explained in the manual's Scripting page. Scripts → Example scripts
//! adds one to your library, where it can be read, run and changed.
//!
//! They're kept as files in `src/examples/`, with `%folder%` where the
//! scripting folder's name goes (`fetch "%folder%/scene"`), so the app's
//! name is never written in them.

use crate::{Names, ScriptFile};

/// The examples' files, by name, as kept in `src/examples/`.
const FILES: &[(&str, &str)] = &[
    (
        r"Rename by pattern.lsc",
        include_str!("examples/Rename by pattern.lsc"),
    ),
    (r"Gear.lsc", include_str!("examples/Gear.lsc")),
    (
        r"Check before export.lsc",
        include_str!("examples/Check before export.lsc"),
    ),
    (
        r"Export each shape.lsc",
        include_str!("examples/Export each shape.lsc"),
    ),
    (
        r"Select polygons facing up.lsc",
        include_str!("examples/Select polygons facing up.lsc"),
    ),
    (r"Tidy names.lsc", include_str!("examples/Tidy names.lsc")),
];

/// The example scripts, in the order the Scripts menu lists them, with
/// the scripting folder's name from `names` in their `fetch` lines.
pub fn examples(names: &Names) -> Vec<ScriptFile> {
    FILES
        .iter()
        .map(|(name, text)| ScriptFile {
            name: (*name).to_string(),
            text: text.replace(r"%folder%", &names.folder),
        })
        .collect()
}

#[cfg(test)]
#[path = "examples_tests.rs"]
mod tests;
