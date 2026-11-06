//! UV → Auto-unwrap ([decision 0245](../../../../docs/decisions/0245-explained-auto-unwrap.md)):
//! one button runs the automatic UV tools in order (suggest seams,
//! unwrap, straighten, texel density, pack and lightmap UVs), each
//! recorded as its own edit in a new edit layer called "Auto-unwrap", all
//! one undo step. A window then lists the steps with what each did, and
//! each can be switched off (and on) on its own.

use super::Workbench;
use super::joining::counted;
use crate::text::tr;
use egui::Ui;
use scene::{Command, EditId, FaceId, LayerChange, ObjectId, Operation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The menu's name, the button's, the layer's and the window's.
pub(crate) const AUTO_UNWRAP_LABEL: &str = crate::text!("Auto-unwrap");
/// What it does, for the button's tooltip.
pub(crate) const AUTO_UNWRAP_HELP: &str = crate::text!(
    "Suggest seams, unwrap, straighten, set the texel density, pack and make lightmap UVs in one \
     go, each step an edit of its own in an Auto-unwrap edit layer, then see what each did and \
     switch any off"
);
/// The button that runs it.
pub(crate) const RUN_LABEL: &str = crate::text!("Auto-unwrap the selection");

/// Which of the steps Auto-unwrap runs, chosen in its menu and remembered
/// with the UV settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoSteps {
    /// Suggest seams.
    pub seams: bool,
    /// Unwrap the whole of each object.
    pub unwrap: bool,
    /// Straighten pieces made of quads.
    pub straighten: bool,
    /// Set every piece to the project's texel density.
    pub density: bool,
    /// Pack every piece.
    pub pack: bool,
    /// Make lightmap UVs.
    pub lightmap: bool,
}

impl Default for AutoSteps {
    fn default() -> Self {
        Self {
            seams: true,
            unwrap: true,
            straighten: true,
            density: true,
            pack: true,
            lightmap: true,
        }
    }
}

/// One step of Auto-unwrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoStep {
    Seams,
    Unwrap,
    Straighten,
    Density,
    Pack,
    Lightmap,
}

impl AutoStep {
    /// The steps, in the order they run.
    pub(crate) const ALL: [AutoStep; 6] = [
        Self::Seams,
        Self::Unwrap,
        Self::Straighten,
        Self::Density,
        Self::Pack,
        Self::Lightmap,
    ];

    /// Its name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Seams => tr("Suggest seams"),
            Self::Unwrap => tr("Unwrap"),
            Self::Straighten => tr("Straighten"),
            Self::Density => tr("Set texel density"),
            Self::Pack => tr("Pack"),
            Self::Lightmap => tr("Lightmap UVs"),
        }
    }

    /// What it does, for its tooltip.
    pub(crate) fn help(self) -> &'static str {
        match self {
            Self::Seams => tr(
                "Cut the shapes where they need it to lie flat, by the seam settings (UV → \
                 Settings)",
            ),
            Self::Unwrap => tr("Lay every piece flat, by the unwrap settings"),
            Self::Straighten => tr(
                "Line up pieces made of quads in rows and columns, so trim and tiling textures \
                 run straight",
            ),
            Self::Density => tr(
                "Give every piece the project's texel density, so they all get the same amount \
                 of texture for their size",
            ),
            Self::Pack => tr("Fit every piece into the texture square, by the packing settings"),
            Self::Lightmap => tr("A second UV set for Godot's baked light, exported as UV2"),
        }
    }

    /// Whether `steps` runs it.
    pub(crate) fn on(self, steps: &AutoSteps) -> bool {
        match self {
            Self::Seams => steps.seams,
            Self::Unwrap => steps.unwrap,
            Self::Straighten => steps.straighten,
            Self::Density => steps.density,
            Self::Pack => steps.pack,
            Self::Lightmap => steps.lightmap,
        }
    }

    /// Its switch in `steps`.
    pub(crate) fn switch(self, steps: &mut AutoSteps) -> &mut bool {
        match self {
            Self::Seams => &mut steps.seams,
            Self::Unwrap => &mut steps.unwrap,
            Self::Straighten => &mut steps.straighten,
            Self::Density => &mut steps.density,
            Self::Pack => &mut steps.pack,
            Self::Lightmap => &mut steps.lightmap,
        }
    }
}

