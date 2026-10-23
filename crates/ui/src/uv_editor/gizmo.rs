//! Changing UVs by hand in the UV Editor ([decision 0097](../../../../docs/decisions/0097-editing-uvs-by-hand.md)):
//! which UVs a change moves, recording it as an edit, and the flat gizmo
//! that moves, turns and scales them.
//!
//! The gizmo is the viewport's, laid flat: W shows the move arrows (red
//! across, blue up) and a square in the middle to move freely, R a ring
//! to turn, S boxes to stretch across or up and a square in the middle to
//! scale evenly, and T all of them. A drag is shown live and makes one
//! undo step; Shift makes it ten times finer, snapping (the palette
//! button, or Ctrl) steps it like the viewport's, and Esc or a right
//! click puts everything back.

use crate::palette::Mode;
use crate::text::tr;
use crate::theme::tint;
use crate::transforming::{SnapSettings, Tool};
use egui::{Color32, Painter, Pos2, Stroke, vec2};
use glam::{Affine2, Vec2};
use mesh::Anchors;
use scene::{Command, Document, ElementKind, FaceId, ObjectId, Operation, VertexId};
use std::collections::BTreeSet;

/// How far the move arrows reach from the middle, in screen points.
const ARROW: f32 = 80.0;
/// Where the scale boxes sit along their lines: at the arrows' ends with
/// the Scale tool, nearer the middle with all three gizmos.
const BOX_ALONE: f32 = 80.0;
const BOX_WITH_ALL: f32 = 52.0;
/// The turning ring's radius.
const RING: f32 = 64.0;
/// Half the middle square's side.
const MIDDLE: f32 = 7.0;
/// Half a scale box's side.
const BOX: f32 = 5.0;
/// How near the mouse must be to a handle, in screen points.
const GRAB: f32 = 7.0;
/// The arrowhead's length and half width.
const HEAD: f32 = 12.0;
const HEAD_HALF: f32 = 5.0;
/// Moves snap to steps of this part of the texture square (the grid
/// drawn on it).
pub(crate) const MOVE_STEP: f32 = 0.1;
/// The colour of the handle under the mouse or being dragged, and of
/// those that belong to neither direction (as in the viewport).
const HIGHLIGHT: Color32 = Color32::from_rgb(0xff, 0xd8, 0x5a);
const NEUTRAL: Color32 = Color32::from_rgb(0xd8, 0xd8, 0xd8);

/// A part of the gizmo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UvHandle {
    /// The arrow across (along U).
    MoveU,
    /// The arrow up (along V).
    MoveV,
    /// The middle square, with a move tool: moves freely.
    MoveFree,
    /// The ring.
    Turn,
    /// The box across.
    ScaleU,
    /// The box up.
    ScaleV,
    /// The middle square with the Scale tool: scales evenly.
    ScaleAll,
}

impl UvHandle {
    /// Its direction, for its colour: 0 across, 1 up.
    fn direction(self) -> Option<usize> {
        match self {
            Self::MoveU | Self::ScaleU => Some(0),
            Self::MoveV | Self::ScaleV => Some(1),
            Self::MoveFree | Self::Turn | Self::ScaleAll => None,
        }
    }

    /// What a drag of it is called, as an undo step.
    fn label(self) -> &'static str {
        match self {
            Self::MoveU | Self::MoveV | Self::MoveFree => tr("Move UVs"),
            Self::Turn => tr("Rotate UVs"),
            Self::ScaleU | Self::ScaleV | Self::ScaleAll => tr("Scale UVs"),
        }
    }
}

/// The handles `tool` shows, most wanted first (so the middle square
/// wins over the ring and arrows it overlaps).
fn handles(tool: Tool) -> &'static [UvHandle] {
    use UvHandle::{MoveFree, MoveU, MoveV, ScaleAll, ScaleU, ScaleV, Turn};
    match tool {
        Tool::Move => &[MoveFree, MoveU, MoveV],
        Tool::Rotate => &[Turn],
        Tool::Scale => &[ScaleAll, ScaleU, ScaleV],
        Tool::All => &[MoveFree, ScaleU, ScaleV, MoveU, MoveV, Turn],
        _ => &[],
    }
}

/// Whether `tool` has a gizmo in the UV Editor.
pub(crate) fn has_gizmo(tool: Tool) -> bool {
    !handles(tool).is_empty()
}

