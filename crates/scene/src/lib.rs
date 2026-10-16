//! The scene document: objects, their operation stacks, commands and
//! undo history.
//!
//! Layer 1 (core). Changed only through commands. See this crate's README
//! for its purpose and limits.
//!
//! The scene is a tree of [`Object`]s. Every change is a [`Command`],
//! carried out by [`Document::execute`]; it is recorded as small reversible
//! edits, so [`Document::undo`] and [`Document::redo`] can take it back
//! and do it again exactly, with the same object ids.

pub mod command;
pub mod document;
mod history;
mod memory;
pub mod mirroring;
pub mod object;
pub mod stack;
pub mod tree;
pub mod units;

pub use command::Command;
pub use document::Document;
pub use history::HistoryLimit;
pub use mesh::edit::{
    Bend, BevelOptions, BevelShape, ExtrudeDirection, ExtrudeShape, Eye, Grouping, Slide, SlitLine,
};
pub use mesh::{
    Bounds, Combine, EdgeKey, Element, ElementKind, ElementSelection, FaceId, Mesh, Primitive,
    PrimitiveKind, VertexId,
};
pub use object::{Collision, Material, Object, ObjectId, ObjectKind, Transform};
pub use stack::{
    EDITS_LAYER, EditId, EditSettings, Entry, EvalContext, Evaluated, Layer, MAX_SHAPE_POLYGONS,
    Operation, Stack, StackWarning,
};
pub use tree::{Place, Scene, SceneError, stretch_of};
pub use units::{ExportTarget, Units};

#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod mirroring_tests;
#[cfg(test)]
mod pivot_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod stack_tests;
#[cfg(test)]
mod tests;
