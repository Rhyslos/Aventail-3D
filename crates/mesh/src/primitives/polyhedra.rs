//! Polyhedra (decision 0129): the icosphere, the hex sphere and the hex
//! grid.
//!
//! - **Icosphere:** a 20-sided icosahedron with one point straight up and
//!   one straight down, each triangle cut into four as many times as asked,
//!   every point pushed out onto the ball. Its triangles are all nearly the
//!   same size, unlike a [sphere](super::SphereParams)'s, which bunch up at
//!   the poles.
//! - **Hex sphere:** the icosphere turned inside out (its *dual*): one face
//!   for each icosphere point and one point for each icosphere triangle.
//!   This gives 12 pentagons (at the icosahedron's 12 corners) and hexagons
//!   everywhere else, like a football.
//! - **Hex grid:** a flat honeycomb on the ground, facing up.
//!
//! The balls rest on the ground (centre at the height of their radius) and
//! are smooth all over: no edge is marked hard. The grid is flat, so none
//! of its edges are hard either. Points are always shared between the faces
//! that meet at them, and everything is built in a fixed order, so the same
//! settings always give the same mesh, ids included.

use super::{HexGridParams, HexSphereParams, IcosphereParams};
use crate::Mesh;
use crate::polygon::VertexId;
use glam::Vec3;
use std::collections::BTreeMap;
use std::f32::consts::TAU;

/// A ball of radius one round the origin, made of triangles: its points
/// (each one unit from the centre) and its triangles (as indices into the
/// points, counter-clockwise seen from outside).
struct UnitIcosphere {
    points: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
}

/// The 20-sided icosahedron, one point straight up (+Z) and one straight
/// down, the other ten in two rings of five between them.
fn icosahedron() -> UnitIcosphere {
    // The two rings sit at a height of 1/√5 above and below the middle,
    // which makes every edge the same length. The lower ring is turned a
    // tenth of a turn so its points sit between the upper ring's.
    let ring_height = 1.0 / 5.0f32.sqrt();
    let ring_radius = 2.0 / 5.0f32.sqrt();
    let upper_ring: Vec<Vec3> = (0..5)
        .map(|k| {
            let angle = TAU * k as f32 / 5.0;
            Vec3::new(
                ring_radius * angle.cos(),
                ring_radius * angle.sin(),
                ring_height,
            )
        })
        .collect();
    // Each lower point is exactly opposite an upper one (the lower ring's
    // point k, half a turn round from the upper ring's point k + 3), so
    // the ball is exactly the same on opposite sides, to the last digit.
    // That keeps the box round it centred exactly on the up axis, which
    // the ready-made UVs rely on to find the poles.
    let lower_ring = (0..5).map(|k| -upper_ring[(k + 3) % 5]);
    let mut points = vec![Vec3::Z];
    points.extend(upper_ring.iter().copied());
    points.extend(lower_ring);
    points.push(Vec3::NEG_Z);

    // Point 0 is the top, 1–5 the upper ring, 6–10 the lower ring and 11
    // the bottom.
    let (top, bottom) = (0u32, 11u32);
    let upper = |k: u32| 1 + k % 5;
    let lower = |k: u32| 6 + k % 5;
    let mut triangles = Vec::with_capacity(20);
    for k in 0..5 {
        triangles.push([top, upper(k), upper(k + 1)]);
    }
    for k in 0..5 {
        // Round the middle, a band of triangles pointing down and up.
        triangles.push([upper(k), lower(k), upper(k + 1)]);
        triangles.push([lower(k), lower(k + 1), upper(k + 1)]);
    }
    for k in 0..5 {
        triangles.push([bottom, lower(k + 1), lower(k)]);
    }
    UnitIcosphere { points, triangles }
}

/// The icosahedron with each triangle cut into four `subdivisions` times,
/// every new point pushed out onto the ball.
fn unit_icosphere(subdivisions: u32) -> UnitIcosphere {
    let mut ball = icosahedron();
    for _ in 0..subdivisions {
        ball = subdivide(&ball);
    }
    ball
}

