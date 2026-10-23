//! Drawing the editor: the texture square with its grid, pixels and trim
//! rows, each object's polygons, borders, seams and selection, and the
//! stretch view's colours and note.

use super::UvEditor;
use super::picking::Hit;
use super::shape::UvShape;
use crate::palette::Mode;
use crate::text::tr;
use crate::theme;
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, pos2, vec2};
use glam::Vec2;
use scene::{Element, ElementKind, ElementSelection, FaceId, ObjectId};
use std::collections::BTreeSet;

/// Selected edges in the editor: thicker than seams (2) and borders (1.6),
/// so the selection is the clearest thing drawn (decision 0126).
const UV_SELECTED_WIDTH: f32 = 3.5;

/// Selected points' radius.
const UV_SELECTED_RADIUS: f32 = 4.5;

/// How many screen points a texture pixel must cover before the pixels
/// are drawn.
const PIXELS_FROM: f32 = 8.0;

/// The numbered checker's texture in egui.
#[derive(Clone)]
pub(super) struct GridTexture(egui::TextureHandle);

impl std::fmt::Debug for GridTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("GridTexture").field(&self.0.id()).finish()
    }
}

impl UvEditor {
    /// The texture's pixels, when zoomed in far enough to see them
    /// (decision 0100): faint lines between them, over what is in view.
    fn paint_pixels(&self, painter: &egui::Painter, rect: Rect, square: Rect, size: u32) {
        let step = self.zoom / size as f32;
        if step < PIXELS_FROM {
            return;
        }
        let shown = square.intersect(rect);
        if !shown.is_positive() {
            return;
        }
        let low = self.to_uv(rect, shown.left_bottom());
        let high = self.to_uv(rect, shown.right_top());
        let n = size as f32;
        let stroke = Stroke::new(1.0, theme::UV_GRID.gamma_multiply(0.6));
        let first = |v: f32| (v * n).ceil() as i64;
        let last = |v: f32| (v * n).floor() as i64;
        for i in first(low.x)..=last(high.x) {
            let x = self.to_screen(rect, Vec2::new(i as f32 / n, 0.0)).x;
            painter.vline(x, shown.y_range(), stroke);
        }
        for j in first(low.y)..=last(high.y) {
            let y = self.to_screen(rect, Vec2::new(0.0, j as f32 / n)).y;
            painter.hline(shown.x_range(), y, stroke);
        }
    }

    /// The trim sheet's rows (decision 0106): faint bands across the
    /// square, every other one stronger, with their edges and names, and
    /// a fainter continuation to the right, where the rows repeat.
    pub(super) fn paint_trim(&self, painter: &egui::Painter, rect: Rect, trim: &scene::TrimSheet) {
        for (i, row) in trim.rows.iter().enumerate() {
            let a = self.to_screen(rect, Vec2::new(0.0, row.bottom));
            let b = self.to_screen(rect, Vec2::new(1.0, row.top));
            let band = Rect::from_two_pos(a, b);
            let fill = if i % 2 == 0 {
                theme::TRIM_ROW
            } else {
                theme::TRIM_ROW.gamma_multiply(1.8)
            };
            painter.rect_filled(band, 0.0, fill);
            let beyond = Rect::from_min_max(
                pos2(band.right(), band.top()),
                pos2(band.right() + band.width() * 0.5, band.bottom()),
            );
            painter.rect_filled(beyond, 0.0, fill.gamma_multiply(0.4));
            let edge = Stroke::new(1.0, theme::TRIM_ROW_EDGE.gamma_multiply(0.7));
            painter.hline(band.x_range(), band.top(), edge);
            painter.hline(band.x_range(), band.bottom(), edge);
            if band.height() >= 14.0 {
                painter.text(
                    band.left_top() + vec2(6.0, 3.0),
                    Align2::LEFT_TOP,
                    &row.name,
                    FontId::proportional(11.0),
                    theme::TRIM_ROW_EDGE,
                );
            }
        }
    }

    /// The colours of the numbered checker image shown, if one has been.
    #[cfg(test)]
    pub(crate) fn grid_colors(&self) -> Option<[[u8; 3]; 2]> {
        self.grid.as_ref().map(|(c, _)| *c)
    }