fn box_at(tool: Tool) -> f32 {
    if tool == Tool::All {
        BOX_WITH_ALL
    } else {
        BOX_ALONE
    }
}

/// The screen direction of `direction` (0 across, 1 up; the screen's y
/// grows down).
fn screen_way(direction: usize) -> egui::Vec2 {
    if direction == 0 {
        vec2(1.0, 0.0)
    } else {
        vec2(0.0, -1.0)
    }
}

fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The handle of `tool`'s gizmo at `centre` that `at` is on, if any.
pub(crate) fn hit(centre: Pos2, tool: Tool, at: Pos2) -> Option<UvHandle> {
    let reach = box_at(tool);
    handles(tool).iter().copied().find(|handle| match handle {
        UvHandle::MoveFree | UvHandle::ScaleAll => {
            (at.x - centre.x).abs() <= MIDDLE + 2.0 && (at.y - centre.y).abs() <= MIDDLE + 2.0
        }
        UvHandle::ScaleU | UvHandle::ScaleV => {
            let spot = centre + screen_way(handle.direction().unwrap_or(0)) * reach;
            (at.x - spot.x).abs() <= BOX + GRAB * 0.5 && (at.y - spot.y).abs() <= BOX + GRAB * 0.5
        }
        UvHandle::MoveU | UvHandle::MoveV => {
            let way = screen_way(handle.direction().unwrap_or(0));
            segment_distance(at, centre + way * (MIDDLE + 4.0), centre + way * ARROW) <= GRAB
        }
        UvHandle::Turn => (at.distance(centre) - RING).abs() <= GRAB,
    })
}

/// Draws `tool`'s gizmo at `centre`, `lit` brighter.
pub(crate) fn paint(painter: &Painter, centre: Pos2, tool: Tool, lit: Option<UvHandle>) {
    let reach = box_at(tool);
    for &handle in handles(tool) {
        let color = if lit == Some(handle) {
            HIGHLIGHT
        } else {
            match handle.direction() {
                Some(0) => tint::AXIS_X,
                Some(_) => tint::AXIS_Y,
                None => NEUTRAL,
            }
        };
        let stroke = Stroke::new(if lit == Some(handle) { 3.0 } else { 2.0 }, color);
        match handle {
            UvHandle::MoveFree | UvHandle::ScaleAll => {
                let square = egui::Rect::from_center_size(centre, egui::Vec2::splat(MIDDLE * 2.0));
                painter.rect_filled(square, 1.0, color.gamma_multiply(0.25));
                painter.rect_stroke(square, 1.0, stroke, egui::StrokeKind::Inside);
            }
            UvHandle::MoveU | UvHandle::MoveV => {
                let way = screen_way(handle.direction().unwrap_or(0));
                let tip = centre + way * ARROW;
                painter.line_segment([centre + way * (MIDDLE + 2.0), tip - way * HEAD], stroke);
                let side = vec2(-way.y, way.x);
                let base = tip - way * HEAD;
                painter.add(egui::Shape::convex_polygon(
                    vec![tip, base + side * HEAD_HALF, base - side * HEAD_HALF],
                    color,
                    Stroke::NONE,
                ));
            }
            UvHandle::ScaleU | UvHandle::ScaleV => {
                let way = screen_way(handle.direction().unwrap_or(0));
                let spot = centre + way * reach;
                if tool != Tool::All {
                    painter.line_segment([centre + way * (MIDDLE + 2.0), spot - way * BOX], stroke);
                }
                let square = egui::Rect::from_center_size(spot, egui::Vec2::splat(BOX * 2.0));
                painter.rect_filled(square, 0.0, color);
            }
            UvHandle::Turn => {
                painter.circle_stroke(centre, RING, stroke);
            }
        }
    }
}

/// The UVs one object's change moves: its selected polygons, or (in the
/// Points and Edges modes) points, and where they were.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UvTarget {
    pub(crate) id: ObjectId,
    pub(crate) faces: Vec<FaceId>,
    pub(crate) points: Vec<VertexId>,
    pub(crate) anchors: Option<Anchors>,
}

impl UvTarget {
    /// The edit moving these UVs by `transform`.
    pub(crate) fn operation(&self, transform: Affine2) -> Operation {
        Operation::TransformUvs {
            faces: self.faces.clone(),
            points: self.points.clone(),
            transform,
            anchors: self.anchors.clone(),
        }
    }
}

