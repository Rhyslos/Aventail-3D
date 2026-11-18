//! Scripts as commands (plan, milestone 4; [decision 0305](../../../docs/decisions/0305-scripts-as-commands.md)):
//! the commands your library's scripts make, read on a thread of their
//! own whenever a script's file changes, and what you've set up for them
//! (kept between sessions): the time and memory limits, which commands
//! sit on the command palette, the keys given to them, the scripts turned
//! off, and each command's settings as last used.
//!
//! A script's command is named by its file and its command's name in the
//! script ([`CommandKey`]); a script that makes no commands is one
//! command of its own, running the whole file.

use script::library::Library;
use script::library::Stamp;
use script::{Described, Limits, Names, ScriptFile, Setting, SettingKind, SettingValue};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// How often the library is looked at for scripts added, changed or
/// removed.
const LOOK_EVERY: Duration = Duration::from_secs(2);

/// The shortest and longest time limit Settings → Scripts offers, in
/// seconds (choice 5: from 1 second to 10 minutes).
pub(crate) const TIME_RANGE: std::ops::RangeInclusive<u32> = 1..=600;
/// The smallest and largest memory limit it offers, in megabytes.
pub(crate) const MEMORY_RANGE: std::ops::RangeInclusive<u32> = 32..=2048;

/// The most commands kept on the command palette, keys and remembered
/// settings each: more is refused (a damaged settings file is cut).
pub(crate) const MAX_KEPT: usize = 200;

/// The most Ctrl + wheel takes a script command's count to.
pub(crate) const MAX_WHEEL_COUNT: i64 = 1_000;

/// A script's command, by its file's name in your library (or in the
/// project) and the command's name in the script (`None`: the whole
/// script, for one that makes no commands).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct CommandKey {
    pub(crate) file: String,
    pub(crate) command: Option<String>,
    /// Whether the script is one kept with the project (decision 0307),
    /// not one in your library.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) project: bool,
}

/// A key given to a script's command, kept by the key's name so the
/// settings file reads plainly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedKey {
    pub(crate) key: SavedKeyName,
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
}

/// A key's name, as egui names it (`F6`, `K`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SavedKeyName(pub(crate) egui::Key);

impl Serialize for SavedKeyName {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.0.name())
    }
}

impl<'de> Deserialize<'de> for SavedKeyName {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        egui::Key::from_name(&name)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom(r"not a key"))
    }
}

impl SavedKey {
    /// The key as egui checks it.
    pub(crate) fn shortcut(self) -> egui::KeyboardShortcut {
        let modifiers = egui::Modifiers {
            alt: self.alt,
            shift: self.shift,
            ..if self.ctrl {
                egui::Modifiers::COMMAND
            } else {
                egui::Modifiers::NONE
            }
        };
        egui::KeyboardShortcut::new(modifiers, self.key.0)
    }

    /// The key from a press.
    pub(crate) fn from_press(key: egui::Key, modifiers: egui::Modifiers) -> Self {
        Self {
            key: SavedKeyName(key),
            ctrl: modifiers.command || modifiers.ctrl,
            shift: modifiers.shift,
            alt: modifiers.alt,
        }
    }
}

/// A setting's value as the settings file keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum SavedValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Choice(String),
    Vec3([f64; 3]),
    Colour([f64; 4]),
}

impl From<&SettingValue> for SavedValue {
    fn from(value: &SettingValue) -> Self {
        match value {
            SettingValue::Bool(b) => Self::Bool(*b),
            SettingValue::Int(n) => Self::Int(*n),
            SettingValue::Float(n) => Self::Float(*n),
            SettingValue::Text(t) => Self::Text(t.clone()),
            SettingValue::Choice(c) => Self::Choice(c.clone()),
            SettingValue::Vec3(v) => Self::Vec3(*v),
            SettingValue::Colour(c) => Self::Colour(*c),
        }
    }
}

impl From<&SavedValue> for SettingValue {
    fn from(value: &SavedValue) -> Self {
        match value {
            SavedValue::Bool(b) => Self::Bool(*b),
            SavedValue::Int(n) => Self::Int(*n),
            SavedValue::Float(n) => Self::Float(*n),
            SavedValue::Text(t) => Self::Text(t.clone()),
            SavedValue::Choice(c) => Self::Choice(c.clone()),
            SavedValue::Vec3(v) => Self::Vec3(*v),
            SavedValue::Colour(c) => Self::Colour(*c),
        }
    }
}

