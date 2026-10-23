//! Outlines pushed out into solids (decision 0129): the pie sector, the
//! gear, the stairs and the arch.
//!
//! Each shape is drawn as a flat [`Profile`]: an outline (one or more
//! closed loops of points) and how its flat end (cap) is cut into faces.
//! [`extrude`] pushes it out along the line square to it, building the
//! walls, the two caps, the hard edges and the UVs in one place. The shape
//! is then moved so it stands on the ground with its pivot at the bottom
//! centre.
//!
//! - **Caps without slivers:** a cap is only one big face (an n-gon) where
//!   that n-gon is round and plain. Concave or spiky caps (a pie slice,
//!   stairs, a gear's teeth) are cut into quads and triangles instead, so
//!   cutting them into triangles for drawing and baking never makes long
//!   thin slivers.
//! - **Hard edges:** the rims where the walls meet the caps, and the wall
//!   edges at sharp corners of the outline (turning more than
//!   [`SHARP_TURN_DEGREES`]). Points on a curve (an arc, a round hole) are
//!   never sharp, so curves shade smoothly.
//! - **UVs:** each cap is its own flat piece, seen from its own side; the
//!   walls are unrolled into strips (across = distance along the outline,
//!   up = depth), cut into separate pieces at the sharp corners so long
//!   thin strips don't take over the texture. A smooth curve stays one
//!   piece.

use super::uvs::{self, Chart};
use super::{ArchParams, GearParams, SectorParams, StairsParams};
use crate::Mesh;
use crate::polygon::VertexId;
use glam::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// An outline corner turning more than this many degrees is sharp: its
/// wall edge is hard and the wall's UV strip is cut there.
const SHARP_TURN_DEGREES: f32 = 30.0;

/// A point of a profile, in the profile's own flat plane.
#[derive(Debug, Clone, Copy)]
struct OutlinePoint {
    /// Where it lies in the plane.
    at: Vec2,
    /// Whether it lies on a smooth curve (an arc or a round hole): then it
    /// is never a sharp corner, however few pieces the curve is cut into.
    on_curve: bool,
}

impl OutlinePoint {
    /// A corner that may be sharp (decided by how much the outline turns).
    fn corner(x: f32, y: f32) -> Self {
        Self {
            at: Vec2::new(x, y),
            on_curve: false,
        }
    }

    /// A point on a smooth curve.
    fn curve(x: f32, y: f32) -> Self {
        Self {
            at: Vec2::new(x, y),
            on_curve: true,
        }
    }
}

/// A flat shape to push out into a solid: its points, the closed loops
/// round its edge that become walls, and the faces its cap is cut into.
/// Loops and faces refer to points by their place in `points`, and run
/// counter-clockwise seen from the side the plane faces (see [`Plane`]):
/// so a loop has the shape on its left, and a hole's loop runs clockwise.
#[derive(Debug, Clone, Default)]
struct Profile {
    points: Vec<OutlinePoint>,
    walls: Vec<Vec<usize>>,
    cap: Vec<Vec<usize>>,
}

impl Profile {
    /// One closed loop whose cap is a single face. Only for plain convex
    /// outlines (a round slab): anything spiky or dented gets cut up.
    fn single_face(outline: Vec<OutlinePoint>) -> Self {
        let all: Vec<usize> = (0..outline.len()).collect();
        Self {
            points: outline,
            walls: vec![all.clone()],
            cap: vec![all],
        }
    }

    /// A closed loop with a hole: `hole` has as many points as `outer`,
    /// each straight in from its partner. The cap is a ring of quads
    /// between them, and the hole's wall faces inward.
    fn holed(outer: Vec<OutlinePoint>, hole: Vec<OutlinePoint>) -> Self {
        let n = outer.len();
        let cap = (0..n)
            .map(|i| {
                let j = (i + 1) % n;
                vec![i, j, n + j, n + i]
            })
            .collect();
        Self {
            points: [outer, hole].concat(),
            // The hole's wall runs the other way, so it faces inward.
            walls: vec![(0..n).collect(), (n..2 * n).rev().collect()],
            cap,
        }
    }

    /// A closed loop whose cap is a ring of quads in to `inner` (as many
    /// points, each straight in from its partner, not part of any wall)
    /// and a fan of triangles from there to `middle`: for a spiky outline
    /// like a gear's, whose single face would cut into slivers.
    fn spoked(outer: Vec<OutlinePoint>, inner: Vec<OutlinePoint>, middle: OutlinePoint) -> Self {
        let n = outer.len();
        let mut profile = Self::holed(outer, inner);
        profile.walls.truncate(1);
        profile.points.push(middle);
        let centre = 2 * n;
        for i in 0..n {
            profile.cap.push(vec![n + i, n + (i + 1) % n, centre]);
        }
        profile
    }

    /// A band between two open lines, `outer` and `inner`, with the same
    /// number of points, running the same way, each point opposite its
    /// partner. Their ends are joined by straight walls; the cap is quads
    /// between them.
    fn band(outer: Vec<OutlinePoint>, inner: Vec<OutlinePoint>) -> Self {
        let n = outer.len();
        Self {
            points: [outer, inner].concat(),
            // Along the outer line, then back along the inner one.
            walls: vec![(0..n).chain((n..2 * n).rev()).collect()],
            cap: (0..n.saturating_sub(1))
                .map(|i| vec![i, i + 1, n + i + 1, n + i])
                .collect(),
        }
    }

