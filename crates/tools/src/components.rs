//! Finding points, edges, polygons and objects on screen: the one under the
//! mouse, and everything inside a box, a lasso or the selection brush
//! ([decision 0027](../../../docs/decisions/0027-component-modes.md),
//! [decision 0029](../../../docs/decisions/0029-selection.md)).
//!
//! Each shown object's mesh is indexed once ([`MeshPick`]: boxes inside
//! boxes around its triangles, points, edges and polygons), and a
//! [`PickScene`] places those in the world. Asking what is under the mouse
//! then only looks inside the boxes near it, so it stays quick for meshes
//! of millions of polygons ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)):
//!
//! - **Points** count when they appear within [`POINT_RADIUS`] of the
//!   mouse on screen; the closest wins.
//! - **Edges** count when they pass within [`EDGE_RADIUS`]; the closest
//!   wins.
//! - **Polygons**: the face the mouse ray hits first.
//!
//! Only what can be seen counts under the mouse: a point or edge hidden
//! behind a surface (another object's, or the far side of its own) is
//! skipped, as is a polygon behind something else. Inside a [`Region`],
//! hidden elements count too unless only visible ones are asked for.

use crate::bvh::{Aabb, Bvh};
use crate::camera::{Camera, Ray};
use glam::{Mat4, Vec2, Vec3};
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
pub use mesh::{Element, ElementKind};
use std::sync::{Arc, OnceLock};

/// How close to the mouse (in screen points) a point must appear to be
/// under it.
pub const POINT_RADIUS: f32 = 10.0;
/// How close to the mouse (in screen points) an edge must pass to be
/// under it.
pub const EDGE_RADIUS: f32 = 8.0;

/// An element found under the mouse: which object (its position in the
/// list the scene was built from) and which element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hovered {
    /// The object's index in the list given to [`PickScene::new`].
    pub object: usize,
    /// The element.
    pub element: Element,
}

/// An area of the screen to select in, in screen points from the view's
/// top-left corner.
#[derive(Debug, Clone, PartialEq)]
pub enum Region {
    /// A box between two corners (in any order).
    Rect {
        /// One corner.
        a: Vec2,
        /// The opposite corner.
        b: Vec2,
    },
    /// A closed outline drawn with the mouse; the last point joins the
    /// first.
    Lasso(Vec<Vec2>),
    /// The selection brush: a circle.
    Circle {
        /// Its middle.
        centre: Vec2,
        /// Its radius.
        radius: f32,
    },
}

impl Region {
    /// Whether the screen point is inside.
    pub fn contains(&self, p: Vec2) -> bool {
        match self {
            Self::Rect { a, b } => {
                let (min, max) = (a.min(*b), a.max(*b));
                p.cmpge(min).all() && p.cmple(max).all()
            }
            Self::Lasso(outline) => inside_outline(outline, p),
            Self::Circle { centre, radius } => p.distance(*centre) <= *radius,
        }
    }

    /// Whether the screen segment `a`–`b` passes through it at all.
    pub fn touches_segment(&self, a: Vec2, b: Vec2) -> bool {
        if self.contains(a) || self.contains(b) {
            return true;
        }
        match self {
            Self::Rect { a: c, b: d } => {
                let (min, max) = (c.min(*d), c.max(*d));
                let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
                (0..4).any(|i| segments_cross(a, b, corners[i], corners[(i + 1) % 4]))
            }
            Self::Lasso(outline) => {
                let n = outline.len();
                n >= 2 && (0..n).any(|i| segments_cross(a, b, outline[i], outline[(i + 1) % n]))
            }
            Self::Circle { centre, radius } => distance_to_segment(*centre, a, b).0 <= *radius,
        }
    }

    /// The box on screen around the region.
    fn screen_box(&self) -> (Vec2, Vec2) {
        match self {
            Self::Rect { a, b } => (a.min(*b), a.max(*b)),
            Self::Lasso(outline) => outline.iter().fold(
                (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
                |(lo, hi), p| (lo.min(*p), hi.max(*p)),
            ),
            Self::Circle { centre, radius } => around(*centre, *radius),
        }
    }

    /// A point well inside the region (the box's or circle's middle, or
    /// the lasso's average point).
    pub fn middle(&self) -> Vec2 {
        match self {
            Self::Rect { a, b } => (*a + *b) / 2.0,
            Self::Lasso(outline) if !outline.is_empty() => {
                outline.iter().copied().sum::<Vec2>() / outline.len() as f32
            }
            Self::Lasso(_) => Vec2::ZERO,
            Self::Circle { centre, .. } => *centre,
        }
    }
}

/// What a moving gizmo may snap to (the grid is handled by its caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapTargets {
    /// Points.
    pub points: bool,
    /// Spots along edges.
    pub edges: bool,
    /// Spots on polygons.
    pub polygons: bool,
}

