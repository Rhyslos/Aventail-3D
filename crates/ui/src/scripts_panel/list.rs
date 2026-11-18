//! The Scripts panel's left side (plan, milestone 3): the scripts in your
//! library, and making a new one from a short starter.

use super::console::language_name;
use super::{Request, ScriptsPanel};
use crate::text::tr;
use egui::{RichText, Ui};
use script::Language;
use script::library::Library;
use std::path::Path;
use std::time::{Duration, Instant};

/// How often the library's folder is read again for scripts added or
/// removed outside the app, while the panel shows.
const LIST_EVERY: Duration = Duration::from_secs(2);

/// The list's entry putting a script's command on the command palette.
pub(crate) const ADD_TO_PALETTE: &str = crate::text!("Add to the command palette");
/// The list's entry taking it off.
pub(crate) const OFF_PALETTE: &str = crate::text!("Take it off the command palette");
/// The list's entry giving it a key.
pub(crate) const GIVE_KEY: &str = crate::text!("Give it a key…");

/// A new script being named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewScript {
    pub(crate) name: String,
    pub(crate) language: Language,
    /// Why the name last tried was refused.
    pub(crate) why: Option<String>,
}

impl ScriptsPanel {
    /// Reads the library's folder again when due, writing the
    /// description files into it until that has worked once.
    pub(crate) fn refresh_list(&mut self, now: Instant) {
        let Some(library) = &self.library else {
            return;
        };
        if self
            .listed_at
            .is_some_and(|at| now.duration_since(at) < LIST_EVERY)
        {
            return;
        }
        self.listed_at = Some(now);
        let mut trouble = None;
        if !self.prepared {
            match library.prepare(&self.names()) {
                Ok(()) => self.prepared = true,
                Err(why) => trouble = Some(why),
            }
        }
        match library.scripts() {
            Ok(found) => self.listed = found,
            Err(why) => trouble = trouble.or(Some(why)),
        }
        self.library_trouble = trouble;
    }

    /// Reads the library's folder again now.
    pub(crate) fn list_again(&mut self, now: Instant) {
        self.listed_at = None;
        self.refresh_list(now);
    }

