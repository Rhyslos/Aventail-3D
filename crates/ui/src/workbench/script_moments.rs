//! Scripts that run by themselves (decision 0309): when a project opens,
//! as it's saved and as an export starts, the scripts switched on for the
//! moment run one after another, in name order. Saving and exporting are
//! held back from the app until they've ended (each gets at most 2
//! seconds); a script can stop an export with a reason, and a slow or
//! failing script never stops anything.
//!
//! A project's own scripts take part only once they're turned on (trust,
//! decision 0307), and a script reaches only the files you've already
//! allowed it: one still asking is left out, with a note, since nobody is
//! there to answer while a save waits.

use super::{Workbench, WorkbenchAction};
use crate::script_commands::CommandKey;
use crate::script_moments::{Current, Job, MomentScript, OnCancel, moment_words};
use crate::script_sharing::ScriptId;
use crate::scripts_panel::{Ended, Row};
use crate::text::tr;
use fileio::export::{ExportFormat, ExportWhat};
use script::{Choice, ExportInfo, Line, Moment, Run, ScriptFile};
use std::collections::VecDeque;

impl Workbench {
    /// Whether any script is switched on for `moment` that may run in
    /// this project: one of yours turned on, or one of the project's
    /// turned on (trusted).
    pub(crate) fn has_moment_scripts(&self, moment: Moment) -> bool {
        let settings = &self.scripts.settings;
        let yours = settings.moments.iter().any(|(id, _)| {
            matches!(id, ScriptId::Library(file) if !settings.is_off(file))
                && settings.runs_when(id, moment)
        });
        yours
            || self.document.scene().scripts().iter().any(|s| {
                let fingerprint = script::fingerprint(&s.text);
                settings.is_trusted(&fingerprint)
                    && settings.runs_when(&ScriptId::Project(fingerprint), moment)
            })
    }

    /// A project was opened: its scripts for the moment run, if any.
    pub(super) fn project_opened_moment(&mut self) {
        if self.has_moment_scripts(Moment::ProjectOpened) {
            self.scripts.moments.jobs.push_back(Job {
                moment: Moment::ProjectOpened,
                export: None,
                then: None,
                on_cancel: OnCancel::Nothing,
            });
        }
    }

    /// Holds back the saves and exports among `actions` whose moments
    /// have scripts switched on: they're handed to the app once those
    /// scripts have run.
    pub(super) fn hold_for_moments(&mut self, actions: &mut Vec<WorkbenchAction>) {
        if !actions.iter().any(waits_for_moments) {
            return;
        }
        let mut kept = Vec::new();
        for action in std::mem::take(actions) {
            let jobs = self.jobs_for(&action);
            let Some(last) = jobs.len().checked_sub(1) else {
                kept.push(action);
                continue;
            };
            for (at, (moment, export, on_cancel)) in jobs.into_iter().enumerate() {
                self.scripts.moments.jobs.push_back(Job {
                    moment,
                    export,
                    then: (at == last).then(|| action.clone()),
                    on_cancel,
                });
            }
        }
        *actions = kept;
    }

    /// The moments `action` waits for, with what goes into an export.
    fn jobs_for(&mut self, action: &WorkbenchAction) -> Vec<(Moment, Option<ExportInfo>, OnCancel)> {
        let exporting = self.has_moment_scripts(Moment::ExportStarting);
        match action {
            WorkbenchAction::Save { .. } => {
                let mut jobs = Vec::new();
                if self.has_moment_scripts(Moment::ProjectSaved) {
                    jobs.push((Moment::ProjectSaved, None, OnCancel::Nothing));
                }
                let on_save = self
                    .document
                    .scene()
                    .export_target()
                    .is_some_and(|t| t.on_save);
                if exporting
                    && on_save
                    && let Some(target) = self.target_request(None)
                {
                    let format = target.preset.settings().format;
                    let info = self.export_info(target.options.what, &[], format);
                    jobs.push((Moment::ExportStarting, Some(info), OnCancel::SkipSaveExport));
                }
                jobs
            }
            WorkbenchAction::Export(request) if exporting => {
                let info = self.export_info(request.options.what, &[], request.format);
                vec![(Moment::ExportStarting, Some(info), OnCancel::DropAction)]
            }
            WorkbenchAction::ExportToTarget(request) if exporting => {
                let format = request.preset.settings().format;
                let info = self.export_info(request.options.what, &[], format);
                vec![(Moment::ExportStarting, Some(info), OnCancel::DropAction)]
            }
            WorkbenchAction::ExportKit(request) if exporting => {
                let info = self.export_info(ExportWhat::Selected, &request.pieces, request.format);
                vec![(Moment::ExportStarting, Some(info), OnCancel::DropAction)]
            }
            _ => Vec::new(),
        }
    }