    /// The numbered checker image in `colors`, made again when they
    /// change.
    pub(super) fn grid_texture(
        &mut self,
        ctx: &egui::Context,
        colors: [[u8; 3]; 2],
    ) -> egui::TextureId {
        if let Some((made, handle)) = &self.grid
            && *made == colors
        {
            return handle.0.id();
        }
        let size = crate::viewport::GRID_IMAGE_SIZE;
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [size, size],
            &crate::viewport::grid_image(colors),
        );
        let handle = ctx.load_texture("numbered checker", image, egui::TextureOptions::LINEAR);
        let id = handle.id();
        self.grid = Some((colors, GridTexture(handle)));
        id
    }

    /// The texture square: a lighter tile (or the numbered checker,
    /// dimmed so the UVs stand out over it), a grid every tenth and its
    /// edges.
    pub(super) fn paint_square(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        texture_size: u32,
        grid: Option<egui::TextureId>,
    ) {
        let a = self.to_screen(rect, Vec2::ZERO);
        let b = self.to_screen(rect, Vec2::ONE);
        let square = Rect::from_two_pos(a, b);
        painter.rect_filled(square, 0.0, theme::UV_SQUARE);
        if let Some(image) = grid {
            // The image's first row is the top of the square.
            painter.image(
                image,
                square,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                theme::UV_GRID_IMAGE_TINT,
            );
        }
        for i in (1..10).filter(|_| grid.is_none()) {
            let t = i as f32 / 10.0;
            let color = if i == 5 {
                theme::UV_GRID.gamma_multiply(1.8)
            } else {
                theme::UV_GRID
            };
            let x = self.to_screen(rect, Vec2::new(t, 0.0)).x;
            let y = self.to_screen(rect, Vec2::new(0.0, t)).y;
            painter.vline(x, square.y_range(), Stroke::new(1.0, color));
            painter.hline(square.x_range(), y, Stroke::new(1.0, color));
        }
        self.paint_pixels(painter, rect, square, texture_size);
        painter.rect_stroke(
            square,
            0.0,
            Stroke::new(1.0, theme::UV_SQUARE_EDGE),
            egui::StrokeKind::Middle,
        );
        let label = |at: Pos2, text: &str, align| {
            painter.text(
                at,
                align,
                text,
                FontId::proportional(11.0),
                theme::TEXT_WEAK,
            );
        };
        label(
            square.left_bottom() + vec2(-4.0, 4.0),
            "0",
            Align2::RIGHT_TOP,
        );
        label(
            square.right_bottom() + vec2(0.0, 4.0),
            "1",
            Align2::CENTER_TOP,
        );
        label(
            square.left_top() + vec2(-4.0, 0.0),
            "1",
            Align2::RIGHT_CENTER,
        );
    }

    /// One object's UVs: selected and hovered polygons tinted, every edge,
    /// island borders brighter, and selected edges and points on top.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_shape(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        shape: &UvShape,
        selected: &ElementSelection,
        kind: Option<ElementKind>,
        hover: Option<Element>,
        fill: Color32,
        light: Color32,
        mode: Mode,
        stretch: bool,
        contrast: bool,
    ) {
        let screen = |uv: Vec2| self.to_screen(rect, uv);
        // Tinted polygons.
        let hovered_faces: BTreeSet<FaceId> = match (hover, mode) {
            (Some(Element::Polygon(f)), Mode::Uv) => shape
                .island_of
                .get(f.get() as usize)
                .and_then(|i| shape.islands.get(*i))
                .map(|faces| faces.iter().copied().collect())
                .unwrap_or_default(),
            (Some(Element::Polygon(f)), _) => [f].into_iter().collect(),
            _ => BTreeSet::new(),
        };
        let mut tint = egui::Mesh::default();
        // Locked pieces: faint stripes across them.
        let mut stripes = Vec::new();
        for ((_, points), locked) in shape.polygons.iter().zip(&shape.locked) {
            if *locked {
                let corners: Vec<Pos2> = points.iter().map(|(_, uv)| screen(*uv)).collect();
                add_polygon(&mut tint, &corners, theme::UV_LOCKED);
                stripes.push(corners);
            }
        }
        if stretch {
            for ((_, points), s) in shape.polygons.iter().zip(&shape.stretch) {
                let corners: Vec<Pos2> = points.iter().map(|(_, uv)| screen(*uv)).collect();
                add_polygon(&mut tint, &corners, stretch_color(s).gamma_multiply(0.55));
            }
        }
        if matches!(kind, Some(ElementKind::Polygons)) {
            for (face, points) in &shape.polygons {
                let color = if selected.contains(Element::Polygon(*face)) {
                    fill.gamma_multiply(0.55)
                } else if hovered_faces.contains(face) {
                    light.gamma_multiply(0.25)
                } else {
                    continue;
                };
                let corners: Vec<Pos2> = points.iter().map(|(_, uv)| screen(*uv)).collect();
                add_polygon(&mut tint, &corners, color);
            }
        }
        painter.add(egui::Shape::mesh(tint));
        for corners in &stripes {
            let (min, max) = corners.iter().fold(
                (
                    Pos2::new(f32::INFINITY, f32::INFINITY),
                    Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                ),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            );
            let clip = painter.with_clip_rect(Rect::from_min_max(min, max).intersect(rect));
            let mut x = min.x - (max.y - min.y);
            while x < max.x {
                clip.line_segment(
                    [Pos2::new(x, max.y), Pos2::new(x + (max.y - min.y), min.y)],
                    egui::Stroke::new(1.0, theme::UV_LOCKED.gamma_multiply(2.0)),
                );
                x += 8.0;
            }
        }
        // Every edge.
        let mut lines = egui::Mesh::default();
        let mut borders = egui::Mesh::default();
        let mut seams = egui::Mesh::default();
        for (uvs, key, border) in &shape.edges {
            let (a, b) = (screen(uvs[0]), screen(uvs[1]));
            if !rect.expand(2.0).intersects(Rect::from_two_pos(a, b)) {
                continue;
            }
            if shape.seams.contains(key) {
                add_line(&mut seams, a, b, 2.0, theme::UV_SEAM);
            } else if let Some(preference) = shape.preferences.get(key) {
                match preference {
                    scene::SeamPreference::Prefer => {
                        add_dashes(&mut seams, a, b, 2.0, theme::UV_SEAM_PREFER);
                    }
                    scene::SeamPreference::Never => {
                        add_line(&mut seams, a, b, 2.0, theme::UV_SEAM_NEVER);
                    }
                    scene::SeamPreference::Free => {
                        add_line(&mut lines, a, b, 1.0, theme::UV_EDGE);
                    }
                }
            } else if *border {
                add_line(&mut borders, a, b, 1.6, theme::UV_BORDER);
            } else {
                add_line(&mut lines, a, b, 1.0, theme::UV_EDGE);
            }
        }
        painter.add(egui::Shape::mesh(lines));
        painter.add(egui::Shape::mesh(borders));
        painter.add(egui::Shape::mesh(seams));
        // Selected and hovered edges and points: thicker than seams and
        // borders, the selection drawn last so it lies on top, with a dark
        // rim under High contrast (decision 0126).
        let mut marks = egui::Mesh::default();
        let grow = if contrast { 1.5 } else { 1.0 };
        let rim = Color32::from_black_alpha(230);
        match kind {
            Some(ElementKind::Edges) => {
                // The hovered edge first, then the selected ones on top.
                for pass in [false, true] {
                    for (uvs, key, _) in &shape.edges {
                        let is_selected = selected.contains(Element::Edge(*key));
                        let color = if pass && is_selected {
                            fill
                        } else if !pass && !is_selected && hover == Some(Element::Edge(*key)) {
                            light
                        } else {
                            continue;
                        };
                        let (a, b) = (screen(uvs[0]), screen(uvs[1]));
                        let width = UV_SELECTED_WIDTH * grow;
                        if contrast {
                            add_line(&mut marks, a, b, width + 4.0, rim);
                        }
                        add_line(&mut marks, a, b, width, color);
                    }
                }
            }
            Some(ElementKind::Points) => {
                for pass in 0..3 {
                    for (_, points) in &shape.polygons {
                        for (v, uv) in points {
                            let is_selected = selected.contains(Element::Point(*v));
                            let is_hovered = hover == Some(Element::Point(*v));
                            let (radius, color) = match pass {
                                0 if !is_selected && !is_hovered => (2.0, theme::UV_EDGE),
                                1 if is_hovered && !is_selected => (4.5 * grow, light),
                                2 if is_selected => (UV_SELECTED_RADIUS * grow, fill),
                                _ => continue,
                            };
                            if contrast && pass > 0 {
                                add_dot(&mut marks, screen(*uv), radius + 2.0, rim);
                            }
                            add_dot(&mut marks, screen(*uv), radius, color);
                        }
                    }
                }
            }
            _ => {}
        }
        // Pinned UVs, in every mode: a dark ring with a bright middle.
        for pin in &shape.pins {
            let at = screen(*pin);
            if rect.expand(6.0).contains(at) {
                add_dot(&mut marks, at, 5.0, theme::PANEL);
                add_dot(&mut marks, at, 3.5, theme::UV_PIN);
            }
        }
        painter.add(egui::Shape::mesh(marks));
    }
}

