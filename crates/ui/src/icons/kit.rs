//! The modular kit helpers' icons (decision 0160): the pivot presets,
//! each a box seen from the front with its pivot where the preset puts
//! it, and Size to kit grid, a box filling whole squares of a grid.
//! Drawn here, after nothing but the app's own pivot icons.

use super::Icon;
use super::pen::{Ink, Pen};
use super::tools::pivot;

/// The box the pivot presets show, seen from the front.
const BOX: [(f32, f32); 4] = [(4.0, 7.0), (20.0, 7.0), (20.0, 21.0), (4.0, 21.0)];

/// Draws `icon` with `pen`, if it is one of this file's icons:
/// [`super::shapes`] sends each icon to the file that draws it.
pub(super) fn draw(pen: &mut Pen, icon: Icon) {
    match icon {
        Icon::PivotPresets => {
            // The box with the presets' places marked, the pivot on one.
            pen.stroke(&BOX, true, Ink::Faint);
            for (x, y) in [(4.0, 21.0), (12.0, 14.0), (12.0, 7.0)] {
                pen.dot(x, y, 1.2);
            }
            pivot(pen, (12.0, 21.0));
        }
        Icon::PivotToBottomCentre => {
            pen.stroke(&BOX, true, Ink::Faint);
            pivot(pen, (12.0, 21.0));
        }
        Icon::PivotToBottomCorner => {
            pen.stroke(&BOX, true, Ink::Faint);
            pivot(pen, (4.0, 21.0));
        }
        Icon::PivotToTopCentre => {
            pen.stroke(&BOX, true, Ink::Faint);
            pivot(pen, (12.0, 7.0));
        }
        Icon::SizeToKitGrid => {
            // A grid, and a box grown to fill two of its squares.
            for at in [3.0, 12.0, 21.0] {
                pen.stroke(&[(at, 3.0), (at, 21.0)], false, Ink::Faint);
                pen.stroke(&[(3.0, at), (21.0, at)], false, Ink::Faint);
            }
            pen.stroke(
                &[(5.0, 14.0), (16.0, 14.0), (16.0, 19.0), (5.0, 19.0)],
                true,
                Ink::Dashed,
            );
            pen.stroke(
                &[(3.0, 12.0), (21.0, 12.0), (21.0, 21.0), (3.0, 21.0)],
                true,
                Ink::Line,
            );
        }
        _ => {}
    }
}
