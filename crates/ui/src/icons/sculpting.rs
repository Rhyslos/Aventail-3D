//! The sculpting brushes' icons (decision 0306), also the pictures on the
//! shelf's brush cards: the surface seen from the side, dashed as it was
//! and solid after a stroke, with an arrow where the brush pulls. Drawn
//! for the app, in the same 24-unit box and line weights as the others.

use super::pen::{Ink, Paint, Pen};
use sculpt::BrushKind;

/// The flat surface's height on the design grid.
const GROUND: f32 = 16.0;

/// A small arrow from `from` to `to`, its head filled.
fn arrow(pen: &mut Pen, from: (f32, f32), to: (f32, f32)) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / length, dy / length);
    let back = (to.0 - ux * 3.0, to.1 - uy * 3.0);
    pen.stroke(&[from, back], false, Ink::Line);
    pen.shape(
        &[
            to,
            (back.0 - uy * 2.0, back.1 + ux * 2.0),
            (back.0 + uy * 2.0, back.1 - ux * 2.0),
        ],
        Paint::Solid,
    );
}

/// A smooth bump of `height` (up: less than nothing) and half-width `half`
/// centred on `x0`, sampled across the box.
fn bump(x0: f32, half: f32, height: f32) -> Vec<(f32, f32)> {
    (0..=20)
        .map(|i| {
            let x = 2.0 + i as f32;
            let t = ((x - x0) / half).abs().min(1.0);
            let s = 1.0 - t * t * (3.0 - 2.0 * t);
            (x, GROUND + height * s)
        })
        .collect()
}

/// The flat surface, dashed: how it was.
fn flat(pen: &mut Pen) {
    pen.stroke(&[(2.0, GROUND), (22.0, GROUND)], false, Ink::Dashed);
}

/// A line with a lump at x 8 and a pit at x 16, dashed: the uneven
/// surface the levelling brushes start from.
fn lumpy() -> Vec<(f32, f32)> {
    bump(8.0, 4.0, -5.0)
        .into_iter()
        .zip(bump(16.0, 4.0, 4.0))
        .map(|((x, a), (_, b))| (x, a + b - GROUND))
        .collect()
}