/// The colour of a polygon in the stretch view.
pub(super) fn stretch_color(s: &uv::stretch::Stretch) -> Color32 {
    let off = s.off();
    if !off.is_finite() {
        return theme::STRETCH_NO_ROOM;
    }
    let half = uv::stretch::SOME.log2();
    let mix = |a: Color32, b: Color32, t: f32| {
        let t = t.clamp(0.0, 1.0);
        let c = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
        Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
    };
    if off < half {
        mix(theme::STRETCH_EVEN, theme::STRETCH_SOME, off / half)
    } else {
        mix(
            theme::STRETCH_SOME,
            theme::STRETCH_FAR,
            (off - half) / (1.0 - half),
        )
    }
}

/// The stretch view's note: what is wrong with the polygon under the
/// mouse, or how many polygons are off.
pub(super) fn stretch_note(shapes: &[(ObjectId, UvShape)], hover: Option<Hit>) -> Option<String> {
    if let Some(Hit {
        id,
        element: Element::Polygon(face),
    }) = hover
    {
        let (_, shape) = shapes.iter().find(|(i, _)| *i == id)?;
        let at = shape.polygons.iter().position(|(f, _)| *f == face)?;
        let s = shape.stretch.get(at)?;
        return Some(match s.explain() {
            Some(why) => crate::trf!("This polygon {}", why),
            None => tr("This polygon lies evenly on the texture.").to_string(),
        });
    }
    let off = shapes
        .iter()
        .flat_map(|(_, s)| &s.stretch)
        .filter(|s| s.off() >= uv::stretch::SOME.log2())
        .count();
    Some(match off {
        0 => tr("Stretch: every polygon lies evenly.").to_string(),
        1 => tr("Stretch: 1 polygon is 1.5 times off or more. Point at it to see why.").to_string(),
        n => crate::trf!(
            "Stretch: {} polygons are 1.5 times off or more. Point at one to see why.",
            n
        ),
    })
}

