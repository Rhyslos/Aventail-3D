//! Shape: whatever is drawn on its shape canvas (milestone 11, decision
//! 0273), as a sharp mask, a rounded height and its outline.
//!
//! The drawing is vectors (`shapes`), filled at the size asked for, so it
//! is never blurred by being made larger. Pieces crossing an edge of the
//! texture come back on the other side, so it tiles like every other
//! box. The rounded height rises with the exact distance in from the
//! outline (as Bevel's does), up to Puff; the outline is a band Outline
//! wide along the edge, with the drawing's lines on it.

use super::pattern::distance_inside;
use super::{info, length, number};
use crate::image::{Grey, Size};
use crate::kind::ValueKind;
use crate::parallel;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Shape box's kind id.
pub const DRAWN_SHAPE: &str = "drawn_shape";

/// The Shape box.
#[derive(Debug)]
pub(crate) struct DrawnShape {
    info: BoxInfo,
}

impl DrawnShape {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                DRAWN_SHAPE,
                text!("Shape"),
                Group::Make,
                text!(
                    "A shape you draw yourself on its own canvas: a leaf, a blade, a rune. Open \
                     the canvas (double-click the box) and draw with the pen or start from a \
                     basic shape, cut pieces out, and mirror one half to make the other. It \
                     gives the shape as a sharp mask at any texture size, a rounded height (the \
                     shape puffed up by Puff), and its outline. A shape crossing the texture's \
                     edge comes back on the other side, so it tiles."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("mask", text!("Mask"), ValueKind::Shape, false),
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("outline", text!("Outline"), ValueKind::Shape, false),
                    ],
                ),
                vec![
                    length(
                        "puff",
                        text!("Puff"),
                        text!(
                            "How far in from the outline the shape rises to its full height, in \
                             centimetres: more makes it rounder."
                        ),
                        0.0,
                        50.0,
                        3.0,
                    ),
                    number(
                        "height",
                        text!("Height"),
                        text!("How high the puffed-up shape is: 1 is white."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    length(
                        "outline",
                        text!("Outline"),
                        text!("How wide the outline and the drawn lines are, in centimetres."),
                        0.05,
                        20.0,
                        0.5,
                    ),
                ],
                &["material_from_height", "bevel", "blend", "scatter"],
            ),
        }
    }
}

impl BoxType for DrawnShape {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn takes_drawing(&self) -> bool {
        true
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let empty = shapes::Drawing::new();
        let drawing = job.drawing().map_or(&empty, |d| d);
        let [mask, height, outline] = drawn(
            drawing,
            job.size(),
            Look {
                puff: job.length_px("puff"),
                top: job.number("height").clamp(0.0, 1.0),
                outline: job.length_px("outline").max(0.5),
            },
            job.threads(),
        );
        Ok(vec![
            Value::Shape(Arc::new(mask)),
            Value::Height(Arc::new(height)),
            Value::Shape(Arc::new(outline)),
        ])
    }
}

/// How a drawing is made into pictures, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// How far in from the outline the height rises to the top.
    pub puff: f32,
    /// The top's height, 0 to 1.
    pub top: f32,
    /// How wide the outline and lines are.
    pub outline: f32,
}

