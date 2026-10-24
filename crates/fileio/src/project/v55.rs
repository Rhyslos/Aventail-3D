//! Version 55 of the project file ([decision 0154](../../../../docs/decisions/0154-shell.md)):
//! version 54's, with the Shell modifier (`v43::KindV43::Shell`, its
//! settings stored as [`ShellV55`]). Files before version 55 never hold
//! it.

use scene::{ShellDirection, ShellOptions};
use serde::{Deserialize, Serialize};

/// The Shell modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ShellV55 {
    /// How thick, in centimetres.
    pub(crate) thickness: f32,
    /// Which way it grows: 0 inward, 1 outward, 2 both sides (anything
    /// else is inward).
    pub(crate) direction: u8,
    /// Even thickness.
    pub(crate) even: bool,
    /// Fill the rims.
    pub(crate) fill_rims: bool,
}

/// A direction as stored.
fn direction_number(direction: ShellDirection) -> u8 {
    match direction {
        ShellDirection::Inward => 0,
        ShellDirection::Outward => 1,
        ShellDirection::Both => 2,
    }
}

impl ShellV55 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &ShellOptions) -> Self {
        Self {
            thickness: options.thickness,
            direction: direction_number(options.direction),
            even: options.even,
            fill_rims: options.fill_rims,
        }
    }

    /// The settings read back (brought within their limits by the scene).
    pub(crate) fn options(&self) -> ShellOptions {
        ShellOptions {
            thickness: self.thickness,
            direction: ShellDirection::ALL
                .into_iter()
                .find(|d| direction_number(*d) == self.direction)
                .unwrap_or_default(),
            even: self.even,
            fill_rims: self.fill_rims,
        }
    }
}
