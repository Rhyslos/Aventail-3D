//! Material graphs: boxes joined by wires, worked out into seamless
//! textures ([decision 0257](../../../docs/decisions/0257-material-graphs-core.md),
//! the plan in `docs/plans/phase-12.md`).
//!
//! Layer 1 (core). No files, GPU or interface: everything here can be
//! tested without a window. See this crate's README for its purpose and
//! limits.
//!
//! The pieces, from the bottom up:
//!
//! - [`ValueKind`]: what a wire carries (a shape, a height, a colour or a
//!   whole material), and which kinds fit which plugs ([`kind`]).
//! - [`Image`], [`Size`] and [`Value`]: the pictures boxes make. Every
//!   image wraps round at its edges, so everything tiles seamlessly.
//! - [`BoxType`] and [`Registry`]: what a kind of box is (its plain name,
//!   "What does this do?", plugs, settings and what usually comes next)
//!   and how it works. The first boxes are in [`boxes`].
//! - [`Graph`] and [`GraphEdit`]: the boxes, wires and comment frames of
//!   one graph, and every change to it, checked, with a plain-words
//!   [`Refusal`] (and ways to make it work) when it can't be made.
//! - [`Evaluator`]: works a graph out at a chosen size, in tiles on every
//!   core, keeping each box's result so only what changed is worked out
//!   again.
//! - [`GraphStore`]: a project's graphs by [`GraphId`].

pub mod boxes;
mod cache;
pub mod eval;
pub mod graph;
pub mod image;
pub mod kind;
mod parallel;
pub mod refusal;
pub mod registry;
pub mod seed;
pub mod settings;
pub mod store;
pub mod value;

pub use cache::Cache;
pub use eval::{BoxResult, Evaluation, Evaluator, real_size_cm};
pub use graph::{
    BoxId, Frame, FrameId, Graph, GraphBox, GraphEdit, GraphParts, MAX_BOXES, MAX_FRAMES, MAX_NAME,
    MAX_NOTE, PlugRef, Wire,
};
pub use image::{Grey, Image, Rgb, Size};
pub use kind::{Fit, ValueKind};
pub use refusal::{Fix, Refusal};
pub use registry::{
    BoxInfo, BoxType, Group, Job, Label, MIN_REAL_SIZE_CM, PlugSpec, Registry,
    STANDARD_REAL_SIZE_CM,
};
pub use settings::{
    ColourGradient, GradientStop, MAX_STOPS, SettingKind, SettingSpec, SettingValue, Settings,
};
pub use store::{GraphId, GraphStore, MAX_GRAPHS};
pub use value::{MaterialMaps, Value};
