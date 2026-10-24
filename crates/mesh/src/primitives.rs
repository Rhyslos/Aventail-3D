//! Primitives: the ready-made shapes every model starts from (cube,
//! plane, cylinder, sphere, disc, tube and cone; and, from decision 0129,
//! icosphere, torus, capsule, dome, pipe elbow, pie sector, gear, hex
//! grid, hex sphere, stairs and arch), each built from a few settings
//! such as size and number of segments.
//!
//! Every primitive stands on the ground with its pivot at the bottom
//! centre (the plane and disc lie flat on it), so it can be placed in a
//! game engine by its base. Sizes are in centimetres. Settings are kept
//! within sensible limits (see [`MIN_SIZE`] and friends) so a typo can't
//! make a shape too small to see or too big to build.
//!
//! Building the same settings always gives the same mesh, ids included.

mod boxy;
mod lathe;
mod polyhedra;
mod prism;
mod round;
#[cfg(test)]
mod tests;
pub(crate) mod uvs;

use crate::{Bounds, Mesh};
use glam::Vec3;
use words::{text, tr};

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
/// The most times an icosphere or hex sphere is subdivided (an icosphere
/// at 6 has 81,920 triangles).
pub const MAX_SUBDIVISIONS: u32 = 6;
/// The most teeth a gear can have.
pub const MAX_TEETH: u32 = 256;
/// The most steps a flight of stairs can have.
pub const MAX_STEPS: u32 = 256;
/// The most rings of hexagons round the middle one of a hex grid.
pub const MAX_HEX_RINGS: u32 = 64;

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

/// A ball made of even triangles (a subdivided icosahedron), resting on
/// the ground. Unlike the [sphere](SphereParams), it has no poles where
/// thin triangles bunch up, so it bends and subdivides evenly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IcosphereParams {
    /// Radius.
    pub radius: f32,
    /// How many times each triangle is cut into four (0: the 20-sided
    /// icosahedron).
    pub subdivisions: u32,
}

impl Default for IcosphereParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            subdivisions: 2,
        }
    }
}

/// A ring (a doughnut) lying flat on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorusParams {
    /// From the middle to the centre of the ring's tube.
    pub radius: f32,
    /// The tube's own radius.
    pub tube_radius: f32,
    /// How many pieces around the ring.
    pub sides: u32,
    /// How many pieces around the tube.
    pub tube_sides: u32,
}

impl Default for TorusParams {
    fn default() -> Self {
        Self {
            radius: 40.0,
            tube_radius: 10.0,
            sides: 32,
            tube_sides: 12,
        }
    }
}

/// An upright cylinder with round (half-ball) ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapsuleParams {
    /// Radius.
    pub radius: f32,
    /// Height from end to end, round ends included.
    pub height: f32,
    /// How many pieces around.
    pub sides: u32,
    /// How many rings each round end has, from its rim to its tip.
    pub cap_rings: u32,
    /// How many pieces the straight middle is cut into.
    pub height_segments: u32,
}

impl Default for CapsuleParams {
    fn default() -> Self {
        Self {
            radius: 25.0,
            height: 100.0,
            sides: 16,
            cap_rings: 4,
            height_segments: 1,
        }
    }
}

/// Half a ball, standing on its flat side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeParams {
    /// Radius.
    pub radius: f32,
    /// How many pieces around.
    pub sides: u32,
    /// How many rings from the rim to the top.
    pub rings: u32,
    /// Whether the flat bottom is closed.
    pub base: bool,
}

impl Default for DomeParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            sides: 24,
            rings: 6,
            base: true,
        }
    }
}

/// A bent piece of pipe lying on the ground, turning round the up axis
/// from +X towards +Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElbowParams {
    /// How far the pipe's middle is from the point it bends round.
    pub bend_radius: f32,
    /// The pipe's radius.
    pub pipe_radius: f32,
    /// The pipe's hole radius: zero for a solid bend with closed ends.
    pub inner_radius: f32,
    /// How far it bends, in degrees.
    pub angle: f32,
    /// How many pieces round the pipe.
    pub sides: u32,
    /// How many pieces along the bend.
    pub segments: u32,
}

impl Default for ElbowParams {
    fn default() -> Self {
        Self {
            bend_radius: 50.0,
            pipe_radius: 15.0,
            inner_radius: 0.0,
            angle: 90.0,
            sides: 16,
            segments: 8,
        }
    }
}

