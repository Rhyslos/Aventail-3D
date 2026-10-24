//! The modifier icons (decisions 0142 to 0145), one per kind of
//! modifier on the modifiers palette, and the crease commands'. Original
//! drawings.

use super::Icon;
use super::pen::{Ink, Paint, Pen};

pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::Remesh => remesh(pen),
        Icon::Subdivide => subdivide(pen),
        Icon::Creases => creases(pen),
        Icon::CreaseEdges => crease_edges(pen),
        Icon::Uncrease => uncrease(pen),
        Icon::Lattice => lattice(pen),
        Icon::Decimate => decimate(pen),
        _ => {}
    }
}

/// The outline of a square from `min` to `max` with its corners rounded
/// by `radius` (each corner's own: top left, top right, bottom right,
/// bottom left), clockwise from the top left.
fn rounded(min: (f32, f32), max: (f32, f32), radius: [f32; 4]) -> Vec<(f32, f32)> {
    let corners = [
        ((min.0, min.1), (1.0, 1.0), std::f32::consts::PI),
        ((max.0, min.1), (-1.0, 1.0), 1.5 * std::f32::consts::PI),
        ((max.0, max.1), (-1.0, -1.0), 0.0),
        ((min.0, max.1), (1.0, -1.0), 0.5 * std::f32::consts::PI),
    ];
    let mut out = Vec::new();
    for (((x, y), (sx, sy), start), r) in corners.into_iter().zip(radius) {
        if r <= 0.0 {
            out.push((x, y));
            continue;
        }
        let (cx, cy) = (x + sx * r, y + sy * r);
        for k in 0..=6 {
            let a = start + k as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
            out.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    out
}

/// Subdivide: a square cage, dashed, with the smooth rounded shape it
/// makes inside.
fn subdivide(pen: &mut Pen) {
    let shape = rounded((5.0, 5.0), (19.0, 19.0), [5.5; 4]);
    pen.shape(&shape, Paint::Tone);
    pen.stroke(&shape, true, Ink::Line);
    pen.stroke(
        &[(3.0, 3.0), (21.0, 3.0), (21.0, 21.0), (3.0, 21.0)],
        true,
        Ink::Dashed,
    );
}

/// A shape rounded at the top and kept sharp along its bottom edge: the
/// crease drawn strong.
fn half_creased(pen: &mut Pen) {
    let shape = rounded((4.0, 5.0), (20.0, 19.0), [6.0, 6.0, 0.0, 0.0]);
    pen.shape(&shape, Paint::Tone);
    pen.stroke(&shape, true, Ink::Line);
}

/// Creases (the group): a shape sharp along one edge and rounded along
/// the rest, with a dashed rounding beside it.
fn creases(pen: &mut Pen) {
    half_creased(pen);
    pen.segment((4.0, 19.0), (20.0, 19.0), 2.5, 1.0);
    pen.stroke(&[(4.0, 22.0), (20.0, 22.0)], false, Ink::Dashed);
}

/// Crease edges: the sharp edge drawn bold, a corner kept square.
fn crease_edges(pen: &mut Pen) {
    half_creased(pen);
    pen.segment((4.0, 19.0), (20.0, 19.0), 2.5, 1.0);
}

/// Uncrease: rounded all round now, the edge that was sharp dashed.
fn uncrease(pen: &mut Pen) {
    let shape = rounded((4.0, 5.0), (20.0, 19.0), [6.0; 4]);
    pen.shape(&shape, Paint::Tone);
    pen.stroke(&shape, true, Ink::Line);
    pen.stroke(&[(4.0, 19.0), (4.0, 13.0)], false, Ink::Dashed);
    pen.stroke(
        &[(4.0, 19.0), (20.0, 19.0), (20.0, 13.0)],
        false,
        Ink::Dashed,
    );
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

/// Lattice: a cage of three by three points with its right side pulled
/// out, and the shape inside swelling with it.
fn lattice(pen: &mut Pen) {
    // The cage's points, row by row: the right column bulges outward.
    let cage: [[(f32, f32); 3]; 3] = [
        [(3.0, 4.0), (11.0, 4.0), (19.0, 2.5)],
        [(3.0, 12.0), (11.0, 12.0), (22.0, 12.0)],
        [(3.0, 20.0), (11.0, 20.0), (19.0, 21.5)],
    ];
    let inside = [
        (6.0, 7.0),
        (11.0, 7.0),
        (16.0, 6.5),
        (19.0, 12.0),
        (16.0, 17.5),
        (11.0, 17.0),
        (6.0, 17.0),
    ];
    pen.shape(&inside, Paint::Tone);
    pen.stroke(&inside, true, Ink::Faint);
    for row in &cage {
        pen.stroke(row, false, Ink::Line);
    }
    for column in 0..3 {
        let line: Vec<(f32, f32)> = cage.iter().map(|row| row[column]).collect();
        pen.stroke(&line, false, Ink::Line);
    }
    for (x, y) in cage.iter().flatten() {
        pen.dot(*x, *y, 1.6);
    }
}

/// Decimate: a square of many small triangles on the left becoming a few
/// big ones on the right.
fn decimate(pen: &mut Pen) {
    let outline = [(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0)];
    pen.shape(&outline, Paint::Tone);
    // The dense half: a grid of small squares, each cut corner to corner.
    for x in [8.0, 12.0] {
        pen.stroke(&[(x, 4.0), (x, 20.0)], false, Ink::Faint);
    }
    for y in [8.0, 12.0, 16.0] {
        pen.stroke(&[(4.0, y), (12.0, y)], false, Ink::Faint);
    }
    for (x, y) in [(4.0, 4.0), (8.0, 4.0)] {
        pen.stroke(&[(x, y), (x + 4.0, y + 16.0)], false, Ink::Faint);
    }
    // The sparse half: two big triangles.
    pen.stroke(&[(12.0, 20.0), (20.0, 4.0)], false, Ink::Line);
    pen.stroke(&outline, true, Ink::Line);
}
