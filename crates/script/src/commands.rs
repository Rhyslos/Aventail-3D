//! A script's commands and their settings (plan, milestone 4;
//! [decision 0305](../../../docs/decisions/0305-scripts-as-commands.md)).
//!
//! A LustScript `component` with a `run()` method, or Lua's
//! `command("Name", { settings })` with a `run` function, is a command:
//! its fields (or the table's entries) are its settings, which the app
//! shows in Attributes. [`commands_of`] finds them by running the script's
//! own start once, in a fresh sandbox, against an empty scene: nothing it
//! does lands, and nothing outside the sandbox is reached. A run of one
//! command with chosen settings is [`crate::Run::start_command`] with a
//! [`Choice`].

use crate::compile::{Prepared, prepare};
use crate::describe::API_VERSION;
use crate::header::{Header, read_header};
use crate::natives::{Lent, lend};
use crate::output::{Event, Line, Output};
use crate::problem::{Problem, lua_problem, lust_problem};
use crate::run::{Snapshot, copy_of};
use crate::sandbox::Sandbox;
use crate::{Limits, Names, ScriptFile};
use mlua::{Function, Table, Value as LuaValue};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use words::trf;

/// The longest a script's start may take while its commands are read:
/// it's read again whenever the file changes, so it must be quick.
pub const DESCRIBE_LIMIT: Duration = Duration::from_secs(2);

/// The most commands read from one script.
pub const MAX_COMMANDS: usize = 64;

/// The most settings read from one command.
pub const MAX_SETTINGS: usize = 64;

/// The most characters kept of a text setting.
pub(crate) const MAX_TEXT_CHARS: usize = 1_000;

/// What kind of value a setting holds.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingKind {
    /// Yes or no.
    Bool,
    /// A whole number.
    Int,
    /// A number with decimals.
    Float,
    /// Text.
    Text,
    /// One of a Lust state's options, by name.
    Choice(Vec<String>),
    /// Three numbers: x, y and z.
    Vec3,
    /// A colour: red, green, blue and how solid, each 0 to 1.
    Colour,
    /// A kind Attributes can't show (a rotation, a vec2, a shape): the
    /// setting keeps its starting value. Its kind's name, as Lust says it.
    Other(String),
}

/// A setting's value.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    /// Yes or no.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A number with decimals.
    Float(f64),
    /// Text.
    Text(String),
    /// A state's option, by name.
    Choice(String),
    /// Three numbers: x, y and z.
    Vec3([f64; 3]),
    /// Red, green, blue and how solid, each 0 to 1.
    Colour([f64; 4]),
}

impl SettingValue {
    /// Whether it can be a value of `kind`.
    pub fn fits(&self, kind: &SettingKind) -> bool {
        match (self, kind) {
            (Self::Bool(_), SettingKind::Bool)
            | (Self::Int(_), SettingKind::Int)
            | (Self::Float(_), SettingKind::Float)
            | (Self::Text(_), SettingKind::Text)
            | (Self::Vec3(_), SettingKind::Vec3)
            | (Self::Colour(_), SettingKind::Colour) => true,
            (Self::Choice(option), SettingKind::Choice(options)) => options.contains(option),
            _ => false,
        }
    }
}

/// One of a command's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    /// Its name as the script writes it (`width`).
    pub name: String,
    /// Its name in words, for Attributes (`Width`).
    pub words: String,
    /// What kind of value it holds.
    pub kind: SettingKind,
    /// The value it starts with (`None` for [`SettingKind::Other`]).
    pub start: Option<SettingValue>,
    /// The Lua field it's kept in (Lust's may differ from its name).
    key: String,
}

/// A command a script makes.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandInfo {
    /// Its name as the script writes it (`BevelHardEdges`).
    pub name: String,
    /// Its name in words (`Bevel hard edges`).
    pub words: String,
    /// Its settings: in the order written for LustScript, by name for Lua
    /// (a Lua table keeps no order).
    pub settings: Vec<Setting>,
}

