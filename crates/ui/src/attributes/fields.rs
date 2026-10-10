//! The settings each kind of primitive shows, in order, with their
//! limits: each one borrows the value it edits.

use mesh::primitives::{MAX_SEGMENTS, MAX_SIDES, MIN_SIDES, MIN_SIZE};
use scene::Primitive;

/// What a setting holds, with its limits.
pub(super) enum Value<'a> {
    /// A length in centimetres, from the minimum given.
    Length(&'a mut f32, f32),
    /// A count, between the limits given.
    Count(&'a mut u32, u32, u32),
    /// On or off.
    Toggle(&'a mut bool),
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
    use Value::{Count, Length, Toggle};
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
    }
}
