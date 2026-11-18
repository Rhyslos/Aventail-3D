//! Running your scripts' commands (decision 0305): from the Scripts menu,
//! command search, the command palette or a key of their own, with their
//! settings as last used. While a command's run is still the last thing
//! done, its settings show in Attributes; changing one takes that run
//! back and runs the command again (one undo step, the newest change
//! winning), and Ctrl + wheel changes its count.

use super::Workbench;
use crate::script_commands::{CommandKey, CommandRun, LiveCommand, SavedKey, ScriptCommand};
use crate::scripts_panel::Row;
use crate::text::tr;
use egui::Ui;
use script::{Choice, Line, Run, ScriptFile, SettingKind, SettingValue};

/// What Attributes shows of a command whose run is the last thing done.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScriptView {
    pub(crate) name: String,
    pub(crate) settings: Vec<script::Setting>,
    pub(crate) values: Vec<(String, SettingValue)>,
    /// The setting Ctrl + wheel changes, if any.
    pub(crate) count: Option<String>,
    /// Whether it's running again now.
    pub(crate) running: bool,
}

impl Workbench {
    /// Runs one of your scripts' commands with its settings as last used.
    pub(crate) fn run_script_command(&mut self, key: &CommandKey) {
        let Some(command) = self.find_command(key) else {
            self.note = Some(
                tr("That script command isn't in your library any more (or its script is turned off).")
                    .to_string(),
            );
            return;
        };
        let values = command.values(&self.scripts.settings);
        self.start_command(&command, values, None);
    }

    /// The command `key` names, among those of scripts turned on.
    pub(crate) fn find_command(&self, key: &CommandKey) -> Option<ScriptCommand> {
        self.scripts.commands().into_iter().find(|c| c.key == *key)
    }

    /// Starts `command` with `values`; `replaces`: the run taken back
    /// for it.
    fn start_command(
        &mut self,
        command: &ScriptCommand,
        values: Vec<(String, SettingValue)>,
        replaces: Option<LiveCommand>,
    ) {
        if let Some(editor) = self
            .scripts
            .editor
            .as_mut()
            .filter(|e| e.path == command.path)
        {
            let _ = editor.save_now();
        }
        let file = match ScriptFile::read(&command.path) {
            Ok(file) => file,
            Err(why) => {
                self.scripts.push(Row::Line(Line::Note(why.clone())));
                self.note = Some(why);
                self.restore(replaces);
                return;
            }
        };
        let choice = command.key.command.clone().map(|name| Choice {
            command: name,
            values: values.clone(),
        });
        self.scripts.settings.remember(&command.key, &values);
        if !self.start_run(file, command.name.clone(), None, choice) {
            self.restore(replaces);
            return;
        }
        self.scripts.command_run = Some(CommandRun {
            key: command.key.clone(),
            name: command.name.clone(),
            values,
            replaces,
            next: None,
        });
    }

    /// Starts `file` running, called `name`, against a copy of the scene:
    /// one of its commands when `choice` names one. Gives back whether it
    /// started.
    pub(crate) fn start_run(
        &mut self,
        file: ScriptFile,
        name: String,
        console: Option<u32>,
        choice: Option<Choice>,
    ) -> bool {
        if self.scripts.is_running() {
            self.note = Some(
                tr("A script is already running: stop it first (Esc, or Stop in the status bar).")
                    .to_string(),
            );
            return false;
        }
        let console = console.map(|before| (file.name.clone(), before));
        let snapshot = self.script_snapshot();
        let limits = self.scripts.settings.limits();
        let names = self.scripts.names();
        let run = match choice {
            Some(choice) => Run::start_command(file, snapshot, limits, names, choice),
            None => Run::start(file, snapshot, limits, names),
        };
        self.scripts.limits = limits;
        self.document.hold();
        crate::layout::show_beside_coordinates(&mut self.dock, crate::panel::Panel::Scripts);
        self.note = Some(crate::trf!("Running {}… (Esc stops it)", name));
        self.scripts.start(run, name, console);
        true
    }

    /// A taken-back run done again: a new one failed, so the last good
    /// result stays (with its settings).
    fn restore(&mut self, replaces: Option<LiveCommand>) {
        let Some(mut old) = replaces else {
            return;
        };
        if old.landed && self.document.redo().is_none() {
            return;
        }
        self.scripts.settings.remember(&old.key, &old.values);
        old.changes_made = self.document.changes_made();
        old.selection = self.document.selection_order().to_vec();
        old.elements = self.document.element_revision();
        self.scripts.live = Some(old);
    }

