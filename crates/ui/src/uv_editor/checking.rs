//! Seeing and checking in the UV Editor ([decision 0115](../../../../docs/decisions/0115-seeing-and-checking.md)):
//!
//! - **Warnings**, always on: pieces lying over others (and polygons
//!   folded over within their piece) tinted red, pieces lying mirrored
//!   tinted purple, and how many of each, top right
//!   (`uv::warnings`, worked out again when the document changes).
//! - **Numbers on pieces** (View menu): each piece's size in texture
//!   pixels, its texel density and how stretched it is at worst, written
//!   on it.
//! - **Measure** (View menu, then drag): a line's length in texture pixels
//!   and on the surface, in centimetres (`uv::measure`).

use super::{UvEditor, UvShape};
use crate::text::tr;
use crate::theme;
use egui::{Align2, FontId, Pos2, Rect, Stroke, Ui, vec2};
use glam::Vec2;
use scene::{Document, FaceId, ObjectId};
use std::collections::HashMap;

/// The View menu's switch for numbers on pieces.
pub(crate) const PIECE_NUMBERS: &str = crate::text!("Numbers on pieces");

/// The View menu's entry that starts measuring.
pub(crate) const MEASURE: &str = crate::text!("Measure");

/// Pieces narrower than this on screen, in points, get no numbers.
const NUMBERS_FROM: f32 = 70.0;

/// How texture pieces are grouped onto textures, as packing does
/// (decision 0101): by material, by object, both or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TextureGroups {
    pub(crate) per_material: bool,
    pub(crate) per_object: bool,
}

/// The warnings worked out for one state of the document.
#[derive(Debug, Clone, Default)]
pub(super) struct WarningCache {
    /// The document's revision, what was shown and how pieces group.
    key: (u64, Vec<ObjectId>, TextureGroups),
    each: HashMap<ObjectId, uv::warnings::Warnings>,
}

/// Measuring: the line drawn, and the surfaces under it.
#[derive(Debug, Clone, Default)]
pub(super) struct Measuring {
    /// Where the line starts and ends, in the texture.
    line: Option<(Vec2, Vec2)>,
    /// Whether the line is being dragged.
    dragging: bool,
    /// The shown shapes' surfaces, for the document's revision.
    surfaces: Option<(u64, Vec<uv::measure::Surface>)>,
    /// The line's length on the surface, in centimetres, for `line`.
    length: Option<((Vec2, Vec2), f32)>,
}

/// One piece's numbers, as written on it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PieceNumbers {
    /// The piece's box in the texture.
    pub(super) min: Vec2,
    pub(super) max: Vec2,
    /// Its texel density, in texture widths per scene unit.
    pub(super) density: Option<f32>,
    /// How stretched its worst polygon is (1: evenly).
    pub(super) stretch: f32,
}

/// The numbers of each of `shape`'s pieces.
pub(super) fn piece_numbers(shape: &UvShape) -> Vec<PieceNumbers> {
    shape
        .islands
        .iter()
        .map(|faces| {
            let slots: Vec<usize> = faces
                .iter()
                .filter_map(|f| shape.polygons.iter().position(|(g, _)| g == f))
                .collect();
            let mut min = Vec2::INFINITY;
            let mut max = Vec2::NEG_INFINITY;
            let (mut flat, mut real) = (0.0, 0.0);
            let mut stretch: f32 = 1.0;
            for i in &slots {
                for (_, uv) in &shape.polygons[*i].1 {
                    min = min.min(*uv);
                    max = max.max(*uv);
                }
                let (f, r) = shape.areas[*i];
                flat += f;
                real += r;
                if let Some(s) = shape.stretch.get(*i)
                    && !s.empty
                {
                    stretch = stretch.max(s.uneven);
                }
            }
            PieceNumbers {
                min,
                max,
                density: (flat > 1e-20 && real > 1e-20).then(|| (flat / real).sqrt()),
                stretch,
            }
        })
        .collect()
}

/// The text written on a piece: its size in pixels, its texel density
/// and, when it's uneven, its worst stretch.
pub(super) fn numbers_text(numbers: &PieceNumbers, texture: scene::TextureSettings) -> String {
    let size = (numbers.max - numbers.min) * texture.size as f32;
    let mut text = crate::trf!("{} × {} px", size.x.round(), size.y.round());
    if let Some(d) = numbers.density {
        text.push_str(&crate::trf!(
            "\n{} px/m",
            texture.pixels_per_metre(d).round()
        ));
    }
    if numbers.stretch > 1.05 {
        text.push_str(&crate::trf!("\nstretched {:.1}×", numbers.stretch));
    }
    text
}