/// What reading a script's commands found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Described {
    /// Its header.
    pub header: Header,
    /// Its commands, in name order for LustScript and in the order made
    /// for Lua. Empty for a script that makes none (it runs whole).
    pub commands: Vec<CommandInfo>,
    /// Its mistakes: a script with mistakes has no commands, and can't
    /// be run from the menus until they're mended.
    pub problems: Vec<Problem>,
    /// Notes on what was left out (a second command of the same name, a
    /// setting named like the command's own `run`).
    pub notes: Vec<String>,
}

/// One command to run, with the settings chosen for it: settings it
/// doesn't have, or values of the wrong kind, are left at their start.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// The command's name, as the script writes it.
    pub command: String,
    /// Settings by name.
    pub values: Vec<(String, SettingValue)>,
}

/// A name as words: `BevelHardEdges`, `bevelHardEdges` and
/// `bevel_hard_edges` are all "Bevel hard edges".
pub fn words_of(name: &str) -> String {
    let mut out = String::new();
    let mut last: Option<char> = None;
    for c in name.chars() {
        if c == '_' || c == '-' {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
            last = Some(' ');
            continue;
        }
        let starts_word = c.is_uppercase()
            && last.is_some_and(|l| l.is_lowercase() || l.is_ascii_digit())
            && !out.ends_with(' ');
        if starts_word {
            out.push(' ');
        }
        if out.is_empty() {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
        last = Some(c);
    }
    out.trim().to_string()
}

/// Reads `file`'s commands and their settings: its start is run once
/// against an empty scene, for at most [`DESCRIBE_LIMIT`], and nothing it
/// does is kept. Runs on the caller's thread: the app calls it from a
/// thread of its own.
pub fn commands_of(file: &ScriptFile, names: &Names, limits: Limits) -> Described {
    let header = read_header(&file.text);
    let mut found = Described {
        header,
        ..Described::default()
    };
    if let Some(version) = found.header.api.filter(|v| *v > API_VERSION) {
        found.problems.push(Problem::whole(
            &file.name,
            trf!(
                "This script needs a newer version of the app (it was written for scripting version {}; this app has {}).",
                version,
                API_VERSION
            ),
        ));
        return found;
    }
    let prepared = match prepare(file, names) {
        Ok(prepared) => prepared,
        Err(problems) => {
            found.problems = problems;
            return found;
        }
    };
    let limits = Limits {
        time: limits.time.min(DESCRIBE_LIMIT),
        memory: limits.memory,
    };
    let (sender, events) = std::sync::mpsc::channel();
    let output = Rc::new(Output::new(sender));
    let read = read_commands(file, &prepared, names, limits, &output);
    drop(output);
    // What the start said is left out; its mistakes are kept.
    found
        .problems
        .extend(events.try_iter().filter_map(|e| match e {
            Event::Line(Line::Mistake(problem)) => Some(problem),
            _ => None,
        }));
    match read {
        Ok((commands, notes)) => {
            found.commands = commands;
            found.notes = notes;
        }
        Err(problem) => found.problems.push(problem),
    }
    found
}

/// Runs the start in a fresh sandbox and reads the commands it made, with
/// notes on what was left out.
fn read_commands(
    file: &ScriptFile,
    prepared: &Prepared,
    names: &Names,
    limits: Limits,
    output: &Rc<Output>,
) -> Result<(Vec<CommandInfo>, Vec<String>), Problem> {
    let sandbox = Sandbox::new(limits, Arc::new(AtomicBool::new(false)), names)
        .map_err(|e| Problem::whole(&file.name, e.to_string()))?;
    let mut empty = Snapshot::default();
    let lent = Rc::new(Lent {
        document: RefCell::new(copy_of(&mut empty)),
        snapshot: empty,
        changes: RefCell::new(Vec::new()),
        built: Cell::new(0),
        spoilt: Cell::new(false),
        output: output.clone(),
        lua: matches!(prepared, Prepared::Lua),
        folder: names.folder.clone(),
    });
    lend(&sandbox, lent).map_err(|e| Problem::whole(&file.name, e.to_string()))?;
    let mut notes = Vec::new();
    let read = match prepared {
        Prepared::Lust(chunk) => lust_commands(&sandbox, chunk, &mut notes)
            .map_err(|err| lust_problem(chunk, &file.name, &err)),
        Prepared::Lua => {
            lua_commands(&sandbox, file, &mut notes).map_err(|err| lua_problem(&file.name, &err))
        }
    };
    let stopped = sandbox.clock.halted().is_some();
    match read {
        Ok(commands) => Ok((commands, notes)),
        Err(_) if stopped => Err(Problem::whole(
            &file.name,
            trf!(
                "Its start took longer than {} seconds, so its commands couldn't be read. Move slow work into a command's run.",
                DESCRIBE_LIMIT.as_secs()
            ),
        )),
        Err(problem) => Err(problem),
    }
}

/// A LustScript's components with a `run()`, in name order.
fn lust_commands(
    sandbox: &Sandbox,
    chunk: &lust::LuaChunk,
    notes: &mut Vec<String>,
) -> mlua::Result<Vec<CommandInfo>> {
    let api = crate::run::lust_start(sandbox, chunk)?;
    let components: Table = api.raw_get("components")?;
    let new: Function = api.raw_get("new")?;
    let mut found = Vec::new();
    for name in crate::run::sorted_names(&components) {
        let class: Table = components.raw_get(name.as_str())?;
        if class.raw_get::<Option<Function>>("run")?.is_none() {
            continue;
        }
        if found.len() == MAX_COMMANDS {
            notes.push(trf!("Only its first {} commands are used.", MAX_COMMANDS));
            break;
        }
        let made: Table = new.call((name.as_str(), LuaValue::Nil))?;
        let schema: Option<Table> = class.get("__schema")?;
        let mut settings = Vec::new();
        for field in schema.iter().flat_map(|s| s.sequence_values::<Table>()) {
            let field = field?;
            let key: String = field.raw_get(1)?;
            let setting: String = field.raw_get(2)?;
            let kind: String = field.raw_get(3)?;
            let options: Option<Table> = field.raw_get(4)?;
            let kind = lust_kind(&kind, options)?;
            let start = value_of(&made.raw_get(key.as_str())?, &kind);
            push_setting(&mut settings, &name, key, setting, kind, start, notes);
        }
        found.push(CommandInfo {
            words: words_of(&name),
            name,
            settings,
        });
    }
    Ok(found)
}

/// A Lua script's commands, in the order made.
fn lua_commands(
    sandbox: &Sandbox,
    file: &ScriptFile,
    notes: &mut Vec<String>,
) -> mlua::Result<Vec<CommandInfo>> {
    crate::run::lua_start(sandbox, file)?;
    let made: Table = sandbox.commands.call(())?;
    let mut found: Vec<CommandInfo> = Vec::new();
    for class in made.sequence_values::<Table>() {
        let class = class?;
        let name: String = class.raw_get("__name")?;
        if class.raw_get::<Option<Function>>("run")?.is_none() {
            notes.push(trf!(
                "The command {} has no run function, so it isn't one of the script's commands.",
                name
            ));
            continue;
        }
        if found.iter().any(|c| c.name == name) {
            notes.push(trf!(
                "The script makes two commands called {}: only the first is used.",
                name
            ));
            continue;
        }
        if found.len() == MAX_COMMANDS {
            notes.push(trf!("Only its first {} commands are used.", MAX_COMMANDS));
            break;
        }
        let table: Table = class.raw_get("__settings")?;
        let mut pairs: Vec<(String, LuaValue)> = table
            .pairs::<LuaValue, LuaValue>()
            .filter_map(Result::ok)
            .filter_map(|(k, v)| match k {
                LuaValue::String(s) => Some((s.to_string_lossy(), v)),
                _ => None,
            })
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut settings = Vec::new();
        for (setting, value) in pairs {
            if setting == "run" || setting.starts_with("__") {
                notes.push(trf!(
                    "{}'s setting {} is left out: that name is the command's own.",
                    name,
                    setting
                ));
                continue;
            }
            let kind = lua_kind(&value);
            let start = value_of(&value, &kind);
            push_setting(
                &mut settings,
                &name,
                setting.clone(),
                setting,
                kind,
                start,
                notes,
            );
        }
        found.push(CommandInfo {
            words: words_of(&name),
            name,
            settings,
        });
    }
    Ok(found)
}

fn push_setting(
    settings: &mut Vec<Setting>,
    command: &str,
    key: String,
    name: String,
    kind: SettingKind,
    start: Option<SettingValue>,
    notes: &mut Vec<String>,
) {
    if settings.len() == MAX_SETTINGS {
        let note = trf!(
            "{} has more than {} settings: only the first are used.",
            command,
            MAX_SETTINGS
        );
        if !notes.contains(&note) {
            notes.push(note);
        }
        return;
    }
    let start = if matches!(kind, SettingKind::Other(_)) {
        None
    } else {
        start
    };
    settings.push(Setting {
        words: words_of(&name),
        name,
        kind,
        start,
        key,
    });
}

/// A Lust field's kind, from its schema entry.
fn lust_kind(kind: &str, options: Option<Table>) -> mlua::Result<SettingKind> {
    Ok(match kind {
        "bool" => SettingKind::Bool,
        "int" => SettingKind::Int,
        "float" => SettingKind::Float,
        "string" => SettingKind::Text,
        "vec3" => SettingKind::Vec3,
        "colour" => SettingKind::Colour,
        "state" => {
            let mut all = Vec::new();
            for option in options.iter().flat_map(|o| o.sequence_values::<String>()) {
                all.push(option?);
            }
            SettingKind::Choice(all)
        }
        other => SettingKind::Other(other.to_string()),
    })
}

/// A Lua setting's kind, from its starting value: a whole number is a
/// whole-number setting (choice 38).
fn lua_kind(value: &LuaValue) -> SettingKind {
    match value {
        LuaValue::Boolean(_) => SettingKind::Bool,
        LuaValue::Integer(_) => SettingKind::Int,
        LuaValue::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => SettingKind::Int,
        LuaValue::Number(_) => SettingKind::Float,
        LuaValue::String(_) => SettingKind::Text,
        LuaValue::Vector(_) => SettingKind::Vec3,
        LuaValue::Table(t) if numbers(t, &["x", "y", "z"]).is_some() => SettingKind::Vec3,
        LuaValue::Table(t) if numbers(t, &["r", "g", "b"]).is_some() => SettingKind::Colour,
        LuaValue::Table(_) => SettingKind::Other(words::tr("table").to_string()),
        _ => SettingKind::Other(words::tr("value").to_string()),
    }
}

/// The numbers in `table` at `keys`, if all are numbers.
fn numbers(table: &Table, keys: &[&str]) -> Option<Vec<f64>> {
    keys.iter()
        .map(|k| match table.raw_get::<LuaValue>(*k).ok()? {
            LuaValue::Integer(n) => Some(n as f64),
            LuaValue::Number(n) => Some(n),
            _ => None,
        })
        .collect()
}

/// `value` as a setting of `kind`, if it is one.
fn value_of(value: &LuaValue, kind: &SettingKind) -> Option<SettingValue> {
    let number = match value {
        LuaValue::Integer(n) => Some(*n as f64),
        LuaValue::Number(n) => Some(*n),
        _ => None,
    };
    match (kind, value) {
        (SettingKind::Bool, LuaValue::Boolean(b)) => Some(SettingValue::Bool(*b)),
        (SettingKind::Int, _) => number
            .filter(|n| n.is_finite() && n.abs() < 9.0e15)
            .map(|n| SettingValue::Int(n.round() as i64)),
        (SettingKind::Float, _) => number.filter(|n| n.is_finite()).map(SettingValue::Float),
        (SettingKind::Text, LuaValue::String(s)) => Some(SettingValue::Text(
            crate::header::capped(&s.to_string_lossy(), MAX_TEXT_CHARS),
        )),
        (SettingKind::Choice(options), LuaValue::String(s)) => {
            let option = s.to_string_lossy();
            // Lust gives a state's option by its full name ("Side.Left").
            let short = option.rsplit('.').next().unwrap_or(&option).to_string();
            options
                .iter()
                .find(|o| **o == option || **o == short)
                .map(|o| SettingValue::Choice(o.clone()))
        }
        (SettingKind::Vec3, LuaValue::Vector(v)) => Some(SettingValue::Vec3([
            f64::from(v.x()),
            f64::from(v.y()),
            f64::from(v.z()),
        ])),
        (SettingKind::Vec3, LuaValue::Table(t)) => {
            numbers(t, &["x", "y", "z"]).map(|n| SettingValue::Vec3([n[0], n[1], n[2]]))
        }
        (SettingKind::Colour, LuaValue::Table(t)) => {
            let rgb = numbers(t, &["r", "g", "b"])?;
            let a = numbers(t, &["a"]).map_or(1.0, |a| a[0]);
            Some(SettingValue::Colour([rgb[0], rgb[1], rgb[2], a]))
        }
        _ => None,
    }
}

/// Gives the command `made` the values `choice` has for its `settings`:
/// one it doesn't have, or of the wrong kind, is left at its start.
pub(crate) fn set_values(
    lua: &mlua::Lua,
    made: &Table,
    settings: &[Setting],
    choice: &Choice,
    luau_vectors: bool,
) -> mlua::Result<()> {
    for (name, value) in &choice.values {
        let Some(setting) = settings.iter().find(|s| s.name == *name) else {
            continue;
        };
        if !value.fits(&setting.kind) {
            continue;
        }
        let given = lua_value(lua, value, luau_vectors)?;
        made.raw_set(setting.key.as_str(), given)?;
    }
    Ok(())
}

/// A setting's value as Lua holds it.
fn lua_value(lua: &mlua::Lua, value: &SettingValue, luau_vectors: bool) -> mlua::Result<LuaValue> {
    Ok(match value {
        SettingValue::Bool(b) => LuaValue::Boolean(*b),
        SettingValue::Int(n) => LuaValue::Number(*n as f64),
        SettingValue::Float(n) => LuaValue::Number(if n.is_finite() { *n } else { 0.0 }),
        SettingValue::Text(text) => {
            LuaValue::String(lua.create_string(crate::header::capped(text, MAX_TEXT_CHARS))?)
        }
        SettingValue::Choice(option) => LuaValue::String(lua.create_string(option)?),
        SettingValue::Vec3([x, y, z]) if luau_vectors => {
            LuaValue::Vector(mlua::Vector::new(*x as f32, *y as f32, *z as f32))
        }
        SettingValue::Vec3([x, y, z]) => {
            let t = lua.create_table()?;
            t.raw_set("x", *x as f32)?;
            t.raw_set("y", *y as f32)?;
            t.raw_set("z", *z as f32)?;
            LuaValue::Table(t)
        }
        SettingValue::Colour(c) => {
            let t = lua.create_table()?;
            for (key, n) in ["r", "g", "b", "a"].into_iter().zip(c) {
                t.raw_set(key, n.clamp(0.0, 1.0))?;
            }
            LuaValue::Table(t)
        }
    })
}

/// The settings of a LustScript component, to set chosen values on: read
/// from its class's schema.
pub(crate) fn lust_settings(class: &Table, made: &Table) -> mlua::Result<Vec<Setting>> {
    let schema: Option<Table> = class.get("__schema")?;
    let mut settings = Vec::new();
    let mut notes = Vec::new();
    for field in schema.iter().flat_map(|s| s.sequence_values::<Table>()) {
        let field = field?;
        let key: String = field.raw_get(1)?;
        let name: String = field.raw_get(2)?;
        let kind = lust_kind(&field.raw_get::<String>(3)?, field.raw_get(4)?)?;
        let start = value_of(&made.raw_get(key.as_str())?, &kind);
        push_setting(&mut settings, "", key, name, kind, start, &mut notes);
    }
    Ok(settings)
}

/// The settings of a Lua command, to set chosen values on.
pub(crate) fn lua_settings(table: &Table) -> Vec<Setting> {
    let mut settings = Vec::new();
    let mut notes = Vec::new();
    for (key, value) in table.pairs::<String, LuaValue>().filter_map(Result::ok) {
        let kind = lua_kind(&value);
        let start = value_of(&value, &kind);
        push_setting(&mut settings, "", key.clone(), key, kind, start, &mut notes);
    }
    settings
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
