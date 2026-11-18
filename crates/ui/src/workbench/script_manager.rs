//! The script manager (Scripts → Script manager…, decision 0305): your
//! library's scripts, what each says it does and asks for, its commands
//! with their keys and places on the command palette; turn a script off
//! or on, rename, remove or reload it, or show it in its folder. Also the
//! small window that gives a command a key: a key already taken is
//! refused, naming what uses it.

use super::Workbench;
use crate::dialog::{self, Dialog};
use crate::keyboard::KeyId;
use crate::script_commands::{CommandKey, KeyAsk, LibraryScript, SavedKey, commands_of};
use crate::scripts_panel::Request;
use crate::text::tr;
use egui::{Key, Modifiers, RichText, Ui};
use std::path::Path;

/// The manager's title.
pub(crate) const MANAGER_TITLE: &str = crate::text!("Script manager");

/// What a key would clash with, if it's taken: the job of the app's key,
/// or the script command given it. `None` when it's free.
pub(crate) fn taken_by(
    key: SavedKey,
    settings: &crate::script_commands::ScriptSettings,
    names: &dyn Fn(&CommandKey) -> String,
    asking: &CommandKey,
) -> Option<String> {
    let pressed = key.shortcut();
    let plain = !key.ctrl && !key.alt && !key.shift;
    if plain
        && matches!(
            pressed.logical_key,
            Key::Escape | Key::Enter | Key::Tab | Key::Backspace
        )
    {
        return Some(tr("typing and closing windows").to_string());
    }
    for binding in super::shortcuts::bindings() {
        let KeyId::Key(bound) = binding.key else {
            continue;
        };
        let m = binding.modifiers;
        let ctrl = m.command || m.ctrl || m.mac_cmd;
        if bound == pressed.logical_key
            && ctrl == key.ctrl
            && m.shift == key.shift
            && m.alt == key.alt
        {
            return Some(binding.action);
        }
    }
    settings
        .command_with(key)
        .filter(|other| *other != asking)
        .map(names)
}

impl Workbench {
    /// Draws the script manager and the key window, when open.
    pub(super) fn script_windows(&mut self, ctx: &egui::Context) {
        if self.scripts.manager.open {
            self.manager_window(ctx);
        }
        if self.scripts.asking_key.is_some() {
            self.key_window(ctx);
        }
    }

    /// The name a command shows, for messages.
    fn command_name(&self, key: &CommandKey) -> String {
        self.scripts
            .registry
            .scripts
            .iter()
            .flat_map(commands_of)
            .find(|c| c.key == *key)
            .map_or_else(|| key.file.clone(), |c| c.name)
    }

    fn key_window(&mut self, ctx: &egui::Context) {
        let Some(ask) = self.scripts.asking_key.clone() else {
            return;
        };
        let pressed = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        let mut close = false;
        let mut clear = false;
        let current = self.scripts.settings.key_of(&ask.key);
        let outcome = Dialog::new(crate::trf!("Give {} a key", ask.name), 420.0)
            .id("script key")
            .modal()
            .show(ctx, |d| {
                d.header(true, |_| {});
                d.body(|ui| {
                    dialog::heading(ui, tr("Press the keys you want for it"));
                    dialog::help(
                        ui,
                        tr("Hold Ctrl, Shift or Alt with a key if you like. A key the app already uses is refused. Esc cancels."),
                    );
                    if let Some(key) = current {
                        ui.label(crate::trf!(
                            "Now: {}",
                            ctx.format_shortcut(&key.shortcut())
                        ));
                    }
                    if let Some(why) = &ask.refused {
                        ui.label(RichText::new(why).color(ui.visuals().error_fg_color));
                    }
                });
                d.footer(None, |ui| {
                    close = dialog::plain_button(ui, tr("Cancel")).clicked();
                    if current.is_some() {
                        clear = dialog::plain_button(ui, tr("Take its key away")).clicked();
                    }
                });
            });
        if outcome.closed || close {
            self.scripts.asking_key = None;
            return;
        }
        if clear {
            self.scripts.settings.keys.retain(|(k, _)| *k != ask.key);
            self.note = Some(crate::trf!("{} has no key now.", ask.name));
            self.scripts.asking_key = None;
            return;
        }
        let Some((key, modifiers)) = pressed else {
            return;
        };
        ctx.input_mut(|i| i.consume_key(modifiers, key));
        if key == Key::Escape && modifiers == Modifiers::NONE {
            self.scripts.asking_key = None;
            return;
        }
        let saved = SavedKey::from_press(key, modifiers);
        self.give_key(&ask, saved, ctx);
    }

