//! Bending a shape along a curve
//! ([decision 0147](../../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md)):
//! the work of the *Bend along curve* modifier.
//!
//! One of the shape's axes (its longest, standard) is laid along the
//! curve: the shape's start along that axis goes to the curve's start
//! (plus an *offset* along it), and each point travels as far along the
//! curve as it was along the axis, or proportionally with *stretch to
//! fit* so the shape's length becomes the curve's. Across the axis, a
//! point keeps its distance from the shape's pivot, measured in the
//! curve's frames there: the same rotation-minimising frames a sweep
//! follows (decision 0138), so the cross-section never flips or spins on
//! its own, with an optional *twist* on top. A shape whose pivot is on
//! its axis (a standing cylinder) is threaded on the curve through its
//! middle; one whose pivot is at its foot (a primitive lying along X)
//! rides on top of it.
//!
//! Past an open curve's ends the shape carries on straight, along the way
//! the curve leaves; round a closed curve it goes round again. Only the
//! points move: polygons, ids, UVs, seams and materials stay, as for
//! every deformer (`crate::deform`).

use super::Curve;
use super::path::{Detail, Frame, first_up, frames, samples};
use crate::deform::{Deformer, deform};
use crate::{Bounds, Mesh};
use glam::{Mat4, Vec3};
use words::tr;

/// Which of the shape's axes runs along the curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BendAxis {
    /// Whichever side of the shape's box is longest (X first on a tie).
    #[default]
    Longest,
    /// The shape's X.
    X,
    /// The shape's Y.
    Y,
    /// The shape's Z (up).
    Z,
}

impl BendAxis {
    /// Every choice, in the order the interface lists them.
    pub const ALL: [Self; 4] = [Self::Longest, Self::X, Self::Y, Self::Z];

    /// The axis (0 for X, 1 Y, 2 Z) for a shape in `bounds`.
    pub fn resolved(self, bounds: &Bounds) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
            Self::Longest => {
                let size = bounds.size();
                if size.x >= size.y && size.x >= size.z {
                    0
                } else if size.y >= size.z {
                    1
                } else {
                    2
                }
            }
        }
    }
}

/// How a shape is bent along a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendOptions {
    /// Which axis runs along the curve.
    pub axis: BendAxis,
    /// Whether the shape is stretched or squashed along the axis so its
    /// length is the curve's.
    pub stretch: bool,
    /// How far along the curve the shape starts, in centimetres (negative
    /// starts it before the curve's start).
    pub offset: f32,
    /// How far the shape turns round the curve from its start to its end,
    /// in degrees, on top of the curve's own frames (rounded to whole
    /// turns on a closed curve, so it meets itself).
    pub twist: f32,
}

impl Default for BendOptions {
    fn default() -> Self {
        Self {
            axis: BendAxis::Longest,
            stretch: false,
            offset: 0.0,
            twist: 0.0,
        }
    }
}

impl BendOptions {
    /// The furthest offset either way, in centimetres (1 km).
    pub const MAX_OFFSET: f32 = 100_000.0;
    /// The most twist either way, in degrees (a hundred turns), as a
    /// sweep's.
    pub const MAX_TWIST: f32 = super::sweep::MAX_TWIST;

    /// The same options within limits; values that aren't real numbers
    /// become the standard ones.
    pub fn clamped(self) -> Self {
        let real = |v: f32, limit: f32| {
            if v.is_finite() {
                v.clamp(-limit, limit)
            } else {
                0.0
            }
        };
        Self {
            axis: self.axis,
            stretch: self.stretch,
            offset: real(self.offset, Self::MAX_OFFSET),
            twist: real(self.twist, Self::MAX_TWIST),
        }
    }
}

/// Why a shape couldn't be bent along a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendProblem {
    /// The curve has no length (its points are all in one spot).
    NoLength,
}

impl std::fmt::Display for BendProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLength => f.write_str(tr(
                "the curve has no length: its points are all in one spot",
            )),
        }
    }
}

impl std::error::Error for BendProblem {}

/// One place along the curve, with its frame.
#[derive(Debug, Clone, Copy)]
struct Place {
    position: Vec3,
    frame: Frame,
    along: f32,
}

