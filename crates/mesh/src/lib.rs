//! Polygon meshes: the data structure, topology queries and modelling
//! operations.
//!
//! Layer 0 (geometry). Knows nothing about scenes, files, the GPU or UI.
//! See this crate's README for its purpose and limits.

pub mod bounds;

pub use bounds::Bounds;
