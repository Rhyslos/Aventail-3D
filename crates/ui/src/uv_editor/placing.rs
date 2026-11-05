//! Putting pieces exactly where they belong in the UV Editor
//! ([decision 0237](../../../../docs/decisions/0237-precise-uv-placement.md)):
//! what moves snap to, what the gizmo turns and scales about, and typing
//! exact numbers.
//!
//! - **Snapping** works as in the viewport: the magnet button, Ctrl
//!   flipping it. The editor's **Snap** menu says what to: a grid of your
//!   own size (a tenth of the square, as before), texture pixels, other
//!   pieces' corners and edges, and trim rows. Corners, edges and trim
//!   rows win when one is within reach of a moving corner; otherwise the
//!   middle lands on the grid (or on a pixel).
//! - **Pivot** (the editor's Pivot menu): the selection's middle, each
//!   piece's own middle, a corner of the selection's box, the texture's
//!   middle, or a point of your own, placed with **Ctrl + click** in the
//!   editor.
//! - **Exact** (the editor's Exact menu): the selection's middle (U, V)
//!   and size, in pixels or as parts of the square, and a turn, typed;
//!   each change is one undo step.

use crate::text::tr;
use egui::Ui;
use glam::Vec2;
use scene::{Document, FaceId, VertexId};
use std::collections::BTreeSet;

/// The UV Editor's menus for placing.
pub(crate) const SNAP_MENU: &str = crate::text!("Snap");
pub(crate) const PIVOT_MENU: &str = crate::text!("Pivot");
pub(crate) const EXACT_MENU: &str = crate::text!("Exact");

/// What moves in the UV Editor snap to.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UvSnap {
    /// A grid of `grid_size`.
    pub grid: bool,
    /// The grid's step, as a part of the square (0.1: ten across).
    pub grid_size: f32,
    /// Texture pixels.
    pub pixels: bool,
    /// Other pieces' corners.
    pub corners: bool,
    /// Other pieces' edges.
    pub edges: bool,
    /// The trim sheet's row edges.
    pub trim: bool,
}

impl Default for UvSnap {
    fn default() -> Self {
        Self {
            grid: true,
            grid_size: 0.1,
            pixels: false,
            corners: false,
            edges: false,
            trim: false,
        }
    }
}

/// The smallest and largest grid step.
pub(crate) const GRID_RANGE: std::ops::RangeInclusive<f32> = 0.001..=0.5;

/// How near (screen points) a corner, edge or trim row must be for a
/// moving corner to snap to it.
pub(crate) const SNAP_REACH: f32 = 10.0;

/// The most moving corners tried against targets (more is slow and
/// changes nothing a person can see).
const MOST_TRIED: usize = 4000;

/// What a move can snap to, gathered as a drag begins.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UvSnapping {
    pub(crate) settings: UvSnap,
    /// The texture's size in pixels.
    pub(crate) pixels: u32,
    /// How near counts, in texture widths.
    pub(crate) reach: f32,
    /// The corners that move, where they started.
    pub(crate) moving: Vec<Vec2>,
    /// Corners that stay.
    pub(crate) points: Vec<Vec2>,
    /// Edges that stay.
    pub(crate) segments: Vec<[Vec2; 2]>,
    /// The trim rows' edges, as heights.
    pub(crate) rows: Vec<f32>,
}

fn nearest_on(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
    a + ab * t
}

/// `x` on the nearest step of `by`.
fn step(x: f32, by: f32) -> f32 {
    if by > 0.0 { (x / by).round() * by } else { x }
}

