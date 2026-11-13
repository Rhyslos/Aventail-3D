//! Version 100 of the project file ([decision 0273](../../../../docs/decisions/0273-shape-canvas.md)):
//! version 99's, with what is drawn on each Shape box's canvas
//! (`v95::BoxV95::drawing`, stored as [`DrawingV100`]): its pieces (each
//! path's points and handles, Add, Cut out or Line, mirrored or not), the
//! mirror line and the reference image. Files before version 100 hold
//! none; an older app would drop the drawings, so the version goes up.
//!
//! Everything read is checked by `shapes::Drawing::from_parts`, and the
//! reference image's pixels must be exactly its size.

use super::v1::Problem;
use crate::limits::Bounded;
use glam::Vec2;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use shapes::{Drawing, DrawingParts, Knot, Mirror, Path, Piece, PieceId, PieceKind, Reference};
use std::fmt;
use std::sync::Arc;
use words::{tr, trf};

/// What is drawn on a Shape box's canvas, as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DrawingV100 {
    /// The id the next piece gets.
    pub(crate) next_id: u32,
    /// Its pieces, in order (the last on top).
    pub(crate) pieces: Bounded<PieceV100>,
    /// Whether the mirror line mirrors.
    pub(crate) mirror_on: bool,
    /// Where the mirror line is across the texture.
    pub(crate) mirror_x: f32,
    /// The reference image under the canvas.
    pub(crate) reference: Option<ReferenceV100>,
    /// Its groups of pieces (version 101, see `v101`).
    #[serde(default = "Bounded::new")]
    pub(crate) groups: Bounded<super::v101::GroupV101>,
}

/// One piece.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PieceV100 {
    /// Its id.
    pub(crate) id: u32,
    /// Its name.
    pub(crate) name: String,
    /// What it does: 0 adds, 1 cuts out, 2 is a line.
    pub(crate) kind: u8,
    /// Whether the mirror line mirrors it.
    pub(crate) mirrored: bool,
    /// Whether its path is closed.
    pub(crate) closed: bool,
    /// Its points: where each is, then its incoming and outgoing handles
    /// (from the point), across then down.
    pub(crate) knots: Bounded<[f32; 6]>,
    /// Where it is placed (version 101, see `v101::place_of`); none: where
    /// its points are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) place: Option<[f32; 6]>,
    /// Its colour (version 101).
    #[serde(default = "super::v101::standard_colour")]
    pub(crate) colour: [f32; 3],
    /// Its height (version 101).
    #[serde(default = "super::v101::standard_height")]
    pub(crate) height: f32,
    /// The link it shares with its linked copies (version 101); none: its
    /// own id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) link: Option<u32>,
    /// Its group's id (version 101).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) group: Option<u32>,
    /// Its own seed (version 102, see `v102::seed_of`); none: from its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) seed: Option<u32>,
    /// Its edge (version 102); none: smooth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) edge: Option<super::v102::EdgeV102>,
    /// How its copies vary (version 102); none: not at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) vary: Option<super::v102::VaryV102>,
    /// Its vein (version 102); none: no vein.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) vein: Option<super::v102::VeinV102>,
}

/// The reference image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ReferenceV100 {
    /// Its name.
    pub(crate) name: String,
    /// How strongly it shows.
    pub(crate) opacity: f32,
    /// Pixels across.
    pub(crate) width: u32,
    /// Pixels down.
    pub(crate) height: u32,
    /// Its pixels, four bytes each.
    pub(crate) rgba: ReferenceBytes,
}

/// A reference image's pixels, saved as one block of bytes and read only
/// up to the largest a reference image may be.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReferenceBytes(pub(crate) Vec<u8>);

/// The most bytes a reference image's pixels take.
const MAX_REFERENCE_BYTES: usize =
    shapes::reference::MAX_SIDE as usize * shapes::reference::MAX_SIDE as usize * 4;

