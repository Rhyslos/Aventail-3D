//! Sweeps: a cross-section (the *profile*) carried along a curve, making
//! a pipe, a cable, a rail or a trim.
//!
//! The profile is turned to follow the curve without flipping
//! (rotation-minimising frames, [`super::path::frames`]): it starts with
//! its y pointing up the world (Z) as far as the curve's first direction
//! allows, and its x across. At a sharp corner of the curve the profile
//! is stretched across the corner (a mitre), so the solid keeps its width
//! on both sides of it.
//!
//! The result is one UV piece for the length of it (U round the profile,
//! V along the curve, both at their real size) and one for each cap,
//! packed into the texture square as every primitive's are, with seams
//! where they part. The edges along the profile's sharp corners and round
//! the caps are hard, as are the rings at the curve's sharp corners.

use super::grid::{Grid, cap, join};
use super::path::{Detail, frames, samples};
use super::{Curve, path::Corner};
use crate::Mesh;
use crate::hardness::DEFAULT_ANGLE;
use crate::polygon::VertexId;
use crate::primitives::uvs::{Chart, set_charts};
use glam::{Vec2, Vec3};

/// The smallest size a profile may have: 0.01 cm.
pub const MIN_SIZE: f32 = 0.01;
/// The largest: 1 km.
pub const MAX_SIZE: f32 = 100_000.0;
/// The fewest and most sides a round profile may have.
pub const SIDES: std::ops::RangeInclusive<u32> = 3..=64;
/// The most polygons one sweep or lathe makes: more detail is cut back
/// evenly.
pub const MAX_POLYGONS: usize = 2_000_000;
/// The most twist, in degrees (either way): 100 turns.
pub const MAX_TWIST: f32 = 36_000.0;

/// A cross-section: points in its own flat plane (x across, y up),
/// round its middle (where the curve runs through it), each marked hard
/// where it is a sharp corner.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    points: Vec<Vec2>,
    hard: Vec<bool>,
    closed: bool,
}

/// A size within limits; one that isn't a number becomes the smallest.
fn size(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_SIZE, MAX_SIZE)
    } else {
        MIN_SIZE
    }
}

impl Profile {
    /// A circle `diameter` across, as `sides` straight pieces (within
    /// limits), all soft. Its first point is straight up.
    pub fn circle(diameter: f32, sides: u32) -> Self {
        let radius = size(diameter) / 2.0;
        let sides = sides.clamp(*SIDES.start(), *SIDES.end());
        let points = (0..sides)
            .map(|i| {
                let a =
                    std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / sides as f32;
                Vec2::new(a.cos(), a.sin()) * radius
            })
            .collect();
        Self {
            points,
            hard: vec![false; sides as usize],
            closed: true,
        }
    }

    /// A rectangle `width` across and `height` up (within limits), its
    /// corners hard.
    pub fn rectangle(width: f32, height: f32) -> Self {
        let (w, h) = (size(width) / 2.0, size(height) / 2.0);
        Self {
            points: vec![
                Vec2::new(-w, -h),
                Vec2::new(w, -h),
                Vec2::new(w, h),
                Vec2::new(-w, h),
            ],
            hard: vec![true; 4],
            closed: true,
        }
    }

    /// A profile from another curve (in its own space), laid flat on the
    /// plane it lies in (or nearest to), round that curve's own centre
    /// (its pivot): drawn in the top view, x is the world's X and y its Y;
    /// in the front view, X and Z; in the right view, Y and Z. Its sharp
    /// corners are hard. `None` for a curve with less than a line.
    pub fn from_curve(curve: &Curve, detail: Detail) -> Option<Self> {
        let places = samples(curve, detail, 0.0);
        if places.samples.len() < 2 {
            return None;
        }
        let positions: Vec<Vec3> = places.samples.iter().map(|s| s.position).collect();
        let normal = plane_of(&positions);
        let x = (Vec3::X - normal * normal.x)
            .try_normalize()
            .unwrap_or_else(|| (Vec3::Y - normal * normal.y).normalize_or(Vec3::Y));
        let y = normal.cross(x);
        let mut points = Vec::with_capacity(positions.len());
        let mut hard = Vec::with_capacity(positions.len());
        for sample in &places.samples {
            let p = Vec2::new(sample.position.dot(x), sample.position.dot(y));
            if points.last().is_some_and(|l: &Vec2| l.distance(p) < 1e-5) {
                continue;
            }
            points.push(p);
            hard.push(
                sample
                    .corner
                    .is_some_and(|c: Corner| c.angle() > DEFAULT_ANGLE.to_radians()),
            );
        }
        let closed = places.closed;
        if closed && points.len() > 1 && points[0].distance(points[points.len() - 1]) < 1e-5 {
            points.pop();
            hard.pop();
        }
        let enough = if closed { 3 } else { 2 };
        (points.len() >= enough).then_some(Self {
            points,
            hard,
            closed,
        })
    }

    /// Its points.
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    /// Whether it closes round (a tube) or stays open (a strip).
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// Twice its area, positive when its points go counter-clockwise.
    fn winding(&self) -> f32 {
        let n = self.points.len();
        (0..n)
            .map(|i| {
                let (a, b) = (self.points[i], self.points[(i + 1) % n]);
                a.perp_dot(b)
            })
            .sum()
    }

    /// The same profile going counter-clockwise, so a closed sweep faces
    /// outward.
    fn counter_clockwise(mut self) -> Self {
        if self.closed && self.winding() < 0.0 {
            self.points.reverse();
            self.hard.reverse();
        }
        self
    }
}

