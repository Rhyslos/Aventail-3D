//! Version 101 of the project file ([decision 0274](../../../../docs/decisions/0274-assemblies.md)):
//! version 100's, with assemblies on the shape canvas. Each drawn piece
//! (`v100::PieceV100`) gains where it is placed (its shape moved, turned,
//! sized or flipped), its colour, its height, its link (pieces sharing
//! one shape are linked copies) and its group; each drawing
//! (`v100::DrawingV100`) gains its groups ([`GroupV101`]). Files before
//! version 101 hold none, so their pieces open where they were drawn,
//! white, full height, each with a shape of its own and in no group; an
//! older app would drop them all (and draw every copy in one place), so
//! the version goes up.
//!
//! Everything read is checked by `shapes::Drawing::from_parts`: linked
//! pieces must share one shape, a group's pieces must be together, and a
//! place must not squash a piece flat.

use glam::{Mat2, Vec2};
use serde::{Deserialize, Serialize};
use shapes::{Change, Group, GroupId, LinkId, Piece, PieceId};

/// One group of pieces, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GroupV101 {
    /// Its id.
    pub(crate) id: u32,
    /// Its name.
    pub(crate) name: String,
}

/// A piece's colour stored before version 101: white.
pub(crate) fn standard_colour() -> [f32; 3] {
    shapes::drawing::STANDARD_COLOUR
}

/// A piece's height stored before version 101: full.
pub(crate) fn standard_height() -> f32 {
    1.0
}

/// Where `piece` is placed, as stored: its turning and sizing (across,
/// then down, by columns) and its move. `None` when it isn't moved at
/// all, as pieces before version 101 weren't.
pub(crate) fn place_of(piece: &Piece) -> Option<[f32; 6]> {
    let c = Change::IDENTITY.then(&piece.place);
    (c != Change::IDENTITY).then(|| {
        let (x, y) = (c.matrix.x_axis, c.matrix.y_axis);
        [x.x, x.y, y.x, y.y, c.by.x, c.by.y]
    })
}

/// The place stored as `stored` (`None`: not moved).
pub(crate) fn place_from(stored: Option<[f32; 6]>) -> Change {
    match stored {
        None => Change::IDENTITY,
        Some([a, b, c, d, x, y]) => Change {
            about: Vec2::ZERO,
            matrix: Mat2::from_cols(Vec2::new(a, b), Vec2::new(c, d)),
            by: Vec2::new(x, y),
        },
    }
}

/// `piece`'s link as stored: `None` when it is its own (a piece on its
/// own, or the first of its linked copies to be drawn).
pub(crate) fn link_of(piece: &Piece) -> Option<u32> {
    (piece.link.0 != piece.id.0).then_some(piece.link.0)
}

/// The link stored as `stored` for piece `id`.
pub(crate) fn link_from(id: PieceId, stored: Option<u32>) -> LinkId {
    LinkId(stored.unwrap_or(id.0))
}

/// The groups stored as `stored`.
pub(crate) fn groups_from(stored: impl IntoIterator<Item = GroupV101>) -> Vec<Group> {
    stored
        .into_iter()
        .map(|g| Group {
            id: GroupId(g.id),
            name: g.name,
        })
        .collect()
}

/// `groups` as stored.
pub(crate) fn groups_of(groups: &[Group]) -> crate::limits::Bounded<GroupV101> {
    groups
        .iter()
        .map(|g| GroupV101 {
            id: g.id.0,
            name: g.name.clone(),
        })
        .collect()
}