impl Serialize for ReferenceBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for ReferenceBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Pixels;

        impl<'de> Visitor<'de> for Pixels {
            type Value = ReferenceBytes;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(tr("a reference image's pixels"))
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<ReferenceBytes, E> {
                self.visit_byte_buf(bytes.to_vec())
            }

            fn visit_byte_buf<E: serde::de::Error>(
                self,
                bytes: Vec<u8>,
            ) -> Result<ReferenceBytes, E> {
                if bytes.len() > MAX_REFERENCE_BYTES {
                    return Err(E::custom(tr("a reference image is too large")));
                }
                if !crate::limits::spend(bytes.len()) {
                    return Err(E::custom(tr(crate::limits::TOO_MUCH)));
                }
                Ok(ReferenceBytes(bytes))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<ReferenceBytes, A::Error> {
                // Formats without byte blocks (a graph file's JSON) save
                // the pixels as a list.
                let mut bytes = Vec::new();
                while let Some(byte) = seq.next_element::<u8>()? {
                    if bytes.len() == MAX_REFERENCE_BYTES {
                        return Err(serde::de::Error::custom(tr(
                            "a reference image is too large",
                        )));
                    }
                    bytes.push(byte);
                }
                self.visit_byte_buf(bytes)
            }
        }

        deserializer.deserialize_byte_buf(Pixels)
    }
}

/// `drawing` as stored.
pub(crate) fn drawing_of(drawing: &Drawing) -> DrawingV100 {
    let parts = drawing.to_parts();
    DrawingV100 {
        next_id: parts.next_id,
        pieces: parts
            .pieces
            .iter()
            .map(|p| PieceV100 {
                id: p.id.0,
                name: p.name.clone(),
                kind: match p.kind {
                    PieceKind::Add => 0,
                    PieceKind::Cut => 1,
                    PieceKind::Line => 2,
                },
                mirrored: p.mirrored,
                closed: p.path.is_closed(),
                knots: p
                    .path
                    .knots()
                    .iter()
                    .map(|k| [k.at.x, k.at.y, k.inn.x, k.inn.y, k.out.x, k.out.y])
                    .collect(),
                place: super::v101::place_of(p),
                colour: p.colour,
                height: p.height,
                link: super::v101::link_of(p),
                group: p.group.map(|g| g.0),
                seed: super::v102::seed_of(p),
                edge: super::v102::edge_of(&p.edge),
                vary: super::v102::vary_of(&p.vary),
                vein: super::v102::vein_of(&p.vein),
            })
            .collect(),
        groups: super::v101::groups_of(&parts.groups),
        mirror_on: parts.mirror.on,
        mirror_x: parts.mirror.x,
        reference: parts.reference.map(|r| ReferenceV100 {
            name: r.name.clone(),
            opacity: r.opacity,
            width: r.width(),
            height: r.height(),
            rgba: ReferenceBytes(r.rgba().to_vec()),
        }),
    }
}

/// The drawing stored as `stored`, checked.
pub(crate) fn drawing_from(stored: DrawingV100) -> Result<Drawing, Problem> {
    let damaged = |why: &'static str| trf!("a drawn shape is damaged: {}", tr(why));
    let mut pieces = Vec::with_capacity(stored.pieces.len());
    for p in stored.pieces.0 {
        let kind = match p.kind {
            0 => PieceKind::Add,
            1 => PieceKind::Cut,
            2 => PieceKind::Line,
            _ => {
                return Err(damaged(words::text!(
                    "a piece is of a kind this app doesn't know"
                )));
            }
        };
        let knots = p
            .knots
            .0
            .into_iter()
            .map(|[x, y, ix, iy, ox, oy]| Knot {
                at: Vec2::new(x, y),
                inn: Vec2::new(ix, iy),
                out: Vec2::new(ox, oy),
            })
            .collect();
        let edge = super::v102::edge_from(p.edge).map_err(damaged)?;
        pieces.push(Piece {
            id: PieceId(p.id),
            seed: super::v102::seed_from(PieceId(p.id), p.seed),
            edge,
            vary: super::v102::vary_from(p.vary),
            vein: super::v102::vein_from(p.vein),
            name: p.name,
            path: Path::new(knots, p.closed),
            kind,
            mirrored: p.mirrored,
            place: super::v101::place_from(p.place),
            colour: p.colour,
            height: p.height,
            link: super::v101::link_from(PieceId(p.id), p.link),
            group: p.group.map(shapes::GroupId),
        });
    }
    let reference = match stored.reference {
        Some(r) => {
            let mut image =
                Reference::new(r.width, r.height, r.rgba.0, &r.name).map_err(damaged)?;
            image = image.with_opacity(r.opacity);
            Some(Arc::new(image))
        }
        None => None,
    };
    Drawing::from_parts(DrawingParts {
        pieces,
        groups: super::v101::groups_from(stored.groups.0),
        next_id: stored.next_id,
        mirror: Mirror {
            on: stored.mirror_on,
            x: stored.mirror_x,
        },
        reference,
    })
    .map_err(|why| damaged(why.describe()))
}