/// What you've set up for scripts, kept between sessions and shared by
/// every tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptSettings {
    /// The longest a run may take, in seconds.
    pub(crate) time_seconds: u32,
    /// The most memory a run may use, in megabytes.
    pub(crate) memory_mb: u32,
    /// The commands on the command palette, in order.
    pub(crate) palette: Vec<CommandKey>,
    /// The keys given to commands.
    pub(crate) keys: Vec<(CommandKey, SavedKey)>,
    /// The scripts turned off in the script manager, by file name: their
    /// commands leave the menus, the palette and their keys.
    pub(crate) off: Vec<String>,
    /// Each command's settings as last used.
    pub(crate) values: Vec<(CommandKey, Vec<(String, SavedValue)>)>,
    /// What you allowed scripts to reach (decision 0307).
    pub(crate) approvals: Vec<crate::script_sharing::Approval>,
    /// The fingerprints of the project scripts you turned on (decision
    /// 0307).
    pub(crate) trusted: Vec<String>,
    /// The moments each script runs by itself at, once switched on in the
    /// script manager (decision 0309).
    pub(crate) moments: Vec<(crate::script_sharing::ScriptId, Vec<SavedMoment>)>,
}

/// A moment a script runs by itself at, as the settings file keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SavedMoment {
    /// A project was opened.
    Opened,
    /// A project is being saved.
    Saved,
    /// An export is starting.
    Exporting,
}

impl From<script::Moment> for SavedMoment {
    fn from(moment: script::Moment) -> Self {
        match moment {
            script::Moment::ProjectOpened => Self::Opened,
            script::Moment::ProjectSaved => Self::Saved,
            script::Moment::ExportStarting => Self::Exporting,
        }
    }
}

impl Default for ScriptSettings {
    fn default() -> Self {
        let limits = Limits::default();
        Self {
            time_seconds: limits.time.as_secs() as u32,
            memory_mb: (limits.memory >> 20) as u32,
            palette: Vec::new(),
            keys: Vec::new(),
            off: Vec::new(),
            values: Vec::new(),
            approvals: Vec::new(),
            trusted: Vec::new(),
            moments: Vec::new(),
        }
    }
}

