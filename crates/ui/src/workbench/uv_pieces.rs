//! Editing pieces by hand ([decision 0236](../../../../docs/decisions/0236-editing-pieces.md)):
//! Stitch (**C** in the UV Editor) and Cut (**X** in the UV Editor), in the
//! E ring's Stitch & cut group too; Align and spread, Orient by edge and
//! Lattice warp in UV → Pieces. Each records `Operation::SetUvs` (and
//! seam changes) as one undo step.
//!
//! The lattice is a grid of handles over the selected pieces in the UV
//! Editor: drag a handle and the UVs follow, bent smoothly between the
//! handles; each drag is one undo step. **Esc** (or Done) puts the
//! lattice away (the project lead's choice: from the menus, then drag,
//! no keys).

use super::Workbench;
use super::joining::counted;
use super::seams::{changing, seams_op};
use super::unwrapping::targets;
use crate::palette::Mode;
use crate::text::tr;
use egui::{Key, Modifiers, Pos2, Sense, Stroke, Ui};
use glam::Vec2;
use scene::{EdgeKey, FaceId, ObjectId, Operation};
use std::collections::BTreeSet;
use uv::pieces::{Align, Corner};

/// The keys for Stitch and Cut, in the UV Editor only (chosen by the
/// project lead: C bridges in the viewport, X slits there).
pub(crate) const STITCH_KEY: Key = Key::C;
pub(crate) const CUT_KEY: Key = Key::X;
/// The names and what they do.
pub(crate) const STITCH_LABEL: &str = crate::text!("Stitch");
pub(crate) const STITCH_HELP: &str = crate::text!(
    "Join the selected edges where the texture is cut: the smaller piece is moved, turned and \
     sized to meet the other, and the seam goes. C over the UV Editor."
);
pub(crate) const CUT_LABEL: &str = crate::text!("Cut");
pub(crate) const CUT_HELP: &str = crate::text!(
    "Cut the texture along the selected edges: they become seams, and a part cut off is nudged \
     away so you can move it. X over the UV Editor."
);
pub(crate) const ALIGN_LABEL: &str = crate::text!("Align and spread");
pub(crate) const ORIENT_LABEL: &str = crate::text!("Orient by edge");
pub(crate) const ORIENT_HELP: &str = crate::text!(
    "Turn each piece holding a selected edge so the edge lies straight across or straight up \
     in the texture, whichever is nearer. Select one edge per piece."
);
pub(crate) const LATTICE_LABEL: &str = crate::text!("Lattice warp");
pub(crate) const LATTICE_HELP: &str = crate::text!(
    "Put a grid of handles over the selected pieces in the UV Editor: drag the handles to bend \
     the UVs smoothly (each drag one undo step). Esc or Done puts it away."
);
pub(crate) const LATTICE_DONE_LABEL: &str = crate::text!("Done warping");

/// An alignment's name in the menu.
pub(crate) fn align_name(how: Align) -> &'static str {
    match how {
        Align::Left => tr("Align left"),
        Align::Centre => tr("Align centres across"),
        Align::Right => tr("Align right"),
        Align::Top => tr("Align top"),
        Align::Middle => tr("Align middles up and down"),
        Align::Bottom => tr("Align bottom"),
        Align::SpreadAcross => tr("Spread evenly across"),
        Align::SpreadUp => tr("Spread evenly up"),
    }
}

/// How many handles the lattice has each way.
pub(crate) const LATTICE_SIZE: usize = 3;
/// How near (screen points) a click must be to grab a handle.
const GRAB: f32 = 9.0;

/// A lattice over the selected pieces.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Lattice {
    /// Each object's corners, with their UVs as the lattice began: where
    /// each corner hangs on the lattice.
    parts: Vec<(ObjectId, Vec<Corner>)>,
    lo: Vec2,
    hi: Vec2,
    /// The handles now.
    pub(super) points: Vec<Vec2>,
    /// The drag going on, if any.
    grabbed: Option<LatticeDrag>,
    /// The drags made, oldest first, and the drags undone, the next to
    /// redo last: so undo and redo put the handles back with the UVs.
    done: Vec<LatticeStep>,
    undone: Vec<LatticeStep>,
}

