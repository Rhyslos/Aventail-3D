//! The boolean icons (decision 0135): a square (the shape kept) and a
//! circle (the cutter) overlapping at a corner, the result drawn solid
//! and lightly filled, the cutter's own outline dashed where it is no
//! longer part of the shape. Original drawings.

use super::Icon;
use super::pen::{Ink, Paint, Pen};

/// The circle's middle and size on the 24 × 24 grid.
const CENTRE: (f32, f32) = (15.0, 15.0);
const RADIUS: f32 = 6.0;

/// Points along the circle from `from` to `to` degrees (0° to the right,
/// 90° down, as the grid runs).
fn arc(from: f32, to: f32) -> Vec<(f32, f32)> {
    let steps = 12;
    (0..=steps)
        .map(|i| {
            let a = (from + (to - from) * i as f32 / steps as f32).to_radians();
            (CENTRE.0 + RADIUS * a.cos(), CENTRE.1 + RADIUS * a.sin())
        })
        .collect()
}

pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::Union => {
            // Square and circle filled as one, with one outline round both.
            pen.fill((3.0, 3.0), (15.0, 15.0), 0.3);
            pen.disc(CENTRE, RADIUS, Paint::Tone);
            let mut outline = vec![(3.0, 3.0), (15.0, 3.0)];
            outline.extend(arc(-90.0, 180.0));
            outline.push((3.0, 15.0));
            pen.stroke(&outline, true, Ink::Line);
        }
        Icon::Subtract => {
            // The square with a round bite out of its corner; the cutter
            // dashed.
            pen.fill((3.0, 3.0), (15.0, 9.0), 0.3);
            pen.fill((3.0, 9.0), (9.0, 15.0), 0.3);
            pen.ring(CENTRE, (RADIUS, RADIUS), Ink::Dashed);
            let mut outline = vec![(3.0, 3.0), (15.0, 3.0)];
            outline.extend(arc(-90.0, -180.0));
            outline.push((3.0, 15.0));
            pen.stroke(&outline, true, Ink::Line);
        }
        Icon::Intersect => {
            // Only the overlap is left: a quarter circle, filled; the
            // square and circle dashed.
            pen.stroke(
                &[(3.0, 3.0), (15.0, 3.0), (15.0, 15.0), (3.0, 15.0)],
                true,
                Ink::Dashed,
            );
            pen.ring(CENTRE, (RADIUS, RADIUS), Ink::Dashed);
            let mut overlap = vec![CENTRE];
            overlap.extend(arc(-90.0, -180.0));
            pen.shape(&overlap, Paint::Tone);
            pen.stroke(&overlap, true, Ink::Line);
        }
        // Drawn by the other icon files.
        _ => {}
    }
}
