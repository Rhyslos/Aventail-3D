//! Packs ([decision 0292](../../../docs/decisions/0292-library-browser.md)):
//! several material graphs and own boxes shared as one file, so a set
//! (say "Dungeon stone": four graphs and the two boxes they share) goes
//! to a friend or another computer in one go. The library browser writes
//! them and brings them into your library.
//!
//! A pack is readable JSON: a format number, then each graph as a graph
//! file stores it (with the own boxes it uses) and each own box as a box
//! file stores it (with the boxes inside it). An own box used by two
//! entries is stored with each, so every entry reads on its own, exactly
//! as its own file would. Everything read is checked, as graph and box
//! files are, and the whole pack is refused if one entry is damaged
//! (rather than bringing in half of it without a word).

use crate::graph_file::{GraphFile, graph_from_stored, graph_to_stored};
use crate::limits::Bounded;
use crate::own_box_file::{BoxFile, box_from_stored, box_to_stored};
use crate::project::v95::GraphV95;
use crate::project::v98::OwnBoxV98;
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// The extension of pack files, without the dot. Generic on purpose: it
/// doesn't name the app, so it survives a rename (RENAMING.md).
pub const PACK_EXTENSION: &str = "matpack";

/// The largest pack read, in bytes: room for a few graphs holding photos.
pub const MAX_PACK_FILE: u64 = 64 << 20;

/// The most graphs and own boxes, together, one pack holds.
pub const MAX_PACK_ITEMS: usize = 256;

/// The newest version of the pack format read. Newer packs are refused
/// with a message rather than read wrongly.
pub const PACK_FORMAT: u32 = 1;

/// Graphs and own boxes shared together: what a pack holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pack {
    /// The graphs, each with the own boxes it uses.
    pub graphs: Vec<GraphFile>,
    /// The own boxes, each with the boxes inside it.
    pub boxes: Vec<BoxFile>,
}

impl Pack {
    /// How many graphs and own boxes it holds.
    pub fn len(&self) -> usize {
        self.graphs.len() + self.boxes.len()
    }

    /// Whether it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Serialize, Deserialize)]
struct StoredGraph {
    graph: GraphV95,
    #[serde(default = "Bounded::new")]
    own: Bounded<OwnBoxV98>,
}

#[derive(Serialize, Deserialize)]
struct StoredBox {
    main: String,
    boxes: Bounded<OwnBoxV98>,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: u32,
    #[serde(default = "Bounded::new")]
    graphs: Bounded<StoredGraph>,
    #[serde(default = "Bounded::new")]
    boxes: Bounded<StoredBox>,
}

/// The text of a pack holding `pack`.
pub fn write_pack(pack: &Pack) -> String {
    let stored = Stored {
        format: PACK_FORMAT,
        graphs: pack
            .graphs
            .iter()
            .map(|file| {
                let (graph, own) = graph_to_stored(file);
                StoredGraph { graph, own }
            })
            .collect(),
        boxes: pack
            .boxes
            .iter()
            .map(|file| {
                let (main, boxes) = box_to_stored(file);
                StoredBox { main, boxes }
            })
            .collect(),
    };
    serde_json::to_string_pretty(&stored).unwrap_or_default()
}

/// The pack in `bytes`, checked; or why it can't be read, in plain words.
pub fn read_pack(bytes: &[u8]) -> Result<Pack, String> {
    let unreadable = || tr("it isn't a pack, or it is damaged").to_string();
    if bytes.len() as u64 > MAX_PACK_FILE {
        return Err(tr("it is too large to be a pack").to_string());
    }
    // The format number first, so a newer pack says so.
    #[derive(Deserialize)]
    struct Version {
        format: u32,
    }
    let version: Version = serde_json::from_slice(bytes).map_err(|_| unreadable())?;
    if version.format > PACK_FORMAT {
        return Err(tr("it was made by a newer version of the app").to_string());
    }
    let stored: Stored =
        crate::limits::within_budget(|| serde_json::from_slice(bytes)).map_err(|_| {
            if crate::limits::limit_hit() {
                tr(crate::limits::TOO_MUCH).to_string()
            } else {
                unreadable()
            }
        })?;
    if stored.graphs.len() + stored.boxes.len() > MAX_PACK_ITEMS {
        return Err(trf!(
            "it holds more than {} graphs and boxes",
            MAX_PACK_ITEMS
        ));
    }
    let mut pack = Pack::default();
    for (i, entry) in stored.graphs.0.into_iter().enumerate() {
        let file = graph_from_stored(entry.graph, entry.own)
            .map_err(|why| trf!("its graph number {} is damaged: {}", i + 1, why))?;
        pack.graphs.push(file);
    }
    for (i, entry) in stored.boxes.0.into_iter().enumerate() {
        let file = box_from_stored(&entry.main, entry.boxes)
            .map_err(|why| trf!("its own box number {} is damaged: {}", i + 1, why))?;
        pack.boxes.push(file);
    }
    Ok(pack)
}

#[cfg(test)]
mod tests;
