//! Extruding and insetting: the first operations that add points and
//! polygons rather than only moving them ([decision 0038](../../../docs/decisions/0038-extrude-and-inset.md)).
//!
//! - **Extrude polygons** ([`extrude_polygons`]): the polygons move out
//!   along their facing, and a wall of new four-sided polygons joins
//!   their old border to the new one.
//! - **Extrude edges** ([`extrude_edges`]): edges on an open border grow
//!   a new strip of polygons outward, flat with the polygon they belong
//!   to, like pulling more cloth off the edge of a sheet.
//! - **Inset** ([`inset_polygons`]): the polygons shrink inward, flat
//!   where they are, leaving a ring of new polygons around them.
//!
//! [`Grouping`] says whether neighbouring polygons (or edges joined at a
//! point) work as one piece or each on its own.
//!
//! **Ids stay put.** Each extruded or inset polygon keeps its id (it gets
//! new corners), so what was selected is still selected afterwards and
//! pressing the key again carries on from there. New points and polygons
//! get the next free ids, in a fixed order, so replaying an edit gives
//! exactly the same ids.
//!
//! **Even thickness.** Where several polygons meet at a point, the point
//! moves along their average facing, a little further where they meet at
//! an angle, so every polygon ends up the full distance away (at most
//! four times as far, at very sharp corners).
//!
//! **Hard edges carry over.** The new border copies the old border's hard
//! edges, and a wall's upright edge is hard where a hard border edge meets
//! it, so an extruded cube stays crisp and an extruded sphere stays smooth.
//!
//! **Attributes carry over** through the [`Builder`]: new points copy the
//! point they came from, walls are like the polygon they came from (its
//! material slot, say), their corners copy that polygon's corners, and the
//! new border takes the old border's edge values (seams as well as hard
//! edges).

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::edit::Outcome;
use crate::polygon::sides;
use crate::topology::Topology;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

/// Whether neighbouring polygons (or edges) work as one piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Grouping {
    /// Polygons that share an edge (or edges that share a point) move as
    /// one piece: a block pulled out of a wall.
    #[default]
    Together,
    /// Every polygon (or edge) on its own: a separate bump for each.
    Separate,
}

/// The smallest cosine used when keeping thickness even: a point never
/// moves more than four times the distance.
const MIN_COS: f32 = 0.25;

/// The direction a point moves so that each of `directions` (unit
/// vectors: the facings of the polygons around it, say) ends up a whole
/// step away: their average, lengthened where they spread apart. Zero if
/// there is no direction to go.
fn even_direction(directions: &[Vec3]) -> Vec3 {
    let sum: Vec3 = directions.iter().copied().sum();
    let Some(direction) = sum.try_normalize() else {
        // They cancel out (a sliver folded flat): take any one of them.
        return directions
            .iter()
            .find_map(|d| d.try_normalize())
            .unwrap_or(Vec3::ZERO);
    };
    let cos = directions
        .iter()
        .filter(|d| d.length_squared() > 0.0)
        .map(|d| direction.dot(*d))
        .fold(1.0_f32, f32::min);
    direction / cos.max(MIN_COS)
}

/// The polygons of `faces` that are in the mesh, sorted and each once, and
/// how many listed weren't.
fn existing_faces(mesh: &Mesh, faces: &[FaceId]) -> (Vec<FaceId>, usize) {
    let mut found = BTreeSet::new();
    let mut missing = 0;
    for face in faces {
        if mesh.face(*face).is_some() {
            found.insert(*face);
        } else if !found.contains(face) {
            missing += 1;
        }
    }
    (found.into_iter().collect(), missing)
}

/// `faces` split into the pieces that work together: polygons joined by
/// an edge (`Together`), or each alone (`Separate`).
fn face_regions(topology: &Topology, faces: &[FaceId], grouping: Grouping) -> Vec<Vec<FaceId>> {
    if grouping == Grouping::Separate {
        return faces.iter().map(|f| vec![*f]).collect();
    }
    let chosen: BTreeSet<FaceId> = faces.iter().copied().collect();
    let mut seen = BTreeSet::new();
    let mut regions = Vec::new();
    for start in faces {
        if !seen.insert(*start) {
            continue;
        }
        let mut region = Vec::new();
        let mut stack = vec![*start];
        while let Some(face) = stack.pop() {
            region.push(face);
            for edge in topology.face_edges(face) {
                for other in topology.edge_faces(*edge) {
                    if chosen.contains(&other) && seen.insert(other) {
                        stack.push(other);
                    }
                }
            }
        }
        region.sort_unstable();
        regions.push(region);
    }
    regions
}

