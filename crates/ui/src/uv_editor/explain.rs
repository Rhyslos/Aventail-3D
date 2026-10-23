//! "Why is this here?" ([decision 0116](../../../../docs/decisions/0116-why-is-this-here.md)):
//! while **View → Explain choices** is ticked in the UV Editor, pointing
//! at a seam or a piece says why it is there and how to change it.
//!
//! Seams are explained by working out, with the current settings, where
//! Suggest seams would put them and why (`uv::seams::suggest_explained`):
//! a seam it would make gets its reason (a hard edge, a sharp corner, a
//! cut to lie flat, a preferred edge, a stretch budget slit); any other
//! seam was made by hand or by another tool. Pieces are explained from
//! what they are: locked, on a trim row, pinned, with settings of their
//! own, their size against the texel density aimed for, and the warnings
//! and stretch on them. Nothing is saved.

use super::{PICK_DISTANCE, UvEditor, UvShape, inside, segment_distance};
use crate::text::tr;
use crate::theme;
use egui::{FontId, Pos2, Rect, vec2};
use glam::Vec2;
use scene::{Document, EdgeKey, FaceId, Mesh, ObjectId};
use std::collections::BTreeMap;
use std::sync::Arc;
use uv::SeamReason;

/// The View menu's switch.
pub(crate) const EXPLAIN_CHOICES: &str = crate::text!("Explain choices");

/// Shapes with more polygons than this aren't run through Suggest seams
/// to explain a seam: it would take too long while pointing.
const EXPLAIN_SEAMS_UP_TO: usize = 20_000;

/// What the mouse points at, to explain.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Pointed {
    /// A seam of an object.
    Seam(ObjectId, EdgeKey),
    /// A piece of an object: its polygons.
    Piece(ObjectId, Vec<FaceId>),
}

/// Where Suggest seams would put seams on a shape, and why, for one set
/// of settings.
#[derive(Debug, Clone)]
pub(super) struct SeamReasons {
    mesh: Arc<Mesh>,
    options: uv::SuggestOptions,
    hardness: mesh::Hardness,
    reasons: Option<Arc<BTreeMap<EdgeKey, SeamReason>>>,
}

/// What the explanations need from the rest of the window.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ExplainContext {
    /// The settings Suggest seams would use.
    pub(crate) suggest: uv::SuggestOptions,
}

/// Why Suggest seams would put a seam on an edge, in plain words, with
/// the corner setting `corner` (degrees).
pub(crate) fn reason_text(reason: SeamReason, corner: f32) -> String {
    match reason {
        SeamReason::HardEdge => tr(
            "It's a hard edge. A seam where the shading turns hard keeps \
                                 the normal map clean there.",
        )
        .into(),
        SeamReason::SharpCorner(angle) => crate::trf!(
            "The polygons meet at {:.0}°, sharper than the {:.0}° set in UV → \
             Settings → Seam suggestions. Pieces fold apart best along sharp corners.",
            angle,
            corner
        ),
        SeamReason::ToLieFlat => tr("This piece is closed or loops round, so it can't lie flat \
                                  without a cut. This is the most hidden way to cut it, as UV → \
                                  Settings → Seam suggestions → Hide cuts says.")
        .into(),
        SeamReason::Preferred => tr(
            "The piece needed a cut to lie flat, and you preferred a seam \
                                  here (UV → Seams → Prefer seams here).",
        )
        .into(),
        SeamReason::StretchBudget(budget) => crate::trf!(
            "Laid flat, the piece stretched more than its budget of {:.2}×, so it was slit \
             here, from where it was squeezed most.",
            budget
        ),
    }
}

/// How to change a seam, the same for every seam.
pub(crate) const SEAM_FIX: &str = crate::text!(
    "To change it: select the edge (Edges mode, 2) and choose UV → \
                                   Seams → Clear seams, or Never seams here so suggestions avoid \
                                   it. Prefer seams here on other edges draws cuts there instead."
);

