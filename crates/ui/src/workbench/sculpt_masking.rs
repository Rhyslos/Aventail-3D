//! Symmetry and masks in Sculpt mode ([decision 0310](../../../../docs/decisions/0310-sculpt-symmetry-and-masks.md)):
//! the keys (L, 9 again, Ctrl + I, Alt + M, Ctrl + wheel), the actions on a
//! whole mask (each one undo step, as a `Command::Sculpt` carrying the new
//! mask), the box and lasso masks, the gizmo moving the unmasked part,
//! Make symmetrical, Make a shape from the mask, and the brush's circle
//! drawn on the surface with its reflected copies.

use super::Workbench;
use crate::palette::Mode;
use crate::sculpt_masks::{MaskTool, SculptAction};
use crate::text::tr;
use crate::transforming::Tool;
use crate::viewport::{GizmoEvent, SurfaceCircle};
use egui::{Key, Modifiers, Ui};
use glam::{Mat4, Quat, Vec2, Vec3};
use scene::{Command, Mesh, ObjectId};
use sculpt::mask::{self, FULL};
use sculpt::{BrushKind, Mask};
use std::sync::Arc;
use tools::Change;

/// A box or lasso mask being drawn.
pub(super) struct Outline {
    /// The points drawn, in the view (for a box: where it began and the
    /// mouse).
    points: Vec<Vec2>,
    /// Box or lasso.
    tool: MaskTool,
    /// Unmask instead (Ctrl).
    unmask: bool,
    camera: tools::Camera,
    size: Vec2,
    model: Mat4,
}

/// The gizmo moving the unmasked part: where it began, in the world.
pub(super) struct MaskedMove {
    pivot: Vec3,
    axes: [Vec3; 3],
    model: Mat4,
    key: u64,
}

