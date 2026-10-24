//! Error quadrics (Garland and Heckbert): sums of squared distances to
//! planes, and of squared differences from a texture's slope, kept as a
//! few numbers each so that merging two is adding them.

use glam::DVec3;

/// A sum of weighted squared distances to planes, as a function of a
/// place `x`: `xᵀAx + 2b·x + c` with `A` symmetric.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Quadric {
    /// `A`'s six different entries: xx, xy, xz, yy, yz, zz.
    a: [f64; 6],
    b: DVec3,
    c: f64,
}

impl Quadric {
    /// The squared distance to the plane through `point` facing `normal`
    /// (of length 1), times `weight`.
    pub(super) fn plane(normal: DVec3, point: DVec3, weight: f64) -> Self {
        let d = -normal.dot(point);
        let n = normal * weight;
        Self {
            a: [
                n.x * normal.x,
                n.x * normal.y,
                n.x * normal.z,
                n.y * normal.y,
                n.y * normal.z,
                n.z * normal.z,
            ],
            b: n * d,
            c: weight * d * d,
        }
    }

    /// Adds `other` to this one.
    pub(super) fn add(&mut self, other: &Self) {
        for (a, o) in self.a.iter_mut().zip(other.a) {
            *a += o;
        }
        self.b += other.b;
        self.c += other.c;
    }

    /// `xᵀAx`, the quadratic part at `x`.
    fn quadratic(a: &[f64; 6], x: DVec3) -> f64 {
        a[0] * x.x * x.x
            + a[3] * x.y * x.y
            + a[5] * x.z * x.z
            + 2.0 * (a[1] * x.x * x.y + a[2] * x.x * x.z + a[4] * x.y * x.z)
    }

    /// The error at `x` (never below 0: rounding can dip under it).
    pub(super) fn error(&self, x: DVec3) -> f64 {
        (Self::quadratic(&self.a, x) + 2.0 * self.b.dot(x) + self.c).max(0.0)
    }
}

/// How far a texture coordinate (one channel: U or V) moving with the
/// surface strays from what the original triangles gave there: for each
/// triangle, the channel is a slope `g` across its plane, `g·x + d`, and
/// the error at a place `x` with value `t` is `(g·x + d − t)²`, weighted
/// by the triangle's area. Kept as the sums that make it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Channel {
    /// Σ w·g·gᵀ (xx, xy, xz, yy, yz, zz).
    gg: [f64; 6],
    /// Σ w·g.
    g: DVec3,
    /// Σ w·g·d.
    gd: DVec3,
    /// Σ w·d.
    d: f64,
    /// Σ w·d².
    dd: f64,
    /// Σ w.
    w: f64,
}

impl Channel {
    fn of(slope: DVec3, offset: f64, weight: f64) -> Self {
        let g = slope * weight;
        Self {
            gg: [
                g.x * slope.x,
                g.x * slope.y,
                g.x * slope.z,
                g.y * slope.y,
                g.y * slope.z,
                g.z * slope.z,
            ],
            g,
            gd: g * offset,
            d: weight * offset,
            dd: weight * offset * offset,
            w: weight,
        }
    }

    fn add(&mut self, other: &Self) {
        for (a, o) in self.gg.iter_mut().zip(other.gg) {
            *a += o;
        }
        self.g += other.g;
        self.gd += other.gd;
        self.d += other.d;
        self.dd += other.dd;
        self.w += other.w;
    }

    fn error(&self, x: DVec3, t: f64) -> f64 {
        let e = Quadric::quadratic(&self.gg, x) + 2.0 * x.dot(self.gd) + self.dd
            - 2.0 * t * (x.dot(self.g) + self.d)
            + t * t * self.w;
        e.max(0.0)
    }
}

/// The texture error of one UV corner group (a *wedge*): both channels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct UvQuadric {
    u: Channel,
    v: Channel,
}

impl UvQuadric {
    /// The error of the triangle with corners at `p` and UVs `uv`,
    /// weighted by `weight` (its area). A triangle with no area gives
    /// nothing.
    pub(super) fn triangle(p: [DVec3; 3], uv: [[f64; 2]; 3], weight: f64) -> Self {
        let (e1, e2) = (p[1] - p[0], p[2] - p[0]);
        let n = e1.cross(e2);
        let nn = n.length_squared();
        if nn <= f64::MIN_POSITIVE || !nn.is_finite() || weight <= 0.0 {
            return Self::default();
        }
        // The slope within the plane: g·e1 = Δ1, g·e2 = Δ2, g·n = 0.
        let (c1, c2) = (e2.cross(n), n.cross(e1));
        let channel = |k: usize| {
            let slope = (c1 * (uv[1][k] - uv[0][k]) + c2 * (uv[2][k] - uv[0][k])) / nn;
            let offset = uv[0][k] - slope.dot(p[0]);
            Channel::of(slope, offset, weight)
        };
        Self {
            u: channel(0),
            v: channel(1),
        }
    }

    /// Adds `other` to this one.
    pub(super) fn add(&mut self, other: &Self) {
        self.u.add(&other.u);
        self.v.add(&other.v);
    }

    /// The error at place `x` with UV `uv`.
    pub(super) fn error(&self, x: DVec3, uv: [f64; 2]) -> f64 {
        self.u.error(x, uv[0]) + self.v.error(x, uv[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plane_measures_squared_distance() {
        let q = Quadric::plane(DVec3::Z, DVec3::new(3.0, 4.0, 2.0), 2.0);
        assert!(q.error(DVec3::new(-7.0, 1.0, 2.0)).abs() < 1e-12);
        assert!((q.error(DVec3::new(0.0, 0.0, 5.0)) - 18.0).abs() < 1e-9);
        let mut sum = q;
        sum.add(&Quadric::plane(DVec3::X, DVec3::ZERO, 1.0));
        assert!((sum.error(DVec3::new(1.0, 0.0, 3.0)) - (2.0 + 1.0)).abs() < 1e-9);
    }

    #[test]
    fn uvs_on_their_slope_cost_nothing_and_off_it_cost_their_square() {
        let p = [
            DVec3::ZERO,
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(0.0, 2.0, 0.0),
        ];
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let q = UvQuadric::triangle(p, uv, 1.0);
        // Halfway along the first side is U 0.5: no error there.
        assert!(q.error(DVec3::new(1.0, 0.0, 0.0), [0.5, 0.0]) < 1e-12);
        // A U off by 0.25 costs 0.0625.
        let off = q.error(DVec3::new(1.0, 0.0, 0.0), [0.75, 0.0]);
        assert!((off - 0.0625).abs() < 1e-9, "{off}");
        // A flat triangle gives nothing.
        let flat = UvQuadric::triangle([DVec3::ZERO; 3], uv, 1.0);
        assert_eq!(flat, UvQuadric::default());
    }
}
