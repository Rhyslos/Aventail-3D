//! Version 50 of the project file ([decision 0149](../../../../docs/decisions/0149-live-bevel.md)):
//! version 49's, with the Live bevel modifier (`v43::KindV43::LiveBevel`,
//! its settings stored as [`LiveBevelV50`]) and bevel weights, one more
//! kind of recorded edit (`v37::OperationV37::SetBevelWeights`). Files
//! before version 50 hold neither.
//!
//! Version 57 ([decision 0156](../../../../docs/decisions/0156-game-bevel.md))
//! adds *Soft bevels* ([`LiveBevelV50::soften`]); stored with a standard
//! value (off), so files from version 50 to 56 read it as off.

use scene::{BevelEdges, LiveBevelOptions};
use serde::{Deserialize, Serialize};

/// The Live bevel modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LiveBevelV50 {
    /// Which edges: 0 by angle, 1 hard edges, 2 marked edges (anything
    /// else is by angle).
    pub(crate) edges: u8,
    /// The angle, in degrees.
    pub(crate) angle: f32,
    /// The width, in centimetres.
    pub(crate) width: f32,
    /// The rows of each bevel.
    pub(crate) segments: u32,
    /// Round (1), flat (0) or hollow (-1).
    pub(crate) profile: f32,
    /// Stop at neighbours.
    pub(crate) clamp: bool,
    /// Soft bevels (version 57; off before).
    #[serde(default)]
    pub(crate) soften: bool,
}

impl LiveBevelV50 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &LiveBevelOptions) -> Self {
        Self {
            edges: match options.edges {
                BevelEdges::ByAngle => 0,
                BevelEdges::Hard => 1,
                BevelEdges::Marked => 2,
            },
            angle: options.angle,
            width: options.width,
            segments: options.segments,
            profile: options.profile,
            clamp: options.clamp,
            soften: options.soften,
        }
    }

    /// The settings read back (brought within their limits when the
    /// modifier is made).
    pub(crate) fn options(&self) -> LiveBevelOptions {
        LiveBevelOptions {
            edges: match self.edges {
                1 => BevelEdges::Hard,
                2 => BevelEdges::Marked,
                _ => BevelEdges::ByAngle,
            },
            angle: self.angle,
            width: self.width,
            segments: self.segments,
            profile: self.profile,
            clamp: self.clamp,
            soften: self.soften,
        }
    }
}