/// Cuts each triangle into four: one point at the middle of each edge
/// (shared with the triangle on the other side of the edge), pushed out
/// onto the ball.
fn subdivide(ball: &UnitIcosphere) -> UnitIcosphere {
    let mut points = ball.points.clone();
    // Each edge's middle point, by the edge's two ends (lower index first),
    // so the two triangles using an edge share its middle.
    let mut middles: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let mut middle = |a: u32, b: u32, points: &mut Vec<Vec3>| -> u32 {
        let key = (a.min(b), a.max(b));
        *middles.entry(key).or_insert_with(|| {
            let p = (points[a as usize] + points[b as usize]).normalize();
            points.push(p);
            (points.len() - 1) as u32
        })
    };
    let mut triangles = Vec::with_capacity(ball.triangles.len() * 4);
    for &[a, b, c] in &ball.triangles {
        let ab = middle(a, b, &mut points);
        let bc = middle(b, c, &mut points);
        let ca = middle(c, a, &mut points);
        // Three corner triangles and one in the middle, all wound the same
        // way as the triangle they came from.
        triangles.push([a, ab, ca]);
        triangles.push([ab, b, bc]);
        triangles.push([ca, bc, c]);
        triangles.push([ab, bc, ca]);
    }
    UnitIcosphere { points, triangles }
}

/// A ball of even triangles, resting on the ground: 20 × 4ⁿ triangles for
/// `n` subdivisions.
pub(super) fn icosphere(p: &IcosphereParams) -> Mesh {
    let ball = unit_icosphere(p.subdivisions);
    let centre = Vec3::new(0.0, 0.0, p.radius);
    let mut mesh = Mesh::new();
    let ids: Vec<VertexId> = ball
        .points
        .iter()
        .map(|&unit| mesh.push_vertex(centre + unit * p.radius))
        .collect();
    for [a, b, c] in &ball.triangles {
        mesh.push_face(&[ids[*a as usize], ids[*b as usize], ids[*c as usize]]);
    }
    mesh
}

/// A ball of hexagons and 12 pentagons (a Goldberg polyhedron), resting
/// on the ground: the dual of the icosphere with the same subdivisions.
/// Each icosphere point becomes a face (a pentagon at the icosahedron's 12
/// corners, where five triangles meet; a hexagon everywhere else, where
/// six meet), and each icosphere triangle becomes a point, at the
/// triangle's middle pushed out onto the ball. That gives 10 × 4ⁿ + 2
/// faces and 20 × 4ⁿ points for `n` subdivisions; with none, it is a
/// dodecahedron (12 pentagons).
///
/// Because every point sits exactly on the ball, most hexagons bow very
/// slightly rather than being perfectly flat (the pentagons stay flat).
/// It is too little to see, and the faces shade smoothly anyway.
///
/// A pentagon sits at the very top and another at the very bottom, so the
/// ball rests flat on that bottom pentagon: its centre is a little lower
/// than its radius, at the height of the pentagon's corners above it.
pub(super) fn hex_sphere(p: &HexSphereParams) -> Mesh {
    let ball = unit_icosphere(p.subdivisions);
    // One point per triangle, at its middle, on a ball of radius one.
    let unit_points: Vec<Vec3> = ball
        .triangles
        .iter()
        .map(|[a, b, c]| {
            (ball.points[*a as usize] + ball.points[*b as usize] + ball.points[*c as usize])
                .normalize()
        })
        .collect();
    // Lifted so the lowest points (the bottom pentagon's corners) touch
    // the ground.
    let lowest = unit_points.iter().map(|u| u.z).fold(0.0f32, f32::min);
    let centre = Vec3::new(0.0, 0.0, -lowest * p.radius);
    let mut mesh = Mesh::new();
    let ids: Vec<VertexId> = unit_points
        .iter()
        .map(|&unit| mesh.push_vertex(centre + unit * p.radius))
        .collect();

    // For each icosphere point, the triangles round it. A triangle seen
    // from its point `v` as (v, x, y), counter-clockwise from outside, is
    // followed (going counter-clockwise round `v`) by the triangle that
    // starts (v, y, …). Kept as (x, y, triangle).
    let mut around: Vec<Vec<(u32, u32, usize)>> = vec![Vec::new(); ball.points.len()];
    for (t, &[a, b, c]) in ball.triangles.iter().enumerate() {
        around[a as usize].push((b, c, t));
        around[b as usize].push((c, a, t));
        around[c as usize].push((a, b, t));
    }
    for fan in &around {
        let Some(&(_, mut next, first)) = fan.first() else {
            continue;
        };
        // Walk round the point from its first triangle.
        let mut face = vec![ids[first]];
        while face.len() < fan.len() {
            let Some(&(_, y, t)) = fan.iter().find(|(x, _, _)| *x == next) else {
                break;
            };
            face.push(ids[t]);
            next = y;
        }
        mesh.push_face(&face);
    }
    mesh
}

