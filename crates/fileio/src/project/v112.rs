//! Version 112 of the project file ([decision 0304](../../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! version 111's, with Sculpt layers, one more kind of recorded edit
//! (`v37::OperationV37::Sculpt`, stored as [`SculptEditV112`]).
//!
//! A sculpt is stored as the structure of the shape it was sculpted on,
//! where that shape's points were, and each chunk of points that moved:
//! its place in the list and its moves. The places and each chunk are
//! little-endian `f32`s packed with zstd. Chunks where nothing moved are
//! left out, so a fresh Sculpt layer stores almost nothing beyond the
//! places. Reading checks every count before unpacking, never unpacks more
//! than the count allows, pays for what it unpacks from the read's memory
//! budget, and refuses moves that aren't real numbers.
//!
//! An older app would refuse the unknown edit, so the version goes up;
//! files before version 112 hold no Sculpt layers.

use super::v1::Problem;
use super::v107::PackedBytes;
use crate::limits::Bounded;
use glam::Vec3;
use scene::Operation;
use sculpt::{CHUNK, MAX_POINTS, Moves, Sculpt};
use serde::{Deserialize, Serialize};
use words::tr;

/// How hard zstd works on a sculpt: quick (saving a heavy sculpt must
/// stay well under a few seconds).
const PACK_LEVEL: i32 = 1;

/// A stored sculpt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SculptEditV112 {
    /// The structure of the shape it was sculpted on.
    pub(crate) structure: u64,
    /// How many point slots.
    pub(crate) points: u64,
    /// Where the points were, packed.
    pub(crate) places: PackedBytes,
    /// The chunks that moved.
    pub(crate) chunks: Bounded<ChunkV112>,
}

/// One chunk of moves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ChunkV112 {
    /// Which chunk, from the first (0).
    pub(crate) index: u32,
    /// Its moves, packed.
    pub(crate) moves: PackedBytes,
}

/// `values` as little-endian numbers, packed.
fn pack(values: &[Vec3]) -> PackedBytes {
    let mut bytes = Vec::with_capacity(values.len() * 12);
    for v in values {
        for x in v.to_array() {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
    }
    PackedBytes(zstd::bulk::compress(&bytes, PACK_LEVEL).unwrap_or_default())
}

/// `count` points unpacked from `packed`; `None` when they don't unpack
/// to exactly that many, or the budget runs out.
fn unpack(packed: &PackedBytes, count: usize) -> Result<Vec<Vec3>, Problem> {
    let expected = count * 12;
    if !crate::limits::spend(expected) {
        return Err(tr(crate::limits::TOO_MUCH).into());
    }
    let damaged = || -> Problem { tr("a Sculpt layer is damaged").into() };
    let bytes = if expected == 0 {
        Vec::new()
    } else {
        zstd::bulk::decompress(&packed.0, expected).map_err(|_| damaged())?
    };
    if bytes.len() != expected {
        return Err(damaged());
    }
    Ok(bytes
        .chunks_exact(12)
        .map(|b| {
            let f = |k: usize| f32::from_le_bytes([b[k], b[k + 1], b[k + 2], b[k + 3]]);
            Vec3::new(f(0), f(4), f(8))
        })
        .collect())
}

impl SculptEditV112 {
    /// The stored form of `op`, if it is a sculpt.
    pub(crate) fn of(op: &Operation) -> Option<Self> {
        let Operation::Sculpt { sculpt } = op else {
            return None;
        };
        let moves = sculpt.moves();
        let len = moves.len();
        let chunks = moves
            .chunks()
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let c = c.as_ref()?;
                Some(ChunkV112 {
                    index: i as u32,
                    moves: pack(c),
                })
            })
            .collect();
        Some(Self {
            structure: sculpt.structure(),
            points: len as u64,
            places: pack(sculpt.places()),
            chunks,
        })
    }

    /// The sculpt read back, checked: too many points, a chunk out of
    /// range or twice, packed numbers that don't unpack to their count,
    /// or moves that aren't real numbers are damage.
    pub(crate) fn into_operation(self) -> Result<Operation, Problem> {
        let damaged = || -> Problem { tr("a Sculpt layer is damaged").into() };
        let len = usize::try_from(self.points).map_err(|_| damaged())?;
        if len > MAX_POINTS {
            return Err(damaged());
        }
        let places = unpack(&self.places, len)?;
        let count = len.div_ceil(CHUNK);
        let mut chunks: Vec<sculpt::moves::Chunk> = vec![None; count];
        for chunk in self.chunks.0 {
            let index = chunk.index as usize;
            if index >= count || chunks[index].is_some() {
                return Err(damaged());
            }
            let moves = unpack(&chunk.moves, sculpt::moves::chunk_len(len, index))?;
            if !moves.iter().all(|m| m.is_finite()) {
                return Err(damaged());
            }
            chunks[index] = Some(moves.into());
        }
        let moves = Moves::from_chunks(len, chunks).ok_or_else(damaged)?;
        let sculpt = Sculpt::from_parts(self.structure, places, moves).ok_or_else(damaged)?;
        Ok(Operation::Sculpt { sculpt })
    }
}
