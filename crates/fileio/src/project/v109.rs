//! Version 109 of the project file ([decision 0285](../../../../docs/decisions/0285-stamp-images.md)):
//! version 108's, where a Stamps box (`v95::BoxV95::stamps`, stored only
//! when it holds some) keeps its small pictures, each with its weight.
//!
//! A stamp is stored as its file's name, its weight and the PNG it was
//! packed into when brought in (`matgraph::Stamp::packed`): a PNG keeps
//! the see-through parts exactly, so saving never packs a stamp again and
//! its pixels are the same after opening. Reading unpacks each through
//! the image reader, which limits its size before reading any pixels,
//! and refuses one bigger than a Stamps box keeps, or a list longer or
//! bigger than one box holds.
//!
//! An older app would drop the stamps without a word, so the version goes
//! up; files before version 109 hold no stamps.

use super::v1::Problem;
use super::v107::PackedBytes;
use matgraph::{StampEntry, Stamps};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use words::{tr, trf};

/// One stamp as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StampV109 {
    /// Its file's name.
    pub(crate) name: String,
    /// How often it is picked (1 to `matgraph::stamp::MAX_WEIGHT`).
    pub(crate) weight: u32,
    /// The PNG it was packed into.
    pub(crate) packed: PackedBytes,
}

/// `stamps` as stored.
pub(crate) fn stamps_of(stamps: &Stamps) -> Vec<StampV109> {
    stamps
        .entries()
        .iter()
        .map(|e| StampV109 {
            name: e.stamp.name().to_string(),
            weight: e.weight,
            packed: PackedBytes(e.stamp.packed().to_vec()),
        })
        .collect()
}

/// The stamps stored as `stored`, checked: a picture that can't be
/// unpacked or is too big, or a list too long or too big, is damage.
pub(crate) fn stamps_from(stored: Vec<StampV109>) -> Result<Stamps, Problem> {
    if stored.len() > matgraph::MAX_STAMPS {
        return Err(tr("a Stamps box holds more stamps than the app keeps").into());
    }
    let most = matgraph::MAX_STAMP_SIDE as usize;
    let mut entries = Vec::with_capacity(stored.len());
    for s in stored {
        if !crate::limits::spend(most * most * 4) {
            return Err(tr(crate::limits::TOO_MUCH).into());
        }
        let stamp = crate::image_file::stamp_from_packed(s.packed.0, &s.name)
            .map_err(|why| trf!("a Stamps box's picture is damaged: {}", why))?;
        entries.push(StampEntry {
            stamp: Arc::new(stamp),
            weight: s.weight,
        });
    }
    Stamps::new(entries).map_err(|why| trf!("a Stamps box is damaged: {}", tr(why)).into())
}

/// Whether a graph holds stamps (graph and box files then need format 5).
pub(crate) fn holds_stamps(graph: &matgraph::Graph) -> bool {
    graph.boxes().any(|(_, b)| b.stamps.is_some())
}