/// A measured line's text: its length in texture pixels, and on the
/// surface in centimetres if it crosses any.
pub(crate) fn measure_text(pixels: f32, centimetres: f32) -> String {
    if centimetres > 0.0 {
        crate::trf!(
            "{} px · {:.1} cm on the surface",
            pixels.round(),
            centimetres
        )
    } else {
        crate::trf!("{} px · not over a piece", pixels.round())
    }
}

/// How many pieces each warning is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct WarningCounts {
    pub(crate) overlapping: usize,
    pub(crate) mirrored: usize,
    /// Sticking out of the texture square (decision 0118).
    pub(crate) outside: usize,
}

impl WarningCounts {
    /// The counts over every shown shape.
    pub(crate) fn of<'a>(each: impl IntoIterator<Item = &'a uv::warnings::Warnings>) -> Self {
        each.into_iter().fold(Self::default(), |c, w| Self {
            overlapping: c.overlapping + w.overlapping_pieces,
            mirrored: c.mirrored + w.mirrored_pieces,
            outside: c.outside + w.outside_pieces,
        })
    }
}

/// The warnings' summary, top right: how many pieces overlap, lie
/// mirrored and stick out of the square.
pub(crate) fn warning_text(counts: WarningCounts) -> Option<String> {
    let mut parts = Vec::new();
    let pieces = |n: usize| if n == 1 { "piece" } else { "pieces" };
    if counts.overlapping > 0 {
        parts.push(crate::trf!(
            "{} {} overlapping",
            counts.overlapping,
            pieces(counts.overlapping)
        ));
    }
    if counts.mirrored > 0 {
        parts.push(crate::trf!(
            "{} {} mirrored",
            counts.mirrored,
            pieces(counts.mirrored)
        ));
    }
    if counts.outside > 0 {
        parts.push(crate::trf!(
            "{} {} outside the square",
            counts.outside,
            pieces(counts.outside)
        ));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

impl UvEditor {
    /// Whether numbers are written on pieces.
    pub(crate) fn piece_numbers(&self) -> bool {
        self.numbers
    }

    /// Turns the numbers on pieces on or off.
    pub(crate) fn set_piece_numbers(&mut self, on: bool) {
        self.numbers = on;
    }

    /// Whether measuring is on.
    pub(crate) fn measuring(&self) -> bool {
        self.measure.is_some()
    }

    /// Starts or stops measuring.
    pub(crate) fn set_measuring(&mut self, on: bool) {
        self.measure = on.then(Measuring::default);
    }

    /// The measured line and its length on the surface in centimetres, if
    /// one has been drawn (for tests).
    #[cfg(test)]
    pub(crate) fn measured(&self) -> Option<((Vec2, Vec2), f32)> {
        self.measure.as_ref()?.length
    }

    /// How many pieces each warning is about, as last drawn (for tests).
    #[cfg(test)]
    pub(crate) fn warning_counts(&self) -> WarningCounts {
        self.warnings
            .as_ref()
            .map_or_else(WarningCounts::default, |w| {
                WarningCounts::of(w.each.values())
            })
    }

    /// The warnings for what is shown, worked out again if the document
    /// changed.
    pub(super) fn warnings(
        &mut self,
        doc: &Document,
        shapes: &[(ObjectId, UvShape)],
        groups: TextureGroups,
    ) -> &HashMap<ObjectId, uv::warnings::Warnings> {
        let ids: Vec<ObjectId> = shapes.iter().map(|(id, _)| *id).collect();
        let key = (doc.revision(), ids, groups);
        if self.warnings.as_ref().is_some_and(|w| w.key != key) {
            self.warnings = None;
        }
        let cache = self.warnings.get_or_insert_with(|| {
            let scene = doc.scene();
            let meshes: Vec<&scene::Mesh> = shapes.iter().map(|(_, s)| s.mesh.as_ref()).collect();
            let rows: Vec<(f32, f32)> = scene
                .trim()
                .rows
                .iter()
                .map(|r| (r.bottom, r.top))
                .collect();
            let group = |m: usize, piece: &[FaceId]| {
                let id = shapes[m].0;
                let material = if groups.per_material {
                    let slot = piece
                        .first()
                        .map_or(0, |f| mesh::material::slot_of(meshes[m], *f));
                    scene
                        .object(id)
                        .and_then(|o| o.materials.get(slot as usize).map(|m| m.name.clone()))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                (groups.per_object.then_some(id), material)
            };
            let found = uv::warnings::warnings(&meshes, group, &rows);
            WarningCache {
                key,
                each: shapes.iter().map(|(id, _)| *id).zip(found).collect(),
            }
        });
        &cache.each
    }

    /// Tints the polygons warned about: red where pieces overlap, purple
    /// where they lie mirrored, amber where they stick out of the square.
    pub(super) fn paint_warnings(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        shape: &UvShape,
        warnings: &uv::warnings::Warnings,
    ) {
        if warnings.is_clean() {
            return;
        }
        let mut tint = egui::Mesh::default();
        for (face, points) in &shape.polygons {
            let color = if warnings.overlapping.contains(face) {
                theme::UV_OVERLAP
            } else if warnings.mirrored.contains(face) {
                theme::UV_MIRRORED
            } else if warnings.outside.contains(face) {
                theme::UV_OUTSIDE
            } else {
                continue;
            };
            let corners: Vec<Pos2> = points
                .iter()
                .map(|(_, uv)| self.to_screen(rect, *uv))
                .collect();
            super::add_polygon(&mut tint, &corners, color);
        }
        painter.add(egui::Shape::mesh(tint));
    }

    /// Writes each big enough piece's numbers on it.
    pub(super) fn paint_numbers(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        shape: &UvShape,
        texture: scene::TextureSettings,
    ) {
        for numbers in piece_numbers(shape) {
            let (a, b) = (
                self.to_screen(rect, numbers.min),
                self.to_screen(rect, numbers.max),
            );
            if (b.x - a.x).abs() < NUMBERS_FROM || !rect.contains(a.lerp(b, 0.5)) {
                continue;
            }
            let galley = painter.layout(
                numbers_text(&numbers, texture),
                FontId::proportional(11.0),
                theme::TEXT,
                (b.x - a.x).abs(),
            );
            let at = a.lerp(b, 0.5) - galley.size() / 2.0;
            painter.rect_filled(
                Rect::from_min_size(at, galley.size()).expand(3.0),
                3.0,
                theme::UV_BACKGROUND.gamma_multiply(0.8),
            );
            painter.galley(at, galley, theme::TEXT);
        }
    }

    /// Measuring: a drag draws the line (replacing the last one); its
    /// length is worked out as it changes. Returns whether measuring took
    /// the mouse.
    pub(super) fn use_measure(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        rect: Rect,
        doc: &Document,
    ) -> bool {
        let Some(mut measure) = self.measure.take() else {
            return false;
        };
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(from) = ui.input(|i| i.pointer.press_origin())
        {
            let start = self.to_uv(rect, from);
            measure.line = Some((start, start));
            measure.dragging = true;
        }
        if measure.dragging {
            if let (Some((start, _)), Some(now)) =
                (measure.line, ui.input(|i| i.pointer.latest_pos()))
            {
                measure.line = Some((start, self.to_uv(rect, now)));
            }
            if response.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
                measure.dragging = false;
            }
        }
        if let Some(line) = measure.line
            && measure.length.is_none_or(|(l, _)| l != line)
        {
            let revision = doc.revision();
            if measure
                .surfaces
                .as_ref()
                .is_none_or(|(r, _)| *r != revision)
            {
                let scene = doc.scene();
                let surfaces = super::shown_objects(doc)
                    .into_iter()
                    .filter_map(|id| {
                        let mesh = scene.evaluated(id)?.mesh;
                        uv::measure::Surface::new(&mesh, scene.world_matrix(id))
                    })
                    .collect();
                measure.surfaces = Some((revision, surfaces));
            }
            let surfaces = measure
                .surfaces
                .as_ref()
                .map_or(&[][..], |(_, s)| s.as_slice());
            measure.length = Some((line, uv::measure::surface_length(surfaces, line.0, line.1)));
        }
        self.measure = Some(measure);
        true
    }

    /// Draws the measured line and its length, or how to measure.
    pub(super) fn paint_measure(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        texture: scene::TextureSettings,
    ) {
        let Some(measure) = &self.measure else {
            return;
        };
        let Some(((a, b), cm)) = measure.length else {
            painter.text(
                rect.center_top() + vec2(0.0, 10.0),
                Align2::CENTER_TOP,
                tr("Measuring: drag a line · Esc stops"),
                FontId::proportional(12.0),
                theme::UV_MEASURE,
            );
            return;
        };
        let (p, q) = (self.to_screen(rect, a), self.to_screen(rect, b));
        let stroke = Stroke::new(2.0, theme::UV_MEASURE);
        painter.line_segment([p, q], stroke);
        for end in [p, q] {
            painter.circle_filled(end, 3.5, theme::UV_MEASURE);
        }
        let pixels = (b - a).length() * texture.size as f32;
        let galley = painter.layout_no_wrap(
            measure_text(pixels, cm),
            FontId::proportional(12.0),
            theme::UV_MEASURE,
        );
        let at = p.lerp(q, 0.5) + vec2(8.0, -galley.size().y - 6.0);
        painter.rect_filled(
            Rect::from_min_size(at, galley.size()).expand(3.0),
            3.0,
            theme::UV_BACKGROUND.gamma_multiply(0.9),
        );
        painter.galley(at, galley, theme::UV_MEASURE);
    }
}

#[cfg(test)]
mod tests;
