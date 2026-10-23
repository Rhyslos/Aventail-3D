//! The interface's everyday icons: Undo and Redo, the selection tools,
//! world or object coordinates, snapping, symmetry, search, the modes,
//! and the small icons of the viewport and the panels (shown, hidden).

use super::Icon;
use super::pen::Pen;
use std::f32::consts::{PI, TAU};

/// Draws `icon` with `pen`, if it is one of this file's icons:
/// [`super::shapes`] sends each icon to the file that draws it.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::Undo => {
            pen.polyline(&[(9.0, 14.0), (4.0, 9.0), (9.0, 4.0)]);
            let mut curve = pen.points(&[(4.0, 9.0)]);
            curve.extend(pen.arc((14.5, 14.5), (5.5, 5.5), -PI / 2.0, PI / 2.0));
            curve.push(pen.p(11.0, 20.0));
            pen.line(curve);
        }
        Icon::Redo => {
            pen.polyline(&[(15.0, 14.0), (20.0, 9.0), (15.0, 4.0)]);
            let mut curve = pen.points(&[(20.0, 9.0)]);
            curve.extend(pen.arc((9.5, 14.5), (5.5, 5.5), -PI / 2.0, -1.5 * PI));
            curve.push(pen.p(13.0, 20.0));
            pen.line(curve);
        }
        Icon::PaintSelect => {
            // A dashed circle with a plus in it.
            for i in 0..12 {
                let a = i as f32 / 12.0 * TAU;
                let dash = pen.arc((12.0, 12.0), (8.0, 8.0), a, a + TAU / 24.0);
                pen.line(dash);
            }
            pen.polyline(&[(12.0, 9.0), (12.0, 15.0)]);
            pen.polyline(&[(9.0, 12.0), (15.0, 12.0)]);
        }
        Icon::BoxSelection => {
            // A dashed square with a pointer at its corner.
            for i in 0..4 {
                let at = 4.0 + i as f32 * 4.0;
                pen.segment((at, 4.0), (at + 2.5, 4.0), 1.5, 1.0);
                pen.segment((4.0, at), (4.0, at + 2.5), 1.5, 1.0);
                pen.segment((at, 18.0), (at + 2.0, 18.0), 1.5, 1.0);
                pen.segment((18.0, at), (18.0, at + 2.0), 1.5, 1.0);
            }
            pen.polygon(&[(14.0, 14.0), (21.0, 17.5), (17.5, 18.0), (18.5, 21.5)]);
        }
        Icon::LassoSelection => {
            // A loose dashed loop with a tail.
            for i in 0..10 {
                let a = i as f32 / 10.0 * TAU;
                let dash = pen.arc((12.0, 10.0), (9.0, 6.5), a, a + TAU / 20.0);
                pen.line(dash);
            }
            pen.polyline(&[(7.0, 15.0), (6.0, 18.5), (8.5, 21.0)]);
        }
        Icon::VisibleOnly => {
            // A box seen from the front, right and above: the edges in
            // sight solid, the hidden ones barely there.
            pen.polygon(&[(4.0, 9.0), (15.0, 9.0), (15.0, 20.0), (4.0, 20.0)]);
            pen.polyline(&[(4.0, 9.0), (9.0, 4.0), (20.0, 4.0), (15.0, 9.0)]);
            pen.polyline(&[(20.0, 4.0), (20.0, 15.0), (15.0, 20.0)]);
            pen.segment((9.0, 4.0), (9.0, 15.0), 1.0, 0.3);
            pen.segment((9.0, 15.0), (20.0, 15.0), 1.0, 0.3);
            pen.segment((9.0, 15.0), (4.0, 20.0), 1.0, 0.3);
        }
        Icon::World => {
            pen.ellipse((12.0, 12.0), (9.0, 9.0));
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.ellipse((12.0, 12.0), (3.8, 9.0));
        }
        Icon::Null => {
            pen.polyline(&[(12.0, 3.0), (12.0, 21.0)]);
            pen.polyline(&[(3.0, 12.0), (21.0, 12.0)]);
            pen.ellipse((12.0, 12.0), (3.0, 3.0));
        }
        Icon::Snap => {
            // A horseshoe magnet: down the left side, round the bottom
            // (from the left, through the bottom, to the right), and up.
            let mut magnet = pen.points(&[(6.0, 3.0), (6.0, 11.0)]);
            magnet.extend(pen.arc((12.0, 11.0), (6.0, 6.0), PI, 0.0));
            magnet.push(pen.p(18.0, 3.0));
            pen.line(magnet);
            pen.polyline(&[(6.0, 7.0), (10.0, 7.0)]);
            pen.polyline(&[(14.0, 7.0), (18.0, 7.0)]);
        }
        Icon::Symmetry => {
            for i in 0..6 {
                let y = 3.0 + i as f32 * 3.0;
                pen.polyline(&[(12.0, y), (12.0, y + 1.5)]);
            }
            pen.polygon(&[(9.0, 6.0), (3.0, 12.0), (9.0, 18.0)]);
            pen.polygon(&[(15.0, 6.0), (21.0, 12.0), (15.0, 18.0)]);
        }
        Icon::Search => {
            pen.ellipse((11.0, 11.0), (7.0, 7.0));
            pen.polyline(&[(16.0, 16.0), (20.5, 20.5)]);
        }
        Icon::ObjectMode => {
            pen.polyline(&[(12.0, 12.0), (12.0, 3.0)]);
            pen.polyline(&[(12.0, 12.0), (20.0, 17.0)]);
            pen.polyline(&[(12.0, 12.0), (4.0, 17.0)]);
            pen.dot(12.0, 12.0, 2.2);
        }
        Icon::Points => {
            pen.square((5.0, 5.0), (19.0, 19.0), true);
            for &(x, y) in &[(5.0, 5.0), (19.0, 5.0), (5.0, 19.0), (19.0, 19.0)] {
                pen.dot(x, y, 2.4);
            }
        }
        Icon::Edges => {
            pen.square((5.0, 5.0), (19.0, 19.0), true);
            pen.segment((5.0, 19.0), (19.0, 19.0), 2.0, 1.0);
        }
        Icon::Polygons => {
            pen.fill((5.0, 5.0), (19.0, 19.0), 0.6);
        }
        Icon::Uv => {
            pen.square((4.0, 4.0), (20.0, 20.0), false);
            pen.segment((4.0, 12.0), (20.0, 12.0), 1.0, 0.5);
            pen.segment((12.0, 4.0), (12.0, 20.0), 1.0, 0.5);
            pen.fill((6.5, 6.5), (10.0, 10.0), 1.0);
        }
        Icon::Workplane => {
            pen.polygon(&[(3.0, 16.0), (8.0, 8.0), (21.0, 8.0), (16.0, 16.0)]);
            pen.segment((10.0, 12.0), (18.0, 12.0), 1.0, 0.5);
        }
        Icon::Solo => {
            pen.ellipse((12.0, 12.0), (10.0, 6.0));
            pen.ellipse((12.0, 12.0), (3.0, 3.0));
        }
        Icon::GridDetail => {
            // A square of ground with strong 1 m lines and faint fine ones.
            pen.square((4.0, 4.0), (20.0, 20.0), false);
            pen.segment((12.0, 4.0), (12.0, 20.0), 1.0, 1.0);
            pen.segment((4.0, 12.0), (20.0, 12.0), 1.0, 1.0);
            for at in [8.0, 16.0] {
                pen.segment((at, 4.0), (at, 20.0), 0.6, 0.5);
                pen.segment((4.0, at), (20.0, at), 0.6, 0.5);
            }
        }
        Icon::Dolly => {
            pen.polyline(&[(12.0, 3.5), (12.0, 20.5)]);
            pen.polyline(&[(8.0, 7.5), (12.0, 3.5), (16.0, 7.5)]);
            pen.polyline(&[(8.0, 16.5), (12.0, 20.5), (16.0, 16.5)]);
        }
        Icon::Maximise => {
            pen.square((4.5, 4.5), (19.5, 19.5), false);
        }
        Icon::Menu => {
            for y in [7.0, 12.0, 17.0] {
                pen.polyline(&[(4.0, y), (20.0, y)]);
            }
        }
        Icon::Mouse => {
            let mut body = pen.arc((12.0, 9.0), (6.0, 6.0), PI, TAU);
            body.extend(pen.arc((12.0, 15.0), (6.0, 6.0), 0.0, PI));
            pen.closed(body);
            pen.polyline(&[(12.0, 3.0), (12.0, 9.0)]);
        }
        Icon::Shown => {
            // An almond-shaped eye with its pupil.
            // Two arcs meeting at the corners (4, 12) and (20, 12).
            let mut eye = pen.arc((12.0, 15.9), (8.9, 8.9), -2.688, -0.453);
            eye.extend(pen.arc((12.0, 8.1), (8.9, 8.9), 0.453, 2.688));
            pen.closed(eye);
            pen.dot(12.0, 12.0, 2.6);
        }
        Icon::Hidden => {
            // A closed eye: the lower lid, with three lashes.
            let lid = pen.arc((12.0, 8.1), (8.9, 8.9), 0.453, 2.688);
            pen.line(lid);
            pen.polyline(&[(12.0, 17.0), (12.0, 20.0)]);
            pen.polyline(&[(7.0, 15.5), (5.5, 18.0)]);
            pen.polyline(&[(17.0, 15.5), (18.5, 18.0)]);
        }
        // Drawn by the other icon files.
        _ => {}
    }
}
