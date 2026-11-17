//! Running a script on a thread of its own, against a copy of the scene
//! (choice 4), so the app never waits for it: what it says comes back as
//! [`Event`]s, and Stop works at any moment.
//!
//! A run: read the header (nothing runs), check and compile ([`crate::compile`]),
//! make a fresh sandbox, lend it the calls, run the script's own start,
//! then each command it made (a LustScript `component` with a `run()`, or
//! Lua's `command(…)` with a `run` function) once, with its settings as
//! they start, in name order for LustScript and in the order made for Lua.

use crate::compile::{Prepared, prepare};
use crate::describe::API_VERSION;
use crate::header::read_header;
use crate::natives::{Lent, lend};
use crate::output::{Ending, Event, Output};
use crate::problem::{Problem, lua_problem, lust_problem};
use crate::sandbox::{Halt, Sandbox, as_text};
use crate::{Limits, Names, ScriptFile};
use mlua::{Function, Table, Value as LuaValue};
use scene::{ObjectId, Scene};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use words::trf;

/// The app's modes, as scripts see them (`Selection:mode()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Whole objects.
    #[default]
    Model,
    /// Whole objects and their pivots.
    Object,
    /// Points.
    Points,
    /// Edges.
    Edges,
    /// Polygons.
    Polygons,
    /// UVs.
    Uv,
    /// Painting.
    Paint,
    /// Material graphs.
    Material,
}

impl Mode {
    /// Its option's name in the `Mode` state.
    pub fn name(self) -> &'static str {
        match self {
            Self::Model => r"Model",
            Self::Object => r"Object",
            Self::Points => r"Points",
            Self::Edges => r"Edges",
            Self::Polygons => r"Polygons",
            Self::Uv => r"Uv",
            Self::Paint => r"Paint",
            Self::Material => r"Material",
        }
    }
}

/// What a script runs against: a copy of the project as it was when the
/// script started.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// The scene, copied with what's worked out (`Scene::copy_with_results`).
    pub scene: Scene,
    /// The selected objects.
    pub selection: Vec<ObjectId>,
    /// The selected points, edges and polygons, by object.
    pub elements: Vec<(ObjectId, mesh::ElementSelection)>,
    /// The mode the app was in.
    pub mode: Mode,
    /// The project's name, as its tab shows it.
    pub project_name: String,
    /// The folder it's saved in (empty if it isn't saved yet).
    pub project_folder: String,
}

/// A script running on a thread of its own.
pub struct Run {
    events: Receiver<Event>,
    stop: Arc<AtomicBool>,
    ended: bool,
}

impl Run {
    /// Starts `file` running against `snapshot`.
    pub fn start(file: ScriptFile, snapshot: Snapshot, limits: Limits, names: Names) -> Self {
        let (sender, events) = std::sync::mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let theirs = stop.clone();
        let failed = sender.clone();
        let started = std::thread::Builder::new()
            .name(r"script run".into())
            .stack_size(16 << 20)
            .spawn(move || {
                let output = Rc::new(Output::new(sender));
                let ending = run_here(&file, snapshot, limits, &names, theirs, &output);
                output.send(Event::Ended(ending));
            });
        if let Err(why) = started {
            let _ = failed.send(Event::Line(crate::output::Line::Note(trf!(
                "The script couldn't start: there was no room ({}).",
                why
            ))));
            let _ = failed.send(Event::Ended(Ending::Failed));
        }
        Self {
            events,
            stop,
            ended: false,
        }
    }

    /// Asks the script to stop: it stops at its next step, and its ending
    /// says [`Ending::Stopped`].
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Whether it was asked to stop.
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// What it has said since this was last asked, in order.
    pub fn events(&mut self) -> Vec<Event> {
        let mut found = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    self.ended |= matches!(event, Event::Ended(_));
                    found.push(event);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !self.ended {
                        self.ended = true;
                        found.push(Event::Ended(Ending::Failed));
                    }
                    break;
                }
            }
        }
        found
    }

    /// Whether it has ended (its last event was [`Event::Ended`]).
    pub fn has_ended(&self) -> bool {
        self.ended
    }

    /// Waits for it to end, and gives every event it said (for tests and
    /// the command line).
    pub fn wait(mut self) -> Vec<Event> {
        let mut all = Vec::new();
        while !self.ended {
            match self.events.recv() {
                Ok(event) => {
                    self.ended |= matches!(event, Event::Ended(_));
                    all.push(event);
                }
                Err(_) => {
                    self.ended = true;
                    all.push(Event::Ended(Ending::Failed));
                }
            }
        }
        all
    }
}