    /// Gives `ask`'s command `saved`, or says why not.
    pub(crate) fn give_key(&mut self, ask: &KeyAsk, saved: SavedKey, ctx: &egui::Context) {
        let names = |k: &CommandKey| self.command_name(k);
        let shown = ctx.format_shortcut(&saved.shortcut());
        if let Some(user) = taken_by(saved, &self.scripts.settings, &names, &ask.key) {
            let why = crate::trf!("{} is taken: it's {}. Press another.", shown, user);
            if let Some(asking) = self.scripts.asking_key.as_mut() {
                asking.refused = Some(why);
            }
            return;
        }
        let keys = &mut self.scripts.settings.keys;
        keys.retain(|(k, _)| *k != ask.key);
        if keys.len() < crate::script_commands::MAX_KEPT {
            keys.push((ask.key.clone(), saved));
        }
        self.note = Some(crate::trf!("{} now runs {}.", shown, ask.name));
        self.scripts.asking_key = None;
    }

    fn manager_window(&mut self, ctx: &egui::Context) {
        let scripts = self.scripts.registry.scripts.clone();
        let mut done = false;
        let mut folder = false;
        let note = self.scripts.manager.note.clone();
        let outcome = Dialog::new(tr(MANAGER_TITLE), 640.0)
            .body_height(460.0)
            .show(ctx, |d| {
                d.header(true, |_| {});
                // The body keeps its height and scrolls when taller.
                d.body(|ui| {
                    if scripts.is_empty() {
                        dialog::help(
                            ui,
                            tr("Your library has no scripts yet. Make one with + in the Scripts panel (F11)."),
                        );
                    }
                    for script in &scripts {
                        ui.push_id(&script.file, |ui| self.manager_entry(ui, script));
                        dialog::divider(ui);
                    }
                    self.manager_project_entries(ui);
                });
                d.footer(note.as_deref(), |ui| {
                    done = dialog::main_button(ui, tr("Close")).clicked();
                    folder = dialog::plain_button(ui, tr("Show the folder")).clicked();
                });
            });
        let (_, escape) = dialog::enter_or_escape(ctx);
        let busy =
            self.scripts.manager.renaming.is_some() || self.scripts.manager.removing.is_some();
        if outcome.closed || done || (escape && !busy) {
            self.scripts.manager = Default::default();
        }
        if folder && let Some(library) = &self.scripts.library {
            let path = library.folder().to_path_buf();
            self.scripts.requests.push(Request::Outside(path));
        }
    }

