//! Interactive tools and the camera: navigation, selection, transform
//! gizmos, and the modelling and UV tools.
//!
//! Layer 3 (front end). Produces operations and commands; never changes
//! the scene directly. See this crate's README for its purpose and limits.

pub mod camera;

pub use camera::{Camera, Ray};