    /// An open line `outer` whose ends are joined by straight walls through
    /// `point` (a pie slice): the cap is a fan of triangles from `point`.
    fn fan(outer: Vec<OutlinePoint>, point: OutlinePoint) -> Self {
        let n = outer.len();
        Self {
            points: [outer, vec![point]].concat(),
            walls: vec![(0..=n).collect()],
            cap: (0..n.saturating_sub(1))
                .map(|i| vec![i, i + 1, n])
                .collect(),
        }
    }
}

/// Where a profile's flat plane lies in space: its two directions. The
/// profile is pushed out along `across × up`, the side it faces.
#[derive(Debug, Clone, Copy)]
struct Plane {
    across: Vec3,
    up: Vec3,
}

impl Plane {
    /// The point at `at` in the plane, pushed out by `out`.
    fn point(self, at: Vec2, out: f32) -> Vec3 {
        self.across * at.x + self.up * at.y + self.across.cross(self.up) * out
    }
}

/// A face to add, with the UV place of each corner (in centimetres) and
/// the piece (chart) of the UV layout it belongs to.
struct PlannedFace {
    corners: Vec<VertexId>,
    uvs: Vec<Vec2>,
    chart: usize,
}

/// Pushes `profile` (in `plane`) out by `depth` into a closed solid with
/// outward-facing faces, hard rims and sharp corners, and UVs.
fn extrude(profile: &Profile, plane: Plane, depth: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let points = &profile.points;
    let bottom: Vec<VertexId> = points
        .iter()
        .map(|p| mesh.push_vertex(plane.point(p.at, 0.0)))
        .collect();
    let top: Vec<VertexId> = points
        .iter()
        .map(|p| mesh.push_vertex(plane.point(p.at, depth)))
        .collect();

    let mut faces = Vec::new();
    // The caps: the top one (pushed out) faces the way the plane does and
    // is seen as drawn; the bottom one faces back and is seen mirrored.
    for face in &profile.cap {
        faces.push(PlannedFace {
            corners: face.iter().map(|&i| top[i]).collect(),
            uvs: face.iter().map(|&i| points[i].at).collect(),
            chart: 0,
        });
        faces.push(PlannedFace {
            corners: face.iter().rev().map(|&i| bottom[i]).collect(),
            uvs: face
                .iter()
                .rev()
                .map(|&i| Vec2::new(-points[i].at.x, points[i].at.y))
                .collect(),
            chart: 1,
        });
    }
    // The walls, each a strip unrolled from its first sharp corner and cut
    // into a new piece at every sharp corner after it.
    let mut next_chart = 2;
    let mut sharp_corners = Vec::new();
    for wall in &profile.walls {
        let n = wall.len();
        let sharp: Vec<bool> = (0..n)
            .map(|k| {
                let p = |i: usize| points[wall[i % n]];
                is_sharp(p(k + n - 1).at, p(k), p(k + 1).at)
            })
            .collect();
        let start = sharp.iter().position(|s| *s).unwrap_or(0);
        let mut along = 0.0;
        let mut chart = next_chart;
        for step in 0..n {
            let k = (start + step) % n;
            let (a, b) = (wall[k], wall[(k + 1) % n]);
            if sharp[k] {
                sharp_corners.push(a);
                if step > 0 {
                    chart += 1;
                    along = 0.0;
                }
            }
            let length = points[a].at.distance(points[b].at);
            faces.push(PlannedFace {
                corners: vec![bottom[a], bottom[b], top[b], top[a]],
                uvs: vec![
                    Vec2::new(along, 0.0),
                    Vec2::new(along + length, 0.0),
                    Vec2::new(along + length, depth),
                    Vec2::new(along, depth),
                ],
                chart,
            });
            along += length;
            // The rims, where the wall meets the caps.
            mesh.set_hard(bottom[a], bottom[b], true);
            mesh.set_hard(top[a], top[b], true);
        }
        next_chart = chart + 1;
    }
    for i in sharp_corners {
        mesh.set_hard(bottom[i], top[i], true);
    }

    let mut charts: Vec<Chart> = (0..next_chart).map(|_| Chart::default()).collect();
    for face in faces {
        let Ok(id) = mesh.add_face(&face.corners) else {
            continue;
        };
        let corners = uvs::corners_of(&mesh, id);
        if let Some(chart) = charts.get_mut(face.chart) {
            for ((index, _), uv) in corners.into_iter().zip(face.uvs) {
                chart.corners.push((index, uv));
            }
        }
    }
    charts.retain(|c| !c.corners.is_empty());
    uvs::set_charts(&mut mesh, &charts);
    mesh
}

