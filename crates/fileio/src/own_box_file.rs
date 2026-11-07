//! Box files ([decision 0263](../../../docs/decisions/0263-your-own-boxes.md)):
//! one of your own boxes kept in a small file to share or use in another
//! project, and the same text keeps your library of own boxes with the
//! app's settings.
//!
//! A box file is readable JSON: a format number, the id of the box it is
//! for, and that box with every own box inside it (stored as the project
//! file stores them, `project::v98`). Everything read is checked, as a
//! project's own boxes are.

use crate::limits::Bounded;
use crate::project::v98::{OwnBoxV98, own_box_of, own_boxes_from};
use matgraph::OwnBox;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use words::tr;

/// The extension of box files, without the dot. Generic on purpose: it
/// doesn't name the app, so it survives a rename (RENAMING.md).
pub const BOX_EXTENSION: &str = "matbox";

/// The largest box file read, in bytes.
pub const MAX_BOX_FILE: u64 = 8 << 20;

/// The version of the box file format written. Newer files are refused
/// with a message rather than read wrongly.
pub const BOX_FORMAT: u32 = 1;

/// One of your own boxes with every own box inside it: what a box file
/// holds.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxFile {
    /// The box it is for.
    pub main: Arc<OwnBox>,
    /// It and the own boxes inside it, each after those inside it.
    pub boxes: Vec<Arc<OwnBox>>,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: u32,
    /// The id of the box it is for.
    main: String,
    boxes: Bounded<OwnBoxV98>,
}

impl BoxFile {
    /// Own box `main` with the own boxes inside it, found by `find` (the
    /// project's); `None` if one inside can't be found.
    pub fn of(main: &Arc<OwnBox>, find: impl Fn(&str) -> Option<Arc<OwnBox>>) -> Option<Self> {
        let mut found: BTreeMap<String, Arc<OwnBox>> = BTreeMap::new();
        let mut todo: Vec<String> = main.uses().into_iter().collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = todo.pop() {
            if id == main.id || !seen.insert(id.clone()) {
                continue;
            }
            let own = find(&id)?;
            todo.extend(own.uses());
            found.insert(id, own);
        }
        // Each after those inside it.
        let mut boxes: Vec<Arc<OwnBox>> = Vec::new();
        let mut left: Vec<Arc<OwnBox>> = found.into_values().collect();
        while !left.is_empty() {
            let (ready, later): (Vec<_>, Vec<_>) = left
                .into_iter()
                .partition(|o| o.uses().iter().all(|u| boxes.iter().any(|b| &b.id == u)));
            if ready.is_empty() {
                return None;
            }
            boxes.extend(ready);
            left = later;
        }
        boxes.push(main.clone());
        Some(Self {
            main: main.clone(),
            boxes,
        })
    }
}

/// The text of a box file holding `file`.
pub fn write_box_file(file: &BoxFile) -> String {
    let stored = Stored {
        format: BOX_FORMAT,
        main: file.main.id.clone(),
        boxes: file.boxes.iter().map(|o| own_box_of(o)).collect(),
    };
    serde_json::to_string_pretty(&stored).unwrap_or_default()
}

/// The box file in `bytes`, checked; or why it can't be read, in plain
/// words.
pub fn read_box_file(bytes: &[u8]) -> Result<BoxFile, String> {
    let unreadable = || tr("it isn't a box file, or it is damaged").to_string();
    if bytes.len() as u64 > MAX_BOX_FILE {
        return Err(tr("it is too large to be a box file").to_string());
    }
    // The format number first, so a newer file says so.
    #[derive(Deserialize)]
    struct Version {
        format: u32,
    }
    let version: Version = serde_json::from_slice(bytes).map_err(|_| unreadable())?;
    if version.format > BOX_FORMAT {
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
    let boxes: Vec<Arc<OwnBox>> = own_boxes_from(stored.boxes)?
        .into_iter()
        .map(Arc::new)
        .collect();
    let main = boxes
        .iter()
        .find(|o| o.id == stored.main)
        .cloned()
        .ok_or_else(unreadable)?;
    Ok(BoxFile { main, boxes })
}

#[cfg(test)]
mod tests;
