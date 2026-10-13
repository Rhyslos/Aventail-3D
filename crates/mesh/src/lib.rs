//! Polygon meshes: the data structure, topology queries and modelling
//! operations.
//!
//! Layer 0 (geometry). Knows nothing about scenes, files, the GPU or UI.
//! See this crate's README for its purpose and limits.

pub mod anchors;
pub mod bevel;
pub mod bounds;
pub mod check;
pub mod edit;
pub mod extrude;
pub mod parallel;
pub mod polygon;
pub mod primitives;
pub mod samples;
pub mod selection;
pub mod shading;
pub mod topology;

pub use anchors::{AnchorShape, Anchors};
pub use bounds::Bounds;
pub use check::MeshReport;
pub use polygon::{EdgeKey, FaceId, Mesh, MeshError, VertexId};
pub use primitives::{
    ConeParams, CubeParams, CylinderParams, DiscParams, PlaneParams, Primitive, PrimitiveKind,
    SphereParams, TubeParams,
};
pub use selection::{Combine, Element, ElementKind, ElementSelection};
pub use shading::{
    ShadedTriangle, corner_normal_list, corner_normals, corner_shading, group_normals_at,
    shaded_triangles, shading_reach, smooth_groups,
};
pub use topology::{EdgeUse, Topology};