/// The plane points lie in (or nearest to): its facing, signed so the top
/// view's plane faces up (+Z), the front view's the viewer (−Y) and the
/// right view's +X.
fn plane_of(points: &[Vec3]) -> Vec3 {
    let mut n = Vec3::ZERO;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        n += Vec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    let n = n.try_normalize().unwrap_or_else(|| {
        // A straight line: a plane standing up through it (or, for an
        // upright line, the front view's).
        let way = (points[points.len() - 1] - points[0]).normalize_or(Vec3::X);
        way.cross(Vec3::Z)
            .try_normalize()
            .unwrap_or_else(|| way.cross(Vec3::X).normalize_or(Vec3::Y))
    });
    if n.dot(Vec3::new(1.0, -1.0, 1.0)) < 0.0 {
        -n
    } else {
        n
    }
}

/// How a sweep follows its curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepShape {
    /// How finely.
    pub detail: Detail,
    /// How far the profile turns round the curve from start to end, in
    /// degrees (rounded to whole turns on a closed curve, so it meets
    /// itself).
    pub twist: f32,
    /// Whether an open curve's ends are closed with flat faces.
    pub caps: bool,
}

impl Default for SweepShape {
    fn default() -> Self {
        Self {
            detail: Detail::default(),
            twist: 0.0,
            caps: true,
        }
    }
}

/// How much a corner stretches the profile across it: at most four times,
/// for a corner turning almost straight back.
fn mitre(corner: &Corner) -> f32 {
    (1.0 / (corner.angle() / 2.0).cos().max(0.25)).min(4.0)
}

/// The solid made by carrying `profile` along `path`, in the path's own
/// space. An empty mesh for a path with less than a line.
pub fn sweep(path: &Curve, profile: &Profile, shape: &SweepShape) -> Mesh {
    let profile = profile.clone().counter_clockwise();
    let mut twist = if shape.twist.is_finite() {
        shape.twist.clamp(-MAX_TWIST, MAX_TWIST)
    } else {
        0.0
    };
    if path.closed() {
        twist = (twist / 360.0).round() * 360.0;
    }
    let twist = twist.to_radians();
    let n = profile.points.len();
    let mut detail = shape.detail.clamped();
    let mut places = samples(path, detail, twist);
    // Too much detail: cut back evenly.
    while places.samples.len() * n > MAX_POLYGONS && detail.segments > 1 {
        detail.segments = (detail.segments / 2).max(1);
        places = samples(path, detail, twist);
    }
    let mut mesh = Mesh::new();
    if places.samples.len() < 2 || n < 2 || places.samples.len() * n > MAX_POLYGONS * 2 {
        return mesh;
    }
    let frames = frames(&places, twist);
    let rows: Vec<Vec<VertexId>> = places
        .samples
        .iter()
        .zip(&frames)
        .map(|(sample, frame)| {
            // At a corner, stretched across it.
            let stretch = sample.corner.map(|c| {
                let way = c.after - c.before;
                let side = (way - frame.tangent * way.dot(frame.tangent)).normalize_or(Vec3::ZERO);
                (side, mitre(&c))
            });
            profile
                .points
                .iter()
                .map(|q| {
                    let mut offset = frame.across * q.x + frame.up * q.y;
                    if let Some((side, factor)) = stretch {
                        offset += side * offset.dot(side) * (factor - 1.0);
                    }
                    mesh.push_vertex(sample.position + offset)
                })
                .collect()
        })
        .collect();
    // U round the profile, V along the curve, at their real sizes.
    let mut u = vec![0.0];
    for i in 1..=n {
        if i == n && !profile.closed {
            break;
        }
        let d = profile.points[i - 1].distance(profile.points[i % n]);
        u.push(u[i - 1] + d);
    }
    let mut v: Vec<f32> = places.samples.iter().map(|s| s.along).collect();
    if places.closed {
        v.push(places.length);
    }
    let columns = if profile.closed { n } else { n - 1 };
    let (tube, _) = join(
        &mut mesh,
        &Grid {
            rows: &rows,
            columns,
            wrap_rows: places.closed,
            u: &u,
            v: &v,
            flip: false,
        },
    );
    let mut charts: Vec<Chart> = vec![tube];
    harden(&mut mesh, &rows, &profile, &places.samples, places.closed);
    if shape.caps && profile.closed && !places.closed {
        let first = &frames[0];
        let last = &frames[frames.len() - 1];
        if let Some((chart, _)) = cap(&mut mesh, &rows[0], -first.tangent) {
            charts.push(chart);
        }
        if let Some((chart, _)) = cap(&mut mesh, &rows[rows.len() - 1], last.tangent) {
            charts.push(chart);
        }
    }
    set_charts(&mut mesh, &charts);
    mesh
}

/// Marks hard the edges along the profile's sharp corners, and the rings
/// at the curve's sharp corners (those turning more than 30°).
fn harden(
    mesh: &mut Mesh,
    rows: &[Vec<VertexId>],
    profile: &Profile,
    samples: &[super::path::Sample],
    closed: bool,
) {
    let count = rows.len();
    let bands = if closed { count } else { count - 1 };
    for (j, hard) in profile.hard.iter().enumerate() {
        if !hard {
            continue;
        }
        for i in 0..bands {
            mesh.set_hard(rows[i][j], rows[(i + 1) % count][j], true);
        }
    }
    let n = profile.points.len();
    let around = if profile.closed { n } else { n - 1 };
    for (row, sample) in rows.iter().zip(samples) {
        if sample
            .corner
            .is_some_and(|c| c.angle() > DEFAULT_ANGLE.to_radians())
        {
            for j in 0..around {
                mesh.set_hard(row[j], row[(j + 1) % n], true);
            }
        }
    }
}
