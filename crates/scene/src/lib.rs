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

pub mod array;
mod axis_deform;
pub mod boolean;
pub mod collision;
pub mod command;
pub mod curve;
pub mod displace;
pub mod document;
pub mod game_bevel;
mod history;
mod kit;
pub mod lattice;
pub mod lod;
mod memory;
pub mod mirroring;
pub mod modifier;
pub mod object;
mod results_copy;
pub mod scatter;
pub mod snapshot;
pub mod stack;
pub mod tree;
pub mod trim;
pub mod units;
pub mod weighted;
pub mod wrap;

pub use array::ArraySettings;
pub use boolean::{
    Boolean, BooleanKind, BooleanProblem, BooleanRefusal, CUT_IN_BACKGROUND_FROM, CutDone, CutJob,
    CutState, MAX_CUTTERS,
};
pub use collision::{
    COLLISION_IN_BACKGROUND_FROM, CollisionDone, CollisionJob, CollisionShapes, CollisionState,
};
pub use command::Command;
pub use curve::{CurveObject, CurveShape, CurveSolid, Sweep, SweepProfile};
pub use displace::DisplaceSettings;
pub use document::Document;
pub use game_bevel::{GameBevelDone, game_bevel_options};
pub use history::HistoryLimit;
pub use kit::{KIT_GRID_CHOICES, KitGrid};
pub use lattice::LatticeOptions;
pub use lod::{
    HandLevel, LODS_IN_BACKGROUND_FROM, LodDone, LodJob, LodLevel, LodShapes, LodState, Lods,
    ModifierOverride, level_name,
};
pub use mesh::array::{ArrayAxis, ArrayLayout, ArrayOptions};
pub use mesh::axis_deform::{AxisDeformKind, AxisDeformOptions, DeformAxis, DeformHelper};
pub use mesh::convex_parts::ConvexPartsOptions;
pub use mesh::curve::bend::{BendAxis, BendOptions};
pub use mesh::curve::{
    Curve, CurveEdit, CurveEnd, CurvePart, CurvePoint, CurvePointId, Detail, LatheAxis, LatheShape,
    PartKind, PointKind, ReadyCurve, ReadyCurveKind, Span,
};
pub use mesh::decimate::{DecimateOptions, Keep as DecimateKeep, ReduceBy};
pub use mesh::displace::{DisplaceDirection, DisplaceOptions, NoiseKind};
pub use mesh::edit::{
    Bend, BevelOptions, BevelShape, ExtrudeDirection, ExtrudeShape, Eye, GrooveShape, GrooveSides,
    Grouping, Slide, SlitLine,
};
pub use mesh::hull::Hull;
pub use mesh::lattice::{Interpolation, Lattice, LatticeEdit};
pub use mesh::live_bevel::{BevelEdges, LiveBevelOptions};
pub use mesh::quadify::QuadifyOptions;
pub use mesh::remesh::RemeshOptions;
pub use mesh::scatter::{PlaceOn, ScatterOptions};
pub use mesh::shell::{ShellDirection, ShellOptions};
pub use mesh::shrinkwrap::{ShrinkwrapOptions, WrapMode};
pub use mesh::struts::StrutsOptions;
pub use mesh::subdivide::SubdivideOptions;
pub use mesh::triangulate::{TriangulateMethod, TriangulateOptions};
pub use mesh::uvs::{BUDGET_RANGE, SeamPreference, UvLayout};
pub use mesh::weighted_normals::{WeightBy, WeightedNormalsOptions};
pub use mesh::weld_distance::WeldOptions;
pub use mesh::{
    Bounds, Combine, EdgeKey, Element, ElementKind, ElementSelection, FaceId, Hardness, Mesh,
    Primitive, PrimitiveKind, VertexId,
};
pub use modifier::{
    MAX_MODIFIERS, MODIFIERS_IN_BACKGROUND_FROM, Modifier, ModifierContext, ModifierFamily,
    ModifierId, ModifierKind, ModifierRefusal, Purpose, SourceData, SourceShape, StackDone,
    StackJob, StackResult, StackState, has_nothing_to_do, subdivide_levels,
};
pub use object::{
    ApplyParts, Collision, LiveMirror, Material, Object, ObjectId, ObjectKind, Outline,
    STANDARD_SMOOTHING_ANGLE, Transform, hardness_is_valid,
};
pub use scatter::{MAX_SCATTER_SHAPES, ScatterSettings};
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
pub use wrap::{CurveBendSettings, ShrinkwrapSettings};

#[cfg(test)]
mod apply_tests;
#[cfg(test)]
mod array_tests;
#[cfg(test)]
mod axis_deform_tests;
#[cfg(test)]
mod boolean_tests;
#[cfg(test)]
mod collision_tests;
#[cfg(test)]
mod curve_editing_tests;
#[cfg(test)]
mod curve_tests;
#[cfg(test)]
mod decimate_tests;
#[cfg(test)]
mod displace_tests;
#[cfg(test)]
mod edit_list_tests;
#[cfg(test)]
mod game_bevel_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod instance_source_tests;
#[cfg(test)]
mod kind_change_tests;
#[cfg(test)]
mod lattice_tests;
#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod levels_tests;
#[cfg(test)]
mod live_bevel_tests;
#[cfg(test)]
mod live_mirror_tests;
#[cfg(test)]
mod lod_tests;
#[cfg(test)]
mod mirroring_tests;
#[cfg(test)]
mod modifier_list_tests;
#[cfg(test)]
mod modifier_tests;
#[cfg(test)]
mod outline_tests;
#[cfg(test)]
mod paint_tests;
#[cfg(test)]
mod pivot_tests;
#[cfg(test)]
mod quadify_tests;
#[cfg(test)]
mod scatter_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod stack_tests;
#[cfg(test)]
mod stack_work_tests;
#[cfg(test)]
mod struts_tests;
#[cfg(test)]
mod subdivide_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod triangulate_tests;
#[cfg(test)]
mod uv_pin_tests;
#[cfg(test)]
mod uv_survive_tests;
#[cfg(test)]
mod vertex_colour_tests;
#[cfg(test)]
mod weighted_tests;
#[cfg(test)]
mod weld_distance_tests;
#[cfg(test)]
mod wrap_tests;