impl UvSnapping {
    /// `by` (a move of the selection, whose middle starts at `pivot`)
    /// snapped: to the nearest corner, edge or trim row within reach of a
    /// moving corner, else the middle to the grid or a pixel. `keep`
    /// limits it to across (0) or up (1).
    pub(crate) fn snap_move(&self, pivot: Vec2, by: Vec2, keep: Option<usize>) -> Vec2 {
        let s = &self.settings;
        let limit = |mut d: Vec2| {
            match keep {
                Some(0) => d.y = 0.0,
                Some(1) => d.x = 0.0,
                _ => {}
            }
            d
        };
        let mut best: Option<Vec2> = None;
        let mut consider = |d: Vec2| {
            let d = limit(d);
            if d.length() <= self.reach && best.is_none_or(|b| d.length() < b.length()) {
                best = Some(d);
            }
        };
        if s.corners || s.edges || s.trim {
            for m in self.moving.iter().take(MOST_TRIED) {
                let p = *m + by;
                if s.corners {
                    for q in &self.points {
                        consider(*q - p);
                    }
                }
                if s.edges {
                    for [a, b] in &self.segments {
                        consider(nearest_on(p, *a, *b) - p);
                    }
                }
                if s.trim {
                    for y in &self.rows {
                        consider(Vec2::new(0.0, y - p.y));
                    }
                }
            }
        }
        if let Some(d) = best {
            return by + d;
        }
        let size = if s.grid {
            if s.grid_size.is_finite() {
                s.grid_size.clamp(*GRID_RANGE.start(), *GRID_RANGE.end())
            } else {
                UvSnap::default().grid_size
            }
        } else if s.pixels {
            1.0 / self.pixels.max(1) as f32
        } else {
            return by;
        };
        let to = pivot + by;
        limit(Vec2::new(step(to.x, size), step(to.y, size)) - pivot)
    }
}

/// What the gizmo turns and scales about.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum UvPivot {
    /// The middle of the selection's box.
    #[default]
    Middle,
    /// Each piece about its own middle.
    EachPiece,
    /// A corner of the selection's box (0 bottom left, 1 bottom right, 2
    /// top right, 3 top left).
    BoxCorner(u8),
    /// The texture square's middle.
    TextureMiddle,
    /// A point of your own (placed with Ctrl + click).
    Own,
}

impl UvPivot {
    /// Every choice, in menu order.
    pub(crate) const ALL: [Self; 8] = [
        Self::Middle,
        Self::EachPiece,
        Self::BoxCorner(0),
        Self::BoxCorner(1),
        Self::BoxCorner(2),
        Self::BoxCorner(3),
        Self::TextureMiddle,
        Self::Own,
    ];

    /// Its name in the menu.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Middle => tr("The selection's middle"),
            Self::EachPiece => tr("Each piece's own middle"),
            Self::BoxCorner(0) => tr("Bottom left of the selection"),
            Self::BoxCorner(1) => tr("Bottom right of the selection"),
            Self::BoxCorner(2) => tr("Top right of the selection"),
            Self::BoxCorner(_) => tr("Top left of the selection"),
            Self::TextureMiddle => tr("The texture's middle"),
            Self::Own => tr("Your own point (Ctrl + click)"),
        }
    }

    /// Where it is for a selection boxed by `lo` and `hi`, with `own` the
    /// placed point. Each piece's own middle is the selection's middle
    /// here (each piece uses its own as it changes).
    pub(crate) fn spot(self, lo: Vec2, hi: Vec2, own: Option<Vec2>) -> Vec2 {
        let middle = (lo + hi) / 2.0;
        match self {
            Self::Middle | Self::EachPiece => middle,
            Self::BoxCorner(0) => lo,
            Self::BoxCorner(1) => Vec2::new(hi.x, lo.y),
            Self::BoxCorner(2) => hi,
            Self::BoxCorner(_) => Vec2::new(lo.x, hi.y),
            Self::TextureMiddle => Vec2::splat(0.5),
            Self::Own => own.unwrap_or(middle),
        }
    }
}

