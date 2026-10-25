//! Version 60 of the project file ([decision 0159](../../../../docs/decisions/0159-struts.md)):
//! version 59's, with the Struts modifier (`v43::KindV43::Struts`, its
//! settings stored as [`StrutsV60`]). Files before version 60 never hold
//! it.

use scene::StrutsOptions;
use serde::{Deserialize, Serialize};

/// The Struts modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StrutsV60 {
    /// How wide each strut is, in centimetres.
    pub(crate) thickness: f32,
    /// How many sides a strut's profile has.
    pub(crate) sides: u32,
    /// Even thickness: measured across the flats.
    pub(crate) even: bool,
    /// Keep the faces.
    pub(crate) keep_faces: bool,
}

impl StrutsV60 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &StrutsOptions) -> Self {
        Self {
            thickness: options.thickness,
            sides: options.sides,
            even: options.even,
            keep_faces: options.keep_faces,
        }
    }

    /// The settings read back (brought within their limits by the scene).
    pub(crate) fn options(&self) -> StrutsOptions {
        StrutsOptions {
            thickness: self.thickness,
            sides: self.sides,
            even: self.even,
            keep_faces: self.keep_faces,
        }
    }
}