/// How a piece of polygons is put together.
struct Region {
    faces: Vec<FaceId>,
    /// The sides on the piece's border, each as its polygon runs along it.
    border: Vec<(FaceId, VertexId, VertexId)>,
    /// Edges with the piece on both sides.
    inner: Vec<EdgeKey>,
    /// Every corner point of the piece, and whether it lies on the border.
    points: BTreeMap<VertexId, bool>,
}

impl Region {
    fn new(mesh: &Mesh, faces: Vec<FaceId>) -> Self {
        let mut uses: BTreeMap<EdgeKey, usize> = BTreeMap::new();
        let mut points = BTreeMap::new();
        for face in &faces {
            let corners = mesh.face(*face).unwrap_or_default();
            for (a, b) in sides(corners) {
                *uses.entry(EdgeKey::new(a, b)).or_default() += 1;
            }
            for v in corners {
                points.insert(*v, false);
            }
        }
        let mut border = Vec::new();
        for face in &faces {
            for (a, b) in sides(mesh.face(*face).unwrap_or_default()) {
                if uses.get(&EdgeKey::new(a, b)) == Some(&1) {
                    border.push((*face, a, b));
                    points.insert(a, true);
                    points.insert(b, true);
                }
            }
        }
        let inner = uses
            .into_iter()
            .filter(|(_, n)| *n >= 2)
            .map(|(e, _)| e)
            .collect();
        Self {
            faces,
            border,
            inner,
            points,
        }
    }

    /// Gives the piece's polygons their new corners (`map`), and carries
    /// hard edges inside the piece over to the new corners.
    fn rewire(&self, build: &mut Builder, map: &BTreeMap<VertexId, VertexId>) {
        let to = |v: VertexId| map.get(&v).copied().unwrap_or(v);
        for face in &self.faces {
            let corners: Vec<VertexId> = build
                .original()
                .face(*face)
                .unwrap_or_default()
                .iter()
                .map(|v| to(*v))
                .collect();
            // Can't fail: the same number of distinct, existing corners.
            // Corner values stay with their corners.
            let _ = build.mesh().replace_corners(*face, &corners);
        }
        for edge in &self.inner {
            let (a, b) = edge.ends();
            build.move_edge(*edge, to(a), to(b));
        }
    }

    /// Adds a four-sided polygon along each border side, from the old
    /// border to the new one, and copies the border's hard edges to the
    /// new border. With `hard_uprights`, a wall's upright edge is hard
    /// where a hard border edge meets it.
    fn walls(&self, build: &mut Builder, map: &BTreeMap<VertexId, VertexId>, hard_uprights: bool) {
        let mut hard_points = BTreeSet::new();
        for (face, a, b) in &self.border {
            let (Some(x), Some(y)) = (map.get(a), map.get(b)) else {
                continue;
            };
            // The wall is like the polygon it grew from; its corners copy
            // that polygon's corners at the same points.
            let corners = [
                (*a, CornerFrom::Copy(*face, *a)),
                (*b, CornerFrom::Copy(*face, *b)),
                (*y, CornerFrom::Copy(*face, *b)),
                (*x, CornerFrom::Copy(*face, *a)),
            ];
            build.face(&corners, Some(*face));
            build.copy_edge(EdgeKey::new(*a, *b), *x, *y);
            if build.original().is_hard(*a, *b) {
                hard_points.insert(*a);
                hard_points.insert(*b);
            }
        }
        if hard_uprights {
            for v in hard_points {
                if let Some(copy) = map.get(&v) {
                    build.mesh().set_hard(v, *copy, true);
                }
            }
        }
    }
}

