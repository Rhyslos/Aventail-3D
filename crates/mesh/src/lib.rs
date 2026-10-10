//! Polygon meshes: the data structure, topology queries and modelling
//! operations.
//!
//! Layer 0 (geometry). Knows nothing about scenes, files, the GPU or UI.
//! See this crate's README for its purpose and limits.

pub mod bounds;
pub mod polygon;
pub mod primitives;
pub mod shading;

pub use bounds::Bounds;
pub use polygon::{EdgeKey, FaceId, Mesh, MeshError, VertexId};
pub use primitives::{
    ConeParams, CubeParams, CylinderParams, DiscParams, PlaneParams, Primitive, PrimitiveKind,
    SphereParams, TubeParams,
};
pub use shading::{ShadedTriangle, corner_normals, shaded_triangles};
