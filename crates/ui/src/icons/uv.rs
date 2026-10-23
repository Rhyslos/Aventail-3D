//! The UV tools' icons: turning and flipping, seams, unwrap, relax,
//! straighten, pack, pins, stitching and cutting.

use super::Icon;
use super::pen::{Ink, Paint, Pen};
use std::f32::consts::PI;

/// Draws `icon` with `pen`, if it is one of this file's icons:
/// [`super::shapes`] sends each icon to the file that draws it.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::UvTurnLeft | Icon::UvTurnRight => {
            // A tile with a marked corner, and a quarter-and-more arc
            // around it ending in an arrowhead.
            let left = icon == Icon::UvTurnLeft;
            pen.shape(
                &[(8.5, 8.5), (15.5, 8.5), (15.5, 15.5), (8.5, 15.5)],
                Paint::Tone,
            );
            pen.dot(if left { 8.5 } else { 15.5 }, 8.5, 1.6);
            let (from, to) = if left {
                (0.6_f32, -3.6_f32)
            } else {
                (PI - 0.6, PI + 3.6)
            };
            let arc = pen.arc((12.0, 12.0), (8.5, 8.5), from, to);
            pen.line(arc);
            // The arrowhead at the end, along the way the arc turns.
            let (c, sn) = (to.cos(), to.sin());
            let end = (12.0 + 8.5 * c, 12.0 + 8.5 * sn);
            let way = if left { (sn, -c) } else { (-sn, c) };
            pen.shape(
                &[
                    (end.0 + way.0 * 3.0, end.1 + way.1 * 3.0),
                    (end.0 + c * 2.6, end.1 + sn * 2.6),
                    (end.0 - c * 2.6, end.1 - sn * 2.6),
                ],
                Paint::Solid,
            );
        }
        Icon::UvFlipU => {
            pen.stroke(&[(12.0, 3.0), (12.0, 21.0)], false, Ink::Dashed);
            pen.shape(&[(10.0, 6.0), (10.0, 18.0), (3.5, 18.0)], Paint::Tone);
            pen.stroke(&[(14.0, 6.0), (14.0, 18.0), (20.5, 18.0)], true, Ink::Line);
        }
        Icon::UvFlipV => {
            pen.stroke(&[(3.0, 12.0), (21.0, 12.0)], false, Ink::Dashed);
            pen.shape(&[(6.0, 10.0), (18.0, 10.0), (18.0, 3.5)], Paint::Tone);
            pen.stroke(&[(6.0, 14.0), (18.0, 14.0), (18.0, 20.5)], true, Ink::Line);
        }
        Icon::MarkSeam | Icon::ClearSeam => {
            // A line across, with stitches; cleared, both are dashed.
            let ink = if icon == Icon::MarkSeam {
                Ink::Line
            } else {
                Ink::Dashed
            };
            pen.stroke(&[(4.0, 20.0), (20.0, 4.0)], false, ink);
            if icon == Icon::MarkSeam {
                for t in [0.2_f32, 0.4, 0.6, 0.8] {
                    let (x, y) = (4.0 + 16.0 * t, 20.0 - 16.0 * t);
                    pen.stroke(&[(x - 2.5, y - 2.5), (x + 2.5, y + 2.5)], false, Ink::Line);
                }
            } else {
                pen.stroke(&[(14.0, 18.0), (20.0, 18.0)], false, Ink::Line);
            }
        }
        Icon::Unwrap => {
            // A box's six sides laid out flat, as a cross.
            pen.stroke(
                &[
                    (9.0, 3.0),
                    (15.0, 3.0),
                    (15.0, 9.0),
                    (21.0, 9.0),
                    (21.0, 15.0),
                    (15.0, 15.0),
                    (15.0, 21.0),
                    (9.0, 21.0),
                    (9.0, 15.0),
                    (3.0, 15.0),
                    (3.0, 9.0),
                    (9.0, 9.0),
                ],
                true,
                Ink::Line,
            );
            pen.shape(
                &[(9.0, 9.0), (15.0, 9.0), (15.0, 15.0), (9.0, 15.0)],
                Paint::Tone,
            );
        }
        Icon::Relax => {
            // A square whose inside lines are evened out: the bent ones
            // faint, the even ones solid.
            pen.stroke(
                &[(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(9.0, 4.0), (6.5, 12.0), (10.0, 20.0)], false, Ink::Faint);
            pen.stroke(&[(4.0, 9.5), (13.0, 7.0), (20.0, 10.0)], false, Ink::Faint);
            pen.stroke(&[(14.0, 4.0), (14.0, 20.0)], false, Ink::Line);
            pen.stroke(&[(4.0, 14.0), (20.0, 14.0)], false, Ink::Line);
        }
        Icon::Straighten => {
            pen.stroke(
                &[
                    (3.0, 9.0),
                    (7.5, 5.5),
                    (12.0, 9.5),
                    (16.5, 5.0),
                    (21.0, 8.0),
                ],
                false,
                Ink::Faint,
            );
            pen.stroke(&[(3.0, 16.0), (21.0, 16.0)], false, Ink::Line);
            pen.dot(3.0, 16.0, 1.6);
            pen.dot(21.0, 16.0, 1.6);
        }
        Icon::Pack => {
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Line,
            );
            pen.shape(
                &[(5.0, 5.0), (13.0, 5.0), (13.0, 13.0), (5.0, 13.0)],
                Paint::Tone,
            );
            pen.shape(
                &[(15.0, 5.0), (19.0, 5.0), (19.0, 19.0), (15.0, 19.0)],
                Paint::Tone,
            );
            pen.shape(
                &[(5.0, 15.0), (13.0, 15.0), (13.0, 19.0), (5.0, 19.0)],
                Paint::Tone,
            );
        }
        Icon::UvPin | Icon::UvUnpin => {
            // A pin's head and needle over a point; unpinned, it is lifted
            // off and dashed.
            let pinned = icon == Icon::UvPin;
            let lift = if pinned { 0.0 } else { -3.0 };
            pen.dot(9.0, 19.0, 1.8);
            let ink = if pinned { Ink::Line } else { Ink::Dashed };
            pen.stroke(&[(9.0, 19.0 + lift), (13.5, 14.5 + lift)], false, ink);
            let head = [
                (12.0, 12.0 + lift),
                (16.0, 16.0 + lift),
                (21.0, 11.0 + lift),
                (17.0, 7.0 + lift),
                (19.0, 5.0 + lift),
                (15.0, 3.0 + lift),
                (13.0, 5.0 + lift),
                (11.0, 7.0 + lift),
            ];
            if pinned {
                pen.shape(&head, Paint::Tone);
                pen.stroke(&head, true, Ink::Line);
            } else {
                pen.stroke(&head, true, Ink::Dashed);
            }
        }
        Icon::UvTurnFlip => {
            // A shape and its mirror across a dashed line, an arrow
            // turning over them, and dots for "more inside".
            pen.stroke(&[(12.0, 6.0), (12.0, 18.0)], false, Ink::Dashed);
            pen.shape(&[(10.0, 9.0), (10.0, 17.0), (4.5, 17.0)], Paint::Tone);
            pen.stroke(&[(14.0, 9.0), (14.0, 17.0), (19.5, 17.0)], true, Ink::Line);
            let (from, to) = (-2.7_f32, -0.6_f32);
            let arc = pen.arc((12.0, 10.0), (8.0, 7.0), from, to);
            pen.line(arc);
            let end = (12.0 + 8.0 * to.cos(), 10.0 + 7.0 * to.sin());
            pen.shape(
                &[
                    (end.0 + 1.2, end.1 + 2.8),
                    (end.0 - 2.4, end.1 - 0.6),
                    (end.0 + 2.2, end.1 - 1.6),
                ],
                Paint::Solid,
            );
            for x in [8.0, 12.0, 16.0] {
                pen.dot(x, 21.5, 1.3);
            }
        }
        Icon::UvStitch | Icon::UvStitchCut => {
            // Two edges, one either side, with arrows drawing them
            // together; the group adds dots for "more inside".
            pen.stroke(&[(4.0, 4.0), (4.0, 17.0)], false, Ink::Faint);
            pen.stroke(&[(20.0, 4.0), (20.0, 17.0)], false, Ink::Faint);
            pen.stroke(&[(12.0, 3.0), (12.0, 18.0)], false, Ink::Line);
            for y in [7.0, 14.0] {
                pen.stroke(&[(6.5, y), (10.0, y)], false, Ink::Line);
                pen.stroke(&[(14.0, y), (17.5, y)], false, Ink::Line);
            }
            if icon == Icon::UvStitchCut {
                for x in [8.0, 12.0, 16.0] {
                    pen.dot(x, 21.5, 1.3);
                }
            }
        }
        Icon::UvCut => {
            // One edge split in two, pulled apart, with a cut mark.
            pen.stroke(&[(9.0, 4.0), (9.0, 20.0)], false, Ink::Line);
            pen.stroke(&[(15.0, 4.0), (15.0, 20.0)], false, Ink::Line);
            pen.stroke(&[(12.0, 2.5), (12.0, 21.5)], false, Ink::Dashed);
            pen.stroke(&[(6.5, 12.0), (3.5, 12.0)], false, Ink::Line);
            pen.stroke(&[(17.5, 12.0), (20.5, 12.0)], false, Ink::Line);
        }
        // Drawn by the other icon files.
        _ => {}
    }
}