/// Extrudes `faces` by `distance` (in the mesh's own units; negative
/// pushes them in), as one piece where they touch or each on its own
/// (`grouping`). The polygons keep their ids; the walls are new.
///
/// With [`Grouping::Together`], a point inside the piece (every polygon
/// around it chosen, and not on its border) simply moves; points on the
/// border get a copy that moves, and the old point stays for the walls.
/// A piece covering a whole closed shape has no border, so it grows (or
/// shrinks) instead.
///
/// Polygons not in the mesh are counted as missing. A distance that isn't
/// a number changes nothing.
pub fn extrude_polygons(
    mesh: &mut Mesh,
    faces: &[FaceId],
    distance: f32,
    grouping: Grouping,
) -> Outcome {
    let (faces, missing) = existing_faces(mesh, faces);
    let outcome = Outcome {
        matched: faces.len(),
        missing,
        skipped: 0,
    };
    if faces.is_empty() || !distance.is_finite() {
        return outcome;
    }
    let original = mesh.clone();
    let topology = original.topology();
    let mut build = Builder::new(&original, mesh);
    for faces in face_regions(topology, &faces, grouping) {
        let region = Region::new(&original, faces);
        let chosen: BTreeSet<FaceId> = region.faces.iter().copied().collect();
        // Each point's facings, from the piece's polygons around it.
        let mut facings: BTreeMap<VertexId, Vec<Vec3>> = BTreeMap::new();
        for face in &region.faces {
            let normal = original.face_normal(*face);
            for v in original.face(*face).unwrap_or_default() {
                facings.entry(*v).or_default().push(normal);
            }
        }
        let mut map = BTreeMap::new();
        for (v, on_border) in &region.points {
            let Some(at) = original.position(*v) else {
                continue;
            };
            let offset = even_direction(facings.get(v).map_or(&[][..], Vec::as_slice)) * distance;
            let to = if (at + offset).is_finite() {
                at + offset
            } else {
                at
            };
            // A point shared with a polygon outside the piece must stay
            // for that polygon.
            let shared = topology
                .vertex_faces(*v)
                .iter()
                .any(|f| !chosen.contains(f));
            if *on_border || shared {
                map.insert(*v, build.point(to, PointFrom::Copy(*v)));
            } else {
                build.mesh().set_position(*v, to);
            }
        }
        region.rewire(&mut build, &map);
        region.walls(&mut build, &map, true);
    }
    outcome
}

/// Insets `faces` by `distance` (in the mesh's own units, inward; negative
/// grows them outward), as one piece where they touch or each on its own
/// (`grouping`). The polygons keep their ids and their plane; the ring
/// around them is new.
///
/// Only the piece's border moves in: points inside a piece stay where they
/// are. Insetting further than a polygon is wide makes it cross over
/// itself, as in other modelling programs; the tool keeps the distance
/// sensible.
///
/// Polygons not in the mesh are counted as missing. A distance that isn't
/// a number changes nothing.
pub fn inset_polygons(
    mesh: &mut Mesh,
    faces: &[FaceId],
    distance: f32,
    grouping: Grouping,
) -> Outcome {
    let (faces, missing) = existing_faces(mesh, faces);
    let outcome = Outcome {
        matched: faces.len(),
        missing,
        skipped: 0,
    };
    if faces.is_empty() || !distance.is_finite() {
        return outcome;
    }
    let original = mesh.clone();
    let topology = original.topology();
    let mut build = Builder::new(&original, mesh);
    for faces in face_regions(topology, &faces, grouping) {
        let region = Region::new(&original, faces);
        // Each border point's inward directions, from the border sides
        // meeting there.
        let mut inward: BTreeMap<VertexId, Vec<Vec3>> = BTreeMap::new();
        for (face, a, b) in &region.border {
            let (Some(pa), Some(pb)) = (original.position(*a), original.position(*b)) else {
                continue;
            };
            let direction = original
                .face_normal(*face)
                .cross(pb - pa)
                .normalize_or_zero();
            inward.entry(*a).or_default().push(direction);
            inward.entry(*b).or_default().push(direction);
        }
        let mut map = BTreeMap::new();
        for (v, directions) in &inward {
            let Some(at) = original.position(*v) else {
                continue;
            };
            let offset = even_direction(directions) * distance;
            let to = if (at + offset).is_finite() {
                at + offset
            } else {
                at
            };
            map.insert(*v, build.point(to, PointFrom::Copy(*v)));
        }
        region.rewire(&mut build, &map);
        // The ring is flat with the polygons: no hard uprights.
        region.walls(&mut build, &map, false);
    }
    outcome
}