/// `drawing` at `size`: its mask, its rounded height and its outline (the
/// Shape box's three outputs; the shape canvas's strip shows them too).
pub fn drawn(drawing: &shapes::Drawing, size: Size, look: Look, threads: usize) -> [Grey; 3] {
    let n = size.get();
    let pixels = shapes::raster::mask(drawing, n);
    let mask = Grey::from_pixels(size, pixels).unwrap_or_else(|| Grey::filled(size, 0.0));
    let lines = Grey::from_pixels(size, shapes::raster::lines(drawing, n, look.outline))
        .unwrap_or_else(|| Grey::filled(size, 0.0));
    let puff = if look.puff.is_finite() {
        look.puff.max(0.0)
    } else {
        0.0
    };
    let half = (look.outline * 0.5).max(0.5);
    let far = puff.max(half) + 2.0;
    let inside = distance_inside(&mask, far, threads);
    let outside = distance_inside(&mask.map(|v| 1.0 - v), half + 2.0, threads);
    let height = parallel::fill(size, threads, |x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        let cover = mask.at(x, y);
        let rise = if puff < 0.5 {
            1.0
        } else {
            let t = ((inside.at(x, y) + 0.5) / puff).clamp(0.0, 1.0);
            (1.0 - (1.0 - t) * (1.0 - t)).sqrt()
        };
        cover * rise * look.top
    });
    let outline = parallel::fill(size, threads, |x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        // How far from the edge, in or out (the edge runs between
        // pixels, half a pixel from each side).
        let d = inside.at(x, y).max(outside.at(x, y));
        let band = (half + 0.5 - d).clamp(0.0, 1.0);
        band.max(lines.at(x, y))
    });
    [mask, height, outline]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use crate::{Evaluator, Graph, GraphEdit};
    use glam::Vec2;
    use shapes::{Basic, Drawing, PieceKind};

    fn leaf() -> Drawing {
        Drawing::new()
            .with_piece(
                "Leaf",
                Basic::Circle.path(Vec2::splat(0.5), 0.3),
                PieceKind::Add,
            )
            .unwrap()
            .with_piece(
                "Bite",
                Basic::Circle.path(Vec2::new(0.8, 0.5), 0.1),
                PieceKind::Cut,
            )
            .unwrap()
    }

    fn look() -> Look {
        Look {
            puff: 6.0,
            top: 1.0,
            outline: 2.0,
        }
    }

    #[test]
    fn the_height_rises_in_from_the_edge_and_the_outline_follows_it() {
        let size = Size::nearest(64);
        let [mask, height, outline] = drawn(&leaf(), size, look(), 2);
        // Middle: full height, no outline; outside: nothing.
        assert_eq!(mask.at(32, 32), 1.0);
        assert!((height.at(32, 32) - 1.0).abs() < 1e-6);
        assert_eq!(outline.at(32, 32), 0.0);
        assert_eq!(height.at(2, 2), 0.0);
        assert_eq!(outline.at(2, 2), 0.0);
        // Near the edge: lower, and on the outline.
        let edge_x = (0.2 * 64.0) as i64 + 1;
        assert!(height.at(edge_x, 32) < 0.8);
        assert!(outline.at(edge_x, 32) > 0.5);
        // The bite is cut out of all three.
        assert_eq!(mask.at(52, 32), 0.0);
        assert_eq!(height.at(52, 32), 0.0);
        // No puff: flat top.
        let flat = Look {
            puff: 0.0,
            ..look()
        };
        let [_, h, _] = drawn(&leaf(), size, flat, 1);
        assert_eq!(h.at(edge_x, 32), mask.at(edge_x, 32));
    }

    #[test]
    fn the_box_runs_its_drawing_and_works_out_again_only_when_it_changes() {
        let registry = Registry::standard();
        let mut g = Graph::new("Leaf");
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: DRAWN_SHAPE.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        let id = crate::BoxId(1);
        let mut evaluator = Evaluator::with_threads(1);
        let size = Size::nearest(32);
        // Nothing drawn: black, no refusal.
        let empty = evaluator.run(registry, &g, size, &[id]);
        let out = empty.outcome(id).unwrap().as_ref().unwrap();
        assert_eq!(out.outputs.len(), 3);
        assert!(
            out.outputs[0]
                .as_ref()
                .unwrap()
                .grey()
                .unwrap()
                .pixels()
                .iter()
                .all(|v| *v == 0.0)
        );
        g.apply(
            registry,
            &GraphEdit::SetDrawing {
                node: id,
                drawing: Arc::new(leaf()),
            },
        )
        .unwrap();
        let drawn_now = evaluator.run(registry, &g, size, &[id]);
        assert_eq!(drawn_now.made(), 1);
        let mask = drawn_now.outcome(id).unwrap().as_ref().unwrap().outputs[0].clone();
        assert!(
            mask.unwrap()
                .grey()
                .unwrap()
                .pixels()
                .iter()
                .any(|v| *v > 0.5)
        );
        // A reference image changes nothing worked out.
        let r = shapes::Reference::new(1, 1, vec![0; 4], "r").unwrap();
        let with_ref = leaf().with_reference(Some(Arc::new(r)));
        g.apply(
            registry,
            &GraphEdit::SetDrawing {
                node: id,
                drawing: Arc::new(with_ref),
            },
        )
        .unwrap();
        assert_eq!(evaluator.run(registry, &g, size, &[id]).made(), 0);
    }

    #[test]
    fn a_shape_over_the_edge_tiles_in_every_output() {
        // A star across the top left corner, at several sizes.
        let d = Drawing::new()
            .with_piece(
                "Star",
                Basic::Star(5).path(Vec2::splat(0.05), 0.3),
                PieceKind::Add,
            )
            .unwrap();
        for side in [32, 128, 512] {
            let size = Size::nearest(side);
            let made = drawn(&d, size, look(), 2);
            let n = i64::from(side);
            // Something on both sides of each edge.
            assert!(made[0].at(n - 1, 2) > 0.5 && made[0].at(1, 2) > 0.5);
            for (i, g) in made.into_iter().enumerate() {
                let value = if i == 1 {
                    Value::Height(Arc::new(g))
                } else {
                    Value::Shape(Arc::new(g))
                };
                let check = crate::SeamCheck::of(&value);
                assert!(!check.shows(), "{side} output {i}: {}", check.describe());
            }
        }
    }

    #[test]
    fn only_a_shape_box_keeps_a_drawing() {
        let registry = Registry::standard();
        let mut g = Graph::new("Not drawn");
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        let refused = g.apply(
            registry,
            &GraphEdit::SetDrawing {
                node: crate::BoxId(1),
                drawing: Arc::new(leaf()),
            },
        );
        assert!(refused.unwrap_err().reason.contains("Shape box"));
        // From a file: a drawing on another kind of box is dropped.
        let mut parts = g.to_parts();
        parts.boxes[0].1.drawing = Some(Arc::new(leaf()));
        let back = Graph::from_parts(parts, registry).unwrap();
        assert!(back.get(crate::BoxId(1)).unwrap().drawing.is_none());
    }
}
