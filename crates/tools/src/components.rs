//! Finding the point, edge or polygon under the mouse, for the Points,
//! Edges and Polygons modes ([decision 0027](../../../docs/decisions/0027-component-modes.md)).
//!
//! A [`PickScene`] holds every shown object's mesh placed in the world,
//! built once when the scene changes. Asking it what is under the mouse
//! then costs one pass over the editable objects' elements, plus a ray
//! cast for each one close enough to the mouse to matter:
//!
//! - **Points** count when they appear within [`POINT_RADIUS`] of the
//!   mouse on screen; the closest wins.
//! - **Edges** count when they pass within [`EDGE_RADIUS`]; the closest
//!   wins.
//! - **Polygons**: the face the mouse ray hits first.
//!
//! Only what can be seen counts: a point or edge hidden behind a surface
//! (another object's, or the far side of its own) is skipped, as is a
//! polygon behind something else.

use crate::camera::{Camera, Ray};
use glam::{Mat4, Vec2, Vec3};
use mesh::{EdgeKey, FaceId, Mesh, VertexId};

/// How close to the mouse (in screen points) a point must appear to be
/// under it.
pub const POINT_RADIUS: f32 = 10.0;
/// How close to the mouse (in screen points) an edge must pass to be
/// under it.
pub const EDGE_RADIUS: f32 = 8.0;

/// Which kind of element to look for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// Points (vertices).
    Points,
    /// Edges.
    Edges,
    /// Polygons (faces).
    Polygons,
}

/// One element of a mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    /// A point.
    Point(VertexId),
    /// An edge, named by its points.
    Edge(EdgeKey),
    /// A polygon.
    Polygon(FaceId),
}

/// An element found under the mouse: which object (its position in the
/// list the scene was built from) and which element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hovered {
    /// The object's index in the list given to [`PickScene::new`].
    pub object: usize,
    /// The element.
    pub component: Component,
}

/// One triangle of a face, in the world.
#[derive(Debug, Clone, Copy)]
struct Triangle {
    corners: [Vec3; 3],
    object: usize,
    face: FaceId,
}

/// An object whose elements can be found.
#[derive(Debug, Clone, Default)]
struct Editable {
    points: Vec<(VertexId, Vec3)>,
    edges: Vec<(EdgeKey, Vec3, Vec3)>,
}

/// The shown objects' meshes, placed in the world, ready for finding
/// what is under the mouse.
#[derive(Debug, Clone, Default)]
pub struct PickScene {
    /// Every shown object's faces, cut into triangles: they hide what is
    /// behind them.
    triangles: Vec<Triangle>,
    /// For each object, its elements if it is being edited.
    editable: Vec<Option<Editable>>,
}

