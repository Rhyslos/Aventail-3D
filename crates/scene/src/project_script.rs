//! Scripts kept with the project ([decision 0307](../../../docs/decisions/0307-keeping-sharing-and-permissions.md)):
//! a script copied into the project file so it travels with it.
//!
//! The scene doesn't know what a script says or does (that's the
//! `script` crate's and the interface's); it keeps each one's file name
//! and text, checked only for size and naming. Changing the list is a
//! step to undo ([`crate::Command::SetScripts`]). A project's scripts
//! never run until the person opening it turns them on.

/// The most scripts a project keeps.
pub const MAX_PROJECT_SCRIPTS: usize = 64;
/// The longest file name a kept script has, in bytes.
pub const MAX_PROJECT_SCRIPT_NAME: usize = 128;
/// The most text one kept script holds, in bytes (a script file's own
/// limit).
pub const MAX_PROJECT_SCRIPT_BYTES: usize = 1 << 20;

/// One script kept with the project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectScript {
    /// Its file's name, with its ending (`Bevel hard edges.lsc`).
    pub name: String,
    /// Its text.
    pub text: String,
}

impl ProjectScript {
    /// Whether its name can be kept: not empty, not too long, ending in
    /// `.lsc` or `.lua`, without a folder or a character no file name
    /// can have.
    pub fn name_is_valid(name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        !name.trim().is_empty()
            && name.len() <= MAX_PROJECT_SCRIPT_NAME
            && (lower.ends_with(".lsc") || lower.ends_with(".lua"))
            && lower.len() > 4
            && !name.starts_with('.')
            && !name.chars().any(|c| {
                c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
            })
    }
}

/// Whether `list` can be kept: not too many, each named well and within
/// size, no two with the same name (whatever their letters' case).
pub fn scripts_are_valid(list: &[ProjectScript]) -> bool {
    list.len() <= MAX_PROJECT_SCRIPTS
        && list.iter().enumerate().all(|(i, s)| {
            ProjectScript::name_is_valid(&s.name)
                && s.text.len() <= MAX_PROJECT_SCRIPT_BYTES
                && !list[..i]
                    .iter()
                    .any(|t| t.name.eq_ignore_ascii_case(&s.name))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Document};

    fn kept(name: &str, text: &str) -> ProjectScript {
        ProjectScript {
            name: name.into(),
            text: text.into(),
        }
    }

    #[test]
    fn scripts_change_in_one_undo_step() {
        let mut doc = Document::new();
        let steps = doc.undo_steps();
        doc.execute(Command::SetScripts(vec![kept(
            "Bevel.lsc",
            "-- name: Bevel",
        )]))
        .unwrap();
        assert_eq!(doc.scene().scripts().len(), 1);
        assert_eq!(doc.undo_steps(), steps + 1);
        doc.undo();
        assert!(doc.scene().scripts().is_empty());
        doc.redo();
        assert_eq!(doc.scene().scripts()[0].name, "Bevel.lsc");
    }

    #[test]
    fn the_same_list_makes_no_step() {
        let mut doc = Document::new();
        doc.execute(Command::SetScripts(vec![kept("A.lua", "")]))
            .unwrap();
        let steps = doc.undo_steps();
        doc.execute(Command::SetScripts(vec![kept("A.lua", "")]))
            .unwrap();
        assert_eq!(doc.undo_steps(), steps);
    }

    #[test]
    fn bad_lists_are_refused() {
        let too_many: Vec<_> = (0..=MAX_PROJECT_SCRIPTS)
            .map(|i| kept(&format!("{i}.lua"), ""))
            .collect();
        let long = "a".repeat(MAX_PROJECT_SCRIPT_NAME) + ".lua";
        let big = "x".repeat(MAX_PROJECT_SCRIPT_BYTES + 1);
        for list in [
            too_many,
            vec![kept("", "")],
            vec![kept(".lua", "")],
            vec![kept("notes.txt", "")],
            vec![kept("../up.lua", "")],
            vec![kept("C:\\x.lua", "")],
            vec![kept("tab\t.lua", "")],
            vec![kept(&long, "")],
            vec![kept("big.lua", &big)],
            vec![kept("Twice.lua", ""), kept("twice.LUA", "")],
        ] {
            let mut doc = Document::new();
            assert!(doc.execute(Command::SetScripts(list)).is_err());
            assert!(doc.scene().scripts().is_empty());
        }
    }

    #[test]
    fn opening_keeps_them_without_a_step() {
        let doc = Document::new().with_scripts(vec![kept("A.lsc", "end")]);
        assert_eq!(doc.scene().scripts().len(), 1);
        assert_eq!(doc.undo_steps(), 0);
    }
}
