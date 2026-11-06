//! The baker's cage on the model ([decision 0251](../../../../docs/decisions/0251-high-to-low.md)):
//! shown as see-through lines while the Bake window is open, and painted
//! further out (or in) with a brush, as the seam brush paints seams:
//! drag over points to push the cage out there, Ctrl + drag to pull it in,
//! `[` and `]` for the size, Esc to put the brush down. Each stroke is one
//! undo step (`Operation::SetCagePush`).

use super::Workbench;
use super::joining::counted;
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::selecting::{Gesture, SelectTool};
use crate::text::tr;
use egui::{Key, Modifiers, Ui};
use scene::{Element, ElementKind, ObjectId, Operation, VertexId};
use std::collections::BTreeMap;

/// The brush's state while in use.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CageBrush {
    /// The select tool to go back to.
    before: SelectTool,
    /// Ctrl was held: strokes pull the cage in.
    ctrl: bool,
    stroke: Option<Stroke>,
}

/// One stroke: its gesture's number, undo key, and each object's points
/// touched with the push they had before it.
#[derive(Debug, Clone, PartialEq)]
struct Stroke {
    number: Option<u64>,
    key: u64,
    inward: bool,
    points: BTreeMap<ObjectId, BTreeMap<VertexId, f32>>,
}

/// The size of the shape's box (its diagonal, in its own units).
pub(crate) fn shape_size(mesh: &scene::Mesh) -> f32 {
    let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
    for (_, p) in mesh.vertices() {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    if lo.x > hi.x { 0.0 } else { (hi - lo).length() }
}

/// The cage of `mesh`, pushed out `share` of its size plus each point's
/// own push, along its points' averaged facing: each edge's two ends.
pub(crate) fn cage_lines(mesh: &scene::Mesh, share: f32) -> Vec<[glam::Vec3; 2]> {
    let push = shape_size(mesh) * share;
    let mut normals: BTreeMap<VertexId, glam::Vec3> = BTreeMap::new();
    for (f, corners) in mesh.faces() {
        let n = mesh.face_normal(f);
        for v in corners {
            *normals.entry(*v).or_default() += n;
        }
    }
    let out = |v: VertexId| {
        let p = mesh.position(v)?;
        let n = normals.get(&v).copied().unwrap_or_default().normalize_or_zero();
        Some(p + n * (push + mesh::cage::push_of(mesh, v)).max(0.0))
    };
    mesh.topology()
        .edges()
        .filter_map(|e| {
            let (a, b) = e.ends();
            Some([out(a)?, out(b)?])
        })
        .collect()
}

impl Workbench {
    /// Whether the cage brush is in use.
    pub(crate) fn cage_brush_on(&self) -> bool {
        self.cage_brush.is_some()
    }

    /// Picks up the cage brush: the Points mode, and the selection
    /// brush's circle.
    pub(crate) fn start_cage_brush(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some(tr("Paint the cage: select the low-detail object first.").to_string());
            return;
        }
        if self.cage_brush.is_none() {
            self.cage_brush = Some(CageBrush {
                before: self.viewport.select_tool(),
                ctrl: false,
                stroke: None,
            });
        }
        self.baker.show_cage = true;
        self.mode = Mode::Points;
        self.tool = crate::transforming::Tool::Select;
        self.viewport.set_select_tool(SelectTool::Brush);
        self.note = Some(tr(
            "Cage brush: drag over points to push the cage further out; Ctrl + drag pulls it in. \
             [ and ] change the size; Esc goes back to selecting.",
        )
        .into());
    }

    /// Puts the cage brush down, finishing any stroke.
    pub(crate) fn stop_cage_brush(&mut self) {
        if let Some(brush) = self.cage_brush.take() {
            self.viewport.set_select_tool(brush.before);
            self.note = Some(tr("Cage brush put down: dragging selects again.").into());
        }
    }

    /// Each frame while the brush is in use: Esc puts it down, `[` and `]`
    /// change its size, Ctrl is noted, a stroke ends when the button is
    /// let go. Leaving the Points mode or the brush tool puts it down.
    pub(super) fn cage_brush_keys(&mut self, ui: &Ui) {
        let Some(brush) = &mut self.cage_brush else {
            return;
        };
        brush.ctrl = ui.input(|i| i.modifiers.command);
        if !ui.input(|i| i.pointer.primary_down()) {
            brush.stroke = None;
        }
        let left = self.mode != Mode::Points
            || self.viewport.select_tool() != SelectTool::Brush
            || self.tool != crate::transforming::Tool::Select;
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        if left || (!typing && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))) {
            self.stop_cage_brush();
            return;
        }
        if typing {
            return;
        }
        let (smaller, bigger) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::OpenBracket),
                i.consume_key(Modifiers::NONE, Key::CloseBracket),
            )
        });
        if smaller {
            self.viewport.scale_brush_radius(-1.0);
        }
        if bigger {
            self.viewport.scale_brush_radius(1.0);
        }
    }

    /// While the brush is in use, takes this frame's brush dabs and clicks
    /// and paints the cage at the points under them; the rest are given
    /// back.
    pub(super) fn take_cage_brush(&mut self, gestures: Vec<Gesture>, drawing: &SceneDrawing) -> Vec<Gesture> {
        if self.cage_brush.is_none() {
            return gestures;
        }
        let mut rest = Vec::new();
        for gesture in gestures {
            let (number, hits) = match gesture {
                Gesture::Region {
                    camera,
                    size,
                    regions,
                    stroke,
                    ..
                } => {
                    let scene = self.components.pick_scene(drawing);
                    let hits: Vec<_> = regions
                        .iter()
                        .flat_map(|r| scene.elements_in(ElementKind::Points, &camera, size, r, true))
                        .collect();
                    (stroke, hits)
                }
                Gesture::Click { pointer, .. } => {
                    let scene = self.components.pick_scene(drawing);
                    let hit = scene.under(ElementKind::Points, &pointer.camera, pointer.size, pointer.at);
                    (None, hit.into_iter().collect())
                }
                other => {
                    rest.push(other);
                    continue;
                }
            };
            let found: Vec<(ObjectId, VertexId)> = hits
                .into_iter()
                .filter_map(|hit| {
                    let id = drawing.items.get(hit.object)?.id;
                    match hit.element {
                        Element::Point(v) => Some((id, v)),
                        _ => None,
                    }
                })
                .collect();
            self.cage_dab(number, found);
            if number.is_none()
                && let Some(brush) = &mut self.cage_brush
            {
                brush.stroke = None;
            }
        }
        rest
    }

    /// Adds `found` to the stroke `number` (starting it if new) and
    /// records the stroke so far: each touched point's push a step past
    /// what it was before the stroke, out (or in, with Ctrl).
    pub(super) fn cage_dab(&mut self, number: Option<u64>, found: Vec<(ObjectId, VertexId)>) {
        let same = self
            .cage_brush
            .as_ref()
            .and_then(|b| b.stroke.as_ref())
            .is_some_and(|s| s.number.is_some() && s.number == number);
        if !same {
            let key = self.new_step_key();
            let Some(brush) = &mut self.cage_brush else {
                return;
            };
            brush.stroke = Some(Stroke {
                number,
                key,
                inward: brush.ctrl,
                points: BTreeMap::new(),
            });
        }
        let mut grew = false;
        for (id, v) in found {
            let before = self
                .document
                .scene()
                .evaluated(id)
                .map_or(0.0, |e| mesh::cage::push_of(&e.mesh, v));
            let Some(stroke) = self.cage_brush.as_mut().and_then(|b| b.stroke.as_mut()) else {
                return;
            };
            let points = stroke.points.entry(id).or_default();
            if let std::collections::btree_map::Entry::Vacant(e) = points.entry(v) {
                e.insert(before);
                grew = true;
            }
        }
        let Some(stroke) = self.cage_brush.as_ref().and_then(|b| b.stroke.clone()) else {
            return;
        };
        if !grew {
            return;
        }
        self.document.abandon(stroke.key);
        let share = self.baker.cage;
        let mut count = 0;
        let ops: Vec<(ObjectId, Operation)> = stroke
            .points
            .iter()
            .filter_map(|(id, points)| {
                let mesh = self.document.scene().evaluated(*id)?.mesh;
                // A step is half the cage's own push.
                let step = shape_size(&mesh) * share * 0.5;
                let step = if stroke.inward { -step } else { step };
                count += points.len();
                Some((
                    *id,
                    Operation::SetCagePush {
                        points: points.iter().map(|(v, before)| (*v, before + step)).collect(),
                    },
                ))
            })
            .collect();
        let verb = if stroke.inward {
            tr("Pull the cage in at")
        } else {
            tr("Push the cage out at")
        };
        let label = counted(verb, count, tr("point"));
        if self.record_each_in(&label, ops, stroke.key) {
            self.note = Some(crate::trf!("{}.", label));
        }
    }

    /// Clears the selected objects' painted cage, one undo step.
    pub(crate) fn clear_cage(&mut self) {
        let ops: Vec<(ObjectId, Operation)> = self
            .own_shapes()
            .into_iter()
            .filter_map(|(id, mesh)| {
                let points: Vec<(VertexId, f32)> =
                    mesh::cage::pushes(&mesh).into_iter().map(|(v, _)| (v, 0.0)).collect();
                (!points.is_empty()).then_some((id, Operation::SetCagePush { points }))
            })
            .collect();
        if ops.is_empty() {
            self.note = Some(tr("The cage has nothing painted on it.").to_string());
            return;
        }
        if self.record_each(tr("Clear the painted cage"), ops) {
            self.note = Some(tr("The painted cage is cleared. Ctrl + Z takes it back.").to_string());
        }
    }
}
