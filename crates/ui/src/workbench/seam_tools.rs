//! Quicker seam work ([decision 0234](../../../../docs/decisions/0234-quicker-seam-work.md)):
//! the **Fewer seams ↔ less stretch** slider, the **Seam brush**, **Mark
//! seam loop** and **Mark seam ring**, and **Mirror seams**.
//!
//! All are in UV → Seams, without keys (the project lead's choice). The
//! slider re-runs Suggest seams as it is dragged, each place replacing the
//! last in one undo step, so the seams can be watched changing before
//! letting go. The brush is the selection brush's circle: dragging over
//! edges marks them as seams, Ctrl + drag clears them, `[` and `]` change
//! its size, and Esc goes back to selecting (the controls chosen for the
//! UV brushes, phase 8 plan). Each stroke is one undo step holding one
//! seam change per object (and, with Unwrap as seams change, one unwrap
//! of the pieces around them, made when the stroke ends).

use super::Workbench;
use super::joining::counted;
use super::seams::{changing, seams_op};
use crate::drawing::SceneDrawing;
use crate::palette::Mode;
use crate::selecting::{Gesture, SelectTool};
use crate::text::tr;
use egui::{Key, Modifiers, Ui};
use scene::{EdgeKey, Element, ElementKind, ObjectId};
use std::collections::{BTreeMap, BTreeSet};

/// The slider's name, and what it does.
pub(crate) const BALANCE_LABEL: &str = crate::text!("Fewer seams ↔ less stretch");
pub(crate) const BALANCE_HELP: &str = crate::text!(
    "Drag to choose between few seams (pieces stretch more) and many (pieces lie flatter), and \
     watch Suggest seams follow on the selected objects. Letting go keeps it; Ctrl + Z takes \
     it back. It sets the corner angle and stretch budget in Settings → Seam suggestions."
);
/// The slider's two ends.
pub(crate) const FEWER_SEAMS_LABEL: &str = crate::text!("Fewer seams");
pub(crate) const LESS_STRETCH_LABEL: &str = crate::text!("Less stretch");
/// The Seam brush's name, and what it does.
pub(crate) const SEAM_BRUSH_LABEL: &str = crate::text!("Seam brush");
pub(crate) const SEAM_BRUSH_HELP: &str = crate::text!(
    "Paint seams: drag over edges of the selected objects to mark them as seams, Ctrl + drag to \
     clear them. [ and ] change the brush's size; Esc goes back to selecting."
);
/// Mark seam loop's and Mark seam ring's names, and what they do.
pub(crate) const SEAM_LOOP_LABEL: &str = crate::text!("Mark seam loop");
pub(crate) const SEAM_LOOP_HELP: &str = crate::text!(
    "Mark the whole edge loop through each selected edge as a seam: the line of edges running \
     straight on, as a double-click selects it. Good for cutting a cylinder open along its length."
);
pub(crate) const SEAM_RING_LABEL: &str = crate::text!("Mark seam ring");
pub(crate) const SEAM_RING_HELP: &str = crate::text!(
    "Mark the edge ring through each selected edge as seams: the edges side by side across a \
     strip of quads, as Alt + double-click selects them. Good for cutting a strip into pieces."
);
/// Mirror seams' name, and what it does.
pub(crate) const MIRROR_SEAMS_LABEL: &str = crate::text!("Mirror seams");
pub(crate) const MIRROR_SEAMS_HELP: &str = crate::text!(
    "Copy the selected objects' seams to the other side, left to right and right to left \
     (across the object's own X axis), so both halves unwrap alike. Shapes made with Symmetry \
     get mirrored seams already."
);

/// The seam brush while in use.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SeamBrush {
    /// The select tool to go back to.
    before: SelectTool,
    /// Whether Ctrl is held this frame (a stroke starting now clears).
    pub(super) ctrl: bool,
    /// The stroke in progress.
    stroke: Option<Stroke>,
}

/// A seam brush stroke.
#[derive(Debug, Clone, PartialEq)]
struct Stroke {
    /// The viewport's number for the stroke (`None`: a click).
    number: Option<u64>,
    /// Its undo step.
    key: u64,
    /// Whether it clears seams instead of marking them.
    clear: bool,
    /// The edges gone over, per object.
    edges: BTreeMap<ObjectId, BTreeSet<EdgeKey>>,
}

impl Workbench {
    /// UV → Seams' slider: Fewer seams ↔ less stretch.
    pub(super) fn seam_balance_slider(&mut self, ui: &mut Ui) {
        ui.label(tr(BALANCE_LABEL)).on_hover_text(tr(BALANCE_HELP));
        let response = ui
            .horizontal(|ui| {
                ui.small(tr(FEWER_SEAMS_LABEL));
                let slider = egui::Slider::new(&mut self.uv_settings.seam_balance, 0.0..=1.0)
                    .show_value(false);
                let response = ui.add(slider).on_hover_text(tr(BALANCE_HELP));
                ui.small(tr(LESS_STRETCH_LABEL));
                response
            })
            .inner;
        if response.changed() {
            self.apply_seam_balance();
        }
        if !response.dragged() {
            self.seam_slider_key = None;
        }
    }

