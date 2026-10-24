//! The modifier icons (decisions 0142 to 0150), one per kind of
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
        Icon::Scatter => scatter(pen),
        Icon::BendAlongCurve => bend_along_curve(pen),
        Icon::Shrinkwrap => shrinkwrap(pen),
        Icon::WeightedNormals => weighted_normals(pen),
        Icon::LiveBevel => live_bevel(pen),
        Icon::Quadify => quadify(pen),
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

/// Scatter: a curved hill with three little cones standing on it, each
/// leaning the way the ground faces where it stands.
fn scatter(pen: &mut Pen) {
    // The hill: part of a circle round (12, 27).
    let (cx, cy, r) = (12.0f32, 27.0f32, 13.0f32);
    let on_hill = |x: f32| (x, cy - (r * r - (x - cx) * (x - cx)).sqrt());
    let mut ground: Vec<(f32, f32)> = (0..=10).map(|i| on_hill(2.0 + 2.0 * i as f32)).collect();
    let hill = ground.clone();
    ground.extend([(22.0, 21.0), (2.0, 21.0)]);
    pen.shape(&ground, Paint::Tone);
    pen.stroke(&hill, false, Ink::Line);
    for x in [5.5, 12.0, 18.5] {
        let (px, py) = on_hill(x);
        // Out from the hill, and along it.
        let (nx, ny) = ((px - cx) / r, (py - cy) / r);
        let (tx, ty) = (-ny, nx);
        let cone = [
            (px + tx * 2.0, py + ty * 2.0),
            (px + nx * 5.0, py + ny * 5.0),
            (px - tx * 2.0, py - ty * 2.0),
        ];
        pen.shape(&cone, Paint::Solid);
        pen.stroke(&cone, true, Ink::Line);
    }
}

/// Bend along curve: a striped bar lying along a curve that runs on past
/// its ends.
fn bend_along_curve(pen: &mut Pen) {
    // A quarter circle round (4, 21), from the top to the right.
    let (cx, cy) = (4.0f32, 21.0f32);
    let at = |r: f32, t: f32| {
        let a = -std::f32::consts::FRAC_PI_2 * (1.0 - t);
        (cx + r * a.cos(), cy + r * a.sin())
    };
    let (inner, outer) = (11.0, 16.0);
    let mut band: Vec<(f32, f32)> = (0..=8).map(|i| at(outer, 0.1 + 0.1 * i as f32)).collect();
    band.extend((0..=8).rev().map(|i| at(inner, 0.1 + 0.1 * i as f32)));
    pen.shape(&band, Paint::Tone);
    pen.stroke(&band, true, Ink::Line);
    for t in [0.3, 0.5, 0.7] {
        pen.stroke(&[at(inner, t), at(outer, t)], false, Ink::Line);
    }
    // The curve it follows, through the bar's middle and past its ends.
    let curve: Vec<(f32, f32)> = (0..=12).map(|i| at(13.5, i as f32 / 12.0)).collect();
    pen.stroke(&curve, false, Ink::Dashed);
}

/// Shrinkwrap: a sheet pressed down onto a hump, hugging it, with arrows
/// pressing it on.
fn shrinkwrap(pen: &mut Pen) {
    // The hump: half a circle round (12, 21).
    let hump: Vec<(f32, f32)> = (0..=12)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
            (12.0 + 8.0 * a.cos(), 21.0 + 8.0 * a.sin())
        })
        .collect();
    pen.shape(&hump, Paint::Tone);
    pen.stroke(&hump, true, Ink::Faint);
    // The sheet, a little above it and flat beyond it.
    let mut sheet = vec![(2.0, 19.5)];
    sheet.extend((0..=12).map(|i| {
        let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
        (12.0 + 9.5 * a.cos(), 21.0 + 9.5 * a.sin())
    }));
    sheet.push((22.0, 19.5));
    pen.stroke(&sheet, false, Ink::Line);
    // Arrows pressing down.
    for x in [5.0, 19.0] {
        pen.stroke(&[(x, 3.0), (x, 9.0)], false, Ink::Line);
        pen.stroke(
            &[(x - 2.0, 7.0), (x, 9.0), (x + 2.0, 7.0)],
            false,
            Ink::Line,
        );
    }
}

/// Weighted normals: a bevelled box seen from the side. The arrows on its
/// flat top all point straight up (the big face shades flat); those on
/// its rounded corners fan round between the top and the sides.
fn weighted_normals(pen: &mut Pen) {
    // A corner arc of radius 4, centred at (cx, 12), from the side to the
    // top; `left` for the left corner.
    let arc = |cx: f32, left: bool| -> Vec<(f32, f32)> {
        (0..=6)
            .map(|i| {
                let t = i as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
                let (dx, dy) = (t.cos() * 4.0, -t.sin() * 4.0);
                (if left { cx - dx } else { cx + dx }, 12.0 + dy)
            })
            .collect()
    };
    let mut outline = vec![(4.0, 21.0)];
    outline.extend(arc(8.0, true));
    let mut right = arc(16.0, false);
    right.reverse();
    outline.extend(right);
    outline.push((20.0, 21.0));
    pen.shape(&outline, Paint::Tone);
    pen.stroke(&outline, false, Ink::Line);
    // Straight up from the flat top.
    for x in [9.5, 12.0, 14.5] {
        pen.stroke(&[(x, 7.0), (x, 3.0)], false, Ink::Line);
    }
    // Fanning round each corner: along the corner's own facing.
    for (cx, side) in [(8.0f32, -1.0f32), (16.0, 1.0)] {
        for t in [0.2f32, 0.5, 0.8] {
            let a = t * std::f32::consts::FRAC_PI_2;
            let (nx, ny) = (side * a.cos(), -a.sin());
            let start = (cx + nx * 4.0, 12.0 + ny * 4.0);
            pen.stroke(
                &[start, (start.0 + nx * 3.0, start.1 + ny * 3.0)],
                false,
                Ink::Faint,
            );
        }
    }
}

/// Live bevel: a block whose top left edge is rounded off, and a circling
/// arrow above its right: the bevel worked out again whenever the shape
/// changes.
fn live_bevel(pen: &mut Pen) {
    let block = rounded((3.0, 8.0), (17.0, 21.0), [7.0, 0.0, 0.0, 0.0]);
    pen.shape(&block, Paint::Tone);
    pen.stroke(&block, true, Ink::Line);
    // Three quarters of a circle round (18, 6), then an arrowhead.
    let (cx, cy, r) = (18.0_f32, 6.0_f32, 3.5_f32);
    let circle: Vec<(f32, f32)> = (0..=12)
        .map(|i| {
            let a = -0.25 * std::f32::consts::PI + i as f32 / 12.0 * 1.5 * std::f32::consts::PI;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    pen.stroke(&circle, false, Ink::Line);
    let (ex, ey) = circle[circle.len() - 1];
    pen.stroke(
        &[(ex - 2.2, ey - 0.6), (ex, ey), (ex + 0.4, ey - 2.2)],
        false,
        Ink::Line,
    );
}

/// Quadify: a four-sided polygon with its four corner points, made of
/// two triangles whose edge between them is going (dashed).
fn quadify(pen: &mut Pen) {
    let outline = [(4.0, 5.0), (20.0, 5.0), (20.0, 19.0), (4.0, 19.0)];
    pen.shape(&outline, Paint::Tone);
    pen.stroke(&[(4.0, 19.0), (20.0, 5.0)], false, Ink::Dashed);
    pen.stroke(&outline, true, Ink::Line);
    for (x, y) in outline {
        pen.dot(x, y, 1.8);
    }
}
