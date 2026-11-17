//! Sculpt mode in the workbench ([decision 0304](../../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! its layout, the live sculpt of the selected shape, strokes turned into
//! dabs and, when they end, into one `Command::Sculpt` each, the brush's
//! keys, and the question about booleans, the live mirror and modifiers.
//!
//! While a stroke runs nothing in the document changes: the live sculpt
//! moves points and the drawing shows it, the renderer updating only the
//! points that moved. When the stroke ends, its sculpt goes to the
//! document as one undo step, and the scene takes the live shape as the
//! Sculpt layer's result, so nothing is worked out again. An undo or redo
//! brings the live sculpt to the document's, chunk by chunk.

use super::{LayoutChoice, Workbench};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::text::tr;
use crate::transforming::Tool;
use crate::viewport::BrushEvent;
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Vec2};
use scene::{Command, Mesh, ObjectId};
use sculpt::{BrushKind, Dab, LiveSculpt};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A shape with at least this many polygons is made ready to sculpt on
/// another thread, so entering Sculpt mode never freezes the window.
const READY_IN_BACKGROUND_FROM: usize = 200_000;
/// How long the dabs of one frame may take before the rest of the way
/// the mouse moved is covered by one last dab: a fast stroke with a big
/// brush on a heavy shape then stays smooth instead of falling behind.
const DABS_PER_FRAME: Duration = Duration::from_millis(14);
/// The most dabs one mouse move makes.
const MOST_DABS: usize = 200;

/// What to do with a shape's booleans, live mirror and modifiers while
/// sculpting it (choice 7 of the plan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Under {
    /// Sculpt the shape as its edits make it; they come back after.
    SculptUnder,
}

/// The live sculpt of one object.
struct Live {
    id: ObjectId,
    live: LiveSculpt,
    /// The document's sculpting it last followed (`None` before the first
    /// stroke makes a Sculpt layer).
    doc: Option<sculpt::Sculpt>,
}

/// A live sculpt being made ready on another thread.
struct Getting {
    id: ObjectId,
    input: Arc<Mesh>,
    doc: Option<sculpt::Sculpt>,
    job: JoinHandle<LiveSculpt>,
}

/// A stroke under way.
struct Stroke {
    kind: BrushKind,
    invert: bool,
    /// Where the last dab was, on screen.
    last: Vec2,
    camera: tools::Camera,
    size: Vec2,
    /// The shape's place in the world.
    model: Mat4,
    key: u64,
}

/// Sculpt mode's state.
#[derive(Default)]
pub(super) struct Sculpting {
    on: bool,
    layout_before: Option<LayoutChoice>,
    live: Option<Live>,
    getting: Option<Getting>,
    stroke: Option<Stroke>,
    /// The answers to the question about booleans, the mirror and
    /// modifiers, per object, while the project is open.
    answered: HashMap<ObjectId, Under>,
    /// The object the question is being asked about.
    asking: Option<ObjectId>,
    /// The keys held, as last seen.
    modifiers: Modifiers,
}

impl std::fmt::Debug for Sculpting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sculpting")
            .field("on", &self.on)
            .field("live", &self.live.as_ref().map(|l| l.id))
            .field("stroking", &self.stroke.is_some())
            .finish_non_exhaustive()
    }
}

impl Sculpting {
    /// The object sculpted now, if its live sculpt is ready.
    pub(super) fn sculpted(&self) -> Option<ObjectId> {
        self.live.as_ref().map(|l| l.id)
    }

    /// Whether a stroke is under way.
    #[cfg(test)]
    pub(super) fn stroking(&self) -> bool {
        self.stroke.is_some()
    }

    /// Whether a shape is being made ready to sculpt.
    #[cfg(test)]
    pub(super) fn getting_ready(&self) -> bool {
        self.getting.is_some()
    }
}

/// Whether object `id` has booleans, a live mirror or modifiers: what
/// comes after its edits, and so after a sculpt.
fn has_extras(document: &scene::Document, id: ObjectId) -> bool {
    document
        .scene()
        .object(id)
        .is_some_and(|o| !o.booleans.is_empty() || o.mirror.is_some() || !o.modifiers.is_empty())
}

