//! The Luau a script runs in, made safe for scripts from strangers
//! (plan, "The sandbox"). Adapted from Solis's `luau.rs` (the project
//! lead's own MIT code, Solis decision 0007):
//!
//! - **One Luau per run**, made fresh, so nothing one script leaves
//!   behind reaches another.
//! - **No reaching out:** no `io`, `os`, `debug`, `require`, `loadstring`,
//!   `getfenv` or `setfenv` (Luau has no `package` or bytecode loading of
//!   its own; text is always loaded as text). The libraries, the app's
//!   global and its calls are read-only.
//! - **Time:** the run's time limit, checked by mlua's own, safe
//!   interrupt (Solis sets Luau's directly with `unsafe` code, for game
//!   code's speed; the app's no-`unsafe` rule stands, decision 0005).
//!   Once past, every interrupt stops the script, so catching the error
//!   with `pcall` in a loop doesn't hold it off. Stop (and Esc) work the
//!   same way.
//! - **Memory:** at most the run's memory limit.

use crate::describe::{CALLS, OWNERS, OwnerKind, STATES};
use crate::{Limits, Names};
use mlua::{Function, Lua, Table, Value as LuaValue};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// The app's own Lua, run before any script.
const PRELUDE: &str = include_str!("prelude.lua");

/// Why a script was stopped from outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Halt {
    /// It ran past its time limit.
    TimeUp,
    /// Someone pressed Stop (or Esc).
    Asked,
}

/// The run's deadline and Stop, which the interrupt asks.
pub(crate) struct Clock {
    /// When the run must end.
    until: Instant,
    /// Set when someone asks the run to stop.
    stop: Arc<AtomicBool>,
    /// Why it was stopped, once it was: from then on every interrupt
    /// stops it again.
    halted: Cell<Option<Halt>>,
    /// Interrupts since the clock was last read.
    since: Cell<u32>,
}

/// How often the interrupt reads the clock: once in this many.
const READ_EVERY: u32 = 64;

impl Clock {
    /// A clock for a run of `limits.time` from now, stopped early by
    /// `stop`.
    pub(crate) fn new(limits: Limits, stop: Arc<AtomicBool>) -> Self {
        Self {
            until: Instant::now() + limits.time,
            stop,
            halted: Cell::new(None),
            since: Cell::new(0),
        }
    }

    /// Why the run must stop now, if it must.
    pub(crate) fn verdict(&self) -> Option<Halt> {
        if let Some(halt) = self.halted.get() {
            return Some(halt);
        }
        if self.stop.load(Ordering::Relaxed) {
            self.halted.set(Some(Halt::Asked));
            return Some(Halt::Asked);
        }
        let since = self.since.get() + 1;
        if since < READ_EVERY {
            self.since.set(since);
            return None;
        }
        self.since.set(0);
        if Instant::now() > self.until {
            self.halted.set(Some(Halt::TimeUp));
            return Some(Halt::TimeUp);
        }
        None
    }

    /// Why the run was stopped, if it was.
    pub(crate) fn halted(&self) -> Option<Halt> {
        self.halted.get()
    }
}

/// What Luau's safe libraries give that scripts must not have: `os`
/// (the clock and date) and `debug` are left out as the plan says;
/// `require` reads `.lua` files from disk; `loadstring` compiles code into
/// the shared globals; `getfenv` and `setfenv` reach and replace any
/// function's globals.
const TAKEN_OUT: [&str; 6] = ["os", "debug", "require", "loadstring", "getfenv", "setfenv"];

/// A Luau with only the libraries scripts may use.
pub(crate) fn sandboxed_lua() -> mlua::Result<Lua> {
    let lua = Lua::new_with(mlua::StdLib::ALL_SAFE, mlua::LuaOptions::default())?;
    let globals = lua.globals();
    for name in TAKEN_OUT {
        globals.raw_remove(name)?;
    }
    Ok(lua)
}