/// What a mask's value is from: the brush's or a whole-mask action.
impl Workbench {
    /// Sculpt mode's keys for symmetry and masks: L symmetry, 9 again
    /// for the box and lasso masks, Ctrl + I invert, Alt + M clear.
    /// Called before the brush keys, so 9 pressed on the Mask brush goes
    /// round its tools.
    pub(super) fn sculpt_mask_keys(&mut self, ui: &Ui) {
        let pressed = |m: Modifiers, key: Key| ui.input_mut(|i| i.consume_key(m, key));
        if pressed(Modifiers::NONE, Key::L) {
            self.run_sculpt_action(SculptAction::Symmetry);
        }
        if pressed(Modifiers::COMMAND, Key::I) {
            self.run_sculpt_action(SculptAction::InvertMask);
        }
        if pressed(Modifiers::ALT, Key::M) {
            self.run_sculpt_action(SculptAction::ClearMask);
        }
        if self.attributes.sculpt.brush == BrushKind::Mask && pressed(Modifiers::NONE, Key::Num9) {
            let settings = &mut self.attributes.sculpt;
            settings.mask_tool = settings.mask_tool.next();
            self.note = Some(crate::trf!(
                "{}: press 9 again for the next way of masking.",
                settings.mask_tool.name()
            ));
            if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            }
        }
    }

    /// The Sculpt menu (decision 0310): symmetry, then the mask.
    pub(super) fn sculpt_menu(&mut self, ui: &mut Ui) {
        for action in SculptAction::ALL {
            if matches!(action, SculptAction::InvertMask | SculptAction::ShapeFromMask) {
                ui.separator();
            }
            let mut button = egui::Button::new(tr(action.name()));
            if let Some(keys) = action.shortcut() {
                button = button.shortcut_text(ui.ctx().format_shortcut(&keys));
            }
            if ui
                .add(button)
                .on_hover_text(tr(action.explanation()))
                .clicked()
            {
                self.run_sculpt_action(action);
                ui.close();
            }
        }
    }

    /// Carries out an action asked for in the Sculpt tab, if any.
    pub(super) fn carry_out_sculpt_asks(&mut self) {
        if let Some(action) = self.attributes.sculpt.asked.take() {
            self.run_sculpt_action(action);
        }
    }

    /// Carries out `action` on the shape being sculpted.
    pub(crate) fn run_sculpt_action(&mut self, action: SculptAction) {
        if action == SculptAction::Symmetry {
            let symmetry = &mut self.attributes.sculpt.symmetry;
            symmetry.on = !symmetry.on;
            self.note = Some(if symmetry.on {
                tr("Symmetry on: every stroke is repeated on the other side. L turns it off.")
                    .to_string()
            } else {
                tr("Symmetry off. L turns it on.").to_string()
            });
            return;
        }
        if self.sculpting.stroke.is_some() {
            return;
        }
        // The ways up, down and towards the view, in the shape's own space.
        let ways = self.sculpting.live.as_ref().map(|l| {
            let towards = -self.viewport.camera().forward();
            [
                self.object_up(l.id, Vec3::Z),
                self.object_up(l.id, Vec3::NEG_Z),
                self.object_up(l.id, towards),
            ]
        });
        let Some(live) = self.sculpting.live.as_mut() else {
            self.note = Some(tr("Select a shape to sculpt it.").to_string());
            return;
        };
        if action.needs_mask() && !live.live.has_mask() {
            self.note = Some(
                tr("Nothing is masked yet: paint a mask with the Mask brush (9) first.")
                    .to_string(),
            );
            return;
        }
        let mesh = live.live.mesh();
        let now = live.live.mask();
        let new_mask = match action {
            SculptAction::InvertMask => Some(mask::invert(now)),
            SculptAction::ClearMask => Some(Mask::zero(now.len())),
            SculptAction::GrowMask => Some(mask::grow(mesh, now)),
            SculptAction::ShrinkMask => Some(mask::shrink(mesh, now)),
            SculptAction::SharpenMask => Some(mask::sharpen(now)),
            SculptAction::SoftenMask => Some(mask::soften(mesh, now)),
            SculptAction::MaskFromCavity => Some(mask::from_cavity(mesh)),
            SculptAction::MaskFromEdges => Some(mask::from_edges(mesh)),
            SculptAction::MaskFacingUp => ways.map(|w| mask::from_facing(mesh, w[0])),
            SculptAction::MaskFacingDown => ways.map(|w| mask::from_facing(mesh, w[1])),
            SculptAction::MaskFacingView => ways.map(|w| mask::from_facing(mesh, w[2])),
            _ => None,
        };
        if let Some(new_mask) = new_mask {
            let Some(live) = self.sculpting.live.as_mut() else {
                return;
            };
            match live.live.set_mask(new_mask) {
                Some(sculpt) => {
                    self.send_sculpt(sculpt, tr(action.name()));
                    self.note = Some(crate::trf!("{}.", tr(action.name())));
                }
                None => self.note = Some(tr("The mask is already like that.").to_string()),
            }
            return;
        }
        match action {
            SculptAction::MakeSymmetricalPlus | SculptAction::MakeSymmetricalMinus => {
                self.make_symmetrical(action == SculptAction::MakeSymmetricalPlus);
            }
            SculptAction::ShapeFromMask => self.shape_from_mask(),
            _ => {}
        }
    }

    /// The world direction `way` in object `id`'s shape space.
    fn object_up(&self, id: ObjectId, way: Vec3) -> Vec3 {
        let model = self.document.scene().shape_matrix(id);
        let inverse = model.inverse();
        if inverse.is_finite() {
            inverse.transform_vector3(way)
        } else {
            way
        }
    }

    /// Sends the live sculpt's new sculpt to the document as one undo step
    /// labelled `label`.
    fn send_sculpt(&mut self, sculpt: sculpt::Sculpt, label: &str) {
        let Some(live) = self.sculpting.live.as_mut() else {
            return;
        };
        let id = live.id;
        let key = self.new_step_key();
        let Some(live) = self.sculpting.live.as_mut() else {
            return;
        };
        match self.document.execute_labelled(
            Command::Sculpt {
                id,
                sculpt: sculpt.clone(),
            },
            label,
            Some(key),
        ) {
            Ok(_) => {
                live.doc = Some(sculpt);
                let shape = live.live.shape();
                self.document.scene().prime_sculpt(id, shape);
            }
            Err(why) => {
                let back = live
                    .doc
                    .clone()
                    .unwrap_or_else(|| sculpt::Sculpt::new(live.live.input()));
                live.live.follow(&back);
                self.note = Some(crate::trf!("Couldn't sculpt: {}.", why));
            }
        }
    }

    /// Sends `sculpt` as one undo step (for the heavy check).
    #[cfg(test)]
    pub(super) fn send_sculpt_for_tests(&mut self, sculpt: sculpt::Sculpt) {
        self.send_sculpt(sculpt, tr("Mask"));
    }

    /// Copies one side's sculpting onto the other across the first
    /// symmetry axis.
    fn make_symmetrical(&mut self, positive: bool) {
        let axis = self.attributes.sculpt.symmetry.first_axis();
        let Some(live) = self.sculpting.live.as_mut() else {
            return;
        };
        match live.live.make_symmetrical(axis, positive) {
            Ok(Some(sculpt)) => {
                self.send_sculpt(sculpt, tr("Make symmetrical"));
                self.note = Some(
                    tr("Made symmetrical: one side's sculpting is copied onto the other.")
                        .to_string(),
                );
            }
            Ok(None) => self.note = Some(tr("It is already symmetrical.").to_string()),
            Err(sculpt::symmetry::NotSymmetric::Unmatched(n)) => {
                self.note = Some(crate::trf!(
                    "Couldn't make it symmetrical: the shape under the sculpt isn't symmetrical \
                     itself ({} points have no partner on the other side). Strokes still mirror \
                     in space; Make symmetrical needs a shape built symmetrically.",
                    n
                ));
            }
            Err(sculpt::symmetry::NotSymmetric::Empty) => {
                self.note = Some(tr("The shape has no points to make symmetrical.").to_string());
            }
        }
    }

    /// The masked part becomes a new object with a thickness, as one
    /// undo step: the polygons whose corners are all at least half masked.
    fn shape_from_mask(&mut self) {
        let Some(live) = self.sculpting.live.as_ref() else {
            return;
        };
        let id = live.id;
        let part = masked_part(live.live.mesh(), live.live.mask());
        if part.face_count() == 0 {
            self.note = Some(
                tr("No polygon is masked all round: paint the mask over the part to make a shape of.")
                    .to_string(),
            );
            return;
        }
        let options = mesh::shell::ShellOptions {
            thickness: self.attributes.sculpt.shape_thickness,
            ..Default::default()
        };
        let shape = match mesh::shell::shell(&part, &options, scene::MAX_SHAPE_POLYGONS) {
            Ok(shape) => shape,
            Err(why) => {
                self.note = Some(crate::trf!("Couldn't make the shape: {}.", why));
                return;
            }
        };
        let scene = self.document.scene();
        let Some(source) = scene.object(id) else {
            return;
        };
        let name = crate::trf!("{} from mask", source.name);
        let mut object = scene::Object::new(scene::ObjectKind::Mesh(Arc::new(shape)));
        object.name = scene.unused_name(&name);
        object.offset = source.offset;
        object.materials = source.materials.clone();
        if let Some(world) = scene::Transform::from_matrix(scene.world_matrix(id)) {
            object.transform = world;
        }
        let command = Command::Insert {
            object,
            place: scene::Place {
                parent: None,
                index: 0,
            },
        };
        match self
            .document
            .execute_labelled(command, tr("Make a shape from the mask"), None)
        {
            Ok(()) => {
                self.note = Some(crate::trf!(
                    "Made {} from the mask. It is selected: pick the sculpt again in the Objects \
                     panel to carry on sculpting it.",
                    name
                ));
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't make the shape: {}.", why)),
        }
    }

    /// Whether Ctrl + wheel grows and shrinks the mask now.
    pub(super) fn sculpt_wheel_live(&self) -> bool {
        self.mode == Mode::Sculpt
            && self
                .sculpting
                .live
                .as_ref()
                .is_some_and(|l| l.live.has_mask())
    }

    /// Ctrl + wheel in Sculpt mode: grows (up) or shrinks (down) the mask
    /// a ring of points per notch. Whether it was Sculpt mode's.
    pub(super) fn sculpt_wheel(&mut self, steps: i32) -> bool {
        if self.mode != Mode::Sculpt {
            return false;
        }
        let action = if steps > 0 {
            SculptAction::GrowMask
        } else {
            SculptAction::ShrinkMask
        };
        for _ in 0..steps.unsigned_abs().min(20) {
            self.run_sculpt_action(action);
        }
        true
    }

    /// Whether the brush's events go to a box or lasso mask now.
    pub(super) fn masking_outline(&self) -> bool {
        self.attributes.sculpt.brush == BrushKind::Mask
            && self.attributes.sculpt.mask_tool != MaskTool::Brush
            && !self.sculpting.modifiers.shift
    }

    /// Starts a box or lasso mask at `p`.
    pub(super) fn begin_mask_outline(&mut self, p: &crate::components::ViewPointer) {
        let Some(live) = &self.sculpting.live else {
            self.note = Some(tr("Select a shape to sculpt it.").to_string());
            return;
        };
        let model = self.document.scene().shape_matrix(live.id);
        self.sculpting.outline = Some(Outline {
            points: vec![p.at, p.at],
            tool: self.attributes.sculpt.mask_tool,
            unmask: self.sculpting.modifiers.command,
            camera: p.camera,
            size: p.size,
            model,
        });
        self.show_mask_outline();
    }

    /// Carries a box or lasso mask on to `at`.
    pub(super) fn continue_mask_outline(&mut self, at: Vec2) {
        if let Some(outline) = &mut self.sculpting.outline {
            match outline.tool {
                MaskTool::Lasso => {
                    if outline.points.last().is_none_or(|l| l.distance(at) >= 2.0) {
                        outline.points.push(at);
                    }
                }
                _ => outline.points[1] = at,
            }
        }
        self.show_mask_outline();
    }

    fn show_mask_outline(&mut self) {
        let shown = self
            .sculpting
            .outline
            .as_ref()
            .map(|o| (o.points.clone(), o.tool == MaskTool::Box));
        self.viewport.set_mask_outline(shown);
    }

    /// Ends a box or lasso mask: every point inside it on screen is
    /// masked (unmasked with Ctrl), through the shape, as one undo step.
    pub(super) fn end_mask_outline(&mut self) {
        self.viewport.set_mask_outline(None);
        let Some(outline) = self.sculpting.outline.take() else {
            return;
        };
        let Some(live) = self.sculpting.live.as_mut() else {
            return;
        };
        let (camera, model, size) = (outline.camera, outline.model, outline.size);
        let (low, high) = (
            outline.points[0].min(outline.points[1]),
            outline.points[0].max(outline.points[1]),
        );
        let lasso = outline.points.clone();
        let inside = |p: Vec3| {
            let Some(at) = camera.project(model.transform_point3(p), size) else {
                return false;
            };
            match outline.tool {
                MaskTool::Box => at.cmpge(low).all() && at.cmple(high).all(),
                _ => in_lasso(&lasso, at),
            }
        };
        let big_enough = match outline.tool {
            MaskTool::Box => (high - low).min_element() >= 2.0,
            _ => lasso.len() >= 3,
        };
        if !big_enough {
            return;
        }
        let value = if outline.unmask { 0 } else { FULL };
        let new = mask::set_where(live.live.mesh(), live.live.mask(), value, inside);
        if let Some(sculpt) = live.live.set_mask(new) {
            let label = if outline.tool == MaskTool::Box {
                tr("Box mask")
            } else {
                tr("Lasso mask")
            };
            self.send_sculpt(sculpt, label);
        }
    }

    /// The gizmo in Sculpt mode with a mask (decision 0310): at the
    /// unmasked part's middle, to move, turn and scale it. `None` without
    /// a mask (the gizmo then moves the object, as elsewhere).
    pub(super) fn sculpt_gizmo(&mut self) -> Option<Option<tools::Gizmo>> {
        if self.mode != Mode::Sculpt {
            return None;
        }
        let tool = self.tool.gizmo()?;
        let live = self.sculpting.live.as_mut()?;
        if !live.live.has_mask() {
            return None;
        }
        if let Some(moving) = &self.sculpting.moving {
            return Some(Some(tools::Gizmo {
                origin: moving.pivot,
                axes: moving.axes,
                tool,
            }));
        }
        let model = self.document.scene().shape_matrix(live.id);
        let middle = live.live.unmasked_middle()?;
        Some(Some(tools::Gizmo {
            origin: model.transform_point3(middle),
            axes: object_axes(model),
            tool,
        }))
    }

    /// Carries out the gizmo's events in Sculpt mode with a mask. Whether
    /// they were Sculpt mode's.
    pub(super) fn sculpt_gizmo_events(&mut self, events: &[GizmoEvent]) -> bool {
        if self.mode != Mode::Sculpt
            || (self.sculpting.moving.is_none()
                && !self
                    .sculpting
                    .live
                    .as_ref()
                    .is_some_and(|l| l.live.has_mask()))
        {
            return false;
        }
        for event in events {
            match event {
                GizmoEvent::Begin { .. } => {
                    let Some(Some(gizmo)) = self.sculpt_gizmo() else {
                        continue;
                    };
                    let key = self.new_step_key();
                    let Some(live) = self.sculpting.live.as_mut() else {
                        continue;
                    };
                    let model = self.document.scene().shape_matrix(live.id);
                    live.live.begin_move();
                    self.sculpting.moving = Some(MaskedMove {
                        pivot: gizmo.origin,
                        axes: gizmo.axes,
                        model,
                        key,
                    });
                }
                GizmoEvent::Update { change, .. } => {
                    let (Some(moving), Some(live)) =
                        (&self.sculpting.moving, self.sculpting.live.as_mut())
                    else {
                        continue;
                    };
                    let world = change_matrix(change, moving.pivot, moving.axes);
                    let inverse = moving.model.inverse();
                    if inverse.is_finite() {
                        live.live.move_unmasked(inverse * world * moving.model);
                    }
                }
                GizmoEvent::End => {
                    let Some(moving) = self.sculpting.moving.take() else {
                        continue;
                    };
                    let Some(live) = self.sculpting.live.as_mut() else {
                        continue;
                    };
                    if let Some(sculpt) = live.live.finish() {
                        let id = live.id;
                        match self.document.execute_labelled(
                            Command::Sculpt {
                                id,
                                sculpt: sculpt.clone(),
                            },
                            tr("Move the unmasked part"),
                            Some(moving.key),
                        ) {
                            Ok(_) => {
                                live.doc = Some(sculpt);
                                let shape = live.live.shape();
                                self.document.scene().prime_sculpt(id, shape);
                            }
                            Err(why) => {
                                let back = live
                                    .doc
                                    .clone()
                                    .unwrap_or_else(|| sculpt::Sculpt::new(live.live.input()));
                                live.live.follow(&back);
                                self.note = Some(crate::trf!("Couldn't sculpt: {}.", why));
                            }
                        }
                    }
                }
                GizmoEvent::Cancel => {
                    self.sculpting.moving = None;
                    if let Some(live) = self.sculpting.live.as_mut() {
                        live.live.cancel();
                    }
                    self.note = Some(tr("Cancelled: everything is back where it was.").into());
                }
            }
        }
        true
    }

    /// The brush's circles on the surface under the mouse (decision 0310):
    /// the brush's own, turned to lie along the surface, with an inner
    /// ring for its falloff, and a dashed one where each copy of the dab
    /// goes with symmetry. Empty when the mouse isn't over the shape (the
    /// circle is then drawn flat on the view).
    pub(super) fn sculpt_circles(&self, radius: f32) -> Vec<SurfaceCircle> {
        let (Some(live), Some(pointer)) = (&self.sculpting.live, self.viewport.pointer()) else {
            return Vec::new();
        };
        let model = self.document.scene().shape_matrix(live.id);
        let inverse = model.inverse();
        if !inverse.is_finite() {
            return Vec::new();
        }
        let ray = pointer.camera.ray_through(pointer.at, pointer.size);
        let Some(hit) = live.live.hit(
            inverse.transform_point3(ray.origin),
            inverse.transform_vector3(ray.direction),
        ) else {
            return Vec::new();
        };
        let world = model.transform_point3(hit.point);
        let per_pixel = pointer.camera.world_per_pixel(world, pointer.size).max(1e-6);
        let reach = radius * per_pixel;
        let scale = model
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element()
            .max(1e-6);
        let object_reach = reach / scale;
        let falloff = self.attributes.sculpt.of(self.attributes.sculpt.brush).falloff;
        // Where the falloff has dropped to half.
        let inner = (0..100)
            .map(|i| i as f32 / 100.0)
            .find(|x| falloff.at(*x) < 0.5)
            .unwrap_or(1.0);
        live.live
            .copies()
            .iter()
            .enumerate()
            .map(|(i, copy)| {
                let centre = copy.turn * hit.point;
                let normal = copy.turn * hit.normal;
                let world_normal = inverse.transpose().transform_vector3(normal).normalize_or(Vec3::Z);
                SurfaceCircle {
                    centre: model.transform_point3(centre),
                    normal: world_normal,
                    radius: object_reach * scale,
                    inner,
                    dashed: i > 0,
                }
            })
            .collect()
    }
}