impl Workbench {
    /// Sculpt mode each frame, before the scene is drawn: its layout on
    /// the way in and out, the shape to sculpt (the first selected that
    /// can be), its live sculpt made ready and kept in step with the
    /// document.
    pub(super) fn prepare_sculpt(&mut self, ctx: &egui::Context) {
        self.sculpting.modifiers = ctx.input(|i| i.modifiers);
        let on = self.mode == Mode::Sculpt;
        if on && !self.sculpting.on && self.layout_choice != LayoutChoice::Sculpt {
            self.sculpting.layout_before = Some(self.layout_choice);
            self.swap_layout(LayoutChoice::Sculpt);
        }
        if !on && self.sculpting.on {
            self.cancel_sculpt_stroke();
            self.sculpting.live = None;
            self.sculpting.getting = None;
            self.sculpting.asking = None;
            let before = self.sculpting.layout_before.take();
            if self.layout_choice == LayoutChoice::Sculpt {
                let back = before.unwrap_or(match self.mode {
                    Mode::Uv => LayoutChoice::UvEdit,
                    Mode::Paint => LayoutChoice::Paint,
                    _ => LayoutChoice::Standard,
                });
                self.swap_layout(back);
            }
            self.viewport.set_sculpting(None);
        }
        self.sculpting.on = on;
        if !on {
            self.drawing.sculpting = None;
            return;
        }
        self.keep_live_sculpt();
        self.drawing.sculpting = self.sculpting.sculpted();
        self.attributes.sculpt.status = self.sculpt_status();
        if self.sculpting.getting.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    /// The object Sculpt mode works on: the first selected that can be
    /// sculpted.
    fn sculpt_choice(&self) -> Option<ObjectId> {
        let scene = self.document.scene();
        self.document
            .selection()
            .into_iter()
            .find(|id| scene.sculpt_target(*id).is_some())
    }

    /// Makes the live sculpt ready for the chosen object and keeps it in
    /// step with the document (an undo, a redo, a change below).
    fn keep_live_sculpt(&mut self) {
        if self.sculpting.stroke.is_some() {
            return;
        }
        let Some(id) = self.sculpt_choice() else {
            self.sculpting.live = None;
            self.sculpting.getting = None;
            return;
        };
        // Booleans, the mirror or modifiers: asked once per object.
        if has_extras(&self.document, id) && !self.sculpting.answered.contains_key(&id) {
            self.sculpting.asking = Some(id);
            self.sculpting.live = None;
            return;
        }
        let Some(target) = self.document.scene().sculpt_target(id) else {
            return;
        };
        // One being made ready: taken when done, if still wanted.
        if let Some(getting) = &self.sculpting.getting {
            let wanted = getting.id == id && Arc::ptr_eq(&getting.input, &target.input);
            if !wanted {
                self.sculpting.getting = None;
            } else if getting.job.is_finished() {
                if let Some(getting) = self.sculpting.getting.take()
                    && let Ok(live) = getting.job.join()
                {
                    self.sculpting.live = Some(Live {
                        id,
                        live,
                        doc: getting.doc,
                    });
                }
            } else {
                return;
            }
        }
        let same = self
            .sculpting
            .live
            .as_ref()
            .is_some_and(|l| l.id == id && Arc::ptr_eq(l.live.input(), &target.input));
        if !same {
            self.sculpting.live = None;
            let heavy = target.input.face_count() >= READY_IN_BACKGROUND_FROM;
            if heavy {
                let (input, doc) = (Arc::clone(&target.input), target.sculpt.clone());
                let job = std::thread::spawn(move || LiveSculpt::new(input, doc.as_ref()));
                self.sculpting.getting = Some(Getting {
                    id,
                    input: target.input,
                    doc: target.sculpt,
                    job,
                });
            } else {
                let live = LiveSculpt::new(Arc::clone(&target.input), target.sculpt.as_ref());
                self.sculpting.live = Some(Live {
                    id,
                    live,
                    doc: target.sculpt,
                });
            }
            return;
        }
        // The document's sculpting changed (undo, redo): follow it.
        if let Some(live) = &mut self.sculpting.live
            && live.doc != target.sculpt
        {
            let doc = target
                .sculpt
                .clone()
                .unwrap_or_else(|| sculpt::Sculpt::new(&target.input));
            live.live.follow(&doc);
            live.doc = target.sculpt;
            if live.doc.is_some() {
                let shape = live.live.shape();
                self.document.scene().prime_sculpt(id, shape);
            }
        }
    }

    /// What the Sculpt tab says is sculpted.
    fn sculpt_status(&self) -> String {
        let name = |id: ObjectId| {
            self.document
                .scene()
                .object(id)
                .map_or_else(String::new, |o| o.name.clone())
        };
        if let Some(live) = &self.sculpting.live {
            let points = live.live.mesh().vertex_count();
            let mut text = crate::trf!("Sculpting {} ({} points).", name(live.id), points);
            if live.live.was_fitted() {
                text.push(' ');
                text.push_str(tr(
                    "The shape under the sculpt changed its points: the sculpt was fitted to it \
                     by position.",
                ));
            }
            return text;
        }
        if let Some(getting) = &self.sculpting.getting {
            return crate::trf!("Getting {} ready to sculpt…", name(getting.id));
        }
        if let Some(id) = self.sculpting.asking {
            return crate::trf!("{} has booleans, a mirror or modifiers: answer the question first.", name(id));
        }
        tr("Select a shape to sculpt it. Groups, instances and curves can't be sculpted.").to_string()
    }

    /// Shows the live sculpt's shape in the drawing, when it changed.
    pub(super) fn show_live_sculpt(&mut self) {
        let Some(live) = &mut self.sculpting.live else {
            return;
        };
        // The same shape as last frame costs nothing: it is shared.
        let shape = live.live.shape();
        self.drawing.show_live(live.id, &shape);
    }

    /// Sculpt mode's brush in the views, after the panels are laid out:
    /// its circle, the look, and the brush bar over the view.
    pub(super) fn update_sculpt_brush(&mut self) {
        if self.mode != Mode::Sculpt {
            return;
        }
        let settings = &self.attributes.sculpt;
        self.viewport
            .set_sculpting(Some((settings.look.matcap(), settings.cavity)));
        let kind = self.sculpt_brush_kind();
        let brush = settings.of(kind);
        // The tool keys (W, R, S, T) put the brush aside, as in Paint mode.
        let busy = self.tool != Tool::Select;
        self.viewport
            .set_paint_brush((!busy).then_some((brush.radius, 0.5)));
    }

    /// The brush a stroke would use now: Smooth while Shift is held.
    fn sculpt_brush_kind(&self) -> BrushKind {
        if self.sculpting.modifiers.shift {
            BrushKind::Smooth
        } else {
            self.attributes.sculpt.brush
        }
    }

    /// Carries out the brush's events from the views in Sculpt mode.
    pub(super) fn carry_out_sculpt_brush(&mut self, _drawing: &SceneDrawing) {
        for event in self.viewport.take_brush_events() {
            match event {
                BrushEvent::Begin(p) => self.begin_sculpt_stroke(&p, false),
                BrushEvent::Line(p) => self.begin_sculpt_stroke(&p, true),
                BrushEvent::Move(p) => self.continue_sculpt_stroke(p.at),
                BrushEvent::End => self.end_sculpt_stroke(),
                BrushEvent::Adjust {
                    size, hardness, ..
                } => {
                    self.attributes.sculpt.scale_size(size);
                    self.attributes.sculpt.add_strength(hardness);
                }
            }
        }
    }

    /// Starts a stroke where `p` points; `shift` when Shift was held (to
    /// smooth).
    fn begin_sculpt_stroke(&mut self, p: &crate::components::ViewPointer, shift: bool) {
        self.sculpting.stroke = None;
        if self.tool != Tool::Select {
            return;
        }
        let Some(live) = &self.sculpting.live else {
            self.note = Some(if self.sculpting.getting.is_some() {
                tr("The shape is still being made ready to sculpt: a moment.").to_string()
            } else {
                tr("Select a shape to sculpt it.").to_string()
            });
            return;
        };
        let model = self.document.scene().shape_matrix(live.id);
        let kind = if shift {
            BrushKind::Smooth
        } else {
            self.attributes.sculpt.brush
        };
        let key = self.new_step_key();
        self.sculpting.stroke = Some(Stroke {
            kind,
            invert: self.sculpting.modifiers.command,
            last: p.at,
            camera: p.camera,
            size: p.size,
            model,
            key,
        });
        if !self.sculpt_dab_at(p.at) {
            self.sculpting.stroke = None;
            self.note = Some(
                tr("Sculpt on the selected shape: press over it and drag. Pick another shape in \
                    the Objects panel to sculpt it.")
                .to_string(),
            );
        }
    }

    /// One dab where the screen point `at` meets the shape; whether it met
    /// it.
    fn sculpt_dab_at(&mut self, at: Vec2) -> bool {
        let (Some(live), Some(stroke)) = (&mut self.sculpting.live, &self.sculpting.stroke) else {
            return false;
        };
        let ray = stroke.camera.ray_through(at, stroke.size);
        let inverse = stroke.model.inverse();
        if !inverse.is_finite() {
            return false;
        }
        let origin = inverse.transform_point3(ray.origin);
        let way = inverse.transform_vector3(ray.direction);
        let Some(hit) = live.live.hit(origin, way) else {
            return false;
        };
        let world = stroke.model.transform_point3(hit.point);
        let per_pixel = stroke.camera.world_per_pixel(world, stroke.size).max(1e-6);
        let scale = stroke
            .model
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element()
            .max(1e-6);
        let settings = self.attributes.sculpt.of(stroke.kind);
        live.live.dab(&Dab {
            kind: stroke.kind,
            centre: hit.point,
            radius: settings.radius * per_pixel / scale,
            strength: settings.strength,
            falloff: settings.falloff,
            invert: stroke.invert,
        });
        true
    }

    /// Carries the stroke on to `at`, a dab every spacing along the way,
    /// within the frame's time for dabs.
    fn continue_sculpt_stroke(&mut self, at: Vec2) {
        let Some(stroke) = &self.sculpting.stroke else {
            return;
        };
        let settings = self.attributes.sculpt.of(stroke.kind);
        let step = (settings.radius * settings.spacing).max(1.0);
        let from = stroke.last;
        let distance = from.distance(at);
        if distance < step {
            return;
        }
        let count = ((distance / step).floor() as usize).clamp(1, MOST_DABS);
        let start = Instant::now();
        let mut reached = from;
        for i in 1..=count {
            // Out of time: the last dab covers the rest of the way.
            let i = if start.elapsed() > DABS_PER_FRAME {
                count
            } else {
                i
            };
            reached = from + (at - from) * (i as f32 * step / distance).min(1.0);
            self.sculpt_dab_at(reached);
            if i == count {
                break;
            }
        }
        if let Some(stroke) = &mut self.sculpting.stroke {
            stroke.last = reached;
        }
    }

    /// Ends the stroke: its sculpt goes to the document as one undo step.
    pub(super) fn end_sculpt_stroke(&mut self) {
        let Some(stroke) = self.sculpting.stroke.take() else {
            return;
        };
        let Some(live) = &mut self.sculpting.live else {
            return;
        };
        let Some(new) = live.live.finish() else {
            return;
        };
        let id = live.id;
        let command = Command::Sculpt {
            id,
            sculpt: new.clone(),
        };
        match self
            .document
            .execute_labelled(command, tr("Sculpt"), Some(stroke.key))
        {
            Ok(_) => {
                live.doc = Some(new);
                let shape = live.live.shape();
                self.document.scene().prime_sculpt(id, shape);
            }
            Err(why) => {
                live.live.follow(&live.doc.clone().unwrap_or_else(|| {
                    sculpt::Sculpt::new(live.live.input())
                }));
                self.note = Some(crate::trf!("Couldn't sculpt: {}.", why));
            }
        }
    }

    /// Takes back the stroke under way, if any (Esc, leaving the mode).
    pub(super) fn cancel_sculpt_stroke(&mut self) {
        if self.sculpting.stroke.take().is_some()
            && let Some(live) = &mut self.sculpting.live
        {
            live.live.cancel();
        }
    }

    /// Sculpt mode's keys (decision 0304): 1 Draw, [ and ] the size, Esc
    /// cancels a stroke or leaves the mode, and Tab stays in it. 2 to 9 are
    /// kept for the brushes to come, so they don't switch modes here.
    pub(super) fn sculpt_keys(&mut self, ui: &Ui) {
        if self.mode != Mode::Sculpt || ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |ui: &Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(ui, Key::Num1) {
            self.attributes.sculpt.brush = BrushKind::Draw;
            if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            }
        }
        for key in [
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
        ] {
            if pressed(ui, key) {
                self.note = Some(
                    tr("Keys 2 to 9 are kept for the sculpting brushes still to come.").to_string(),
                );
            }
        }
        for (key, factor) in [(Key::OpenBracket, 1.0 / 1.15), (Key::CloseBracket, 1.15)] {
            if pressed(ui, key) {
                self.attributes.sculpt.scale_size(factor);
            }
        }
        pressed(ui, Key::Tab);
        if !egui::Popup::is_any_open(ui.ctx())
            && self.sculpting.asking.is_none()
            && pressed(ui, Key::Escape)
        {
            if self.sculpting.stroke.is_some() {
                self.cancel_sculpt_stroke();
            } else {
                self.mode = Mode::Model;
            }
        }
    }

