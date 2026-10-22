//! Outlines pushed out into solids (decision 0129): pie sector, gear,
//! stairs and arch. Placeholder until built.

use super::{ArchParams, GearParams, SectorParams, StairsParams};
use crate::Mesh;

pub(super) fn sector(_: &SectorParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn gear(_: &GearParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn stairs(_: &StairsParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn arch(_: &ArchParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}