/// The axes of a shape placed by `model`, unit length (the world's when
/// it is squashed flat).
fn object_axes(model: Mat4) -> [Vec3; 3] {
    let (_, turn, _) = model.to_scale_rotation_translation();
    if turn.is_finite() {
        [turn * Vec3::X, turn * Vec3::Y, turn * Vec3::Z]
    } else {
        [Vec3::X, Vec3::Y, Vec3::Z]
    }
}

/// A gizmo's change as a matrix in the world, round `pivot`, scaling
/// along `axes`.
fn change_matrix(change: &Change, pivot: Vec3, axes: [Vec3; 3]) -> Mat4 {
    let to = Mat4::from_translation(pivot);
    let from = Mat4::from_translation(-pivot);
    match *change {
        Change::Translate(d) => Mat4::from_translation(d),
        Change::Rotate { axis, angle } => to * Mat4::from_quat(Quat::from_axis_angle(axis, angle)) * from,
        Change::Scale(f) => {
            let basis = Mat4::from_cols(
                axes[0].extend(0.0),
                axes[1].extend(0.0),
                axes[2].extend(0.0),
                glam::Vec4::W,
            );
            to * basis * Mat4::from_scale(f) * basis.transpose() * from
        }
    }
}

/// Whether `p` is inside the lasso through `points` (closed back to the
/// first), by counting crossings.
fn in_lasso(points: &[Vec2], p: Vec2) -> bool {
    let mut inside = false;
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The polygons of `mesh` whose corners are all at least half masked, as
/// a shape of their own.
pub(super) fn masked_part(mesh: &Mesh, mask: &Mask) -> Mesh {
    let mut out = Mesh::new();
    let mut new_id = vec![None; mesh.point_slot_count()];
    for (_, corners) in mesh.faces() {
        if !corners
            .iter()
            .all(|v| mask.get(v.get() as usize) > FULL / 2)
        {
            continue;
        }
        let mut ids = Vec::with_capacity(corners.len());
        for v in corners {
            let slot = v.get() as usize;
            let id = match new_id[slot] {
                Some(id) => id,
                None => {
                    let Some(p) = mesh.position(*v) else {
                        break;
                    };
                    let Ok(id) = out.add_vertex(p) else {
                        break;
                    };
                    new_id[slot] = Some(id);
                    id
                }
            };
            ids.push(id);
        }
        if ids.len() == corners.len() {
            let _ = out.add_face(&ids);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lasso_holds_what_it_goes_round() {
        let square = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(in_lasso(&square, Vec2::new(5.0, 5.0)));
        assert!(!in_lasso(&square, Vec2::new(15.0, 5.0)));
        assert!(!in_lasso(&square[..2], Vec2::new(5.0, 0.0)));
    }

    #[test]
    fn a_scale_changes_only_along_the_gizmos_axes_round_its_pivot() {
        let pivot = Vec3::new(1.0, 2.0, 3.0);
        let m = change_matrix(&Change::Scale(Vec3::new(2.0, 1.0, 1.0)), pivot, [Vec3::X, Vec3::Y, Vec3::Z]);
        assert!(m.transform_point3(pivot).distance(pivot) < 1e-6);
        assert!(m.transform_point3(pivot + Vec3::X).distance(pivot + 2.0 * Vec3::X) < 1e-6);
        let t = change_matrix(&Change::Translate(Vec3::Z), pivot, [Vec3::X, Vec3::Y, Vec3::Z]);
        assert_eq!(t.transform_point3(Vec3::ZERO), Vec3::Z);
    }

    #[test]
    fn the_masked_part_is_the_polygons_masked_all_round() {
        let cube = mesh::PrimitiveKind::Cube.standard().mesh();
        assert_eq!(masked_part(&cube, &Mask::zero(cube.point_slot_count())).face_count(), 0);
        let all = Mask::from_values(&vec![FULL; cube.point_slot_count()]);
        assert_eq!(masked_part(&cube, &all).face_count(), 6);
        // The top's four corners only: the top polygon.
        let top = cube.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
        let values: Vec<u8> = cube
            .point_slots()
            .map(|p| if p.is_some_and(|p| p.z >= top - 1e-4) { FULL } else { 0 })
            .collect();
        let part = masked_part(&cube, &Mask::from_values(&values));
        assert_eq!(part.face_count(), 1);
        assert_eq!(part.vertex_count(), 4);
    }
}
