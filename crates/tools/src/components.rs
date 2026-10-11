//! Finding points, edges, polygons and objects on screen: the one under the
//! mouse, and everything inside a box, a lasso or the selection brush
//! ([decision 0027](../../../docs/decisions/0027-component-modes.md),
//! [decision 0029](../../../docs/decisions/0029-selection.md)).
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
//! Only what can be seen counts under the mouse: a point or edge hidden
//! behind a surface (another object's, or the far side of its own) is
//! skipped, as is a polygon behind something else. Inside a [`Region`],
//! hidden elements count too unless only visible ones are asked for.

use crate::camera::{Camera, Ray};
use glam::{Mat4, Vec2, Vec3};
use mesh::{EdgeKey, FaceId, Mesh, VertexId};
pub use mesh::{Element, ElementKind};

/// How close to the mouse (in screen points) a point must appear to be
/// under it.
pub const POINT_RADIUS: f32 = 10.0;
/// How close to the mouse (in screen points) an edge must pass to be
/// under it.
pub const EDGE_RADIUS: f32 = 8.0;

/// An element found under the mouse: which object (its position in the
/// list the scene was built from) and which element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// One triangle of a face, in the world.
#[derive(Debug, Clone, Copy)]
struct Triangle {
    corners: [Vec3; 3],
    object: usize,
    face: FaceId,
}

/// A polygon placed in the world: its ID, its middle and its corners.
type PlacedPolygon = (FaceId, Vec3, Vec<(VertexId, Vec3)>);

/// One object's elements, placed in the world.
#[derive(Debug, Clone, Default)]
struct Placed {
    /// Whether its elements can be found (it is being edited).
    editable: bool,
    points: Vec<(VertexId, Vec3)>,
    edges: Vec<(EdgeKey, Vec3, Vec3)>,
    /// Each polygon's middle and corners.
    polygons: Vec<PlacedPolygon>,
}

/// The shown objects' meshes, placed in the world, ready for finding
/// what is under the mouse or inside a region.
#[derive(Debug, Clone, Default)]
pub struct PickScene {
    /// Every shown object's faces, cut into triangles: they hide what is
    /// behind them.
    triangles: Vec<Triangle>,
    /// Each object's elements, in the order given.
    objects: Vec<Placed>,
}

