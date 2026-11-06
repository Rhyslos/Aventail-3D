//! Basic shape: a circle, square, polygon or star, once or repeated in a
//! grid.
//!
//! Each shape is measured by how far a pixel is from its outline (a
//! signed distance), so its edge is smooth at any size. A shape near the
//! edge of its repeat is joined with its neighbours', so shapes reaching
//! past the repeat still tile. The maths is plain geometry; nothing is
//! taken from other programs.

use super::{choice, info, number, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use glam::Vec2;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use words::text;

/// The Basic shape box.
#[derive(Debug)]
pub(crate) struct BasicShape {
    info: BoxInfo,
}

/// The shapes, in the Shape setting's order.
const CIRCLE: u32 = 0;
const SQUARE: u32 = 1;
const POLYGON: u32 = 2;
const STAR: u32 = 3;

impl BasicShape {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "basic_shape",
                text!("Basic shape"),
                Group::Make,
                text!(
                    "Makes one simple shape (a circle, square, polygon or star) in the middle of \
                     the texture, or a grid of them with Repeats. Use it for studs, rivets, \
                     holes, dots and stamped patterns. Size is how much of each repeat the shape \
                     fills; Soft edge blurs its outline."
                ),
                (
                    Vec::new(),
                    vec![PlugSpec::new(
                        "shape",
                        text!("Shape"),
                        ValueKind::Shape,
                        false,
                    )],
                ),
                vec![
                    choice(
                        "shape",
                        text!("Shape"),
                        text!("Which shape."),
                        &[
                            text!("Circle"),
                            text!("Square"),
                            text!("Polygon"),
                            text!("Star"),
                        ],
                        CIRCLE,
                    ),
                    number(
                        "size",
                        text!("Size"),
                        text!("How much of each repeat the shape fills, across."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    whole(
                        "sides",
                        text!("Sides"),
                        text!("How many sides a polygon has, or points a star has."),
                        3,
                        16,
                        5,
                    ),
                    number(
                        "inner",
                        text!("Star depth"),
                        text!(
                            "How far in a star's points start: low is spiky, high is nearly round."
                        ),
                        0.1,
                        0.95,
                        0.45,
                    ),
                    number(
                        "turn",
                        text!("Turn"),
                        text!("How far the shape is turned, in degrees."),
                        0.0,
                        360.0,
                        0.0,
                    ),
                    whole(
                        "repeats",
                        text!("Repeats"),
                        text!("How many shapes across and down the texture."),
                        1,
                        64,
                        1,
                    ),
                    number(
                        "soft",
                        text!("Soft edge"),
                        text!("How soft the outline is, as a share of the shape's size."),
                        0.0,
                        1.0,
                        0.0,
                    ),
                ],
                &["bevel", "blend", "repeat"],
            ),
        }
    }
}

impl BoxType for BasicShape {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let kind = job.choice("shape");
        let radius = job.number("size").clamp(0.0, 1.0) * 0.5;
        let sides = job.whole("sides").clamp(3, 16) as f32;
        let inner = job.number("inner").clamp(0.1, 0.95);
        let turn = job.number("turn").to_radians();
        let repeats = job.whole("repeats").max(1) as f32;
        let soft = job.number("soft").clamp(0.0, 1.0);
        let n = job.size().get() as f32;
        // One repeat in pixels: distances are in repeats, turned into pixels.
        let repeat_px = n / repeats;
        let soft_px = (soft * radius * 2.0 * repeat_px).max(1.0);
        let (sin, cos) = (-turn).sin_cos();
        let image = job.fill_grey(|x, y| {
            if radius <= 0.0 {
                return 0.0;
            }
            let (u, v) = uv(x, y, n);
            let local = Vec2::new((u * repeats).fract() - 0.5, (v * repeats).fract() - 0.5);
            // This repeat's shape and its neighbours', joined.
            let mut d = f32::MAX;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let p = local - Vec2::new(dx as f32, dy as f32);
                    let p = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
                    d = d.min(distance(kind, p, radius, sides, inner));
                }
            }
            (0.5 - d * repeat_px / soft_px).clamp(0.0, 1.0)
        });
        Ok(vec![Value::Shape(Arc::new(image))])
    }
}

/// How far `p` is outside shape `kind` of `radius` (negative inside).
fn distance(kind: u32, p: Vec2, radius: f32, sides: f32, inner: f32) -> f32 {
    match kind {
        SQUARE => {
            let q = p.abs() - Vec2::splat(radius);
            q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0)
        }
        POLYGON => star(p, radius, sides, (PI / sides).cos()),
        STAR => star(p, radius, sides, inner),
        _ => p.length() - radius,
    }
}

/// How far `p` is outside a star of `points` points reaching `radius`,
/// whose dips are `inner` times as far out (a regular polygon when the
/// dips sit on the straight line between points).
fn star(p: Vec2, radius: f32, points: f32, inner: f32) -> f32 {
    let sector = TAU / points;
    // Fold the point into the half sector from a tip (along x) to a dip.
    let angle = p.y.atan2(p.x).rem_euclid(sector);
    let angle = if angle > sector * 0.5 {
        sector - angle
    } else {
        angle
    };
    let q = Vec2::new(angle.cos(), angle.sin()) * p.length();
    let tip = Vec2::new(radius, 0.0);
    let half = sector * 0.5;
    let dip = Vec2::new(half.cos(), half.sin()) * radius * inner;
    let edge = dip - tip;
    let t = ((q - tip).dot(edge) / edge.length_squared()).clamp(0.0, 1.0);
    let d = (q - (tip + edge * t)).length();
    // Inside when on the middle's side of the edge.
    let side = edge.x * (q.y - tip.y) - edge.y * (q.x - tip.x);
    if side > 0.0 { -d } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_are_negative_inside_and_positive_outside() {
        for kind in [CIRCLE, SQUARE, POLYGON, STAR] {
            assert!(distance(kind, Vec2::ZERO, 0.3, 5.0, 0.45) < 0.0, "{kind}");
            assert!(distance(kind, Vec2::new(0.45, 0.0), 0.3, 5.0, 0.45) > 0.0);
            assert!(distance(kind, Vec2::new(-0.2, 0.4), 0.3, 5.0, 0.45) > 0.0);
        }
        // A star's tip is on its outline; its dip is in by Star depth.
        assert!(star(Vec2::new(0.3, 0.0), 0.3, 5.0, 0.5).abs() < 1e-5);
        let half = PI / 5.0;
        let dip = Vec2::new(half.cos(), half.sin()) * 0.15;
        assert!(star(dip, 0.3, 5.0, 0.5).abs() < 1e-5);
        assert!(distance(SQUARE, Vec2::new(0.2, 0.2), 0.2, 4.0, 0.5).abs() < 1e-6);
    }
}