/// Adds a line of `width` to `mesh`, as a thin quad.
pub(super) fn add_line(mesh: &mut egui::Mesh, a: Pos2, b: Pos2, width: f32, color: Color32) {
    let along = b - a;
    let length = along.length();
    if length < 1e-3 {
        return;
    }
    let side = vec2(-along.y, along.x) / length * (width / 2.0);
    let i = mesh.vertices.len() as u32;
    for p in [a + side, a - side, b - side, b + side] {
        mesh.colored_vertex(p, color);
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

/// Adds a round dot to `mesh`.
/// Adds a dashed line from `a` to `b` to `mesh`: dashes of 6 points with
/// gaps of 4.
fn add_dashes(mesh: &mut egui::Mesh, a: Pos2, b: Pos2, width: f32, color: Color32) {
    let length = a.distance(b);
    if length < 1e-3 {
        return;
    }
    let way = (b - a) / length;
    let mut at = 0.0;
    while at < length {
        let end = (at + 6.0).min(length);
        add_line(mesh, a + way * at, a + way * end, width, color);
        at += 10.0;
    }
}

fn add_dot(mesh: &mut egui::Mesh, at: Pos2, radius: f32, color: Color32) {
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(at, color);
    const STEPS: u32 = 10;
    for k in 0..STEPS {
        let a = k as f32 / STEPS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(at + vec2(a.cos(), a.sin()) * radius, color);
    }
    for k in 0..STEPS {
        mesh.add_triangle(i, i + 1 + k, i + 1 + (k + 1) % STEPS);
    }
}

/// Adds a filled polygon to `mesh` (cut into triangles, so any shape
/// fills right).
pub(super) fn add_polygon(mesh: &mut egui::Mesh, corners: &[Pos2], color: Color32) {
    if corners.len() < 3 {
        return;
    }
    let i = mesh.vertices.len() as u32;
    for p in corners {
        mesh.colored_vertex(*p, color);
    }
    mesh::triangulate::triangulate(
        corners.len(),
        |k| glam::Vec3::new(corners[k].x, corners[k].y, 0.0),
        |[a, b, c]| mesh.add_triangle(i + a as u32, i + b as u32, i + c as u32),
    );
}