impl Default for SnapTargets {
    fn default() -> Self {
        Self {
            points: true,
            edges: true,
            polygons: true,
        }
    }
}

/// One triangle of a face: the face, and which of its corners the
/// triangle joins (places in the face, as [`mesh::triangulate`] cuts it,
/// so picking finds what is drawn).
#[derive(Debug, Clone, Copy)]
struct Tri {
    face: FaceId,
    corners: [u32; 3],
}

/// One mesh, ready for finding what is under the mouse, in its own space:
/// its triangles, points, edges and polygons in boxes inside boxes (a
/// bounding volume hierarchy). Built once per shape and shared
/// ([`MeshPick::new`]); when only points have moved, a new one is made
/// from the old one far more quickly ([`MeshPick::moved`]). The point, edge and polygon indexes are only
/// built when first needed (the Points, Edges or Polygons mode).
#[derive(Debug)]
pub struct MeshPick {
    mesh: Arc<Mesh>,
    tris: Arc<Vec<Tri>>,
    /// Where each face's triangles start in `tris` (one more than faces).
    tri_starts: Arc<Vec<u32>>,
    tri_bvh: Bvh,
    /// The points that exist, by id number, and their index.
    points: OnceLock<(Arc<Vec<u32>>, Bvh)>,
    /// An index over the mesh's edges (as [`Mesh::edges`] lists them).
    edges: OnceLock<Bvh>,
    /// The faces that exist, and an index over their middles.
    faces: OnceLock<(Arc<Vec<FaceId>>, Bvh)>,
}

impl MeshPick {
    /// Gets `mesh` ready for picking.
    pub fn new(mesh: Arc<Mesh>) -> Self {
        let mut tris = Vec::new();
        let mut tri_starts = vec![0u32];
        for (index, corners) in mesh.face_slots().enumerate() {
            let face = FaceId::from_raw(index as u32);
            mesh::face_triangles(&mesh, corners.unwrap_or_default(), |t| {
                tris.push(Tri {
                    face,
                    corners: t.map(|i| i as u32),
                });
            });
            tri_starts.push(tris.len() as u32);
        }
        let tri_bvh = Bvh::build(tris.len(), |i| {
            Aabb::around(tri_corners(&mesh, tris[i as usize]).into_iter().flatten())
        });
        Self {
            mesh,
            tris: Arc::new(tris),
            tri_starts: Arc::new(tri_starts),
            tri_bvh,
            points: OnceLock::new(),
            edges: OnceLock::new(),
            faces: OnceLock::new(),
        }
    }

    /// `mesh` ready for picking, made from this one if `mesh` only moves
    /// points of the shape this was made for ([`Mesh::same_structure`]):
    /// the indexes keep their order and only their boxes are worked out
    /// again.
    pub fn moved(&self, mesh: Arc<Mesh>) -> Self {
        let Some(moved) = mesh.moved_since(&self.mesh) else {
            return Self::new(mesh);
        };
        // Only a few points moved: only the boxes around what uses them
        // are worked out again. Many: everything, which is then quicker.
        let few = moved.len() * 8 < self.mesh.vertex_count().max(1);
        let topology = mesh.topology();
        let faces: Vec<FaceId> = if few {
            let mut faces: Vec<FaceId> = moved
                .iter()
                .flat_map(|v| topology.vertex_faces(*v).iter().copied())
                .collect();
            faces.sort_unstable();
            faces.dedup();
            faces
        } else {
            Vec::new()
        };
        let tri_box = |i: u32| {
            Aabb::around(
                tri_corners(&mesh, self.tris[i as usize])
                    .into_iter()
                    .flatten(),
            )
        };
        let tri_bvh = if few {
            let items: Vec<u32> = faces
                .iter()
                .flat_map(|f| {
                    let f = f.get() as usize;
                    self.tri_starts[f]..self.tri_starts[f + 1]
                })
                .collect();
            self.tri_bvh.refit_items(&items, tri_box)
        } else {
            self.tri_bvh.refit(tri_box)
        };
        let points = OnceLock::new();
        if let Some((ids, bvh)) = self.points.get() {
            let point_box = |i: u32| point_box(&mesh, ids[i as usize]);
            let refit = if few {
                let items: Vec<u32> = moved
                    .iter()
                    .filter_map(|v| ids.binary_search(&v.get()).ok())
                    .map(|i| i as u32)
                    .collect();
                bvh.refit_items(&items, point_box)
            } else {
                bvh.refit(point_box)
            };
            let _ = points.set((Arc::clone(ids), refit));
        }
        let edges = OnceLock::new();
        if let Some(bvh) = self.edges.get() {
            let list = mesh.edges();
            let edge_box = |i: u32| edge_box(&mesh, list[i as usize]);
            let refit = if few {
                let items: Vec<u32> = moved
                    .iter()
                    .flat_map(|v| topology.vertex_edges(*v).iter())
                    .filter_map(|e| list.binary_search(e).ok())
                    .map(|i| i as u32)
                    .collect();
                bvh.refit_items(&items, edge_box)
            } else {
                bvh.refit(edge_box)
            };
            let _ = edges.set(refit);
        }
        let face_index = OnceLock::new();
        if let Some((ids, bvh)) = self.faces.get() {
            let face_box = |i: u32| face_box(&mesh, ids[i as usize]);
            let refit = if few {
                let items: Vec<u32> = faces
                    .iter()
                    .filter_map(|f| ids.binary_search(f).ok())
                    .map(|i| i as u32)
                    .collect();
                bvh.refit_items(&items, face_box)
            } else {
                bvh.refit(face_box)
            };
            let _ = face_index.set((Arc::clone(ids), refit));
        }
        Self {
            mesh,
            tris: Arc::clone(&self.tris),
            tri_starts: Arc::clone(&self.tri_starts),
            tri_bvh,
            points,
            edges,
            faces: face_index,
        }
    }

