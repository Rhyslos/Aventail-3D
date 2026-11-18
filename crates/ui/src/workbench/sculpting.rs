//! Sculpt mode in the workbench ([decision 0304](../../../../docs/decisions/0304-sculpt-mode-and-layers.md),
//! [decision 0306](../../../../docs/decisions/0306-sculpt-brushes-for-forms.md)):
//! its layout, the live sculpt of the selected shape, strokes turned into
//! dabs and, when they end, into one `Command::Sculpt` each, the brushes'
//! keys, and the question about booleans, the live mirror and modifiers.
//! The grabbing brushes take hold where a stroke starts and then follow
//! the mouse on a plane facing the view. Decision 0308 adds the pen's
//! pressure (size and strength by their curves), smoothing (the lazy
//! mouse, D), Line and Drag a stamp strokes (each move takes back what the
//! stroke did so far and lays it again, so the line or stamp shows as it
//! is dragged) and alphas laid on the dabs.
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
use crate::sculpt_tab::StrokeKind;
use crate::text::tr;
use crate::transforming::Tool;
use crate::viewport::BrushEvent;
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Vec2, Vec3};
use scene::{Command, Mesh, ObjectId};
use sculpt::{BrushKind, Dab, DabAlpha, LiveSculpt};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A shape with at least this many polygons is made ready to sculpt on
/// another thread, so entering Sculpt mode never freezes the window.
pub(super) const READY_IN_BACKGROUND_FROM: usize = 200_000;
/// How long the dabs of one frame may take before the rest of the way
/// the mouse moved is left undabbed: a fast stroke with a big brush on a
/// heavy shape then stays smooth instead of falling behind.
const DABS_PER_FRAME: Duration = Duration::from_millis(14);
/// The most dabs one mouse move makes.
const MOST_DABS: usize = 200;
/// The most dabs a Line stroke lays: a longer line spaces them further.
const MOST_LINE_DABS: usize = 2000;
/// How far a stamp must be dragged before it is laid, in points.
const LEAST_STAMP: f32 = 3.0;

/// What to do with a shape's booleans, live mirror and modifiers while
/// sculpting it (choice 7 of the plan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Under {
    /// Sculpt the shape as its edits make it; they come back after.
    SculptUnder,
}

/// The live sculpt of one object.
pub(super) struct Live {
    pub(super) id: ObjectId,
    pub(super) live: LiveSculpt,
    /// The document's sculpting it last followed (`None` before the first
    /// stroke makes a Sculpt layer).
    pub(super) doc: Option<sculpt::Sculpt>,
}

/// A live sculpt being made ready on another thread.
struct Getting {
    id: ObjectId,
    input: Arc<Mesh>,
    doc: Option<sculpt::Sculpt>,
    job: JoinHandle<LiveSculpt>,
}

