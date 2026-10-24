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