/// Extrudes `edges` by `distance` (in the mesh's own units): each edge on
/// an open border grows a new four-sided polygon outward, flat with the
/// polygon it belongs to. Edges joined at a point grow together
/// ([`Grouping::Together`]) or each on its own.
///
/// Edges not in the mesh are counted as missing; edges with polygons on
/// both sides (or none) are skipped, as there is no open side to grow
/// from. A distance that isn't a number changes nothing.
pub fn extrude_edges(
    mesh: &mut Mesh,
    edges: &[EdgeKey],
    distance: f32,
    grouping: Grouping,
) -> Outcome {
    let original = mesh.clone();
    let topology = original.topology();
    let mut outcome = Outcome::default();
    // Each border edge as its polygon runs along it, with the outward
    // direction.
    let mut border: BTreeMap<EdgeKey, (VertexId, VertexId, Vec3, FaceId)> = BTreeMap::new();
    for edge in edges.iter().copied().collect::<BTreeSet<_>>() {
        if !topology.has_edge(edge) {
            outcome.missing += 1;
            continue;
        }
        let [only] = topology.edge_uses(edge) else {
            outcome.skipped += 1;
            continue;
        };
        let (low, high) = edge.ends();
        let (a, b) = if only.forward {
            (low, high)
        } else {
            (high, low)
        };
        let (Some(pa), Some(pb)) = (original.position(a), original.position(b)) else {
            outcome.missing += 1;
            continue;
        };
        let outward = (pb - pa)
            .cross(original.face_normal(only.face))
            .normalize_or_zero();
        border.insert(edge, (a, b, outward, only.face));
        outcome.matched += 1;
    }
    if border.is_empty() || !distance.is_finite() {
        return outcome;
    }
    let mut build = Builder::new(&original, mesh);
    for group in edge_regions(&border, grouping) {
        let mut outward: BTreeMap<VertexId, Vec<Vec3>> = BTreeMap::new();
        for edge in &group {
            let (a, b, direction, _) = border[edge];
            outward.entry(a).or_default().push(direction);
            outward.entry(b).or_default().push(direction);
        }
        let mut map = BTreeMap::new();
        for (v, directions) in &outward {
            let Some(at) = original.position(*v) else {
                continue;
            };
            let offset = even_direction(directions) * distance;
            let to = if (at + offset).is_finite() {
                at + offset
            } else {
                at
            };
            map.insert(*v, build.point(to, PointFrom::Copy(*v)));
        }
        for edge in &group {
            let (a, b, _, face) = border[edge];
            // The polygon runs a → b, so the new one runs b → a. It is like
            // that polygon, and its corners copy that polygon's.
            let corners = [
                (b, CornerFrom::Copy(face, b)),
                (a, CornerFrom::Copy(face, a)),
                (map[&a], CornerFrom::Copy(face, a)),
                (map[&b], CornerFrom::Copy(face, b)),
            ];
            build.face(&corners, Some(face));
        }
    }
    outcome
}

/// Border edges split into the groups that grow together: joined at a
/// point (`Together`), or each alone (`Separate`).
fn edge_regions<T>(border: &BTreeMap<EdgeKey, T>, grouping: Grouping) -> Vec<Vec<EdgeKey>> {
    if grouping == Grouping::Separate {
        return border.keys().map(|e| vec![*e]).collect();
    }
    let mut at_point: BTreeMap<VertexId, Vec<EdgeKey>> = BTreeMap::new();
    for edge in border.keys() {
        let (a, b) = edge.ends();
        at_point.entry(a).or_default().push(*edge);
        at_point.entry(b).or_default().push(*edge);
    }
    let mut seen = BTreeSet::new();
    let mut groups = Vec::new();
    for start in border.keys() {
        if !seen.insert(*start) {
            continue;
        }
        let mut group = Vec::new();
        let mut stack = vec![*start];
        while let Some(edge) = stack.pop() {
            group.push(edge);
            let (a, b) = edge.ends();
            for point in [a, b] {
                for other in &at_point[&point] {
                    if seen.insert(*other) {
                        stack.push(*other);
                    }
                }
            }
        }
        group.sort_unstable();
        groups.push(group);
    }
    groups
}

// ------------------------------------------------------------ bending ---

