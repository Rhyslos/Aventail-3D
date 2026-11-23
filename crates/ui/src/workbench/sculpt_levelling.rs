//! Detail levels in the workbench ([decision 0315](../../../../docs/decisions/0315-detail-levels.md)):
//! Add a level, Page Up and Page Down, Delete higher and lower levels,
//! Rebuild lower levels, and the level exports and bakes take. Each is
//! one undo step: a `Command::Sculpt` carrying the sculpt with its levels
//! changed, or a `Command::RebaseSculpt` when the shape under the sculpt
//! changes (Delete lower levels, Rebuild lower levels).
//!
//! On a shape of many polygons the work runs on another thread: the live
//! sculpt goes there and comes back with the result, strokes waiting
//! meanwhile. Adding a level past [`HEAVY_POLYGONS`] asks first; past
//! [`MAX_POLYGONS`] it is refused before it starts.

use super::Workbench;
use super::sculpting::{Live, READY_IN_BACKGROUND_FROM};
use crate::sculpt_levels::polygons_text;
use crate::sculpt_masks::SculptAction;
use crate::text::tr;
use egui::{Key, Modifiers};
use scene::{Command, Mesh, ObjectId};
use sculpt::levels::{HEAVY_POLYGONS, MAX_LEVELS, MAX_POLYGONS};
use sculpt::rebuild::{RebuildProblem, RebuildProgress, Rebuilt};
use sculpt::{LevelProblem, LiveSculpt, Sculpt};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// What a level action gave.
enum Outcome {
    /// The sculpt with its levels changed (`None`: nothing to change).
    Sculpt(Option<Sculpt>),
    /// A level couldn't be added.
    Problem(LevelProblem),
    /// A shape of its own for the sculpt, with the sculpt made for it
    /// (`None`: it couldn't be worked out).
    Rebase(Option<(Arc<Mesh>, Sculpt)>),
    /// Lower levels rebuilt, or why not.
    Rebuilt(Result<Rebuilt, RebuildProblem>),
}

/// A level action carried out: the live sculpt back, and what it gave.
struct Done {
    live: LiveSculpt,
    outcome: Outcome,
}

/// The object a level action works on, and the document's sculpting
/// when it started: if that changes before it ends, its result is
/// dropped.
struct Order {
    id: ObjectId,
    action: SculptAction,
    doc: Option<Sculpt>,
    input: Arc<Mesh>,
    key: u64,
}

/// A level action under way on another thread.
struct Running {
    order: Order,
    job: JoinHandle<Done>,
    /// How far a rebuild is, and its way to stop.
    progress: Option<Arc<RebuildProgress>>,
}

/// The detail levels' state in Sculpt mode.
#[derive(Default)]
pub(super) struct Levelling {
    running: Option<Running>,
    /// A heavy level waiting for its question's answer: how many polygons
    /// it would have.
    asking: Option<u64>,
}

impl Levelling {
    /// Whether a level is being worked out on another thread.
    pub(super) fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// Stops what can be stopped, and the question (leaving Sculpt mode).
    pub(super) fn stop(&mut self) {
        if let Some(progress) = self.running.as_ref().and_then(|r| r.progress.as_ref()) {
            progress.stop();
        }
        self.running = None;
        self.asking = None;
    }

    /// Whether a heavy level waits for its answer.
    #[cfg(test)]
    pub(super) fn asking(&self) -> bool {
        self.asking.is_some()
    }