impl PickScene {
    /// Builds the scene from shown objects: each mesh, where it is in the
    /// world, and whether its elements can be found (the objects being
    /// edited). The others can still be picked as whole objects, and hide
    /// what is behind them. Objects placed with a broken transform are
    /// left out.
    pub fn new<'a>(objects: impl IntoIterator<Item = (&'a Mesh, Mat4, bool)>) -> Self {
        let mut scene = Self::default();
        for (index, (mesh, matrix, editable)) in objects.into_iter().enumerate() {
            if !matrix.is_finite() {
                scene.objects.push(Placed::default());
                continue;
            }
            let place = |v: VertexId| mesh.position(v).map(|p| matrix.transform_point3(p));
            let mut placed = Placed {
                editable,
                ..Placed::default()
            };
            for (face, corners) in mesh.faces() {
                let world: Vec<(VertexId, Vec3)> = corners
                    .iter()
                    .filter_map(|v| place(*v).map(|p| (*v, p)))
                    .collect();
                if let Some(((_, first), rest)) = world.split_first() {
                    for pair in rest.windows(2) {
                        scene.triangles.push(Triangle {
                            corners: [*first, pair[0].1, pair[1].1],
                            object: index,
                            face,
                        });
                    }
                    let middle = world.iter().map(|(_, p)| *p).sum::<Vec3>() / world.len() as f32;
                    placed.polygons.push((face, middle, world));
                }
            }
            placed.points = mesh
                .vertices()
                .map(|(v, p)| (v, matrix.transform_point3(p)))
                .collect();
            placed.edges = mesh
                .edges()
                .into_iter()
                .filter_map(|e| {
                    let (a, b) = e.ends();
                    Some((e, place(a)?, place(b)?))
                })
                .collect();
            scene.objects.push(placed);
        }
        scene
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
                let (_, triangle) = self.first_hit(&camera.ray_through(mouse, size))?;
                self.editable(triangle.object)?;
                Some(Hovered {
                    object: triangle.object,
                    element: Element::Polygon(triangle.face),
                })
            }
        }
    }

    /// The object whose surface is under the mouse, whether it is being
    /// edited or not.
    pub fn object_under(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<usize> {
        self.first_hit(&camera.ray_through(mouse, size))
            .map(|(_, t)| t.object)
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
        let (_, _, corners) = self
            .objects
            .get(object)?
            .polygons
            .iter()
            .find(|(f, ..)| *f == face)?;
        let n = corners.len();
        (0..n)
            .filter_map(|i| {
                let ((va, a), (vb, b)) = (corners[i], corners[(i + 1) % n]);
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
        for (object, placed) in self.objects.iter().enumerate() {
            if !placed.editable {
                continue;
            }
            let mut push = |element| out.push(Hovered { object, element });
            match kind {
                ElementKind::Points => {
                    for (id, p) in &placed.points {
                        if camera.project(*p, size).is_some_and(|s| region.contains(s)) && seen(*p)
                        {
                            push(Element::Point(*id));
                        }
                    }
                }
                ElementKind::Edges => {
                    for (edge, a, b) in &placed.edges {
                        let (Some(sa), Some(sb)) =
                            (camera.project(*a, size), camera.project(*b, size))
                        else {
                            continue;
                        };
                        let (inside, at) = if let Region::Circle { centre, radius } = region {
                            let (distance, t) = distance_to_segment(*centre, sa, sb);
                            (distance <= *radius, t)
                        } else {
                            (region.contains(sa) && region.contains(sb), 0.5)
                        };
                        if inside && seen(a.lerp(*b, at)) {
                            push(Element::Edge(*edge));
                        }
                    }
                }
                ElementKind::Polygons => {
                    for (face, middle, _) in &placed.polygons {
                        if camera
                            .project(*middle, size)
                            .is_some_and(|s| region.contains(s))
                            && seen(*middle)
                        {
                            push(Element::Polygon(*face));
                        }
                    }
                }
            }
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
        let mut out: Vec<usize> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, placed)| {
                placed.points.iter().any(|(_, p)| {
                    camera.project(*p, size).is_some_and(|s| region.contains(s)) && seen(*p)
                }) || placed.edges.iter().any(|(_, a, b)| {
                    let (Some(sa), Some(sb)) = (camera.project(*a, size), camera.project(*b, size))
                    else {
                        return false;
                    };
                    region.touches_segment(sa, sb)
                        && (!visible_only || self.edge_seen_in(camera, size, region, *a, *b))
                })
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
                .map(|(_, t)| vec![(t.object, t.face)])
                .unwrap_or_default();
        }
        let mut hits: Vec<(f32, usize, FaceId)> = self
            .triangles
            .iter()
            .filter_map(|t| hit(ray, &t.corners).map(|d| (d, t.object, t.face)))
            .collect();
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out: Vec<(usize, FaceId)> = Vec::new();
        for (_, object, face) in hits {
            if !out.contains(&(object, face)) {
                out.push((object, face));
            }
        }
        out
    }

    /// The object's elements if it is being edited.
    fn editable(&self, object: usize) -> Option<&Placed> {
        self.objects.get(object).filter(|p| p.editable)
    }

    fn point_under(&self, camera: &Camera, size: Vec2, mouse: Vec2) -> Option<Hovered> {
        let mut best: Option<(f32, Hovered)> = None;
        for (object, placed) in self.objects.iter().enumerate() {
            if !placed.editable {
                continue;
            }
            for (id, point) in &placed.points {
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
                            element: Element::Point(*id),
                        },
                    ));
                }
            }
        }
        best.map(|(_, hovered)| hovered)
    }

    fn edge_within(
        &self,
        camera: &Camera,
        size: Vec2,
        mouse: Vec2,
        radius: f32,
    ) -> Option<Hovered> {
        let mut best: Option<(f32, Hovered)> = None;
        for (object, placed) in self.objects.iter().enumerate() {
            if !placed.editable {
                continue;
            }
            for (edge, a, b) in &placed.edges {
                let (Some(sa), Some(sb)) = (camera.project(*a, size), camera.project(*b, size))
                else {
                    continue;
                };
                let (distance, t) = distance_to_segment(mouse, sa, sb);
                if distance > radius || best.is_some_and(|(d, _)| d <= distance) {
                    continue;
                }
                // The edge is seen where it passes the mouse.
                if self.can_see(camera, size, a.lerp(*b, t)) {
                    best = Some((
                        distance,
                        Hovered {
                            object,
                            element: Element::Edge(*edge),
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
        let mut best: Option<(f32, Vec3)> = None;
        let consider = |distance: f32, at: Vec3, best: &mut Option<(f32, Vec3)>| {
            if best.is_none_or(|(d, _)| distance < d) {
                *best = Some((distance, at));
            }
        };
        if targets.points {
            for (object, placed) in self.objects.iter().enumerate() {
                if whole(object) {
                    continue;
                }
                for (id, p) in &placed.points {
                    if moving(object, *id) {
                        continue;
                    }
                    let Some(s) = camera.project(*p, size) else {
                        continue;
                    };
                    let d = s.distance(mouse);
                    if d <= POINT_RADIUS && self.can_see(camera, size, *p) {
                        consider(d, *p, &mut best);
                    }
                }
            }
            if let Some((_, at)) = best {
                return Some(at);
            }
        }
        if targets.edges {
            for (object, placed) in self.objects.iter().enumerate() {
                if whole(object) {
                    continue;
                }
                for (edge, a, b) in &placed.edges {
                    let (va, vb) = edge.ends();
                    if moving(object, va) || moving(object, vb) {
                        continue;
                    }
                    let (Some(sa), Some(sb)) = (camera.project(*a, size), camera.project(*b, size))
                    else {
                        continue;
                    };
                    let (d, t) = distance_to_segment(mouse, sa, sb);
                    let at = a.lerp(*b, t);
                    if d <= EDGE_RADIUS && self.can_see(camera, size, at) {
                        consider(d, at, &mut best);
                    }
                }
            }
            if let Some((_, at)) = best {
                return Some(at);
            }
        }
        if targets.polygons {
            let ray = camera.ray_through(mouse, size);
            let still = |t: &Triangle| {
                !whole(t.object)
                    && self.objects[t.object]
                        .polygons
                        .iter()
                        .find(|(f, ..)| *f == t.face)
                        .is_none_or(|(_, _, corners)| {
                            !corners.iter().any(|(v, _)| moving(t.object, *v))
                        })
            };
            return self
                .triangles
                .iter()
                .filter(|t| still(t))
                .filter_map(|t| hit(&ray, &t.corners).map(|d| (d, *t)))
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(d, _)| ray.at(d));
        }
        None
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
