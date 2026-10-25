//! The curve icons (decision 0138): thin curves with their points as
//! dots, the way a curve looks in the viewport. Original drawings.

use super::Icon;
use super::pen::{Ink, Pen};

/// Points along a cubic Bézier on the 24 × 24 grid.
fn bezier(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> Vec<(f32, f32)> {
    let steps = 16;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let s = 1.0 - t;
            let w = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
            (
                w[0] * a.0 + w[1] * b.0 + w[2] * c.0 + w[3] * d.0,
                w[0] * a.1 + w[1] * b.1 + w[2] * c.1 + w[3] * d.1,
            )
        })
        .collect()
}

pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::DrawCurve => {
            // A curve pulled out of a point by its handle, with the pen's
            // nib at its end.
            pen.stroke(
                &bezier((3.0, 19.0), (5.0, 7.0), (13.0, 7.0), (14.0, 12.0)),
                false,
                Ink::Line,
            );
            pen.dot(3.0, 19.0, 1.6);
            pen.stroke(&[(9.0, 9.0), (19.0, 15.0)], false, Ink::Faint);
            pen.dot(9.0, 9.0, 1.2);
            pen.dot(19.0, 15.0, 1.2);
            pen.dot(14.0, 12.0, 1.6);
            // The nib.
            pen.stroke(
                &[
                    (15.0, 13.0),
                    (21.0, 7.0),
                    (22.0, 3.0),
                    (18.0, 4.0),
                    (15.0, 13.0),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(16.5, 10.0), (20.0, 5.5)], false, Ink::Faint);
        }
        Icon::CurveLine => {
            pen.stroke(&[(3.0, 18.0), (21.0, 6.0)], false, Ink::Line);
            pen.dot(3.0, 18.0, 1.6);
            pen.dot(21.0, 6.0, 1.6);
        }
        Icon::CurveCircle => {
            pen.ring((12.0, 12.0), (8.0, 8.0), Ink::Line);
            for (x, y) in [(20.0, 12.0), (12.0, 4.0), (4.0, 12.0), (12.0, 20.0)] {
                pen.dot(x, y, 1.6);
            }
        }
        Icon::CurveArc => {
            pen.stroke(
                &bezier((4.0, 19.0), (4.0, 10.0), (10.0, 4.0), (19.0, 4.0)),
                false,
                Ink::Line,
            );
            pen.stroke(
                &[(4.0, 19.0), (19.0, 19.0), (19.0, 4.0)],
                false,
                Ink::Dashed,
            );
            pen.dot(4.0, 19.0, 1.6);
            pen.dot(19.0, 4.0, 1.6);
        }
        Icon::CurveSpiral => {
            // A spring seen from the side: loops climbing.
            let mut points = Vec::new();
            for i in 0..=48 {
                let t = i as f32 / 48.0;
                let a = t * 3.0 * std::f32::consts::TAU;
                points.push((12.0 + 7.0 * a.cos(), 20.0 - 16.0 * t + 2.0 * a.sin()));
            }
            pen.stroke(&points, false, Ink::Line);
            pen.dot(19.0, 20.0, 1.6);
            pen.dot(19.0, 4.0, 1.6);
        }
        _ => {}
    }
}