/// One run, on this thread.
fn run_here(
    file: &ScriptFile,
    snapshot: Snapshot,
    limits: Limits,
    names: &Names,
    stop: Arc<AtomicBool>,
    output: &Rc<Output>,
) -> Ending {
    let header = read_header(&file.text);
    for note in &header.notes {
        output.note(format!(r"{}: {note}", file.name));
    }
    match header.api {
        Some(version) if version > API_VERSION => {
            output.mistake(Problem::whole(
                &file.name,
                trf!(
                    "This script needs a newer version of the app (it was written for scripting version {}; this app has {}).",
                    version,
                    API_VERSION
                ),
            ));
            return Ending::Failed;
        }
        Some(_) => {}
        None => output.note(trf!(
            "{} doesn't say which scripting version it was written for (an -- api: {} line at its top), so it's taken as version {}.",
            file.name,
            API_VERSION,
            API_VERSION
        )),
    }
    let prepared = match prepare(file, names) {
        Ok(prepared) => prepared,
        Err(problems) => {
            for problem in problems {
                output.mistake(problem);
            }
            return Ending::Failed;
        }
    };
    if stop.load(Ordering::Relaxed) {
        return Ending::Stopped;
    }
    let sandbox = match Sandbox::new(limits, stop, names) {
        Ok(sandbox) => sandbox,
        Err(err) => {
            output.mistake(Problem::whole(
                &file.name,
                trf!(
                    "The script couldn't start: the sandbox couldn't be made ({}).",
                    err
                ),
            ));
            return Ending::Failed;
        }
    };
    let lent = Rc::new(Lent {
        snapshot,
        output: output.clone(),
        lua: matches!(prepared, Prepared::Lua),
        folder: names.folder.clone(),
    });
    if let Err(err) = lend(&sandbox, lent) {
        output.mistake(Problem::whole(&file.name, err.to_string()));
        return Ending::Failed;
    }
    let ran = match &prepared {
        Prepared::Lust(chunk) => {
            for note in &chunk.notes {
                output.note(Problem::from(note.clone()).to_string());
            }
            run_lust(&sandbox, chunk).map_err(|err| (lust_problem(chunk, &file.name, &err), err))
        }
        Prepared::Lua => {
            run_lua(&sandbox, file, output).map_err(|err| (lua_problem(&file.name, &err), err))
        }
    };
    match ran {
        Ok(()) => Ending::Finished,
        Err((problem, err)) => match sandbox.clock.halted() {
            Some(Halt::TimeUp) => Ending::OutOfTime,
            Some(Halt::Asked) => Ending::Stopped,
            None if out_of_memory(&err) => Ending::OutOfMemory,
            None => {
                output.mistake(problem);
                Ending::Failed
            }
        },
    }
}

/// Runs a LustScript's start, then each of its components with a
/// `run()`, in name order.
fn run_lust(sandbox: &Sandbox, chunk: &lust::LuaChunk) -> mlua::Result<()> {
    let env = sandbox.env()?;
    let start = as_text(sandbox.lua.load(&chunk.code))
        .set_name(lust::LuaChunk::NAME)
        .set_environment(env)
        .into_function()?;
    let api: Table = start.call(())?;
    let components: Table = api.raw_get("components")?;
    let new: Function = api.raw_get("new")?;
    let mut names: Vec<String> = components
        .pairs::<String, LuaValue>()
        .filter_map(Result::ok)
        .map(|(name, _)| name)
        .collect();
    names.sort();
    for name in names {
        let class: Table = components.raw_get(name.as_str())?;
        let Some(run) = class.raw_get::<Option<Function>>("run")? else {
            continue;
        };
        let command: LuaValue = new.call((name, LuaValue::Nil))?;
        run.call::<()>(command)?;
    }
    Ok(())
}

/// Runs a Lua script's start, then each command it made, in order.
fn run_lua(sandbox: &Sandbox, file: &ScriptFile, output: &Output) -> mlua::Result<()> {
    let env = sandbox.env()?;
    as_text(sandbox.lua.load(&file.text))
        .set_name(format!("={}", file.name))
        .set_environment(env)
        .exec()?;
    let made: Table = sandbox.commands.call(())?;
    for class in made.sequence_values::<Table>() {
        let class = class?;
        let name: String = class.raw_get("__name")?;
        let Some(run) = class.raw_get::<Option<Function>>("run")? else {
            output.note(trf!(
                "The command {} has no run function, so it was left alone. Give it one: function {}:run() … end",
                name,
                name
            ));
            continue;
        };
        let settings: Table = class.raw_get("__settings")?;
        let command = sandbox.lua.create_table()?;
        for pair in settings.pairs::<LuaValue, LuaValue>() {
            let (key, value) = pair?;
            command.raw_set(key, value)?;
        }
        command.set_metatable(Some(class))?;
        run.call::<()>(command)?;
    }
    Ok(())
}

/// Whether `err` is the memory limit's.
fn out_of_memory(err: &mlua::Error) -> bool {
    match err {
        mlua::Error::MemoryError(_) => true,
        mlua::Error::CallbackError { cause, .. } => out_of_memory(cause),
        other => other.to_string().contains("not enough memory"),
    }
}

/// How a run ended, in plain words, for the output and the status bar.
pub fn ending_words(ending: &Ending, script: &str, limits: Limits) -> String {
    match ending {
        Ending::Finished => trf!("{} finished.", script),
        Ending::Failed => trf!("{} stopped on a mistake.", script),
        Ending::Stopped => trf!("{} was stopped.", script),
        Ending::OutOfTime => trf!(
            "{} was stopped after {} seconds, the longest a script may run. Is there a loop that never ends?",
            script,
            limits.time.as_secs()
        ),
        Ending::OutOfMemory => trf!(
            "{} was stopped: it used more than {} MB of memory, the most a script may use.",
            script,
            limits.memory >> 20
        ),
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
pub(crate) mod tests;
