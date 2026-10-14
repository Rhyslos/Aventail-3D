//! Primitives: the ready-made shapes every model starts from (cube,
//! plane, cylinder, sphere, disc, tube and cone), each built from a few
//! settings such as size and number of segments.
//!
//! Every primitive stands on the ground with its pivot at the bottom
//! centre (the plane and disc lie flat on it), so it can be placed in a
//! game engine by its base. Sizes are in centimetres. Settings are kept
//! within sensible limits (see [`MIN_SIZE`] and friends) so a typo can't
//! make a shape too small to see or too big to build.
//!
//! Building the same settings always gives the same mesh, ids included.

mod boxy;
mod round;
#[cfg(test)]
mod tests;

use crate::{Bounds, Mesh};
use glam::Vec3;

/// The smallest size anything can have, in centimetres (1 mm).
pub const MIN_SIZE: f32 = 0.1;
/// The largest size anything can have, in centimetres (10 km).
pub const MAX_SIZE: f32 = 1_000_000.0;
/// The most segments along any one direction.
pub const MAX_SEGMENTS: u32 = 256;
/// The fewest sides a round shape can have (a triangle).
pub const MIN_SIDES: u32 = 3;
/// The most sides a round shape can have.
pub const MAX_SIDES: u32 = 512;

/// A box, standing on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubeParams {
    /// Width (X), depth (Y) and height (Z).
    pub size: Vec3,
    /// How many pieces each side is cut into, along X, Y and Z.
    pub segments: [u32; 3],
}

impl Default for CubeParams {
    fn default() -> Self {
        Self {
            size: Vec3::splat(100.0),
            segments: [1, 1, 1],
        }
    }
}

/// A flat rectangle lying on the ground, facing up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneParams {
    /// Size along X.
    pub width: f32,
    /// Size along Y.
    pub depth: f32,
    /// How many pieces it is cut into along X and Y.
    pub segments: [u32; 2],
}

impl Default for PlaneParams {
    fn default() -> Self {
        Self {
            width: 100.0,
            depth: 100.0,
            segments: [1, 1],
        }
    }
}

/// An upright cylinder, smooth around its side, with flat ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylinderParams {
    /// Radius.
    pub radius: f32,
    /// Height.
    pub height: f32,
    /// How many flat sides make up the round side.
    pub sides: u32,
    /// How many pieces the side is cut into from bottom to top.
    pub height_segments: u32,
    /// Whether the ends are closed.
    pub caps: bool,
    /// How many rings each closed end is cut into, from the rim inwards.
    pub cap_segments: u32,
    /// Whether the ends meet in a point in the middle (triangles around a
    /// centre point) rather than a flat many-cornered face (an n-gon).
    pub cap_centre: bool,
}

impl Default for CylinderParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            height: 100.0,
            sides: 16,
            height_segments: 1,
            caps: true,
            cap_segments: 1,
            cap_centre: false,
        }
    }
}

/// An upright cone; with a top radius above zero it is cut off flat (a
/// frustum).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConeParams {
    /// Radius at the bottom.
    pub bottom_radius: f32,
    /// Radius at the top; zero for a point.
    pub top_radius: f32,
    /// Height.
    pub height: f32,
    /// How many flat sides make up the round side.
    pub sides: u32,
    /// How many pieces the side is cut into from bottom to top.
    pub height_segments: u32,
    /// Whether the ends are closed.
    pub caps: bool,
    /// How many rings each closed end is cut into, from the rim inwards.
    pub cap_segments: u32,
    /// Whether the ends meet in a point in the middle (triangles around a
    /// centre point) rather than a flat many-cornered face (an n-gon).
    pub cap_centre: bool,
}

impl Default for ConeParams {
    fn default() -> Self {
        Self {
            bottom_radius: 50.0,
            top_radius: 0.0,
            height: 100.0,
            sides: 16,
            height_segments: 1,
            caps: true,
            cap_segments: 1,
            cap_centre: false,
        }
    }
}

/// A smooth ball resting on the ground, made of rings from pole to pole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereParams {
    /// Radius.
    pub radius: f32,
    /// How many pieces around (like lines of longitude).
    pub sides: u32,
    /// How many pieces from top to bottom (like lines of latitude).
    pub rings: u32,
}