    /// The mesh it was made for.
    pub fn mesh(&self) -> &Arc<Mesh> {
        &self.mesh
    }

    fn points(&self) -> &(Arc<Vec<u32>>, Bvh) {
        self.points.get_or_init(|| {
            let ids: Vec<u32> = self.mesh.vertices().map(|(v, _)| v.get()).collect();
            let bvh = Bvh::build(ids.len(), |i| point_box(&self.mesh, ids[i as usize]));
            (Arc::new(ids), bvh)
        })
    }

    fn edges(&self) -> &Bvh {
        self.edges.get_or_init(|| {
            let list = self.mesh.edges();
            Bvh::build(list.len(), |i| edge_box(&self.mesh, list[i as usize]))
        })
    }

    fn faces(&self) -> &(Arc<Vec<FaceId>>, Bvh) {
        self.faces.get_or_init(|| {
            let ids: Vec<FaceId> = self.mesh.faces().map(|(f, _)| f).collect();
            let bvh = Bvh::build(ids.len(), |i| face_box(&self.mesh, ids[i as usize]));
            (Arc::new(ids), bvh)
        })
    }

    /// The nearest triangle `ray` (in the mesh's own space) hits, among
    /// those `allowed` lets through: how far along, and its face.
    fn first_hit(&self, ray: &Ray, allowed: &dyn Fn(FaceId) -> bool) -> Option<(f32, FaceId)> {
        self.tri_bvh
            .nearest(ray, |i| {
                let tri = self.tris[i as usize];
                if !allowed(tri.face) {
                    return None;
                }
                let [a, b, c] = tri_corners(&self.mesh, tri);
                hit(ray, &[a?, b?, c?])
            })
            .map(|(d, i)| (d, self.tris[i as usize].face))
    }
}

/// A triangle's corners.
fn tri_corners(mesh: &Mesh, tri: Tri) -> [Option<Vec3>; 3] {
    let corners = mesh.face(tri.face).unwrap_or_default();
    let at = |i: u32| corners.get(i as usize).and_then(|v| mesh.position(*v));
    tri.corners.map(at)
}

fn point_box(mesh: &Mesh, id: u32) -> Aabb {
    Aabb::around(mesh.position(VertexId::from_raw(id)))
}

fn edge_box(mesh: &Mesh, edge: EdgeKey) -> Aabb {
    let (a, b) = edge.ends();
    Aabb::around([mesh.position(a), mesh.position(b)].into_iter().flatten())
}

fn face_box(mesh: &Mesh, face: FaceId) -> Aabb {
    Aabb::around(face_middle(mesh, face))
}

/// The middle of a face (the average of its corners).
fn face_middle(mesh: &Mesh, face: FaceId) -> Option<Vec3> {
    let corners = mesh.face(face)?;
    let sum: Vec3 = corners.iter().filter_map(|v| mesh.position(*v)).sum();
    Some(sum / corners.len().max(1) as f32)
}

/// One shown object: its mesh ready for picking, where it is, and
/// whether its elements can be found.
#[derive(Debug, Clone)]
struct Placed {
    /// `None` for an object with a broken placement (left out).
    pick: Option<Arc<MeshPick>>,
    matrix: Mat4,
    inverse: Mat4,
    editable: bool,
}