/// The most rows a bend can have in one turn (or in the whole bend, for
/// a bend of one turn or less).
pub const MAX_BEND_SEGMENTS: u32 = 64;
/// The furthest a bend with a radius can turn, in degrees: ten turns, a
/// long spiral ([decision 0062](../../../docs/decisions/0062-spirals.md)).
pub const MAX_BEND_ANGLE: f32 = 3600.0;
/// The most rows a whole spiral can have, so changing a long spiral stays
/// quick: enough for 64 rows a turn over four turns, or 25 over ten.
pub const MAX_BEND_ROWS: usize = 256;
/// How much room a spiral leaves between its loops, as a share of the
/// pipe's width, so the loops don't touch.
pub const SPIRAL_GAP: f32 = 0.05;
/// The sharpest a joint without a bend radius can turn, in degrees: a
/// mitre joint stretches its ring by 1 / cos(half the angle), which grows
/// without limit towards 180°.
pub const MAX_MITRE_ANGLE: f32 = 150.0;

/// A bend at the start of an extrude, so a pipe turns a corner and keeps
/// its full width ([decision 0050](../../../docs/decisions/0050-pipe-bends.md)).
///
/// With a radius it can turn past a full circle into a spiral
/// ([decision 0062](../../../docs/decisions/0062-spirals.md)): each loop
/// climbs along the turn's axis just far enough to clear the one before,
/// plus `rise`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bend {
    /// How far it turns, in degrees: 0 (straight) to 180 for a sharp
    /// corner, up to [`MAX_BEND_ANGLE`] with a radius.
    pub angle: f32,
    /// Which way round it turns, in degrees: 0 turns up (an upright pipe
    /// turns towards +X), and it goes round the pipe from there.
    pub direction: f32,
    /// How far the middle of the turn is from the middle of the pipe, in
    /// the mesh's own units: 0 is a sharp corner (a mitre joint).
    pub radius: f32,
    /// How many rows a rounded turn has (1 to [`MAX_BEND_SEGMENTS`]); for a
    /// spiral, how many each full turn has.
    pub segments: u32,
    /// How much further each full turn climbs along the turn's axis, in
    /// the mesh's own units (0 or more): past one turn on top of what
    /// clears the loop before, under one turn on its own. Needs a radius.
    pub rise: f32,
    /// Whether it climbs the other way along its axis: set on a mirrored
    /// copy, so the copy is the spiral's mirror image (a mirror turns a
    /// right-handed spiral into a left-handed one).
    pub mirrored: bool,
}

impl Default for Bend {
    fn default() -> Self {
        Self {
            angle: 0.0,
            direction: 0.0,
            radius: 0.0,
            segments: 8,
            rise: 0.0,
            mirrored: false,
        }
    }
}

impl Bend {
    /// The same bend with every number in range (and numbers that aren't
    /// real made straight).
    #[must_use]
    pub fn clamped(self) -> Self {
        let real = |x: f32, or: f32| if x.is_finite() { x } else { or };
        let radius = real(self.radius, 0.0).max(0.0);
        let most = if radius > 0.0 {
            MAX_BEND_ANGLE
        } else {
            MAX_MITRE_ANGLE
        };
        Self {
            angle: real(self.angle, 0.0).clamp(0.0, most),
            direction: real(self.direction, 0.0).rem_euclid(360.0),
            radius,
            segments: self.segments.clamp(1, MAX_BEND_SEGMENTS),
            rise: real(self.rise, 0.0).max(0.0),
            mirrored: self.mirrored,
        }
    }

    /// Whether it turns past a full circle, into a spiral whose loops
    /// climb clear of each other.
    pub fn spirals(&self) -> bool {
        let bend = self.clamped();
        bend.radius > 0.0 && bend.angle > 360.0 + 1e-3
    }

    /// How many rows its turn has: the segments, or for a spiral the
    /// segments in each turn (at most [`MAX_BEND_ROWS`]).
    pub fn rows(&self) -> usize {
        let bend = self.clamped();
        if !bend.bends() || bend.radius <= 0.0 {
            0
        } else if bend.spirals() {
            let rows = (bend.segments as f32 * bend.angle / 360.0).ceil() as usize;
            rows.min(MAX_BEND_ROWS)
        } else {
            bend.segments as usize
        }
    }