/// A seam's explanation: its reason, if Suggest seams would make it,
/// else where it may have come from; whether it's hard or preferred.
pub(crate) fn seam_text(
    reason: Option<SeamReason>,
    known: bool,
    corner: f32,
    preference: scene::SeamPreference,
) -> String {
    let why = match reason {
        Some(r) => reason_text(r, corner),
        None if known => tr(
            "Suggest seams wouldn't put one here with the current settings: it was \
                          marked by hand, came with the ready-made shape, or was added by Seams \
                          from hard edges or an unwrap cutting a closed piece open.",
        )
        .into(),
        None => {
            tr("This shape is too big to work out Suggest seams' reasons while pointing.").into()
        }
    };
    let extra = match preference {
        scene::SeamPreference::Never => {
            tr("\nIt's marked Never seams here, so suggestions won't put it back.")
        }
        _ => "",
    };
    crate::trf!("Seam\n{}{}\n{}", why, extra, tr(SEAM_FIX))
}

/// What is known about a piece, for its explanation.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PieceFacts {
    pub(crate) polygons: usize,
    pub(crate) locked: bool,
    /// The name of the trim row it lies on.
    pub(crate) trim_row: Option<String>,
    pub(crate) pinned: bool,
    pub(crate) own_settings: bool,
    /// Its texel density and the one aimed for, in px/m.
    pub(crate) density: Option<f32>,
    pub(crate) aim: f32,
    pub(crate) overlapping: bool,
    pub(crate) mirrored: bool,
    /// Its worst polygon's stretch (1: even).
    pub(crate) stretch: f32,
}

/// A piece's explanation, a line per fact, each with what to do about
/// it.
pub(crate) fn piece_text(f: &PieceFacts) -> String {
    let mut lines = vec![crate::trf!(
        "Piece of {} {}",
        f.polygons,
        if f.polygons == 1 {
            tr("polygon")
        } else {
            tr("polygons")
        }
    )];
    if f.locked {
        lines.push(
            tr("Locked: no automatic tool changes it. UV → Pieces → Unlock pieces frees it.")
                .into(),
        );
    }
    if let Some(row) = &f.trim_row {
        lines.push(crate::trf!(
            "On the trim row “{}”, laid along it by Put on trim row. Pack leaves it there.",
            row
        ));
    }
    if f.pinned {
        lines.push(tr("Some of its UVs are pinned: Unwrap lays it out around them.").into());
    }
    if f.own_settings {
        lines.push(
            tr(
                "It has unwrap settings of its own (UV → Pieces → Piece settings), used instead of \
             the app's.",
            )
            .into(),
        );
    }
    if let Some(d) = f.density {
        let ratio = d / f.aim.max(1e-6);
        lines.push(if (0.95..=1.05).contains(&ratio) {
            crate::trf!(
                "Its size: {} px/m, the texel density aimed for (Texture menu). Unwrap sizes \
                 pieces to it.",
                d.round()
            )
        } else if ratio < 1.0 {
            crate::trf!(
                "Its size: {} px/m, {:.2}× the aim of {}. Pack shrinks every piece by the same \
                 amount so all fit in the square. For more texture: a bigger texture size \
                 (Texture menu), less padding, or a texture per material.",
                d.round(),
                ratio,
                f.aim.round()
            )
        } else {
            crate::trf!(
                "Its size: {} px/m, {:.2}× the aim of {}: scaled up by hand, by Pack filling the \
                 square, or by a density of its own. UV → Lay out → Set texel density puts it \
                 back to the aim.",
                d.round(),
                ratio,
                f.aim.round()
            )
        });
    }
    if !f.locked && f.trim_row.is_none() {
        lines.push(
            tr(
                "Its place: where it was laid out, moved by hand, or where Pack put it (biggest \
             pieces first, from the bottom left).",
            )
            .into(),
        );
    }
    if f.overlapping {
        lines.push(
            tr("It overlaps another piece (red): UV → Lay out → Pack moves pieces apart.").into(),
        );
    }
    if f.mirrored {
        lines.push(
            tr("It lies mirrored (purple): UV → Turn and flip → Flip left to right turns it back.")
                .into(),
        );
    }
    if f.stretch > 1.05 {
        lines.push(crate::trf!(
            "Stretched up to {:.1}×: UV → Lay out → Relax evens it out; a stretch budget (UV → \
             Pieces → Stretch budget) cuts it where it's worst.",
            f.stretch
        ));
    }
    lines.join("\n")
}