/// What one step did: its words, and the edits it recorded on each
/// object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StepDone {
    pub(crate) step: AutoStep,
    pub(crate) said: String,
    pub(crate) edits: Vec<(ObjectId, EditId)>,
}

/// The last Auto-unwrap's steps, for its window.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AutoReport {
    pub(crate) steps: Vec<StepDone>,
    /// Whether the window is open.
    pub(crate) open: bool,
}

impl Workbench {
    /// The menu: which steps run, and the button.
    pub(super) fn auto_unwrap_entries(&mut self, ui: &mut Ui) {
        ui.label(egui::RichText::new(tr("Steps")).strong());
        for step in AutoStep::ALL {
            ui.checkbox(step.switch(&mut self.uv_settings.auto_steps), tr(step.label()))
                .on_hover_text(tr(step.help()));
        }
        ui.separator();
        if ui
            .button(tr(RUN_LABEL))
            .on_hover_text(tr(AUTO_UNWRAP_HELP))
            .clicked()
        {
            self.run_auto_unwrap();
            ui.close();
        }
        let has_report = self.auto_report.as_ref().is_some_and(|r| !r.steps.is_empty());
        if ui
            .add_enabled(has_report, egui::Button::new(tr("Show the last auto-unwrap")))
            .clicked()
        {
            if let Some(report) = &mut self.auto_report {
                report.open = true;
            }
            ui.close();
        }
    }