/// A drag of one lattice handle. Each drag moves the UVs from where they
/// are as it begins, by how much the lattice changes during it: so UV
/// edits made while the lattice is up (or an undo) are kept.
#[derive(Debug, Clone, PartialEq)]
struct LatticeDrag {
    /// The handle dragged.
    handle: usize,
    /// The drag's undo step.
    key: u64,
    /// The handles as the drag began.
    from: Vec<Vec2>,
    /// Each object's corners: their UV as the lattice began (where they
    /// hang on it) and as the drag began.
    corners: Vec<(ObjectId, Vec<(Corner, Vec2)>)>,
}

/// A drag recorded in the undo history: where in the history it is (the
/// steps made before it and it, counting forgotten ones), and the handles
/// before and after it.
#[derive(Debug, Clone, PartialEq)]
struct LatticeStep {
    place: usize,
    before: Vec<Vec2>,
    after: Vec<Vec2>,
}

impl Workbench {
    /// The selected edges to stitch or cut: in the Edges mode the
    /// selected ones; with polygons, every edge of the selected polygons
    /// (for stitching) or the border of the selection (for cutting).
    fn piece_edges(
        &self,
        stitching: bool,
    ) -> Vec<(ObjectId, std::sync::Arc<scene::Mesh>, Vec<EdgeKey>)> {
        self.uv_selected_parts()
            .into_iter()
            .map(|(id, selected, mesh)| {
                let edges: Vec<EdgeKey> = match self.mode {
                    Mode::Edges => selected.edges().collect(),
                    Mode::Polygons | Mode::Uv => {
                        let faces: BTreeSet<FaceId> = selected.polygons().collect();
                        if stitching {
                            let t = mesh.topology();
                            faces
                                .iter()
                                .flat_map(|f| t.face_edges(*f).iter().copied())
                                .collect::<BTreeSet<_>>()
                                .into_iter()
                                .collect()
                        } else {
                            super::seams::border_of(&mesh, &faces).into_iter().collect()
                        }
                    }
                    _ => Vec::new(),
                };
                (id, mesh, edges)
            })
            .filter(|(_, _, e)| !e.is_empty())
            .collect()
    }

