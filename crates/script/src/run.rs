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
use crate::output::{Ending, Event, Landing, Output};
use crate::problem::{Problem, lua_problem, lust_problem};
use crate::sandbox::{Halt, Sandbox, as_text};
use crate::commands::{Choice, set_values};
use crate::{Limits, Names, ScriptFile};
use mlua::{Function, Table, Value as LuaValue};
use scene::{Change, Document, HistoryLimit, ObjectId, Scene};
use std::cell::{Cell, RefCell};
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
    /// How `shape:unwrap()` unwraps: the app's UV settings, as the UV
    /// Editor's Unwrap uses them.
    pub unwrap: scene::UnwrapOptions,
}

/// A script running on a thread of its own.
pub struct Run {
    events: Receiver<Event>,
    stop: Arc<AtomicBool>,
    ended: bool,
}

impl Drop for Run {
    /// A run no one listens to any more (its tab was closed) stops.
    fn drop(&mut self) {
        self.stop();
    }
}

impl Run {
    /// Starts `file` running against `snapshot`: its start, then each of
    /// its commands once with its starting settings.
    pub fn start(file: ScriptFile, snapshot: Snapshot, limits: Limits, names: Names) -> Self {
        Self::begin(file, snapshot, limits, names, None)
    }

    /// Starts one command of `file` running against `snapshot`, with the
    /// settings `choice` gives it (milestone 4): the script's start, then
    /// that command alone.
    pub fn start_command(
        file: ScriptFile,
        snapshot: Snapshot,
        limits: Limits,
        names: Names,
        choice: Choice,
    ) -> Self {
        Self::begin(file, snapshot, limits, names, Some(choice))
    }

