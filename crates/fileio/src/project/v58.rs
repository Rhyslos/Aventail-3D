//! Version 58 of the project file ([decision 0157](../../../../docs/decisions/0157-displace.md)):
//! version 57's, with the Displace modifier (`v43::KindV43::Displace`, its
//! settings stored as [`DisplaceV58`]). Files before version 58 never
//! hold it.

use crate::limits::Bounded;
use scene::{DisplaceDirection, DisplaceOptions, DisplaceSettings, NoiseKind, VertexId};
use serde::{Deserialize, Serialize};

/// The Displace modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DisplaceV58 {
    /// How far the noise pushes, in centimetres.
    pub(crate) strength: f32,
    /// The size of its bumps, in centimetres.
    pub(crate) scale: f32,
    /// The seed.
    pub(crate) seed: u32,
    /// How many layers of detail.
    pub(crate) detail: u32,
    /// The noise: 0 smooth, 1 ridged, 2 cells (anything else is smooth).
    pub(crate) noise: u8,
    /// Which way: 0 along the normals, 1 to 3 along X, Y or Z (anything
    /// else is along the normals).
    pub(crate) direction: u8,
    /// The falloff round chosen points, in centimetres.
    pub(crate) falloff: f32,
    /// The points it pushes (none for all).
    #[serde(default = "Bounded::new")]
    pub(crate) points: Bounded<u32>,
}

/// A noise as stored.
fn noise_number(noise: NoiseKind) -> u8 {
    match noise {
        NoiseKind::Smooth => 0,
        NoiseKind::Ridged => 1,
        NoiseKind::Cells => 2,
    }
}

/// A direction as stored.
fn direction_number(direction: DisplaceDirection) -> u8 {
    match direction {
        DisplaceDirection::Normals => 0,
        DisplaceDirection::AxisX => 1,
        DisplaceDirection::AxisY => 2,
        DisplaceDirection::AxisZ => 3,
    }
}

impl DisplaceV58 {
    /// The stored form of `settings`.
    pub(crate) fn of(settings: &DisplaceSettings) -> Self {
        let o = &settings.options;
        Self {
            strength: o.strength,
            scale: o.scale,
            seed: o.seed,
            detail: o.detail,
            noise: noise_number(o.noise),
            direction: direction_number(o.direction),
            falloff: o.falloff,
            points: settings.points.iter().map(|v| v.get()).collect(),
        }
    }

    /// The settings read back (brought within their limits by the scene).
    pub(crate) fn settings(&self) -> DisplaceSettings {
        DisplaceSettings {
            options: DisplaceOptions {
                strength: self.strength,
                scale: self.scale,
                seed: self.seed,
                detail: self.detail,
                noise: NoiseKind::ALL
                    .into_iter()
                    .find(|n| noise_number(*n) == self.noise)
                    .unwrap_or_default(),
                direction: DisplaceDirection::ALL
                    .into_iter()
                    .find(|d| direction_number(*d) == self.direction)
                    .unwrap_or_default(),
                falloff: self.falloff,
            },
            points: self.points.iter().map(|v| VertexId::from_raw(*v)).collect(),
        }
    }
}