/// The curve editing icons (decision 0139).
pub(super) fn draw_editing(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::CurveSplit => {
            // A curve cut in two: a gap between two end points.
            pen.stroke(
                &bezier((3.0, 19.0), (4.0, 12.0), (7.0, 10.0), (10.0, 10.0)),
                false,
                Ink::Line,
            );
            pen.stroke(
                &bezier((14.0, 14.0), (17.0, 14.0), (20.0, 12.0), (21.0, 5.0)),
                false,
                Ink::Line,
            );
            pen.dot(10.0, 10.0, 1.6);
            pen.dot(14.0, 14.0, 1.6);
            pen.stroke(&[(9.0, 16.0), (15.0, 8.0)], false, Ink::Dashed);
        }
        Icon::CurveReverse => {
            // A curve with its arrow turned back to the start.
            pen.stroke(
                &bezier((4.0, 15.0), (8.0, 6.0), (16.0, 6.0), (20.0, 15.0)),
                false,
                Ink::Line,
            );
            pen.stroke(&[(7.0, 13.5), (4.0, 15.0), (3.0, 11.5)], false, Ink::Line);
            pen.dot(20.0, 15.0, 1.6);
            pen.stroke(&[(5.0, 20.0), (19.0, 20.0)], false, Ink::Dashed);
        }
        Icon::CurveOpenClose => {
            // A loop, its last stretch dashed: closing it, or opening.
            let arc = |from: f32, to: f32| -> Vec<(f32, f32)> {
                (0..=24)
                    .map(|i| {
                        let a = from + (to - from) * i as f32 / 24.0;
                        (12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin())
                    })
                    .collect()
            };
            let tau = std::f32::consts::TAU;
            pen.stroke(&arc(0.15 * tau, 0.95 * tau), false, Ink::Line);
            pen.stroke(&arc(-0.05 * tau, 0.15 * tau), false, Ink::Dashed);
            let (a, b) = (0.15 * tau, -0.05 * tau);
            pen.dot(12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin(), 1.6);
            pen.dot(12.0 + 8.0 * b.cos(), 12.0 + 8.0 * b.sin(), 1.6);
        }
        Icon::CurveHandles => {
            // A bend with a point and its handle across it.
            pen.stroke(
                &bezier((3.0, 19.0), (6.0, 9.0), (18.0, 9.0), (21.0, 19.0)),
                false,
                Ink::Line,
            );
            pen.stroke(&[(5.0, 11.5), (19.0, 11.5)], false, Ink::Faint);
            pen.dot(12.0, 11.5, 1.8);
            pen.dot(5.0, 11.5, 1.2);
            pen.dot(19.0, 11.5, 1.2);
        }
        Icon::CurveSharp => {
            pen.stroke(&[(3.0, 19.0), (12.0, 5.0), (21.0, 19.0)], false, Ink::Line);
            pen.dot(12.0, 5.0, 1.8);
            pen.dot(3.0, 19.0, 1.4);
            pen.dot(21.0, 19.0, 1.4);
        }
        Icon::CurveSmooth => {
            pen.stroke(
                &bezier((3.0, 19.0), (6.0, 6.0), (18.0, 6.0), (21.0, 19.0)),
                false,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 9.5), (20.0, 9.5)], false, Ink::Faint);
            pen.dot(12.0, 9.3, 1.8);
            pen.dot(4.0, 9.5, 1.2);
            pen.dot(20.0, 9.5, 1.2);
        }
        Icon::CurveBroken => {
            // Curved sides meeting in a kink, each handle its own way.
            pen.stroke(
                &bezier((3.0, 19.0), (4.0, 11.0), (8.0, 6.0), (12.0, 5.0)),
                false,
                Ink::Line,
            );
            pen.stroke(
                &bezier((12.0, 5.0), (16.0, 6.0), (20.0, 11.0), (21.0, 19.0)),
                false,
                Ink::Line,
            );
            pen.stroke(&[(6.0, 6.5), (12.0, 5.0), (18.0, 6.5)], false, Ink::Faint);
            pen.dot(12.0, 5.0, 1.8);
            pen.dot(6.0, 6.5, 1.2);
            pen.dot(18.0, 6.5, 1.2);
        }
        Icon::CurveAuto => {
            // A bend through three points, its handles worked out (dashed).
            pen.stroke(
                &bezier((3.0, 18.0), (6.0, 6.0), (18.0, 6.0), (21.0, 18.0)),
                false,
                Ink::Line,
            );
            pen.stroke(&[(5.5, 9.0), (18.5, 9.0)], false, Ink::Dashed);
            for (x, y) in [(3.0, 18.0), (12.0, 9.0), (21.0, 18.0)] {
                pen.dot(x, y, 1.6);
            }
        }
        Icon::CurveRound => {
            // A square corner, dashed, cut by a rounded arc.
            pen.stroke(&[(4.0, 20.0), (4.0, 4.0), (20.0, 4.0)], false, Ink::Dashed);
            pen.stroke(&[(4.0, 20.0), (4.0, 13.0)], false, Ink::Line);
            pen.stroke(
                &bezier((4.0, 13.0), (4.0, 8.0), (8.0, 4.0), (13.0, 4.0)),
                false,
                Ink::Line,
            );
            pen.stroke(&[(13.0, 4.0), (20.0, 4.0)], false, Ink::Line);
            pen.dot(4.0, 13.0, 1.5);
            pen.dot(13.0, 4.0, 1.5);
        }
        Icon::CurveEven => {
            // Points evenly along a curve, each step the same.
            let line = bezier((3.0, 17.0), (8.0, 5.0), (16.0, 5.0), (21.0, 17.0));
            pen.stroke(&line, false, Ink::Line);
            for k in [0, 4, 8, 12, 16] {
                let (x, y) = line[k];
                pen.dot(x, y, 1.5);
            }
            pen.stroke(&[(4.0, 21.0), (20.0, 21.0)], false, Ink::Faint);
            for x in [4.0, 8.0, 12.0, 16.0, 20.0] {
                pen.stroke(&[(x, 19.5), (x, 22.5)], false, Ink::Faint);
            }
        }
        _ => {}
    }
}
