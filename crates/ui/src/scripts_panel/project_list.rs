//! The Scripts panel's list of the scripts kept with the project
//! (decision 0307), under your own: each can be looked at (never edited
//! here), run once the project's scripts are turned on, copied into your
//! library, exported, or taken out of the project.

use super::{Request, ScriptsPanel, Viewer};
use crate::script_sharing::Source;
use crate::text::tr;
use egui::{RichText, Ui};
use script::ScriptFile;

/// The project list's entry for looking at a script.
pub(crate) const LOOK_AT_IT: &str = crate::text!("Look at it");
/// The entry copying a project's script into your library.
pub(crate) const COPY_IN: &str = crate::text!("Copy to your library");
/// The entry taking a script out of the project.
pub(crate) const TAKE_OUT: &str = crate::text!("Take out of this project");
/// The entry turning on the project's scripts.
pub(crate) const TURN_ON: &str = crate::text!("Turn on this project's scripts");

impl ScriptsPanel {
    /// The project's script called `name`, as a file to run or show.
    pub(crate) fn project_file(&self, name: &str) -> Option<ScriptFile> {
        self.project.text_of(name).map(|text| ScriptFile {
            name: name.to_string(),
            text,
        })
    }

    /// The scripts kept with the project, under your own.
    pub(crate) fn show_project_list(&mut self, ui: &mut Ui) {
        if self.project.entries.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.label(RichText::new(tr("In this project")).strong())
            .on_hover_text(tr("Scripts kept with this project: they travel with the project file. They don't run until you turn them on."));
        let off = self.project.untrusted(&self.settings) > 0;
        if off {
            ui.label(
                RichText::new(tr("Off: right-click one to look at it or turn them on."))
                    .weak()
                    .small(),
            );
        }
        for entry in self.project.entries.clone() {
            let viewing = self
                .viewing
                .as_ref()
                .is_some_and(|v| v.file.name == entry.name);
            let trusted = self.settings.is_trusted(&entry.fingerprint);
            let shown = if trusted {
                entry.name.clone()
            } else {
                crate::trf!("{} (off)", entry.name)
            };
            let response = ui
                .add(egui::Button::selectable(viewing, shown).truncate())
                .on_hover_text(tr("Double-click to look at it; right-click for more"));
            if response.double_clicked() {
                self.look_at(&entry.name);
            }
            response.context_menu(|ui| {
                if ui.button(tr(LOOK_AT_IT)).clicked() {
                    self.look_at(&entry.name);
                    ui.close();
                }
                if trusted {
                    if ui
                        .add_enabled(!self.is_running(), egui::Button::new(tr("Run")))
                        .clicked()
                    {
                        self.run_project_script(&entry.name);
                        ui.close();
                    }
                } else if ui
                    .button(tr(TURN_ON))
                    .on_hover_text(tr("Let this project's scripts run on this computer. Look at them first: a script can change your scene, and read or write files if you allow it."))
                    .clicked()
                {
                    self.requests.push(Request::TurnOn);
                    ui.close();
                }
                ui.separator();
                if ui
                    .button(tr(COPY_IN))
                    .on_hover_text(tr("Copy it into your library, to edit it or use it in every project"))
                    .clicked()
                {
                    self.requests.push(Request::CopyIn(entry.name.clone()));
                    ui.close();
                }
                if ui.button(tr(super::list::EXPORT_ENTRY)).clicked() {
                    self.requests.push(Request::Export {
                        file: entry.name.clone(),
                        project: true,
                    });
                    ui.close();
                }
                if ui
                    .button(tr(TAKE_OUT))
                    .on_hover_text(tr("Take it out of this project (Ctrl + Z puts it back)"))
                    .clicked()
                {
                    self.requests.push(Request::TakeOut(entry.name.clone()));
                    ui.close();
                }
            });
        }
    }

    /// Shows the project's script `name` in place of the editor.
    pub(crate) fn look_at(&mut self, name: &str) {
        if let Some(file) = self.project_file(name) {
            self.viewing = Some(Viewer::new(file));
        }
    }

    /// Runs the project's script `name` (only once turned on).
    pub(crate) fn run_project_script(&mut self, name: &str) {
        let Some(file) = self.project_file(name) else {
            return;
        };
        let trusted = self
            .project
            .entry(name)
            .is_some_and(|e| self.settings.is_trusted(&e.fingerprint));
        if !trusted {
            self.say_trouble(
                tr("This project's scripts are off: turn them on first (right-click one, or the bar over the view).")
                    .to_string(),
            );
            return;
        }
        let name = script::read_header(&file.text).shown_name(&file.name);
        let source = Source::Project(file.name.clone());
        self.requests.push(Request::Run {
            file,
            name,
            console: None,
            source,
        });
    }

    /// The script kept with the project, shown in place of the editor.
    pub(crate) fn show_viewing(&mut self, ui: &mut Ui) {
        let Some(viewer) = self.viewing.as_mut() else {
            return;
        };
        let name = viewer.file.name.clone();
        let (mut close, mut copy) = (false, false);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&name).strong());
            ui.label(
                RichText::new(tr("kept with this project: look, don't touch"))
                    .weak()
                    .small(),
            )
            .on_hover_text(tr("A project's scripts can't be edited here. Copy one to your library to change it, then keep it with the project again."));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                close = ui
                    .small_button("×")
                    .on_hover_text(tr("Stop looking at it"))
                    .clicked();
                copy = ui.small_button(tr(COPY_IN)).clicked();
            });
        });
        viewer.show(ui);
        if copy {
            self.requests.push(Request::CopyIn(name));
        }
        if close {
            self.viewing = None;
        }
    }
}