/// The bend: where each distance along the curve is, and how the shape's
/// axes sit before it is bent.
struct Along {
    places: Vec<Place>,
    length: f32,
    closed: bool,
    /// The axis running along the curve, and the two across it (the
    /// shape's "across" and "up" before bending, as a sweep's frame).
    axis: usize,
    across: Vec3,
    up: Vec3,
    /// Where the shape starts along its axis, how much it is stretched,
    /// and the offset along the curve.
    start: f32,
    scale: f32,
    offset: f32,
    pivot: Vec3,
}

impl Along {
    /// The place, way and up at distance `s` along the curve.
    fn at(&self, s: f32) -> (Vec3, Vec3, Vec3) {
        let first = &self.places[0];
        let last = &self.places[self.places.len() - 1];
        let s = if self.closed {
            s.rem_euclid(self.length)
        } else if s <= 0.0 {
            return (
                first.position + first.frame.tangent * s,
                first.frame.tangent,
                first.frame.up,
            );
        } else if s >= last.along {
            return (
                last.position + last.frame.tangent * (s - last.along),
                last.frame.tangent,
                last.frame.up,
            );
        } else {
            s
        };
        let i = self
            .places
            .partition_point(|p| p.along <= s)
            .clamp(1, self.places.len() - 1)
            - 1;
        let (a, b) = (&self.places[i], &self.places[i + 1]);
        let span = b.along - a.along;
        let f = if span > 1e-9 {
            ((s - a.along) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let position = a.position.lerp(b.position, f);
        let tangent = a
            .frame
            .tangent
            .lerp(b.frame.tangent, f)
            .normalize_or(a.frame.tangent);
        let up = a.frame.up.lerp(b.frame.up, f);
        let up = (up - tangent * tangent.dot(up)).normalize_or(a.frame.up);
        (position, tangent, up)
    }
}

impl Deformer for Along {
    fn moved(&self, p: Vec3) -> Vec3 {
        let s = (p[self.axis] - self.start) * self.scale + self.offset;
        let from_pivot = p - self.pivot;
        let (across, up) = (from_pivot.dot(self.across), from_pivot.dot(self.up));
        let (position, tangent, frame_up) = self.at(s);
        position + frame_up.cross(tangent) * across + frame_up * up
    }
}

/// The unit vector along axis `i`.
fn unit(i: usize) -> Vec3 {
    match i {
        0 => Vec3::X,
        1 => Vec3::Y,
        _ => Vec3::Z,
    }
}

/// `mesh` bent along `curve` (both in the same space), measuring across
/// the curve from `pivot` (the shape's pivot, in the same space). Only the
/// points move. An empty mesh comes back as it is; a curve with no length
/// is refused.
pub fn bend_along(
    mesh: &Mesh,
    curve: &Curve,
    pivot: Vec3,
    options: &BendOptions,
) -> Result<Mesh, BendProblem> {
    let options = options.clamped();
    let Some(bounds) = Bounds::around(mesh.vertices().map(|(_, p)| p).filter(|p| p.is_finite()))
    else {
        return Ok(mesh.clone());
    };
    let mut twist = options.twist;
    if curve.closed() {
        twist = (twist / 360.0).round() * 360.0;
    }
    let twist = twist.to_radians();
    let detail = Detail {
        segments: Detail::MAX_SEGMENTS,
        adaptive: true,
    };
    let found = samples(curve, detail, twist);
    if found.samples.len() < 2 || found.length < 1e-4 {
        return Err(BendProblem::NoLength);
    }
    let frames = frames(&found, twist);
    let mut places: Vec<Place> = found
        .samples
        .iter()
        .zip(frames)
        .map(|(s, frame)| Place {
            position: s.position,
            frame,
            along: s.along,
        })
        .collect();
    // Round a closed curve, the way back to the start is the last stretch.
    if found.closed {
        let mut back = places[0];
        back.along = found.length;
        places.push(back);
    }
    let axis = options.axis.resolved(&bounds);
    let tangent = unit(axis);
    let up = first_up(tangent);
    let extent = bounds.size()[axis];
    let scale = if options.stretch && extent > 1e-4 {
        found.length / extent
    } else {
        1.0
    };
    let along = Along {
        places,
        length: found.length,
        closed: found.closed,
        axis,
        across: up.cross(tangent),
        up,
        start: bounds.min[axis],
        scale,
        offset: options.offset,
        pivot: if pivot.is_finite() { pivot } else { Vec3::ZERO },
    };
    // The deformer works in the shape's own space.
    deform(mesh, Mat4::IDENTITY, &along).map_err(|_| BendProblem::NoLength)
}

#[cfg(test)]
mod tests;