    /// How far it climbs along its axis in each full turn, for a pipe
    /// `width` wide along that axis: past one turn, the width and a small
    /// gap so the loops clear each other; plus the rise.
    pub fn climb_per_turn(&self, width: f32) -> f32 {
        let bend = self.clamped();
        if bend.radius <= 0.0 {
            return 0.0;
        }
        let clear = if bend.spirals() {
            width.max(0.0) * (1.0 + SPIRAL_GAP)
        } else {
            0.0
        };
        clear + bend.rise
    }

    /// Whether it turns at all.
    pub fn bends(&self) -> bool {
        self.clamped().angle > 1e-3
    }

    /// How many rings of new points an extrude with this bend makes: one
    /// per row of the turn, and one more for a straight part after it.
    pub fn rings(&self, distance: f32) -> usize {
        let turn = self.rows();
        let straight = usize::from(distance > 1e-4 || turn == 0);
        turn + straight
    }
}

/// Where a bend's direction 0° points for a piece facing `facing` (unit):
/// up, or +X for a piece facing straight up or down.
fn bend_reference(facing: Vec3) -> Option<Vec3> {
    let reference = if facing.z.abs() < 0.99 {
        Vec3::Z
    } else {
        Vec3::X
    };
    (reference - facing * reference.dot(facing)).try_normalize()
}

/// The way a bend with `direction` (degrees) turns a piece facing
/// `facing` (unit): a unit direction across the facing.
pub fn bend_towards(facing: Vec3, direction: f32) -> Option<Vec3> {
    let first = bend_reference(facing)?;
    Some(glam::Quat::from_axis_angle(facing, direction.to_radians()) * first)
}

/// The bend direction (degrees, 0 to 360) that turns a piece facing
/// `facing` (unit) towards `towards`: the opposite of [`bend_towards`].
/// Symmetry uses it to turn a mirrored pipe the mirrored way.
pub fn bend_direction(facing: Vec3, towards: Vec3) -> f32 {
    let Some(first) = bend_reference(facing) else {
        return 0.0;
    };
    let across = (towards - facing * towards.dot(facing)).normalize_or_zero();
    let angle = facing.dot(first.cross(across)).atan2(first.dot(across));
    angle.to_degrees().rem_euclid(360.0)
}

/// The way a piece faces and turns: its middle, its facing, the way the
/// bend turns towards, and the axis it turns around.
struct Turn {
    middle: Vec3,
    facing: Vec3,
    towards: Vec3,
    axis: Vec3,
}

impl Turn {
    fn of(mesh: &Mesh, faces: &[FaceId], direction: f32) -> Option<Self> {
        let points: BTreeSet<VertexId> = faces
            .iter()
            .flat_map(|f| mesh.face(*f).unwrap_or_default().iter().copied())
            .collect();
        let places: Vec<Vec3> = points.iter().filter_map(|v| mesh.position(*v)).collect();
        if places.is_empty() {
            return None;
        }
        let middle = places.iter().copied().sum::<Vec3>() / places.len() as f32;
        let facing = faces
            .iter()
            .map(|f| mesh.face_normal(*f))
            .sum::<Vec3>()
            .try_normalize()?;
        let towards = bend_towards(facing, direction)?;
        Some(Self {
            middle,
            facing,
            towards,
            axis: facing.cross(towards),
        })
    }

    /// `p` turned by `angle` (radians) around the turn's axis through
    /// `pivot`.
    fn turned(&self, pivot: Vec3, angle: f32, p: Vec3) -> Vec3 {
        pivot + glam::Quat::from_axis_angle(self.axis, angle) * (p - pivot)
    }

    /// Where `p`, on the piece's border, goes in a mitre joint turning by
    /// `angle` (radians): onto the plane halfway between the two
    /// directions, stretched across the turn so the pipe keeps its width.
    fn mitred(&self, angle: f32, p: Vec3) -> Vec3 {
        let q = p - self.middle;
        let (x, y, z) = (q.dot(self.towards), q.dot(self.axis), q.dot(self.facing));
        let half = angle * 0.5;
        let flat = self.towards * (x / half.cos()) + self.axis * y;
        self.middle + glam::Quat::from_axis_angle(self.axis, half) * flat + self.facing * z
    }
}

