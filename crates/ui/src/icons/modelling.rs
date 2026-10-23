//! The modelling tools' icons: extrude, inset, bevel, groove, bridge, fill
//! hole, flip normals, hard and soft edges, dissolve, weld, connect, slit,
//! loop cut, duplicate, instance, join, apply transforms, edit layer.

use super::Icon;
use super::pen::{Ink, Paint, Pen};

/// Draws `icon` with `pen`, if it is one of this file's icons:
/// [`super::shapes`] sends each icon to the file that draws it.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::Extrude => {
            pen.shape(
                &[(3.0, 12.0), (14.0, 12.0), (21.0, 8.0), (10.0, 8.0)],
                Paint::Solid,
            );
            pen.stroke(&[(3.0, 13.0), (3.0, 21.0)], false, Ink::Line);
            pen.stroke(&[(14.0, 13.0), (14.0, 21.0)], false, Ink::Line);
            pen.stroke(&[(21.0, 9.0), (21.0, 17.0)], false, Ink::Line);
            pen.stroke(
                &[(3.0, 21.0), (14.0, 21.0), (21.0, 17.0)],
                false,
                Ink::Faint,
            );
            pen.stroke(&[(12.0, 6.5), (12.0, 3.0)], false, Ink::Line);
            pen.shape(&[(12.0, 0.5), (9.0, 4.1), (15.0, 4.1)], Paint::Solid);
        }
        Icon::Inset => {
            pen.stroke(
                &[(2.5, 2.5), (21.5, 2.5), (21.5, 21.5), (2.5, 21.5)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(6.5, 6.5), (17.5, 6.5), (17.5, 17.5), (6.5, 17.5)],
                true,
                Ink::Faint,
            );
            pen.shape(
                &[(10.0, 10.0), (14.0, 10.0), (14.0, 14.0), (10.0, 14.0)],
                Paint::Solid,
            );
        }
        Icon::Bevel => {
            pen.stroke(
                &[
                    (4.0, 21.0),
                    (4.0, 13.0),
                    (4.17, 11.24),
                    (4.69, 9.56),
                    (5.52, 8.0),
                    (6.64, 6.64),
                    (8.0, 5.52),
                    (9.56, 4.69),
                    (11.24, 4.17),
                    (13.0, 4.0),
                    (21.0, 4.0),
                ],
                false,
                Ink::Line,
            );
            pen.stroke(
                &[
                    (8.0, 13.0),
                    (8.1, 12.02),
                    (8.38, 11.09),
                    (8.84, 10.22),
                    (9.46, 9.46),
                    (10.22, 8.84),
                    (11.09, 8.38),
                    (12.02, 8.1),
                    (13.0, 8.0),
                ],
                false,
                Ink::Faint,
            );
        }
        Icon::Groove => {
            // A slab seen from the side, a groove cut into its top.
            pen.stroke(
                &[
                    (3.0, 9.0),
                    (10.0, 9.0),
                    (10.0, 14.5),
                    (14.0, 14.5),
                    (14.0, 9.0),
                    (21.0, 9.0),
                ],
                false,
                Ink::Line,
            );
            pen.stroke(
                &[(3.0, 9.0), (3.0, 19.0), (21.0, 19.0), (21.0, 9.0)],
                false,
                Ink::Faint,
            );
            pen.shape(
                &[(10.0, 9.0), (14.0, 9.0), (14.0, 14.5), (10.0, 14.5)],
                Paint::Solid,
            );
        }
        Icon::Bridge => {
            pen.stroke(
                &[(2.0, 7.0), (8.0, 7.0), (8.0, 17.0), (2.0, 17.0)],
                true,
                Ink::Line,
            );
            pen.stroke(
                &[(16.0, 7.0), (22.0, 7.0), (22.0, 17.0), (16.0, 17.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(8.0, 7.0), (16.0, 7.0)], false, Ink::Dashed);
            pen.stroke(&[(8.0, 12.0), (16.0, 12.0)], false, Ink::Dashed);
            pen.stroke(&[(8.0, 17.0), (16.0, 17.0)], false, Ink::Dashed);
        }
        Icon::FillHole => {
            pen.stroke(
                &[
                    (4.0, 10.0),
                    (4.0, 17.0),
                    (12.0, 21.5),
                    (20.0, 17.0),
                    (20.0, 10.0),
                ],
                false,
                Ink::Line,
            );
            pen.stroke(&[(12.0, 14.5), (12.0, 21.5)], false, Ink::Line);
            pen.shape(
                &[(4.0, 10.0), (12.0, 14.5), (20.0, 10.0), (12.0, 5.5)],
                Paint::Tone,
            );
            pen.stroke(
                &[(4.0, 10.0), (12.0, 14.5), (20.0, 10.0), (12.0, 5.5)],
                true,
                Ink::Dashed,
            );
            pen.stroke(&[(12.0, 1.5), (12.0, 7.0)], false, Ink::Line);
            pen.shape(&[(12.0, 9.5), (9.8, 6.64), (14.2, 6.64)], Paint::Solid);
        }
        Icon::FlipNormals => {
            pen.stroke(&[(3.0, 12.0), (21.0, 12.0)], false, Ink::Line);
            pen.stroke(&[(8.0, 12.0), (8.0, 4.0)], false, Ink::AxisX);
            pen.stroke(&[(5.5, 6.5), (8.0, 4.0), (10.5, 6.5)], false, Ink::AxisX);
            pen.stroke(&[(16.0, 12.0), (16.0, 20.0)], false, Ink::AxisY);
            pen.stroke(
                &[(13.5, 17.5), (16.0, 20.0), (18.5, 17.5)],
                false,
                Ink::AxisY,
            );
        }
        Icon::HardEdges => {
            pen.shape(
                &[(4.0, 7.0), (8.0, 9.5), (8.0, 20.5), (4.0, 18.0)],
                Paint::Shade,
            );
            pen.shape(
                &[(8.0, 9.5), (16.0, 9.5), (16.0, 20.5), (8.0, 20.5)],
                Paint::Tone,
            );
            pen.stroke(
                &[
                    (4.0, 7.0),
                    (8.0, 4.5),
                    (16.0, 4.5),
                    (20.0, 7.0),
                    (16.0, 9.5),
                    (8.0, 9.5),
                ],
                true,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 7.0), (4.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(20.0, 7.0), (20.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(8.0, 9.5), (8.0, 20.5)], false, Ink::Line);
            pen.stroke(&[(16.0, 9.5), (16.0, 20.5)], false, Ink::Line);
            pen.stroke(
                &[(4.0, 18.0), (8.0, 20.5), (16.0, 20.5), (20.0, 18.0)],
                false,
                Ink::Line,
            );
        }
        Icon::SoftEdges => {
            pen.ring((12.0, 6.0), (8.0, 3.0), Ink::Line);
            pen.shape(
                &[
                    (4.0, 6.0),
                    (4.0, 18.0),
                    (4.15, 18.59),
                    (4.61, 19.15),
                    (5.35, 19.67),
                    (6.34, 20.12),
                    (7.56, 20.49),
                    (8.94, 20.77),
                    (10.44, 20.94),
                    (12.0, 21.0),
                    (13.56, 20.94),
                    (15.06, 20.77),
                    (16.44, 20.49),
                    (17.66, 20.12),
                    (18.65, 19.67),
                    (19.39, 19.15),
                    (19.85, 18.59),
                    (20.0, 18.0),
                    (20.0, 6.0),
                    (19.85, 6.59),
                    (19.39, 7.15),
                    (18.65, 7.67),
                    (17.66, 8.12),
                    (16.44, 8.49),
                    (15.06, 8.77),
                    (13.56, 8.94),
                    (12.0, 9.0),
                    (10.44, 8.94),
                    (8.94, 8.77),
                    (7.56, 8.49),
                    (6.34, 8.12),
                    (5.35, 7.67),
                    (4.61, 7.15),
                    (4.15, 6.59),
                    (4.0, 6.0),
                ],
                Paint::Shading,
            );
            pen.stroke(&[(4.0, 6.0), (4.0, 18.0)], false, Ink::Line);
            pen.stroke(&[(20.0, 6.0), (20.0, 18.0)], false, Ink::Line);
            pen.stroke(
                &[
                    (4.0, 18.0),
                    (4.15, 18.59),
                    (4.61, 19.15),
                    (5.35, 19.67),
                    (6.34, 20.12),
                    (7.56, 20.49),
                    (8.94, 20.77),
                    (10.44, 20.94),
                    (12.0, 21.0),
                    (13.56, 20.94),
                    (15.06, 20.77),
                    (16.44, 20.49),
                    (17.66, 20.12),
                    (18.65, 19.67),
                    (19.39, 19.15),
                    (19.85, 18.59),
                    (20.0, 18.0),
                ],
                false,
                Ink::Line,
            );
        }
        Icon::Dissolve => {
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(&[(3.0, 21.0), (9.5, 14.5)], false, Ink::Line);
            pen.disc((12.0, 12.0), 1.25, Paint::Solid);
            pen.disc((14.8, 9.2), 1.0, Paint::Solid);
            pen.disc((17.5, 6.5), 0.8, Paint::Solid);
            pen.disc((20.0, 4.0), 0.6, Paint::Solid);
        }
        Icon::Weld => {
            pen.stroke(&[(3.0, 12.0), (9.0, 12.0)], false, Ink::Line);
            pen.disc((3.0, 12.0), 1.8, Paint::Solid);
            pen.disc((9.0, 12.0), 1.8, Paint::Solid);
            pen.stroke(&[(11.5, 12.0), (13.5, 12.0)], false, Ink::Line);
            pen.shape(&[(16.0, 12.0), (13.4, 14.0), (13.4, 10.0)], Paint::Solid);
            pen.disc((19.5, 12.0), 2.6, Paint::Solid);
        }
        Icon::Connect => {
            pen.stroke(
                &[(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(4.0, 12.0), (20.0, 12.0)], false, Ink::Dashed);
            pen.disc((4.0, 12.0), 2.0, Paint::Solid);
            pen.disc((20.0, 12.0), 2.0, Paint::Solid);
        }
        Icon::Slit => {
            pen.stroke(
                &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(
                &[
                    (3.0, 15.0),
                    (8.0, 10.0),
                    (12.0, 13.0),
                    (17.0, 8.0),
                    (21.0, 11.0),
                ],
                false,
                Ink::Line,
            );
            pen.disc((3.0, 15.0), 1.6, Paint::Solid);
            pen.disc((21.0, 11.0), 1.6, Paint::Solid);
        }
        Icon::LoopCut => {
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
                Ink::Faint,
            );
            pen.stroke(&[(4.0, 12.0), (12.0, 16.5), (20.0, 12.0)], false, Ink::Line);
            pen.stroke(&[(12.0, 3.0), (12.0, 7.5)], false, Ink::Faint);
        }
        Icon::Cancel => {
            pen.ring((12.0, 12.0), (9.0, 9.0), Ink::Line);
            pen.stroke(&[(7.5, 12.0), (16.5, 12.0)], false, Ink::Line);
        }
        Icon::Duplicate => {
            // The copy behind, then the square in front.
            pen.stroke(
                &[(9.0, 4.0), (20.0, 4.0), (20.0, 15.0), (16.0, 15.0)],
                false,
                Ink::Line,
            );
            pen.stroke(&[(9.0, 4.0), (9.0, 8.0)], false, Ink::Line);
            pen.stroke(
                &[(4.0, 9.0), (15.0, 9.0), (15.0, 20.0), (4.0, 20.0)],
                true,
                Ink::Line,
            );
        }
        Icon::Instance => {
            // A dashed copy behind, joined to the square in front.
            pen.stroke(
                &[(9.0, 4.0), (20.0, 4.0), (20.0, 15.0), (16.0, 15.0)],
                false,
                Ink::Dashed,
            );
            pen.stroke(&[(9.0, 4.0), (9.0, 8.0)], false, Ink::Dashed);
            pen.stroke(
                &[(4.0, 9.0), (15.0, 9.0), (15.0, 20.0), (4.0, 20.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(15.0, 9.0), (20.0, 4.0)], false, Ink::Line);
        }
        Icon::Join => {
            // Two halves made one: the outline whole, the old join faint.
            pen.stroke(
                &[(3.0, 7.0), (21.0, 7.0), (21.0, 17.0), (3.0, 17.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(12.0, 7.0), (12.0, 17.0)], false, Ink::Dashed);
            pen.stroke(&[(9.0, 12.0), (15.0, 12.0)], false, Ink::Line);
        }
        Icon::ApplyTransforms => {
            // A turned, stretched box (faint) and the same box squared up,
            // with a tick of an arrow between them.
            pen.stroke(
                &[(3.0, 9.0), (13.0, 3.0), (17.0, 9.0), (7.0, 15.0)],
                true,
                Ink::Faint,
            );
            pen.stroke(
                &[(11.0, 11.0), (21.0, 11.0), (21.0, 21.0), (11.0, 21.0)],
                true,
                Ink::Line,
            );
            pen.stroke(&[(8.0, 17.0), (8.0, 20.0), (5.0, 20.0)], false, Ink::Line);
        }
        Icon::EditLayer => {
            // Two sheets below (faint) and a new one on top, seen from
            // above at a slant.
            pen.stroke(
                &[(3.0, 13.0), (12.0, 17.0), (21.0, 13.0)],
                false,
                Ink::Faint,
            );
            pen.stroke(
                &[(3.0, 17.0), (12.0, 21.0), (21.0, 17.0)],
                false,
                Ink::Faint,
            );
            pen.stroke(
                &[(3.0, 9.0), (12.0, 5.0), (21.0, 9.0), (12.0, 13.0)],
                true,
                Ink::Line,
            );
        }
        // Drawn by the other icon files.
        _ => {}
    }
}
