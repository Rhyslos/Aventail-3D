//! Everything that touches files: `.avtl` projects, OBJ and glTF import and
//! export, and the settings file.
//!
//! Layer 2 (services). Every file is treated as hostile input. See this
//! crate's README for its purpose and limits.

pub mod autosave;
pub mod engine;
pub mod export;
pub mod gltf;
pub mod image_file;
pub mod limits;
pub mod obj;
pub mod project;

/// The name written into exported files (an OBJ's object, a game
/// project's file) for an object with none: the same in every language,
/// so a game project's files keep their names whatever language the app
/// is in.
pub(crate) const UNNAMED_IN_FILES: &str = "Object";

pub use project::{FORMAT_VERSION, ProjectError, SIGNATURE};
