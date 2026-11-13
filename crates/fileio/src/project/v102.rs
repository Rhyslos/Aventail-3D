//! Version 102 of the project file ([decision 0275](../../../../docs/decisions/0275-edges-and-variation.md)):
//! version 101's, with the edge randomiser and variation on the shape
//! canvas. Each drawn piece (`v100::PieceV100`) gains its own seed, its
//! edge ([`EdgeV102`]: kind, amount, tooth size, seed), how its copies
//! vary ([`VaryV102`]) and its vein ([`VeinV102`], for a line). Files
//! before version 102 hold none, so their pieces open smooth, not varied,
//! with the seed each would have been given, and their lines with no vein
//! (so they look as they did); an older app would drop them all, so the
//! version goes up.
//!
//! Numbers read are brought within their ranges by
//! `shapes::Drawing::from_parts`, which refuses ones that aren't numbers
//! and linked pieces whose edges, varying or veins differ.

use serde::{Deserialize, Serialize};
use shapes::{Edge, EdgeKind, Piece, PieceId, Vary, Vein};

/// A piece's edge, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct EdgeV102 {
    /// 0 smooth, 1 jagged, 2 serrated, 3 wavy, 4 torn.
    pub(crate) kind: u8,
    /// How deep its teeth are.
    pub(crate) amount: f32,
    /// How wide one tooth is, a share of the texture.
    pub(crate) size: f32,
    /// Its seed.
    pub(crate) seed: u32,
}

/// How a piece's copies vary, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct VaryV102 {
    /// "Each copy differs".
    pub(crate) on: bool,
    /// Size ± (a share).
    pub(crate) size: f32,
    /// Angle ± (radians).
    pub(crate) angle: f32,
    /// Colour shift.
    pub(crate) colour: f32,
}

/// A line's vein, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct VeinV102 {
    /// How wide, a share of the texture.
    pub(crate) width: f32,
    /// Groove (below 0) or ridge.
    pub(crate) depth: f32,
    /// Darker (below 0) or lighter.
    pub(crate) shade: f32,
}

/// `piece`'s seed as stored: none when it is the one its id gives.
pub(crate) fn seed_of(piece: &Piece) -> Option<u32> {
    (piece.seed != shapes::drawing::first_seed(piece.id)).then_some(piece.seed)
}

/// The seed stored as `stored` for piece `id`.
pub(crate) fn seed_from(id: PieceId, stored: Option<u32>) -> u32 {
    stored.unwrap_or_else(|| shapes::drawing::first_seed(id))
}

/// `edge` as stored: none when smooth as new.
pub(crate) fn edge_of(edge: &Edge) -> Option<EdgeV102> {
    (*edge != Edge::SMOOTH).then_some(EdgeV102 {
        kind: match edge.kind {
            EdgeKind::Smooth => 0,
            EdgeKind::Jagged => 1,
            EdgeKind::Serrated => 2,
            EdgeKind::Wavy => 3,
            EdgeKind::Torn => 4,
        },
        amount: edge.amount,
        size: edge.size,
        seed: edge.seed,
    })
}

/// The edge stored as `stored`, or why it can't be read.
pub(crate) fn edge_from(stored: Option<EdgeV102>) -> Result<Edge, &'static str> {
    let Some(e) = stored else {
        return Ok(Edge::SMOOTH);
    };
    let kind = match e.kind {
        0 => EdgeKind::Smooth,
        1 => EdgeKind::Jagged,
        2 => EdgeKind::Serrated,
        3 => EdgeKind::Wavy,
        4 => EdgeKind::Torn,
        _ => return Err(words::text!("an edge is of a kind this app doesn't know")),
    };
    Ok(Edge {
        kind,
        amount: e.amount,
        size: e.size,
        seed: e.seed,
    })
}

/// `vary` as stored: none when as new.
pub(crate) fn vary_of(vary: &Vary) -> Option<VaryV102> {
    (*vary != Vary::OFF).then_some(VaryV102 {
        on: vary.on,
        size: vary.size,
        angle: vary.angle,
        colour: vary.colour,
    })
}

/// The varying stored as `stored`.
pub(crate) fn vary_from(stored: Option<VaryV102>) -> Vary {
    stored.map_or(Vary::OFF, |v| Vary {
        on: v.on,
        size: v.size,
        angle: v.angle,
        colour: v.colour,
    })
}

/// `vein` as stored: none when it is no vein (as lines before version
/// 102 were).
pub(crate) fn vein_of(vein: &Vein) -> Option<VeinV102> {
    (*vein != Vein::NONE).then_some(VeinV102 {
        width: vein.width,
        depth: vein.depth,
        shade: vein.shade,
    })
}

/// The vein stored as `stored`.
pub(crate) fn vein_from(stored: Option<VeinV102>) -> Vein {
    stored.map_or(Vein::NONE, |v| Vein {
        width: v.width,
        depth: v.depth,
        shade: v.shade,
    })
}