impl Default for SphereParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            sides: 24,
            rings: 12,
        }
    }
}

/// A flat circle lying on the ground, facing up; with an inner radius
/// above zero it is a flat ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscParams {
    /// Outer radius.
    pub radius: f32,
    /// Radius of the hole in the middle; zero for none.
    pub inner_radius: f32,
    /// How many straight pieces make up the circle.
    pub sides: u32,
    /// How many rings it is cut into, from the rim inwards.
    pub ring_segments: u32,
    /// Whether the middle is a point with triangles around it rather than
    /// a flat many-cornered face (an n-gon). Only matters without a hole.
    pub centre_point: bool,
}

impl Default for DiscParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            inner_radius: 0.0,
            sides: 24,
            ring_segments: 1,
            centre_point: false,
        }
    }
}

/// An upright pipe: a cylinder with a round hole through it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeParams {
    /// Radius of the outside.
    pub outer_radius: f32,
    /// Radius of the hole.
    pub inner_radius: f32,
    /// Height.
    pub height: f32,
    /// How many flat sides make up each round side.
    pub sides: u32,
    /// How many pieces the sides are cut into from bottom to top.
    pub height_segments: u32,
}

impl Default for TubeParams {
    fn default() -> Self {
        Self {
            outer_radius: 50.0,
            inner_radius: 35.0,
            height: 100.0,
            sides: 24,
            height_segments: 1,
        }
    }
}

/// Which kind of primitive, without its settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    /// A box.
    Cube,
    /// A flat rectangle.
    Plane,
    /// An upright cylinder.
    Cylinder,
    /// A ball.
    Sphere,
    /// A flat circle or ring.
    Disc,
    /// A pipe.
    Tube,
    /// A cone or cut-off cone.
    Cone,
}

impl PrimitiveKind {
    /// Every kind, in the order the Create menu lists them.
    pub const ALL: [PrimitiveKind; 7] = [
        Self::Cube,
        Self::Plane,
        Self::Cylinder,
        Self::Sphere,
        Self::Disc,
        Self::Tube,
        Self::Cone,
    ];

    /// The kind's name, as used for new objects ("Cube").
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cube => "Cube",
            Self::Plane => "Plane",
            Self::Cylinder => "Cylinder",
            Self::Sphere => "Sphere",
            Self::Disc => "Disc",
            Self::Tube => "Tube",
            Self::Cone => "Cone",
        }
    }

    /// A plain-language description, for menus and tooltips.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Cube => "A 1 m box standing on the ground",
            Self::Plane => "A flat 1 m square lying on the ground",
            Self::Cylinder => "An upright cylinder, 1 m across and 1 m tall",
            Self::Sphere => "A ball 1 m across, resting on the ground",
            Self::Disc => "A flat circle 1 m across, lying on the ground",
            Self::Tube => "A pipe 1 m across and 1 m tall",
            Self::Cone => "A cone 1 m across and 1 m tall",
        }
    }

    /// This kind with its standard settings.
    pub fn standard(self) -> Primitive {
        match self {
            Self::Cube => Primitive::Cube(CubeParams::default()),
            Self::Plane => Primitive::Plane(PlaneParams::default()),
            Self::Cylinder => Primitive::Cylinder(CylinderParams::default()),
            Self::Sphere => Primitive::Sphere(SphereParams::default()),
            Self::Disc => Primitive::Disc(DiscParams::default()),
            Self::Tube => Primitive::Tube(TubeParams::default()),
            Self::Cone => Primitive::Cone(ConeParams::default()),
        }
    }
}

/// Any primitive, with its settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    /// See [`CubeParams`].
    Cube(CubeParams),
    /// See [`PlaneParams`].
    Plane(PlaneParams),
    /// See [`CylinderParams`].
    Cylinder(CylinderParams),
    /// See [`SphereParams`].
    Sphere(SphereParams),
    /// See [`DiscParams`].
    Disc(DiscParams),
    /// See [`TubeParams`].
    Tube(TubeParams),
    /// See [`ConeParams`].
    Cone(ConeParams),
}