impl Placed {
    /// `ray` in the object's own space. Its direction isn't unit length,
    /// so distances along it are the same as along the world ray.
    fn local(&self, ray: &Ray) -> Ray {
        Ray {
            origin: self.inverse.transform_point3(ray.origin),
            direction: self.inverse.transform_vector3(ray.direction),
        }
    }

    fn world(&self, p: Vec3) -> Vec3 {
        self.matrix.transform_point3(p)
    }

    /// Where a box of the object's own space appears on screen, as the
    /// box around its corners; `None` if part of it is behind the camera
    /// (so it can't be ruled out).
    fn on_screen(&self, b: &Aabb, camera: &Camera, size: Vec2) -> Option<(Vec2, Vec2)> {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for corner in b.corners() {
            let s = camera.project(self.world(corner), size)?;
            min = min.min(s);
            max = max.max(s);
        }
        Some((min, max))
    }

    /// Whether a box of the object's own space may appear within `area`
    /// on screen.
    fn may_reach(&self, b: &Aabb, camera: &Camera, size: Vec2, area: (Vec2, Vec2)) -> bool {
        self.on_screen(b, camera, size).is_none_or(|(min, max)| {
            min.x <= area.1.x && max.x >= area.0.x && min.y <= area.1.y && max.y >= area.0.y
        })
    }
}

/// The shown objects' meshes, placed in the world, ready for finding
/// what is under the mouse or inside a region. Cheap to make from
/// [`MeshPick`]s that already exist: only the placements are new.
#[derive(Debug, Clone, Default)]
pub struct PickScene {
    objects: Vec<Placed>,
}

/// The screen area within `radius` of `at`.
fn around(at: Vec2, radius: f32) -> (Vec2, Vec2) {
    (at - Vec2::splat(radius), at + Vec2::splat(radius))
}

