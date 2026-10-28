//! Version 76 of the project file ([decision 0201](../../../../docs/decisions/0201-outlines.md)):
//! version 75's, with each object's outline, stored with its development
//! settings (`v39::ObjectV39::outline`, as [`OutlineV76`]). Files before
//! version 76 hold none.

use serde::{Deserialize, Serialize};

/// An object's outline as stored.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct OutlineV76 {
    /// How thick, in centimetres.
    pub(crate) width: f32,
    /// Its colour (sRGB).
    pub(crate) colour: [u8; 3],
}

impl OutlineV76 {
    /// The stored form of an object's outline, if it has one.
    pub(crate) fn of(object: &scene::Object) -> Option<Self> {
        object.outline.map(|o| Self {
            width: o.width,
            colour: o.colour,
        })
    }

    /// The outline it describes (checked when it is set on the object).
    pub(crate) fn outline(self) -> scene::Outline {
        scene::Outline {
            width: self.width,
            colour: self.colour,
        }
    }
}
