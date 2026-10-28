//! A shape as the brush sees it: its triangles with their UVs and
//! material slots, found quickly near a point or along a ray.
//!
//! Built from the shape as drawn, in its own space, with the same cut into
//! triangles as drawing, picking and export ([`mesh::face_triangles`]).

use glam::{Vec2, Vec3};
use mesh::Mesh;

/// One triangle of the shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintTriangle {
    /// Its corners, in the shape's own space.
    pub points: [Vec3; 3],
    /// Its corners' UVs.
    pub uvs: [Vec2; 3],
    /// Which way it faces (unit length, or zero for one with no area).
    pub normal: Vec3,
    /// Its polygon's material slot.
    pub slot: u32,
    /// Its polygon's id.
    pub face: u32,
}

/// The most cells the grid that finds triangles near a point may have.
const MAX_CELLS: usize = 1 << 18;

/// A shape's triangles, with a grid to find those near a point.
#[derive(Debug, Clone, Default)]
pub struct PaintSurface {
    triangles: Vec<PaintTriangle>,
    /// Whether the shape has UVs (without, there is nowhere to paint).
    has_uvs: bool,
    min: Vec3,
    cell: f32,
    cells: [usize; 3],
    /// For each cell, its triangles' numbers.
    grid: Vec<Vec<u32>>,
}

impl PaintSurface {
    /// The surface of `mesh`.
    pub fn of(mesh: &Mesh) -> Self {
        let uvs = mesh::uvs::Uvs::of(mesh);
        let (starts, corners) = mesh.face_layout();
        let mut triangles = Vec::new();
        for (f, list) in mesh.faces() {
            let start = starts[f.get() as usize] as usize;
            let slot = mesh::material::slot_of(mesh, f);
            mesh::face_triangles(mesh, list, |[a, b, c]| {
                let p = |i: usize| mesh.position(corners[start + i]).unwrap_or_default();
                let uv = |i: usize| uvs.as_ref().map_or(Vec2::ZERO, |u| u.at(start + i));
                let points = [p(a), p(b), p(c)];
                let normal = (points[1] - points[0])
                    .cross(points[2] - points[0])
                    .normalize_or_zero();
                triangles.push(PaintTriangle {
                    points,
                    uvs: [uv(a), uv(b), uv(c)],
                    normal,
                    slot,
                    face: f.get(),
                });
            });
        }
        let mut surface = Self {
            triangles,
            has_uvs: uvs.is_some(),
            ..Self::default()
        };
        surface.build_grid();
        surface
    }

    /// Its triangles.
    pub fn triangles(&self) -> &[PaintTriangle] {
        &self.triangles
    }

    /// Whether the shape has UVs to paint through.
    pub fn has_uvs(&self) -> bool {
        self.has_uvs
    }

    fn build_grid(&mut self) {
        let finite = |t: &PaintTriangle| t.points.iter().all(|p| p.is_finite());
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for t in self.triangles.iter().filter(|t| finite(t)) {
            for p in t.points {
                min = min.min(p);
                max = max.max(p);
            }
        }
        if min.x > max.x {
            return;
        }
        let extent = (max - min).max(Vec3::splat(1e-3));
        // About two triangles a cell, within the cap.
        let wanted = (self.triangles.len() / 2).clamp(1, MAX_CELLS) as f32;
        let cell = (extent.x * extent.y * extent.z / wanted).cbrt().max(1e-3);
        let count = |e: f32| ((e / cell).ceil() as usize).clamp(1, 256);
        self.cells = [count(extent.x), count(extent.y), count(extent.z)];
        self.min = min;
        self.cell = cell;
        self.grid = vec![Vec::new(); self.cells[0] * self.cells[1] * self.cells[2]];
        for (i, t) in self.triangles.iter().enumerate() {
            if !finite(t) {
                continue;
            }
            let lo = t.points[0].min(t.points[1]).min(t.points[2]);
            let hi = t.points[0].max(t.points[1]).max(t.points[2]);
            let (a, b) = (self.cell_of(lo), self.cell_of(hi));
            for z in a[2]..=b[2] {
                for y in a[1]..=b[1] {
                    for x in a[0]..=b[0] {
                        let at = self.index(x, y, z);
                        self.grid[at].push(i as u32);
                    }
                }
            }
        }
    }