    /// Runs the chosen steps on every selected object as one undo step,
    /// in a new edit layer, and opens the window listing them.
    pub(crate) fn run_auto_unwrap(&mut self) {
        let shapes: Vec<_> = self
            .own_shapes()
            .into_iter()
            .filter(|(_, mesh)| mesh.face_count() > 0)
            .collect();
        if shapes.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects first.",
                tr(AUTO_UNWRAP_LABEL)
            ));
            return;
        }
        let steps = self.uv_settings.auto_steps;
        if !AutoStep::ALL.iter().any(|s| s.on(&steps)) {
            self.note = Some(crate::trf!(
                "{}: every step is switched off.",
                tr(AUTO_UNWRAP_LABEL)
            ));
            return;
        }
        let ids: Vec<ObjectId> = shapes.iter().map(|(id, _)| *id).collect();
        let key = self.new_step_key();
        let label = tr(AUTO_UNWRAP_LABEL);
        if !self.add_auto_layer(&ids, label, key) {
            return;
        }
        let mut done = Vec::new();
        for step in AutoStep::ALL.into_iter().filter(|s| s.on(&steps)) {
            let before: Vec<(ObjectId, EditId)> = ids
                .iter()
                .filter_map(|id| Some((*id, self.document.scene().object(*id)?.stack.next_edit_id())))
                .collect();
            let said = self.auto_step(step, key);
            let edits = before
                .into_iter()
                .flat_map(|(id, from)| {
                    self.document
                        .scene()
                        .object(id)
                        .map(|o| {
                            o.stack
                                .layers()
                                .iter()
                                .flat_map(|l| &l.entries)
                                .filter(|e| e.id >= from)
                                .map(|e| (id, e.id))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect();
            done.push(StepDone { step, said, edits });
        }
        let objects = counted("", ids.len(), tr("object"));
        self.note = Some(crate::trf!(
            "{} {}: {} steps, each an edit in the Auto-unwrap layer. Ctrl + Z takes it all back.",
            label,
            objects.trim_start(),
            done.len()
        ));
        self.auto_report = Some(AutoReport {
            steps: done,
            open: true,
        });
    }

    /// Adds an edit layer called Auto-unwrap on top of each object, as
    /// part of undo step `key`.
    fn add_auto_layer(&mut self, ids: &[ObjectId], label: &str, key: u64) -> bool {
        if let Err(why) =
            self.document
                .execute_all_labelled([Command::AddLayer(ids.to_vec())], label, key)
        {
            self.note = Some(crate::trf!("Couldn't do that: {}.", why));
            return false;
        }
        let renames: Vec<Command> = ids
            .iter()
            .filter_map(|id| {
                let layers = self.document.scene().object(*id)?.stack.layers().len();
                Some(Command::ChangeLayer {
                    id: *id,
                    index: layers.checked_sub(1)?,
                    change: LayerChange::Rename(label.to_string()),
                })
            })
            .collect();
        if let Err(why) = self.document.execute_all_labelled(renames, label, key) {
            self.note = Some(crate::trf!("Couldn't do that: {}.", why));
            return false;
        }
        true
    }

    /// Runs one step on the selected objects as part of undo step `key`:
    /// what it did, in words.
    fn auto_step(&mut self, step: AutoStep, key: u64) -> String {
        let label = tr(step.label());
        match step {
            AutoStep::Seams => match self.suggest_seams_in(key) {
                Some((total, changed)) => crate::trf!(
                    "{} ({} edges changed)",
                    counted("", total, tr("seam")).trim_start(),
                    changed
                ),
                None => tr("The seams were already where they'd be suggested").to_string(),
            },
            AutoStep::Unwrap => {
                let ops = self.whole_ops(|w, mesh, faces| {
                    w.uv_tool_op(super::unwrapping::UvTool::Unwrap, mesh, faces)
                });
                self.record_each_in(label, ops, key);
                counted("", self.piece_count(), tr("piece")).trim_start().to_string()
            }
            AutoStep::Straighten => {
                let quads = self.quad_piece_count();
                if quads == 0 {
                    return tr("No piece is made only of quads: nothing to straighten").to_string();
                }
                let ops = self.whole_ops(|w, mesh, faces| {
                    w.uv_tool_op(super::unwrapping::UvTool::Straighten, mesh, faces)
                });
                self.record_each_in(label, ops, key);
                crate::trf!(
                    "{} straightened",
                    counted("", quads, tr("piece of quads")).trim_start()
                )
            }
            AutoStep::Density => {
                let texture = self.document.scene().texture();
                let density = texture.uv_per_unit();
                let ops = self.whole_ops(|_, mesh, faces| {
                    let anchors = super::unwrapping::anchors(mesh, &faces);
                    Operation::SetTexelDensity {
                        faces,
                        density,
                        anchors,
                    }
                });
                self.record_each_in(label, ops, key);
                crate::trf!("{:.0} px/m", texture.pixels_per_metre(density))
            }
            AutoStep::Pack => {
                let shapes = self.own_shapes();
                if self.pack_shapes(shapes, true, Some(key)) {
                    crate::trf!(
                        "{} packed, {:.0}% of the texture used",
                        counted("", self.piece_count(), tr("piece")).trim_start(),
                        self.texture_used() * 100.0
                    )
                } else {
                    self.note.clone().unwrap_or_default()
                }
            }
            AutoStep::Lightmap => {
                let size = self.uv_settings.lightmap_size;
                let padding = self.uv_settings.lightmap_padding.clamp(1, 64);
                let ops: Vec<(ObjectId, Operation)> = self
                    .own_shapes()
                    .into_iter()
                    .map(|(id, _)| {
                        (
                            id,
                            Operation::MakeLightmap {
                                size,
                                padding,
                                hardness: self.smoothing_of(id),
                            },
                        )
                    })
                    .collect();
                self.record_each_in(label, ops, key);
                crate::trf!("A lightmap UV set, {} × {} px, exported as UV2", size, size)
            }
        }
    }

    /// One operation per selected object, on all its polygons.
    fn whole_ops(
        &self,
        op: impl Fn(&Self, &scene::Mesh, Vec<FaceId>) -> Operation,
    ) -> Vec<(ObjectId, Operation)> {
        self.own_shapes()
            .into_iter()
            .filter(|(_, mesh)| mesh.face_count() > 0)
            .map(|(id, mesh)| {
                let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
                (id, op(self, &mesh, faces))
            })
            .collect()
    }

    /// Each selected object's pieces, as their polygons.
    fn pieces(&self) -> Vec<(Arc<scene::Mesh>, Vec<FaceId>)> {
        self.own_shapes()
            .into_iter()
            .flat_map(|(_, mesh)| {
                let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
                uv::pack::islands_of(&mesh, &faces)
                    .into_iter()
                    .map(move |island| (Arc::clone(&mesh), island))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// How many pieces the selected objects have.
    fn piece_count(&self) -> usize {
        self.pieces().len()
    }

    /// How many of the selected objects' pieces are made only of quads.
    fn quad_piece_count(&self) -> usize {
        self.pieces()
            .iter()
            .filter(|(mesh, faces)| {
                faces
                    .iter()
                    .all(|f| mesh.face(*f).is_some_and(|l| l.len() == 4))
            })
            .count()
    }

    /// How much of the texture square the selected objects' pieces
    /// cover (0 to 1; overlapping pieces count twice).
    fn texture_used(&self) -> f32 {
        let tiles = self.uv_settings.udim_tiles.clamp(1, uv::pack::MAX_TILES) as f32;
        let flat: f32 = self
            .own_shapes()
            .iter()
            .filter_map(|(_, mesh)| {
                let uvs = mesh::uvs::Uvs::of(mesh)?;
                let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
                Some(uv::density::areas(mesh, &faces, &uvs).0)
            })
            .sum();
        (flat / tiles).clamp(0.0, 1.0)
    }

    /// The window listing the last Auto-unwrap's steps, while open.
    pub(super) fn auto_unwrap_window(&mut self, ctx: &egui::Context) {
        let Some(report) = &self.auto_report else {
            return;
        };
        if !report.open {
            return;
        }
        let steps = report.steps.clone();
        let mut switch: Option<(usize, bool)> = None;
        let mut close = false;
        let outcome = crate::dialog::Dialog::new(tr(AUTO_UNWRAP_LABEL), 420.0)
            .anchor(egui::Align2::RIGHT_TOP, [-24.0, 72.0])
            .show(ctx, |d| {
                d.header(true, |_| {});
                d.body(|ui| {
                    ui.label(tr(
                        "Each step is an edit of its own in the Auto-unwrap edit layer. Switch \
                         one off to see the result without it; change it in the Edit layers \
                         panel.",
                    ));
                    ui.add_space(6.0);
                    for (i, done) in steps.iter().enumerate() {
                        let on = self.step_enabled(done);
                        let mut shown = on;
                        ui.horizontal(|ui| {
                            let response = ui
                                .add_enabled(
                                    !done.edits.is_empty(),
                                    egui::Checkbox::new(&mut shown, tr(done.step.label())),
                                )
                                .on_hover_text(tr(done.step.help()));
                            if response.changed() {
                                switch = Some((i, shown));
                            }
                        });
                        ui.label(
                            egui::RichText::new(&done.said)
                                .font(crate::dialog::help_font())
                                .color(crate::theme::TEXT_WEAK),
                        );
                    }
                });
                d.footer(None, |ui| {
                    if crate::dialog::plain_button(ui, tr("Close")).clicked() {
                        close = true;
                    }
                });
            });
        if let Some((i, on)) = switch {
            self.switch_auto_step(&steps[i], on);
        }
        if (close || outcome.closed)
            && let Some(report) = &mut self.auto_report
        {
            report.open = false;
        }
    }

    /// Whether a step's edits are all switched on (and still there).
    fn step_enabled(&self, done: &StepDone) -> bool {
        !done.edits.is_empty()
            && done.edits.iter().all(|(id, edit)| {
                self.document
                    .scene()
                    .object(*id)
                    .and_then(|o| o.stack.entry(*edit).map(|(_, _, e)| e.enabled))
                    .unwrap_or(false)
            })
    }

    /// Switches a step's edits on or off, as one undo step.
    pub(crate) fn switch_auto_step(&mut self, done: &StepDone, on: bool) {
        let commands: Vec<Command> = done
            .edits
            .iter()
            .filter(|(id, edit)| {
                self.document
                    .scene()
                    .object(*id)
                    .is_some_and(|o| o.stack.entry(*edit).is_some())
            })
            .map(|(id, edit)| Command::SetEditEnabled {
                id: *id,
                edit: *edit,
                enabled: on,
            })
            .collect();
        if commands.is_empty() {
            self.note = Some(crate::trf!(
                "{}: that step's edits are gone.",
                tr(done.step.label())
            ));
            return;
        }
        let label = if on {
            crate::trf!("Switch on {}", tr(done.step.label()))
        } else {
            crate::trf!("Switch off {}", tr(done.step.label()))
        };
        let key = self.new_step_key();
        match self.document.execute_all_labelled(commands, &label, key) {
            Ok(()) => self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label)),
            Err(why) => self.note = Some(crate::trf!("Couldn't do that: {}.", why)),
        }
    }
}