/// Whether the outline turns sharply at `point` (between `before` and
/// `after`): never on a curve; otherwise when it turns more than
/// [`SHARP_TURN_DEGREES`].
fn is_sharp(before: Vec2, point: OutlinePoint, after: Vec2) -> bool {
    if point.on_curve {
        return false;
    }
    let incoming = (point.at - before).normalize_or_zero();
    let outgoing = (after - point.at).normalize_or_zero();
    // A zero-length side has no direction: treat its corner as sharp.
    incoming.dot(outgoing) < SHARP_TURN_DEGREES.to_radians().cos()
}

/// Moves `mesh` so it stands on the ground (lowest point at height zero)
/// with its bounding box centred on the pivot across the ground.
fn stand_on_pivot(mesh: &mut Mesh) {
    let Some(bounds) = mesh.bounds() else {
        return;
    };
    let centre = bounds.centre();
    let shift = Vec3::new(-centre.x, -centre.y, -bounds.min.z);
    let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    for (id, p) in points {
        mesh.set_position(id, p + shift);
    }
}

/// The points of an arc of `radius` from angle 0 counter-clockwise
/// through `angle` radians, cut into `pieces`: `pieces + 1` points, all on
/// the curve except the two ends, which are corners where the arc meets
/// straight walls.
fn open_arc(radius: f32, angle: f32, pieces: u32) -> Vec<OutlinePoint> {
    (0..=pieces)
        .map(|i| {
            let a = angle * i as f32 / pieces as f32;
            let (x, y) = (radius * a.cos(), radius * a.sin());
            if i == 0 || i == pieces {
                OutlinePoint::corner(x, y)
            } else {
                OutlinePoint::curve(x, y)
            }
        })
        .collect()
}

/// The points of a whole circle of `radius`, cut into `pieces`, starting
/// on +X: all on the curve.
fn circle(radius: f32, pieces: u32) -> Vec<OutlinePoint> {
    (0..pieces)
        .map(|i| {
            let a = TAU * i as f32 / pieces as f32;
            OutlinePoint::curve(radius * a.cos(), radius * a.sin())
        })
        .collect()
}

/// The widest a single piece of a sector's arc may be, in degrees. A
/// piece spanning half a turn or more would fold the slice inside out, so
/// wide sectors with few sides get extra pieces: at most three for a whole
/// turn.
const WIDEST_ARC_PIECE_DEGREES: f32 = 120.0;

/// The shortest a piece of a sector's outer arc may be, in centimetres:
/// a very thin sector with many sides gets fewer, so no edge is so short
/// that its two ends count as the same spot.
const SHORTEST_ARC_PIECE: f32 = 1e-3;

/// How many pieces a sector's arc of `radius` through `degrees` is cut
/// into: `sides`, but more if each piece would be wider than
/// [`WIDEST_ARC_PIECE_DEGREES`] and fewer if shorter than
/// [`SHORTEST_ARC_PIECE`].
fn arc_pieces(sides: u32, degrees: f32, radius: f32) -> u32 {
    let fewest = (degrees / WIDEST_ARC_PIECE_DEGREES).ceil().max(1.0) as u32;
    let arc_length = radius * degrees.to_radians();
    let most = (arc_length / SHORTEST_ARC_PIECE).floor().max(1.0) as u32;
    sides.min(most).max(fewest)
}

/// A slice of a round slab: from +X counter-clockwise through `angle`
/// degrees, `sides` pieces along the arc (fewer or more at the extremes,
/// see [`arc_pieces`]). With no inner radius it is a pie slice meeting at
/// a point, its ends fans of triangles from there; with one, a slice of a
/// ring. A whole turn gives a full disc (each end one n-gon) or ring slab
/// with no straight walls.
pub(super) fn sector(p: &SectorParams) -> Mesh {
    let flat = Plane {
        across: Vec3::X,
        up: Vec3::Y,
    };
    let pieces = arc_pieces(p.sides, p.angle, p.radius);
    let profile = if p.angle >= 360.0 {
        if p.inner_radius > 0.0 {
            Profile::holed(circle(p.radius, pieces), circle(p.inner_radius, pieces))
        } else {
            Profile::single_face(circle(p.radius, pieces))
        }
    } else {
        let angle = p.angle.to_radians();
        let outer = open_arc(p.radius, angle, pieces);
        if p.inner_radius > 0.0 {
            Profile::band(outer, open_arc(p.inner_radius, angle, pieces))
        } else {
            Profile::fan(outer, OutlinePoint::corner(0.0, 0.0))
        }
    };
    let mut mesh = extrude(&profile, flat, p.height);
    stand_on_pivot(&mut mesh);
    mesh
}

/// How wide each tooth's base is, as a share of the distance from one
/// tooth to the next: the rest is the gap.
const TOOTH_BASE_SHARE: f32 = 0.5;

/// How wide a tooth's tip is against its base (measured along the
/// circle), so the teeth taper.
const TOOTH_TIP_SHARE: f32 = 0.6;