/// A slice of a round slab (a pie piece), standing on the ground, from
/// +X round towards +Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectorParams {
    /// Outer radius.
    pub radius: f32,
    /// Radius of the cut-out middle: zero for a point (a pie piece), more
    /// for a slice of a ring.
    pub inner_radius: f32,
    /// How much of a circle, in degrees.
    pub angle: f32,
    /// Height (thickness).
    pub height: f32,
    /// How many pieces along the curve.
    pub sides: u32,
}

impl Default for SectorParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            inner_radius: 0.0,
            angle: 90.0,
            height: 10.0,
            sides: 16,
        }
    }
}

/// A flat gear (cog) standing on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GearParams {
    /// Radius at the tips of the teeth.
    pub radius: f32,
    /// How deep each tooth is, from its tip to the gap between teeth.
    pub tooth_depth: f32,
    /// How many teeth.
    pub teeth: u32,
    /// Height (thickness).
    pub height: f32,
    /// Radius of the hole for the axle: zero for none.
    pub hole_radius: f32,
}

impl Default for GearParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            tooth_depth: 8.0,
            teeth: 12,
            height: 10.0,
            hole_radius: 10.0,
        }
    }
}

/// A flat grid of hexagons (a honeycomb) lying on the ground: one in the
/// middle and rings of them round it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexGridParams {
    /// Each hexagon's size, from its middle to a corner.
    pub size: f32,
    /// How many rings of hexagons round the middle one.
    pub rings: u32,
}

impl Default for HexGridParams {
    fn default() -> Self {
        Self {
            size: 10.0,
            rings: 3,
        }
    }
}

/// A ball made of hexagons and 12 pentagons (like a football), resting on
/// the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexSphereParams {
    /// Radius.
    pub radius: f32,
    /// How finely it is cut: more gives more, smaller hexagons.
    pub subdivisions: u32,
}

impl Default for HexSphereParams {
    fn default() -> Self {
        Self {
            radius: 50.0,
            subdivisions: 2,
        }
    }
}

/// A solid flight of straight stairs, rising towards +Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairsParams {
    /// Width (X).
    pub width: f32,
    /// Height of the top step (Z).
    pub height: f32,
    /// Depth from the first step to the back of the last (Y).
    pub depth: f32,
    /// How many steps.
    pub steps: u32,
}

impl Default for StairsParams {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
            depth: 150.0,
            steps: 6,
        }
    }
}

/// An arch standing on the ground, like a doorway: two straight legs and a
/// half-round top, facing ±Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArchParams {
    /// The width of the opening.
    pub width: f32,
    /// How high the legs go before the round top starts.
    pub leg_height: f32,
    /// How thick the arch is round the opening.
    pub thickness: f32,
    /// How deep it is (Y).
    pub depth: f32,
    /// How many pieces the round top is cut into.
    pub segments: u32,
}

