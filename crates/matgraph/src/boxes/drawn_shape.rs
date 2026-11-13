//! Shape: whatever is drawn on its shape canvas (milestone 11, decision
//! 0273), as a sharp mask, a rounded height, its outline and its pieces'
//! colours (milestone 12, decision 0274).
//!
//! The drawing is vectors (`shapes`), filled at the size asked for, so it
//! is never blurred by being made larger. Pieces crossing an edge of the
//! texture come back on the other side, so it tiles like every other
//! box. The rounded height rises with the exact distance in from the
//! outline (as Bevel's does), up to Puff, times each piece's own height
//! (the top piece where pieces overlap); the outline is a band Outline
//! wide along the edge, with the drawing's lines on it; the colours are
//! each piece's colour laid in order over black.
//!
//! Milestone 13 (decision 0275): pieces' edges, varied copies and the
//! mirror line are already in the drawing's paths; Line pieces that are
//! veins sink or raise the height along them and darken or lighten the
//! colours, only where the shape is.

use super::pattern::distance_inside;
use super::{info, length, number};
use crate::image::{Grey, Rgb, Size};
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
                     shape puffed up by Puff, each piece as high as you set it), its outline, \
                     and its colours (each piece in its own colour, over black). Build an \
                     assembly from many pieces: a twig of linked leaves, say. Give a shape a \
                     jagged, serrated, wavy or torn edge, let every copy differ, and draw lines \
                     for veins. A shape crossing the texture's edge comes back on the other \
                     side, so it tiles."
                ),
                (
                    Vec::new(),
                    vec![
                        PlugSpec::new("mask", text!("Mask"), ValueKind::Shape, false),
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("outline", text!("Outline"), ValueKind::Shape, false),
                        PlugSpec::new("colours", text!("Colours"), ValueKind::Colour, false),
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
                &[
                    "material_from_height",
                    "bevel",
                    "blend",
                    "scatter",
                    "gradient_map",
                ],
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
        let made = drawn(
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
            Value::Shape(Arc::new(made.mask)),
            Value::Height(Arc::new(made.height)),
            Value::Shape(Arc::new(made.outline)),
            Value::Colour(Arc::new(made.colours)),
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

/// What a drawing gives: the Shape box's four outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    /// The filled shape.
    pub mask: Grey,
    /// The rounded height, each piece as high as it is set.
    pub height: Grey,
    /// The outline and the drawn lines.
    pub outline: Grey,
    /// Each piece's colour laid in order over black.
    pub colours: Rgb,
}

/// `drawing` at `size`: its mask, its rounded height, its outline and its
/// colours (the Shape box's outputs; the shape canvas's strip shows them
/// too).
pub fn drawn(drawing: &shapes::Drawing, size: Size, look: Look, threads: usize) -> Drawn {
    let n = size.get();
    let layers = shapes::raster::layers(drawing, n);
    let mask = Grey::from_pixels(size, layers.mask).unwrap_or_else(|| Grey::filled(size, 0.0));
    let heights =
        Grey::from_pixels(size, layers.heights).unwrap_or_else(|| Grey::filled(size, 0.0));
    let colours =
        Rgb::from_pixels(size, layers.colours).unwrap_or_else(|| Rgb::filled(size, [0.0; 3]));
    let veins = shapes::raster::veins(drawing, n);
    let colours = shaded(colours, size, &mask, &veins);
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
        // The pieces' heights laid over nothing are already their cover
        // times their height.
        let raised = heights.at(x, y);
        let rise = if puff < 0.5 {
            1.0
        } else {
            let t = ((inside.at(x, y) + 0.5) / puff).clamp(0.0, 1.0);
            (1.0 - (1.0 - t) * (1.0 - t)).sqrt()
        };
        let vein = veins
            .raise
            .get((y * i64::from(n) + x) as usize)
            .map_or(0.0, |r| r * mask.at(x, y));
        ((raised * rise + vein) * look.top).clamp(0.0, 1.0)
    });
    let outline = parallel::fill(size, threads, |x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        // How far from the edge, in or out (the edge runs between
        // pixels, half a pixel from each side).
        let d = inside.at(x, y).max(outside.at(x, y));
        let band = (half + 0.5 - d).clamp(0.0, 1.0);
        band.max(lines.at(x, y))
    });
    Drawn {
        mask,
        height,
        outline,
        colours,
    }
}