/// What a UV change in `mode` moves, object by object, and the box around
/// all of it (lowest and highest UV). Objects with nothing selected, or
/// without UVs, are left out. In the Model and Object modes nothing.
pub(crate) fn uv_targets(doc: &Document, mode: Mode) -> (Vec<UvTarget>, Option<(Vec2, Vec2)>) {
    let mut targets = Vec::new();
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    let Some(kind) = super::element_kind(mode) else {
        return (targets, None);
    };
    for id in super::shown_objects(doc) {
        let Some(evaluated) = doc.scene().evaluated(id) else {
            continue;
        };
        let mesh = &evaluated.mesh;
        if !mesh::uvs::has_uvs(mesh) {
            continue;
        }
        let Some(selected) = doc.elements(id) else {
            continue;
        };
        let selected = selected.valid_in(mesh, mesh.topology());
        let (faces, points, anchors) = match kind {
            ElementKind::Polygons => {
                let faces: Vec<FaceId> = selected.polygons().collect();
                let places: Vec<Vec<VertexId>> = faces
                    .iter()
                    .filter_map(|f| mesh.face(*f).map(<[VertexId]>::to_vec))
                    .collect();
                let anchors = Anchors::record(mesh, ElementKind::Polygons, &places);
                (faces, Vec::new(), anchors)
            }
            ElementKind::Points | ElementKind::Edges => {
                let mut points: BTreeSet<VertexId> = selected.points().collect();
                for edge in selected.edges() {
                    let (a, b) = edge.ends();
                    points.insert(a);
                    points.insert(b);
                }
                let points: Vec<VertexId> = points.into_iter().collect();
                let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
                let anchors = Anchors::record(mesh, ElementKind::Points, &places);
                (Vec::new(), points, anchors)
            }
        };
        if faces.is_empty() && points.is_empty() {
            continue;
        }
        if let Some((low, high)) = mesh::uvs::uv_bounds(mesh, &faces, &points) {
            min = min.min(low);
            max = max.max(high);
        }
        targets.push(UvTarget {
            id,
            faces,
            points,
            anchors,
        });
    }
    let bounds = min.is_finite().then_some((min, max));
    (targets, bounds)
}

/// `change` made about `pivot` instead of the origin.
pub(crate) fn about(pivot: Vec2, change: Affine2) -> Affine2 {
    Affine2::from_translation(pivot) * change * Affine2::from_translation(-pivot)
}

