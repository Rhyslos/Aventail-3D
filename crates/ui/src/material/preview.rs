//! The preview beside the canvas (decision 0258): the graph's material
//! (what Material out is given) on a ball, or flat and tiled so its seams
//! can be seen; or, after a click on a box's picture, what that box makes
//! ("looking through" it). Drawn on the processor from the pictures the
//! canvas already has, so it costs nothing extra to work out.

use super::MaterialEditor;
use super::pictures::{byte, colour_at, colour_image, picture_key};
use crate::text::tr;
use crate::theme;
use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Ui};
use glam::Vec3;
use matgraph::{BoxId, Graph, Registry, Value};
use scene::Document;

/// How many pixels across the ball is drawn.
const BALL: u32 = 256;
/// How many times the flat preview repeats the texture each way.
const TILES: f32 = 2.0;

/// What the preview shows the picture on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PreviewShape {
    /// A lit ball.
    #[default]
    Ball,
    /// A flat square, tiled.
    Flat,
}

/// The preview's own state.
#[derive(Default)]
pub(crate) struct Preview {
    /// What it shows the picture on.
    pub(crate) shape: PreviewShape,
    /// The picture drawn last, with what it was drawn from.
    drawn: Option<(usize, PreviewShape, TextureHandle)>,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("shape", &self.shape)
            .finish_non_exhaustive()
    }
}

/// The box the preview shows: the one looked through, else Material
/// out, else the one selected.
pub(crate) fn shown_box(
    graph: &Graph,
    looking_at: Option<BoxId>,
    selected: &[BoxId],
) -> Option<BoxId> {
    looking_at
        .filter(|id| graph.get(*id).is_some())
        .or_else(|| graph.first_of(matgraph::eval::MATERIAL_OUT))
        .or(match selected {
            [one] => Some(*one),
            _ => None,
        })
}

impl MaterialEditor {
    /// The Preview panel.
    pub(crate) fn preview_tab(&mut self, ui: &mut Ui, document: &Document) {
        crate::dialog::segmented(
            ui,
            &mut self.preview.shape,
            &[
                (
                    PreviewShape::Ball,
                    tr("Ball"),
                    tr("The material on a lit ball"),
                ),
                (
                    PreviewShape::Flat,
                    tr("Flat"),
                    tr("The texture flat, repeated twice each way, so you can see that it tiles"),
                ),
            ],
        );
        ui.add_space(6.0);
        let Some(graph) = self.current(document).cloned() else {
            crate::dialog::help(ui, tr("Make a material graph to see it here."));
            return;
        };
        let selected: Vec<BoxId> = self.canvas.selected.iter().copied().collect();
        let Some(id) = shown_box(&graph, self.canvas.looking_at, &selected) else {
            crate::dialog::help(ui, tr("Add boxes to the graph to see what they make here."));
            return;
        };
        let name = graph
            .get(id)
            .and_then(|b| Registry::standard().get(&b.kind))
            .map_or_else(String::new, |t| t.info().name.show());
        let side = ui
            .available_width()
            .min(ui.available_height() - 60.0)
            .max(64.0);
        match self.pictures.value(id).cloned() {
            Some(value) => {
                let texture = self.preview_texture(ui.ctx(), &value);
                let rect = square(ui, side);
                let uv = match self.preview.shape {
                    PreviewShape::Ball => {
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
                    }
                    PreviewShape::Flat => {
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(TILES, TILES))
                    }
                };
                ui.painter().image(texture, rect, uv, Color32::WHITE);
            }
            None => {
                let rect = square(ui, side);
                ui.painter().rect_filled(rect, 4.0, theme::FIELD);
                let why = self
                    .problems(&graph)
                    .remove(&id)
                    .map_or_else(|| tr("Working it out…").to_string(), |r| r.reason);
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(16.0)));
                crate::dialog::help(&mut inner, why);
            }
        }
        ui.add_space(4.0);
        if self.canvas.looking_at.is_some_and(|l| l == id) {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "Looking through {}: what it makes, on its own. Click its picture again to go \
                     back to the material.",
                    name
                ),
            );
            if ui.button(tr("Back to the material")).clicked() {
                self.canvas.looking_at = None;
            }
        } else {
            crate::dialog::help(
                ui,
                crate::trf!(
                    "Showing {}. Click a box's picture to look through that box.",
                    name
                ),
            );
        }
    }

    /// The preview's picture of `value`, drawn again only when it or the
    /// shape changed.
    fn preview_texture(&mut self, ctx: &egui::Context, value: &Value) -> egui::TextureId {
        let key = picture_key(value);
        let shape = self.preview.shape;
        if let Some((k, s, handle)) = &self.preview.drawn
            && *k == key
            && *s == shape
        {
            return handle.id();
        }
        let (image, options) = match shape {
            PreviewShape::Ball => (ball(value, BALL), TextureOptions::LINEAR),
            PreviewShape::Flat => (colour_image(value, 512), TextureOptions::LINEAR_REPEAT),
        };
        let handle = ctx.load_texture("material preview", image, options);
        let id = handle.id();
        self.preview.drawn = Some((key, shape, handle));
        id
    }
}