/// A flat gear: `teeth` tapering teeth with tips at `radius` and gaps
/// `tooth_depth` lower, 4 points per tooth. Its top and bottom are rings
/// of quads from the teeth inward, with points straight in from the
/// outline's: out to a round hole, or, with no hole, to a ring at half the
/// gaps' radius and a fan of triangles to the middle.
pub(super) fn gear(p: &GearParams) -> Mesh {
    let flat = Plane {
        across: Vec3::X,
        up: Vec3::Y,
    };
    let tip = p.radius;
    let gap = p.radius - p.tooth_depth;
    let pitch = TAU / p.teeth as f32;
    let half_base = pitch * TOOTH_BASE_SHARE / 2.0;
    // The tip's half-width as an angle, so its length along its circle is
    // a share of the base's.
    let half_tip = half_base * TOOTH_TIP_SHARE * gap / tip;
    // The angle and radius of each outline point, tooth by tooth.
    let polar: Vec<(f32, f32)> = (0..p.teeth)
        .flat_map(|k| {
            let middle = pitch * k as f32;
            [
                (middle - half_base, gap),
                (middle - half_tip, tip),
                (middle + half_tip, tip),
                (middle + half_base, gap),
            ]
        })
        .collect();
    let outer: Vec<OutlinePoint> = polar
        .iter()
        .map(|&(a, r)| OutlinePoint::corner(r * a.cos(), r * a.sin()))
        .collect();
    let ring = |radius: f32| -> Vec<OutlinePoint> {
        polar
            .iter()
            .map(|&(a, _)| OutlinePoint::curve(radius * a.cos(), radius * a.sin()))
            .collect()
    };
    let profile = if p.hole_radius > 0.0 {
        Profile::holed(outer, ring(p.hole_radius))
    } else {
        Profile::spoked(outer, ring(gap / 2.0), OutlinePoint::corner(0.0, 0.0))
    };
    let mut mesh = extrude(&profile, flat, p.height);
    stand_on_pivot(&mut mesh);
    mesh
}

/// A solid flight of `steps` even steps rising towards +Y: the side
/// profile (a staircase over a flat bottom and an upright back) pushed out
/// along X by the width. Each side is a grid of quads, one per step and
/// rise below it, so the bottom and the back are cut at every step too.
pub(super) fn stairs(p: &StairsParams) -> Mesh {
    // Seen from +X: across is +Y (the way the stairs rise), up is +Z.
    let side = Plane {
        across: Vec3::Y,
        up: Vec3::Z,
    };
    let n = p.steps as usize;
    let rise = p.height / n as f32;
    let tread = p.depth / n as f32;
    // The grid's points: column c (0 at the front to n at the back), row r
    // (0 on the ground), for each column up to the step on its left and
    // right: rows 0 to c + 1, but no higher than the top step.
    let mut points = Vec::new();
    let mut index = vec![Vec::new(); n + 1];
    for (c, column) in index.iter_mut().enumerate() {
        for r in 0..=(c + 1).min(n) {
            column.push(points.len());
            points.push(OutlinePoint::corner(tread * c as f32, rise * r as f32));
        }
    }
    let at = |c: usize, r: usize| index[c][r];
    // Step c (0 at the front) stands on cells in rows 0 to c.
    let mut cap = Vec::new();
    for c in 0..n {
        for r in 0..=c {
            cap.push(vec![at(c, r), at(c + 1, r), at(c + 1, r + 1), at(c, r + 1)]);
        }
    }
    // Round the edge, counter-clockwise: along the bottom, up the back,
    // then down the steps from the top one to the front of the first.
    let mut wall: Vec<usize> = (0..n).map(|c| at(c, 0)).collect();
    wall.extend((0..n).map(|r| at(n, r)));
    for c in (1..=n).rev() {
        // The top of step c - 1's tread at the back, then its front.
        wall.push(at(c, c));
        wall.push(at(c - 1, c));
    }
    let profile = Profile {
        points,
        walls: vec![wall],
        cap,
    };
    let mut mesh = extrude(&profile, side, p.width);
    stand_on_pivot(&mut mesh);
    mesh
}