    /// Sets the corner angle and budget from the slider, and re-runs
    /// Suggest seams on the selection, replacing what the slider did a
    /// moment ago in the same undo step.
    pub(super) fn apply_seam_balance(&mut self) {
        let (corner, budget) = uv::seam_tools::balanced(self.uv_settings.seam_balance);
        let s = &mut self.uv_settings;
        s.seam_corner_angle = corner;
        s.budget_on = budget.is_some();
        if let Some(budget) = budget {
            s.budget = budget;
        }
        if self.document.selection().is_empty() {
            self.note = Some(crate::trf!(
                "{}: the settings changed. Select objects to see their seams follow.",
                tr(BALANCE_LABEL)
            ));
            return;
        }
        let key = match self.seam_slider_key {
            Some(key) => {
                self.document.abandon(key);
                key
            }
            None => self.new_step_key(),
        };
        self.seam_slider_key = Some(key);
        self.note = Some(match self.suggest_seams_in(key) {
            Some((total, _)) => crate::trf!(
                "{}: {} for the selected objects. Ctrl + Z takes it back.",
                tr(BALANCE_LABEL),
                counted("", total, tr("seam")).trim_start()
            ),
            None => crate::trf!("{}: the seams stay as they are here.", tr(BALANCE_LABEL)),
        });
    }

