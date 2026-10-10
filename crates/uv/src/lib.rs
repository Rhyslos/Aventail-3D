//! UV mapping: seams, unwrapping, relaxing, straightening and packing.
//!
//! Layer 1 (core). Writes UV coordinates and seam flags; never changes a
//! mesh's shape. See this crate's README for its purpose and limits.