/// Records `transform` on every target as one undo step called `label`,
/// merged under `key` (so a drag can take it back and show the next).
pub(crate) fn record(
    doc: &mut Document,
    targets: &[UvTarget],
    transform: Affine2,
    label: &str,
    key: u64,
) -> Result<(), String> {
    for target in targets {
        doc.execute_labelled(
            Command::Record {
                id: target.id,
                operation: target.operation(transform),
            },
            label,
            Some(key),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Marks the UV gizmo's undo merge keys apart from every other.
pub(crate) const UV_GIZMO_KEY: u64 = 0x5556_475a_0000_0000;

/// A drag of the gizmo, from its start.
#[derive(Debug, Clone)]
pub(crate) struct GizmoDrag {
    pub(crate) handle: UvHandle,
    /// The middle the change is made about, in UVs.
    pub(crate) pivot: Vec2,
    /// What it moves.
    targets: Vec<UvTarget>,
    /// Where the mouse was last frame.
    last: Pos2,
    /// How far it has moved (UVs), turned (radians) and grown (a part of
    /// the size, per direction), Shift's fine steps included.
    shift: Vec2,
    angle: f32,
    grow: Vec2,
    /// Its undo merge key, and whether a step is shown under it.
    pub(crate) key: u64,
    pub(crate) shown: bool,
    /// The change last shown, to skip frames where nothing changed.
    last_shown: Option<Affine2>,
}

impl GizmoDrag {
    /// A drag of `handle`, beginning with the mouse at `at`.
    pub(crate) fn new(
        handle: UvHandle,
        pivot: Vec2,
        targets: Vec<UvTarget>,
        at: Pos2,
        key: u64,
    ) -> Self {
        Self {
            handle,
            pivot,
            targets,
            last: at,
            shift: Vec2::ZERO,
            angle: 0.0,
            grow: Vec2::ZERO,
            key,
            shown: false,
            last_shown: None,
        }
    }

    /// Follows the mouse to `now`. `centre` is the pivot on screen and
    /// `zoom` the screen points per texture width; `fine` is Shift.
    pub(crate) fn follow(&mut self, now: Pos2, centre: Pos2, zoom: f32, fine: bool) {
        let k = if fine { 0.1 } else { 1.0 };
        let d = now - self.last;
        let reach = BOX_ALONE;
        match self.handle {
            UvHandle::MoveU => self.shift.x += d.x / zoom * k,
            UvHandle::MoveV => self.shift.y -= d.y / zoom * k,
            UvHandle::MoveFree => self.shift += Vec2::new(d.x, -d.y) / zoom * k,
            UvHandle::Turn => {
                let a = (self.last - centre).angle();
                let b = (now - centre).angle();
                let mut turned = b - a;
                if turned > std::f32::consts::PI {
                    turned -= std::f32::consts::TAU;
                } else if turned < -std::f32::consts::PI {
                    turned += std::f32::consts::TAU;
                }
                // The screen's y grows down, the texture's up.
                if (self.last - centre).length() > 4.0 && (now - centre).length() > 4.0 {
                    self.angle -= turned * k;
                }
            }
            UvHandle::ScaleU => self.grow.x += d.x / reach * k,
            UvHandle::ScaleV => self.grow.y -= d.y / reach * k,
            UvHandle::ScaleAll => {
                let even = (d.x - d.y) / reach * k;
                self.grow += Vec2::splat(even);
            }
        }
        self.last = now;
    }

    /// The change so far, about the pivot, snapped as `snap` asks (if at
    /// all).
    pub(crate) fn transform(&self, snap: Option<&SnapSettings>) -> Affine2 {
        let step = |x: f32, by: f32| {
            if by > 0.0 { (x / by).round() * by } else { x }
        };
        match self.handle {
            UvHandle::MoveU | UvHandle::MoveV | UvHandle::MoveFree => {
                let mut by = self.shift;
                if snap.is_some() {
                    // The middle lands on the grid.
                    let to = self.pivot + by;
                    let snapped = Vec2::new(step(to.x, MOVE_STEP), step(to.y, MOVE_STEP));
                    by = snapped - self.pivot;
                    if self.handle == UvHandle::MoveU {
                        by.y = 0.0;
                    }
                    if self.handle == UvHandle::MoveV {
                        by.x = 0.0;
                    }
                }
                Affine2::from_translation(by)
            }
            UvHandle::Turn => {
                let angle = match snap {
                    Some(s) => step(self.angle, s.angle.to_radians()),
                    None => self.angle,
                };
                about(self.pivot, Affine2::from_angle(angle))
            }
            UvHandle::ScaleU | UvHandle::ScaleV | UvHandle::ScaleAll => {
                let grow = match snap {
                    Some(s) => Vec2::new(step(self.grow.x, s.scale), step(self.grow.y, s.scale)),
                    None => self.grow,
                };
                // Never squash to nothing: that can't be undone by
                // scaling back.
                let size = (Vec2::ONE + grow).map(|s| {
                    if s.abs() < 1e-3 {
                        1e-3_f32.copysign(s)
                    } else {
                        s
                    }
                });
                about(self.pivot, Affine2::from_scale(size))
            }
        }
    }

    /// Shows the change so far in the scene, in place of last frame's.
    /// Returns `false` (and stops showing) when something else was done
    /// meanwhile, or the change can't be made.
    pub(crate) fn show(&mut self, doc: &mut Document, snap: Option<&SnapSettings>) -> bool {
        let transform = self.transform(snap);
        if self.last_shown == Some(transform) {
            return true;
        }
        if self.shown {
            if !doc.abandon(self.key) {
                self.shown = false;
                return false;
            }
            self.shown = false;
        }
        self.last_shown = Some(transform);
        if transform == Affine2::IDENTITY {
            return true;
        }
        let ok = record(doc, &self.targets, transform, self.handle.label(), self.key).is_ok();
        self.shown = ok;
        ok
    }

    /// Puts everything back.
    pub(crate) fn cancel(&mut self, doc: &mut Document) {
        if self.shown {
            doc.abandon(self.key);
            self.shown = false;
        }
    }

    /// Where the gizmo's middle is now (it moves along with a move).
    pub(crate) fn pivot_now(&self, snap: Option<&SnapSettings>) -> Vec2 {
        match self.handle {
            UvHandle::MoveU | UvHandle::MoveV | UvHandle::MoveFree => {
                self.transform(snap).transform_point2(self.pivot)
            }
            _ => self.pivot,
        }
    }
}

/// A point for tests: where `handle` of `tool`'s gizmo at `centre` is.
#[cfg(test)]
pub(crate) fn handle_spot(centre: Pos2, tool: Tool, handle: UvHandle) -> Pos2 {
    match handle {
        UvHandle::MoveFree | UvHandle::ScaleAll => centre,
        UvHandle::MoveU => centre + vec2(ARROW * 0.85, 0.0),
        UvHandle::MoveV => centre + vec2(0.0, -ARROW * 0.85),
        UvHandle::ScaleU => centre + vec2(box_at(tool), 0.0),
        UvHandle::ScaleV => centre + vec2(0.0, -box_at(tool)),
        UvHandle::Turn => {
            centre
                + vec2(
                    RING * std::f32::consts::FRAC_1_SQRT_2,
                    RING * std::f32::consts::FRAC_1_SQRT_2,
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    const C: Pos2 = pos2(300.0, 300.0);

    fn drag(handle: UvHandle, by: egui::Vec2, fine: bool) -> GizmoDrag {
        let start = handle_spot(C, Tool::All, handle);
        let mut drag = GizmoDrag::new(handle, Vec2::new(0.23, 0.41), Vec::new(), start, 1);
        drag.follow(start + by, C, 100.0, fine);
        drag
    }

    #[test]
    fn each_handle_is_found_where_it_is_drawn() {
        for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::All] {
            for &handle in handles(tool) {
                assert_eq!(
                    hit(C, tool, handle_spot(C, tool, handle)),
                    Some(handle),
                    "{tool:?}"
                );
            }
            assert_eq!(hit(C, tool, C + vec2(200.0, 200.0)), None);
        }
        assert!(!has_gizmo(Tool::Select));
        assert!(!has_gizmo(Tool::Extrude));
        // The ring has no middle to grab.
        assert_eq!(hit(C, Tool::Rotate, C), None);
    }

    #[test]
    fn moves_follow_the_mouse_shift_slows_them_and_snapping_lands_on_the_grid() {
        let moved = drag(UvHandle::MoveU, vec2(50.0, 30.0), false).transform(None);
        assert!((moved.translation - Vec2::new(0.5, 0.0)).length() < 1e-5);
        let free = drag(UvHandle::MoveFree, vec2(50.0, 30.0), false).transform(None);
        assert!((free.translation - Vec2::new(0.5, -0.3)).length() < 1e-5);
        let fine = drag(UvHandle::MoveFree, vec2(50.0, 0.0), true).transform(None);
        assert!((fine.translation.x - 0.05).abs() < 1e-5);
        // 0.23 + 0.05 is 0.28: the middle snaps to 0.3.
        let snap = SnapSettings::default();
        let snapped = drag(UvHandle::MoveU, vec2(5.0, 0.0), false).transform(Some(&snap));
        assert!((snapped.translation - Vec2::new(0.07, 0.0)).length() < 1e-5);
    }

    #[test]
    fn turning_and_scaling_keep_the_middle_and_snap_in_steps() {
        let pivot = Vec2::new(0.23, 0.41);
        let mut turn = GizmoDrag::new(UvHandle::Turn, pivot, Vec::new(), C + vec2(64.0, 0.0), 1);
        // From the right to the top of the ring: anticlockwise.
        for i in 1..=10 {
            let a = -std::f32::consts::FRAC_PI_2 * i as f32 / 10.0;
            turn.follow(C + vec2(a.cos(), a.sin()) * 64.0, C, 100.0, false);
        }
        let t = turn.transform(None);
        assert!((t.transform_point2(pivot) - pivot).length() < 1e-5);
        let x = t.transform_vector2(Vec2::X);
        assert!((x - Vec2::Y).length() < 1e-4, "{x}");
        let snap = SnapSettings::default();
        let stepped = turn.transform(Some(&snap)).transform_vector2(Vec2::X);
        assert!((stepped - Vec2::Y).length() < 1e-4);

        let grown = drag(UvHandle::ScaleU, vec2(BOX_ALONE, 0.0), false).transform(None);
        assert!((grown.transform_vector2(Vec2::ONE) - Vec2::new(2.0, 1.0)).length() < 1e-4);
        assert!((grown.transform_point2(pivot) - pivot).length() < 1e-5);
        // Squashing flat stops short of nothing.
        let flat = drag(UvHandle::ScaleV, vec2(0.0, BOX_ALONE), false).transform(None);
        assert!(flat.matrix2.determinant().abs() > 0.0);
    }
}