    /// Mark seam loop (`ring` false) or Mark seam ring, on the selected
    /// edges.
    pub(crate) fn run_seam_loops(&mut self, ring: bool) {
        let name = if ring {
            tr(SEAM_RING_LABEL)
        } else {
            tr(SEAM_LOOP_LABEL)
        };
        if self.mode != Mode::Edges {
            self.note = Some(crate::trf!(
                "{}: press 2 for the Edges mode and select an edge on the loop or ring first.",
                name
            ));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        let mut any = false;
        for (id, selected, mesh) in self.selected_parts() {
            let edges: Vec<EdgeKey> = selected.edges().collect();
            any |= !edges.is_empty();
            let wider = if ring {
                uv::seam_tools::rings(&mesh, &edges)
            } else {
                uv::seam_tools::loops(&mesh, &edges)
            };
            let marked = changing(&mesh, wider, true);
            if marked.is_empty() {
                continue;
            }
            count += marked.len();
            let live = self.live_unwrap(&mesh, &marked);
            ops.push((id, seams_op(&mesh, marked, true)));
            ops.extend(live.map(|op| (id, op)));
        }
        if ops.is_empty() {
            self.note = Some(if any {
                crate::trf!("{}: every edge there is a seam already.", name)
            } else {
                crate::trf!("{}: select an edge on the loop or ring first.", name)
            });
            return;
        }
        let label = counted(tr("Mark as seams"), count, tr("edge"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }

    /// Mirror seams: copies each selected object's seams across its X
    /// axis.
    pub(crate) fn run_mirror_seams(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects first.",
                tr(MIRROR_SEAMS_LABEL)
            ));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh) in self.own_shapes() {
            let seams = mesh::uvs::seams(&mesh);
            // Never on an edge marked "never" (decision 0111).
            let mirrored = uv::seam_tools::mirrored(&mesh, &seams, 0)
                .into_iter()
                .filter(|e| mesh::uvs::seam_preference(&mesh, *e) != scene::SeamPreference::Never);
            let marked = changing(&mesh, mirrored, true);
            if marked.is_empty() {
                continue;
            }
            count += marked.len();
            let live = self.live_unwrap(&mesh, &marked);
            ops.push((id, seams_op(&mesh, marked, true)));
            ops.extend(live.map(|op| (id, op)));
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: both sides have the same seams already (or the shape isn't symmetrical \
                 across X).",
                tr(MIRROR_SEAMS_LABEL)
            ));
            return;
        }
        let label = counted(tr("Mirror"), count, tr("seam"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }

    /// Whether the seam brush is in use.
    pub(crate) fn seam_brush_on(&self) -> bool {
        self.seam_brush.is_some()
    }

    /// Picks up the seam brush: the Edges mode, and the selection brush's
    /// circle.
    pub(crate) fn start_seam_brush(&mut self) {
        if self.document.selection().is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the objects to paint seams on first.",
                tr(SEAM_BRUSH_LABEL)
            ));
            return;
        }
        if self.seam_brush.is_none() {
            self.seam_brush = Some(SeamBrush {
                before: self.viewport.select_tool(),
                ctrl: false,
                stroke: None,
            });
        }
        self.mode = Mode::Edges;
        self.tool = crate::transforming::Tool::Select;
        self.viewport.set_select_tool(SelectTool::Brush);
        self.note = Some(tr(
            "Seam brush: drag over edges to mark them as seams; Ctrl + drag clears them. [ and ] \
             change the size; Esc goes back to selecting.",
        )
        .into());
    }

    /// Puts the seam brush down, finishing any stroke.
    pub(crate) fn stop_seam_brush(&mut self) {
        self.finish_seam_stroke();
        if let Some(brush) = self.seam_brush.take() {
            self.viewport.set_select_tool(brush.before);
            self.note = Some(tr("Seam brush put down: dragging selects again.").into());
        }
    }

    /// Each frame while the brush is in use: Esc puts it down, `[` and `]`
    /// change its size, Ctrl is noted for the next stroke, and a stroke
    /// ends when the button is let go. Leaving the Edges mode, or choosing
    /// another select tool or a gizmo, puts it down too.
    pub(super) fn seam_brush_keys(&mut self, ui: &Ui) {
        let Some(brush) = &mut self.seam_brush else {
            return;
        };
        brush.ctrl = ui.input(|i| i.modifiers.command);
        let down = ui.input(|i| i.pointer.primary_down());
        if !down {
            self.finish_seam_stroke();
        }
        let left = self.mode != Mode::Edges
            || self.viewport.select_tool() != SelectTool::Brush
            || self.tool != crate::transforming::Tool::Select;
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        if left || (!typing && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))) {
            self.stop_seam_brush();
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
    /// from the gestures and marks (or clears) the edges under them; the
    /// rest are given back.
    pub(super) fn take_seam_brush(
        &mut self,
        gestures: Vec<Gesture>,
        drawing: &SceneDrawing,
    ) -> Vec<Gesture> {
        if self.seam_brush.is_none() {
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
                        .flat_map(|r| scene.elements_in(ElementKind::Edges, &camera, size, r, true))
                        .collect();
                    (stroke, hits)
                }
                Gesture::Click { pointer, .. } => {
                    let scene = self.components.pick_scene(drawing);
                    let hit = scene.under(
                        ElementKind::Edges,
                        &pointer.camera,
                        pointer.size,
                        pointer.at,
                    );
                    (None, hit.into_iter().collect())
                }
                other => {
                    rest.push(other);
                    continue;
                }
            };
            let found: Vec<(ObjectId, EdgeKey)> = hits
                .into_iter()
                .filter_map(|hit| {
                    let id = drawing.items.get(hit.object)?.id;
                    match hit.element {
                        Element::Edge(e) => Some((id, e)),
                        _ => None,
                    }
                })
                .collect();
            self.seam_dab(number, found);
            if number.is_none() {
                // A click is a stroke of its own.
                self.finish_seam_stroke();
            }
        }
        rest
    }

    /// Adds `found` to the stroke `number` (starting it if it is new) and
    /// records the stroke so far, replacing what it recorded a dab ago.
    pub(super) fn seam_dab(&mut self, number: Option<u64>, found: Vec<(ObjectId, EdgeKey)>) {
        let same = self
            .seam_brush
            .as_ref()
            .and_then(|b| b.stroke.as_ref())
            .is_some_and(|s| s.number.is_some() && s.number == number);
        if !same {
            self.finish_seam_stroke();
            let key = self.new_step_key();
            let Some(brush) = &mut self.seam_brush else {
                return;
            };
            brush.stroke = Some(Stroke {
                number,
                key,
                clear: brush.ctrl,
                edges: BTreeMap::new(),
            });
        }
        let Some(stroke) = self.seam_brush.as_mut().and_then(|b| b.stroke.as_mut()) else {
            return;
        };
        let mut grew = false;
        for (id, edge) in found {
            grew |= stroke.edges.entry(id).or_default().insert(edge);
        }
        if !grew {
            return;
        }
        let (key, clear) = (stroke.key, stroke.clear);
        let edges = stroke.edges.clone();
        self.document.abandon(key);
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, edges) in edges {
            let Some(mesh) = self.document.scene().evaluated(id).map(|e| e.mesh) else {
                continue;
            };
            let changed = changing(&mesh, edges, !clear);
            if changed.is_empty() {
                continue;
            }
            count += changed.len();
            ops.push((id, seams_op(&mesh, changed, !clear)));
        }
        let verb = if clear {
            tr("Clear the seams of")
        } else {
            tr("Mark as seams")
        };
        let label = counted(verb, count, tr("edge"));
        if self.record_each_in(&label, ops, key) {
            self.note = Some(crate::trf!("{}.", label));
        }
    }

    /// Ends the stroke in progress: with Unwrap as seams change on, the
    /// pieces around its edges are unwrapped again, in the same undo
    /// step.
    pub(super) fn finish_seam_stroke(&mut self) {
        let Some(stroke) = self.seam_brush.as_mut().and_then(|b| b.stroke.take()) else {
            return;
        };
        let mut ops = Vec::new();
        for (id, edges) in &stroke.edges {
            let Some(mesh) = self.document.scene().evaluated(*id).map(|e| e.mesh) else {
                continue;
            };
            let edges: Vec<EdgeKey> = edges.iter().copied().collect();
            if let Some(op) = self.live_unwrap(&mesh, &edges) {
                ops.push((*id, op));
            }
        }
        // Only onto the stroke's own step: if something else was done
        // since, the unwrap is left for the next Unwrap.
        if !ops.is_empty() && self.document.last_merge_key() == Some(stroke.key) {
            let label = self.document.undo_label().unwrap_or_default().to_owned();
            self.record_each_in(&label, ops, stroke.key);
        }
    }
}