    /// A command's run has ended (`ok`: finished well; `landed`: its
    /// changes were made). Starts it again when newer settings wait.
    pub(super) fn command_ended(&mut self, ok: bool, landed: bool) {
        let Some(run) = self.scripts.command_run.take() else {
            // Another run (a file, the console) leaves no live command.
            self.scripts.live = None;
            return;
        };
        if let Some(next) = run.next.clone() {
            // Stopped for newer settings: what it replaced is still taken
            // back, and the next run replaces it in turn.
            if let Some(command) = self.find_command(&run.key) {
                let replaces = if landed {
                    self.document.undo();
                    Some(self.live_after(&run, true))
                } else {
                    run.replaces
                };
                self.start_command(&command, next, replaces);
            }
            return;
        }
        if ok {
            self.scripts.live = Some(self.live_after(&run, landed));
        } else {
            let had = run.replaces.is_some();
            self.restore(run.replaces);
            if had {
                self.note = Some(crate::trf!(
                    "{} didn't finish with the new settings, so the last ones are back. The output says why.",
                    run.name
                ));
            }
        }
    }

    fn live_after(&self, run: &CommandRun, landed: bool) -> LiveCommand {
        LiveCommand {
            key: run.key.clone(),
            name: run.name.clone(),
            values: run.values.clone(),
            landed,
            changes_made: self.document.changes_made(),
            selection: self.document.selection_order().to_vec(),
            elements: self.document.element_revision(),
        }
    }

    /// The command whose settings show in Attributes, if any: its run is
    /// still the last thing done, or it's running again.
    pub(crate) fn live_command(&self) -> Option<&LiveCommand> {
        let live = self.scripts.live.as_ref()?;
        let rerunning = self
            .scripts
            .command_run
            .as_ref()
            .is_some_and(|r| r.key == live.key);
        (rerunning || live.still_last(&self.document)).then_some(live)
    }

    /// What Attributes shows for the live command.
    pub(super) fn script_view(&self) -> Option<ScriptView> {
        let live = self.live_command()?;
        let command = self.find_command(&live.key)?;
        let values = match &self.scripts.command_run {
            Some(run) if run.key == live.key => run.next.clone().unwrap_or(run.values.clone()),
            _ => live.values.clone(),
        };
        Some(ScriptView {
            name: command.name.clone(),
            settings: command.settings.clone(),
            values,
            count: command.count.clone(),
            running: self.scripts.is_running(),
        })
    }

    /// Settings changed in Attributes (or by Ctrl + wheel) for the live
    /// command: its run is taken back and it runs again with them; while
    /// it's running, it's stopped and runs again with the newest.
    pub(crate) fn change_script_settings(&mut self, values: Vec<(String, SettingValue)>) {
        let Some(live) = self.live_command().cloned() else {
            return;
        };
        self.scripts.settings.remember(&live.key, &values);
        if let Some(run) = self.scripts.command_run.as_mut() {
            if run.key == live.key {
                run.next = Some(values);
                self.scripts.stop();
            }
            return;
        }
        if self.scripts.is_running() {
            return;
        }
        let Some(command) = self.find_command(&live.key) else {
            return;
        };
        if live.landed && self.document.undo().is_none() {
            return;
        }
        self.start_command(&command, values, Some(live));
    }

    /// Whether Ctrl + wheel changes a script command's count now: its
    /// settings show in Attributes and its header names a count.
    pub(crate) fn script_count_live(&self) -> bool {
        self.live_command().is_some() && self.script_view().is_some_and(|v| v.count.is_some())
    }

    /// Ctrl + wheel for the live command's count (decision 0048):
    /// `steps` notches up or down, from 1 up. Gives back whether it took
    /// them.
    pub(crate) fn script_wheel(&mut self, steps: i32) -> bool {
        let Some(view) = self.script_view() else {
            return false;
        };
        let Some(count) = view.count.clone() else {
            return false;
        };
        let mut values = view.values.clone();
        let Some((_, SettingValue::Int(n))) = values.iter_mut().find(|(name, _)| *name == count)
        else {
            return false;
        };
        *n = (*n + i64::from(steps)).clamp(1, crate::script_commands::MAX_WHEEL_COUNT);
        let shown = *n;
        self.note = Some(crate::trf!(
            "{}: {} {} (Ctrl + wheel for more or fewer).",
            view.name,
            shown,
            script::words_of(&count).to_lowercase()
        ));
        self.change_script_settings(values);
        true
    }