    /// Stitch: joins the selected edges where the texture is cut.
    pub(crate) fn run_stitch(&mut self) {
        let parts = self.piece_edges(true);
        if parts.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select edges where the texture is cut (2 for the Edges mode), or polygons.",
                tr(STITCH_LABEL)
            ));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh, edges) in parts {
            let done = uv::pieces::stitch(&mesh, &edges);
            if done.joined.is_empty() {
                continue;
            }
            count += done.joined.len();
            ops.push((
                id,
                Operation::SetUvs {
                    corners: done.corners,
                },
            ));
            let seams = changing(&mesh, done.joined, false);
            if !seams.is_empty() {
                ops.push((id, seams_op(&mesh, seams, false)));
            }
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: those edges aren't cut in the texture (or both sides are locked).",
                tr(STITCH_LABEL)
            ));
            return;
        }
        let label = counted(tr("Stitch"), count, tr("edge"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }

    /// Cut: marks the selected edges as seams and nudges what they part.
    pub(crate) fn run_cut(&mut self) {
        let parts = self.piece_edges(false);
        if parts.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select the edges to cut along (2 for the Edges mode), or polygons to cut out.",
                tr(CUT_LABEL)
            ));
            return;
        }
        // A nudge of two texture pixels.
        let gap = 2.0 / self.document.scene().texture().size.max(1) as f32;
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, mesh, edges) in parts {
            let marked = changing(&mesh, edges.iter().copied(), true);
            if marked.is_empty() {
                continue;
            }
            count += marked.len();
            let moved = uv::pieces::cut(&mesh, &marked, gap);
            ops.push((id, seams_op(&mesh, marked, true)));
            if !moved.is_empty() {
                ops.push((id, Operation::SetUvs { corners: moved }));
            }
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: those edges are cut already (or open edges).",
                tr(CUT_LABEL)
            ));
            return;
        }
        let label = counted(tr("Cut along"), count, tr("edge"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!(
                "{}. Ctrl + Z takes it back. A cut that doesn't part a piece opens when you Unwrap.",
                label
            ));
        }
    }

    /// The pieces holding the selection, per object.
    fn chosen_uv_pieces(&self) -> Vec<(ObjectId, std::sync::Arc<scene::Mesh>, Vec<Vec<FaceId>>)> {
        self.own_shapes()
            .into_iter()
            .filter(|(_, mesh)| mesh::uvs::has_uvs(mesh))
            .map(|(id, mesh)| {
                let faces = targets(&mesh, self.document.elements(id), self.mode);
                let pieces = uv::pieces::pieces_holding(&mesh, &faces);
                (id, mesh, pieces)
            })
            .filter(|(_, _, p)| !p.is_empty())
            .collect()
    }

    /// UV → Pieces → Align and spread's entries.
    pub(super) fn align_entries(&mut self, ui: &mut Ui) {
        for how in Align::ALL {
            if matches!(how, Align::SpreadAcross) {
                ui.separator();
            }
            if ui.button(align_name(how)).clicked() {
                self.run_align(how);
                ui.close();
            }
        }
    }

    /// Lines up (or spreads) the pieces holding the selection, each
    /// object's on their own.
    pub(crate) fn run_align(&mut self, how: Align) {
        let mut ops = Vec::new();
        let mut count = 0;
        let mut pieces_found = 0;
        for (id, mesh, pieces) in self.chosen_uv_pieces() {
            pieces_found += pieces.len();
            let corners = uv::pieces::align(&mesh, &pieces, how);
            if corners.is_empty() {
                continue;
            }
            count += pieces.len();
            ops.push((id, Operation::SetUvs { corners }));
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select polygons in two or more pieces (three to spread), in the same \
                 object. They may be in line already.",
                align_name(how)
            ));
            let _ = pieces_found;
            return;
        }
        let label = counted(align_name(how), count, tr("piece"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }

    /// Orient by edge: turns each piece holding a selected edge square to
    /// it.
    pub(crate) fn run_orient(&mut self) {
        if self.mode != Mode::Edges {
            self.note = Some(crate::trf!(
                "{}: press 2 for the Edges mode and select an edge of the piece.",
                tr(ORIENT_LABEL)
            ));
            return;
        }
        let mut ops = Vec::new();
        let mut count = 0;
        for (id, selected, mesh) in self.selected_parts() {
            let mut corners: Vec<Corner> = Vec::new();
            let mut used: BTreeSet<FaceId> = BTreeSet::new();
            for edge in selected.edges() {
                let faces: Vec<FaceId> = mesh.topology().edge_faces(edge).collect();
                for piece in uv::pieces::pieces_holding(&mesh, &faces) {
                    if piece.iter().any(|f| used.contains(f)) {
                        continue;
                    }
                    let turned = uv::pieces::orient(&mesh, &piece, edge);
                    if !turned.is_empty() {
                        used.extend(piece.iter().copied());
                        corners.extend(turned);
                        count += 1;
                    }
                    break;
                }
            }
            if !corners.is_empty() {
                ops.push((id, Operation::SetUvs { corners }));
            }
        }
        if ops.is_empty() {
            self.note = Some(crate::trf!(
                "{}: select an edge of each piece to turn (it may lie straight already).",
                tr(ORIENT_LABEL)
            ));
            return;
        }
        let label = counted(tr("Orient"), count, tr("piece"));
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }

    /// C and X over the UV Editor.
    pub(super) fn uv_piece_keys(&mut self, ui: &Ui) {
        if !self.uv_editor.hovered(ui.ctx()) || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        if crate::components::kind_for(self.mode).is_none() && self.mode != Mode::Uv {
            return;
        }
        let (stitch, cut) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, STITCH_KEY),
                i.consume_key(Modifiers::NONE, CUT_KEY),
            )
        });
        if stitch {
            self.run_stitch();
        }
        if cut {
            self.run_cut();
        }
    }

    /// Whether the lattice is up.
    pub(crate) fn warping(&self) -> bool {
        self.lattice.is_some()
    }

    /// Puts a lattice over the pieces holding the selection.
    pub(crate) fn start_lattice(&mut self) {
        self.stop_uv_brush();
        let mut parts = Vec::new();
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        for (id, mesh, pieces) in self.chosen_uv_pieces() {
            let faces: Vec<FaceId> = pieces.into_iter().flatten().collect();
            let corners = uv::pieces::corners_now(&mesh, &faces);
            for (_, _, uv) in &corners {
                lo = lo.min(*uv);
                hi = hi.max(*uv);
            }
            if !corners.is_empty() {
                parts.push((id, corners));
            }
        }
        if parts.is_empty() || !lo.is_finite() || (hi - lo).min_element() <= 1e-6 {
            self.note = Some(crate::trf!(
                "{}: select the pieces to warp first (they need some size both ways).",
                tr(LATTICE_LABEL)
            ));
            return;
        }
        self.lattice = Some(Lattice {
            parts,
            lo,
            hi,
            points: uv::pieces::lattice_grid(lo, hi, LATTICE_SIZE),
            grabbed: None,
            done: Vec::new(),
            undone: Vec::new(),
        });
        self.note = Some(tr(
            "Lattice warp: drag the handles in the UV Editor to bend the UVs. Esc or Done puts it \
             away.",
        )
        .into());
    }

    /// Puts the lattice away (what it did stays).
    pub(crate) fn stop_lattice(&mut self) {
        if self.lattice.take().is_some() {
            self.note = Some(tr("Lattice put away. Ctrl + Z takes back each drag.").into());
        }
    }

    /// Starts dragging lattice handle `k`: notes where the handles and
    /// the corners' UVs are now, for the drag to move them from.
    fn begin_lattice_drag(&mut self, k: usize) {
        let key = self.new_step_key();
        let scene = self.document.scene();
        let Some(lattice) = &self.lattice else {
            return;
        };
        if k >= lattice.points.len() {
            return;
        }
        let corners = lattice
            .parts
            .iter()
            .filter_map(|(id, hung)| {
                let mesh = scene.evaluated(*id)?.mesh;
                let faces: Vec<FaceId> = hung
                    .iter()
                    .map(|(f, _, _)| *f)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                // Polygons deleted or locked since are left out.
                let now: std::collections::BTreeMap<(FaceId, scene::VertexId), Vec2> =
                    uv::pieces::corners_now(&mesh, &faces)
                        .into_iter()
                        .map(|(f, v, uv)| ((f, v), uv))
                        .collect();
                let pairs: Vec<(Corner, Vec2)> = hung
                    .iter()
                    .filter_map(|c| Some((*c, *now.get(&(c.0, c.1))?)))
                    .collect();
                (!pairs.is_empty()).then_some((*id, pairs))
            })
            .collect();
        let from = lattice.points.clone();
        if let Some(l) = &mut self.lattice {
            // A new drag can't be followed by a redo.
            l.undone.clear();
            l.grabbed = Some(LatticeDrag {
                handle: k,
                key,
                from,
                corners,
            });
        }
    }

    /// Moves the dragged handle to `uv` and records the drag's warp,
    /// replacing what it recorded before.
    fn move_lattice_handle(&mut self, uv: Vec2) {
        let Some(lattice) = &mut self.lattice else {
            return;
        };
        let Some(drag) = &lattice.grabbed else {
            return;
        };
        if lattice.points[drag.handle] == uv {
            return;
        }
        lattice.points[drag.handle] = uv;
        let (lo, hi, n) = (lattice.lo, lattice.hi, LATTICE_SIZE);
        let ops: Vec<(ObjectId, Operation)> = drag
            .corners
            .iter()
            .map(|(id, corners)| {
                let moved = corners
                    .iter()
                    .map(|((f, v, hung), now)| {
                        // How far the lattice has moved this spot during
                        // the drag, added to where the corner was.
                        let by = uv::pieces::warp_point(*hung, lo, hi, n, &lattice.points)
                            - uv::pieces::warp_point(*hung, lo, hi, n, &drag.from);
                        (*f, *v, *now + by)
                    })
                    .filter(|(_, _, uv)| uv.is_finite())
                    .collect();
                (*id, Operation::SetUvs { corners: moved })
            })
            .collect();
        let key = drag.key;
        self.document.abandon(key);
        self.record_each_in(tr(LATTICE_LABEL), ops, key);
    }

    /// Ends a handle drag, noting it for undo and redo if it made a step.
    fn end_lattice_drag(&mut self) {
        let place = self.history_place();
        let last = self.document.last_merge_key();
        if let Some(l) = &mut self.lattice
            && let Some(drag) = l.grabbed.take()
            && last == Some(drag.key)
        {
            l.done.push(LatticeStep {
                place,
                before: drag.from,
                after: l.points.clone(),
            });
        }
    }

    /// How many steps have been made up to the last that can be undone,
    /// forgotten ones included: a step's place in the history.
    fn history_place(&self) -> usize {
        self.document.undo_steps() + self.document.forgotten_steps()
    }

    /// Undoes the last step, putting the lattice's handles back when it
    /// was a lattice drag. Returns what was undone.
    pub(super) fn undo_with_lattice(&mut self) -> Option<String> {
        let place = self.history_place();
        let what = self.document.undo()?;
        if let Some(l) = &mut self.lattice
            && l.grabbed.is_none()
            && l.done.last().is_some_and(|d| d.place == place)
            && let Some(step) = l.done.pop()
        {
            l.points = step.before.clone();
            l.undone.push(step);
        }
        Some(what)
    }

    /// Redoes the last undone step, putting the lattice's handles where
    /// it left them when it was a lattice drag. Returns what was redone.
    pub(super) fn redo_with_lattice(&mut self) -> Option<String> {
        let what = self.document.redo()?;
        let place = self.history_place();
        if let Some(l) = &mut self.lattice
            && l.grabbed.is_none()
            && l.undone.last().is_some_and(|d| d.place == place)
            && let Some(step) = l.undone.pop()
        {
            l.points = step.after.clone();
            l.done.push(step);
        }
        Some(what)
    }

    /// Each frame while the lattice is up: Esc puts it away, and over the
    /// UV Editor its handles are drawn and dragged.
    pub(super) fn use_lattice(&mut self, ui: &Ui) {
        if self.lattice.is_none() {
            return;
        }
        if self.mode == Mode::Paint {
            self.stop_lattice();
            return;
        }
        if !ui.ctx().egui_wants_keyboard_input()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.stop_lattice();
            return;
        }
        // A new step after an undo means there is nothing to redo.
        if self.document.redo_steps() == 0
            && let Some(l) = &mut self.lattice
        {
            l.undone.clear();
        }
        let Some(rect) = self.uv_editor.drawn() else {
            return;
        };
        let accent = crate::theme::accent(ui.ctx()).colors();
        let screen: Vec<Option<Pos2>> = self
            .lattice
            .as_ref()
            .map(|l| {
                l.points
                    .iter()
                    .map(|p| self.uv_editor.screen_of(*p))
                    .collect()
            })
            .unwrap_or_default();
        let done = egui::Area::new(egui::Id::new("uv-lattice"))
            .fixed_pos(rect.min)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                let response = ui.allocate_rect(
                    egui::Rect::from_min_size(rect.min, rect.size()),
                    Sense::drag(),
                );
                let painter = ui.painter_at(rect);
                let n = LATTICE_SIZE;
                for j in 0..n {
                    for i in 0..n {
                        let here = screen[j * n + i];
                        for next in [
                            (i + 1 < n).then(|| j * n + i + 1),
                            (j + 1 < n).then(|| (j + 1) * n + i),
                        ]
                        .into_iter()
                        .flatten()
                        {
                            if let (Some(a), Some(b)) = (here, screen[next]) {
                                painter.line_segment([a, b], Stroke::new(1.0, accent.light));
                            }
                        }
                    }
                }
                for p in screen.iter().flatten() {
                    painter.circle(*p, 4.5, accent.fill, Stroke::new(1.0, accent.light));
                }
                let button = egui::Rect::from_min_size(
                    rect.right_top() + egui::vec2(-130.0, 8.0),
                    egui::vec2(120.0, 22.0),
                );
                let done = ui
                    .put(button, egui::Button::new(tr(LATTICE_DONE_LABEL)))
                    .clicked();
                (response, done)
            })
            .inner;
        let (response, finished) = done;
        if finished {
            self.stop_lattice();
            return;
        }
        // Panning and zooming work over the lattice as over the editor
        // (a middle drag pans): they take the drag, not the handles.
        let navigation = self.viewport.options().navigation;
        self.uv_editor.navigate(ui, &response, rect, &navigation);
        let navigating = self.uv_editor.navigating();
        if response.drag_started_by(egui::PointerButton::Primary)
            && !navigating
            && let Some(at) = response.interact_pointer_pos()
        {
            let nearest = screen
                .iter()
                .enumerate()
                .filter_map(|(k, p)| Some((k, p.as_ref()?.distance(at))))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .filter(|(_, d)| *d <= GRAB);
            if let Some((k, _)) = nearest {
                self.begin_lattice_drag(k);
            }
        }
        if response.dragged()
            && self.lattice.as_ref().is_some_and(|l| l.grabbed.is_some())
            && let Some(at) = response.interact_pointer_pos()
            && let Some(uv) = self.uv_editor.uv_of(at)
        {
            self.move_lattice_handle(uv);
        }
        if response.drag_stopped() || !ui.input(|i| i.pointer.any_down()) {
            self.end_lattice_drag();
        }
    }

    /// Moves lattice handle `k` to `uv` as one drag (for tests).
    #[cfg(test)]
    pub(super) fn drag_lattice_handle(&mut self, k: usize, uv: Vec2) {
        self.begin_lattice_drag(k);
        self.move_lattice_handle(uv);
        self.end_lattice_drag();
    }
}
