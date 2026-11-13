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
//! - [`SeamCheck`]: whether a texture's joins show when it repeats.
//! - [`OnModel`] and [`ModelMaps`]: working a graph out on a model in the
//!   painter, with the model's own maps (`model`).
//! - [`OwnBox`]: your own boxes, made from a group of boxes, with their
//!   own plugs and sliders (`own`).
//! - [`bake_notes`]: what an export should say about baking a graph into
//!   textures (`baking`).
//! - [`health::check`]: the material health check, what may look wrong
//!   in a game, in plain words, each with a fix (`health`).
//! - [`tags`]: the words graphs and own boxes are found by in the library
//!   browser.

pub mod baking;
pub mod boxes;
mod cache;
pub mod eval;
pub mod graph;
pub mod health;
pub mod image;
pub mod kind;
pub mod light;
pub mod model;
pub mod own;
pub mod painting;
pub mod palettes;
mod parallel;
pub mod photo;
pub mod refusal;
pub mod registry;
pub mod seams;
pub mod seed;
pub mod settings;
pub mod stamp;
pub mod store;
pub mod tags;
pub mod trim;
pub mod value;

pub use baking::{BakeNote, bake_notes};
pub use boxes::pixelate::{PIXEL_COUNTS, PIXELATE, pixel_grid};
pub use cache::Cache;
pub use eval::{BoxResult, Evaluation, Evaluator, Given, real_size_cm};
pub use graph::{
    BoxId, Found, FoundBy, FoundThing, Frame, FrameId, Graph, GraphBox, GraphEdit, GraphParts,
    MAX_BENDS, MAX_BOXES, MAX_FOUND, MAX_FRAMES, MAX_MOMENT, MAX_NAME, MAX_NOTE, MAX_ROLLED_SEED,
    MAX_VARIATION_VALUES, MAX_VARIATIONS, PlugRef, Shown, Variation, VariationId, VariationValue,
    Wire, changes_in_game, seed_keys, usable_moment,
};
pub use image::{Grey, Image, Rgb, Size};
pub use kind::{Fit, ValueKind};
pub use model::{ModelMap, ModelMaps, OnModel};
pub use own::{OwnBox, OwnBoxType, OwnInput, OwnOutput, OwnParts, OwnSlider};
pub use painting::{PAINTING_SIZES, Painting, STANDARD_PAINTING_SIZE};
pub use palettes::{Palette, Palettes};
pub use photo::{MAX_PHOTO_SIDE, Photo};
pub use refusal::{Fix, Refusal};
pub use registry::{
    BoxInfo, BoxType, Group, Job, Label, MIN_REAL_SIZE_CM, PlugSpec, Registry,
    STANDARD_REAL_SIZE_CM,
};
pub use seams::{Join, SeamCheck};
pub use settings::{
    ColourGradient, GradientStop, MAX_PALETTE_NAME, MAX_STOPS, SettingKind, SettingSpec,
    SettingValue, Settings,
};
pub use stamp::{MAX_STAMP_SIDE, MAX_STAMPS, Stamp, StampEntry, Stamps};
pub use store::{GraphId, GraphStore, MAX_GRAPHS};
pub use trim::{TrimBand, TrimRows, is_trim_sheet, uses_trim_rows};
pub use value::{MaterialMaps, Value};
