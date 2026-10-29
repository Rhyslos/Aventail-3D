//! The paint tools' icons (decision 0213): drawn for the app, in the
//! same 24-unit box and line weights as the others. The paint brush
//! itself uses the Paint mode's icon.

use super::Icon;
use super::pen::{Ink, Paint, Pen};

/// Draws `icon` with `pen`, if it is one of this file's icons.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::PaintEraser => {
            // A block eraser leaning right, its rubber end toned, over the
            // faint line it has taken away.
            pen.polygon(&[(4.0, 15.0), (13.0, 6.0), (20.0, 13.0), (11.0, 22.0)]);
            pen.shape(&[(4.0, 15.0), (8.5, 10.5), (15.5, 17.5), (11.0, 22.0)], Paint::Tone);
            pen.stroke(&[(13.0, 21.5), (21.0, 21.5)], false, Ink::Faint);
        }
        Icon::PaintProjection => {
            // A picture (a frame with a hill) and an arrow down onto a
            // surface below it.
            pen.square((5.0, 3.0), (19.0, 12.0), false);
            pen.polyline(&[(7.0, 10.5), (10.5, 7.0), (13.0, 9.0), (15.0, 7.5), (17.5, 10.5)]);
            pen.stroke(&[(12.0, 13.5), (12.0, 18.0)], false, Ink::Line);
            pen.shape(&[(9.5, 16.5), (14.5, 16.5), (12.0, 19.5)], Paint::Solid);
            pen.stroke(&[(3.0, 21.0), (21.0, 21.0)], false, Ink::Faint);
        }
        Icon::PaintPolygonFill => {
            // A box seen from the corner, its front face filled.
            pen.polygon(&[(4.0, 8.0), (12.0, 4.0), (20.0, 8.0), (12.0, 12.0)]);
            pen.shape(&[(4.0, 8.0), (12.0, 12.0), (12.0, 21.0), (4.0, 17.0)], Paint::Solid);
            pen.polygon(&[(12.0, 12.0), (20.0, 8.0), (20.0, 17.0), (12.0, 21.0)]);
        }
        Icon::PaintSmudge => {
            // A fingertip dragging a blob into a fading trail.
            pen.disc((7.0, 16.0), 3.5, Paint::Solid);
            pen.stroke(&[(9.0, 13.5), (13.0, 11.5), (17.0, 10.5)], false, Ink::Line);
            pen.stroke(&[(9.5, 17.5), (14.0, 16.0), (19.0, 15.0)], false, Ink::Faint);
            pen.ellipse((18.0, 7.0), (2.6, 3.6));
        }
        Icon::PaintClone => {
            // A rubber stamp over its print, and a dashed copy of the print
            // beside it.
            pen.polygon(&[(9.0, 3.0), (15.0, 3.0), (14.0, 9.0), (10.0, 9.0)]);
            pen.polygon(&[(5.0, 9.0), (19.0, 9.0), (19.0, 13.0), (5.0, 13.0)]);
            pen.shape(&[(5.0, 16.0), (19.0, 16.0), (19.0, 18.0), (5.0, 18.0)], Paint::Tone);
            pen.stroke(&[(5.0, 21.5), (19.0, 21.5)], false, Ink::Dashed);
        }
        _ => {}
    }
}
