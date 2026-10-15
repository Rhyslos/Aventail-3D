//! Everything that touches files: `.avtl` projects, OBJ and glTF import and
//! export, and the settings file.
//!
//! Layer 2 (services). Every file is treated as hostile input. See this
//! crate's README for its purpose and limits.

pub mod autosave;
pub mod export;
pub mod limits;
pub mod obj;
pub mod project;

pub use project::{FORMAT_VERSION, ProjectError, SIGNATURE};