/// Extrudes `faces` with a bend: each piece (see [`Grouping`]) first
/// turns by the bend's angle, in its direction, around a point `radius`
/// away from the piece's middle, then goes on straight for `distance`
/// (negative counts as 0). The turn is made of rings that keep the piece's
/// full size, so a pipe bends without pinching. With no radius the turn
/// is a sharp mitre joint: the piece's old border moves onto the plane
/// halfway between the two directions, stretched so the pipe stays round
/// through the corner.
///
/// A bend that doesn't turn is a plain [`extrude_polygons`]. Polygons keep
/// their ids, new points and polygons take the next ids, and values come
/// along as with every extrude. Polygons not in the mesh are counted as
/// missing; numbers that aren't real change nothing.
pub fn extrude_bent(
    mesh: &mut Mesh,
    faces: &[FaceId],
    distance: f32,
    grouping: Grouping,
    bend: Bend,
) -> Outcome {
    let bend = bend.clamped();
    if !bend.bends() || !distance.is_finite() {
        return extrude_polygons(mesh, faces, distance, grouping);
    }
    let (faces, missing) = existing_faces(mesh, faces);
    let outcome = Outcome {
        matched: faces.len(),
        missing,
        skipped: 0,
    };
    if faces.is_empty() {
        return outcome;
    }
    let distance = distance.max(0.0);
    let original = mesh.clone();
    let pieces: Vec<(Vec<FaceId>, Turn)> = face_regions(original.topology(), &faces, grouping)
        .into_iter()
        .filter_map(|piece| {
            let turn = Turn::of(&original, &piece, bend.direction)?;
            Some((piece, turn))
        })
        .collect();
    // Extrude ring after ring (where they go is set afterwards), following
    // each original point to its copy in every ring through the polygons'
    // corners, which keep their order.
    let rings = bend.rings(distance);
    let mut follow: Vec<BTreeMap<VertexId, VertexId>> = Vec::with_capacity(rings);
    for _ in 0..rings {
        let before: Vec<Vec<VertexId>> = faces
            .iter()
            .map(|f| mesh.face(*f).unwrap_or_default().to_vec())
            .collect();
        extrude_polygons(mesh, &faces, 1.0, grouping);
        let mut step = BTreeMap::new();
        for (f, old) in faces.iter().zip(&before) {
            for (a, b) in old.iter().zip(mesh.face(*f).unwrap_or_default()) {
                step.insert(*a, *b);
            }
        }
        follow.push(step);
    }
    let angle = bend.angle.to_radians();
    let rows = bend.rows();
    for (piece, turn) in &pieces {
        let points: BTreeSet<VertexId> = piece
            .iter()
            .flat_map(|f| original.face(*f).unwrap_or_default().iter().copied())
            .collect();
        let ahead = glam::Quat::from_axis_angle(turn.axis, angle) * turn.facing;
        // A spiral climbs along the turn's axis as it goes round, by its
        // climb for each full turn.
        let across: Vec<f32> = points
            .iter()
            .filter_map(|v| original.position(*v))
            .map(|p| (p - turn.middle).dot(turn.axis))
            .collect();
        let width = across.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - across.iter().copied().fold(f32::INFINITY, f32::min);
        let up = if bend.mirrored { -turn.axis } else { turn.axis };
        let climb = |turned: f32| up * bend.climb_per_turn(width) * turned / std::f32::consts::TAU;
        for v in points {
            let Some(at) = original.position(v) else {
                continue;
            };
            // The point in each ring.
            let mut id = v;
            let mut ids = Vec::with_capacity(rings);
            for step in &follow {
                id = step.get(&id).copied().unwrap_or(id);
                ids.push(id);
            }
            let on_border = ids.first().is_some_and(|first| *first != v);
            let mut places = Vec::with_capacity(rings);
            if bend.radius > 0.0 {
                let pivot = turn.middle + turn.towards * bend.radius;
                for k in 1..=rows {
                    let share = k as f32 / rows as f32;
                    places.push(turn.turned(pivot, angle * share, at) + climb(angle * share));
                }
            } else if on_border {
                // The old border becomes the mitre joint.
                mesh.set_position(v, turn.mitred(angle, at));
            }
            if places.len() < rings {
                let end = turn.turned(turn.middle, angle, at);
                let end = if bend.radius > 0.0 {
                    places.last().copied().unwrap_or(end)
                } else {
                    end
                };
                places.push(end + ahead * distance);
            }
            for (id, place) in ids.iter().zip(&places) {
                if place.is_finite() {
                    mesh.set_position(*id, *place);
                }
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests;