/// Where the six corners of a hexagon lie, counter-clockwise from the one
/// on +X, in the grid's whole-number steps (half a hexagon's size across,
/// half its height up: see [`hex_grid`]).
const HEX_CORNERS: [(i32, i32); 6] = [(2, 0), (1, 1), (-1, 1), (-2, 0), (-1, -1), (1, -1)];

/// A flat honeycomb on the ground, facing up: one hexagon in the middle
/// and `rings` rings of hexagons round it, 3r(r + 1) + 1 in all, with
/// 6(r + 1)² points. Neighbouring hexagons share their edges and points,
/// so it is one connected surface.
///
/// The hexagons are *flat-topped*: each has a corner pointing along +X
/// (like the six-sided disc and the other round shapes, whose first point
/// is on +X) and flat sides facing ±Y. Columns of hexagons run along Y.
pub(super) fn hex_grid(p: &HexGridParams) -> Mesh {
    let rings = p.rings as i32;
    // Every corner lies on a lattice of half a hexagon's size across (X)
    // and half its height up (Y), so corners can be matched exactly by
    // whole numbers, without comparing distances.
    let step_x = p.size / 2.0;
    let step_y = p.size * 3.0f32.sqrt() / 2.0;
    let mut mesh = Mesh::new();
    let mut corners: BTreeMap<(i32, i32), VertexId> = BTreeMap::new();
    // Hexagons by their place in the grid: column `q` and the step `r`
    // along it (axial coordinates), within `rings` steps of the middle.
    for q in -rings..=rings {
        let first = (-rings).max(-q - rings);
        let last = rings.min(-q + rings);
        for r in first..=last {
            // The hexagon's middle on the lattice.
            let (cx, cy) = (3 * q, 2 * r + q);
            let face: Vec<VertexId> = HEX_CORNERS
                .iter()
                .map(|(dx, dy)| {
                    let key = (cx + dx, cy + dy);
                    *corners.entry(key).or_insert_with(|| {
                        mesh.push_vertex(Vec3::new(
                            key.0 as f32 * step_x,
                            key.1 as f32 * step_y,
                            0.0,
                        ))
                    })
                })
                .collect();
            mesh.push_face(&face);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{AttributeValues, UV};
    use crate::polygon::EdgeKey;
    use crate::primitives::{MAX_HEX_RINGS, MAX_SIZE, MAX_SUBDIVISIONS, MIN_SIZE, Primitive};
    use crate::shading::shaded_triangles;
    use glam::Vec2;
    use std::f32::consts::PI;

    fn ico(radius: f32, subdivisions: u32) -> Primitive {
        Primitive::Icosphere(IcosphereParams {
            radius,
            subdivisions,
        })
    }

    fn hex_ball(radius: f32, subdivisions: u32) -> Primitive {
        Primitive::HexSphere(HexSphereParams {
            radius,
            subdivisions,
        })
    }

    fn grid(size: f32, rings: u32) -> Primitive {
        Primitive::HexGrid(HexGridParams { size, rings })
    }

    /// How many faces use each edge.
    fn edge_use(mesh: &Mesh) -> BTreeMap<EdgeKey, usize> {
        let mut uses = BTreeMap::new();
        for (_, corners) in mesh.faces() {
            for i in 0..corners.len() {
                let key = EdgeKey::new(corners[i], corners[(i + 1) % corners.len()]);
                *uses.entry(key).or_insert(0) += 1;
            }
        }
        uses
    }

    fn is_closed(mesh: &Mesh) -> bool {
        edge_use(mesh).values().all(|n| *n == 2)
    }

    fn volume(mesh: &Mesh) -> f32 {
        shaded_triangles(mesh)
            .iter()
            .map(|t| t.positions[0].dot(t.positions[1].cross(t.positions[2])) / 6.0)
            .sum()
    }

    /// Points − edges + faces: 2 for anything shaped like a ball.
    fn euler(mesh: &Mesh) -> i64 {
        mesh.vertex_count() as i64 - edge_use(mesh).len() as i64 + mesh.face_count() as i64
    }

    /// Every point is a real number and no face repeats a corner.
    fn is_clean(mesh: &Mesh) -> bool {
        let finite = mesh.vertices().all(|(_, p)| p.is_finite());
        let distinct = mesh.faces().all(|(_, c)| {
            let mut sorted = c.to_vec();
            sorted.sort();
            sorted.dedup();
            sorted.len() == c.len() && c.len() >= 3
        });
        finite && distinct
    }

    /// Each face's UVs, from the corner layer `Primitive::mesh` adds.
    fn face_uvs(mesh: &Mesh) -> Vec<(crate::FaceId, Vec<Vec2>)> {
        let Some((_, _, AttributeValues::Vec2(uvs))) = mesh.attribute_values(UV) else {
            panic!("no UVs");
        };
        let (starts, _) = mesh.face_layout();
        mesh.faces()
            .map(|(face, _)| {
                let i = face.get() as usize;
                let range = starts[i] as usize..starts[i + 1] as usize;
                (face, uvs[range].to_vec())
            })
            .collect()
    }

    #[test]
    fn icosphere_counts_follow_the_subdivisions() {
        for n in 0..=4u32 {
            let mesh = ico(50.0, n).mesh();
            let four = 4usize.pow(n);
            assert_eq!(mesh.face_count(), 20 * four, "faces at {n}");
            assert_eq!(mesh.vertex_count(), 10 * four + 2, "points at {n}");
            assert_eq!(edge_use(&mesh).len(), 30 * four, "edges at {n}");
            assert!(mesh.faces().all(|(_, c)| c.len() == 3), "all triangles");
        }
    }

    #[test]
    fn hex_sphere_counts_follow_the_subdivisions() {
        for n in 0..=4u32 {
            let mesh = hex_ball(50.0, n).mesh();
            let four = 4usize.pow(n);
            assert_eq!(mesh.face_count(), 10 * four + 2, "faces at {n}");
            assert_eq!(mesh.vertex_count(), 20 * four, "points at {n}");
            assert_eq!(edge_use(&mesh).len(), 30 * four, "edges at {n}");
            let pentagons = mesh.faces().filter(|(_, c)| c.len() == 5).count();
            let hexagons = mesh.faces().filter(|(_, c)| c.len() == 6).count();
            assert_eq!(pentagons, 12, "always 12 pentagons");
            assert_eq!(hexagons, mesh.face_count() - 12, "the rest hexagons");
        }
    }

    #[test]
    fn with_no_subdivisions_the_hex_sphere_is_a_dodecahedron() {
        let mesh = hex_ball(50.0, 0).mesh();
        assert_eq!(mesh.face_count(), 12);
        assert_eq!(mesh.vertex_count(), 20);
        assert!(mesh.faces().all(|(_, c)| c.len() == 5));
        // A true dodecahedron: every face is flat.
        for (face, corners) in mesh.faces() {
            let n = mesh.face_normal(face);
            let first = mesh.position(corners[0]).unwrap();
            for v in corners {
                let off = (mesh.position(*v).unwrap() - first).dot(n);
                assert!(off.abs() < 1e-3, "flat pentagon, off by {off}");
            }
        }
    }

    #[test]
    fn hex_grid_counts_follow_the_rings() {
        for rings in [0u32, 1, 2, 3, 7, 20] {
            let mesh = grid(10.0, rings).mesh();
            let r = rings as usize;
            assert_eq!(
                mesh.face_count(),
                3 * r * (r + 1) + 1,
                "hexagons, {rings} rings"
            );
            assert_eq!(
                mesh.vertex_count(),
                6 * (r + 1) * (r + 1),
                "points, {rings} rings"
            );
            assert!(mesh.faces().all(|(_, c)| c.len() == 6), "all hexagons");
        }
        // With no rings, one hexagon on its own.
        assert_eq!(grid(10.0, 0).mesh().face_count(), 1);
    }

    #[test]
    fn the_balls_are_closed_outward_and_shaped_like_a_ball() {
        for n in 0..=4u32 {
            for shape in [ico(50.0, n), hex_ball(50.0, n)] {
                let mesh = shape.mesh();
                assert!(is_closed(&mesh), "{shape:?} has holes");
                assert_eq!(euler(&mesh), 2, "{shape:?}");
                assert!(volume(&mesh) > 0.0, "{shape:?} faces inward");
                // Every face looks away from the middle.
                let centre = mesh.bounds().unwrap().centre();
                for (face, corners) in mesh.faces() {
                    let middle = corners
                        .iter()
                        .map(|v| mesh.position(*v).unwrap())
                        .sum::<Vec3>()
                        / corners.len() as f32;
                    assert!(mesh.face_normal(face).dot(middle - centre) > 0.0);
                }
            }
        }
    }

    #[test]
    fn volumes_approach_a_true_ball() {
        let exact = 4.0 / 3.0 * PI * 50.0f32.powi(3);
        for make in [ico, hex_ball] {
            let mut previous = 0.0;
            for n in 0..=4u32 {
                let v = volume(&make(50.0, n).mesh());
                assert!(v < exact * 1.0001, "inside the true ball");
                assert!(v > previous, "closer with each subdivision");
                previous = v;
            }
            assert!(
                (previous / exact - 1.0).abs() < 0.005,
                "within half a percent"
            );
        }
    }

    #[test]
    fn every_ball_point_is_on_the_ball() {
        for radius in [MIN_SIZE, 1.0, 50.0, 1234.5] {
            for shape in [ico(radius, 3), hex_ball(radius, 3)] {
                let mesh = shape.mesh();
                // The middle of the box round it is the ball's centre.
                let centre = mesh.bounds().unwrap().centre();
                for (_, p) in mesh.vertices() {
                    let error = ((p - centre).length() - radius).abs();
                    assert!(error <= radius * 1e-5, "{shape:?}: {p} off by {error}");
                }
            }
        }
    }

    #[test]
    fn the_icosphere_has_a_point_at_each_pole() {
        let mesh = ico(50.0, 2).mesh();
        let has = |want: Vec3| mesh.vertices().any(|(_, p)| (p - want).length() < 1e-4);
        assert!(has(Vec3::new(0.0, 0.0, 100.0)), "top");
        assert!(has(Vec3::ZERO), "bottom, on the ground");
    }

    #[test]
    fn the_balls_are_smooth() {
        for shape in [ico(50.0, 2), hex_ball(50.0, 2)] {
            assert_eq!(shape.mesh().hard_edge_count(), 0, "{shape:?}");
        }
    }

    #[test]
    fn the_hex_grid_is_flat_facing_up_and_joined() {
        for rings in [0u32, 1, 3, 10] {
            let mesh = grid(10.0, rings).mesh();
            for (_, p) in mesh.vertices() {
                assert_eq!(p.z, 0.0);
            }
            for (face, _) in mesh.faces() {
                assert!((mesh.face_normal(face) - Vec3::Z).length() < 1e-5);
            }
            assert_eq!(mesh.hard_edge_count(), 0);
            // Inside, edges are shared by two hexagons; round the border,
            // used once. The border of r rings has 6(2r + 1) edges.
            let uses = edge_use(&mesh);
            assert!(uses.values().all(|n| *n == 1 || *n == 2));
            let border = uses.values().filter(|n| **n == 1).count();
            assert_eq!(border, 6 * (2 * rings as usize + 1), "{rings} rings");
            // One connected surface: points − edges + faces = 1 for a
            // single piece with no holes.
            assert_eq!(euler(&mesh), 1, "{rings} rings");
        }
    }

    #[test]
    fn hex_grid_hexagons_are_regular_and_the_right_size() {
        let size = 10.0;
        let mesh = grid(size, 2).mesh();
        for (_, corners) in mesh.faces() {
            let points: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
            let middle = points.iter().sum::<Vec3>() / 6.0;
            for (i, p) in points.iter().enumerate() {
                assert!(
                    ((*p - middle).length() - size).abs() < 1e-3,
                    "middle to corner"
                );
                let side = (points[(i + 1) % 6] - *p).length();
                assert!((side - size).abs() < 1e-3, "each side as long as the size");
            }
        }
        // The middle hexagon is flat-topped: a corner on +X.
        let single = grid(size, 0).mesh();
        assert!(
            single
                .vertices()
                .any(|(_, p)| (p - Vec3::new(size, 0.0, 0.0)).length() < 1e-5)
        );
    }

    #[test]
    fn uvs_lie_in_the_square_and_no_face_crosses_it() {
        let mut shapes = vec![grid(10.0, 0), grid(10.0, 3), grid(MIN_SIZE, 5)];
        for n in 0..=4 {
            shapes.extend([ico(50.0, n), hex_ball(50.0, n)]);
        }
        shapes.extend([
            ico(MIN_SIZE, 2),
            ico(MAX_SIZE, 2),
            hex_ball(MIN_SIZE, 2),
            hex_ball(MAX_SIZE, 2),
        ]);
        let mut bad = Vec::new();
        for shape in shapes {
            let mesh = shape.mesh();
            let centre = mesh.bounds().unwrap().centre();
            for (face, uvs) in face_uvs(&mesh) {
                for uv in &uvs {
                    assert!(
                        uv.is_finite() && uv.cmpge(Vec2::ZERO).all() && uv.cmple(Vec2::ONE).all(),
                        "{shape:?}: {uv} outside the square"
                    );
                }
                // A grid of one hexagon fills the square on its own.
                if mesh.face_count() == 1 {
                    continue;
                }
                let corners = mesh.face(face).unwrap();
                let middle = corners
                    .iter()
                    .map(|v| mesh.position(*v).unwrap())
                    .sum::<Vec3>()
                    / corners.len() as f32;
                let towards = (middle - centre).normalize();
                let min = uvs.iter().fold(Vec2::INFINITY, |a, b| a.min(*b));
                let max = uvs.iter().fold(Vec2::NEG_INFINITY, |a, b| a.max(*b));
                let span = max - min;
                if span.x >= 0.5 || span.y >= 0.5 {
                    bad.push(format!(
                        "{shape:?}: face {} ({} corners, towards {towards:.3}) spans {span}",
                        face.get(),
                        corners.len(),
                    ));
                }
            }
        }
        assert!(
            bad.is_empty(),
            "faces crossing the texture:\n{}",
            bad.join("\n")
        );
    }

    #[test]
    fn the_icosphere_is_exactly_centred_on_the_up_axis() {
        // The ready-made UVs find the poles by their distance from the
        // middle of the box round the ball, so it must be exact: a stray
        // hundred-thousandth of a centimetre once sent a pole triangle
        // across the whole texture.
        for n in 0..=5 {
            for radius in [MIN_SIZE, 50.0, MAX_SIZE] {
                let b = ico(radius, n).mesh().bounds().unwrap();
                assert_eq!(b.min.x, -b.max.x, "{n} {radius}");
                assert_eq!(b.min.y, -b.max.y, "{n} {radius}");
                assert_eq!(b.centre().x, 0.0);
                assert_eq!(b.centre().y, 0.0);
            }
        }
    }

    #[test]
    fn extreme_settings_build_valid_meshes() {
        let shapes = [
            ico(MIN_SIZE, 0),
            ico(MAX_SIZE, 0),
            ico(MIN_SIZE, MAX_SUBDIVISIONS),
            ico(f32::NAN, 99),
            hex_ball(MIN_SIZE, 0),
            hex_ball(MAX_SIZE, 0),
            hex_ball(MAX_SIZE, MAX_SUBDIVISIONS),
            hex_ball(-5.0, 99),
            grid(MIN_SIZE, 0),
            grid(MAX_SIZE, 0),
            grid(MIN_SIZE, MAX_HEX_RINGS),
            grid(f32::INFINITY, 1000),
        ];
        for shape in shapes {
            let mesh = shape.mesh();
            assert!(mesh.face_count() > 0, "{shape:?}");
            assert!(is_clean(&mesh), "{shape:?}: NaN or a repeated corner");
            assert_eq!(
                mesh.face_count(),
                shape.clamped().polygon_estimate(),
                "{shape:?}"
            );
            if matches!(shape, Primitive::HexGrid(_)) {
                assert!(edge_use(&mesh).values().all(|n| *n <= 2), "{shape:?}");
            } else {
                assert!(is_closed(&mesh), "{shape:?}");
                assert!(volume(&mesh) > 0.0, "{shape:?}");
            }
        }
        // The largest settings in numbers.
        let top = ico(50.0, MAX_SUBDIVISIONS).mesh();
        assert_eq!(top.face_count(), 81_920);
        let rings = MAX_HEX_RINGS as usize;
        let honeycomb = grid(10.0, MAX_HEX_RINGS).mesh();
        assert_eq!(honeycomb.vertex_count(), 6 * (rings + 1) * (rings + 1));
    }

    #[test]
    fn the_same_settings_build_the_same_mesh() {
        for shape in [ico(50.0, 3), hex_ball(50.0, 3), grid(10.0, 5)] {
            assert_eq!(shape.mesh(), shape.mesh(), "{shape:?}");
        }
    }
}
