//! Version 108 of the project file ([decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! version 107's, where an Image box (`v95::BoxV95::photo`, stored only
//! when a picture is brought in) keeps the photo or painting it was given.
//!
//! A photo is stored as its file's name and the JPEG it was packed into
//! when brought in (`matgraph::Photo::packed`), so saving never packs it
//! again and its pixels are exactly the same after opening. A photo packs
//! to a few hundredths of its pixels (a 2048-pixel photo to one or two
//! megabytes), which keeps projects and graph files (JSON) small. Reading
//! unpacks it through the image reader, which limits its size before
//! reading any pixels, and refuses one bigger than an Image box keeps.
//!
//! An older app would drop the picture without a word, so the version
//! goes up; files before version 108 hold no photos.

use super::v1::Problem;
use super::v107::PackedBytes;
use matgraph::Photo;
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// A photo as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PhotoV108 {
    /// Its file's name.
    pub(crate) name: String,
    /// The JPEG it was packed into.
    pub(crate) packed: PackedBytes,
}

/// `photo` as stored.
pub(crate) fn photo_of(photo: &Photo) -> PhotoV108 {
    PhotoV108 {
        name: photo.name().to_string(),
        packed: PackedBytes(photo.packed().to_vec()),
    }
}

/// The photo stored as `stored`, checked: a picture that can't be
/// unpacked, or is bigger than an Image box keeps, is damage.
pub(crate) fn photo_from(stored: PhotoV108) -> Result<Photo, Problem> {
    let most = matgraph::MAX_PHOTO_SIDE as usize;
    if !crate::limits::spend(most * most * 4) {
        return Err(tr(crate::limits::TOO_MUCH).into());
    }
    crate::image_file::photo_from_packed(stored.packed.0, &stored.name)
        .map_err(|why| trf!("an Image box's picture is damaged: {}", why).into())
}

/// Whether a graph holds a photo (graph and box files then need
/// format 4).
pub(crate) fn holds_photo(graph: &matgraph::Graph) -> bool {
    graph.boxes().any(|(_, b)| b.photo.is_some())
}
