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

/// Packed bytes, saved as one block: raw in the project file, and as
/// base64 text in JSON (graph and box files; decision 0283: a list of
/// numbers took ten times the room, too much for a photo). A list of
/// numbers, as graph files of format 3 hold, is still read. Paid for from
/// the read's memory budget as they are read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PackedBytes(pub(crate) Vec<u8>);

impl Serialize for PackedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&base64(&self.0))
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for PackedBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Reads packed bytes; `text` when they come as base64 text.
        struct Packed {
            text: bool,
        }

        impl<'de> Visitor<'de> for Packed {
            type Value = PackedBytes;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(tr("a painting's packed pixels"))
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<PackedBytes, E> {
                self.visit_bytes(text.as_bytes())
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<PackedBytes, E> {
                if !crate::limits::spend(bytes.len()) {
                    return Err(E::custom(tr(crate::limits::TOO_MUCH)));
                }
                if self.text {
                    return from_base64(bytes)
                        .map(PackedBytes)
                        .ok_or_else(|| E::custom(tr("packed bytes are damaged")));
                }
                Ok(PackedBytes(bytes.to_vec()))
            }

            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<PackedBytes, E> {
                if self.text {
                    return self.visit_bytes(&bytes);
                }
                if !crate::limits::spend(bytes.len()) {
                    return Err(E::custom(tr(crate::limits::TOO_MUCH)));
                }
                Ok(PackedBytes(bytes))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<PackedBytes, A::Error> {
                // JSON from format 3 keeps the bytes as a list of numbers.
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

        let text = deserializer.is_human_readable();
        deserializer.deserialize_byte_buf(Packed { text })
    }
}

/// The base64 letters (RFC 4648, with `=` padding).
const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as base64 text.
pub(crate) fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(LETTERS[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes base64 text `text` holds; `None` when it isn't base64.
pub(crate) fn from_base64(text: &[u8]) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(4) {
        return None;
    }
    let value = |c: u8| -> Option<u32> {
        Some(u32::from(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        }))
    };
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let quads = text.len() / 4;
    for (q, quad) in text.chunks_exact(4).enumerate() {
        let pad = quad.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 2 || (pad > 0 && q + 1 != quads) {
            return None;
        }
        let mut n = 0u32;
        for c in &quad[..4 - pad] {
            n = n << 6 | value(*c)?;
        }
        n <<= 6 * pad as u32;
        let got = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&got[..3 - pad]);
    }
    Some(out)
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