    /// One script in the manager.
    fn manager_entry(&mut self, ui: &mut Ui, script: &LibraryScript) {
        let off = self.scripts.settings.is_off(&script.file);
        ui.horizontal(|ui| {
            let mut on = !off;
            if ui
                .checkbox(&mut on, "")
                .on_hover_text(tr("On: its commands are in the Scripts menu, command search, the command palette and their keys. Off: they're not, though it still runs from the Scripts panel."))
                .changed()
            {
                let settings = &mut self.scripts.settings;
                settings.off.retain(|f| *f != script.file);
                if !on {
                    settings.off.push(script.file.clone());
                }
            }
            ui.label(RichText::new(script.shown_name()).strong());
            ui.label(RichText::new(&script.file).weak().small());
            if off {
                ui.label(RichText::new(tr("(off)")).weak());
            }
        });
        match &script.described {
            None => {
                ui.label(RichText::new(tr("Reading it…")).weak());
            }
            Some(described) => {
                if let Some(about) = &described.header.about {
                    ui.label(about);
                }
                let asks: Vec<&str> = described
                    .header
                    .asks
                    .iter()
                    .map(|a| tr(a.words()))
                    .collect();
                ui.label(
                    RichText::new(if asks.is_empty() {
                        tr("Asks for nothing outside the project.").to_string()
                    } else {
                        crate::trf!("Asks to: {}.", asks.join(", "))
                    })
                    .small()
                    .weak(),
                );
                let id = crate::script_sharing::ScriptId::Library(script.file.clone());
                let wanted = described.header.asks.clone();
                self.approval_line(ui, id, &script.shown_name(), &wanted);
                if let Some(problem) = described.problems.first() {
                    ui.label(
                        RichText::new(crate::trf!(
                            "It has a mistake, so it has no commands until it's mended: {}",
                            problem
                        ))
                        .color(ui.visuals().error_fg_color),
                    );
                }
                for note in &described.notes {
                    ui.label(
                        RichText::new(note)
                            .small()
                            .color(ui.visuals().warn_fg_color),
                    );
                }
            }
        }
        for command in commands_of(script) {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(&command.name);
                self.command_buttons(ui, &command.key, &command.name);
            });
        }
        self.manager_actions(ui, script);
    }

    /// The scripts kept with the project: whether they're on, what each
    /// asks for and whether that's allowed.
    fn manager_project_entries(&mut self, ui: &mut Ui) {
        let entries = self.scripts.project.entries.clone();
        if entries.is_empty() {
            return;
        }
        dialog::heading(ui, tr("In this project"));
        let off = self.scripts.project.untrusted(&self.scripts.settings) > 0;
        if off {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("They're off on this computer until you turn them on."));
                if ui
                    .small_button(tr(super::script_sharing::TURN_THEM_ON))
                    .clicked()
                {
                    self.turn_on_project_scripts();
                }
            });
        }
        for entry in entries {
            ui.push_id(("project", &entry.name), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(entry.header.shown_name(&entry.name)).strong());
                    ui.label(RichText::new(&entry.name).weak().small());
                    if !self.scripts.settings.is_trusted(&entry.fingerprint) {
                        ui.label(RichText::new(tr("(off)")).weak());
                    }
                });
                if let Some(about) = &entry.header.about {
                    ui.label(about);
                }
                let id = crate::script_sharing::ScriptId::Project(entry.fingerprint.clone());
                let name = entry.header.shown_name(&entry.name);
                self.approval_line(ui, id, &name, &entry.header.asks);
            });
            dialog::divider(ui);
        }
    }

    /// A command's palette switch and key button.
    fn command_buttons(&mut self, ui: &mut Ui, key: &CommandKey, name: &str) {
        let shown_key = self.scripts.settings.key_of(key);
        let label = match shown_key {
            Some(k) => crate::trf!("Key: {}", ui.ctx().format_shortcut(&k.shortcut())),
            None => tr(crate::scripts_panel::GIVE_KEY).to_string(),
        };
        if ui.small_button(label).clicked() {
            self.scripts.asking_key = Some(KeyAsk {
                key: key.clone(),
                name: name.to_string(),
                refused: None,
            });
        }
        let mut on = self.scripts.settings.on_palette(key);
        if ui.checkbox(&mut on, tr("On the command palette")).changed() {
            self.scripts.settings.set_on_palette(key, on);
        }
    }

    /// A script's buttons, and renaming or removing it.
    fn manager_actions(&mut self, ui: &mut Ui, script: &LibraryScript) {
        let path = script.path.clone();
        let renaming = self
            .scripts
            .manager
            .renaming
            .as_ref()
            .is_some_and(|(p, _, _)| *p == path);
        let removing = self.scripts.manager.removing.as_ref() == Some(&path);
        if renaming {
            return self.rename_form(ui, &path);
        }
        if removing {
            ui.horizontal_wrapped(|ui| {
                ui.label(crate::trf!(
                    "Remove {}? Its file is deleted from your library, and can't be brought back.",
                    script.file
                ));
                if ui.button(tr("Remove it")).clicked() {
                    self.remove_script(&path, &script.file);
                }
                if ui.button(tr("Keep it")).clicked() {
                    self.scripts.manager.removing = None;
                }
            });
            return;
        }
        ui.horizontal(|ui| {
            if ui.small_button(tr("Edit")).clicked() {
                crate::layout::show_beside_coordinates(
                    &mut self.dock,
                    crate::panel::Panel::Scripts,
                );
                self.scripts.edit(&path, std::time::Instant::now());
            }
            if ui
                .small_button(tr("Reload"))
                .on_hover_text(tr("Read its commands again now"))
                .clicked()
            {
                let names = self.scripts.names();
                let limits = self.scripts.settings.limits();
                self.scripts.registry.read_again(&path, &names, limits);
            }
            if ui.small_button(tr("Rename…")).clicked() {
                let stem = script
                    .file
                    .rsplit_once('.')
                    .map_or(script.file.as_str(), |s| s.0);
                self.scripts.manager.renaming = Some((path.clone(), stem.to_string(), None));
            }
            if ui.small_button(tr("Remove…")).clicked() {
                self.scripts.manager.removing = Some(path.clone());
            }
            if ui.small_button(tr("Show in its folder")).clicked()
                && let Some(parent) = path.parent()
            {
                self.scripts
                    .requests
                    .push(Request::Outside(parent.to_path_buf()));
            }
        });
    }

    fn rename_form(&mut self, ui: &mut Ui, path: &Path) {
        let Some((_, name, why)) = self.scripts.manager.renaming.as_mut() else {
            return;
        };
        let mut go = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            let field = ui.add(egui::TextEdit::singleline(name).desired_width(240.0));
            go = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            go |= ui.button(tr("Rename")).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
        });
        if let Some(why) = why {
            ui.label(RichText::new(why.as_str()).color(ui.visuals().error_fg_color));
        }
        if cancel {
            self.scripts.manager.renaming = None;
        } else if go {
            let name = name.clone();
            self.rename_script(path, &name);
        }
    }

    /// Renames the script at `path` to `name`: its keys, palette places
    /// and remembered settings follow it.
    pub(crate) fn rename_script(&mut self, path: &Path, name: &str) {
        let Some(library) = self.scripts.library.clone() else {
            return;
        };
        let editing = self.scripts.editor.as_ref().is_some_and(|e| e.path == path);
        if editing {
            // Its typing is saved before the file moves.
            self.scripts.editor = None;
        }
        let file = |p: &Path| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };
        match library.rename(path, name) {
            Ok(to) => {
                self.scripts.settings.renamed(&file(path), &file(&to));
                self.scripts.manager.renaming = None;
                self.scripts.manager.note =
                    Some(crate::trf!("{} is now {}.", file(path), file(&to)));
                self.scripts.list_again(std::time::Instant::now());
                if editing {
                    self.scripts.edit(&to, std::time::Instant::now());
                }
                self.scripts.registry.forget(path);
            }
            Err(why) => {
                if editing {
                    self.scripts.edit(path, std::time::Instant::now());
                }
                if let Some((_, _, refused)) = self.scripts.manager.renaming.as_mut() {
                    *refused = Some(why);
                }
            }
        }
    }

    /// Removes the script at `path` from the library.
    pub(crate) fn remove_script(&mut self, path: &Path, file: &str) {
        let Some(library) = self.scripts.library.clone() else {
            return;
        };
        if let Some(mut editor) = self.scripts.editor.take_if(|e| e.path == path) {
            // Nothing left to save when it's dropped.
            let _ = editor.save_now();
        }
        self.scripts.manager.removing = None;
        match library.remove(path) {
            Ok(()) => {
                self.scripts.settings.removed(file);
                self.scripts.registry.forget(path);
                self.scripts.list_again(std::time::Instant::now());
                self.scripts.manager.note = Some(crate::trf!("{} was removed.", file));
            }
            Err(why) => self.scripts.manager.note = Some(why),
        }
    }
}
