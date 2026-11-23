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
//! - [`alpha`]: greyscale pictures a brush stamps or drags, and a set
//!   made in code ([decision 0308](../../../docs/decisions/0308-sculpt-pens-strokes-alphas.md)).
//! - [`mask`]: how much each point is held still, and what changes a
//!   whole mask at once; [`symmetry`]: the copies of a dab
//!   ([decision 0310](../../../docs/decisions/0310-sculpt-symmetry-and-masks.md)).
//! - [`levels`]: detail levels, the shape subdivided up to six times,
//!   each level sculptable ([decision 0315](../../../docs/decisions/0315-detail-levels.md)).
//! - [`LiveSculpt`]: the shape being sculpted, dab by dab.

pub mod alpha;
pub mod brush;
pub mod forms;
pub mod frames;
mod layer;
pub mod levels;
pub mod rebuild;
mod live;
pub mod mask;
pub mod moves;
mod nearest;
mod parts;
pub mod symmetry;
pub mod tree;

pub use alpha::{Alpha, DabAlpha, Made};
pub use brush::{BrushKind, BrushSettings, Dab, Falloff};
pub use layer::{Fit, MAX_POINTS, Sculpt, sculpted_places};
pub use levels::{ExportLevel, LevelProblem};
pub use live::LiveSculpt;
pub use mask::Mask;
pub use moves::{CHUNK, Moves};
pub use symmetry::Symmetry;

#[cfg(test)]
mod alpha_tests;
#[cfg(test)]
mod brush_tests;
#[cfg(test)]
mod level_tests;
#[cfg(test)]
mod mask_tests;
#[cfg(test)]
mod speed_tests;
#[cfg(test)]
mod tests;