    /// What goes into an export of `what` (the selection, or `pieces`
    /// when given): every object shown, in the Objects panel's order.
    fn export_info(
        &self,
        what: ExportWhat,
        pieces: &[scene::ObjectId],
        format: ExportFormat,
    ) -> ExportInfo {
        let scene = self.document.scene();
        let chosen: Vec<scene::ObjectId> = if pieces.is_empty() {
            self.document.selection().to_vec()
        } else {
            pieces.to_vec()
        };
        let inside = |id: scene::ObjectId| {
            chosen
                .iter()
                .any(|c| *c == id || scene.is_inside(id, *c))
        };
        let shapes = scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| scene.is_shown(*id))
            .filter(|id| {
                matches!(what, ExportWhat::Everything) && pieces.is_empty() || inside(*id)
            })
            .collect();
        ExportInfo { shapes, format }
    }

    /// Starts the next script of the moment under way, or the next moment
    /// waiting, when no script runs; hands on what waited once a moment's
    /// scripts have all ended.
    pub(super) fn poll_moments(&mut self) {
        loop {
            if self.scripts.is_running() {
                return;
            }
            if self.scripts.moments.current.is_none() {
                let Some(job) = self.scripts.moments.jobs.pop_front() else {
                    return;
                };
                let scripts = self.moment_scripts(job.moment);
                self.scripts.moments.current = Some(Current {
                    job,
                    scripts,
                    ran: Vec::new(),
                    failed: Vec::new(),
                    cancelled: None,
                });
            }
            let Some(current) = self.scripts.moments.current.as_mut() else {
                return;
            };
            let job = current.job.clone();
            match current.scripts.pop_front() {
                Some(next) => self.start_moment_run(next, &job),
                None => {
                    if let Some(done) = self.scripts.moments.current.take() {
                        self.finish_moment(done);
                    }
                }
            }
        }
    }

    /// The scripts switched on for `moment`, ready to run in name order;
    /// those that can't run now are said in the output.
    fn moment_scripts(&mut self, moment: Moment) -> VecDeque<MomentScript> {
        let mut found: Vec<(ScriptFile, ScriptId, bool)> = Vec::new();
        let mut notes = Vec::new();
        let settings = &self.scripts.settings;
        for (id, _) in &settings.moments {
            let ScriptId::Library(name) = id else {
                continue;
            };
            if settings.is_off(name) || !settings.runs_when(id, moment) {
                continue;
            }
            let Some(library) = &self.scripts.library else {
                continue;
            };
            match ScriptFile::read(&library.folder().join(name)) {
                Ok(file) => found.push((file, id.clone(), false)),
                Err(why) => notes.push(why),
            }
        }
        for kept in self.document.scene().scripts().iter() {
            let id = ScriptId::Project(script::fingerprint(&kept.text));
            let ScriptId::Project(fingerprint) = &id else {
                continue;
            };
            if settings.is_trusted(fingerprint) && settings.runs_when(&id, moment) {
                let file = ScriptFile {
                    name: kept.name.clone(),
                    text: kept.text.clone(),
                };
                found.push((file, id, true));
            }
        }
        for note in notes {
            self.scripts.push(Row::Line(Line::Note(note)));
        }
        let folder = self.project_folder();
        let mut ready = Vec::new();
        for (file, id, project) in found {
            let header = script::read_header(&file.text);
            let name = header.shown_name(&file.name);
            if self.scripts.settings.needs_approval(&id, &header.asks) {
                let why = crate::trf!(
                    "{} didn't run {}: it asks to reach files you haven't allowed yet. Allow it in Scripts → Script manager….",
                    name,
                    tr(moment_words(moment))
                );
                self.scripts.push(Row::Line(Line::Note(why)));
                continue;
            }
            let access =
                self.scripts
                    .settings
                    .access_for(Some(&id), &header.asks, folder.as_deref());
            let choices = self.choices_for(&file.name, project);
            ready.push(MomentScript {
                file,
                name,
                id,
                choices,
                access,
            });
        }
        ready.sort_by(|a, b| {
            (a.name.to_lowercase(), &a.file.name).cmp(&(b.name.to_lowercase(), &b.file.name))
        });
        ready.into()
    }

    /// The settings last used for each command of the script `file`.
    fn choices_for(&self, file: &str, project: bool) -> Vec<Choice> {
        self.scripts
            .settings
            .values
            .iter()
            .filter_map(|(key, _)| {
                let CommandKey {
                    file: of,
                    command: Some(command),
                    project: theirs,
                } = key
                else {
                    return None;
                };
                (of == file && *theirs == project).then(|| Choice {
                    command: command.clone(),
                    values: self.scripts.settings.values_of(key),
                })
            })
            .collect()
    }

    /// Starts `script` running by itself for `job`'s moment.
    fn start_moment_run(&mut self, script: MomentScript, job: &Job) {
        let mut snapshot = self.script_snapshot();
        snapshot.access = script.access;
        snapshot.export = job.export.clone();
        let limits = self.scripts.settings.limits();
        let names = self.scripts.names();
        let run = Run::start_moment(
            script.file,
            snapshot,
            limits,
            names,
            job.moment,
            script.choices,
        );
        self.scripts.limits = script::Limits {
            time: limits.time.min(script::MOMENT_LIMIT),
            memory: limits.memory,
        };
        self.document.hold();
        self.note = Some(crate::trf!(
            "Running {} {}… (Esc stops it)",
            script.name,
            tr(moment_words(job.moment))
        ));
        self.scripts.start(run, script.name, None);
        self.scripts.moments.running = true;
    }

    /// A moment's script has ended: what it did is noted for the moment.
    pub(super) fn moment_run_ended(&mut self, ended: &Ended) {
        self.scripts.moments.running = false;
        // Its own limits again for the next run.
        self.scripts.limits = self.scripts.settings.limits();
        let Some(current) = self.scripts.moments.current.as_mut() else {
            return;
        };
        if ended.ok {
            current.ran.push(ended.name.clone());
        } else {
            current.failed.push(ended.name.clone());
        }
        if current.cancelled.is_none()
            && let Some(why) = &ended.cancelled
        {
            current.cancelled = Some((ended.name.clone(), why.clone()));
        }
    }

    /// Every script of a moment has ended: hands on what waited (unless a
    /// script stopped the export), and says what ran.
    fn finish_moment(&mut self, done: Current) {
        let Current {
            job,
            ran,
            failed,
            cancelled,
            ..
        } = done;
        let mut words = Vec::new();
        if !ran.is_empty() {
            words.push(crate::trf!(
                "Ran by itself {}: {}.",
                tr(moment_words(job.moment)),
                ran.join(", ")
            ));
        }
        if !failed.is_empty() {
            words.push(crate::trf!(
                "{} didn't finish, so nothing it did was kept; the Scripts panel says why.",
                failed.join(", ")
            ));
        }
        let mut then = job.then;
        if let Some((by, why)) = cancelled {
            match job.on_cancel {
                OnCancel::Nothing => {}
                OnCancel::DropAction => {
                    then = None;
                    words.push(crate::trf!(
                        "The export didn't go ahead: {} stopped it ({}).",
                        by,
                        why
                    ));
                }
                OnCancel::SkipSaveExport => {
                    let skipped = crate::trf!(
                        "The export into the game project didn't go ahead: {} stopped it ({}).",
                        by,
                        why
                    );
                    self.scripts.moments.skip_save_export = Some(skipped);
                }
            }
        }
        let said = words.join(" ");
        if !said.is_empty() {
            self.scripts.push(Row::Line(Line::Note(said.clone())));
        }
        match then {
            Some(action) => {
                self.scripts.moments.said = (!said.is_empty()).then_some(said);
                self.scripts.moments.released.push(action);
            }
            None if !said.is_empty() => self.note = Some(said),
            None => {}
        }
    }

    /// What the status bar adds after a save or export that waited for
    /// scripts: which ran first.
    pub(super) fn moments_said(&mut self, note: &mut String) {
        if let Some(said) = self.scripts.moments.said.take() {
            note.push(' ');
            note.push_str(&said);
        }
    }

    /// [`Self::moments_said`] added to the status bar's note.
    pub(super) fn add_moments_said(&mut self) {
        let mut note = self.note.take().unwrap_or_default();
        self.moments_said(&mut note);
        self.note = Some(note.trim_start().to_string());
    }

    /// The saves and exports held back for scripts that are ready now,
    /// for the app: a tab not in use hands them on too.
    pub fn released_actions(&mut self) -> Vec<WorkbenchAction> {
        std::mem::take(&mut self.scripts.moments.released)
    }
}

/// Whether `action` is a save or an export, which may wait for scripts.
fn waits_for_moments(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::Save { .. }
            | WorkbenchAction::Export(_)
            | WorkbenchAction::ExportToTarget(_)
            | WorkbenchAction::ExportKit(_)
    )
}