    /// Waits for the work under way to end (for tests).
    #[cfg(test)]
    pub(super) fn wait(&self) {
        while self.running.as_ref().is_some_and(|r| !r.job.is_finished()) {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Workbench {
    /// Sculpt mode's keys for detail levels: Page Up and Page Down.
    pub(super) fn sculpt_level_keys(&mut self, ui: &egui::Ui) {
        let pressed = |key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(Key::PageUp) {
            self.run_sculpt_action(SculptAction::HigherLevel);
        }
        if pressed(Key::PageDown) {
            self.run_sculpt_action(SculptAction::LowerLevel);
        }
    }

    /// What the Sculpt tab's Detail levels section shows, each frame.
    pub(super) fn show_levels(&mut self) {
        let busy = self.sculpting.levelling.busy();
        let settings = &mut self.attributes.sculpt.levels;
        settings.busy = busy;
        if let Some(live) = &self.sculpting.live {
            settings.levels = Some((live.live.level(), live.live.top_level()));
            settings.polygons = live.live.mesh().face_count() as u64;
            settings.next_polygons = live.live.polygons_if_added();
            settings.export = live.live.sculpt().export();
        } else if !busy {
            settings.levels = None;
        }
        if let Some(export) = self.attributes.sculpt.levels.export_asked.take() {
            self.set_export_level(export);
        }
    }

    /// Sets which detail level exports and bakes take, as one undo step.
    fn set_export_level(&mut self, export: sculpt::ExportLevel) {
        if self.sculpting.stroke.is_some() || self.level_work_waits() {
            return;
        }
        let Some(live) = self.sculpting.live.as_mut() else {
            return;
        };
        let Some(new) = live.live.set_export(export) else {
            return;
        };
        self.send_level_sculpt(new, tr("Detail level for export"), None);
        self.note = Some(
            tr("Exports and bakes take that detail level now; the view shows the one you sculpt on.")
                .to_string(),
        );
    }

    /// Whether something else must end first, saying so.
    fn level_work_waits(&mut self) -> bool {
        if self.sculpting.remeshing.busy() {
            self.note = Some(tr("Remeshing: a moment.").to_string());
            return true;
        }
        if self.sculpting.levelling.busy() {
            self.note = Some(tr("Working out a detail level: a moment.").to_string());
            return true;
        }
        false
    }

    /// Carries out a detail level action on the shape being sculpted.
    pub(super) fn run_level_action(&mut self, action: SculptAction) {
        if self.sculpting.stroke.is_some() || self.sculpting.moving.is_some() {
            return;
        }
        if self.level_work_waits() {
            return;
        }
        let Some(live) = self.sculpting.live.as_ref() else {
            self.note = Some(tr("Select a shape to sculpt it.").to_string());
            return;
        };
        let (level, top) = (live.live.level(), live.live.top_level());
        // What can be told at once.
        let refusal = match action {
            SculptAction::HigherLevel if level >= top => Some(if top >= MAX_LEVELS {
                tr("This is the finest detail level.").to_string()
            } else {
                tr("This is the finest detail level. Add a level (in the Sculpt tab's Detail levels) for a finer one.").to_string()
            }),
            SculptAction::LowerLevel if level == 0 => Some(
                tr("This is the lowest detail level: the shape as it came in. Rebuild lower levels makes coarser ones.")
                    .to_string(),
            ),
            SculptAction::DeleteHigherLevels if level >= top => {
                Some(tr("There are no detail levels above this one.").to_string())
            }
            SculptAction::DeleteLowerLevels if level == 0 => {
                Some(tr("There are no detail levels below this one.").to_string())
            }
            SculptAction::RebuildLowerLevels if level < top => Some(
                tr("Go to the finest detail level first (Page Up), or delete the higher levels: lower levels are rebuilt for the shape with all its detail.")
                    .to_string(),
            ),
            SculptAction::AddLevel if top >= MAX_LEVELS => Some(crate::trf!(
                "There are {} detail levels already, the most there can be.",
                MAX_LEVELS
            )),
            _ => None,
        };
        if let Some(why) = refusal {
            self.note = Some(why);
            return;
        }
        if action == SculptAction::AddLevel {
            let polygons = live.live.polygons_if_added();
            if polygons > MAX_POLYGONS {
                self.note = Some(too_heavy(polygons));
                return;
            }
            if polygons > HEAVY_POLYGONS {
                self.sculpting.levelling.asking = Some(polygons);
                return;
            }
        }
        self.start_level_work(action);
    }

    /// Starts `action`: at once on a light shape, on another thread on a
    /// heavy one.
    fn start_level_work(&mut self, action: SculptAction) {
        let Some(Live { id, live, doc }) = self.sculpting.live.take() else {
            return;
        };
        // The finest level's polygons: what going up, adding or baking
        // may have to work out.
        let heaviest = (live.polygons_if_added() / 4).max(live.mesh().face_count() as u64);
        let rebuild = action == SculptAction::RebuildLowerLevels;
        let background = rebuild
            || heaviest >= READY_IN_BACKGROUND_FROM as u64
            || (action == SculptAction::AddLevel
                && live.polygons_if_added() >= READY_IN_BACKGROUND_FROM as u64);
        let order = Order {
            id,
            action,
            doc,
            input: Arc::clone(live.input()),
            key: self.new_step_key(),
        };
        let wanted = self.attributes.sculpt.levels.rebuild;
        let progress = rebuild.then(|| Arc::new(RebuildProgress::default()));
        let watch = progress.clone();
        let work = move || carry_out(live, action, wanted, watch.as_deref());
        if background {
            self.sculpting.levelling.running = Some(Running {
                order,
                job: std::thread::spawn(work),
                progress,
            });
        } else {
            let done = work();
            self.finish_level_work(order, done);
        }
    }

    /// Each frame in Sculpt mode: a level worked out on another thread
    /// becomes one undo step.
    pub(super) fn keep_levelling(&mut self, ctx: &egui::Context) {
        let Some(running) = &self.sculpting.levelling.running else {
            return;
        };
        if !running.job.is_finished() {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        }
        let Some(Running { order, job, .. }) = self.sculpting.levelling.running.take() else {
            return;
        };
        match job.join() {
            Ok(done) => self.finish_level_work(order, done),
            // The work failed: the live sculpt is made again from the
            // document.
            Err(_) => self.note = Some(tr("Couldn't work out the detail level.").to_string()),
        }
    }

    /// A level action's result: one undo step, or why not.
    fn finish_level_work(&mut self, order: Order, done: Done) {
        let Order {
            id,
            action,
            doc,
            input,
            key,
        } = order;
        let target = self.document.scene().sculpt_target(id);
        let same = target
            .as_ref()
            .is_some_and(|t| Arc::ptr_eq(&t.input, &input) && t.sculpt == doc);
        // The live sculpt comes back as the document had it when the work
        // started: if the document changed meanwhile, it follows it again.
        self.sculpting.live = Some(Live {
            id,
            live: done.live,
            doc,
        });
        if !same {
            self.note = Some(
                tr("The sculpt changed while its detail level was worked out, so that was left out: try again.")
                    .to_string(),
            );
            return;
        }
        let name = tr(action.name());
        match done.outcome {
            Outcome::Sculpt(Some(new)) => {
                let (level, top) = (new.shown(), new.top());
                self.send_level_sculpt(new, name, Some(key));
                let polygons = self
                    .sculpting
                    .live
                    .as_ref()
                    .map_or(0, |l| l.live.mesh().face_count() as u64);
                self.note = Some(match action {
                    SculptAction::AddLevel => crate::trf!(
                        "Added detail level {}: {}. Page Down goes back to the level below for big changes.",
                        level,
                        polygons_text(polygons)
                    ),
                    SculptAction::DeleteHigherLevels => crate::trf!(
                        "Deleted the detail levels above level {}. Ctrl + Z brings them back.",
                        level
                    ),
                    _ => crate::trf!(
                        "Detail level {} of {}: {}.",
                        level,
                        top,
                        polygons_text(polygons)
                    ),
                });
            }
            Outcome::Sculpt(None) => {}
            Outcome::Problem(LevelProblem::TooHeavy { polygons }) => {
                self.note = Some(too_heavy(polygons));
            }
            Outcome::Problem(LevelProblem::TooManyLevels) => {
                self.note = Some(crate::trf!(
                    "There are {} detail levels already, the most there can be.",
                    MAX_LEVELS
                ));
            }
            Outcome::Problem(LevelProblem::NoShape) | Outcome::Rebase(None) => {
                self.note = Some(
                    tr("Couldn't work out the detail level: a point of the shape isn't at a real position.")
                        .to_string(),
                );
            }
            Outcome::Rebase(Some((shape, sculpt))) => {
                self.rebase_sculpt(id, shape, sculpt, name, key);
                self.note = Some(
                    tr("Deleted the lower detail levels: the level shown is level 0 now, baked into the shape. Ctrl + Z takes it back.")
                        .to_string(),
                );
            }
            Outcome::Rebuilt(Ok(rebuilt)) => {
                let levels = rebuilt.sculpt.top();
                let polygons = rebuilt.base.face_count() as u64;
                self.rebase_sculpt(id, Arc::new(rebuilt.base), rebuilt.sculpt, name, key);
                self.note = Some(crate::trf!(
                    "Rebuilt {} lower detail levels: level 0 has {}. Page Down goes down to them; Ctrl + Z takes it back.",
                    levels,
                    polygons_text(polygons)
                ));
            }
            Outcome::Rebuilt(Err(why)) => {
                self.note = Some(match why {
                    RebuildProblem::Cancelled => {
                        tr("Rebuilding stopped: nothing changed.").to_string()
                    }
                    RebuildProblem::TooFew => crate::trf!(
                        "The shape has too few polygons for a lower detail level: the lowest keeps at least {}.",
                        sculpt::rebuild::FEWEST_POLYGONS
                    ),
                    RebuildProblem::Remesh(why) => {
                        crate::trf!("Couldn't rebuild lower levels: {}.", why)
                    }
                    RebuildProblem::Level => {
                        tr("Couldn't rebuild lower levels: a level couldn't be subdivided.")
                            .to_string()
                    }
                });
            }
        }
    }

    /// Sends the live sculpt's new sculpt to the document as one undo
    /// step labelled `label`.
    fn send_level_sculpt(&mut self, sculpt: Sculpt, label: &str, key: Option<u64>) {
        let key = key.unwrap_or_else(|| self.new_step_key());
        let Some(live) = self.sculpting.live.as_mut() else {
            return;
        };
        let id = live.id;
        let command = Command::Sculpt {
            id,
            sculpt: sculpt.clone(),
        };
        match self.document.execute_labelled(command, label, Some(key)) {
            Ok(_) => {
                live.doc = Some(sculpt);
                let shape = live.live.shape();
                self.document.scene().prime_sculpt(id, shape);
            }
            Err(why) => {
                let back = live
                    .doc
                    .clone()
                    .unwrap_or_else(|| Sculpt::new(live.live.input()));
                live.live.follow(&back);
                self.note = Some(crate::trf!("Couldn't change the detail level: {}.", why));
            }
        }
    }

    /// Makes `shape` object `id`'s shape with `sculpt` on it, as one undo
    /// step. The live sculpt is made again for the new shape.
    fn rebase_sculpt(
        &mut self,
        id: ObjectId,
        shape: Arc<Mesh>,
        sculpt: Sculpt,
        label: &str,
        key: u64,
    ) {
        let command = Command::RebaseSculpt { id, shape, sculpt };
        if let Err(why) = self.document.execute_labelled(command, label, Some(key)) {
            self.note = Some(crate::trf!("Couldn't change the detail levels: {}.", why));
        }
    }

    /// The detail levels' windows: the question before a heavy level, and
    /// the progress of a rebuild under way, with Cancel (Esc).
    pub(super) fn level_windows(&mut self, ctx: &egui::Context) {
        if let Some(polygons) = self.sculpting.levelling.asking {
            let mut answer = None;
            crate::dialog::Dialog::new(tr("Add a heavy detail level?"), 460.0)
                .id("sculpt_level_heavy")
                .modal()
                .show(ctx, |d| {
                    d.header(false, |_| {});
                    d.body(|ui| {
                        ui.label(crate::trf!(
                            "One more detail level would have {}. Past {} million, sculpting \
                             slows down and baking takes longer.",
                            polygons_text(polygons),
                            HEAVY_POLYGONS / 1_000_000
                        ));
                        crate::dialog::help(
                            ui,
                            tr("The levels you have stay as they are either way. Esc: don't add it"),
                        );
                    });
                    d.footer(None, |ui| {
                        if crate::dialog::main_button(ui, tr("Add it anyway")).clicked() {
                            answer = Some(true);
                        }
                        if crate::dialog::plain_button(ui, tr("Don't add it")).clicked() {
                            answer = Some(false);
                        }
                    });
                });
            if answer.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                answer = Some(false);
            }
            if let Some(add) = answer {
                self.sculpting.levelling.asking = None;
                if add {
                    self.start_level_work(SculptAction::AddLevel);
                }
            }
            return;
        }
        let Some(running) = &self.sculpting.levelling.running else {
            return;
        };
        let name = self
            .document
            .scene()
            .object(running.order.id)
            .map_or_else(String::new, |o| o.name.clone());
        let mut cancel = false;
        // At the bottom, out of the shape's way.
        crate::dialog::Dialog::new(tr(running.order.action.name()), 400.0)
            .id("sculpt_levelling")
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -90.0])
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(crate::trf!("Working out {}'s detail levels…", name));
                    if let Some(progress) = &running.progress {
                        ui.add(egui::ProgressBar::new(progress.share()).show_percentage());
                    }
                });
                if running.progress.is_some() {
                    d.footer(None, |ui| {
                        if crate::dialog::plain_button(ui, tr("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                }
            });
        if let Some(progress) = &running.progress
            && (cancel || ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)))
        {
            progress.stop();
        }
    }

    /// Whether the detail levels' windows want the keys now.
    pub(super) fn levels_hold_keys(&self) -> bool {
        let state = &self.sculpting.levelling;
        state.asking.is_some() || state.running.is_some()
    }
}