/// The corners a change of `targets` moves (where they are), and the
/// corners and edges of every shown object that stay, for snapping.
pub(crate) fn gather(
    doc: &Document,
    targets: &[super::gizmo::UvTarget],
) -> (Vec<Vec2>, Vec<Vec2>, Vec<[Vec2; 2]>) {
    let mut moving = Vec::new();
    let mut still = Vec::new();
    let mut segments = Vec::new();
    for id in super::shown_objects(doc) {
        let Some(evaluated) = doc.scene().evaluated(id) else {
            continue;
        };
        let mesh = &evaluated.mesh;
        let Some(uvs) = mesh::uvs::Uvs::of(mesh) else {
            continue;
        };
        let target = targets.iter().find(|t| t.id == id);
        let moved: BTreeSet<usize> = target
            .map(|t| mesh::uvs::corners_moved(mesh, &uvs, &t.faces, &t.points))
            .unwrap_or_default()
            .into_iter()
            .collect();
        let (starts, _) = mesh.face_layout();
        for (f, list) in mesh.faces() {
            let s = starts[f.get() as usize] as usize;
            let n = list.len();
            for k in 0..n {
                let i = s + k;
                let p = uvs.at(i);
                if moved.contains(&i) {
                    moving.push(p);
                } else {
                    still.push(p);
                    let j = s + (k + 1) % n;
                    if !moved.contains(&j) {
                        segments.push([p, uvs.at(j)]);
                    }
                }
            }
        }
    }
    (moving, still, segments)
}

/// Each selected piece of every target, as a target of its own, with its
/// own middle: for turning and scaling each piece about itself.
pub(crate) fn per_piece(
    doc: &Document,
    targets: &[super::gizmo::UvTarget],
) -> Vec<(super::gizmo::UvTarget, Vec2)> {
    let mut out = Vec::new();
    for t in targets {
        let Some(evaluated) = doc.scene().evaluated(t.id) else {
            continue;
        };
        let mesh = &evaluated.mesh;
        // The polygons the target touches: its own, or round its points.
        let touched: Vec<FaceId> = if t.faces.is_empty() {
            let pts: BTreeSet<VertexId> = t.points.iter().copied().collect();
            mesh.faces()
                .filter(|(_, l)| l.iter().any(|v| pts.contains(v)))
                .map(|(f, _)| f)
                .collect()
        } else {
            t.faces.clone()
        };
        for piece in uv::pieces::pieces_holding(mesh, &touched) {
            let inside: BTreeSet<FaceId> = piece.iter().copied().collect();
            let faces: Vec<FaceId> = t.faces.iter().copied().filter(|f| inside.contains(f)).collect();
            let points: Vec<VertexId> = if t.faces.is_empty() {
                let in_piece: BTreeSet<VertexId> = piece
                    .iter()
                    .flat_map(|f| mesh.face(*f).unwrap_or(&[]).iter().copied())
                    .collect();
                t.points.iter().copied().filter(|v| in_piece.contains(v)).collect()
            } else {
                Vec::new()
            };
            if faces.is_empty() && points.is_empty() {
                continue;
            }
            let Some((lo, hi)) = mesh::uvs::uv_bounds(mesh, &faces, &points) else {
                continue;
            };
            out.push((
                super::gizmo::UvTarget {
                    id: t.id,
                    faces,
                    points,
                    anchors: t.anchors.clone(),
                },
                (lo + hi) / 2.0,
            ));
        }
    }
    out
}

/// The editor's Snap menu.
pub(crate) fn snap_menu(ui: &mut Ui, snap: &mut UvSnap) {
    ui.label(tr("While snapping is on (the magnet, or Ctrl), moves land on:"));
    ui.horizontal(|ui| {
        ui.checkbox(&mut snap.grid, tr("A grid of"));
        ui.add_enabled(
            snap.grid,
            egui::DragValue::new(&mut snap.grid_size)
                .range(GRID_RANGE)
                .speed(0.001)
                .max_decimals(3),
        )
        .on_hover_text(tr("The grid's step, as a part of the square: 0.1 is ten across"));
    });
    ui.checkbox(&mut snap.pixels, tr("Texture pixels"))
        .on_hover_text(tr("The middle lands on a pixel (when the grid is off)"));
    ui.checkbox(&mut snap.corners, tr("Other pieces' corners"));
    ui.checkbox(&mut snap.edges, tr("Other pieces' edges"));
    ui.checkbox(&mut snap.trim, tr("Trim rows"))
        .on_hover_text(tr("The top and bottom of each row of the trim sheet"));
}

