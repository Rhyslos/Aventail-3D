//! Polyhedra (decision 0129): icosphere, hex sphere and hex grid.
//! Placeholder until built.

use super::{HexGridParams, HexSphereParams, IcosphereParams};
use crate::Mesh;

pub(super) fn icosphere(_: &IcosphereParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn hex_sphere(_: &HexSphereParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn hex_grid(_: &HexGridParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}
