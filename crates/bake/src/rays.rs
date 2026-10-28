//! Rays against a shape's triangles, for occlusion and thickness: a
//! bounding volume hierarchy (boxes inside boxes), so a ray tests a
//! handful of triangles instead of all of them.

use glam::Vec3;
use paint::PaintSurface;
use paint::surface::ray_triangle;

/// Triangles per box at the bottom.
const LEAF: usize = 4;

/// A box node: its box, and either two children or a run of triangles.
#[derive(Debug, Clone, Copy)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// For a branch, the second child (the first follows it); for a
    /// leaf, where its triangles start in the order.
    start: u32,
    /// How many triangles, 0 for a branch.
    count: u32,
}

/// A shape's triangles, ready for rays.
#[derive(Debug, Clone)]
pub struct Rays {
    triangles: Vec<[Vec3; 3]>,
    order: Vec<u32>,
    nodes: Vec<Node>,
}

impl Rays {
    /// `surface`'s triangles (all of them: anything can block a ray).
    pub fn of(surface: &PaintSurface) -> Self {
        let triangles: Vec<[Vec3; 3]> = surface
            .triangles()
            .iter()
            .map(|t| t.points)
            .filter(|p| p.iter().all(|v| v.is_finite()))
            .collect();
        let mut rays = Self {
            order: (0..triangles.len() as u32).collect(),
            triangles,
            nodes: Vec::new(),
        };
        if !rays.triangles.is_empty() {
            let count = rays.order.len();
            rays.build(0, count);
        }
        rays
    }

    /// Builds the node for `order[from..to]`, returning its index.
    fn build(&mut self, from: usize, to: usize) -> usize {
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let (mut cmin, mut cmax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for &i in &self.order[from..to] {
            let t = &self.triangles[i as usize];
            for p in t {
                min = min.min(*p);
                max = max.max(*p);
            }
            let c = (t[0] + t[1] + t[2]) / 3.0;
            cmin = cmin.min(c);
            cmax = cmax.max(c);
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            min,
            max,
            start: from as u32,
            count: (to - from) as u32,
        });
        if to - from <= LEAF {
            return index;
        }
        // Split at the middle along the longest side of the middles' box.
        let extent = cmax - cmin;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        let triangles = &self.triangles;
        let middle = |i: &u32| {
            let t = &triangles[*i as usize];
            ((t[0] + t[1] + t[2]) / 3.0)[axis]
        };
        let half = (from + to) / 2;
        self.order[from..to]
            .select_nth_unstable_by(half - from, |a, b| middle(a).total_cmp(&middle(b)));
        self.build(from, half);
        let second = self.build(half, to);
        self.nodes[index].start = second as u32;
        self.nodes[index].count = 0;
        index
    }

    /// How far along `direction` (unit length) from `origin` the nearest
    /// triangle is, if one is within `reach`.
    pub fn hit(&self, origin: Vec3, direction: Vec3, reach: f32) -> Option<f32> {
        if self.nodes.is_empty() {
            return None;
        }
        let inverse = direction.recip();
        let mut nearest: Option<f32> = None;
        let mut stack = vec![0usize];
        while let Some(at) = stack.pop() {
            let node = self.nodes[at];
            let far = nearest.unwrap_or(reach);
            if !box_hit(node.min, node.max, origin, inverse, far) {
                continue;
            }
            if node.count == 0 {
                stack.push(at + 1);
                stack.push(node.start as usize);
                continue;
            }
            let run = node.start as usize..(node.start + node.count) as usize;
            for &i in &self.order[run] {
                if let Some(d) = ray_triangle(origin, direction, &self.triangles[i as usize])
                    && d <= nearest.unwrap_or(reach)
                {
                    nearest = Some(d);
                }
            }
        }
        nearest
    }

    /// How open the surface at `at` facing `normal` is to the sky within
    /// `reach`: the share of `count` rays out over its side that meet
    /// nothing (1 fully open).
    pub fn openness(&self, at: Vec3, normal: Vec3, reach: f32, count: u32, seed: u32) -> f32 {
        let count = count.max(1);
        let from = at + normal * (reach * 1e-3);
        let blocked = (0..count)
            .filter(|&i| {
                let d = hemisphere(normal, seed, i, count);
                self.hit(from, d, reach).is_some()
            })
            .count();
        1.0 - blocked as f32 / count as f32
    }

    /// How thin the shape is at `at` (facing `normal`): rays sent in
    /// through the surface, and how far they go before leaving the shape
    /// on the other side, `reach` counting as thick. 1 for very thin, 0
    /// for `reach` or more.
    pub fn thinness(&self, at: Vec3, normal: Vec3, reach: f32, count: u32, seed: u32) -> f32 {
        let count = count.max(1);
        let from = at - normal * (reach * 1e-3);
        let total: f32 = (0..count)
            .map(|i| {
                let d = hemisphere(-normal, seed, i, count);
                self.hit(from, d, reach).unwrap_or(reach).min(reach)
            })
            .sum();
        1.0 - total / (count as f32 * reach.max(1e-9))
    }
}

/// Whether the ray from `origin` (with `inverse` its direction's
/// reciprocal) meets the box within `far`.
fn box_hit(min: Vec3, max: Vec3, origin: Vec3, inverse: Vec3, far: f32) -> bool {
    let a = (min - origin) * inverse;
    let b = (max - origin) * inverse;
    let near = a.min(b).max_element().max(0.0);
    let exit = a.max(b).min_element().min(far);
    // NaN (a ray along a box side) counts as a hit, to be safe.
    near.is_nan() || exit.is_nan() || near <= exit
}

/// Ray `i` of `count` over the half of the sphere around `normal`,
/// spread by cosine (more rays straight out than along the surface), the
/// same each time for the same `seed`.
fn hemisphere(normal: Vec3, seed: u32, i: u32, count: u32) -> Vec3 {
    // Spread evenly (a golden-ratio spiral), turned per texel.
    let turn = hash(seed) as f32 / u32::MAX as f32;
    let u = (i as f32 + 0.5) / count as f32;
    let angle = std::f32::consts::TAU * (i as f32 * 0.618_034 + turn);
    let r = u.sqrt();
    let (x, y, z) = (r * angle.cos(), r * angle.sin(), (1.0 - u).max(0.0).sqrt());
    let (t, b) = normal.any_orthonormal_pair();
    (t * x + b * y + normal * z).normalize_or_zero()
}

/// A well-mixed number from `v`.
fn hash(mut v: u32) -> u32 {
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^ (v >> 16)
}