    /// The keys given to script commands run them, when nothing is being
    /// typed, no window is open and no script is running.
    pub(super) fn script_command_keys(&mut self, ui: &Ui) {
        if self.window_open()
            || self.scripts.window_open()
            || self.scripts.is_running()
            || ui.ctx().memory(|m| m.focused().is_some())
        {
            return;
        }
        let keys: Vec<(CommandKey, SavedKey)> = self.scripts.settings.keys.clone();
        for (command, key) in keys {
            if self.scripts.settings.is_off(&command.file) {
                continue;
            }
            if ui.input_mut(|i| i.consume_shortcut(&key.shortcut())) {
                self.run_script_command(&command);
                return;
            }
        }
    }

    /// The Scripts menu's commands, by name, each with its key.
    pub(super) fn script_command_entries(&mut self, ui: &mut Ui) {
        let commands = self.scripts.commands();
        let running = self.scripts.is_running();
        if commands.is_empty() {
            ui.label(
                egui::RichText::new(tr("Your scripts' commands show here."))
                    .weak()
                    .small(),
            );
            return;
        }
        for command in commands {
            let mut button = egui::Button::new(&command.name);
            if let Some(key) = self.scripts.settings.key_of(&command.key) {
                button = button.shortcut_text(ui.ctx().format_shortcut(&key.shortcut()));
            }
            let help = if command.about.is_empty() {
                tr("A command from one of your scripts").to_string()
            } else {
                command.about.clone()
            };
            if ui
                .add_enabled(!running, button)
                .on_hover_text(help)
                .on_disabled_hover_text(tr("A script is running: stop it first."))
                .clicked()
            {
                self.run_script_command(&command.key);
                ui.close();
            }
        }
    }

    /// The script commands on the command palette, for it to draw.
    pub(super) fn palette_scripts(
        &self,
        ctx: &egui::Context,
    ) -> Vec<crate::palette::PaletteScript> {
        let commands = self.scripts.commands();
        let ready = !self.scripts.is_running();
        self.scripts
            .settings
            .palette
            .iter()
            .filter_map(|key| commands.iter().find(|c| c.key == *key))
            .map(|c| {
                let about = if c.about.is_empty() {
                    tr("A command from one of your scripts").to_string()
                } else {
                    c.about.clone()
                };
                let help = match self.scripts.settings.key_of(&c.key) {
                    Some(key) => format!(r"{about} ({})", ctx.format_shortcut(&key.shortcut())),
                    None => about,
                };
                crate::palette::PaletteScript {
                    name: c.name.clone(),
                    help,
                    ready,
                }
            })
            .collect()
    }

    /// The command at `index` among those the palette shows.
    pub(super) fn palette_script_key(&self, index: usize) -> Option<CommandKey> {
        let commands = self.scripts.commands();
        self.scripts
            .settings
            .palette
            .iter()
            .filter(|key| commands.iter().any(|c| c.key == **key))
            .nth(index)
            .cloned()
    }

    /// Takes in Settings changed in Attributes for the live command, and
    /// keeps the library's commands read.
    pub(super) fn poll_script_commands(&mut self) {
        let now = std::time::Instant::now();
        let names = self.scripts.names();
        let limits = self.scripts.settings.limits();
        let library = self.scripts.library.clone();
        self.scripts
            .registry
            .refresh(library.as_ref(), &names, limits, now);
        if let Some(values) = self.attributes.script_asked.take() {
            self.change_script_settings(values);
        }
        self.attributes.script = self.script_view();
    }
}

impl Workbench {
    /// What's set up for scripts, for saving between sessions.
    pub fn script_settings(&self) -> crate::ScriptSettings {
        self.scripts.settings.clone()
    }

    /// The workbench with what was set up for scripts restored.
    #[must_use]
    pub fn with_script_settings(mut self, settings: crate::ScriptSettings) -> Self {
        self.scripts.settings = settings.checked();
        self.scripts.limits = self.scripts.settings.limits();
        self
    }
}

/// Whether `kind` is shown in Attributes.
pub(crate) fn shown_kind(kind: &SettingKind) -> bool {
    !matches!(kind, SettingKind::Other(_))
}