/// A doorway arch facing ±Y: an opening `width` wide with straight legs
/// `leg_height` tall under a half-round top cut into `segments`, the band
/// round it `thickness` thick and `depth` deep.
pub(super) fn arch(p: &ArchParams) -> Mesh {
    // Seen from −Y (the front): across is +X, up is +Z.
    let front = Plane {
        across: Vec3::X,
        up: Vec3::Z,
    };
    let legs = p.leg_height > 0.0;
    // One edge of the band, from the right leg's foot, over the top, to
    // the left leg's foot.
    let edge = |radius: f32| -> Vec<OutlinePoint> {
        let mut line = Vec::new();
        if legs {
            line.push(OutlinePoint::corner(radius, 0.0));
        }
        for k in 0..=p.segments {
            let a = PI * k as f32 / p.segments as f32;
            let (x, z) = (radius * a.cos(), p.leg_height + radius * a.sin());
            // The round top meets the legs smoothly; with no legs, its
            // ends are the feet, which stand square on the ground.
            if !legs && (k == 0 || k == p.segments) {
                line.push(OutlinePoint::corner(x, z));
            } else {
                line.push(OutlinePoint::curve(x, z));
            }
        }
        if legs {
            line.push(OutlinePoint::corner(-radius, 0.0));
        }
        line
    };
    let opening = p.width / 2.0;
    let profile = Profile::band(edge(opening + p.thickness), edge(opening));
    let mut mesh = extrude(&profile, front, p.depth);
    stand_on_pivot(&mut mesh);
    mesh
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_SEGMENTS, MAX_STEPS, MAX_TEETH, Primitive};
    use super::*;
    use crate::attributes::{AttributeValues, UV};
    use crate::polygon::EdgeKey;
    use std::collections::{BTreeMap, BTreeSet};

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

    /// The signed volume inside a closed mesh (positive when its faces
    /// look outward). Fans from each face's first corner: right for any
    /// flat polygon, concave ones included.
    fn volume(mesh: &Mesh) -> f32 {
        let mut total = 0.0;
        for (_, corners) in mesh.faces() {
            let p: Vec<Vec3> = corners
                .iter()
                .map(|v| mesh.position(*v).expect("point"))
                .collect();
            for i in 1..p.len() - 1 {
                total += p[0].dot(p[i].cross(p[i + 1])) / 6.0;
            }
        }
        total
    }

    /// The checks every one of these shapes must pass: closed, facing
    /// outward, no repeated corners, no broken numbers, on the ground,
    /// centred, every corner with a UV inside the texture square.
    fn assert_sound(shape: Primitive) -> Mesh {
        let mesh = shape.mesh();
        assert!(mesh.face_count() > 0, "{shape:?} builds");
        assert!(
            edge_use(&mesh).values().all(|n| *n == 2),
            "{shape:?} is closed"
        );
        assert!(volume(&mesh) > 0.0, "{shape:?} faces outward");
        for (_, p) in mesh.vertices() {
            assert!(p.is_finite(), "{shape:?}: {p}");
        }
        for (_, corners) in mesh.faces() {
            let unique: BTreeSet<_> = corners.iter().collect();
            assert_eq!(unique.len(), corners.len(), "{shape:?}: repeated corner");
        }
        let b = mesh.bounds().expect("not empty");
        assert!(b.min.z.abs() < 1e-3, "{shape:?} stands on the ground");
        assert!(b.centre().x.abs() < 1e-2 && b.centre().y.abs() < 1e-2);
        let corner_count = mesh.face_layout().1.len();
        match mesh.attribute_values(UV) {
            Some((_, _, AttributeValues::Vec2(uvs))) => {
                assert_eq!(uvs.len(), corner_count);
                for uv in uvs.iter() {
                    assert!(uv.is_finite(), "{shape:?}: {uv}");
                    assert!(
                        uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0,
                        "{shape:?}: {uv} outside the texture square"
                    );
                }
            }
            other => panic!("{shape:?} has no UVs: {other:?}"),
        }
        assert_eq!(mesh, shape.mesh(), "{shape:?} is built the same each time");
        mesh
    }

    fn sector(params: SectorParams) -> Primitive {
        Primitive::Sector(params)
    }

    fn gear(params: GearParams) -> Primitive {
        Primitive::Gear(params)
    }

    fn stairs(params: StairsParams) -> Primitive {
        Primitive::Stairs(params)
    }

    fn arch(params: ArchParams) -> Primitive {
        Primitive::Arch(params)
    }

    #[test]
    fn a_pie_sector_has_the_volume_of_its_slice() {
        for (inner, angle) in [(0.0, 90.0), (0.0, 250.0), (20.0, 90.0), (20.0, 360.0)] {
            let p = SectorParams {
                inner_radius: inner,
                angle,
                sides: 128,
                ..SectorParams::default()
            };
            let mesh = assert_sound(sector(p));
            let exact = angle.to_radians() / 2.0 * (p.radius * p.radius - inner * inner) * p.height;
            assert!(
                (volume(&mesh) / exact - 1.0).abs() < 0.01,
                "{p:?}: {} against {exact}",
                volume(&mesh)
            );
        }
        // A whole pie is a round slab.
        let whole = SectorParams {
            angle: 360.0,
            sides: 128,
            ..SectorParams::default()
        };
        let exact = PI * 50.0 * 50.0 * 10.0;
        assert!((volume(&assert_sound(sector(whole))) / exact - 1.0).abs() < 0.01);
    }

    #[test]
    fn sector_face_counts_and_hard_edges() {
        let n = 16;
        // A pie piece: the arc wall, two straight walls, and each end a
        // fan of triangles from the point (no slivers when cut for drawing).
        let pie = assert_sound(sector(SectorParams::default()));
        assert_eq!(pie.face_count(), n + 2 + 2 * n);
        assert_eq!(pie.faces().filter(|(_, c)| c.len() == 3).count(), 2 * n);
        // Rims round both caps, and the three corners' upright edges.
        assert_eq!(pie.hard_edge_count(), 2 * (n + 2) + 3);
        // A ring slice: outer and inner arc walls, two ends, quad caps.
        let ring = assert_sound(sector(SectorParams {
            inner_radius: 20.0,
            ..SectorParams::default()
        }));
        assert_eq!(ring.face_count(), 2 * n + 2 + 2 * n);
        assert_eq!(ring.hard_edge_count(), 2 * (2 * n + 2) + 4);
        // A whole ring: no straight walls, no upright hard edges.
        let whole_ring = assert_sound(sector(SectorParams {
            inner_radius: 20.0,
            angle: 360.0,
            ..SectorParams::default()
        }));
        assert_eq!(whole_ring.face_count(), 4 * n);
        assert_eq!(whole_ring.hard_edge_count(), 4 * n);
        // A whole pie: a round slab with n-gon ends.
        let disc = assert_sound(sector(SectorParams {
            angle: 360.0,
            ..SectorParams::default()
        }));
        assert_eq!(disc.face_count(), n + 2);
        assert_eq!(disc.hard_edge_count(), 2 * n);
    }

    #[test]
    fn the_arc_wall_shades_smoothly() {
        let mesh = Primitive::Sector(SectorParams::default()).mesh();
        // Upright edges between two arc points are soft.
        let mut soft_uprights = 0;
        for key in mesh.edges().iter().copied() {
            let (a, b) = key.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            if (pa.truncate() - pb.truncate()).length() < 1e-4 && !mesh.is_hard(a, b) {
                soft_uprights += 1;
            }
        }
        assert_eq!(soft_uprights, 16 - 1, "the arc's inside points");
    }

    #[test]
    fn a_gear_has_four_points_per_tooth_and_a_hole() {
        let p = GearParams::default();
        let t = p.teeth as usize;
        let holed = assert_sound(gear(p));
        // Outline and hole walls, and two rings of quads.
        assert_eq!(holed.face_count(), 4 * t * 4);
        assert_eq!(holed.vertex_count(), 4 * 4 * t);
        // Rims round outline and hole on both sides, and every tooth
        // corner's upright edge; the round hole's wall is soft.
        assert_eq!(holed.hard_edge_count(), 4 * (4 * t) + 4 * t);
        let solid = assert_sound(gear(GearParams {
            hole_radius: 0.0,
            ..p
        }));
        // With no hole: each end a ring of quads in from the teeth and a
        // fan of triangles to the middle, so the spiky outline never cuts
        // into slivers.
        assert_eq!(solid.face_count(), 4 * t + 2 * (4 * t + 4 * t));
        assert_eq!(solid.faces().filter(|(_, c)| c.len() == 3).count(), 8 * t);
        assert_eq!(solid.faces().filter(|(_, c)| c.len() > 4).count(), 0);
    }

    #[test]
    fn a_gear_s_volume_lies_between_its_gap_and_tip_circles() {
        for (teeth, hole) in [(12, 10.0), (12, 0.0), (40, 0.0), (5, 20.0)] {
            let p = GearParams {
                teeth,
                hole_radius: hole,
                ..GearParams::default()
            };
            let mesh = assert_sound(gear(p));
            let gap = p.radius - p.tooth_depth;
            // The hole and the gaps are polygons inside their circles.
            let low = 0.9 * PI * gap * gap * p.height - PI * hole * hole * p.height;
            let high = PI * p.radius * p.radius * p.height - 0.9 * PI * hole * hole * p.height;
            let v = volume(&mesh);
            assert!(v > low && v < high, "{p:?}: {v} not in {low}..{high}");
        }
    }

    #[test]
    fn stairs_have_even_steps_and_their_volume() {
        for steps in [1u32, 2, 6, 30] {
            let p = StairsParams {
                steps,
                ..StairsParams::default()
            };
            let mesh = assert_sound(stairs(p));
            let n = steps as usize;
            // Each side a grid of quads, one per step and rise below it;
            // walls: a tread and a riser per step, and the bottom and back
            // cut at every step.
            assert_eq!(mesh.face_count(), n * (n + 1) + 4 * n);
            assert!(mesh.faces().all(|(_, c)| c.len() == 4));
            // The outline's 4n edges are rims on both sides; its 2n + 2
            // square corners have hard uprights (the cuts along the bottom
            // and back are straight, so soft).
            assert_eq!(mesh.hard_edge_count(), 2 * 4 * n + 2 * n + 2);
            // Step k (counting from 1) is a column k rises high.
            let (rise, tread) = (p.height / steps as f32, p.depth / steps as f32);
            let side: f32 = (1..=steps).map(|k| tread * rise * k as f32).sum();
            let exact = p.width * side;
            assert!((volume(&mesh) / exact - 1.0).abs() < 1e-4, "{steps}");
            let b = mesh.bounds().unwrap();
            assert!((b.size() - Vec3::new(p.width, p.depth, p.height)).length() < 1e-3);
            // Every step top is level: the points' heights are whole rises.
            for (_, q) in mesh.vertices() {
                let level = q.z / rise;
                assert!((level - level.round()).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn an_arch_has_its_band_s_volume() {
        for (leg_height, segments) in [(100.0, 64), (0.0, 64)] {
            let p = ArchParams {
                leg_height,
                segments,
                ..ArchParams::default()
            };
            let mesh = assert_sound(arch(p));
            let (inner, outer) = (p.width / 2.0, p.width / 2.0 + p.thickness);
            let exact = p.depth
                * (2.0 * p.thickness * leg_height + PI / 2.0 * (outer * outer - inner * inner));
            assert!(
                (volume(&mesh) / exact - 1.0).abs() < 0.01,
                "{p:?}: {}",
                volume(&mesh)
            );
            let b = mesh.bounds().unwrap();
            assert!((b.size().x - 2.0 * outer).abs() < 1e-3);
            assert!((b.size().y - p.depth).abs() < 1e-3);
            assert!((b.size().z - (leg_height + outer)).abs() < 1e-3);
        }
    }

    #[test]
    fn arch_face_counts_and_hard_edges() {
        let p = ArchParams::default();
        let s = p.segments as usize;
        let mesh = assert_sound(arch(p));
        // Front and back: a quad per leg and per piece of the top; walls
        // inside and out, plus the two feet.
        assert_eq!(mesh.face_count(), 4 * (s + 2) + 2);
        // Rims round front and back, and the four corners of the feet: the
        // round top meets the legs smoothly.
        assert_eq!(mesh.hard_edge_count(), 2 * 2 * (s + 3) + 4);
        let half = assert_sound(arch(ArchParams {
            leg_height: 0.0,
            ..p
        }));
        assert_eq!(half.face_count(), 4 * s + 2);
        assert_eq!(half.hard_edge_count(), 2 * 2 * (s + 1) + 4);
    }

    #[test]
    fn walls_are_cut_into_uv_pieces_at_sharp_corners_only() {
        // Count the UV pieces by how many separate islands the corner UVs
        // form: easiest through the seams marked where pieces part.
        let seams = |shape: Primitive| {
            let mesh = shape.mesh();
            crate::uvs::seams(&mesh).len()
        };
        // A whole pie: the round wall is one strip, cut once, plus its two
        // rims from the caps.
        let n = 16;
        let whole = SectorParams {
            angle: 360.0,
            ..SectorParams::default()
        };
        assert_eq!(seams(sector(whole)), 2 * n + 1);
        // Stairs: cut at the rims and every square corner, but the bottom
        // and back each stay one strip.
        let p = StairsParams::default();
        let steps = p.steps as usize;
        assert_eq!(seams(stairs(p)), 2 * 4 * steps + 2 * steps + 2);
    }

    #[test]
    fn degenerate_settings_still_build_sound_solids() {
        let shapes = [
            sector(SectorParams {
                angle: 0.01,
                ..SectorParams::default()
            }),
            sector(SectorParams {
                angle: 0.01,
                inner_radius: 20.0,
                sides: 1,
                ..SectorParams::default()
            }),
            sector(SectorParams {
                angle: 360.0,
                sides: 1,
                ..SectorParams::default()
            }),
            sector(SectorParams {
                angle: 360.0,
                inner_radius: 49.95,
                sides: 2,
                ..SectorParams::default()
            }),
            sector(SectorParams {
                inner_radius: 49.95,
                ..SectorParams::default()
            }),
            sector(SectorParams {
                angle: 359.99,
                sides: 1,
                ..SectorParams::default()
            }),
            sector(SectorParams {
                radius: f32::NAN,
                inner_radius: 1e9,
                angle: -5.0,
                height: 0.0,
                sides: 0,
            }),
            gear(GearParams {
                teeth: 3,
                ..GearParams::default()
            }),
            gear(GearParams {
                teeth: 0,
                tooth_depth: 1e9,
                hole_radius: 0.0,
                ..GearParams::default()
            }),
            gear(GearParams {
                // Just under the gaps.
                hole_radius: 50.0 - 8.0 - 0.01,
                ..GearParams::default()
            }),
            gear(GearParams {
                teeth: MAX_TEETH,
                ..GearParams::default()
            }),
            stairs(StairsParams {
                steps: 1,
                ..StairsParams::default()
            }),
            stairs(StairsParams {
                steps: 0,
                width: -1.0,
                height: f32::INFINITY,
                depth: 0.0,
            }),
            stairs(StairsParams {
                steps: MAX_STEPS,
                ..StairsParams::default()
            }),
            arch(ArchParams {
                leg_height: 0.0,
                segments: 2,
                ..ArchParams::default()
            }),
            arch(ArchParams {
                segments: 0,
                leg_height: -3.0,
                thickness: f32::NAN,
                ..ArchParams::default()
            }),
        ];
        for shape in shapes {
            assert_sound(shape);
        }
    }

    /// What `Primitive::polygon_estimate` should give for these shapes
    /// (the formulas in primitives.rs are to be brought in line with it):
    /// - sector: a ring slice has two arc walls and two caps of a quad per
    ///   piece, plus two ends; a whole turn has at least three pieces;
    /// - gear: 4 walls per tooth, and each end 4 quads per tooth plus, with
    ///   no hole, 4 triangles per tooth;
    /// - stairs: each side n(n + 1)/2 quads, and 4 walls per step;
    /// - arch: unchanged.
    fn estimate(shape: &Primitive) -> usize {
        let n = |x: u32| x as usize;
        match shape.clamped() {
            Primitive::Sector(p) => 4 * n(p.sides).max(3) + 4,
            Primitive::Gear(p) => n(p.teeth) * 20 + 2,
            Primitive::Stairs(p) => n(p.steps) * (n(p.steps) + 5),
            other => other.polygon_estimate(),
        }
    }

    #[test]
    fn face_counts_stay_within_the_polygon_estimate() {
        let mut shapes = Vec::new();
        for sides in [1, 2, 3, 7, 40, MAX_SEGMENTS] {
            for angle in [0.01, 90.0, 200.0, 359.0, 360.0] {
                for inner_radius in [0.0, 20.0] {
                    shapes.push(sector(SectorParams {
                        sides,
                        angle,
                        inner_radius,
                        ..SectorParams::default()
                    }));
                }
            }
        }
        for teeth in [3, 4, 17, MAX_TEETH] {
            for hole_radius in [0.0, 10.0] {
                shapes.push(gear(GearParams {
                    teeth,
                    hole_radius,
                    ..GearParams::default()
                }));
            }
        }
        for steps in [1, 2, 30, MAX_STEPS] {
            shapes.push(stairs(StairsParams {
                steps,
                ..StairsParams::default()
            }));
        }
        for segments in [2, 3, 30, MAX_SEGMENTS] {
            for leg_height in [0.0, 100.0] {
                shapes.push(arch(ArchParams {
                    segments,
                    leg_height,
                    ..ArchParams::default()
                }));
            }
        }
        for shape in shapes {
            let real = shape.mesh().face_count();
            let estimate = estimate(&shape);
            assert!(
                estimate >= real && estimate <= 64 * real + 1024,
                "{shape:?}: estimate {estimate}, real {real}"
            );
        }
    }

    #[test]
    fn wide_sectors_with_few_sides_get_enough_pieces() {
        assert_eq!(arc_pieces(1, 90.0, 50.0), 1);
        assert_eq!(arc_pieces(1, 200.0, 50.0), 2);
        assert_eq!(arc_pieces(1, 359.99, 50.0), 3);
        assert_eq!(arc_pieces(2, 360.0, 50.0), 3);
        assert_eq!(arc_pieces(16, 360.0, 50.0), 16);
        assert_eq!(arc_pieces(1, 0.01, 50.0), 1);
        // A hundredth of a degree at 50 cm is under a tenth of a millimetre
        // of arc: cut into pieces no shorter than SHORTEST_ARC_PIECE.
        assert_eq!(arc_pieces(256, 0.01, 50.0), 8);
    }

    #[test]
    fn every_shape_is_ready_to_bake_and_has_no_problems() {
        let mut shapes = Vec::new();
        for angle in [0.01, 1.0, 90.0, 180.0, 270.0, 359.99, 360.0] {
            for inner_radius in [0.0, 20.0, 49.9] {
                for sides in [1, 3, 16, 64] {
                    shapes.push(sector(SectorParams {
                        angle,
                        inner_radius,
                        sides,
                        ..SectorParams::default()
                    }));
                }
            }
        }
        for teeth in [3, 5, 12, 40, 100] {
            for hole_radius in [0.0, 10.0, 41.9] {
                for tooth_depth in [1.0, 8.0, 40.0] {
                    shapes.push(gear(GearParams {
                        teeth,
                        hole_radius,
                        tooth_depth,
                        ..GearParams::default()
                    }));
                }
            }
        }
        for steps in [1, 2, 6, 30, 100] {
            shapes.push(stairs(StairsParams {
                steps,
                ..StairsParams::default()
            }));
        }
        for leg_height in [0.0, 1.0, 100.0] {
            for segments in [2, 3, 12, 64] {
                shapes.push(arch(ArchParams {
                    leg_height,
                    segments,
                    ..ArchParams::default()
                }));
            }
        }
        for shape in shapes {
            let mesh = shape.mesh();
            let bake = crate::bake::BakeReport::new(&mesh);
            assert!(bake.is_clean(), "{shape:?}: {bake:?}");
            let check = crate::check::MeshReport::new(&mesh);
            assert!(!check.has_problems(), "{shape:?}: {check:?}");
            assert!(check.is_closed(), "{shape:?}");
        }
    }

    #[test]
    fn the_same_settings_build_the_same_mesh() {
        for shape in [
            sector(SectorParams::default()),
            gear(GearParams::default()),
            stairs(StairsParams::default()),
            arch(ArchParams::default()),
        ] {
            assert_eq!(shape.mesh(), shape.mesh());
        }
    }

    #[test]
    fn sharp_turns_are_told_from_gentle_ones() {
        let corner = OutlinePoint::corner(1.0, 0.0);
        assert!(is_sharp(Vec2::ZERO, corner, Vec2::new(1.0, 1.0)), "90°");
        assert!(!is_sharp(Vec2::ZERO, corner, Vec2::new(2.0, 0.2)), "~11°");
        let curve = OutlinePoint::curve(1.0, 0.0);
        assert!(
            !is_sharp(Vec2::ZERO, curve, Vec2::new(1.0, 1.0)),
            "on a curve"
        );
        assert!(
            is_sharp(Vec2::new(1.0, 0.0), corner, Vec2::new(2.0, 0.0)),
            "a zero-length side"
        );
    }
}