impl Primitive {
    /// One of each kind, with standard settings, in the order the Create
    /// menu lists them.
    pub fn all() -> [Primitive; 7] {
        PrimitiveKind::ALL.map(PrimitiveKind::standard)
    }

    /// Which kind it is.
    pub fn kind(&self) -> PrimitiveKind {
        match self {
            Self::Cube(_) => PrimitiveKind::Cube,
            Self::Plane(_) => PrimitiveKind::Plane,
            Self::Cylinder(_) => PrimitiveKind::Cylinder,
            Self::Sphere(_) => PrimitiveKind::Sphere,
            Self::Disc(_) => PrimitiveKind::Disc,
            Self::Tube(_) => PrimitiveKind::Tube,
            Self::Cone(_) => PrimitiveKind::Cone,
        }
    }

    /// The kind's name, as used for new objects ("Cube").
    pub fn name(&self) -> &'static str {
        self.kind().name()
    }

    /// How many polygons the shape has, at most, worked out from its
    /// settings (brought within limits) without building it: for checking
    /// a file before opening it.
    pub fn polygon_estimate(&self) -> usize {
        let n = |x: u32| x as usize;
        match self.clamped() {
            Self::Cube(p) => {
                let [x, y, z] = p.segments.map(n);
                2 * (x * y + y * z + x * z)
            }
            Self::Plane(p) => n(p.segments[0]) * n(p.segments[1]),
            Self::Cylinder(p) => {
                let caps = if p.caps { 2 * n(p.cap_segments) } else { 0 };
                n(p.sides) * (n(p.height_segments) + caps)
            }
            Self::Sphere(p) => n(p.sides) * n(p.rings),
            Self::Disc(p) => n(p.sides) * n(p.ring_segments),
            Self::Tube(p) => n(p.sides) * (2 * n(p.height_segments) + 2),
            Self::Cone(p) => {
                let caps = if p.caps { 2 * n(p.cap_segments) } else { 0 };
                n(p.sides) * (n(p.height_segments) + caps)
            }
        }
    }

    /// The same primitive with its settings brought within limits:
    /// sizes between [`MIN_SIZE`] and [`MAX_SIZE`] (anything that isn't a
    /// number becomes the standard size), segment and side counts within
    /// theirs, and holes smaller than the shapes around them.
    #[must_use]
    pub fn clamped(&self) -> Self {
        match *self {
            Self::Cube(p) => {
                let d = CubeParams::default();
                Self::Cube(CubeParams {
                    size: Vec3::new(
                        size(p.size.x, d.size.x),
                        size(p.size.y, d.size.y),
                        size(p.size.z, d.size.z),
                    ),
                    segments: p.segments.map(segments),
                })
            }
            Self::Plane(p) => {
                let d = PlaneParams::default();
                Self::Plane(PlaneParams {
                    width: size(p.width, d.width),
                    depth: size(p.depth, d.depth),
                    segments: p.segments.map(segments),
                })
            }
            Self::Cylinder(p) => {
                let d = CylinderParams::default();
                Self::Cylinder(CylinderParams {
                    radius: size(p.radius, d.radius),
                    height: size(p.height, d.height),
                    sides: sides(p.sides),
                    height_segments: segments(p.height_segments),
                    caps: p.caps,
                    cap_segments: segments(p.cap_segments),
                    cap_centre: p.cap_centre,
                })
            }
            Self::Sphere(p) => Self::Sphere(SphereParams {
                radius: size(p.radius, SphereParams::default().radius),
                sides: sides(p.sides),
                rings: p.rings.clamp(2, MAX_SEGMENTS),
            }),
            Self::Disc(p) => {
                let radius = size(p.radius, DiscParams::default().radius);
                Self::Disc(DiscParams {
                    radius,
                    inner_radius: hole(p.inner_radius, radius),
                    sides: sides(p.sides),
                    ring_segments: segments(p.ring_segments),
                    centre_point: p.centre_point,
                })
            }
            Self::Tube(p) => {
                let d = TubeParams::default();
                let outer_radius = size(p.outer_radius, d.outer_radius);
                let inner = if p.inner_radius.is_finite() {
                    p.inner_radius
                } else {
                    d.inner_radius
                };
                // A tube always has a hole, smaller than the outside.
                let lowest = MIN_SIZE.min(outer_radius / 2.0);
                let highest = (outer_radius - MIN_SIZE).max(outer_radius / 2.0);
                Self::Tube(TubeParams {
                    outer_radius,
                    inner_radius: inner.clamp(lowest, highest),
                    height: size(p.height, d.height),
                    sides: sides(p.sides),
                    height_segments: segments(p.height_segments),
                })
            }
            Self::Cone(p) => {
                let d = ConeParams::default();
                let bottom = radius_or_point(p.bottom_radius, d.bottom_radius);
                let top = radius_or_point(p.top_radius, d.top_radius);
                // Both ends can't be points: that would be a line.
                let bottom = if bottom == 0.0 && top == 0.0 {
                    d.bottom_radius
                } else {
                    bottom
                };
                Self::Cone(ConeParams {
                    bottom_radius: bottom,
                    top_radius: top,
                    height: size(p.height, d.height),
                    sides: sides(p.sides),
                    height_segments: segments(p.height_segments),
                    caps: p.caps,
                    cap_segments: segments(p.cap_segments),
                    cap_centre: p.cap_centre,
                })
            }
        }
    }

    /// Builds the shape (with its settings brought within limits first).
    pub fn mesh(&self) -> Mesh {
        match self.clamped() {
            Self::Cube(p) => boxy::cube(&p),
            Self::Plane(p) => boxy::plane(&p),
            Self::Cylinder(p) => round::frustum(&round::Frustum {
                bottom_radius: p.radius,
                top_radius: p.radius,
                height: p.height,
                sides: p.sides,
                height_segments: p.height_segments,
                caps: p.caps,
                cap_segments: p.cap_segments,
                cap_centre: p.cap_centre,
            }),
            Self::Cone(p) => round::frustum(&round::Frustum {
                bottom_radius: p.bottom_radius,
                top_radius: p.top_radius,
                height: p.height,
                sides: p.sides,
                height_segments: p.height_segments,
                caps: p.caps,
                cap_segments: p.cap_segments,
                cap_centre: p.cap_centre,
            }),
            Self::Sphere(p) => round::sphere(&p),
            Self::Disc(p) => round::disc(&p),
            Self::Tube(p) => round::tube(&p),
        }
    }

    /// The box the shape fits in, in its own space, worked out from the
    /// settings without building the mesh.
    pub fn bounds(&self) -> Bounds {
        let (half_width, half_depth, height) = match self.clamped() {
            Self::Cube(p) => (p.size.x / 2.0, p.size.y / 2.0, p.size.z),
            Self::Plane(p) => (p.width / 2.0, p.depth / 2.0, 0.0),
            Self::Cylinder(p) => (p.radius, p.radius, p.height),
            Self::Cone(p) => {
                let r = p.bottom_radius.max(p.top_radius);
                (r, r, p.height)
            }
            Self::Sphere(p) => (p.radius, p.radius, p.radius * 2.0),
            Self::Disc(p) => (p.radius, p.radius, 0.0),
            Self::Tube(p) => (p.outer_radius, p.outer_radius, p.height),
        };
        Bounds {
            min: Vec3::new(-half_width, -half_depth, 0.0),
            max: Vec3::new(half_width, half_depth, height),
        }
    }
}

/// A size within limits; `standard` if it isn't a number.
fn size(value: f32, standard: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_SIZE, MAX_SIZE)
    } else {
        standard
    }
}

/// A radius that may be zero (a point), otherwise within limits.
fn radius_or_point(value: f32, standard: f32) -> f32 {
    if !value.is_finite() {
        standard
    } else if value < MIN_SIZE {
        0.0
    } else {
        value.min(MAX_SIZE)
    }
}

/// A hole's radius: zero (no hole) or smaller than `outer`.
fn hole(value: f32, outer: f32) -> f32 {
    if !value.is_finite() || value < MIN_SIZE {
        0.0
    } else {
        value.min(outer - MIN_SIZE).max(0.0)
    }
}

fn segments(value: u32) -> u32 {
    value.clamp(1, MAX_SEGMENTS)
}

fn sides(value: u32) -> u32 {
    value.clamp(MIN_SIDES, MAX_SIDES)
}
