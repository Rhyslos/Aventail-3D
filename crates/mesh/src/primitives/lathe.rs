//! Shapes turned round the up axis (decision 0129): torus, capsule, dome
//! and pipe elbow. Placeholder until built.

use super::{CapsuleParams, DomeParams, ElbowParams, TorusParams};
use crate::Mesh;

pub(super) fn torus(_: &TorusParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn capsule(_: &CapsuleParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn dome(_: &DomeParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}

pub(super) fn elbow(_: &ElbowParams) -> Mesh {
    super::boxy::cube(&super::CubeParams::default())
}