/// A square `side` across, in the middle of the panel's width.
fn square(ui: &mut Ui, side: f32) -> egui::Rect {
    let (row, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::hover());
    egui::Rect::from_center_size(row.center(), egui::vec2(side, side))
}

/// `value` on a lit ball `side` pixels across: wrapped round it twice, lit
/// from the top left, its bumps from the material's normal, shinier where
/// it is smooth.
pub(crate) fn ball(value: &Value, side: u32) -> ColorImage {
    let size = value.size().max(1) as f32;
    let light = Vec3::new(-0.5, 0.6, 0.7).normalize();
    let half = (light + Vec3::Z).normalize();
    let mut pixels = Vec::with_capacity((side * side) as usize);
    for y in 0..side {
        for x in 0..side {
            let nx = (x as f32 + 0.5) / side as f32 * 2.0 - 1.0;
            let ny = 1.0 - (y as f32 + 0.5) / side as f32 * 2.0;
            let r2 = nx * nx + ny * ny;
            if r2 > 1.0 {
                pixels.push(Color32::TRANSPARENT);
                continue;
            }
            let n = Vec3::new(nx, ny, (1.0 - r2).sqrt());
            let u = 0.5 + n.x.atan2(n.z) / std::f32::consts::TAU;
            let v = n.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
            let (px, py) = ((u * TILES * size) as i64, (v * size) as i64);
            let (colour, normal, rough, metal, open) = match value {
                Value::Material(m) => (
                    Vec3::from(m.colour.at(px, py)),
                    Vec3::from(m.normal.at(px, py)),
                    m.roughness.at(px, py),
                    m.metal.at(px, py),
                    m.occlusion.at(px, py),
                ),
                other => (Vec3::from(colour_at(other, px, py)), Vec3::Z, 0.6, 0.0, 1.0),
            };
            let tangent = Vec3::new(n.z, 0.0, -n.x).normalize_or(Vec3::X);
            let up = n.cross(tangent);
            let bent = (tangent * normal.x + up * normal.y + n * normal.z).normalize_or(n);
            let diffuse = bent.dot(light).max(0.0);
            let power = 4.0 + (1.0 - rough) * 60.0;
            let shine = bent.dot(half).max(0.0).powf(power) * (1.0 - rough) * 0.6;
            let base = colour * (0.22 * open + 0.85 * diffuse) * (1.0 - 0.5 * metal);
            let tint = Vec3::ONE.lerp(colour, metal);
            let out = base + tint * shine;
            // Smooth the ball's rim over a pixel.
            let edge = ((1.0 - r2.sqrt()) * side as f32 / 2.0).clamp(0.0, 1.0);
            let alpha = byte(edge);
            pixels.push(Color32::from_rgba_unmultiplied(
                byte(out.x),
                byte(out.y),
                byte(out.z),
                alpha,
            ));
        }
    }
    ColorImage::new([side as usize, side as usize], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matgraph::{Size, ValueKind};

    #[test]
    fn the_ball_is_round_and_lit_from_the_top_left() {
        let value = Value::flat_grey(ValueKind::Height, Size::nearest(64), 1.0).unwrap();
        let image = ball(&value, 64);
        let at = |x: usize, y: usize| image.pixels[y * 64 + x];
        assert_eq!(at(0, 0), Color32::TRANSPARENT, "corners are outside");
        let top_left = at(20, 20).r();
        let bottom_right = at(44, 44).r();
        assert!(top_left > bottom_right, "{top_left} > {bottom_right}");
    }

    #[test]
    fn the_preview_shows_the_box_looked_through_first() {
        let registry = Registry::standard();
        let mut g = Graph::new("Stone");
        for kind in ["noise", "material_out"] {
            g.apply(
                registry,
                &matgraph::GraphEdit::AddBox {
                    kind: kind.into(),
                    at: glam::Vec2::ZERO,
                },
            )
            .unwrap();
        }
        assert_eq!(shown_box(&g, None, &[]), Some(BoxId(2)), "the material");
        assert_eq!(shown_box(&g, Some(BoxId(1)), &[]), Some(BoxId(1)));
        assert_eq!(shown_box(&g, Some(BoxId(9)), &[]), Some(BoxId(2)), "gone");
        let empty = Graph::new("Empty");
        assert_eq!(shown_box(&empty, None, &[BoxId(3)]), Some(BoxId(3)));
        assert_eq!(shown_box(&empty, None, &[]), None);
    }
}