    fn begin(
        file: ScriptFile,
        snapshot: Snapshot,
        limits: Limits,
        names: Names,
        choice: Option<Choice>,
    ) -> Self {
        let (sender, events) = std::sync::mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let theirs = stop.clone();
        let failed = sender.clone();
        let started = std::thread::Builder::new()
            .name(r"script run".into())
            .stack_size(16 << 20)
            .spawn(move || {
                let output = Rc::new(Output::new(sender));
                let ending = run_here(&file, snapshot, limits, &names, theirs, &output, choice);
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
    mut snapshot: Snapshot,
    limits: Limits,
    names: &Names,
    stop: Arc<AtomicBool>,
    output: &Rc<Output>,
    choice: Option<Choice>,
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
        Some(version) if version < API_VERSION => output.note(trf!(
            "{} was written for scripting version {}; this app has {}. It runs with that version's calls, but the output says which ones to change.",
            file.name,
            version,
            API_VERSION
        )),
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
        document: RefCell::new(copy_of(&mut snapshot)),
        snapshot,
        changes: RefCell::new(Vec::new()),
        built: Cell::new(0),
        spoilt: Cell::new(false),
        output: output.clone(),
        lua: matches!(prepared, Prepared::Lua),
        folder: names.folder.clone(),
    });
    if let Err(err) = lend(&sandbox, lent.clone()) {
        output.mistake(Problem::whole(&file.name, err.to_string()));
        return Ending::Failed;
    }
    let ran = match &prepared {
        Prepared::Lust(chunk) => {
            for note in &chunk.notes {
                output.note(Problem::from(note.clone()).to_string());
            }
            run_lust(&sandbox, chunk, choice.as_ref())
                .map_err(|err| (lust_problem(chunk, &file.name, &err), err))
        }
        Prepared::Lua => run_lua(&sandbox, file, output, choice.as_ref())
            .map_err(|err| (lua_problem(&file.name, &err), err)),
    };
    match ran {
        Ok(()) if lent.spoilt.get() => {
            output.note(trf!(
                "{} carried on after a change couldn't be made, so nothing it did was kept.",
                file.name
            ));
            Ending::Failed
        }
        Ok(()) => {
            let changes = lent.changes.take();
            if !changes.is_empty() {
                let scene = lent.document.borrow().scene().clone();
                output.send(Event::Land(Box::new(Landing { changes, scene })));
            }
            Ending::Finished
        }
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

/// The run's own copy of the document: the snapshot's scene, selection
/// and selected elements, keeping no undo history of its own.
pub(crate) fn copy_of(snapshot: &mut Snapshot) -> Document {
    let scene = std::mem::take(&mut snapshot.scene);
    let mut document = Document::from_scene(scene, snapshot.selection.iter().copied());
    document.set_history_limit(HistoryLimit::Bytes(0));
    document
        .execute_batch(
            snapshot
                .elements
                .iter()
                .map(|(id, elements)| Change::SelectElements(*id, elements.clone())),
            "",
        )
        .ok();
    document
}

/// Runs a LustScript's start, and gives back what it makes for the app
/// (its components and how to make them).
pub(crate) fn lust_start(sandbox: &Sandbox, chunk: &lust::LuaChunk) -> mlua::Result<Table> {
    let env = sandbox.env()?;
    let start = as_text(sandbox.lua.load(&chunk.code))
        .set_name(lust::LuaChunk::NAME)
        .set_environment(env)
        .into_function()?;
    start.call(())
}

/// The names in `table`, in order.
pub(crate) fn sorted_names(table: &Table) -> Vec<String> {
    let mut names: Vec<String> = table
        .pairs::<String, LuaValue>()
        .filter_map(Result::ok)
        .map(|(name, _)| name)
        .collect();
    names.sort();
    names
}

/// Runs a LustScript's start, then each of its components with a
/// `run()`, in name order; or only the one `choice` names, with its
/// settings.
fn run_lust(
    sandbox: &Sandbox,
    chunk: &lust::LuaChunk,
    choice: Option<&Choice>,
) -> mlua::Result<()> {
    let api = lust_start(sandbox, chunk)?;
    let components: Table = api.raw_get("components")?;
    let new: Function = api.raw_get("new")?;
    let luau_vectors = api.raw_get::<Option<String>>("vectors")?.as_deref() == Some("luau");
    let names = match choice {
        Some(choice) => vec![choice.command.clone()],
        None => sorted_names(&components),
    };
    for name in names {
        let Some(class) = components.raw_get::<Option<Table>>(name.as_str())? else {
            return Err(mlua::Error::runtime(missing(&name)));
        };
        let Some(run) = class.raw_get::<Option<Function>>("run")? else {
            continue;
        };
        let command: Table = new.call((name, LuaValue::Nil))?;
        if let Some(choice) = choice {
            let settings = crate::commands::lust_settings(&class, &command)?;
            set_values(&sandbox.lua, &command, &settings, choice, luau_vectors)?;
        }
        run.call::<()>(command)?;
    }
    Ok(())
}

/// Runs a Lua script's own code from the top.
pub(crate) fn lua_start(sandbox: &Sandbox, file: &ScriptFile) -> mlua::Result<()> {
    let env = sandbox.env()?;
    as_text(sandbox.lua.load(&file.text))
        .set_name(format!("={}", file.name))
        .set_environment(env)
        .exec()
}

/// Runs a Lua script's start, then each command it made, in order; or
/// only the one `choice` names, with its settings.
fn run_lua(
    sandbox: &Sandbox,
    file: &ScriptFile,
    output: &Output,
    choice: Option<&Choice>,
) -> mlua::Result<()> {
    lua_start(sandbox, file)?;
    let made: Table = sandbox.commands.call(())?;
    let mut found = false;
    for class in made.sequence_values::<Table>() {
        let class = class?;
        let name: String = class.raw_get("__name")?;
        if choice.is_some_and(|c| c.command != name) || (found && choice.is_some()) {
            continue;
        }
        found = true;
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
        if let Some(choice) = choice {
            let settings = crate::commands::lua_settings(&settings);
            set_values(&sandbox.lua, &command, &settings, choice, true)?;
        }
        command.set_metatable(Some(class))?;
        run.call::<()>(command)?;
    }
    match choice {
        Some(choice) if !found => Err(mlua::Error::runtime(missing(&choice.command))),
        _ => Ok(()),
    }
}

/// What's said when the command chosen isn't in the script any more.
fn missing(command: &str) -> String {
    trf!(
        "The script has no command called {} any more, so nothing was run. Choose one of its commands again from the Scripts menu.",
        command
    )
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
