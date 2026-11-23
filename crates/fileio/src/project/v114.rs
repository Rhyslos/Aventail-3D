//! Version 114 of the project file ([decision 0310](../../../../docs/decisions/0310-sculpt-symmetry-and-masks.md)):
//! version 113's, with each Sculpt layer's mask (how much each point is
//! held still), stored with its sculpt ([`super::v112::SculptEditV112::mask`]):
//! each chunk of points with any mask, its place in the list and one byte
//! a point, packed with zstd. Chunks with no mask are left out. Files
//! before version 114 hold no masks; an older app would open a project
//! without them and lose them on saving, so the version goes up.

use super::v1::Problem;
use super::v107::PackedBytes;
use crate::limits::Bounded;
use sculpt::mask::{Mask, MaskChunk};
use sculpt::{CHUNK, Sculpt};
use serde::{Deserialize, Serialize};
use words::tr;

/// How hard zstd works on a mask: quick, as for the moves.
const PACK_LEVEL: i32 = 1;

/// One chunk of a mask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MaskChunkV114 {
    /// Which chunk, from the first (0).
    pub(crate) index: u32,
    /// One byte a point, packed.
    pub(crate) values: PackedBytes,
}

/// The chunks of `sculpt`'s mask to store: those with any point masked.
pub(crate) fn mask_of(sculpt: &Sculpt) -> Bounded<MaskChunkV114> {
    sculpt
        .mask()
        .chunks()
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let c = c.as_ref()?;
            Some(MaskChunkV114 {
                index: i as u32,
                values: PackedBytes(zstd::bulk::compress(c, PACK_LEVEL).unwrap_or_default()),
            })
        })
        .collect()
}

/// `sculpt` with the stored mask, a value for each point of detail level
/// `level` (0 before version 115), checked: a level it hasn't, a chunk
/// out of range or twice, or values that don't unpack to the chunk's
/// count, is damage.
pub(crate) fn with_mask(
    sculpt: Sculpt,
    stored: Bounded<MaskChunkV114>,
    level: usize,
) -> Result<Sculpt, Problem> {
    let damaged = || -> Problem { tr("a Sculpt layer's mask is damaged").into() };
    let len = sculpt.level_places(level).ok_or_else(damaged)?.len();
    if stored.is_empty() {
        return Ok(sculpt);
    }
    let count = len.div_ceil(CHUNK);
    let mut chunks: Vec<MaskChunk> = vec![None; count];
    for chunk in stored.0 {
        let index = chunk.index as usize;
        if index >= count || chunks[index].is_some() {
            return Err(damaged());
        }
        let expected = sculpt::moves::chunk_len(len, index);
        if !crate::limits::spend(expected) {
            return Err(tr(crate::limits::TOO_MUCH).into());
        }
        let values = zstd::bulk::decompress(&chunk.values.0, expected).map_err(|_| damaged())?;
        if values.len() != expected {
            return Err(damaged());
        }
        chunks[index] = Some(values.into());
    }
    let mask = Mask::from_chunks(len, chunks).ok_or_else(damaged)?;
    sculpt.with_mask_at(level, mask).ok_or_else(damaged)
}