/// A stroke under way.
pub(super) struct Stroke {
    kind: BrushKind,
    invert: bool,
    /// How it is laid down (always Freehand for the grabbing brushes).
    way: StrokeKind,
    /// Where it began, on screen.
    start: Vec2,
    /// Where the mouse is, on screen (Line and Drag a stamp).
    to: Vec2,
    /// Where the lazy brush is, on its string (Freehand with smoothing).
    lazy: Vec2,
    /// Whether the line was laid all the way to `to` (a frame's time can
    /// run out on a heavy shape; the rest is laid when it ends).
    complete: bool,
    /// The pen's pressure when it began (Line and Drag a stamp keep it).
    pressure: Option<f32>,
    /// Where a stamp goes, on the shape (in its own space).
    stamp_at: Option<Vec3>,
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
    pub(super) live: Option<Live>,
    getting: Option<Getting>,
    pub(super) stroke: Option<Stroke>,
    /// The answers to the question about booleans, the mirror and
    /// modifiers, per object, while the project is open.
    answered: HashMap<ObjectId, Under>,
    /// The object the question is being asked about.
    asking: Option<ObjectId>,
    /// The keys held, as last seen.
    pub(super) modifiers: Modifiers,
    /// A box or lasso mask being drawn (decision 0310).
    pub(super) outline: Option<super::sculpt_masking::Outline>,
    /// The gizmo moving the unmasked part (decision 0310).
    pub(super) moving: Option<super::sculpt_masking::MaskedMove>,
    /// The select arrow is on (Q, decision 0311): clicks select the shape
    /// to sculpt instead of sculpting.
    pub(super) selecting: bool,
    /// The brush as last seen, so a brush picked anywhere puts the select
    /// arrow down.
    brush_seen: Option<BrushKind>,
    /// Remesh for sculpting (decision 0312).
    pub(super) remeshing: super::sculpt_remeshing::Remeshing,
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
            self.sculpting.outline = None;
            self.sculpting.moving = None;
            self.viewport.set_mask_outline(None);
            self.viewport.set_sculpt_circles(Vec::new());
            self.sculpting.live = None;
            self.sculpting.getting = None;
            self.sculpting.asking = None;
            self.sculpting.selecting = false;
            self.sculpting.remeshing.stop();
            self.viewport.set_voxel_grid(None);
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
        let symmetry = self.attributes.sculpt.symmetry.symmetry();
        if let Some(live) = &mut self.sculpting.live {
            live.live.set_symmetry(symmetry);
        }
        self.attributes.sculpt.pen = self.pen().map(|p| p.pressure);
        if self.attributes.sculpt.own_alphas.is_none() {
            self.read_alpha_library();
        }
        self.drawing.sculpting = self.sculpting.sculpted();
        self.keep_remeshing(ctx);
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
            return crate::trf!(
                "{} has booleans, a mirror or modifiers: answer the question first.",
                name(id)
            );
        }
        tr("Select a shape to sculpt it. Groups, instances and curves can't be sculpted.")
            .to_string()
    }

    /// Shows the live sculpt's shape in the drawing, when it changed.
    pub(super) fn show_live_sculpt(&mut self) {
        let Some(live) = &mut self.sculpting.live else {
            return;
        };
        // The same shape and mask as last frame cost nothing: they are
        // shared.
        let shape = live.live.shape();
        let mask = live.live.mask_values();
        self.drawing.show_live(live.id, &shape, mask);
    }

    /// Sculpt mode's brush in the views, after the panels are laid out:
    /// its circle, the look, and the brush bar over the view.
    pub(super) fn update_sculpt_brush(&mut self) {
        // The brush bar floats over the view in Sculpt mode, so the Sculpt
        // tab leaves its numbers out (decision 0311).
        self.attributes.sculpt.bar_shown = self.mode == Mode::Sculpt;
        if self.mode != Mode::Sculpt {
            return;
        }
        // The look chosen in the tool row's Look list (decision 0311).
        if let Some((look, cavity)) = self.viewport.sculpt_look_chosen.take() {
            self.attributes.sculpt.look = look;
            self.attributes.sculpt.cavity = cavity;
        }
        // A brush picked anywhere (the shelf, the bar's list, E) puts the
        // select arrow down, as its key does.
        let brush = self.attributes.sculpt.brush;
        if self
            .sculpting
            .brush_seen
            .replace(brush)
            .is_some_and(|b| b != brush)
        {
            self.sculpting.selecting = false;
        }
        let settings = &self.attributes.sculpt;
        self.viewport
            .set_sculpting(Some((settings.look, settings.cavity)));
        let kind = self.sculpt_brush_kind();
        let brush = settings.of(kind);
        // The tool keys (W, R, S, T) and the select arrow (Q) put the
        // brush aside, as in Paint mode.
        let busy = self.tool != Tool::Select || self.sculpting.selecting;
        self.viewport
            .set_paint_brush((!busy).then_some((brush.radius, 0.5)));
        // On the surface, with its copies where symmetry sends them
        // (decision 0310); not for the box and lasso masks.
        let circles = if busy || self.masking_outline() {
            Vec::new()
        } else {
            self.sculpt_circles(brush.radius)
        };
        self.viewport.set_sculpt_circles(circles);
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
            // The box and lasso masks (decision 0310).
            if self.masking_outline() || self.sculpting.outline.is_some() {
                match event {
                    BrushEvent::Begin(p) | BrushEvent::Line(p) => self.begin_mask_outline(&p),
                    BrushEvent::Move(p) => self.continue_mask_outline(p.at),
                    BrushEvent::End => self.end_mask_outline(),
                    BrushEvent::Adjust { .. } => {}
                }
                continue;
            }
            match event {
                BrushEvent::Begin(p) => self.begin_sculpt_stroke(&p, false),
                BrushEvent::Line(p) => self.begin_sculpt_stroke(&p, true),
                BrushEvent::Move(p) => self.continue_sculpt_stroke(p.at),
                BrushEvent::End => self.end_sculpt_stroke(),
                BrushEvent::Adjust { size, hardness, .. } => {
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
        if self.sculpting.remeshing.busy() {
            self.note = Some(tr("Remeshing: a moment.").to_string());
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
        // The grabbing brushes always follow the mouse.
        let way = if kind.grabs() {
            StrokeKind::Freehand
        } else {
            self.attributes.sculpt.stroke
        };
        let key = self.new_step_key();
        self.sculpting.stroke = Some(Stroke {
            kind,
            invert: self.sculpting.modifiers.command,
            way,
            start: p.at,
            to: p.at,
            lazy: p.at,
            complete: true,
            pressure: self.pen().map(|pen| pen.pressure),
            stamp_at: None,
            last: p.at,
            camera: p.camera,
            size: p.size,
            model,
            key,
        });
        let met = match way {
            StrokeKind::Stamp => self.stamp_hit(p.at),
            StrokeKind::Freehand | StrokeKind::Line => self.sculpt_dab_at(p.at),
        };
        if !met {
            self.sculpting.stroke = None;
            self.note = Some(
                tr(
                    "Sculpt on the selected shape: press over it and drag. Pick another shape in \
                    the Objects panel to sculpt it.",
                )
                .to_string(),
            );
        }
    }

    /// Where the screen point `at` meets the live shape, in the object's
    /// own space.
    fn sculpt_hit(&self, at: Vec2) -> Option<Vec3> {
        let (live, stroke) = (
            self.sculpting.live.as_ref()?,
            self.sculpting.stroke.as_ref()?,
        );
        let ray = stroke.camera.ray_through(at, stroke.size);
        let inverse = stroke.model.inverse();
        if !inverse.is_finite() {
            return None;
        }
        let origin = inverse.transform_point3(ray.origin);
        let way = inverse.transform_vector3(ray.direction);
        live.live.hit(origin, way).map(|hit| hit.point)
    }

    /// Where a stamp goes: where the screen point `at` meets the shape.
    /// Whether it met it.
    fn stamp_hit(&mut self, at: Vec2) -> bool {
        let hit = self.sculpt_hit(at);
        if let Some(stroke) = &mut self.sculpting.stroke {
            stroke.stamp_at = hit;
        }
        hit.is_some()
    }

    /// The place under the screen point `at` on a plane through `through`
    /// (in the world) facing the view.
    fn on_view_plane(stroke: &Stroke, through: Vec3, at: Vec2) -> Option<Vec3> {
        let ray = stroke.camera.ray_through(at, stroke.size);
        let facing = stroke.camera.forward();
        let across = ray.direction.dot(facing);
        if across.abs() < 1e-9 {
            return None;
        }
        Some(ray.origin + ray.direction * ((through - ray.origin).dot(facing) / across))
    }

    /// The pen's pressure for the stroke's next dab: now for Freehand,
    /// as it was at the press for Line and Drag a stamp.
    fn stroke_pressure(&self) -> Option<f32> {
        match &self.sculpting.stroke {
            Some(stroke) if stroke.way != StrokeKind::Freehand => stroke.pressure,
            _ => self.pen().map(|pen| pen.pressure),
        }
    }

    /// The brush's reach in the object's own space at `world`, for a
    /// brush `radius` points across on screen.
    fn object_radius(stroke: &Stroke, radius: f32, world: glam::Vec3) -> f32 {
        let per_pixel = stroke.camera.world_per_pixel(world, stroke.size).max(1e-6);
        let scale = stroke
            .model
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element()
            .max(1e-6);
        radius * per_pixel / scale
    }

    /// One dab where the screen point `at` meets the shape; whether it met
    /// it. A grabbing brush takes hold there.
    fn sculpt_dab_at(&mut self, at: Vec2) -> bool {
        let Some(centre) = self.sculpt_hit(at) else {
            return false;
        };
        let pressure = self.stroke_pressure();
        let (Some(live), Some(stroke)) = (&mut self.sculpting.live, &self.sculpting.stroke) else {
            return false;
        };
        let world = stroke.model.transform_point3(centre);
        let sculpt = &self.attributes.sculpt;
        let settings = sculpt.pressed(stroke.kind, pressure);
        let dab = Dab {
            kind: stroke.kind,
            centre,
            radius: Self::object_radius(stroke, settings.radius, world),
            strength: settings.strength,
            falloff: settings.falloff,
            invert: stroke.invert,
        };
        // An alpha's top points up the view, unless it follows the stroke.
        let up = stroke.model.inverse().transform_vector3(stroke.camera.up());
        let alpha = sculpt.alpha.as_deref().map(|alpha| DabAlpha {
            alpha,
            up,
            angle: sculpt.alpha_turn(),
            follow: sculpt.alpha_follows,
        });
        live.live.dab_with(&dab, alpha.as_ref());
        true
    }

    /// One dab of a grabbing brush (Grab, Elastic grab, Snake hook) with
    /// the mouse at the screen point `at`: the place it holds goes to
    /// where the mouse is, on a plane through that place facing the view,
    /// so the lump follows the mouse in every view (decision 0039: drags
    /// follow the pull). Whether it could.
    fn sculpt_grab_to(&mut self, at: Vec2) -> bool {
        let pressure = self.stroke_pressure();
        let (Some(live), Some(stroke)) = (&mut self.sculpting.live, &self.sculpting.stroke) else {
            return false;
        };
        let Some(held) = live.live.held() else {
            return false;
        };
        let inverse = stroke.model.inverse();
        if !inverse.is_finite() {
            return false;
        }
        let anchor = stroke.model.transform_point3(held);
        let Some(target) = Self::on_view_plane(stroke, anchor, at) else {
            return false;
        };
        let settings = self.attributes.sculpt.pressed(stroke.kind, pressure);
        live.live.dab(&Dab {
            kind: stroke.kind,
            centre: inverse.transform_point3(target),
            radius: Self::object_radius(stroke, settings.radius, anchor),
            strength: settings.strength,
            falloff: settings.falloff,
            invert: stroke.invert,
        });
        true
    }

    /// Lays the Line stroke again from where it began to where the mouse
    /// is: what it laid so far is taken back first, so the line shows as
    /// it is dragged. Within `budget` while dragging (the rest is laid
    /// when it ends); all of it with none.
    fn lay_line(&mut self, budget: Option<Duration>) {
        let pressure = self.stroke_pressure();
        let Some(stroke) = &self.sculpting.stroke else {
            return;
        };
        let (from, to) = (stroke.start, stroke.to);
        let settings = self.attributes.sculpt.pressed(stroke.kind, pressure);
        let step = (settings.radius * settings.spacing).max(1.0);
        if let Some(live) = &mut self.sculpting.live {
            live.live.cancel();
        }
        let count = ((from.distance(to) / step).floor() as usize).min(MOST_LINE_DABS);
        let start = Instant::now();
        let mut complete = true;
        for i in 0..=count {
            let t = if count == 0 {
                0.0
            } else {
                i as f32 / count as f32
            };
            self.sculpt_dab_at(from + (to - from) * t);
            if i < count && budget.is_some_and(|b| start.elapsed() > b) {
                complete = false;
                break;
            }
        }
        if let Some(stroke) = &mut self.sculpting.stroke {
            stroke.complete = complete;
        }
    }

    /// Lays the stamp again where it was pressed, sized and turned by the
    /// drag to where the mouse is: as big as the drag is long on screen,
    /// its alpha's top pointing along the drag.
    fn lay_stamp(&mut self) {
        let pressure = self.stroke_pressure();
        let (Some(live), Some(stroke)) = (&mut self.sculpting.live, &self.sculpting.stroke) else {
            return;
        };
        let Some(centre) = stroke.stamp_at else {
            return;
        };
        live.live.cancel();
        let drag = stroke.to - stroke.start;
        if drag.length() < LEAST_STAMP {
            return;
        }
        let inverse = stroke.model.inverse();
        let world = stroke.model.transform_point3(centre);
        let Some(target) = Self::on_view_plane(stroke, world, stroke.to) else {
            return;
        };
        if !inverse.is_finite() {
            return;
        }
        let up = inverse.transform_vector3(target - world);
        let sculpt = &self.attributes.sculpt;
        let settings = sculpt.pressed(stroke.kind, pressure);
        let dab = Dab {
            kind: stroke.kind,
            centre,
            radius: Self::object_radius(stroke, drag.length(), world),
            strength: settings.strength,
            falloff: settings.falloff,
            invert: stroke.invert,
        };
        let alpha = sculpt.alpha.as_deref().map(|alpha| DabAlpha {
            alpha,
            up,
            angle: 0.0,
            follow: false,
        });
        live.live.dab_with(&dab, alpha.as_ref());
    }

    /// One step of the stroke at the screen point `at`.
    fn sculpt_step(&mut self, at: Vec2, grabs: bool) {
        if grabs {
            self.sculpt_grab_to(at);
        } else {
            self.sculpt_dab_at(at);
        }
    }

    /// Carries the stroke on to `at`, a dab every spacing along the way,
    /// within the frame's time for dabs. Grab moves its lump straight to
    /// the mouse: only where the mouse is now matters to it.
    fn continue_sculpt_stroke(&mut self, at: Vec2) {
        let reach = self.attributes.sculpt.smoothing_reach();
        let Some(stroke) = &mut self.sculpting.stroke else {
            return;
        };
        match stroke.way {
            StrokeKind::Line => {
                stroke.to = at;
                return self.lay_line(Some(DABS_PER_FRAME));
            }
            StrokeKind::Stamp => {
                stroke.to = at;
                return self.lay_stamp();
            }
            StrokeKind::Freehand => {}
        }
        let grabs = stroke.kind.grabs();
        if stroke.kind == BrushKind::Grab {
            self.sculpt_grab_to(at);
            if let Some(stroke) = &mut self.sculpting.stroke {
                stroke.last = at;
            }
            return;
        }
        // Smoothing: the brush follows the mouse on a string (not the
        // grabbing brushes, which hold the surface under the mouse).
        let at = if grabs {
            at
        } else {
            if !crate::pen::follow(&mut stroke.lazy, at, reach) {
                return;
            }
            stroke.lazy
        };
        let Some(stroke) = &self.sculpting.stroke else {
            return;
        };
        let settings = self
            .attributes
            .sculpt
            .pressed(stroke.kind, self.stroke_pressure());
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
            reached = from + (at - from) * (i as f32 * step / distance).min(1.0);
            self.sculpt_step(reached, grabs);
            // Out of time: the rest of the way is left undabbed, so a fast
            // stroke with a big brush never falls behind the mouse.
            if start.elapsed() > DABS_PER_FRAME {
                if grabs && reached != at {
                    // A grabbing brush still ends where the mouse is.
                    self.sculpt_step(at, grabs);
                }
                reached = at;
                break;
            }
        }
        if let Some(stroke) = &mut self.sculpting.stroke {
            stroke.last = reached;
        }
    }

    /// Ends the stroke: its sculpt goes to the document as one undo step.
    pub(super) fn end_sculpt_stroke(&mut self) {
        // A line the frame's time cut short is laid all the way now.
        if self
            .sculpting
            .stroke
            .as_ref()
            .is_some_and(|s| s.way == StrokeKind::Line && !s.complete)
        {
            self.lay_line(None);
        }
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
        let label = if stroke.kind == BrushKind::Mask {
            tr("Mask")
        } else {
            tr("Sculpt")
        };
        match self
            .document
            .execute_labelled(command, label, Some(stroke.key))
        {
            Ok(_) => {
                live.doc = Some(new);
                let shape = live.live.shape();
                self.document.scene().prime_sculpt(id, shape);
            }
            Err(why) => {
                live.live.follow(
                    &live
                        .doc
                        .clone()
                        .unwrap_or_else(|| sculpt::Sculpt::new(live.live.input())),
                );
                self.note = Some(crate::trf!("Couldn't sculpt: {}.", why));
            }
        }
    }

    /// Takes back the stroke under way, if any (Esc, leaving the mode),
    /// and a box or lasso mask being drawn.
    pub(super) fn cancel_sculpt_stroke(&mut self) {
        if self.sculpting.outline.take().is_some() {
            self.viewport.set_mask_outline(None);
        }
        if self.sculpting.stroke.take().is_some()
            && let Some(live) = &mut self.sculpting.live
        {
            live.live.cancel();
        }
    }

    /// Picks brush `kind` (its key, the shelf or the tool menu), putting
    /// the gizmos away.
    pub(super) fn choose_sculpt_brush(&mut self, kind: BrushKind) {
        self.attributes.sculpt.brush = kind;
        self.sculpting.selecting = false;
        if self.tool != Tool::Select {
            self.set_tool(Tool::Select);
        }
    }

    /// Sculpt mode's keys (decisions 0304, 0306, 0308, 0310 and 0312): 1 to
    /// 7 and 9 pick brushes, L symmetry, Ctrl + I and Alt + M the mask,
    /// Ctrl + R remeshes, D
    /// smoothing, [ and ] the size, Esc cancels a stroke or leaves the
    /// mode, and Tab stays in it. 8 is kept for the Trim brush to come, so
    /// it doesn't switch modes here.
    pub(super) fn sculpt_keys(&mut self, ui: &Ui) {
        if self.mode != Mode::Sculpt {
            return;
        }
        // A pen through the system's touch: its pressure this frame.
        if let Some(touch) = crate::pen::touch_pressure(ui.ctx()) {
            self.brushing.touch = touch;
        }
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |ui: &Ui, key: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if pressed(ui, Key::D) {
            let settings = &mut self.attributes.sculpt;
            settings.smoothing_on = !settings.smoothing_on;
            self.note = Some(if settings.smoothing_on {
                tr("Smoothing on: the brush follows the pointer on a string. D turns it off.")
                    .to_string()
            } else {
                tr("Smoothing off. D turns it on.").to_string()
            });
        }
        // Q (select, as everywhere) turns the select arrow on, as in Paint
        // mode (decision 0311); a brush's key puts it down again.
        if ui.input(|i| i.key_pressed(Key::Q) && i.modifiers.is_none()) {
            self.sculpting.selecting = true;
        }
        // Symmetry and the mask's keys first: 9 on the Mask brush goes
        // round its ways of masking (decision 0310).
        self.sculpt_mask_keys(ui);
        for kind in BrushKind::ALL {
            if let Some(key) = crate::sculpt_tab::brush_key(kind)
                && pressed(ui, key)
            {
                self.choose_sculpt_brush(kind);
                if kind == BrushKind::Mask {
                    self.attributes.sculpt.mask_tool = crate::sculpt_masks::MaskTool::Brush;
                    self.note = Some(
                        tr("Mask brush: paint what should stay still. Press 9 again for the box                             mask, then the lasso mask.")
                            .to_string(),
                    );
                }
            }
        }
        if pressed(ui, Key::Num8) {
            self.note = Some(tr("Key 8 is kept for the Trim brush still to come.").to_string());
        }
        for (key, factor) in [(Key::OpenBracket, 1.0 / 1.15), (Key::CloseBracket, 1.15)] {
            if pressed(ui, key) {
                self.attributes.sculpt.scale_size(factor);
            }
        }
        // Tab stays in Sculpt mode; egui would also move the keyboard
        // focus to the next button, which would then take every key.
        if pressed(ui, super::BACK_KEY) {
            ui.ctx()
                .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        if !egui::Popup::is_any_open(ui.ctx())
            && self.sculpting.asking.is_none()
            && !self.remesh_holds_keys()
            && pressed(ui, Key::Escape)
        {
            if self.sculpting.stroke.is_some() || self.sculpting.outline.is_some() {
                self.cancel_sculpt_stroke();
            } else {
                self.mode = Mode::Model;
            }
        }
    }

    /// Reads your own alphas from the alpha library (decision 0308), once.
    fn read_alpha_library(&mut self) {
        let (alphas, broken) = match &self.info.alpha_library {
            Some(folder) => fileio::alphas::library(folder),
            None => (Vec::new(), Vec::new()),
        };
        self.attributes.sculpt.own_alphas = Some(alphas.into_iter().map(Arc::new).collect());
        if let Some((name, why)) = broken.first() {
            self.note = Some(crate::trf!(
                "{} alphas in your alpha library couldn't be read: {}, because {}.",
                broken.len(),
                name,
                why
            ));
        }
    }

    /// Brings the picture at `path` in as an alpha of your own (decision
    /// 0308): kept in the alpha library for every project, then chosen.
    pub fn bring_in_alpha(&mut self, path: &std::path::Path) {
        let read = match fileio::alphas::open_alpha(path) {
            Ok(read) => read,
            Err(why) => return self.import_failed(path, &why),
        };
        let alpha = match &self.info.alpha_library {
            Some(folder) => match fileio::alphas::keep(folder, &read) {
                Ok(alpha) => alpha,
                Err(why) => return self.import_failed(path, &why),
            },
            // No folder to keep it in: it stays while the app is open.
            None => read.alpha,
        };
        let alpha = Arc::new(alpha);
        let settings = &mut self.attributes.sculpt;
        settings
            .own_alphas
            .get_or_insert_with(Vec::new)
            .push(Arc::clone(&alpha));
        settings.alpha = Some(Arc::clone(&alpha));
        settings.shelf_tab = crate::sculpt_tab::ShelfTab::Alphas;
        self.note = Some(crate::trf!(
            "Added the alpha {}: it is chosen, and kept on the shelf's Alphas tab for every project.",
            alpha.name()
        ));
    }

    /// Takes your own alpha `name` out of the alpha library.
    pub(super) fn forget_alpha(&mut self, name: &str) {
        let settings = &mut self.attributes.sculpt;
        if let Some(own) = &mut settings.own_alphas {
            own.retain(|a| a.name() != name);
        }
        if settings
            .alpha
            .as_ref()
            .is_some_and(|a| a.made().is_none() && a.name() == name)
        {
            settings.alpha = None;
        }
        let result = match &self.info.alpha_library {
            Some(folder) => fileio::alphas::forget(folder, name),
            None => Ok(()),
        };
        self.note = Some(match result {
            Ok(()) => crate::trf!("Took the alpha {} out of your alphas.", name),
            Err(why) => crate::trf!("Couldn't take the alpha {} out: {}.", name, why),
        });
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
                    tr(
                        "Baked in: the booleans, mirror and modifiers are part of the shape now. \
                        Ctrl + Z takes it back.",
                    )
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
