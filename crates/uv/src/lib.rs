//! UV mapping: seams, unwrapping, relaxing, straightening and packing.
//!
//! Layer 1 (core). Writes UV coordinates and seam flags; never changes a
//! mesh's shape. See this crate's README for its purpose and limits.
//!
//! - [`seams`]: where to cut a surface open so it lies flat: suggested
//!   seams, and telling whether a piece lies flat in one go (a disk).
//! - [`check`]: the UV part of the bake check: what in the UVs would
//!   spoil a bake.
//! - [`density`]: texel density: measuring it and setting it.
//! - [`mod@mend`]: keeping UVs whole through modelling: laying out again
//!   what an edit left without room, around what it kept.
//! - [`pack`]: fitting pieces into the texture square.
//! - [`stretch`]: how evenly UVs lie, and what to do about it.
//! - [`mod@unwrap`]: laying it flat: unwrapping (LSCM, and straight grids for
//!   strips), straightening and relaxing.

mod chart;
pub mod check;
pub mod density;
pub mod mend;
pub mod pack;
pub mod seams;
mod solve;
pub mod stretch;
pub mod unwrap;

pub use mend::mend;
pub use unwrap::{relax, straighten, unwrap};