/// `colours` darkened or lightened along the veins, where the shape is.
fn shaded(colours: Rgb, size: Size, mask: &Grey, veins: &shapes::raster::Veins) -> Rgb {
    if veins.is_empty() {
        return colours;
    }
    let n = i64::from(size.get());
    let pixels = colours
        .pixels()
        .iter()
        .zip(&veins.shade)
        .enumerate()
        .map(|(i, (rgb, shade))| {
            let i = i as i64;
            let s = shade * mask.at(i % n, i / n);
            rgb.map(|c| {
                if s < 0.0 {
                    c * (1.0 + s)
                } else {
                    c + (1.0 - c) * s
                }
            })
        })
        .collect();
    Rgb::from_pixels(size, pixels).unwrap_or(colours)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use crate::{Evaluator, Graph, GraphEdit};
    use glam::Vec2;
    use shapes::{Basic, Drawing, PieceId, PieceKind};

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
        let Drawn {
            mask,
            height,
            outline,
            colours,
        } = drawn(&leaf(), size, look(), 2);
        // White pieces: the colours are the mask.
        assert_eq!(colours.at(32, 32), [1.0; 3]);
        assert_eq!(colours.at(2, 2), [0.0; 3]);
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
        let h = drawn(&leaf(), size, flat, 1).height;
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
        assert_eq!(out.outputs.len(), 4);
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
            assert!(made.mask.at(n - 1, 2) > 0.5 && made.mask.at(1, 2) > 0.5);
            let values = [
                Value::Shape(Arc::new(made.mask)),
                Value::Height(Arc::new(made.height)),
                Value::Shape(Arc::new(made.outline)),
                Value::Colour(Arc::new(made.colours)),
            ];
            for (i, value) in values.iter().enumerate() {
                let check = crate::SeamCheck::of(value);
                assert!(!check.shows(), "{side} output {i}: {}", check.describe());
            }
        }
    }

    #[test]
    fn each_piece_gives_its_own_colour_and_height() {
        // A leaf and a linked copy of it moved right, coloured and lower.
        let d = Drawing::new()
            .with_piece(
                "Leaf",
                Basic::Circle.path(Vec2::new(0.25, 0.5), 0.15),
                PieceKind::Add,
            )
            .unwrap();
        let first = d.pieces()[0].id;
        let (d, copy) = d
            .with_pasted(
                &d.copied(&[first]),
                true,
                &shapes::Change::moved(Vec2::new(0.5, 0.0)),
            )
            .unwrap();
        let d = d
            .with_colour(first, [0.2, 0.6, 0.1])
            .unwrap()
            .with_colour(copy[0], [0.8, 0.3, 0.1])
            .unwrap()
            .with_height(copy[0], 0.5)
            .unwrap();
        let made = drawn(&d, Size::nearest(64), look(), 2);
        assert_eq!(made.colours.at(16, 32), [0.2, 0.6, 0.1]);
        assert_eq!(made.colours.at(48, 32), [0.8, 0.3, 0.1]);
        assert!((made.height.at(16, 32) - 1.0).abs() < 1e-5);
        assert!((made.height.at(48, 32) - 0.5).abs() < 1e-5);
        // The two masks match: the copy shares the leaf's shape.
        assert!((made.mask.at(16, 32) - made.mask.at(48, 32)).abs() < 1e-6);
        assert!((made.mask.at(7, 32) - made.mask.at(39, 32)).abs() < 1e-3);
    }

    #[test]
    fn veins_groove_the_height_and_darken_the_colours_only_on_the_shape() {
        let vein = shapes::Path::new(
            vec![
                shapes::Knot::corner(Vec2::new(0.5, 0.1)),
                shapes::Knot::corner(Vec2::new(0.5, 0.9)),
            ],
            false,
        );
        let d = Drawing::new()
            .with_piece(
                "Leaf",
                Basic::Circle.path(Vec2::splat(0.5), 0.25),
                PieceKind::Add,
            )
            .unwrap()
            .with_colour(PieceId(1), [0.2, 0.6, 0.1])
            .unwrap()
            .with_piece("Vein", vein, PieceKind::Line)
            .unwrap();
        let size = Size::nearest(256);
        let made = drawn(&d, size, look(), 2);
        let plain = drawn(
            &d.with_vein(PieceId(2), shapes::Vein::NONE).unwrap(),
            size,
            look(),
            2,
        );
        // On the vein, inside the leaf: lower and darker.
        assert!(made.height.at(128, 128) < plain.height.at(128, 128) - 0.2);
        assert!(made.colours.at(128, 128)[1] < plain.colours.at(128, 128)[1] - 0.1);
        // Beside it: as before; outside the leaf the line changes nothing.
        assert_eq!(made.height.at(100, 128), plain.height.at(100, 128));
        assert_eq!(made.height.at(128, 20), 0.0);
        assert_eq!(made.colours.at(128, 20), [0.0; 3]);
        // A ridge raises it instead.
        let ridge = shapes::Vein {
            depth: 0.5,
            shade: 0.5,
            ..shapes::Vein::STANDARD
        };
        let raised = drawn(&d.with_vein(PieceId(2), ridge).unwrap(), size, look(), 2);
        let lower = Look { top: 0.5, ..look() };
        let raised_low = drawn(&d.with_vein(PieceId(2), ridge).unwrap(), size, lower, 2);
        assert!(raised_low.height.at(128, 128) > 0.5 + 0.1);
        assert!(raised.colours.at(128, 128)[0] > plain.colours.at(128, 128)[0] + 0.1);
    }

    #[test]
    fn rough_edges_and_varied_copies_tile_in_every_output() {
        let d = Drawing::new()
            .with_piece(
                "Star",
                Basic::Star(5).path(Vec2::splat(0.05), 0.3),
                PieceKind::Add,
            )
            .unwrap();
        let edge = shapes::Edge {
            kind: shapes::EdgeKind::Torn,
            amount: 1.0,
            size: 0.03,
            seed: 3,
        };
        let vary = shapes::Vary {
            on: true,
            ..shapes::Vary::OFF
        };
        let d = d
            .with_edge(PieceId(1), edge)
            .unwrap()
            .with_vary(PieceId(1), vary)
            .unwrap();
        for side in [256, 512] {
            let made = drawn(&d, Size::nearest(side), look(), 2);
            for value in [
                Value::Shape(Arc::new(made.mask)),
                Value::Height(Arc::new(made.height)),
                Value::Colour(Arc::new(made.colours)),
            ] {
                let check = crate::SeamCheck::of(&value);
                assert!(
                    !check.shows(),
                    "{side} {:?}: {}",
                    value.kind(),
                    check.describe()
                );
            }
        }
        // Moved half a texture, it comes out the same, half a texture
        // round: nothing is lost or doubled where it crosses the edges.
        let moved = d
            .with_change(&[PieceId(1)], &shapes::Change::moved(Vec2::splat(0.5)))
            .unwrap();
        let size = Size::nearest(64);
        let (a, b) = (drawn(&d, size, look(), 2), drawn(&moved, size, look(), 2));
        for y in 0..64 {
            for x in 0..64 {
                let (u, v) = ((x + 32) % 64, (y + 32) % 64);
                assert!((a.mask.at(x, y) - b.mask.at(u, v)).abs() < 0.05, "{x} {y}");
                assert!(
                    (a.height.at(x, y) - b.height.at(u, v)).abs() < 0.05,
                    "{x} {y}"
                );
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