impl Default for ArchParams {
    fn default() -> Self {
        Self {
            width: 100.0,
            leg_height: 100.0,
            thickness: 20.0,
            depth: 30.0,
            segments: 12,
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
    /// A ball of even triangles.
    Icosphere,
    /// A ring (doughnut).
    Torus,
    /// A cylinder with round ends.
    Capsule,
    /// Half a ball.
    Dome,
    /// A bent piece of pipe.
    Elbow,
    /// A pie slice.
    Sector,
    /// A gear.
    Gear,
    /// A flat grid of hexagons.
    HexGrid,
    /// A ball of hexagons.
    HexSphere,
    /// A flight of stairs.
    Stairs,
    /// An arch.
    Arch,
}

impl PrimitiveKind {
    /// Every kind, in the order the Create menu lists them.
    pub const ALL: [PrimitiveKind; 18] = [
        Self::Cube,
        Self::Plane,
        Self::Cylinder,
        Self::Sphere,
        Self::Disc,
        Self::Tube,
        Self::Cone,
        Self::Icosphere,
        Self::Torus,
        Self::Capsule,
        Self::Dome,
        Self::Elbow,
        Self::Sector,
        Self::Gear,
        Self::HexGrid,
        Self::HexSphere,
        Self::Stairs,
        Self::Arch,
    ];

    /// Whether the oldest file versions can store this kind's settings
    /// (the first seven). Newer kinds are stored with the development
    /// file format (decision 0129).
    pub const fn in_first_files(self) -> bool {
        matches!(
            self,
            Self::Cube
                | Self::Plane
                | Self::Cylinder
                | Self::Sphere
                | Self::Disc
                | Self::Tube
                | Self::Cone
        )
    }

    /// The kind's name, as used for new objects ("Cube"), in the language
    /// in use.
    pub fn name(self) -> &'static str {
        tr(match self {
            Self::Cube => text!("Cube"),
            Self::Plane => text!("Plane"),
            Self::Cylinder => text!("Cylinder"),
            Self::Sphere => text!("Sphere"),
            Self::Disc => text!("Disc"),
            Self::Tube => text!("Tube"),
            Self::Cone => text!("Cone"),
            Self::Icosphere => text!("Icosphere"),
            Self::Torus => text!("Torus"),
            Self::Capsule => text!("Capsule"),
            Self::Dome => text!("Dome"),
            Self::Elbow => text!("Pipe elbow"),
            Self::Sector => text!("Pie sector"),
            Self::Gear => text!("Gear"),
            Self::HexGrid => text!("Hex grid"),
            Self::HexSphere => text!("Hex sphere"),
            Self::Stairs => text!("Stairs"),
            Self::Arch => text!("Arch"),
        })
    }

    /// A plain-language description, for menus and tooltips, in the
    /// language in use.
    pub fn description(self) -> &'static str {
        tr(match self {
            Self::Cube => text!("A 1 m box standing on the ground"),
            Self::Plane => text!("A flat 1 m square lying on the ground"),
            Self::Cylinder => text!("An upright cylinder, 1 m across and 1 m tall"),
            Self::Sphere => text!("A ball 1 m across, resting on the ground"),
            Self::Disc => text!("A flat circle 1 m across, lying on the ground"),
            Self::Tube => text!("A pipe 1 m across and 1 m tall"),
            Self::Cone => text!("A cone 1 m across and 1 m tall"),
            Self::Icosphere => text!(
                "A ball 1 m across made of even triangles, with no poles: subdivides and bends evenly"
            ),
            Self::Torus => text!("A ring (doughnut) 1 m across, lying on the ground"),
            Self::Capsule => text!("A cylinder with round ends, 1 m tall"),
            Self::Dome => text!("Half a ball 1 m across, standing on its flat side"),
            Self::Elbow => text!("A piece of pipe bent through 90°, lying on the ground"),
            Self::Sector => text!("A pie slice of a round slab, a quarter of a circle"),
            Self::Gear => text!("A gear with 12 teeth and a hole for the axle"),
            Self::HexGrid => text!("A flat honeycomb of hexagons, lying on the ground"),
            Self::HexSphere => text!("A ball made of hexagons and 12 pentagons, like a football"),
            Self::Stairs => text!("A solid flight of 6 stairs, 1 m wide and 1 m high"),
            Self::Arch => text!("A doorway arch: two legs and a round top"),
        })
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
            Self::Icosphere => Primitive::Icosphere(IcosphereParams::default()),
            Self::Torus => Primitive::Torus(TorusParams::default()),
            Self::Capsule => Primitive::Capsule(CapsuleParams::default()),
            Self::Dome => Primitive::Dome(DomeParams::default()),
            Self::Elbow => Primitive::Elbow(ElbowParams::default()),
            Self::Sector => Primitive::Sector(SectorParams::default()),
            Self::Gear => Primitive::Gear(GearParams::default()),
            Self::HexGrid => Primitive::HexGrid(HexGridParams::default()),
            Self::HexSphere => Primitive::HexSphere(HexSphereParams::default()),
            Self::Stairs => Primitive::Stairs(StairsParams::default()),
            Self::Arch => Primitive::Arch(ArchParams::default()),
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
    /// See [`IcosphereParams`].
    Icosphere(IcosphereParams),
    /// See [`TorusParams`].
    Torus(TorusParams),
    /// See [`CapsuleParams`].
    Capsule(CapsuleParams),
    /// See [`DomeParams`].
    Dome(DomeParams),
    /// See [`ElbowParams`].
    Elbow(ElbowParams),
    /// See [`SectorParams`].
    Sector(SectorParams),
    /// See [`GearParams`].
    Gear(GearParams),
    /// See [`HexGridParams`].
    HexGrid(HexGridParams),
    /// See [`HexSphereParams`].
    HexSphere(HexSphereParams),
    /// See [`StairsParams`].
    Stairs(StairsParams),
    /// See [`ArchParams`].
    Arch(ArchParams),
}

impl Primitive {
    /// One of each kind, with standard settings, in the order the Create
    /// menu lists them.
    pub fn all() -> [Primitive; 18] {
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
            Self::Icosphere(_) => PrimitiveKind::Icosphere,
            Self::Torus(_) => PrimitiveKind::Torus,
            Self::Capsule(_) => PrimitiveKind::Capsule,
            Self::Dome(_) => PrimitiveKind::Dome,
            Self::Elbow(_) => PrimitiveKind::Elbow,
            Self::Sector(_) => PrimitiveKind::Sector,
            Self::Gear(_) => PrimitiveKind::Gear,
            Self::HexGrid(_) => PrimitiveKind::HexGrid,
            Self::HexSphere(_) => PrimitiveKind::HexSphere,
            Self::Stairs(_) => PrimitiveKind::Stairs,
            Self::Arch(_) => PrimitiveKind::Arch,
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
            Self::Icosphere(p) => 20 * 4usize.pow(p.subdivisions),
            // Its faces are the icosphere's points: 10 × 4ⁿ + 2.
            Self::HexSphere(p) => 10 * 4usize.pow(p.subdivisions) + 2,
            Self::Torus(p) => n(p.sides) * n(p.tube_sides),
            Self::Capsule(p) => n(p.sides) * (2 * n(p.cap_rings) + n(p.height_segments)),
            Self::Dome(p) => n(p.sides) * n(p.rings) + 1,
            Self::Elbow(p) => {
                let walls = if p.inner_radius > 0.0 { 2 } else { 1 };
                // A whole ring is built with at least 3 segments.
                let segments = if p.angle >= 360.0 {
                    n(p.segments).max(3)
                } else {
                    n(p.segments)
                };
                n(p.sides) * (walls * segments + 2)
            }
            // Arcs are cut into pieces of at most 120°, so a whole turn
            // has at least 3.
            Self::Sector(p) => 4 * n(p.sides).max(3) + 4,
            // 16 faces a tooth with a hole, 20 without.
            Self::Gear(p) => n(p.teeth) * 20 + 2,
            Self::HexGrid(p) => 3 * n(p.rings) * (n(p.rings) + 1) + 1,
            // Two sides of n(n+1)/2 quads each, and 4n walls.
            Self::Stairs(p) => n(p.steps) * (n(p.steps) + 5),
            Self::Arch(p) => 4 * (n(p.segments) + 2) + 2,
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

            Self::Icosphere(p) => Self::Icosphere(IcosphereParams {
                radius: size(p.radius, IcosphereParams::default().radius),
                subdivisions: p.subdivisions.min(MAX_SUBDIVISIONS),
            }),
            Self::HexSphere(p) => Self::HexSphere(HexSphereParams {
                radius: size(p.radius, HexSphereParams::default().radius),
                subdivisions: p.subdivisions.min(MAX_SUBDIVISIONS),
            }),
            Self::Torus(p) => {
                let d = TorusParams::default();
                let radius = size(p.radius, d.radius);
                // The tube can't be wider than the ring, or it would pass
                // through itself.
                let tube = size(p.tube_radius, d.tube_radius).min(radius);
                Self::Torus(TorusParams {
                    radius,
                    tube_radius: tube,
                    sides: sides(p.sides),
                    tube_sides: sides(p.tube_sides),
                })
            }
            Self::Capsule(p) => {
                let d = CapsuleParams::default();
                let radius = size(p.radius, d.radius);
                Self::Capsule(CapsuleParams {
                    radius,
                    // At least as tall as its two round ends.
                    height: size(p.height, d.height).max(2.0 * radius),
                    sides: sides(p.sides),
                    cap_rings: segments(p.cap_rings),
                    height_segments: segments(p.height_segments),
                })
            }
            Self::Dome(p) => Self::Dome(DomeParams {
                radius: size(p.radius, DomeParams::default().radius),
                sides: sides(p.sides),
                rings: segments(p.rings),
                base: p.base,
            }),
            Self::Elbow(p) => {
                let d = ElbowParams::default();
                let bend = size(p.bend_radius, d.bend_radius);
                // The pipe can't be wider than its bend.
                let pipe = size(p.pipe_radius, d.pipe_radius).min(bend);
                Self::Elbow(ElbowParams {
                    bend_radius: bend,
                    pipe_radius: pipe,
                    inner_radius: hole(p.inner_radius, pipe),
                    angle: degrees(p.angle, d.angle),
                    sides: sides(p.sides),
                    segments: segments(p.segments),
                })
            }
            Self::Sector(p) => {
                let d = SectorParams::default();
                let radius = size(p.radius, d.radius);
                Self::Sector(SectorParams {
                    radius,
                    inner_radius: hole(p.inner_radius, radius),
                    angle: degrees(p.angle, d.angle),
                    height: size(p.height, d.height),
                    sides: segments(p.sides),
                })
            }
            Self::Gear(p) => {
                let d = GearParams::default();
                let radius = size(p.radius, d.radius);
                // Teeth no deeper than most of the radius, and the hole
                // inside the gaps between them.
                let depth = size(p.tooth_depth, d.tooth_depth).min(radius * 0.9);
                Self::Gear(GearParams {
                    radius,
                    tooth_depth: depth,
                    teeth: p.teeth.clamp(3, MAX_TEETH),
                    height: size(p.height, d.height),
                    hole_radius: hole(p.hole_radius, radius - depth),
                })
            }
            Self::HexGrid(p) => Self::HexGrid(HexGridParams {
                size: size(p.size, HexGridParams::default().size),
                rings: p.rings.min(MAX_HEX_RINGS),
            }),
            Self::Stairs(p) => {
                let d = StairsParams::default();
                Self::Stairs(StairsParams {
                    width: size(p.width, d.width),
                    height: size(p.height, d.height),
                    depth: size(p.depth, d.depth),
                    steps: p.steps.clamp(1, MAX_STEPS),
                })
            }
            Self::Arch(p) => {
                let d = ArchParams::default();
                Self::Arch(ArchParams {
                    width: size(p.width, d.width),
                    leg_height: radius_or_point(p.leg_height, d.leg_height),
                    thickness: size(p.thickness, d.thickness),
                    depth: size(p.depth, d.depth),
                    segments: p.segments.clamp(2, MAX_SEGMENTS),
                })
            }
        }
    }

