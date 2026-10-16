//! UV mapping: seams, unwrapping, relaxing, straightening and packing.
//!
//! Layer 1 (core). Writes UV coordinates and seam flags; never changes a
//! mesh's shape. See this crate's README for its purpose and limits.
//!
//! - [`seams`]: where to cut a surface open so it lies flat: suggested
//!   seams, and telling whether a piece lies flat in one go (a disk).
//! - [`mod@unwrap`]: laying it flat: unwrapping (LSCM, and straight grids for
//!   strips), straightening and relaxing.

mod chart;
pub mod seams;
mod solve;
pub mod unwrap;

pub use unwrap::{relax, straighten, unwrap};
