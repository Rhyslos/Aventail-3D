//! Sculpting ([decision 0304](../../../docs/decisions/0304-sculpt-mode-and-layers.md)):
//! a Sculpt layer's data, the brushes, and the live sculpt a stroke works
//! on.
//!
//! Layer 1 (core). Knows nothing of objects, the scene, files, the GPU or
//! the interface: the scene keeps a [`Sculpt`] in an object's stack and
//! evaluates it with [`Sculpt::apply`]; the interface runs strokes on a
//! [`LiveSculpt`] and hands the finished stroke's [`Sculpt`] to the scene
//! as a command. See this crate's README for its purpose and limits.
//!
//! - [`Sculpt`]: how far each point moved, measured along the surface
//!   (its [`frames::Frame`]), in chunks of [`moves::CHUNK`] points shared
//!   between copies, and where the points were when sculpted, to fit the
//!   sculpt again when the shape under it changes.
//! - [`brush`]: the brushes, their settings and what a dab of Draw or
//!   Smooth does; [`forms`]: what a dab of each other brush does
//!   ([decision 0306](../../../docs/decisions/0306-sculpt-brushes-for-forms.md)).
//! - [`tree::FaceTree`]: boxes round the polygons, refitted as points
//!   move, for finding the surface and the points in reach.
//! - [`LiveSculpt`]: the shape being sculpted, dab by dab.

pub mod brush;
pub mod forms;
pub mod frames;
mod layer;
mod live;
pub mod moves;
mod nearest;
pub mod tree;

pub use brush::{BrushKind, BrushSettings, Dab, Falloff};
pub use layer::{Fit, MAX_POINTS, Sculpt, sculpted_places};
pub use live::LiveSculpt;
pub use moves::{CHUNK, Moves};

#[cfg(test)]
mod brush_tests;
#[cfg(test)]
mod tests;
