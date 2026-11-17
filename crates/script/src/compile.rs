//! Checking a script before it runs, on a thread of its own, one check at
//! a time ([`crate::turns`]): a LustScript file with the app's description
//! files through Lust's compiler, which gives the Lua to run; a Lua file
//! through Luau's own reading of it. Every mistake comes back with its
//! file, line and column.

use crate::describe::descriptions;
use crate::problem::{Problem, lua_problem};
use crate::turns::{CHECK_LIMIT, Turns};
use crate::{Language, Names, ScriptFile};
use words::trf;

/// Whose turn it is to check a script, in this program.
static TURNS: Turns = Turns::new();

/// A script, checked and ready to run.
#[derive(Debug, Clone)]
pub(crate) enum Prepared {
    /// LustScript, turned into Lua by the compiler.
    Lust(lust::LuaChunk),
    /// A Lua file, as it is.
    Lua,
}

/// Checks `file`: every mistake found (none when it's sound).
pub fn check(file: &ScriptFile, names: &Names) -> Vec<Problem> {
    prepare(file, names).err().unwrap_or_default()
}

/// Checks `file` and makes it ready to run, on a thread of its own: a
/// check past [`CHECK_LIMIT`] is given up on.
pub(crate) fn prepare(file: &ScriptFile, names: &Names) -> Result<Prepared, Vec<Problem>> {
    let name = file.name.clone();
    match file.language() {
        Some(Language::LustScript) => {
            let mut files = vec![(name.clone(), file.text.replace("\r\n", "\n"))];
            files.extend(descriptions(names));
            let folder = names.folder.clone();
            let entry = name.clone();
            TURNS.check(&name, CHECK_LIMIT, move || {
                compile_lust(&entry, &folder, &files).map(Prepared::Lust)
            })
        }
        Some(Language::Lua) => {
            let text = file.text.clone();
            let entry = name.clone();
            TURNS.check(&name, CHECK_LIMIT, move || {
                check_lua(&entry, &text).map(|()| Prepared::Lua)
            })
        }
        None => Err(vec![Problem::whole(
            &name,
            trf!(
                "{} isn't a script: scripts end in .lsc (LustScript) or .lua (Lua).",
                name
            ),
        )]),
    }
}

/// The LustScript turned into Lua, or its mistakes. A crash inside the
/// compiler is a mistake too.
fn compile_lust(
    entry: &str,
    folder: &str,
    files: &[(String, String)],
) -> Result<lust::LuaChunk, Vec<Problem>> {
    let engine = lust::Engine {
        folders: vec![folder.to_string()],
    };
    match std::panic::catch_unwind(|| lust::to_lua_with(&engine, entry, files)) {
        Ok(Ok(chunk)) => Ok(chunk),
        Ok(Err(found)) => Err(found.into_iter().map(Problem::from).collect()),
        Err(_) => Err(vec![compiler_stopped(entry)]),
    }
}

/// A Lua file read by Luau, without running it.
fn check_lua(entry: &str, text: &str) -> Result<(), Vec<Problem>> {
    let lua = crate::sandbox::sandboxed_lua().map_err(|_| vec![compiler_stopped(entry)])?;
    crate::sandbox::as_text(lua.load(text))
        .set_name(format!("={entry}"))
        .into_function()
        .map(|_| ())
        .map_err(|err| vec![lua_problem(entry, &err)])
}

/// What's said when the compiler itself crashed.
pub(crate) fn compiler_stopped(file: &str) -> Problem {
    Problem::whole(
        file,
        trf!(
            "the Lust compiler ({}) stopped unexpectedly. This is a bug in Lust; please report it with the script that caused it.",
            lust::VERSION
        ),
    )
}
