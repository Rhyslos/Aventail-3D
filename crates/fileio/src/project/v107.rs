//! Version 107 of the project file ([decision 0282](../../../../docs/decisions/0282-painted-box.md)):
//! version 106's, where a Painted box (`v95::BoxV95::painting`, stored
//! only when something is painted) keeps what is painted on it.
//!
//! A painting is stored as its size and its pixels (RGBA, sRGB, straight
//! alpha, row by row from the top), packed with zstd: what isn't painted
//! is zeros and packs to almost nothing, so even a large painting with a
//! few strokes stays small, in the project and in graph files (which are
//! JSON). Reading checks the size is one a painting can be and unpacks no
//! more than that size's pixels, paid for from the read's memory budget.
//!
//! An older app would drop the paint without a word, so the version goes
//! up; files before version 107 hold no paintings.

use super::v1::Problem;
use matgraph::{PAINTING_SIZES, Painting};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use words::tr;

/// How hard zstd works on a painting: quick, and zeros pack well anyway.
const PACK_LEVEL: i32 = 3;

/// A painting as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PaintingV107 {
    /// Pixels across (and down).
    pub(crate) size: u32,
    /// Its pixels, packed with zstd.
    pub(crate) packed: PackedBytes,
}

/// Packed bytes, saved as one block (a list of numbers in JSON) and paid
/// for from the read's memory budget as they are read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PackedBytes(pub(crate) Vec<u8>);

impl Serialize for PackedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for PackedBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Packed;

        impl<'de> Visitor<'de> for Packed {
            type Value = PackedBytes;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(tr("a painting's packed pixels"))
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<PackedBytes, E> {
                self.visit_byte_buf(bytes.to_vec())
            }

            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<PackedBytes, E> {
                if !crate::limits::spend(bytes.len()) {
                    return Err(E::custom(tr(crate::limits::TOO_MUCH)));
                }
                Ok(PackedBytes(bytes))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<PackedBytes, A::Error> {
                // JSON keeps the bytes as a list of numbers.
                let mut bytes = Vec::new();
                while let Some(byte) = seq.next_element::<u8>()? {
                    if !crate::limits::spend(1) {
                        return Err(serde::de::Error::custom(tr(crate::limits::TOO_MUCH)));
                    }
                    bytes.push(byte);
                }
                Ok(PackedBytes(bytes))
            }
        }

        deserializer.deserialize_byte_buf(Packed)
    }
}

/// `painting` as stored.
pub(crate) fn painting_of(painting: &Painting) -> PaintingV107 {
    let rgba = painting.to_rgba();
    let packed = zstd::bulk::compress(&rgba, PACK_LEVEL).unwrap_or_default();
    PaintingV107 {
        size: painting.size(),
        packed: PackedBytes(packed),
    }
}

/// The painting stored as `stored`, checked: a size a painting can't be,
/// or pixels that don't unpack to exactly that size, are damage.
pub(crate) fn painting_from(stored: PaintingV107) -> Result<Painting, Problem> {
    let damaged = || -> Problem { tr("a Painted box's painting is damaged").into() };
    if !PAINTING_SIZES.contains(&stored.size) {
        return Err(damaged());
    }
    let n = stored.size as usize;
    let expected = n * n * 4;
    if !crate::limits::spend(expected) {
        return Err(tr(crate::limits::TOO_MUCH).into());
    }
    // Never unpacks more than the size's pixels.
    let rgba = zstd::bulk::decompress(&stored.packed.0, expected).map_err(|_| damaged())?;
    if rgba.len() != expected {
        return Err(damaged());
    }
    Painting::from_rgba(stored.size, &rgba).ok_or_else(damaged)
}

/// Whether a graph holds a painting (graph and box files then need
/// format 3).
pub(crate) fn holds_painting(graph: &matgraph::Graph) -> bool {
    graph.boxes().any(|(_, b)| b.painting.is_some())
}