    fn cell_of(&self, p: Vec3) -> [usize; 3] {
        let q = (p - self.min) / self.cell;
        let c = |v: f32, n: usize| (v.max(0.0) as usize).min(n - 1);
        [
            c(q.x, self.cells[0]),
            c(q.y, self.cells[1]),
            c(q.z, self.cells[2]),
        ]
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.cells[1] + y) * self.cells[0] + x
    }

    /// The triangles whose box comes within `radius` of `centre` (some
    /// may be a little farther), each once, into `out`.
    pub fn near(&self, centre: Vec3, radius: f32, out: &mut Vec<usize>) {
        out.clear();
        if self.grid.is_empty() || !centre.is_finite() || !radius.is_finite() {
            return;
        }
        let r = Vec3::splat(radius.max(0.0));
        let (a, b) = (self.cell_of(centre - r), self.cell_of(centre + r));
        for z in a[2]..=b[2] {
            for y in a[1]..=b[1] {
                for x in a[0]..=b[0] {
                    out.extend(self.grid[self.index(x, y, z)].iter().map(|&i| i as usize));
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out.retain(|&i| {
            let t = &self.triangles[i];
            let lo = t.points[0].min(t.points[1]).min(t.points[2]) - r;
            let hi = t.points[0].max(t.points[1]).max(t.points[2]) + r;
            centre.cmpge(lo).all() && centre.cmple(hi).all()
        });
    }

    /// The nearest triangle the ray from `origin` along `direction` (in the
    /// shape's own space) hits: how far along, and which triangle.
    pub fn hit(&self, origin: Vec3, direction: Vec3) -> Option<(f32, usize)> {
        let mut best: Option<(f32, usize)> = None;
        for (i, t) in self.triangles.iter().enumerate() {
            if let Some(d) = ray_triangle(origin, direction, &t.points)
                && best.is_none_or(|(b, _)| d < b)
            {
                best = Some((d, i));
            }
        }
        best
    }
}

/// How far a ray may pass outside a triangle's edge (in the triangle's
/// own coordinates) and still hit it: so a ray along the edge two
/// triangles share can't slip between them.
const EDGE_SLACK: f32 = 1e-5;

/// How far along the ray the triangle is hit (either side), if it is.
pub fn ray_triangle(origin: Vec3, direction: Vec3, [a, b, c]: &[Vec3; 3]) -> Option<f32> {
    let (e1, e2) = (*b - *a, *c - *a);
    let p = direction.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - *a;
    let u = s.dot(p) * inv;
    if !(-EDGE_SLACK..=1.0 + EDGE_SLACK).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = direction.dot(q) * inv;
    if v < -EDGE_SLACK || u + v > 1.0 + EDGE_SLACK {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.0).then_some(t)
}

/// The barycentric weights of `p` in the triangle `a`, `b`, `c` (in the
/// plane), or `None` for a triangle with no area.
pub fn barycentric(p: Vec2, [a, b, c]: [Vec2; 3]) -> Option<[f32; 3]> {
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let den = v0.x * v1.y - v1.x * v0.y;
    if den.abs() < 1e-14 {
        return None;
    }
    let v = (v2.x * v1.y - v1.x * v2.y) / den;
    let w = (v0.x * v2.y - v2.x * v0.y) / den;
    Some([1.0 - v - w, v, w])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_along_a_shared_edge_still_hits() {
        // Regression: a ray through the diagonal two triangles of a quad
        // share passed between them (found by colour picking).
        let surface = PaintSurface::of(&mesh::PrimitiveKind::Cube.standard().mesh());
        let camera = Vec3::new(560.0, -980.0, 330.0);
        let on_diagonal = Vec3::new(5.0, -50.0, 45.0);
        let (t, i) = surface
            .hit(camera, (on_diagonal - camera).normalize())
            .expect("the front is hit");
        assert!((camera + (on_diagonal - camera).normalize() * t).distance(on_diagonal) < 1e-2);
        assert!(
            surface.triangles()[i]
                .points
                .iter()
                .all(|p| (p.y + 50.0).abs() < 1e-4)
        );
    }
    use mesh::PrimitiveKind;

    #[test]
    fn a_cube_has_twelve_triangles_with_uvs() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        assert_eq!(surface.triangles().len(), 12);
        assert!(surface.has_uvs());
        assert!(surface.triangles().iter().all(|t| t.normal.length() > 0.99));
    }

    #[test]
    fn a_ray_hits_the_nearest_side() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        let (d, i) = surface.hit(Vec3::new(0.0, -500.0, 50.0), Vec3::Y).unwrap();
        // The cube is 100 cm across, standing on the ground.
        assert!((d - 450.0).abs() < 0.01, "{d}");
        assert!(surface.triangles()[i].normal.y < -0.99, "the front");
        assert!(
            surface
                .hit(Vec3::new(0.0, -500.0, 500.0), Vec3::Y)
                .is_none()
        );
    }

    #[test]
    fn triangles_near_a_point_are_found() {
        let surface = PaintSurface::of(&PrimitiveKind::Cube.standard().mesh());
        let mut near = Vec::new();
        // The middle of the front: only the front's two triangles.
        surface.near(Vec3::new(0.0, -50.0, 50.0), 5.0, &mut near);
        assert_eq!(near.len(), 2);
        // A corner: the three sides meeting there.
        surface.near(Vec3::new(50.0, -50.0, 100.0), 1.0, &mut near);
        let sides: std::collections::BTreeSet<i32> = near
            .iter()
            .map(|&i| {
                let n = surface.triangles()[i].normal;
                (n.x.round() * 1.0 + n.y.round() * 2.0 + n.z.round() * 4.0) as i32
            })
            .collect();
        assert_eq!(sides.len(), 3);
        // Far away: nothing.
        surface.near(Vec3::new(0.0, 0.0, 900.0), 1.0, &mut near);
        assert!(near.is_empty());
    }

    #[test]
    fn barycentric_weights_add_up() {
        let t = [Vec2::ZERO, Vec2::X, Vec2::Y];
        let w = barycentric(Vec2::new(0.25, 0.25), t).unwrap();
        assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!((w[1] - 0.25).abs() < 1e-6);
        assert!(barycentric(Vec2::ZERO, [Vec2::ZERO, Vec2::ZERO, Vec2::X]).is_none());
    }

    #[test]
    fn an_empty_shape_has_nothing_to_find() {
        let surface = PaintSurface::of(&Mesh::default());
        let mut near = vec![1];
        surface.near(Vec3::ZERO, 10.0, &mut near);
        assert!(near.is_empty());
        assert!(surface.hit(Vec3::ZERO, Vec3::X).is_none());
    }
}
