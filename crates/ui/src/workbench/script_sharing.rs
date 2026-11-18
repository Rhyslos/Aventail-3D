//! Keeping, sharing and permissions (decision 0307): the bar that says a
//! project has scripts and turns them on, keeping a library script with
//! the project (a step to undo), taking one out or copying it into your
//! library, Install a script… and Export…, and the window in which a
//! script asks to reach files before it runs.

use super::{Workbench, WorkbenchAction};
use crate::dialog::{self, Dialog};
use crate::script_sharing::{AccessAsk, Installing, ScriptId, Source, Then};
use crate::scripts_panel::{Row, Viewer};
use crate::text::tr;
use egui::{RichText, Ui};
use scene::{Command, ProjectScript};
use script::{Access, Ask, Line, ScriptFile};
use std::path::{Path, PathBuf};

/// The trust bar's button turning the project's scripts on.
pub(crate) const TURN_THEM_ON: &str = crate::text!("Turn them on");
/// The trust bar's button showing them.
pub(crate) const LOOK_AT_THEM: &str = crate::text!("Look at them");
/// The access window's button allowing and running.
pub(crate) const ALLOW_AND_RUN: &str = crate::text!("Allow and run");
/// The access window's button allowing (from the script manager).
pub(crate) const ALLOW: &str = crate::text!("Allow");
/// The install window's button.
pub(crate) const INSTALL: &str = crate::text!("Install it");
/// The Scripts menu's entry installing a script.
pub(crate) const INSTALL_ENTRY: &str = crate::text!("Install a script…");

impl Workbench {
    /// The folder the project is saved in, if it's saved.
    pub(super) fn project_folder(&self) -> Option<PathBuf> {
        self.project_path()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    }

    /// Takes in the project's scripts when they change (opening a
    /// project, keeping or taking out a script, undo), reading the
    /// commands of those turned on.
    pub(super) fn poll_project_scripts(&mut self) {
        let names = self.scripts.names();
        let limits = self.scripts.settings.limits();
        let kept = self.document.scene().scripts_shared();
        let panel = &mut self.scripts;
        panel.project.refresh(kept, &panel.settings, &names, limits);
        let gone = panel
            .viewing
            .as_ref()
            .is_some_and(|v| panel.project.text_of(&v.file.name).as_deref() != Some(&v.file.text));
        if gone {
            panel.viewing = None;
        }
    }

    /// What a run from `source` of `file` is called by approvals; `None`
    /// for the console line, which reaches nothing.
    fn script_id(&self, source: &Source, file: &ScriptFile) -> Option<ScriptId> {
        match source {
            Source::Library(name) => Some(ScriptId::Library(name.clone())),
            Source::Project(name) => Some(ScriptId::Project(
                self.scripts.project.entry(name).map_or_else(
                    || script::fingerprint(&file.text),
                    |e| e.fingerprint.clone(),
                ),
            )),
            Source::File(path) => Some(ScriptId::File(path.display().to_string())),
            Source::Console => None,
        }
    }

    /// What a run of `file` from `source` may reach, or `None` when it
    /// may not run yet: a project's script not turned on (said in the
    /// status bar), or asks not yet allowed (the access window opens,
    /// running `then` once allowed).
    pub(super) fn access_to_run(
        &mut self,
        file: &ScriptFile,
        name: &str,
        source: &Source,
        then: Then,
    ) -> Option<Access> {
        if let Source::Project(kept) = source {
            let on = self
                .scripts
                .project
                .entry(kept)
                .is_some_and(|e| self.scripts.settings.is_trusted(&e.fingerprint));
            if !on {
                self.note = Some(
                    tr("This project's scripts are off: turn them on first (the bar over the view).")
                        .to_string(),
                );
                return None;
            }
        }
        let id = self.script_id(source, file);
        let asks = script::read_header(&file.text).asks;
        if let Some(id) = &id
            && self.scripts.settings.needs_approval(id, &asks)
        {
            let folder = self
                .scripts
                .settings
                .approval_of(id)
                .and_then(|a| a.folder.clone());
            self.scripts.asking_access = Some(AccessAsk {
                script: id.clone(),
                name: name.to_string(),
                asks,
                folder,
                then,
            });
            return None;
        }
        Some(
            self.scripts
                .settings
                .access_for(id.as_ref(), &asks, self.project_folder().as_deref()),
        )
    }