    /// Draws the list.
    pub(crate) fn show_list(&mut self, ui: &mut Ui, now: Instant) {
        let Some(library) = self.library.clone() else {
            ui.label(
                RichText::new(tr("Your script library isn't available on this computer.")).weak(),
            );
            return;
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Your scripts")).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("+")
                    .on_hover_text(tr("New script: a LustScript or Lua script in your library, from a short starter"))
                    .clicked()
                {
                    self.new_script = Some(NewScript {
                        name: String::new(),
                        language: self.console.language,
                        why: None,
                    });
                }
            });
        });
        if self.new_script.is_some() {
            self.show_new_script(ui, &library, now);
        }
        if let Some(why) = &self.library_trouble {
            ui.label(RichText::new(why).color(ui.visuals().error_fg_color));
        }
        egui::ScrollArea::vertical()
            .id_salt(ui.id().with(r"script list"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.listed.is_empty() {
                    ui.label(RichText::new(tr("None yet: + makes one.")).weak());
                }
                for path in self.listed.clone() {
                    self.list_entry(ui, &path, &library, now);
                }
            });
    }

    /// One script in the list: double-click edits it, right-click shows
    /// what else can be done.
    fn list_entry(&mut self, ui: &mut Ui, path: &Path, library: &Library, now: Instant) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let editing = self.editor.as_ref().is_some_and(|e| e.path == path);
        let response = ui
            .add(egui::Button::selectable(editing, &name).truncate())
            .on_hover_text(tr("Double-click to edit it; right-click for more"));
        if response.double_clicked() {
            self.edit(path, now);
        }
        response.context_menu(|ui| {
            if ui.button(tr("Edit")).clicked() {
                self.edit(path, now);
                ui.close();
            }
            if ui
                .add_enabled(!self.is_running(), egui::Button::new(tr("Run")))
                .clicked()
            {
                self.run_path(path);
                ui.close();
            }
            if ui
                .button(tr("Open in your own editor"))
                .on_hover_text(tr("Open it in the program your computer uses for this kind of file. Changes saved there are loaded here."))
                .clicked()
            {
                self.requests.push(Request::Outside(path.to_path_buf()));
                ui.close();
            }
            if ui
                .button(tr("Show the folder"))
                .on_hover_text(tr("Open your script library's folder"))
                .clicked()
            {
                self.requests
                    .push(Request::Outside(library.folder().to_path_buf()));
                ui.close();
            }
            self.command_entries(ui, path);
        });
    }

    /// The script's commands' entries (decision 0305): each can go on the
    /// command palette or be given a key.
    fn command_entries(&mut self, ui: &mut Ui, path: &Path) {
        let Some(script) = self.registry.scripts.iter().find(|s| s.path == path) else {
            return;
        };
        let commands = crate::script_commands::commands_of(script);
        if commands.is_empty() {
            return;
        }
        let several = commands.len() > 1;
        ui.separator();
        for command in commands {
            let on = self.settings.on_palette(&command.key);
            let (palette, key) = match (several, on) {
                (false, false) => (tr(ADD_TO_PALETTE).to_string(), tr(GIVE_KEY).to_string()),
                (false, true) => (tr(OFF_PALETTE).to_string(), tr(GIVE_KEY).to_string()),
                (true, false) => (
                    crate::trf!("Add {} to the command palette", command.name),
                    crate::trf!("Give {} a key…", command.name),
                ),
                (true, true) => (
                    crate::trf!("Take {} off the command palette", command.name),
                    crate::trf!("Give {} a key…", command.name),
                ),
            };
            if ui.button(palette).clicked() {
                self.settings.set_on_palette(&command.key, !on);
                ui.close();
            }
            if ui.button(key).clicked() {
                self.asking_key = Some(crate::script_commands::KeyAsk {
                    key: command.key.clone(),
                    name: command.name.clone(),
                    refused: None,
                });
                ui.close();
            }
        }
    }

    /// Opens the script at `path` in the editor (saving the one open).
    pub(crate) fn edit(&mut self, path: &Path, now: Instant) {
        if self.editor.as_ref().is_some_and(|e| e.path == path) {
            return;
        }
        // Dropping the open one saves its typing.
        self.editor = None;
        match super::editor::Editor::open(path, now) {
            Ok(editor) => {
                self.console.language = editor.language;
                self.editor = Some(editor);
            }
            Err(why) => self.say_trouble(why),
        }
    }

    /// Runs the script at `path` from its file.
    fn run_path(&mut self, path: &Path) {
        if let Some(editor) = self.editor.as_mut().filter(|e| e.path == path) {
            let notice = editor.save_now();
            self.say(notice);
        }
        match script::ScriptFile::read(path) {
            Ok(file) => {
                let name = script::read_header(&file.text).shown_name(&file.name);
                self.requests.push(Request::Run {
                    file,
                    name,
                    console: None,
                });
            }
            Err(why) => self.say_trouble(why),
        }
    }

    /// The form naming a new script.
    fn show_new_script(&mut self, ui: &mut Ui, library: &Library, now: Instant) {
        let Some(new) = self.new_script.as_mut() else {
            return;
        };
        let mut create = false;
        let mut cancel = false;
        ui.group(|ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut new.name)
                    .hint_text(tr("Its name"))
                    .desired_width(f32::INFINITY),
            );
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                create = true;
            }
            if new.name.is_empty() && new.why.is_none() && !field.has_focus() {
                field.request_focus();
            }
            ui.horizontal(|ui| {
                for language in [Language::LustScript, Language::Lua] {
                    ui.radio_value(&mut new.language, language, language_name(language));
                }
            });
            if let Some(why) = &new.why {
                ui.label(RichText::new(why).color(ui.visuals().error_fg_color));
            }
            ui.horizontal(|ui| {
                create |= ui.button(tr("Make it")).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
        if cancel {
            self.new_script = None;
        } else if create {
            let (name, language) = (new.name.clone(), new.language);
            match library.create(&name, language, &self.names()) {
                Ok(path) => {
                    self.new_script = None;
                    self.list_again(now);
                    self.edit(&path, now);
                }
                Err(why) => {
                    if let Some(new) = self.new_script.as_mut() {
                        new.why = Some(why);
                    }
                }
            }
        }
    }
}