impl PickScene {
    /// Builds the scene from shown objects: each mesh, where it is in the
    /// world, and whether its elements can be found (the objects being
    /// edited). The others only hide what is behind them. Objects placed
    /// with a broken transform are left out.
    pub fn new<'a>(objects: impl IntoIterator<Item = (&'a Mesh, Mat4, bool)>) -> Self {
        let mut scene = Self::default();
        for (index, (mesh, matrix, editable)) in objects.into_iter().enumerate() {
            if !matrix.is_finite() {
                scene.editable.push(None);
                continue;
            }
            let place = |v: VertexId| mesh.position(v).map(|p| matrix.transform_point3(p));
            for (face, corners) in mesh.faces() {
                let Some(first) = corners.first().and_then(|v| place(*v)) else {
                    continue;
                };
                for pair in corners[1..].windows(2) {
                    if let (Some(b), Some(c)) = (place(pair[0]), place(pair[1])) {
                        scene.triangles.push(Triangle {
                            corners: [first, b, c],
                            object: index,
                            face,
                        });
                    }
                }
            }
            scene.editable.push(editable.then(|| {
                Editable {
                    points: mesh
                        .vertices()
                        .map(|(v, p)| (v, matrix.transform_point3(p)))
                        .collect(),
                    edges: mesh
                        .edges()
                        .into_iter()
                        .filter_map(|e| {
                            let (a, b) = e.ends();
                            Some((e, place(a)?, place(b)?))
                        })
                        .collect(),
                }
            }));
        }
        scene
    }

    /// What of `kind` is under the mouse at `mouse` in a view of `size`
    /// (both in screen points, from the view's top-left corner), seen
    /// through `camera`. `None` when nothing is.
    pub fn under(
        &self,
        kind: ComponentKind,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
    ) -> Option<Hovered> {
        match kind {
            ComponentKind::Points => self.point_under(camera, size, mouse),
            ComponentKind::Edges => self.edge_under(camera, size, mouse),
            ComponentKind::Polygons => {
                let (_, triangle) = self.first_hit(&camera.ray_through(mouse, size))?;
                self.editable.get(triangle.object)?.as_ref()?;
                Some(Hovered {
                    object: triangle.object,
                    component: Component::Polygon(triangle.face),
                })
            }
        }
    }

    fn point_under(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<Hovered> {
        let mut best: Option<(f32, Hovered)> = None;
        for (object, editable) in self.editable.iter().enumerate() {
            let Some(editable) = editable else { continue };
            for (id, point) in &editable.points {
                let Some(screen) = camera.project(*point, size) else {
                    continue;
                };
                let distance = screen.distance(mouse);
                if distance > POINT_RADIUS || best.is_some_and(|(d, _)| d <= distance) {
                    continue;
                }
                if self.can_see(camera, size, *point) {
                    best = Some((
                        distance,
                        Hovered {
                            object,
                            component: Component::Point(*id),
                        },
                    ));
                }
            }
        }
        best.map(|(_, hovered)| hovered)
    }

    fn edge_under(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<Hovered> {
        let mut best: Option<(f32, Hovered)> = None;
        for (object, editable) in self.editable.iter().enumerate() {
            let Some(editable) = editable else { continue };
            for (edge, a, b) in &editable.edges {
                let (Some(sa), Some(sb)) = (camera.project(*a, size), camera.project(*b, size))
                else {
                    continue;
                };
                let (distance, t) = distance_to_segment(mouse, sa, sb);
                if distance > EDGE_RADIUS || best.is_some_and(|(d, _)| d <= distance) {
                    continue;
                }
                // The edge is seen where it passes the mouse.
                if self.can_see(camera, size, a.lerp(*b, t)) {
                    best = Some((
                        distance,
                        Hovered {
                            object,
                            component: Component::Edge(*edge),
                        },
                    ));
                }
            }
        }
        best.map(|(_, hovered)| hovered)
    }

    /// Whether `point` can be seen: nothing lies in front of it along the
    /// line of sight (allowing for it lying on a surface itself).
    fn can_see(&self, camera: &Camera, size: Vec2, point: Vec3) -> bool {
        let Some(screen) = camera.project(point, size) else {
            return false;
        };
        let ray = camera.ray_through(screen, size);
        let depth = (point - ray.origin).dot(ray.direction);
        let tolerance = 1e-3 * depth.abs() + 0.05;
        self.first_hit(&ray)
            .is_none_or(|(distance, _)| distance >= depth - tolerance)
    }

    /// The nearest triangle the ray hits, and how far along it.
    fn first_hit(&self, ray: &Ray) -> Option<(f32, Triangle)> {
        self.triangles
            .iter()
            .filter_map(|t| hit(ray, &t.corners).map(|d| (d, *t)))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// How many triangles hide what is behind them (for tests and
    /// statistics).
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

/// How far `point` is from the segment `a`–`b`, and where along it
/// (0 at `a`, 1 at `b`) the nearest spot is.
fn distance_to_segment(point: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let along = b - a;
    let length_squared = along.length_squared();
    let t = if length_squared > 1e-12 {
        ((point - a).dot(along) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((a + along * t).distance(point), t)
}

/// Where along `ray` it hits the triangle (either side), if it does
/// (Möller–Trumbore).
fn hit(ray: &Ray, [a, b, c]: &[Vec3; 3]) -> Option<f32> {
    let ab = *b - *a;
    let ac = *c - *a;
    let p = ray.direction.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / det;
    let to_origin = ray.origin - *a;
    let u = to_origin.dot(p) * inverse;
    if !(-1e-6..=1.0 + 1e-6).contains(&u) {
        return None;
    }
    let q = to_origin.cross(ab);
    let v = ray.direction.dot(q) * inverse;
    if v < -1e-6 || u + v > 1.0 + 1e-6 {
        return None;
    }
    let distance = ac.dot(q) * inverse;
    (distance > 1e-6 && distance.is_finite()).then_some(distance)
}

#[cfg(test)]
mod tests;