/// The editor's Pivot menu.
pub(crate) fn pivot_menu(ui: &mut Ui, pivot: &mut UvPivot, own: Option<Vec2>) {
    for choice in UvPivot::ALL {
        if matches!(choice, UvPivot::BoxCorner(0) | UvPivot::TextureMiddle) {
            ui.separator();
        }
        if ui.radio_value(pivot, choice, choice.name()).changed() {
            ui.close();
        }
    }
    if let Some(at) = own {
        ui.label(
            egui::RichText::new(crate::trf!("Your point: U {:.3}, V {:.3}", at.x, at.y)).small(),
        );
    }
}

/// How the Exact menu shows sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ExactUnits {
    /// Texture pixels.
    #[default]
    Pixels,
    /// Parts of the square, 0 to 1.
    Square,
}

/// A change typed in the Exact menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExactChange {
    /// Move by this much (texture widths).
    Move(Vec2),
    /// Scale by these factors about the middle.
    Scale(Vec2),
    /// Turn by this many degrees about the pivot.
    Turn(f32),
}

/// The Exact menu: the selection's middle and size (in `units`), and a
/// turn, for selection boxed by `lo` to `hi` on a texture of `pixels`.
/// Returns the change typed, if any.
pub(crate) fn exact_menu(
    ui: &mut Ui,
    units: &mut ExactUnits,
    turn: &mut f32,
    bounds: Option<(Vec2, Vec2)>,
    pixels: u32,
) -> Option<ExactChange> {
    ui.horizontal(|ui| {
        ui.radio_value(units, ExactUnits::Pixels, tr("Pixels"));
        ui.radio_value(units, ExactUnits::Square, tr("0 to 1"));
    });
    let Some((lo, hi)) = bounds else {
        ui.label(tr("Select some UVs (1, 2 or 3, or the UV mode) to place them exactly."));
        return None;
    };
    let k = match units {
        ExactUnits::Pixels => pixels.max(1) as f32,
        ExactUnits::Square => 1.0,
    };
    let middle = (lo + hi) / 2.0;
    let size = hi - lo;
    let mut change = None;
    egui::Grid::new("uv-exact").num_columns(3).show(ui, |ui| {
        ui.label("");
        ui.label(tr("Across (U)"));
        ui.label(tr("Up (V)"));
        ui.end_row();
        ui.label(tr("Middle"));
        let mut m = middle * k;
        let speed = if k > 1.0 { 1.0 } else { 0.001 };
        let du = ui.add(egui::DragValue::new(&mut m.x).speed(speed).max_decimals(3));
        let dv = ui.add(egui::DragValue::new(&mut m.y).speed(speed).max_decimals(3));
        if (du.changed() || dv.changed()) && m.is_finite() {
            change = Some(ExactChange::Move(m / k - middle));
        }
        ui.end_row();
        ui.label(tr("Size"));
        let mut s = size * k;
        let su = ui.add_enabled(size.x > 1e-9, egui::DragValue::new(&mut s.x).speed(speed).max_decimals(3));
        let sv = ui.add_enabled(size.y > 1e-9, egui::DragValue::new(&mut s.y).speed(speed).max_decimals(3));
        if (su.changed() || sv.changed()) && s.is_finite() {
            let f = Vec2::new(
                if size.x > 1e-9 { (s.x / k / size.x).max(1e-3) } else { 1.0 },
                if size.y > 1e-9 { (s.y / k / size.y).max(1e-3) } else { 1.0 },
            );
            change = Some(ExactChange::Scale(f));
        }
        ui.end_row();
    });
    ui.horizontal(|ui| {
        ui.label(tr("Turn by"));
        ui.add(egui::DragValue::new(turn).range(-360.0..=360.0).speed(1.0).suffix("°"));
        if ui.button(tr("Turn")).clicked() && turn.is_finite() && *turn != 0.0 {
            change = Some(ExactChange::Turn(*turn));
        }
    });
    change
}

#[cfg(test)]
mod tests;