    /// The bar over the view saying the project has scripts that are
    /// off, with Look at them and Turn them on (choice 6).
    pub(super) fn trust_bar(&mut self, ui: &mut Ui) {
        let off = self.scripts.project.untrusted(&self.scripts.settings);
        if off == 0 || self.scripts.project.bar_hidden {
            return;
        }
        let all = self.scripts.project.entries.len();
        egui::Panel::top(r"script trust bar")
            .frame(crate::theme::palette_frame())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if all == 1 {
                            tr("This project has a script. It won't run until you turn it on.")
                                .to_string()
                        } else if off == all {
                            crate::trf!(
                                "This project has {} scripts. They won't run until you turn them on.",
                                all
                            )
                        } else {
                            crate::trf!(
                                "This project has {} scripts you haven't turned on. They won't run until you do.",
                                off
                            )
                        })
                        .color(ui.visuals().warn_fg_color),
                    );
                    if ui
                        .button(tr(LOOK_AT_THEM))
                        .on_hover_text(tr("Show them in the Scripts panel, under In this project: what they say and do, without running them"))
                        .clicked()
                    {
                        self.look_at_project_scripts();
                    }
                    if ui
                        .button(tr(TURN_THEM_ON))
                        .on_hover_text(tr("Let this project's scripts run on this computer, and their commands show in the Scripts menu. A script still asks before it reaches any files."))
                        .clicked()
                    {
                        self.turn_on_project_scripts();
                    }
                    if ui
                        .button(tr("Not now"))
                        .on_hover_text(tr("Put this bar away; the scripts stay off"))
                        .clicked()
                    {
                        self.scripts.project.bar_hidden = true;
                    }
                });
            });
    }

    /// Shows the Scripts panel with the project's first script in view.
    pub(crate) fn look_at_project_scripts(&mut self) {
        crate::layout::show_beside_coordinates(&mut self.dock, crate::panel::Panel::Scripts);
        if let Some(first) = self.scripts.project.entries.first().map(|e| e.name.clone()) {
            self.scripts.look_at(&first);
        }
    }

    /// Trusts every script kept with the project, on this computer.
    pub(crate) fn turn_on_project_scripts(&mut self) {
        let fingerprints: Vec<String> = self
            .scripts
            .project
            .entries
            .iter()
            .map(|e| e.fingerprint.clone())
            .collect();
        for fingerprint in &fingerprints {
            self.scripts.settings.trust(fingerprint);
        }
        self.poll_project_scripts();
        self.note = Some(
            tr("This project's scripts are on: their commands are in the Scripts menu.")
                .to_string(),
        );
    }

    /// Stops trusting every script kept with the project (decision 0309):
    /// they're off on this computer again until turned on.
    pub(crate) fn turn_off_project_scripts(&mut self) {
        let fingerprints: Vec<String> = self
            .scripts
            .project
            .entries
            .iter()
            .map(|e| e.fingerprint.clone())
            .collect();
        for fingerprint in &fingerprints {
            self.scripts.settings.distrust(fingerprint);
        }
        // The bar asks again; Not now lasts until the scripts change.
        self.scripts.project.bar_hidden = true;
        self.poll_project_scripts();
        self.note = Some(
            tr("This project's scripts are off on this computer: they won't run until you turn them on again.")
                .to_string(),
        );
    }

    /// Keeps the library script at `path` with the project (one step to
    /// undo), trusting it: it's yours.
    pub(crate) fn keep_with_project(&mut self, path: &Path) {
        if let Some(editor) = self.scripts.editor.as_mut().filter(|e| e.path == path) {
            let _ = editor.save_now();
        }
        let file = match ScriptFile::read(path) {
            Ok(file) => file,
            Err(why) => {
                self.note = Some(why);
                return;
            }
        };
        let mut list = self.document.scene().scripts().to_vec();
        let had = list
            .iter()
            .position(|s| s.name.eq_ignore_ascii_case(&file.name));
        let kept = ProjectScript {
            name: file.name.clone(),
            text: file.text.clone(),
        };
        match had {
            Some(at) => list[at] = kept,
            None => list.push(kept),
        }
        match self.document.execute(Command::SetScripts(list)) {
            Ok(_) => {
                self.scripts
                    .settings
                    .trust(&script::fingerprint(&file.text));
                self.note = Some(if had.is_some() {
                    crate::trf!(
                        "{} is updated in this project. Save the project to keep it.",
                        file.name
                    )
                } else {
                    crate::trf!(
                        "{} is kept with this project now. Save the project to keep it.",
                        file.name
                    )
                });
            }
            Err(why) => {
                self.note = Some(crate::trf!(
                    "{} couldn't be kept with the project: {}.",
                    file.name,
                    why
                ));
            }
        }
    }

    /// Takes the script `name` out of the project (one step to undo).
    pub(crate) fn take_out_of_project(&mut self, name: &str) {
        let mut list = self.document.scene().scripts().to_vec();
        list.retain(|s| s.name != name);
        match self.document.execute(Command::SetScripts(list)) {
            Ok(_) => {
                self.note = Some(crate::trf!(
                    "{} is out of this project (Ctrl + Z puts it back).",
                    name
                ));
            }
            Err(why) => self.note = Some(why.to_string()),
        }
    }

    /// Copies the project's script `name` into your library (never over
    /// one of yours).
    pub(crate) fn copy_into_library(&mut self, name: &str) {
        let (Some(file), Some(library)) = (
            self.scripts.project_file(name),
            self.scripts.library.clone(),
        ) else {
            return;
        };
        match library.add(&file, &self.scripts.names()) {
            Ok(path) => {
                let now = std::time::Instant::now();
                self.scripts.list_again(now);
                self.scripts.edit(&path, now);
                self.note = Some(crate::trf!(
                    "{} is in your library now, as {}.",
                    name,
                    super::files::file_name(&path)
                ));
            }
            Err(why) => self.note = Some(why),
        }
    }

    /// Reads the script file at `path` and shows it before installing
    /// (the app hands it over after Scripts → Install a script…).
    pub fn install_script_file(&mut self, path: &Path) {
        match ScriptFile::read(path) {
            Ok(file) => {
                let header = script::read_header(&file.text);
                let viewer = Viewer::new(file.clone());
                self.scripts.installing = Some((
                    Installing {
                        file,
                        header,
                        why: None,
                    },
                    viewer,
                ));
            }
            Err(why) => self.note = Some(why),
        }
    }

    /// The script to export: the project's or your library's `file`.
    pub fn script_to_export(&self, file: &str, project: bool) -> Option<ScriptFile> {
        if project {
            return self.scripts.project_file(file);
        }
        let path = self.scripts.library.as_ref()?.folder().join(file);
        ScriptFile::read(&path).ok()
    }

    /// A script was exported to `path`, or why not.
    pub fn script_exported(&mut self, path: &Path, result: Result<(), String>) {
        self.note = Some(match result {
            Ok(()) => crate::trf!(
                "Exported the script to {}. Whoever you give it to installs it with Scripts → Install a script….",
                super::files::file_name(path)
            ),
            Err(why) => crate::trf!(
                "Couldn't export the script to {}: {}.",
                super::files::file_name(path),
                why
            ),
        });
    }

    /// The folder chosen for a script asking for one (after Choose…).
    pub fn script_folder_chosen(&mut self, folder: PathBuf) {
        if let Some(ask) = self.scripts.asking_access.as_mut() {
            ask.folder = Some(folder);
        }
    }

    /// Draws the access and install windows, when open.
    pub(super) fn sharing_windows(&mut self, ctx: &egui::Context) {
        if self.scripts.asking_access.is_some() {
            self.access_window(ctx);
        }
        if self.scripts.installing.is_some() {
            self.install_window(ctx);
        }
    }

    fn access_window(&mut self, ctx: &egui::Context) {
        let Some(ask) = self.scripts.asking_access.clone() else {
            return;
        };
        let unsaved = self.project_path().is_none();
        let wants_folder = ask.asks.contains(&Ask::ChosenFolder);
        let ready = !wants_folder || ask.folder.is_some();
        let running = !matches!(ask.then, Then::Nothing);
        let (mut allow, mut cancel, mut choose) = (false, false, false);
        let outcome = Dialog::new(crate::trf!("{} asks to reach files", ask.name), 480.0)
            .id(r"script access")
            .modal()
            .show(ctx, |d| {
                d.header(true, |_| {});
                d.body(|ui| {
                    dialog::heading(ui, tr("It asks to:"));
                    for asked in &ask.asks {
                        ui.label(crate::trf!("• {}", tr(asked.words())));
                    }
                    dialog::help(
                        ui,
                        tr("Allow it only if you trust where the script came from: it could change or delete files there. It can't reach anything else, and you can take this back in the script manager. If a later version asks for more, you're asked again."),
                    );
                    let project_asks = ask
                        .asks
                        .iter()
                        .any(|a| matches!(a, Ask::ReadProjectFolder | Ask::WriteProjectFolder));
                    if project_asks && unsaved {
                        ui.label(
                            RichText::new(tr("This project isn't saved yet, so it has no folder: the script can't reach files until it is."))
                                .color(ui.visuals().warn_fg_color),
                        );
                    }
                    if wants_folder {
                        ui.horizontal(|ui| {
                            ui.label(match &ask.folder {
                                Some(folder) => crate::trf!("Folder: {}", folder.display()),
                                None => tr("No folder chosen yet.").to_string(),
                            });
                            choose = ui.button(tr("Choose…")).clicked();
                        });
                    }
                });
                d.footer(None, |ui| {
                    ui.add_enabled_ui(ready, |ui| {
                        allow = dialog::main_button(ui, tr(if running { ALLOW_AND_RUN } else { ALLOW }))
                            .clicked();
                    });
                    let label = if running {
                        tr("Don't run it")
                    } else {
                        tr("Cancel")
                    };
                    cancel = dialog::plain_button(ui, label).clicked();
                });
            });
        if choose {
            self.queued.push(WorkbenchAction::ChooseScriptFolder {
                path: None,
                start: ask.folder.clone(),
            });
        }
        if outcome.closed || cancel {
            self.scripts.asking_access = None;
            return;
        }
        if allow && ready {
            self.scripts.asking_access = None;
            self.scripts
                .settings
                .approve(&ask.script, &ask.asks, ask.folder.clone());
            self.note = Some(crate::trf!("{} is allowed what it asked for.", ask.name));
            match ask.then {
                Then::Nothing => {}
                Then::Command(key) => self.run_script_command(&key),
                Then::Run { file, name, source } => {
                    self.start_run(file, name, None, None, source);
                }
            }
        }
    }

    fn install_window(&mut self, ctx: &egui::Context) {
        let Some((installing, mut viewer)) = self.scripts.installing.take() else {
            return;
        };
        let (mut install, mut cancel) = (false, false);
        let header = installing.header.clone();
        let name = header.shown_name(&installing.file.name);
        let outcome = Dialog::new(crate::trf!("Install {}", name), 640.0)
            .id(r"install script")
            .body_height(360.0)
            .modal()
            .show(ctx, |d| {
                d.header(true, |_| {});
                d.body(|ui| {
                    dialog::heading(ui, &name);
                    ui.label(
                        RichText::new(crate::trf!(
                            "{} · scripting version {}",
                            installing.file.name,
                            header.api.unwrap_or(script::API_VERSION)
                        ))
                        .weak()
                        .small(),
                    );
                    if let Some(about) = &header.about {
                        ui.label(about);
                    }
                    if header.api.is_some_and(|v| v > script::API_VERSION) {
                        ui.label(
                            RichText::new(tr("It was written for a newer version of the app: it won't run here until the app is updated."))
                                .color(ui.visuals().warn_fg_color),
                        );
                    }
                    let asks: Vec<&str> = header.asks.iter().map(|a| tr(a.words())).collect();
                    ui.label(if asks.is_empty() {
                        tr("It asks for nothing outside the project.").to_string()
                    } else {
                        crate::trf!(
                            "It asks to: {}. You're asked to allow that when it first runs.",
                            asks.join(", ")
                        )
                    });
                    dialog::help(
                        ui,
                        tr("Read it before installing: installing runs nothing, but it can change your scene when you run it. Its code:"),
                    );
                    ui.allocate_ui(egui::vec2(ui.available_width(), 200.0), |ui| viewer.show(ui));
                    if let Some(why) = &installing.why {
                        ui.label(RichText::new(why).color(ui.visuals().error_fg_color));
                    }
                });
                d.footer(None, |ui| {
                    install = dialog::main_button(ui, tr(INSTALL)).clicked();
                    cancel = dialog::plain_button(ui, tr("Cancel")).clicked();
                });
            });
        if outcome.closed || cancel {
            return;
        }
        if install {
            return self.install(installing, viewer);
        }
        self.scripts.installing = Some((installing, viewer));
    }

    /// Adds the script being installed to your library.
    pub(crate) fn install(&mut self, mut installing: Installing, viewer: Viewer) {
        let Some(library) = self.scripts.library.clone() else {
            installing.why =
                Some(tr("Your script library isn't available on this computer.").to_string());
            self.scripts.installing = Some((installing, viewer));
            return;
        };
        match library.add(&installing.file, &self.scripts.names()) {
            Ok(path) => {
                let now = std::time::Instant::now();
                self.scripts.list_again(now);
                let file = super::files::file_name(&path);
                self.scripts.push(Row::Line(Line::Note(crate::trf!(
                    "{} was installed in your library.",
                    file
                ))));
                self.note = Some(crate::trf!(
                    "{} is installed: its commands are in the Scripts menu once it's read.",
                    file
                ));
            }
            Err(why) => {
                installing.why = Some(why);
                self.scripts.installing = Some((installing, viewer));
            }
        }
    }

    /// A command's choice and source, for running it.
    pub(super) fn command_source(key: &crate::script_commands::CommandKey) -> Source {
        if key.project {
            Source::Project(key.file.clone())
        } else {
            Source::Library(key.file.clone())
        }
    }

    /// The script manager's line for what a script asks and whether it's
    /// allowed: Allow… and Take back.
    pub(super) fn approval_line(&mut self, ui: &mut Ui, id: ScriptId, name: &str, asks: &[Ask]) {
        if asks.is_empty() {
            return;
        }
        let needed = self.scripts.settings.needs_approval(&id, asks);
        ui.horizontal(|ui| {
            if needed {
                ui.label(
                    RichText::new(tr("Not allowed yet: you're asked when it runs."))
                        .small()
                        .weak(),
                );
                if ui.small_button(tr("Allow…")).clicked() {
                    let folder = self
                        .scripts
                        .settings
                        .approval_of(&id)
                        .and_then(|a| a.folder.clone());
                    self.scripts.asking_access = Some(AccessAsk {
                        script: id.clone(),
                        name: name.to_string(),
                        asks: asks.to_vec(),
                        folder,
                        then: Then::Nothing,
                    });
                }
            } else {
                let folder = self
                    .scripts
                    .settings
                    .approval_of(&id)
                    .and_then(|a| a.folder.clone());
                ui.label(
                    RichText::new(match folder {
                        Some(folder) => crate::trf!("Allowed (folder: {}).", folder.display()),
                        None => tr("Allowed.").to_string(),
                    })
                    .small()
                    .weak(),
                );
                if ui
                    .small_button(tr("Take back"))
                    .on_hover_text(tr(
                        "Take back what you allowed: it's asked again when it next runs",
                    ))
                    .clicked()
                {
                    self.scripts.settings.take_back(&id);
                }
            }
        });
    }
}