impl UvEditor {
    /// Whether choices are explained while pointing.
    pub(crate) fn explaining(&self) -> bool {
        self.explain
    }

    /// Turns explaining on or off.
    pub(crate) fn set_explaining(&mut self, on: bool) {
        self.explain = on;
        if !on {
            self.reasons.clear();
            self.pointed = None;
        }
    }

    /// What was pointed at last frame, while explaining (for tests).
    #[cfg(test)]
    pub(crate) fn pointed(&self) -> Option<&(Pointed, String)> {
        self.pointed.as_ref()
    }

    /// What the mouse at `at` points at: a seam within reach, else the
    /// piece under it (the last drawn, on top, first).
    pub(super) fn point_at(
        &self,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        at: Pos2,
    ) -> Option<Pointed> {
        let mut best: Option<(f32, ObjectId, EdgeKey)> = None;
        for (id, shape) in shapes {
            for ([a, b], key, _) in &shape.edges {
                if !shape.seams.contains(key) {
                    continue;
                }
                let d = segment_distance(at, self.to_screen(rect, *a), self.to_screen(rect, *b));
                if d <= PICK_DISTANCE && best.is_none_or(|(e, _, _)| d < e) {
                    best = Some((d, *id, *key));
                }
            }
        }
        if let Some((_, id, key)) = best {
            return Some(Pointed::Seam(id, key));
        }
        let uv = self.to_uv(rect, at);
        for (id, shape) in shapes.iter().rev() {
            for (face, points) in shape.polygons.iter().rev() {
                let corners: Vec<Vec2> = points.iter().map(|(_, uv)| *uv).collect();
                if inside(&corners, uv) {
                    let island = shape.island_of.get(face.get() as usize)?;
                    return Some(Pointed::Piece(*id, shape.islands.get(*island)?.clone()));
                }
            }
        }
        None
    }

    /// Where Suggest seams would put `shape`'s seams and why, worked out
    /// once per shape and settings; `None` for a shape too big.
    fn seam_reasons(
        &mut self,
        id: ObjectId,
        shape: &UvShape,
        cx: ExplainContext,
        hardness: mesh::Hardness,
    ) -> Option<Arc<BTreeMap<EdgeKey, SeamReason>>> {
        let fresh = self.reasons.get(&id).is_some_and(|r| {
            Arc::ptr_eq(&r.mesh, &shape.mesh) && r.options == cx.suggest && r.hardness == hardness
        });
        if !fresh {
            let reasons = (shape.mesh.face_count() <= EXPLAIN_SEAMS_UP_TO).then(|| {
                // The edges shading hard on the shape, as the viewport
                // shows them: by the object's own smoothing (decision 0126).
                let hard = mesh::hardness::shading_hard_edges(&shape.mesh, hardness);
                Arc::new(
                    uv::seams::suggest_explained(&shape.mesh, &hard, cx.suggest)
                        .into_iter()
                        .collect(),
                )
            });
            self.reasons.insert(
                id,
                SeamReasons {
                    mesh: Arc::clone(&shape.mesh),
                    options: cx.suggest,
                    hardness,
                    reasons,
                },
            );
        }
        self.reasons.get(&id)?.reasons.clone()
    }

