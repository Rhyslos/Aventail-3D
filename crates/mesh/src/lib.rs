//! Polygon meshes: the data structure, topology queries and modelling
//! operations.
//!
//! Layer 0 (geometry). Knows nothing about scenes, files, the GPU or UI.
//! See this crate's README for its purpose and limits.

pub mod anchors;
pub mod attributes;
pub mod bake;
pub mod bevel;
pub mod bounds;
pub mod builder;
pub mod check;
pub mod cleanup;
pub mod crease;
pub mod curve;
pub mod decimate;
pub mod deform;
pub mod edit;
pub mod extrude;
pub mod groove;
pub mod hardness;
pub mod join;
pub mod lattice;
pub mod loopcut;
pub mod material;
pub mod mirror;
pub mod names;
pub mod parallel;
pub mod parts;
pub mod polygon;
pub mod primitives;
pub mod remesh;
pub mod samples;
pub mod scatter;
pub mod selection;
pub mod shading;
pub mod slide;
pub mod slit;
pub mod stretch;
pub mod subdivide;
pub mod symmetry;
pub mod topology;
pub mod triangulate;
pub mod uvs;

pub use anchors::{AnchorShape, Anchors};
pub use attributes::{
    AttributeElement, AttributeError, AttributeKind, AttributeValues, Domain, Value,
};
pub use bounds::Bounds;
pub use builder::{Builder, CornerFrom, PointFrom};
pub use check::MeshReport;
pub use polygon::{EdgeKey, FaceId, Mesh, MeshError, VertexId};
pub use primitives::{
    ArchParams, CapsuleParams, ConeParams, CubeParams, CylinderParams, DiscParams, DomeParams,
    ElbowParams, GearParams, HexGridParams, HexSphereParams, IcosphereParams, PlaneParams,
    Primitive, PrimitiveKind, SectorParams, SphereParams, StairsParams, TorusParams, TubeParams,
};
pub use selection::{Combine, Element, ElementKind, ElementSelection};
pub use shading::{
    Hardness, ShadedTriangle, angle_hardness_changed, corner_normal_list, corner_normals,
    corner_shading, corner_shading_with, group_normals_at, shaded_triangles, shading_reach,
    smooth_groups, smooth_groups_with,
};
pub use topology::{EdgeUse, Topology};
pub use triangulate::{face_triangles, triangulate};
