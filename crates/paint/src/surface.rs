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

    /// A surface of `triangles` as they are (for shapes made by hand,
    /// and tests), with UVs.
    pub fn from_triangles(triangles: Vec<PaintTriangle>) -> Self {
        let mut surface = Self {
            triangles,
            has_uvs: true,
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

    /// The point of the surface nearest `p` (in the shape's own space),
    /// on a triangle whose slot is in `slots`: which triangle, and the
    /// point's weights on it. `None` if there is no such triangle.
    pub fn closest(&self, p: Vec3, slots: &[u32]) -> Option<(usize, [f32; 3])> {
        if self.grid.is_empty() || !p.is_finite() {
            return None;
        }
        let across = Vec3::new(
            self.cells[0] as f32,
            self.cells[1] as f32,
            self.cells[2] as f32,
        ) * self.cell;
        let farthest = across.length() + (p - self.min).length();
        let mut radius = self.cell;
        let mut near = Vec::new();
        loop {
            self.near(p, radius, &mut near);
            let mut best: Option<(f32, usize, [f32; 3])> = None;
            for &i in &near {
                let t = &self.triangles[i];
                if !slots.contains(&t.slot) {
                    continue;
                }
                let (q, w) = closest_on_triangle(p, &t.points);
                let d = q.distance_squared(p);
                if best.is_none_or(|(b, _, _)| d < b) {
                    best = Some((d, i, w));
                }
            }
            // Found within the searched reach: nothing outside is nearer.
            if let Some((d, i, w)) = best
                && (d.sqrt() <= radius || radius >= farthest)
            {
                return Some((i, w));
            }
            if radius >= farthest {
                return None;
            }
            radius = (radius * 2.0).min(farthest);
        }
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

/// The point of triangle `a`, `b`, `c` nearest `p`, and its weights
/// (Ericson, *Real-Time Collision Detection*, 5.1.5).
pub fn closest_on_triangle(p: Vec3, [a, b, c]: &[Vec3; 3]) -> (Vec3, [f32; 3]) {
    let (ab, ac, ap) = (*b - *a, *c - *a, p - *a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (*a, [1.0, 0.0, 0.0]);
    }
    let bp = p - *b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (*b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (*a + ab * v, [1.0 - v, v, 0.0]);
    }
    let cp = p - *c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (*c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (*a + ac * w, [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (*b + (*c - *b) * w, [0.0, 1.0 - w, w]);
    }
    let sum = va + vb + vc;
    if sum.abs() < 1e-20 {
        return (*a, [1.0, 0.0, 0.0]);
    }
    let (v, w) = (vb / sum, vc / sum);
    (*a + ab * v + ac * w, [1.0 - v - w, v, w])
}

/// The most squares of the texture a triangle is followed across, each
/// way: UVs far outside the square are a mistake, not a repeat.
const MAX_TILES: i64 = 16;

/// The moves (whole texture sizes) that bring each part of a triangle at
/// `texel` (texel space, `size` pixels a square) back onto the texture,
/// where the texture repeats (decision 0202): just `(0, 0)` for a triangle
/// inside the square.
pub fn tile_shifts(texel: &[Vec2; 3], size: f32) -> Vec<Vec2> {
    let lo = texel[0].min(texel[1]).min(texel[2]);
    let hi = texel[0].max(texel[1]).max(texel[2]);
    if !lo.is_finite() || !hi.is_finite() || size <= 0.0 {
        return vec![Vec2::ZERO];
    }
    let span = |a: f32, b: f32| {
        let first = (a / size).floor() as i64;
        let last = ((b / size).ceil() as i64 - 1).max(first);
        (first, last.min(first + MAX_TILES - 1))
    };
    let (x0, x1) = span(lo.x, hi.x);
    let (y0, y1) = span(lo.y, hi.y);
    let mut out = Vec::new();
    for ky in y0..=y1 {
        for kx in x0..=x1 {
            out.push(Vec2::new(-(kx as f32) * size, -(ky as f32) * size));
        }
    }
    out
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
    fn triangles_past_the_square_are_moved_back_onto_it() {
        let inside = [
            Vec2::new(1.0, 1.0),
            Vec2::new(5.0, 1.0),
            Vec2::new(1.0, 5.0),
        ];
        assert_eq!(tile_shifts(&inside, 64.0), vec![Vec2::ZERO]);
        // A strip running from the square into the next one along.
        let along = [
            Vec2::new(60.0, 1.0),
            Vec2::new(70.0, 1.0),
            Vec2::new(60.0, 5.0),
        ];
        assert_eq!(
            tile_shifts(&along, 64.0),
            vec![Vec2::ZERO, Vec2::new(-64.0, 0.0)]
        );
        // Wholly two squares to the left.
        let left = [
            Vec2::new(-120.0, 1.0),
            Vec2::new(-110.0, 1.0),
            Vec2::new(-120.0, 5.0),
        ];
        assert_eq!(tile_shifts(&left, 64.0), vec![Vec2::new(128.0, 0.0)]);
        // Far away or broken: a few squares at most, or none moved.
        let far = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1e6, 0.0),
            Vec2::new(0.0, 5.0),
        ];
        assert_eq!(tile_shifts(&far, 64.0).len(), MAX_TILES as usize);
        let broken = [Vec2::NAN, Vec2::ZERO, Vec2::ONE];
        assert_eq!(tile_shifts(&broken, 64.0), vec![Vec2::ZERO]);
    }

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
