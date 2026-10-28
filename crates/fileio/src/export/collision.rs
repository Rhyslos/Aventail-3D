//! Collision in parts in exports ([decision 0151](../../../../docs/decisions/0151-collision-hulls.md)):
//! an object *Wrapped in parts* gets each part's hull beside it, called
//! `name-part1-convcolonly` and so on, so Godot makes a convex collision
//! shape of each when it imports the file (and doesn't draw them), as it
//! does for *A box* (decision 0078).

use super::{ExportCorner, ExportMesh, ExportPolygon, ExportedObject};
use glam::Mat4;
use mesh::Mesh;
use scene::{Collision, ObjectId, Scene};
use std::sync::Arc;

/// The hulls of object `id`'s collision in parts, worked out from
/// `shape` (its exported shape) and placed by `matrix` (its shape's
/// space to the file's, around its pivot); none unless its collision is
/// in parts.
pub(super) fn part_hulls(
    scene: &Scene,
    id: ObjectId,
    shape: &Arc<Mesh>,
    matrix: Mat4,
) -> Vec<ExportMesh> {
    if scene.object(id).map(|o| o.collision) != Some(Collision::Parts) {
        return Vec::new();
    }
    let Some(shapes) = scene.collision_shapes(id, scene::Purpose::Export, Arc::clone(shape)) else {
        return Vec::new();
    };
    shapes
        .hulls
        .iter()
        .map(|hull| hull_mesh(hull, matrix))
        .collect()
}

/// `hull` placed by `matrix`, as triangles facing out with flat normals.
fn hull_mesh(hull: &mesh::hull::Hull, matrix: Mat4) -> ExportMesh {
    // A mirrored placement turns triangles inside out: run them backwards.
    let flipped = matrix.determinant() < 0.0;
    let points: Vec<glam::Vec3> = hull
        .points
        .iter()
        .map(|p| matrix.transform_point3(*p))
        .collect();
    let mut out = ExportMesh {
        points,
        ..ExportMesh::default()
    };
    for t in &hull.triangles {
        let mut t = *t;
        if flipped {
            t.swap(1, 2);
        }
        let [a, b, c] = t.map(|i| out.points[i as usize]);
        let normal = (b - a).cross(c - a).normalize_or(glam::Vec3::Y);
        let n = out.normals.len() as u32;
        out.normals.push(normal);
        out.polygons.push(ExportPolygon {
            corners: t
                .iter()
                .map(|p| ExportCorner {
                    colour: None,
                    point: *p,
                    normal: n,
                    uv: None,
                })
                .collect(),
            material: 0,
        });
    }
    out
}

/// The parts of every object in `objects` wrapped in parts, each as an
/// object of its own beside it, named for Godot (`name-part1-convcolonly`,
/// counting from 1), at the object's pivot.
pub(crate) fn part_objects(objects: &[ExportedObject]) -> Vec<ExportedObject> {
    objects
        .iter()
        .flat_map(|o| {
            o.collision_parts
                .iter()
                .enumerate()
                .map(move |(n, part)| ExportedObject {
                    name: format!("{}-part{}-convcolonly", o.name, n + 1),
                    world: part.clone(),
                    local: part.clone(),
                    collision: Collision::None,
                    collision_parts: Vec::new(),
                    levels: Vec::new(),
                    outline: None,
                    ..o.clone()
                })
        })
        .collect()
}
