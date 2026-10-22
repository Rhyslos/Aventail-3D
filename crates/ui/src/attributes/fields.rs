//! The settings each kind of primitive shows, in order, with their
//! limits: each one borrows the value it edits.

use mesh::primitives::{
    MAX_HEX_RINGS, MAX_SEGMENTS, MAX_SIDES, MAX_STEPS, MAX_SUBDIVISIONS, MAX_TEETH, MIN_SIDES,
    MIN_SIZE,
};
use scene::Primitive;

/// What a setting holds, with its limits.
pub(super) enum Value<'a> {
    /// A length in centimetres, from the minimum given.
    Length(&'a mut f32, f32),
    /// A count, between the limits given.
    Count(&'a mut u32, u32, u32),
    /// On or off.
    Toggle(&'a mut bool),
    /// An angle in degrees, between the limits given.
    Angle(&'a mut f32, f32, f32),
}

/// One row of shape settings.
pub(super) struct Field<'a> {
    pub(super) label: &'static str,
    pub(super) help: &'static str,
    pub(super) value: Value<'a>,
}

const fn field<'a>(label: &'static str, help: &'static str, value: Value<'a>) -> Field<'a> {
    Field { label, help, value }
}

/// The settings of a primitive, in the order the panel lists them, each
/// borrowing the value it edits.
pub(super) fn fields(primitive: &mut Primitive) -> Vec<Field<'_>> {
    use Value::{Angle, Count, Length, Toggle};
    let segments = |v| Count(v, 1, MAX_SEGMENTS);
    let sides = |v| Count(v, MIN_SIDES, MAX_SIDES);
    match primitive {
        Primitive::Cube(p) => {
            let [sx, sy, sz] = &mut p.segments;
            vec![
                field("Width", "Size along X", Length(&mut p.size.x, MIN_SIZE)),
                field("Depth", "Size along Y", Length(&mut p.size.y, MIN_SIZE)),
                field(
                    "Height",
                    "Size along Z (up)",
                    Length(&mut p.size.z, MIN_SIZE),
                ),
                field(
                    "Segments X",
                    "How many pieces each side is cut into along X",
                    segments(sx),
                ),
                field(
                    "Segments Y",
                    "How many pieces each side is cut into along Y",
                    segments(sy),
                ),
                field(
                    "Segments Z",
                    "How many pieces each side is cut into along Z",
                    segments(sz),
                ),
            ]
        }
        Primitive::Plane(p) => {
            let [sx, sy] = &mut p.segments;
            vec![
                field("Width", "Size along X", Length(&mut p.width, MIN_SIZE)),
                field("Depth", "Size along Y", Length(&mut p.depth, MIN_SIZE)),
                field(
                    "Segments X",
                    "How many pieces it is cut into along X",
                    segments(sx),
                ),
                field(
                    "Segments Y",
                    "How many pieces it is cut into along Y",
                    segments(sy),
                ),
            ]
        }
        Primitive::Cylinder(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Height",
                "Size along Z (up)",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many flat sides make up the round side",
                sides(&mut p.sides),
            ),
            field(
                "Height segments",
                "How many pieces the side is cut into from bottom to top",
                segments(&mut p.height_segments),
            ),
            field(
                "Closed ends",
                "Whether the ends are closed",
                Toggle(&mut p.caps),
            ),
            field(
                "Cap segments",
                "How many rings each closed end is cut into, from the rim inwards",
                segments(&mut p.cap_segments),
            ),
            field(
                "Centre point",
                "On: the ends meet in a point in the middle, with triangles around it. \
                 Off: the middle of each end is one flat face with many corners (an n-gon).",
                Toggle(&mut p.cap_centre),
            ),
        ],
        Primitive::Cone(p) => vec![
            field(
                "Bottom radius",
                "Half the width at the bottom",
                Length(&mut p.bottom_radius, 0.0),
            ),
            field(
                "Top radius",
                "Half the width at the top: zero for a point, more to cut it off flat",
                Length(&mut p.top_radius, 0.0),
            ),
            field(
                "Height",
                "Size along Z (up)",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many flat sides make up the round side",
                sides(&mut p.sides),
            ),
            field(
                "Height segments",
                "How many pieces the side is cut into from bottom to top",
                segments(&mut p.height_segments),
            ),
            field(
                "Closed ends",
                "Whether the ends are closed",
                Toggle(&mut p.caps),
            ),
            field(
                "Cap segments",
                "How many rings each closed end is cut into, from the rim inwards",
                segments(&mut p.cap_segments),
            ),
            field(
                "Centre point",
                "On: the ends meet in a point in the middle, with triangles around it. \
                 Off: the middle of each end is one flat face with many corners (an n-gon).",
                Toggle(&mut p.cap_centre),
            ),
        ],
        Primitive::Sphere(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Sides",
                "How many pieces around (like lines of longitude)",
                sides(&mut p.sides),
            ),
            field(
                "Rings",
                "How many pieces from top to bottom (like lines of latitude)",
                Count(&mut p.rings, 2, MAX_SEGMENTS),
            ),
        ],
        Primitive::Disc(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Hole radius",
                "Half the width of a hole in the middle: zero for none",
                Length(&mut p.inner_radius, 0.0),
            ),
            field(
                "Sides",
                "How many straight pieces make up the circle",
                sides(&mut p.sides),
            ),
            field(
                "Ring segments",
                "How many rings it is cut into, from the rim inwards",
                segments(&mut p.ring_segments),
            ),
            field(
                "Centre point",
                "Without a hole: on, the middle is a point with triangles around it; \
                 off, one flat face with many corners (an n-gon).",
                Toggle(&mut p.centre_point),
            ),
        ],
        Primitive::Tube(p) => vec![
            field(
                "Outer radius",
                "Half the width of the outside",
                Length(&mut p.outer_radius, MIN_SIZE),
            ),
            field(
                "Inner radius",
                "Half the width of the hole",
                Length(&mut p.inner_radius, MIN_SIZE),
            ),
            field(
                "Height",
                "Size along Z (up)",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many flat sides make up each round side",
                sides(&mut p.sides),
            ),
            field(
                "Height segments",
                "How many pieces the sides are cut into from bottom to top",
                segments(&mut p.height_segments),
            ),
        ],
        Primitive::Icosphere(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Subdivisions",
                "How many times each triangle is cut into four: 0 is a 20-sided ball, each \
                 step four times the triangles",
                Count(&mut p.subdivisions, 0, MAX_SUBDIVISIONS),
            ),
        ],
        Primitive::HexSphere(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Subdivisions",
                "How finely it is cut: each step gives about four times the hexagons",
                Count(&mut p.subdivisions, 0, MAX_SUBDIVISIONS),
            ),
        ],
        Primitive::HexGrid(p) => vec![
            field(
                "Hexagon size",
                "Each hexagon's size, from its middle to a corner",
                Length(&mut p.size, MIN_SIZE),
            ),
            field(
                "Rings",
                "How many rings of hexagons round the middle one",
                Count(&mut p.rings, 0, MAX_HEX_RINGS),
            ),
        ],
        Primitive::Torus(p) => vec![
            field(
                "Radius",
                "From the middle to the centre of the ring's tube",
                Length(&mut p.radius, MIN_SIZE),
            ),
            field(
                "Tube radius",
                "How thick the ring is: half the tube's width",
                Length(&mut p.tube_radius, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many pieces around the ring",
                sides(&mut p.sides),
            ),
            field(
                "Tube sides",
                "How many pieces around the tube",
                sides(&mut p.tube_sides),
            ),
        ],
        Primitive::Capsule(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field(
                "Height",
                "From end to end, the round ends included",
                Length(&mut p.height, MIN_SIZE),
            ),
            field("Sides", "How many pieces around", sides(&mut p.sides)),
            field(
                "Cap rings",
                "How many rings each round end has, from its rim to its tip",
                segments(&mut p.cap_rings),
            ),
            field(
                "Height segments",
                "How many pieces the straight middle is cut into",
                segments(&mut p.height_segments),
            ),
        ],
        Primitive::Dome(p) => vec![
            field("Radius", "Half the width", Length(&mut p.radius, MIN_SIZE)),
            field("Sides", "How many pieces around", sides(&mut p.sides)),
            field(
                "Rings",
                "How many rings from the rim to the top",
                segments(&mut p.rings),
            ),
            field(
                "Closed base",
                "Whether the flat bottom is closed",
                Toggle(&mut p.base),
            ),
        ],
        Primitive::Elbow(p) => vec![
            field(
                "Bend radius",
                "How far the pipe's middle is from the point it bends round",
                Length(&mut p.bend_radius, MIN_SIZE),
            ),
            field(
                "Pipe radius",
                "Half the pipe's width",
                Length(&mut p.pipe_radius, MIN_SIZE),
            ),
            field(
                "Hole radius",
                "Half the width of the hole through the pipe: zero for a solid bend",
                Length(&mut p.inner_radius, 0.0),
            ),
            field(
                "Angle",
                "How far it bends: 90° for a corner, 360° for a whole ring",
                Angle(&mut p.angle, 1.0, 360.0),
            ),
            field(
                "Sides",
                "How many pieces around the pipe",
                sides(&mut p.sides),
            ),
            field(
                "Segments",
                "How many pieces along the bend",
                segments(&mut p.segments),
            ),
        ],
        Primitive::Sector(p) => vec![
            field(
                "Radius",
                "Half the width of the whole circle",
                Length(&mut p.radius, MIN_SIZE),
            ),
            field(
                "Inner radius",
                "Cut out the middle: zero for a pie slice, more for a slice of a ring",
                Length(&mut p.inner_radius, 0.0),
            ),
            field(
                "Angle",
                "How much of a circle: 90° for a quarter",
                Angle(&mut p.angle, 1.0, 360.0),
            ),
            field(
                "Height",
                "How thick the slab is",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Sides",
                "How many pieces along the curve",
                segments(&mut p.sides),
            ),
        ],
        Primitive::Gear(p) => vec![
            field(
                "Radius",
                "Half the width, at the tips of the teeth",
                Length(&mut p.radius, MIN_SIZE),
            ),
            field(
                "Tooth depth",
                "How deep each tooth is, from its tip to the gap between teeth",
                Length(&mut p.tooth_depth, MIN_SIZE),
            ),
            field("Teeth", "How many teeth", Count(&mut p.teeth, 3, MAX_TEETH)),
            field(
                "Height",
                "How thick the gear is",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Hole radius",
                "Half the width of the hole for the axle: zero for none",
                Length(&mut p.hole_radius, 0.0),
            ),
        ],
        Primitive::Stairs(p) => vec![
            field("Width", "Size along X", Length(&mut p.width, MIN_SIZE)),
            field(
                "Height",
                "How high the top step is",
                Length(&mut p.height, MIN_SIZE),
            ),
            field(
                "Depth",
                "From the front of the first step to the back of the last (along Y)",
                Length(&mut p.depth, MIN_SIZE),
            ),
            field("Steps", "How many steps", Count(&mut p.steps, 1, MAX_STEPS)),
        ],
        Primitive::Arch(p) => vec![
            field(
                "Opening width",
                "How wide the opening is",
                Length(&mut p.width, MIN_SIZE),
            ),
            field(
                "Leg height",
                "How high the straight sides go before the round top starts: zero for a half ring",
                Length(&mut p.leg_height, 0.0),
            ),
            field(
                "Thickness",
                "How thick the arch is round the opening",
                Length(&mut p.thickness, MIN_SIZE),
            ),
            field(
                "Depth",
                "How deep it is (along Y)",
                Length(&mut p.depth, MIN_SIZE),
            ),
            field(
                "Segments",
                "How many pieces the round top is cut into",
                Count(&mut p.segments, 2, MAX_SEGMENTS),
            ),
        ],
    }
}