/// A script's text, loaded as text, never as compiled bytecode (Luau takes
/// text starting with a low byte for bytecode, and its bytecode loader
/// isn't made for hostile input).
pub(crate) fn as_text(chunk: mlua::chunk::Chunk<'_>) -> mlua::chunk::Chunk<'_> {
    chunk.set_mode(mlua::chunk::ChunkMode::Text)
}

/// The Luau one run uses, with its limits set and the app's parts made.
pub(crate) struct Sandbox {
    pub(crate) lua: Lua,
    pub(crate) clock: Rc<Clock>,
    /// The natives, by key (`Scene_selected`), filled in by
    /// [`crate::natives`] and then made read-only.
    pub(crate) natives: Table,
    /// The shared, read-only globals.
    globals: Table,
    /// The metatable of each script's own globals: reads fall through to
    /// the shared ones.
    through: Table,
    /// `wrap(kind, id)`: the value for a thing the app hands over.
    pub(crate) wrap: Function,
    /// `unwrap(value)`: a thing's id and kind, or nils.
    pub(crate) unwrap: Function,
    /// `commands()`: the commands Lua's `command` made.
    pub(crate) commands: Function,
}

impl Sandbox {
    /// A fresh Luau for one run.
    pub(crate) fn new(limits: Limits, stop: Arc<AtomicBool>, names: &Names) -> mlua::Result<Self> {
        let lua = sandboxed_lua()?;
        lua.set_memory_limit(limits.memory)?;
        let clock = Rc::new(Clock::new(limits, stop));
        let watching = clock.clone();
        lua.set_interrupt(move |_| match watching.verdict() {
            None => Ok(mlua::VmState::Continue),
            Some(halt) => Err(mlua::Error::runtime(halted_words(halt))),
        });
        let globals = lua.globals();
        let natives = lua.create_table()?;
        let objects: Vec<&str> = owners(OwnerKind::Object);
        let things: Vec<&str> = owners(OwnerKind::Thing);
        let states = lua.create_sequence_from(
            STATES
                .iter()
                .map(|s| lua.create_sequence_from([LuaValue::String(lua.create_string(s.name)?), LuaValue::Table(lua.create_sequence_from(s.options.iter().copied())?)]))
                .collect::<mlua::Result<Vec<Table>>>()?,
        )?;
        let folder = global_name(&names.folder);
        // All of the app's own Lua is made here, before any script runs,
        // so it holds the libraries' real functions.
        let (wrap, unwrap, commands): (Function, Function, Function) = as_text(lua.load(PRELUDE))
            .set_name("=app")
            .call((natives.clone(), folder, objects, things, states))?;
        lua.sandbox(true)?;
        let through = lua.create_table()?;
        through.raw_set("__index", &globals)?;
        through.raw_set("__metatable", "globals")?;
        through.set_readonly(true);
        Ok(Self {
            lua,
            clock,
            natives,
            globals,
            through,
            wrap,
            unwrap,
            commands,
        })
    }

    /// Makes the natives read-only, once they're all in.
    pub(crate) fn seal(&self) {
        self.natives.set_readonly(true);
    }

    /// Globals of their own for one script: what it sets stays there, and
    /// reads fall through to the shared ones. Lust finds its natives in
    /// them (`LUST_NATIVES`).
    pub(crate) fn env(&self) -> mlua::Result<Table> {
        let env = self.lua.create_table()?;
        env.set_metatable(Some(self.through.clone()))?;
        env.raw_set("_G", &env)?;
        // Lust looks for Luau's vectors with rawget(_G, "vector").
        let vector: LuaValue = self.globals.raw_get("vector")?;
        env.raw_set("vector", vector)?;
        env.raw_set("LUST_NATIVES", &self.natives)?;
        Ok(env)
    }
}

/// The names of the owners of one kind, in order.
fn owners(kind: OwnerKind) -> Vec<&'static str> {
    OWNERS
        .iter()
        .filter(|o| o.kind == kind && CALLS.iter().any(|c| c.owner == o.name))
        .map(|o| o.name)
        .collect()
}

/// The app's global for Lua: the scripting folder's name, if it can be a
/// Lua name (the branding test checks it can); otherwise `app`.
fn global_name(folder: &str) -> &str {
    let mut chars = folder.chars();
    let fits = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if fits { folder } else { "app" }
}

/// The words a stopped script's error carries (the run's ending is told
/// from the clock, never from these).
fn halted_words(halt: Halt) -> String {
    match halt {
        Halt::TimeUp => words::tr("the script ran out of time, so it was stopped").to_string(),
        Halt::Asked => words::tr("the script was stopped").to_string(),
    }
}

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod tests;
