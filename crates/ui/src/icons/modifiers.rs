//! The modifier icons (decision 0142), one per kind of modifier on the
//! modifiers palette. Original drawings.

use super::Icon;
use super::pen::{Ink, Paint, Pen};

pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    if icon == Icon::Remesh {
        remesh(pen);
    }
}

/// Remesh: a lumpy outline on the left made even on the right: a round
/// shape covered by a regular grid of squares.
fn remesh(pen: &mut Pen) {
    let (cx, cy, r) = (12.0_f32, 12.0_f32, 8.5_f32);
    pen.disc((cx, cy), r, Paint::Tone);
    // The even grid, each line cut where it meets the circle.
    for offset in [-4.5_f32, 0.0, 4.5] {
        let half = (r * r - offset * offset).max(0.0).sqrt();
        pen.stroke(
            &[(cx + offset, cy - half), (cx + offset, cy + half)],
            false,
            Ink::Faint,
        );
        pen.stroke(
            &[(cx - half, cy + offset), (cx + half, cy + offset)],
            false,
            Ink::Faint,
        );
    }
    pen.ring((cx, cy), (r, r), Ink::Line);
}
