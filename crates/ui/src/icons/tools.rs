//! The tools' icons (move, rotate, scale, all gizmos, framing, make
//! editable, delete, grouping, the shape list), picked by the project lead
//! from the icon mockup, and the pivot commands' icons.

use super::Icon;
use super::pen::{Ink, Paint, Pen};

/// Draws `icon` with `pen`, if it is one of this file's icons:
/// [`super::shapes`] sends each icon to the file that draws it.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        // The tools' icons below were picked by the project lead in the icon
        // mockup (2026-10-12) and converted from it point for point.
        Icon::Move => {
            pen.stroke(&[(12.0, 3.0), (12.0, 21.0)], false, Ink::Line);
            pen.stroke(&[(3.0, 12.0), (21.0, 12.0)], false, Ink::Line);
            pen.stroke(&[(9.0, 6.0), (12.0, 3.0), (15.0, 6.0)], false, Ink::Line);
            pen.stroke(&[(9.0, 18.0), (12.0, 21.0), (15.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(6.0, 9.0), (3.0, 12.0), (6.0, 15.0)], false, Ink::Line);
            pen.stroke(&[(18.0, 9.0), (21.0, 12.0), (18.0, 15.0)], false, Ink::Line);
        }
        Icon::Rotate => {
            pen.stroke(
                &[
                    (4.48, 9.26),
                    (5.2, 7.78),
                    (6.21, 6.48),
                    (7.46, 5.41),
                    (8.9, 4.62),
                    (10.48, 4.14),
                    (12.12, 4.0),
                    (13.75, 4.19),
                    (15.31, 4.71),
                    (16.73, 5.55),
                    (17.95, 6.65),
                ],
                false,
                Ink::Line,
            );
            pen.shape(&[(19.53, 9.04), (15.59, 7.41), (20.09, 4.81)], Paint::Solid);
            pen.stroke(
                &[
                    (19.52, 14.74),
                    (18.8, 16.22),
                    (17.79, 17.52),
                    (16.54, 18.59),
                    (15.1, 19.38),
                    (13.52, 19.86),
                    (11.88, 20.0),
                    (10.25, 19.81),
                    (8.69, 19.29),
                    (7.27, 18.45),
                    (6.05, 17.35),
                ],
                false,
                Ink::Line,
            );
            pen.shape(&[(4.47, 14.96), (8.41, 16.59), (3.91, 19.19)], Paint::Solid);
        }
        Icon::Scale => {
            pen.stroke(
                &[(3.0, 12.0), (12.0, 12.0), (12.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Dashed,
            );
            pen.stroke(&[(12.0, 12.0), (20.0, 4.0)], false, Ink::Line);
            pen.stroke(&[(15.0, 4.0), (20.0, 4.0), (20.0, 9.0)], false, Ink::Line);
        }
        Icon::AllGizmos => {
            pen.stroke(&[(9.0, 15.0), (9.0, 5.5)], false, Ink::AxisZ);
            pen.shape(&[(9.0, 2.5), (11.3, 5.49), (6.7, 5.49)], Paint::AxisZ);
            pen.stroke(&[(9.0, 15.0), (17.5, 15.0)], false, Ink::AxisX);
            pen.shape(
                &[(17.5, 13.0), (21.5, 13.0), (21.5, 17.0), (17.5, 17.0)],
                Paint::AxisX,
            );
            pen.stroke(
                &[
                    (10.22, 8.11),
                    (11.59, 8.5),
                    (12.85, 9.16),
                    (13.95, 10.05),
                    (14.84, 11.15),
                    (15.5, 12.41),
                    (15.89, 13.78),
                ],
                false,
                Ink::AxisY,
            );
            pen.disc((9.0, 15.0), 1.6, Paint::Solid);
        }
        Icon::FrameSelected => {
            pen.stroke(&[(3.0, 8.0), (3.0, 3.0), (8.0, 3.0)], false, Ink::Line);
            pen.stroke(&[(16.0, 3.0), (21.0, 3.0), (21.0, 8.0)], false, Ink::Line);
            pen.stroke(
                &[(21.0, 16.0), (21.0, 21.0), (16.0, 21.0)],
                false,
                Ink::Line,
            );
            pen.stroke(&[(8.0, 21.0), (3.0, 21.0), (3.0, 16.0)], false, Ink::Line);
            pen.shape(
                &[(8.0, 8.0), (16.0, 8.0), (16.0, 16.0), (8.0, 16.0)],
                Paint::Tone,
            );
            pen.stroke(
                &[(8.0, 8.0), (16.0, 8.0), (16.0, 16.0), (8.0, 16.0)],
                true,
                Ink::Line,
            );
        }
        Icon::MakeEditable => {
            pen.stroke(
                &[
                    (12.0, 3.0),
                    (20.0, 7.5),
                    (20.0, 16.5),
                    (12.0, 21.0),
                    (4.0, 16.5),
                    (4.0, 7.5),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 7.5), (12.0, 12.0), (20.0, 7.5)], false, Ink::Line);
            pen.stroke(&[(12.0, 12.0), (12.0, 21.0)], false, Ink::Line);
            pen.disc((12.0, 3.0), 1.7, Paint::Solid);
            pen.disc((20.0, 7.5), 1.7, Paint::Solid);
            pen.disc((20.0, 16.5), 1.7, Paint::Solid);
            pen.disc((12.0, 21.0), 1.7, Paint::Solid);
            pen.disc((4.0, 16.5), 1.7, Paint::Solid);
            pen.disc((4.0, 7.5), 1.7, Paint::Solid);
            pen.disc((12.0, 12.0), 1.7, Paint::Solid);
        }
        Icon::Delete => {
            pen.stroke(
                &[(6.5, 9.5), (17.5, 9.5), (16.4, 21.0), (7.6, 21.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(10.5, 13.0), (10.5, 18.0)], false, Ink::Line);
            pen.stroke(&[(13.5, 13.0), (13.5, 18.0)], false, Ink::Line);
            pen.stroke(&[(4.21, 8.69), (17.79, 5.31)], false, Ink::Line);
            pen.stroke(
                &[(9.54, 7.36), (8.94, 4.94), (12.82, 3.97), (13.43, 6.4)],
                false,
                Ink::Line,
            );
        }
        Icon::Ungroup => {
            pen.stroke(
                &[
                    (15.0, 7.0),
                    (15.52, 7.07),
                    (16.0, 7.27),
                    (16.41, 7.59),
                    (16.73, 8.0),
                    (16.93, 8.48),
                    (17.0, 9.0),
                    (17.0, 15.0),
                    (16.93, 15.52),
                    (16.73, 16.0),
                    (16.41, 16.41),
                    (16.0, 16.73),
                    (15.52, 16.93),
                    (15.0, 17.0),
                    (9.0, 17.0),
                    (8.48, 16.93),
                    (8.0, 16.73),
                    (7.59, 16.41),
                    (7.27, 16.0),
                    (7.07, 15.52),
                    (7.0, 15.0),
                    (7.0, 9.0),
                    (7.07, 8.48),
                    (7.27, 8.0),
                    (7.59, 7.59),
                    (8.0, 7.27),
                    (8.48, 7.07),
                    (9.0, 7.0),
                ],
                true,
                Ink::Dashed,
            );
            pen.stroke(
                &[(2.0, 2.0), (7.0, 2.0), (7.0, 7.0), (2.0, 7.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(17.0, 17.0), (22.0, 17.0), (22.0, 22.0), (17.0, 22.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(7.5, 7.5), (7.0, 7.0)], false, Ink::Faint);
        }
        Icon::Group => {
            pen.stroke(
                &[(6.0, 6.0), (11.0, 6.0), (11.0, 11.0), (6.0, 11.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(13.0, 13.0), (18.0, 13.0), (18.0, 18.0), (13.0, 18.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[
                    (18.0, 3.0),
                    (18.78, 3.1),
                    (19.5, 3.4),
                    (20.12, 3.88),
                    (20.6, 4.5),
                    (20.9, 5.22),
                    (21.0, 6.0),
                    (21.0, 18.0),
                    (20.9, 18.78),
                    (20.6, 19.5),
                    (20.12, 20.12),
                    (19.5, 20.6),
                    (18.78, 20.9),
                    (18.0, 21.0),
                    (6.0, 21.0),
                    (5.22, 20.9),
                    (4.5, 20.6),
                    (3.88, 20.12),
                    (3.4, 19.5),
                    (3.1, 18.78),
                    (3.0, 18.0),
                    (3.0, 6.0),
                    (3.1, 5.22),
                    (3.4, 4.5),
                    (3.88, 3.88),
                    (4.5, 3.4),
                    (5.22, 3.1),
                    (6.0, 3.0),
                ],
                true,
                Ink::Dashed,
            );
        }
        Icon::NewGroup => {
            pen.stroke(
                &[
                    (17.0, 4.0),
                    (17.78, 4.1),
                    (18.5, 4.4),
                    (19.12, 4.88),
                    (19.6, 5.5),
                    (19.9, 6.22),
                    (20.0, 7.0),
                    (20.0, 17.0),
                    (19.9, 17.78),
                    (19.6, 18.5),
                    (19.12, 19.12),
                    (18.5, 19.6),
                    (17.78, 19.9),
                    (17.0, 20.0),
                    (7.0, 20.0),
                    (6.22, 19.9),
                    (5.5, 19.6),
                    (4.88, 19.12),
                    (4.4, 18.5),
                    (4.1, 17.78),
                    (4.0, 17.0),
                    (4.0, 7.0),
                    (4.1, 6.22),
                    (4.4, 5.5),
                    (4.88, 4.88),
                    (5.5, 4.4),
                    (6.22, 4.1),
                    (7.0, 4.0),
                ],
                true,
                Ink::Dashed,
            );
            pen.stroke(&[(12.0, 8.5), (12.0, 15.5)], false, Ink::Line);
            pen.stroke(&[(8.5, 12.0), (15.5, 12.0)], false, Ink::Line);
        }
        Icon::ShapeList => {
            pen.stroke(
                &[
                    (10.0, 3.5),
                    (17.0, 7.2),
                    (17.0, 15.0),
                    (10.0, 18.7),
                    (3.0, 15.0),
                    (3.0, 7.2),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(3.0, 7.2), (10.0, 11.0), (17.0, 7.2)], false, Ink::Line);
            pen.stroke(&[(10.0, 11.0), (10.0, 18.7)], false, Ink::Line);
            pen.stroke(&[(19.0, 16.0), (19.0, 22.0)], false, Ink::Line);
            pen.stroke(&[(16.0, 19.0), (22.0, 19.0)], false, Ink::Line);
        }
        Icon::PivotToCentre => {
            // A shape with its pivot moved from a corner to its middle.
            pen.stroke(
                &[(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(&[(5.5, 18.5), (10.5, 13.5)], false, Ink::Dashed);
            pivot(pen, (12.0, 12.0));
        }
        Icon::PivotToOrigin => {
            // The origin's cross, a shape away from it, and its pivot
            // moved onto the cross.
            pen.stroke(&[(2.0, 18.0), (14.0, 18.0)], false, Ink::AxisX);
            pen.stroke(&[(6.0, 22.0), (6.0, 10.0)], false, Ink::AxisY);
            pen.stroke(
                &[(12.0, 3.0), (21.0, 3.0), (21.0, 12.0), (12.0, 12.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(&[(14.0, 10.0), (8.5, 15.5)], false, Ink::Dashed);
            pivot(pen, (6.0, 18.0));
        }
        Icon::PivotToObject => {
            // Two shapes: the first's pivot jumps to the second's.
            pen.stroke(
                &[(2.0, 11.0), (10.0, 11.0), (10.0, 19.0), (2.0, 19.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(
                &[(13.0, 5.0), (22.0, 5.0), (22.0, 14.0), (13.0, 14.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(6.0, 15.0), (15.5, 10.5)], false, Ink::Dashed);
            pivot(pen, (17.5, 9.5));
        }
        Icon::ObjectToOrigin => {
            // The origin's cross, the shape's old place dashed, and the
            // shape standing on the cross.
            pen.stroke(&[(2.0, 18.0), (22.0, 18.0)], false, Ink::AxisX);
            pen.stroke(&[(6.0, 22.0), (6.0, 6.0)], false, Ink::AxisY);
            pen.stroke(
                &[(13.0, 3.0), (21.0, 3.0), (21.0, 11.0), (13.0, 11.0)],
                true,
                Ink::Dashed,
            );
            pen.stroke(
                &[(6.0, 10.0), (14.0, 10.0), (14.0, 18.0), (6.0, 18.0)],
                true,
                Ink::Line,
            );
            pen.dot(6.0, 18.0, 2.0);
        }
        // Drawn by the other icon files.
        _ => {}
    }
}

/// A pivot: a dot with two short arms.
pub(super) fn pivot(pen: &mut Pen, at: (f32, f32)) {
    let (x, y) = at;
    pen.polyline(&[(x, y), (x, y - 4.5)]);
    pen.polyline(&[(x, y), (x + 4.5, y)]);
    pen.dot(x, y, 2.0);
}
