//! Version 62 of the project file ([decision 0161](../../../../docs/decisions/0161-triangulate.md)):
//! version 61's, with the Triangulate modifier (`v43::KindV43::Triangulate`,
//! its settings stored as [`TriangulateV62`]). Files before version 62
//! never hold it.

use scene::{TriangulateMethod, TriangulateOptions};
use serde::{Deserialize, Serialize};

/// The Triangulate modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TriangulateV62 {
    /// How the triangles are chosen: 0 Beauty, 1 Fixed (anything else is
    /// read as Beauty, the standard).
    pub(crate) method: u8,
    /// Whether only four-sided polygons are cut.
    pub(crate) quads_only: bool,
}

impl TriangulateV62 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &TriangulateOptions) -> Self {
        Self {
            method: match options.method {
                TriangulateMethod::Beauty => 0,
                TriangulateMethod::Fixed => 1,
            },
            quads_only: options.quads_only,
        }
    }

    /// The settings read back.
    pub(crate) fn options(&self) -> TriangulateOptions {
        TriangulateOptions {
            method: match self.method {
                1 => TriangulateMethod::Fixed,
                _ => TriangulateMethod::Beauty,
            },
            quads_only: self.quads_only,
        }
    }
}