impl PickScene {
    /// Builds the scene from shown objects: each mesh, where it is in the
    /// world, and whether its elements can be found (the objects being
    /// edited). The others can still be picked as whole objects, and hide
    /// what is behind them. Objects placed with a broken transform are
    /// left out. Each mesh is indexed afresh: when the meshes are kept
    /// between changes, use [`Self::from_picks`] with kept [`MeshPick`]s.
    pub fn new<'a>(objects: impl IntoIterator<Item = (&'a Mesh, Mat4, bool)>) -> Self {
        Self::from_picks(objects.into_iter().map(|(mesh, matrix, editable)| {
            (
                Arc::new(MeshPick::new(Arc::new(mesh.clone()))),
                matrix,
                editable,
            )
        }))
    }

    /// The scene from meshes already indexed.
    pub fn from_picks(objects: impl IntoIterator<Item = (Arc<MeshPick>, Mat4, bool)>) -> Self {
        let objects = objects
            .into_iter()
            .map(|(pick, matrix, editable)| {
                let usable = matrix.is_finite() && matrix.determinant().abs() > 1e-20;
                Placed {
                    pick: usable.then_some(pick),
                    matrix,
                    inverse: if usable {
                        matrix.inverse()
                    } else {
                        Mat4::IDENTITY
                    },
                    editable,
                }
            })
            .collect();
        Self { objects }
    }

    /// What of `kind` is under the mouse at `mouse` in a view of `size`
    /// (both in screen points, from the view's top-left corner), seen
    /// through `camera`. `None` when nothing is.
    pub fn under(
        &self,
        kind: ElementKind,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
    ) -> Option<Hovered> {
        match kind {
            ElementKind::Points => self.point_under(camera, size, mouse),
            ElementKind::Edges => self.edge_within(camera, size, mouse, EDGE_RADIUS),
            ElementKind::Polygons => {
                let (_, object, face) = self.first_hit(&camera.ray_through(mouse, size))?;
                self.editable(object)?;
                Some(Hovered {
                    object,
                    element: Element::Polygon(face),
                })
            }
        }
    }

    /// The object whose surface is under the mouse, whether it is being
    /// edited or not.
    pub fn object_under(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<usize> {
        self.first_hit(&camera.ray_through(mouse, size))
            .map(|(_, object, _)| object)
    }

    /// How far along the ray through the mouse the nearest surface is, if
    /// any is under it: for things drawn as lines (a lattice cage) that
    /// are picked only where they are in front.
    pub fn surface_distance(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<f32> {
        self.first_hit(&camera.ray_through(mouse, size))
            .map(|(distance, _, _)| distance)
    }

    /// The object under the mouse among those `wanted` lets through, or
    /// failing that any object under it: an outline-only cutter is picked
    /// only where nothing else is (decision 0135).
    pub fn object_under_preferring(
        &self,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
        wanted: &dyn Fn(usize) -> bool,
    ) -> Option<usize> {
        let ray = camera.ray_through(mouse, size);
        self.first_hit_where(&ray, &|object, _| wanted(object))
            .or_else(|| self.first_hit(&ray))
            .map(|(_, object, _)| object)
    }

    /// The visible edge of an editable object passing nearest the mouse,
    /// within `radius` screen points: for double-clicking a loop in the
    /// Points mode, where the mouse sits between two points.
    pub fn edge_near(
        &self,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
        radius: f32,
    ) -> Option<(usize, EdgeKey)> {
        let found = self.edge_within(camera, size, mouse, radius)?;
        match found.element {
            Element::Edge(edge) => Some((found.object, edge)),
            _ => None,
        }
    }

    /// The edge of `face` (of the object at `object`) that appears
    /// nearest the mouse: which way a double-click in the Polygons mode
    /// runs.
    pub fn nearest_edge_of(
        &self,
        object: usize,
        face: FaceId,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
    ) -> Option<EdgeKey> {
        let placed = self.objects.get(object)?;
        let mesh = &placed.pick.as_ref()?.mesh;
        let corners = mesh.face(face)?;
        let n = corners.len();
        (0..n)
            .filter_map(|i| {
                let (va, vb) = (corners[i], corners[(i + 1) % n]);
                let a = placed.world(mesh.position(va)?);
                let b = placed.world(mesh.position(vb)?);
                let (sa, sb) = (camera.project(a, size)?, camera.project(b, size)?);
                Some((distance_to_segment(mouse, sa, sb).0, EdgeKey::new(va, vb)))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, edge)| edge)
    }

    /// Every element of `kind` of the editable objects inside `region`
    /// (decision 0029):
    /// - **Points** inside it.
    /// - **Edges** with both ends inside a box or lasso, or passing
    ///   through the brush's circle.
    /// - **Polygons** whose middle is inside; the brush also catches the
    ///   polygons under its middle, so painting over a large polygon
    ///   selects it.
    ///
    /// With `visible_only`, anything hidden behind a surface is left out;
    /// otherwise the back of a shape is caught too.
    pub fn elements_in(
        &self,
        kind: ElementKind,
        camera: &Camera,
        size: Vec2,
        region: &Region,
        visible_only: bool,
    ) -> Vec<Hovered> {
        let mut out = Vec::new();
        let seen = |p: Vec3| !visible_only || self.can_see(camera, size, p);
        let brush = matches!(region, Region::Circle { .. });
        let area = region.screen_box();
        for (object, placed) in self.objects.iter().enumerate() {
            let Some(pick) = placed.pick.as_ref().filter(|_| placed.editable) else {
                continue;
            };
            let mesh = &pick.mesh;
            let keep = |b: &Aabb| placed.may_reach(b, camera, size, area);
            let mut found = Vec::new();
            match kind {
                ElementKind::Points => {
                    let (ids, bvh) = pick.points();
                    bvh.visit(keep, |i| {
                        let id = VertexId::from_raw(ids[i as usize]);
                        let Some(p) = mesh.position(id).map(|p| placed.world(p)) else {
                            return;
                        };
                        if camera.project(p, size).is_some_and(|s| region.contains(s)) && seen(p) {
                            found.push(Element::Point(id));
                        }
                    });
                }
                ElementKind::Edges => {
                    let list = mesh.edges();
                    pick.edges().visit(keep, |i| {
                        let edge = list[i as usize];
                        let (va, vb) = edge.ends();
                        let (Some(a), Some(b)) = (mesh.position(va), mesh.position(vb)) else {
                            return;
                        };
                        let (a, b) = (placed.world(a), placed.world(b));
                        let (Some(sa), Some(sb)) =
                            (camera.project(a, size), camera.project(b, size))
                        else {
                            return;
                        };
                        let (inside, at) = if let Region::Circle { centre, radius } = region {
                            let (distance, t) = distance_to_segment(*centre, sa, sb);
                            (distance <= *radius, t)
                        } else {
                            (region.contains(sa) && region.contains(sb), 0.5)
                        };
                        if inside && seen(a.lerp(b, at)) {
                            found.push(Element::Edge(edge));
                        }
                    });
                }
                ElementKind::Polygons => {
                    let (ids, bvh) = pick.faces();
                    bvh.visit(keep, |i| {
                        let face = ids[i as usize];
                        let Some(middle) = face_middle(mesh, face).map(|p| placed.world(p)) else {
                            return;
                        };
                        if camera
                            .project(middle, size)
                            .is_some_and(|s| region.contains(s))
                            && seen(middle)
                        {
                            found.push(Element::Polygon(face));
                        }
                    });
                }
            }
            // Keep each object's elements in id order, as a full scan
            // would list them.
            found.sort_unstable_by_key(element_order);
            out.extend(found.into_iter().map(|element| Hovered { object, element }));
        }
        if brush && kind == ElementKind::Polygons {
            let ray = camera.ray_through(region.middle(), size);
            for (object, face) in self.faces_along(&ray, visible_only) {
                let hovered = Hovered {
                    object,
                    element: Element::Polygon(face),
                };
                if self.editable(object).is_some() && !out.contains(&hovered) {
                    out.push(hovered);
                }
            }
        }
        out
    }

    /// The objects inside `region`: any with a point inside it, an edge
    /// passing through it, or a surface under its middle. With
    /// `visible_only`, an object only counts where it can be seen.
    pub fn objects_in(
        &self,
        camera: &Camera,
        size: Vec2,
        region: &Region,
        visible_only: bool,
    ) -> Vec<usize> {
        let seen = |p: Vec3| !visible_only || self.can_see(camera, size, p);
        let area = region.screen_box();
        let mut out: Vec<usize> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, placed)| {
                let Some(pick) = placed.pick.as_ref() else {
                    return false;
                };
                let mesh = &pick.mesh;
                let any = std::cell::Cell::new(false);
                let (ids, bvh) = pick.points();
                bvh.visit(
                    |b| !any.get() && placed.may_reach(b, camera, size, area),
                    |i| {
                        if any.get() {
                            return;
                        }
                        let Some(p) = mesh
                            .position(VertexId::from_raw(ids[i as usize]))
                            .map(|p| placed.world(p))
                        else {
                            return;
                        };
                        any.set(
                            camera.project(p, size).is_some_and(|s| region.contains(s)) && seen(p),
                        );
                    },
                );
                if any.get() {
                    return true;
                }
                let list = mesh.edges();
                pick.edges().visit(
                    |b| !any.get() && placed.may_reach(b, camera, size, area),
                    |i| {
                        if any.get() {
                            return;
                        }
                        let (va, vb) = list[i as usize].ends();
                        let (Some(a), Some(b)) = (mesh.position(va), mesh.position(vb)) else {
                            return;
                        };
                        let (a, b) = (placed.world(a), placed.world(b));
                        let (Some(sa), Some(sb)) =
                            (camera.project(a, size), camera.project(b, size))
                        else {
                            return;
                        };
                        any.set(
                            region.touches_segment(sa, sb)
                                && (!visible_only || self.edge_seen_in(camera, size, region, a, b)),
                        );
                    },
                );
                any.get()
            })
            .map(|(index, _)| index)
            .collect();
        let ray = camera.ray_through(region.middle(), size);
        for (object, _) in self.faces_along(&ray, visible_only) {
            if !out.contains(&object) {
                out.push(object);
            }
        }
        out.sort_unstable();
        out
    }

    /// Whether some part of the edge `a`–`b` inside `region` can be seen:
    /// tried at a few spots along it.
    fn edge_seen_in(&self, camera: &Camera, size: Vec2, region: &Region, a: Vec3, b: Vec3) -> bool {
        const SPOTS: usize = 8;
        (0..=SPOTS).any(|i| {
            let p = a.lerp(b, i as f32 / SPOTS as f32);
            camera.project(p, size).is_some_and(|s| region.contains(s))
                && self.can_see(camera, size, p)
        })
    }

    /// The faces the ray passes through, nearest first: only the first
    /// with `first_only`.
    fn faces_along(&self, ray: &Ray, first_only: bool) -> Vec<(usize, FaceId)> {
        if first_only {
            return self
                .first_hit(ray)
                .map(|(_, object, face)| vec![(object, face)])
                .unwrap_or_default();
        }
        let mut hits: Vec<(f32, usize, FaceId)> = Vec::new();
        for (object, placed) in self.objects.iter().enumerate() {
            let Some(pick) = &placed.pick else {
                continue;
            };
            let local = placed.local(ray);
            pick.tri_bvh.visit(
                |b| b.entry(&local).is_some(),
                |i| {
                    let tri = pick.tris[i as usize];
                    let [a, b, c] = tri_corners(&pick.mesh, tri);
                    if let (Some(a), Some(b), Some(c)) = (a, b, c)
                        && let Some(d) = hit(&local, &[a, b, c])
                    {
                        hits.push((d, object, tri.face));
                    }
                },
            );
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out: Vec<(usize, FaceId)> = Vec::new();
        for (_, object, face) in hits {
            if !out.contains(&(object, face)) {
                out.push((object, face));
            }
        }
        out
    }

    /// The object's placement if it is being edited.
    fn editable(&self, object: usize) -> Option<&Placed> {
        self.objects
            .get(object)
            .filter(|p| p.editable && p.pick.is_some())
    }

    fn point_under(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<Hovered> {
        // Every point near the mouse, nearest first; the first one that
        // can be seen wins. On a dense mesh many points crowd under the
        // mouse, so checking whether each can be seen would be slow: the
        // nearest is usually in front.
        let mut near: Vec<(f32, (usize, u64), Vec3, Hovered)> = Vec::new();
        for (object, placed) in self.objects.iter().enumerate() {
            let Some(pick) = placed.pick.as_ref().filter(|_| placed.editable) else {
                continue;
            };
            let (ids, bvh) = pick.points();
            bvh.visit(
                |b| placed.may_reach(b, camera, size, around(mouse, POINT_RADIUS)),
                |i| {
                    let id = VertexId::from_raw(ids[i as usize]);
                    let Some(point) = pick.mesh.position(id).map(|p| placed.world(p)) else {
                        return;
                    };
                    let Some(screen) = camera.project(point, size) else {
                        return;
                    };
                    let distance = screen.distance(mouse);
                    if distance <= POINT_RADIUS {
                        let element = Element::Point(id);
                        near.push((
                            distance,
                            (object, element_order(&element)),
                            point,
                            Hovered { object, element },
                        ));
                    }
                },
            );
        }
        self.first_seen(camera, size, near)
    }

    /// The first of `near` (distance on screen, an order for ties, where
    /// to check it can be seen, and what it is) that can be seen, nearest
    /// first.
    fn first_seen(
        &self,
        camera: &Camera,
        size: Vec2,
        mut near: Vec<(f32, (usize, u64), Vec3, Hovered)>,
    ) -> Option<Hovered> {
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        near.into_iter()
            .find(|(_, _, at, _)| self.can_see(camera, size, *at))
            .map(|(.., hovered)| hovered)
    }

    fn edge_within(
        &self,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
        radius: f32,
    ) -> Option<Hovered> {
        let mut near: Vec<(f32, (usize, u64), Vec3, Hovered)> = Vec::new();
        for (object, placed) in self.objects.iter().enumerate() {
            let Some(pick) = placed.pick.as_ref().filter(|_| placed.editable) else {
                continue;
            };
            let list = pick.mesh.edges();
            pick.edges().visit(
                |b| placed.may_reach(b, camera, size, around(mouse, radius)),
                |i| {
                    let edge = list[i as usize];
                    let (va, vb) = edge.ends();
                    let (Some(a), Some(b)) = (pick.mesh.position(va), pick.mesh.position(vb))
                    else {
                        return;
                    };
                    let (a, b) = (placed.world(a), placed.world(b));
                    let (Some(sa), Some(sb)) = (camera.project(a, size), camera.project(b, size))
                    else {
                        return;
                    };
                    let (distance, t) = distance_to_segment(mouse, sa, sb);
                    if distance <= radius {
                        let element = Element::Edge(edge);
                        // The edge is seen where it passes the mouse.
                        near.push((
                            distance,
                            (object, element_order(&element)),
                            a.lerp(b, t),
                            Hovered { object, element },
                        ));
                    }
                },
            );
        }
        self.first_seen(camera, size, near)
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
            .is_none_or(|(distance, ..)| distance >= depth - tolerance)
    }

    /// The nearest surface the ray hits: how far along, which object and
    /// which face.
    fn first_hit(&self, ray: &Ray) -> Option<(f32, usize, FaceId)> {
        self.first_hit_where(ray, &|_, _| true)
    }

    /// As [`Self::first_hit`], among the faces `allowed` lets through.
    fn first_hit_where(
        &self,
        ray: &Ray,
        allowed: &dyn Fn(usize, FaceId) -> bool,
    ) -> Option<(f32, usize, FaceId)> {
        let mut best: Option<(f32, usize, FaceId)> = None;
        for (object, placed) in self.objects.iter().enumerate() {
            let Some(pick) = &placed.pick else {
                continue;
            };
            let local = placed.local(ray);
            if let Some((d, face)) = pick.first_hit(&local, &|f| allowed(object, f))
                && best.is_none_or(|(b, ..)| d < b)
            {
                best = Some((d, object, face));
            }
        }
        best
    }

    /// What a moving gizmo snaps to near the mouse ([decision 0030](../../../docs/decisions/0030-transform-gizmos.md)):
    /// the nearest visible point within [`POINT_RADIUS`], else the nearest
    /// spot on a visible edge within [`EDGE_RADIUS`], else the surface
    /// under the mouse, as `targets` allows. What is being moved is never
    /// a target: whole objects for which `whole` says so, and points for
    /// which `moving` says so (with the edges and polygons they belong to).
    pub fn snap_target(
        &self,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
        targets: SnapTargets,
        whole: &dyn Fn(usize) -> bool,
        moving: &dyn Fn(usize, VertexId) -> bool,
    ) -> Option<Vec3> {
        // Candidates near the mouse, nearest first; the first that can be
        // seen wins (as for hovering).
        let nearest_seen = |mut near: Vec<(f32, Vec3)>| {
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            near.into_iter()
                .find(|(_, at)| self.can_see(camera, size, *at))
                .map(|(_, at)| at)
        };
        if targets.points {
            let mut near = Vec::new();
            for (object, placed) in self.objects.iter().enumerate() {
                let Some(pick) = placed.pick.as_ref().filter(|_| !whole(object)) else {
                    continue;
                };
                let (ids, bvh) = pick.points();
                bvh.visit(
                    |b| placed.may_reach(b, camera, size, around(mouse, POINT_RADIUS)),
                    |i| {
                        let id = VertexId::from_raw(ids[i as usize]);
                        if moving(object, id) {
                            return;
                        }
                        let Some(p) = pick.mesh.position(id).map(|p| placed.world(p)) else {
                            return;
                        };
                        let Some(s) = camera.project(p, size) else {
                            return;
                        };
                        let d = s.distance(mouse);
                        if d <= POINT_RADIUS {
                            near.push((d, p));
                        }
                    },
                );
            }
            if let Some(at) = nearest_seen(near) {
                return Some(at);
            }
        }
        if targets.edges {
            let mut near = Vec::new();
            for (object, placed) in self.objects.iter().enumerate() {
                let Some(pick) = placed.pick.as_ref().filter(|_| !whole(object)) else {
                    continue;
                };
                let list = pick.mesh.edges();
                pick.edges().visit(
                    |b| placed.may_reach(b, camera, size, around(mouse, EDGE_RADIUS)),
                    |i| {
                        let (va, vb) = list[i as usize].ends();
                        if moving(object, va) || moving(object, vb) {
                            return;
                        }
                        let (Some(a), Some(b)) = (pick.mesh.position(va), pick.mesh.position(vb))
                        else {
                            return;
                        };
                        let (a, b) = (placed.world(a), placed.world(b));
                        let (Some(sa), Some(sb)) =
                            (camera.project(a, size), camera.project(b, size))
                        else {
                            return;
                        };
                        let (d, t) = distance_to_segment(mouse, sa, sb);
                        if d <= EDGE_RADIUS {
                            near.push((d, a.lerp(b, t)));
                        }
                    },
                );
            }
            if let Some(at) = nearest_seen(near) {
                return Some(at);
            }
        }
        if targets.polygons {
            let ray = camera.ray_through(mouse, size);
            let still = |object: usize, face: FaceId| {
                !whole(object)
                    && self.objects[object]
                        .pick
                        .as_ref()
                        .and_then(|p| p.mesh.face(face))
                        .is_none_or(|corners| !corners.iter().any(|v| moving(object, *v)))
            };
            return self.first_hit_where(&ray, &still).map(|(d, ..)| ray.at(d));
        }
        None
    }

    /// How many triangles hide what is behind them (for tests and
    /// statistics).
    pub fn triangle_count(&self) -> usize {
        self.objects
            .iter()
            .filter_map(|p| p.pick.as_ref())
            .map(|p| p.tris.len())
            .sum()
    }
}

/// Orders elements by id, as a scan in id order meets them.
fn element_order(element: &Element) -> u64 {
    match element {
        Element::Point(v) => u64::from(v.get()),
        Element::Polygon(f) => u64::from(f.get()),
        Element::Edge(e) => {
            let (a, b) = e.ends();
            (u64::from(a.get()) << 32) | u64::from(b.get())
        }
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

/// Whether the segments `a`–`b` and `c`–`d` cross (touching counts).
fn segments_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let side = |p: Vec2, q: Vec2, r: Vec2| (q - p).perp_dot(r - p);
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    // Touching or lying along each other.
    let on = |p: Vec2, q: Vec2, r: Vec2, s: f32| {
        s.abs() <= 1e-6 && r.cmpge(p.min(q)).all() && r.cmple(p.max(q)).all()
    };
    on(c, d, a, d1) || on(c, d, b, d2) || on(a, b, c, d3) || on(a, b, d, d4)
}

/// Whether `p` is inside the closed outline (even–odd rule, so a lasso
/// that crosses itself leaves holes, as in drawing programs).
fn inside_outline(outline: &[Vec2], p: Vec2) -> bool {
    let n = outline.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
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