impl ScriptSettings {
    /// The settings with every value in range and every list within
    /// the most kept (200): a settings file changed by hand or damaged never
    /// gives the app anything it can't use.
    #[must_use]
    pub fn checked(mut self) -> Self {
        self.time_seconds = self
            .time_seconds
            .clamp(*TIME_RANGE.start(), *TIME_RANGE.end());
        self.memory_mb = self
            .memory_mb
            .clamp(*MEMORY_RANGE.start(), *MEMORY_RANGE.end());
        self.palette.truncate(MAX_KEPT);
        self.keys.truncate(MAX_KEPT);
        self.off.truncate(script::library::MAX_LISTED);
        self.values.truncate(MAX_KEPT);
        for (_, values) in &mut self.values {
            values.truncate(script::MAX_SETTINGS);
        }
        let mut seen = Vec::new();
        self.keys.retain(|(key, saved)| {
            let fresh = !seen.contains(saved);
            seen.push(*saved);
            fresh && !key.file.is_empty()
        });
        self.palette.dedup();
        let most = crate::script_sharing::MAX_TRUSTED;
        self.approvals.truncate(most);
        self.trusted.truncate(most);
        self.trusted
            .retain(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()));
        let mut seen = Vec::new();
        self.approvals.retain(|a| {
            let fresh = !seen.contains(&a.script);
            seen.push(a.script.clone());
            fresh
        });
        // Commands of project scripts keep no keys or palette places.
        self.keys.retain(|(k, _)| !k.project);
        self.palette.retain(|k| !k.project);
        self.moments.truncate(most);
        let mut seen = Vec::new();
        self.moments.retain_mut(|(id, moments)| {
            moments.dedup();
            moments.truncate(script::Moment::ALL.len());
            let fresh = !seen.contains(id) && !moments.is_empty();
            seen.push(id.clone());
            fresh
        });
        self
    }

    /// The limits runs get.
    pub(crate) fn limits(&self) -> Limits {
        Limits {
            time: Duration::from_secs(u64::from(self.time_seconds.max(1))),
            memory: (self.memory_mb.max(1) as usize) << 20,
        }
    }

    /// The key given to `command`, if any.
    pub(crate) fn key_of(&self, command: &CommandKey) -> Option<SavedKey> {
        self.keys
            .iter()
            .find(|(k, _)| k == command)
            .map(|(_, key)| *key)
    }

    /// The command given `key`, if any.
    pub(crate) fn command_with(&self, key: SavedKey) -> Option<&CommandKey> {
        self.keys.iter().find(|(_, k)| *k == key).map(|(c, _)| c)
    }

    /// Whether `command` is on the command palette.
    pub(crate) fn on_palette(&self, command: &CommandKey) -> bool {
        self.palette.contains(command)
    }

    /// Puts `command` on the command palette, or takes it off.
    pub(crate) fn set_on_palette(&mut self, command: &CommandKey, on: bool) {
        self.palette.retain(|k| k != command);
        if on && self.palette.len() < MAX_KEPT {
            self.palette.push(command.clone());
        }
    }

    /// The settings `command` was last used with, by name.
    pub(crate) fn values_of(&self, command: &CommandKey) -> Vec<(String, SettingValue)> {
        self.values
            .iter()
            .find(|(k, _)| k == command)
            .map(|(_, v)| v.iter().map(|(n, v)| (n.clone(), v.into())).collect())
            .unwrap_or_default()
    }

    /// Remembers the settings `command` was used with.
    pub(crate) fn remember(&mut self, command: &CommandKey, values: &[(String, SettingValue)]) {
        self.values.retain(|(k, _)| k != command);
        if self.values.len() >= MAX_KEPT {
            self.values.remove(0);
        }
        self.values.push((
            command.clone(),
            values
                .iter()
                .take(script::MAX_SETTINGS)
                .map(|(n, v)| (n.clone(), v.into()))
                .collect(),
        ));
    }

    /// The script `file` was renamed to `to`: everything kept for its
    /// commands follows it.
    pub(crate) fn renamed(&mut self, file: &str, to: &str) {
        let follow = |key: &mut CommandKey| {
            if key.file == file && !key.project {
                key.file = to.to_string();
            }
        };
        self.palette.iter_mut().for_each(follow);
        self.keys.iter_mut().for_each(|(k, _)| follow(k));
        self.values.iter_mut().for_each(|(k, _)| follow(k));
        for off in &mut self.off {
            if off == file {
                *off = to.to_string();
            }
        }
        let library = crate::script_sharing::ScriptId::Library(file.to_string());
        let renamed = crate::script_sharing::ScriptId::Library(to.to_string());
        for approval in &mut self.approvals {
            if approval.script == library {
                approval.script = renamed.clone();
            }
        }
        for (id, _) in &mut self.moments {
            if *id == library {
                *id = renamed.clone();
            }
        }
    }

    /// The script `file` was removed: what was kept for it goes.
    pub(crate) fn removed(&mut self, file: &str) {
        let ours = |k: &CommandKey| k.file == file && !k.project;
        self.palette.retain(|k| !ours(k));
        self.keys.retain(|(k, _)| !ours(k));
        self.values.retain(|(k, _)| !ours(k));
        self.off.retain(|f| f != file);
        let id = crate::script_sharing::ScriptId::Library(file.to_string());
        self.take_back(&id);
        self.moments.retain(|(m, _)| *m != id);
    }

    /// Whether the script `file` is turned off.
    pub(crate) fn is_off(&self, file: &str) -> bool {
        self.off.iter().any(|f| f == file)
    }
}

/// A command's run under way, and what it replaces.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandRun {
    pub(crate) key: CommandKey,
    /// Its name, as the undo step is called.
    pub(crate) name: String,
    /// The settings it runs with.
    pub(crate) values: Vec<(String, SettingValue)>,
    /// The earlier run it replaces, taken back before this one started
    /// (when a setting changed in Attributes): done again if this one
    /// fails, so the last good result stays.
    pub(crate) replaces: Option<LiveCommand>,
    /// Newer settings, changed while it ran: it's stopped, and runs again
    /// with these once it ends.
    pub(crate) next: Option<Vec<(String, SettingValue)>>,
}

/// The command last run, whose settings show in Attributes while its run
/// is still the last thing done and nothing else has been selected
/// (as a tool's Last edit, decision 0120).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiveCommand {
    pub(crate) key: CommandKey,
    pub(crate) name: String,
    pub(crate) values: Vec<(String, SettingValue)>,
    /// Whether its run changed anything (one undo step to take back).
    pub(crate) landed: bool,
    /// The document's count of changes, its selection and its element
    /// revision just after the run landed.
    pub(crate) changes_made: u64,
    pub(crate) selection: Vec<scene::ObjectId>,
    pub(crate) elements: u64,
}

impl LiveCommand {
    /// Whether nothing has happened to `document` since the run landed.
    pub(crate) fn still_last(&self, document: &scene::Document) -> bool {
        document.changes_made() == self.changes_made
            && document.redo_steps() == 0
            && document.selection_order() == self.selection.as_slice()
            && document.element_revision() == self.elements
    }
}

