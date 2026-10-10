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
pub mod object;
pub mod tree;

pub use command::Command;
pub use document::Document;
pub use mesh::{Bounds, Mesh, Primitive, PrimitiveKind};
pub use object::{Object, ObjectId, ObjectKind, Transform};
pub use tree::{Place, Scene, SceneError};

#[cfg(test)]
mod tests;