    /// The explanation of what `pointed` is.
    pub(super) fn explanation(
        &mut self,
        pointed: &Pointed,
        shapes: &[(ObjectId, UvShape)],
        doc: &Document,
        warnings: &std::collections::HashMap<ObjectId, uv::warnings::Warnings>,
        cx: ExplainContext,
    ) -> Option<String> {
        match pointed {
            Pointed::Seam(id, edge) => {
                let (_, shape) = shapes.iter().find(|(s, _)| s == id)?;
                let own = match doc.scene().object(*id).map(|o| &o.kind) {
                    Some(scene::ObjectKind::Instance(source)) => *source,
                    _ => *id,
                };
                let hardness = doc
                    .scene()
                    .object(own)
                    .map_or(mesh::Hardness::Marked, |o| o.hardness);
                let reasons = self.seam_reasons(*id, shape, cx, hardness);
                let reason = reasons.as_ref().and_then(|r| r.get(edge).copied());
                Some(seam_text(
                    reason,
                    reasons.is_some(),
                    cx.suggest.corner_angle,
                    mesh::uvs::seam_preference(&shape.mesh, *edge),
                ))
            }
            Pointed::Piece(id, faces) => {
                let (_, shape) = shapes.iter().find(|(s, _)| s == id)?;
                let mesh = &shape.mesh;
                let scene = doc.scene();
                let texture = scene.texture();
                let trim_row = scene
                    .trim()
                    .rows
                    .iter()
                    .find(|r| uv::trim::on_row(mesh, faces, r.bottom, r.top))
                    .map(|r| r.name.clone());
                let numbers = super::checking::piece_numbers(shape)
                    .into_iter()
                    .zip(&shape.islands)
                    .find(|(_, island)| *island == faces)
                    .map(|(n, _)| n);
                let (starts, _) = mesh.face_layout();
                let pinned = mesh::uvs::pins(mesh).is_some_and(|pins| {
                    faces.iter().any(|f| {
                        let start = starts[f.get() as usize] as usize;
                        let n = mesh.face(*f).map_or(0, <[scene::VertexId]>::len);
                        (start..start + n).any(|k| pins.get(k).copied().unwrap_or(false))
                    })
                });
                let warned = warnings.get(id);
                let facts = PieceFacts {
                    polygons: faces.len(),
                    locked: mesh::uvs::any_locked(mesh, faces),
                    trim_row,
                    pinned,
                    own_settings: !uv::PieceSettings::of(mesh, faces).is_empty(),
                    density: numbers
                        .as_ref()
                        .and_then(|n| n.density)
                        .map(|d| texture.pixels_per_metre(d)),
                    aim: texture.density,
                    overlapping: warned
                        .is_some_and(|w| faces.iter().any(|f| w.overlapping.contains(f))),
                    mirrored: warned.is_some_and(|w| faces.iter().any(|f| w.mirrored.contains(f))),
                    stretch: numbers.map_or(1.0, |n| n.stretch),
                };
                Some(piece_text(&facts))
            }
        }
    }

    /// Lights up what is explained and writes the explanation by the
    /// mouse, kept inside the editor.
    pub(super) fn paint_explanation(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        shapes: &[(ObjectId, UvShape)],
        at: Pos2,
    ) {
        let Some((pointed, text)) = &self.pointed else {
            return;
        };
        if let Pointed::Seam(id, edge) = pointed
            && let Some((_, shape)) = shapes.iter().find(|(s, _)| s == id)
        {
            for ([a, b], key, _) in &shape.edges {
                if key == edge {
                    painter.line_segment(
                        [self.to_screen(rect, *a), self.to_screen(rect, *b)],
                        egui::Stroke::new(4.0, theme::UV_EXPLAIN),
                    );
                }
            }
        }
        let galley = painter.layout(
            text.clone(),
            FontId::proportional(12.0),
            theme::TEXT,
            320.0_f32.min(rect.width() - 30.0).max(80.0),
        );
        let size = galley.size();
        let mut min = at + vec2(16.0, 16.0);
        if min.x + size.x + 8.0 > rect.right() {
            min.x = at.x - 16.0 - size.x;
        }
        if min.y + size.y + 8.0 > rect.bottom() {
            min.y = at.y - 16.0 - size.y;
        }
        min = min.max(rect.min + vec2(6.0, 6.0));
        let frame = Rect::from_min_size(min, size).expand(6.0);
        painter.rect_filled(frame, 4.0, theme::UV_BACKGROUND.gamma_multiply(0.96));
        painter.rect_stroke(
            frame,
            4.0,
            egui::Stroke::new(1.0, theme::UV_EXPLAIN),
            egui::StrokeKind::Inside,
        );
        painter.galley(min, galley, theme::TEXT);
    }
}

#[cfg(test)]
mod tests;