/// A command being given a key: its window waits for a key press.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeyAsk {
    pub(crate) key: CommandKey,
    pub(crate) name: String,
    /// Why the last key pressed was refused.
    pub(crate) refused: Option<String>,
}

/// The script manager window's own state.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Manager {
    pub(crate) open: bool,
    /// A script being renamed: its path, the name typed and why the last
    /// one was refused.
    pub(crate) renaming: Option<(PathBuf, String, Option<String>)>,
    /// A script whose removal is being asked about.
    pub(crate) removing: Option<PathBuf>,
    /// What the last thing done said.
    pub(crate) note: Option<String>,
}

/// One script in your library, with what reading its commands found.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LibraryScript {
    pub(crate) path: PathBuf,
    /// Its file's name.
    pub(crate) file: String,
    stamp: Option<Stamp>,
    /// What reading it found (`None` until read).
    pub(crate) described: Option<Described>,
}

impl LibraryScript {
    /// A script at `path`, called `file`, with what reading it found.
    pub(crate) fn reading(path: PathBuf, file: String, described: Option<Described>) -> Self {
        Self {
            path,
            file,
            stamp: None,
            described,
        }
    }

    /// Its name in words: its header's, or its file's without the ending.
    pub(crate) fn shown_name(&self) -> String {
        match &self.described {
            Some(d) => d.header.shown_name(&self.file),
            None => script::read_header("").shown_name(&self.file),
        }
    }
}

/// A command one of your scripts makes, ready to show and run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScriptCommand {
    pub(crate) key: CommandKey,
    /// Its name, as menus show it.
    pub(crate) name: String,
    /// What it does: its script's header's `about`.
    pub(crate) about: String,
    pub(crate) settings: Vec<Setting>,
    /// The setting Ctrl + wheel changes (its header's `count`), if it's
    /// a whole number.
    pub(crate) count: Option<String>,
    pub(crate) path: PathBuf,
}

impl ScriptCommand {
    /// Its settings' values: those remembered when they still fit, the
    /// starting ones otherwise.
    pub(crate) fn values(&self, settings: &ScriptSettings) -> Vec<(String, SettingValue)> {
        let kept = settings.values_of(&self.key);
        self.settings
            .iter()
            .filter_map(|s| {
                let value = kept
                    .iter()
                    .find(|(n, v)| *n == s.name && v.fits(&s.kind))
                    .map(|(_, v)| v.clone())
                    .or_else(|| s.start.clone())?;
                Some((s.name.clone(), value))
            })
            .collect()
    }
}

/// Your library's scripts and their commands, read again on a thread of
/// their own when a file changes.
#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) scripts: Vec<LibraryScript>,
    looked_at: Option<Instant>,
    reading: Option<Receiver<(PathBuf, Described)>>,
    /// Paths being read now.
    pending: Vec<PathBuf>,
}

impl Clone for Registry {
    /// A copy for another tab: what was read, without the reading under
    /// way (the other tab looks again itself).
    fn clone(&self) -> Self {
        Self {
            scripts: self.scripts.clone(),
            ..Self::default()
        }
    }
}

impl Registry {
    /// Looks at `library` when due, and starts reading the scripts that
    /// changed. Gives back whether anything new was taken in.
    pub(crate) fn refresh(
        &mut self,
        library: Option<&Library>,
        names: &Names,
        limits: Limits,
        now: Instant,
    ) -> bool {
        let mut changed = self.take_in();
        let Some(library) = library else {
            return changed;
        };
        if self.reading.is_some()
            || self
                .looked_at
                .is_some_and(|at| now.duration_since(at) < LOOK_EVERY)
        {
            return changed;
        }
        self.looked_at = Some(now);
        let Ok(found) = library.scripts() else {
            return changed;
        };
        let before = self.scripts.len();
        self.scripts.retain(|s| found.contains(&s.path));
        changed |= self.scripts.len() != before;
        let mut to_read = Vec::new();
        for path in found {
            let stamp = script::library::stamp(&path);
            match self.scripts.iter_mut().find(|s| s.path == path) {
                Some(known) if known.stamp == stamp => {}
                Some(known) => {
                    known.stamp = stamp;
                    to_read.push(path);
                }
                None => {
                    self.scripts.push(LibraryScript {
                        file: file_name(&path),
                        path: path.clone(),
                        stamp,
                        described: None,
                    });
                    changed = true;
                    to_read.push(path);
                }
            }
        }
        self.scripts.sort_by_key(|s| s.file.to_lowercase());
        if !to_read.is_empty() {
            self.read(to_read, names.clone(), limits);
        }
        changed
    }