/// Draws brush `kind`'s icon with `pen`.
pub(super) fn draw(pen: &mut Pen, kind: BrushKind) {
    match kind {
        BrushKind::Draw => {
            flat(pen);
            pen.polyline(&bump(12.0, 7.0, -6.0));
            arrow(pen, (12.0, 13.0), (12.0, 4.0));
        }
        BrushKind::Smooth => {
            let rough: Vec<_> = (0..=10)
                .map(|i| {
                    let y = if i % 2 == 0 { 12.0 } else { 18.0 };
                    (2.0 + 2.0 * i as f32, y)
                })
                .collect();
            pen.stroke(&rough, false, Ink::Dashed);
            pen.polyline(&bump(12.0, 12.0, -1.5));
        }
        BrushKind::ClayStrips => {
            pen.stroke(&bump(12.0, 6.0, 3.0), false, Ink::Dashed);
            // A square-edged slab laid on top.
            pen.shape(
                &[(7.0, GROUND), (7.5, 10.0), (16.5, 10.0), (17.0, GROUND)],
                Paint::Tone,
            );
            pen.polyline(&[
                (2.0, GROUND),
                (7.0, GROUND),
                (7.5, 10.0),
                (16.5, 10.0),
                (17.0, GROUND),
                (22.0, GROUND),
            ]);
        }
        BrushKind::Clay => {
            pen.stroke(&bump(12.0, 6.0, 3.0), false, Ink::Dashed);
            let soft: Vec<_> = (0..=20)
                .map(|i| {
                    let x = 2.0 + i as f32;
                    let t = ((x - 12.0) / 7.0).abs().min(1.0);
                    (x, GROUND - 5.0 * (1.0 - t.powi(4)))
                })
                .collect();
            pen.polyline(&soft);
        }
        BrushKind::Inflate => {
            pen.ring((12.0, 13.0), (4.0, 4.0), Ink::Dashed);
            pen.ring((12.0, 13.0), (7.5, 7.5), Ink::Line);
            arrow(pen, (12.0, 13.0), (12.0, 3.5));
            arrow(pen, (12.0, 13.0), (21.5, 13.0));
        }
        BrushKind::Crease => {
            flat(pen);
            pen.polyline(&[
                (2.0, GROUND - 1.0),
                (8.0, GROUND - 1.0),
                (12.0, 22.0),
                (16.0, GROUND - 1.0),
                (22.0, GROUND - 1.0),
            ]);
            arrow(pen, (4.0, 9.0), (10.0, 9.0));
            arrow(pen, (20.0, 9.0), (14.0, 9.0));
        }
        BrushKind::Pinch => {
            pen.stroke(&bump(12.0, 8.0, -6.0), false, Ink::Dashed);
            pen.polyline(&[
                (2.0, GROUND),
                (9.0, 14.0),
                (12.0, 6.0),
                (15.0, 14.0),
                (22.0, GROUND),
            ]);
            arrow(pen, (3.0, 8.0), (8.5, 8.0));
            arrow(pen, (21.0, 8.0), (15.5, 8.0));
        }
        BrushKind::Flatten => {
            pen.stroke(&lumpy(), false, Ink::Dashed);
            pen.polyline(&[(2.0, GROUND), (22.0, GROUND)]);
            pen.fill((6.0, 4.0), (18.0, 6.0), 0.6);
        }
        BrushKind::Scrape => {
            pen.stroke(&lumpy(), false, Ink::Dashed);
            let cut: Vec<_> = lumpy()
                .into_iter()
                .map(|(x, y)| (x, y.max(GROUND - 1.0)))
                .collect();
            pen.polyline(&cut);
            arrow(pen, (8.0, 4.0), (8.0, 10.0));
        }
        BrushKind::Fill => {
            pen.stroke(&lumpy(), false, Ink::Dashed);
            let filled: Vec<_> = lumpy()
                .into_iter()
                .map(|(x, y)| (x, y.min(GROUND + 1.0)))
                .collect();
            pen.polyline(&filled);
            arrow(pen, (16.0, 23.0), (16.0, 18.5));
        }
        BrushKind::Grab => {
            flat(pen);
            let pulled: Vec<_> = (0..=20)
                .map(|i| {
                    let x = 2.0 + i as f32;
                    let t = ((x - 11.0) / 6.0).abs().min(1.0);
                    let s = 1.0 - t * t * (3.0 - 2.0 * t);
                    (x + 3.0 * s, GROUND - 9.0 * s)
                })
                .collect();
            pen.polyline(&pulled);
            arrow(pen, (8.0, 15.0), (13.0, 8.0));
        }
        BrushKind::ElasticGrab => {
            flat(pen);
            let bent: Vec<_> = (0..=20)
                .map(|i| {
                    let x = 2.0 + i as f32;
                    let r = (x - 12.0).abs();
                    let s = 3.0 / (3.0 + r);
                    (x + 2.0 * s, GROUND - 8.0 * s)
                })
                .collect();
            pen.polyline(&bent);
            arrow(pen, (9.0, 15.0), (13.0, 9.0));
        }
        BrushKind::SnakeHook => {
            flat(pen);
            pen.polyline(&[
                (2.0, GROUND),
                (8.0, GROUND),
                (9.5, 11.0),
                (11.0, 7.0),
                (14.0, 4.0),
                (18.0, 3.5),
                (20.0, 5.0),
            ]);
            pen.polyline(&[
                (16.0, GROUND),
                (13.5, 11.0),
                (14.5, 8.0),
                (17.0, 6.5),
                (20.0, 5.0),
            ]);
            pen.polyline(&[(16.0, GROUND), (22.0, GROUND)]);
        }
        BrushKind::Mask => {
            // A lumpy surface, its left part shaded as masked, and the
            // brush painting the mask over it.
            pen.shape(
                &[(2.0, GROUND), (2.0, 12.0), (12.0, 12.0), (12.0, GROUND)],
                Paint::Tone,
            );
            pen.polyline(&lumpy());
            arrow(pen, (17.0, 4.0), (12.0, 10.0));
        }
    }
}