    /// Builds the shape (with its settings brought within limits first),
    /// with ready-made UVs and seams (decision 0095).
    pub fn mesh(&self) -> Mesh {
        let mut mesh = self.shape();
        uvs::add_uvs(&mut mesh, self.kind());
        mesh
    }

    /// The shape alone, without UVs.
    fn shape(&self) -> Mesh {
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
            Self::Icosphere(p) => polyhedra::icosphere(&p),
            Self::HexSphere(p) => polyhedra::hex_sphere(&p),
            Self::HexGrid(p) => polyhedra::hex_grid(&p),
            Self::Torus(p) => lathe::torus(&p),
            Self::Capsule(p) => lathe::capsule(&p),
            Self::Dome(p) => lathe::dome(&p),
            Self::Elbow(p) => lathe::elbow(&p),
            Self::Sector(p) => prism::sector(&p),
            Self::Gear(p) => prism::gear(&p),
            Self::Stairs(p) => prism::stairs(&p),
            Self::Arch(p) => prism::arch(&p),
        }
    }

    /// The box the shape fits in, in its own space, worked out from the
    /// settings without building the mesh (the newer kinds, decision
    /// 0129, are built to measure).
    pub fn bounds(&self) -> Bounds {
        if !self.kind().in_first_files() {
            return self.shape().bounds().unwrap_or(Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            });
        }
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
            // Built to measure above.
            _ => (0.0, 0.0, 0.0),
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

/// An angle in degrees, from a hundredth of a degree to a whole turn;
/// `standard` if it isn't a number.
fn degrees(value: f32, standard: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.01, 360.0)
    } else {
        standard
    }
}

fn segments(value: u32) -> u32 {
    value.clamp(1, MAX_SEGMENTS)
}

fn sides(value: u32) -> u32 {
    value.clamp(MIN_SIDES, MAX_SIDES)
}