    /// Reads `paths` again now, on a thread of their own.
    pub(crate) fn read_again(&mut self, path: &Path, names: &Names, limits: Limits) {
        if self.reading.is_some() {
            // Looked at again once the reading under way ends.
            if let Some(known) = self.scripts.iter_mut().find(|s| s.path == path) {
                known.stamp = None;
            }
            self.looked_at = None;
            return;
        }
        self.read(vec![path.to_path_buf()], names.clone(), limits);
    }

    fn read(&mut self, paths: Vec<PathBuf>, names: Names, limits: Limits) {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.pending.clone_from(&paths);
        let started = std::thread::Builder::new()
            .name(r"script commands".into())
            .stack_size(16 << 20)
            .spawn(move || {
                for path in paths {
                    let described = match ScriptFile::read(&path) {
                        Ok(file) => script::commands_of(&file, &names, limits),
                        Err(why) => Described {
                            problems: vec![script::Problem {
                                file: file_name(&path),
                                line: 0,
                                column: 0,
                                message: why,
                            }],
                            ..Described::default()
                        },
                    };
                    if sender.send((path, described)).is_err() {
                        return;
                    }
                }
            });
        if started.is_ok() {
            self.reading = Some(receiver);
        }
    }

    /// Takes in what the reading thread found. Gives back whether there
    /// was anything.
    fn take_in(&mut self) -> bool {
        let Some(reading) = &self.reading else {
            return false;
        };
        let mut any = false;
        loop {
            match reading.try_recv() {
                Ok((path, described)) => {
                    any = true;
                    self.pending.retain(|p| *p != path);
                    if let Some(known) = self.scripts.iter_mut().find(|s| s.path == path) {
                        known.described = Some(described);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.reading = None;
                    self.pending.clear();
                    break;
                }
            }
        }
        any
    }

    /// Forgets the script at `path` (renamed or removed): the next look
    /// finds what's there now.
    pub(crate) fn forget(&mut self, path: &Path) {
        self.scripts.retain(|s| s.path != path);
        self.looked_at = None;
    }

    /// Waits for the reading under way (for tests).
    #[cfg(test)]
    pub(crate) fn wait(&mut self) {
        while self.reading.is_some() {
            self.take_in();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Every command of the scripts that are turned on, by name.
    pub(crate) fn commands(&self, settings: &ScriptSettings) -> Vec<ScriptCommand> {
        let mut all: Vec<ScriptCommand> = self
            .scripts
            .iter()
            .filter(|s| !settings.is_off(&s.file))
            .flat_map(commands_of)
            .collect();
        // Two with the same name are told apart by their files.
        let names: Vec<String> = all.iter().map(|c| c.name.clone()).collect();
        for command in &mut all {
            if names.iter().filter(|n| **n == command.name).count() > 1 {
                command.name = format!(r"{} ({})", command.name, command.key.file);
            }
        }
        all.sort_by_key(|c| c.name.to_lowercase());
        all
    }
}

/// The commands one script makes (none while it has mistakes or hasn't
/// been read yet).
pub(crate) fn commands_of(script: &LibraryScript) -> Vec<ScriptCommand> {
    let Some(described) = &script.described else {
        return Vec::new();
    };
    if !described.problems.is_empty() {
        return Vec::new();
    }
    let header = &described.header;
    let shown = header.shown_name(&script.file);
    let about = header.about.clone().unwrap_or_default();
    let count_of = |settings: &[Setting]| {
        header.count.clone().filter(|count| {
            settings
                .iter()
                .any(|s| s.name == *count && s.kind == SettingKind::Int)
        })
    };
    if described.commands.is_empty() {
        return vec![ScriptCommand {
            key: CommandKey {
                file: script.file.clone(),
                command: None,
                project: false,
            },
            name: shown,
            about,
            settings: Vec::new(),
            count: None,
            path: script.path.clone(),
        }];
    }
    let one = described.commands.len() == 1;
    described
        .commands
        .iter()
        .map(|c| ScriptCommand {
            key: CommandKey {
                file: script.file.clone(),
                command: Some(c.name.clone()),
                project: false,
            },
            name: if one && header.name.is_some() {
                shown.clone()
            } else if one {
                c.words.clone()
            } else {
                format!(r"{shown}: {}", c.words)
            },
            about: about.clone(),
            count: count_of(&c.settings),
            settings: c.settings.clone(),
            path: script.path.clone(),
        })
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "script_commands_tests.rs"]
mod tests;
