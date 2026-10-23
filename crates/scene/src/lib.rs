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

pub mod boolean;
pub mod command;
pub mod document;
mod history;
mod memory;
pub mod mirroring;
pub mod object;
pub mod snapshot;
pub mod stack;
pub mod tree;
pub mod trim;
pub mod units;

pub use boolean::{
    Boolean, BooleanKind, BooleanProblem, BooleanRefusal, CUT_IN_BACKGROUND_FROM, CutDone, CutJob,
    CutState, MAX_CUTTERS,
};
pub use command::Command;
pub use document::Document;
pub use history::HistoryLimit;
pub use mesh::edit::{
    Bend, BevelOptions, BevelShape, ExtrudeDirection, ExtrudeShape, Eye, GrooveShape, GrooveSides,
    Grouping, Slide, SlitLine,
};
pub use mesh::uvs::{BUDGET_RANGE, SeamPreference, UvLayout};
pub use mesh::{
    Bounds, Combine, EdgeKey, Element, ElementKind, ElementSelection, FaceId, Hardness, Mesh,
    Primitive, PrimitiveKind, VertexId,
};
pub use object::{
    ApplyParts, Collision, LiveMirror, Material, Object, ObjectId, ObjectKind,
    STANDARD_SMOOTHING_ANGLE, Transform, hardness_is_valid,
};
pub use snapshot::{MAX_SNAPSHOT_NAME, MAX_SNAPSHOTS, UvSnapshot};
pub use stack::{
    EDITS_LAYER, EditId, EditSettings, Entry, EvalContext, Evaluated, Layer, LayerChange,
    MAX_SHAPE_POLYGONS, Operation, Stack, StackWarning, frozen_count,
};
pub use tree::{Place, Scene, SceneError, stretch_of};
pub use trim::{MAX_ROW_NAME, MAX_TRIM_ROWS, TrimRow, TrimSheet};
pub use units::{ExportTarget, MAX_DENSITY, TEXTURE_SIZES, TextureSettings, Units};
pub use uv::mend::MendOptions;
pub use uv::unwrap::RELAX_STEPS;
pub use uv::{HideSeams, PieceSettings, UnfoldAxis, UnwrapMethod, UnwrapOptions};

#[cfg(test)]
mod apply_tests;
#[cfg(test)]
mod boolean_tests;
#[cfg(test)]
mod edit_list_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod live_mirror_tests;
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
#[cfg(test)]
mod uv_pin_tests;
#[cfg(test)]
mod uv_survive_tests;