/// Why a level that would have `polygons` polygons is refused.
fn too_heavy(polygons: u64) -> String {
    crate::trf!(
        "One more detail level would have {}: past {} million it is refused. Sculpt on the levels you have, or delete lower levels and remesh.",
        polygons_text(polygons),
        MAX_POLYGONS / 1_000_000
    )
}

/// Carries `action` out on `live`, wherever this runs.
fn carry_out(
    mut live: LiveSculpt,
    action: SculptAction,
    rebuild: usize,
    progress: Option<&RebuildProgress>,
) -> Done {
    let level = live.level();
    let outcome = match action {
        SculptAction::AddLevel => match live.add_level() {
            Ok(new) => Outcome::Sculpt(Some(new)),
            Err(why) => Outcome::Problem(why),
        },
        SculptAction::HigherLevel => Outcome::Sculpt(live.set_level(level + 1)),
        SculptAction::LowerLevel => match level.checked_sub(1) {
            Some(lower) => Outcome::Sculpt(live.set_level(lower)),
            None => Outcome::Sculpt(None),
        },
        SculptAction::DeleteHigherLevels => Outcome::Sculpt(live.delete_higher()),
        SculptAction::DeleteLowerLevels => Outcome::Rebase(live.without_lower()),
        SculptAction::RebuildLowerLevels => {
            let none = RebuildProgress::default();
            let shape = live.shape();
            let mask = live.has_mask().then(|| live.mask().clone());
            Outcome::Rebuilt(sculpt::rebuild::rebuild_lower(
                &shape,
                rebuild,
                mask.as_ref(),
                progress.unwrap_or(&none),
            ))
        }
        _ => Outcome::Sculpt(None),
    };
    Done { live, outcome }
}
