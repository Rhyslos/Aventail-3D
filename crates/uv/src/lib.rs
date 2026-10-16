//! UV mapping: seams, unwrapping, relaxing, straightening and packing.
//!
//! Layer 1 (core). Writes UV coordinates and seam flags; never changes a
//! mesh's shape. See this crate's README for its purpose and limits.
//!
//! - [`seams`]: where to cut a surface open so it lies flat: suggested
//!   seams, and telling whether a piece lies flat in one go (a disk).
//! - [`check`]: the UV part of the bake check: what in the UVs would
//!   spoil a bake.
//! - [`grid`]: the numbered checker image laid on shapes by their UVs.
//! - [`density`]: texel density: measuring it and setting it.
//! - [`mod@mend`]: keeping UVs whole through modelling: laying out again
//!   what an edit left without room, around what it kept.
//! - [`methods`]: the ways a piece is laid flat (keep shapes, angle-based,
//!   keep sizes, automatic), with pinned points held.
//! - [`pack`]: fitting pieces into the texture square.
//! - [`trim`]: laying strips straight onto a trim sheet's rows.
//! - [`stretch`]: how evenly UVs lie, and what to do about it.
//! - [`mod@unwrap`]: laying it flat: unwrapping (LSCM, and straight grids for
//!   strips), straightening and relaxing.

mod chart;
pub mod check;
pub mod density;
pub mod grid;
pub mod measure;
pub mod mend;
pub mod methods;
pub mod pack;
pub mod seams;
mod solve;
pub mod stretch;
pub mod trim;
pub mod unwrap;
pub mod warnings;

#[cfg(test)]
mod automation_tests;
#[cfg(test)]
mod lock_tests;
#[cfg(test)]
mod preference_tests;
#[cfg(test)]
mod reason_tests;
pub use mend::mend;
pub use methods::UnwrapMethod;
pub use seams::{HideSeams, SeamReason, SuggestOptions};
pub use unwrap::{
    PieceSettings, UnfoldAxis, UnwrapOptions, relax, straighten, unwrap, unwrap_with,
};