    /// The question about a shape's booleans, live mirror and modifiers,
    /// asked once per object on sculpting it (choice 7).
    pub(super) fn sculpt_question_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.sculpting.asking else {
            return;
        };
        if self.mode != Mode::Sculpt || self.sculpt_choice() != Some(id) {
            self.sculpting.asking = None;
            return;
        }
        let name = self
            .document
            .scene()
            .object(id)
            .map_or_else(String::new, |o| o.name.clone());
        let mut answer = None;
        crate::dialog::Dialog::new(tr("Sculpt a shape with booleans or modifiers?"), 480.0)
            .id("sculpt_question")
            .modal()
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(crate::trf!(
                        "{} has booleans, a live mirror or modifiers. They are worked out after \
                         its edits, so after anything you sculpt.",
                        name
                    ));
                    crate::dialog::help(
                        ui,
                        tr(
                            "Bake them in (recommended): they become part of the shape, as one \
                             undo step, and you sculpt exactly what you see. Sculpt under them: \
                             they're hidden while you sculpt and come back when you leave Sculpt \
                             mode.",
                        ),
                    );
                    crate::dialog::help(ui, tr("Esc: leave Sculpt mode"));
                });
                d.footer(None, |ui| {
                    if crate::dialog::main_button(ui, tr("Bake them in")).clicked() {
                        answer = Some(true);
                    }
                    if crate::dialog::plain_button(ui, tr("Sculpt under them")).clicked() {
                        answer = Some(false);
                    }
                });
            });
        if answer.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.sculpting.asking = None;
            self.mode = Mode::Model;
            return;
        }
        match answer {
            Some(true) => self.bake_in_for_sculpting(id),
            Some(false) => {
                self.sculpting.answered.insert(id, Under::SculptUnder);
                self.sculpting.asking = None;
            }
            None => {}
        }
    }

    /// Bakes object `id`'s live mirror, booleans and modifiers into its
    /// shape, as one undo step, for sculpting it as it looks.
    fn bake_in_for_sculpting(&mut self, id: ObjectId) {
        let key = self.new_step_key();
        let label = tr("Bake in for sculpting");
        let mirror = self
            .document
            .scene()
            .object(id)
            .is_some_and(|o| o.mirror.is_some());
        let mut result = Ok(());
        if mirror {
            result = self
                .document
                .execute_labelled(Command::ApplyMirror(vec![id]), label, Some(key))
                .map(|_| ());
        }
        if result.is_ok() {
            result = self
                .document
                .execute_labelled(Command::MakeEditable(vec![id]), label, Some(key))
                .map(|_| ());
        }
        self.sculpting.asking = None;
        match result {
            Ok(()) => {
                self.sculpting.answered.insert(id, Under::SculptUnder);
                self.note = Some(
                    tr("Baked in: the booleans, mirror and modifiers are part of the shape now. \
                        Ctrl + Z takes it back.")
                    .to_string(),
                );
            }
            Err(why) => {
                self.note = Some(crate::trf!("Couldn't bake them in: {}.", why));
                self.mode = Mode::Model;
            }
        }
    }
}
