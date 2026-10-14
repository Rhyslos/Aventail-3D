//! Symmetry: working out, for points about to be moved, which points on
//! the other side of the shape mirror them, so they can move too
//! ([decision 0054](../../../docs/decisions/0054-symmetry.md)).
//!
//! The mirror planes go across the shape's own X, Y or Z axis, through a
//! chosen point ([`Mirror::centre`]: the shape's origin, its middle, or
//! the world's centre, with an offset). Points are matched by position, within
//! [`TOLERANCE`]. The result is a [`MirrorPlan`]: groups of points, each
//! moved by the change seen through a mirror (`reflect`) and kept on the
//! planes they sit on (`keep_on`).
//!
//! **Mirroring edits** ([decision 0059](../../../docs/decisions/0059-symmetry-for-every-edit.md)).
//! A [`Mirror`] says which planes, through which point; [`Partners`]
//! finds each point's, edge's and polygon's mirror across one of them, so
//! an edit made on one side can be made on the other too.

use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::{Mat4, Vec3};
use std::collections::{BTreeSet, HashMap};

/// How close (in centimetres) a point must be to a mirrored position to
/// count as its mirror, or to a plane to count as on it.
pub const TOLERANCE: f32 = 0.01;

/// Points that move the same way.
#[derive(Debug, Clone, PartialEq)]
pub struct MirrorGroup {
    /// The axes the change is mirrored across for these points (none for
    /// the points chosen themselves).
    pub reflect: [bool; 3],
    /// The axes whose mirror plane these points sit on: they stay on it.
    pub keep_on: [bool; 3],
    /// The points, in order.
    pub points: Vec<VertexId>,
    /// The point the mirror planes go through.
    pub centre: Vec3,
}

impl MirrorGroup {
    /// The change `local` (in the shape's own space) as these points
    /// make it: mirrored, then kept on their planes.
    pub fn matrix(&self, local: Mat4) -> Mat4 {
        let s = reflection(self.reflect);
        let to = Mat4::from_translation(self.centre);
        let from = Mat4::from_translation(-self.centre);
        to * s * flatten(self.keep_on) * from * local * to * s * from
    }

    /// A move by `offset` (in the shape's own space) as these points make
    /// it.
    pub fn offset(&self, offset: Vec3) -> Vec3 {
        let mut d = offset;
        for axis in 0..3 {
            if self.keep_on[axis] {
                d[axis] = 0.0;
            }
            if self.reflect[axis] {
                d[axis] = -d[axis];
            }
        }
        d
    }
}

/// Which points move how, for a change of some chosen points with
/// symmetry across some axes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MirrorPlan {
    /// The groups, the chosen points' own first.
    pub groups: Vec<MirrorGroup>,
}

impl MirrorPlan {
    /// Every point that moves.
    pub fn points(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.groups.iter().flat_map(|g| g.points.iter().copied())
    }
}

/// A mirror across `axes`.
pub fn reflection(axes: [bool; 3]) -> Mat4 {
    let sign = |on: bool| if on { -1.0 } else { 1.0 };
    Mat4::from_scale(Vec3::new(sign(axes[0]), sign(axes[1]), sign(axes[2])))
}

/// Squashes positions onto the planes of `axes` (their coordinate on
/// each becomes 0).
fn flatten(axes: [bool; 3]) -> Mat4 {
    let keep = |on: bool| if on { 0.0 } else { 1.0 };
    Mat4::from_scale(Vec3::new(keep(axes[0]), keep(axes[1]), keep(axes[2])))
}

/// Finds points by position, within [`TOLERANCE`].
struct PointIndex<'a> {
    mesh: &'a Mesh,
    cells: HashMap<[i64; 3], Vec<VertexId>>,
}

impl<'a> PointIndex<'a> {
    fn new(mesh: &'a Mesh) -> Self {
        let mut cells: HashMap<[i64; 3], Vec<VertexId>> = HashMap::new();
        for (v, p) in mesh.vertices() {
            if p.is_finite() {
                cells.entry(cell(p)).or_default().push(v);
            }
        }
        Self { mesh, cells }
    }

    /// The point nearest `at` within the tolerance, other than `not`.
    fn near(&self, at: Vec3, not: VertexId) -> Option<VertexId> {
        let [x, y, z] = cell(at);
        let mut best: Option<(f32, VertexId)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(found) = self.cells.get(&[x + dx, y + dy, z + dz]) else {
                        continue;
                    };
                    for v in found {
                        if *v == not {
                            continue;
                        }
                        let Some(p) = self.mesh.position(*v) else {
                            continue;
                        };
                        let d = p.distance(at);
                        if d <= TOLERANCE && best.is_none_or(|(b, w)| (d, *v) < (b, w)) {
                            best = Some((d, *v));
                        }
                    }
                }
            }
        }
        best.map(|(_, v)| v)
    }
}

fn cell(p: Vec3) -> [i64; 3] {
    let c = |x: f32| (x / TOLERANCE).floor() as i64;
    [c(p.x), c(p.y), c(p.z)]
}

/// The mirrors to use with `axes` on: every combination of them (X; X and
/// Y gives X, Y and both).
fn reflections(axes: [bool; 3]) -> Vec<[bool; 3]> {
    (1u8..8)
        .map(|m| [m & 1 != 0, m & 2 != 0, m & 4 != 0])
        .filter(|r| (0..3).all(|a| !r[a] || axes[a]))
        .collect()
}

/// Which points move how when `chosen` points move with symmetry across
/// `mirror`'s planes (in the shape's own space).
///
/// - Each chosen point drives its mirrors: they move the mirrored way.
/// - A chosen point that is itself the mirror of one before it (both
///   sides chosen) follows that one, so the shape stays symmetrical.
/// - A point on a mirror plane stays on it.
/// - A point with no mirror (the shape isn't symmetrical there) just
///   moves.
///
/// With no axis on, every chosen point is in one group, moving as it is.
pub fn mirror_plan(mesh: &Mesh, chosen: &[VertexId], mirror: Mirror) -> MirrorPlan {
    let axes = mirror.axes;
    let centre = mirror.centre;
    let mut chosen: Vec<VertexId> = chosen
        .iter()
        .copied()
        .filter(|v| mesh.position(*v).is_some())
        .collect();
    chosen.sort_unstable();
    chosen.dedup();
    if !axes.contains(&true) {
        return MirrorPlan {
            groups: vec![MirrorGroup {
                reflect: [false; 3],
                keep_on: [false; 3],
                points: chosen,
                centre,
            }],
        };
    }
    let index = PointIndex::new(mesh);
    let mirrors = reflections(axes);
    let mut taken: BTreeSet<VertexId> = BTreeSet::new();
    let mut groups: Vec<MirrorGroup> = Vec::new();
    let mut add = |reflect: [bool; 3], keep_on: [bool; 3], v: VertexId| match groups
        .iter_mut()
        .find(|g| g.reflect == reflect && g.keep_on == keep_on)
    {
        Some(group) => group.points.push(v),
        None => groups.push(MirrorGroup {
            reflect,
            keep_on,
            points: vec![v],
            centre,
        }),
    };
    for v in chosen {
        if !taken.insert(v) {
            continue;
        }
        let Some(p) = mesh.position(v) else {
            continue;
        };
        let keep_on = [0, 1, 2].map(|a| axes[a] && (p[a] - centre[a]).abs() <= TOLERANCE);
        add([false; 3], keep_on, v);
        for reflect in &mirrors {
            // Mirroring across a plane the point is on gives the point.
            if (0..3).all(|a| !reflect[a] || keep_on[a]) {
                continue;
            }
            let at = mirror.point(*reflect, p);
            if let Some(other) = index.near(at, v)
                && taken.insert(other)
            {
                add(*reflect, keep_on, other);
            }
        }
    }
    // The chosen points' own groups first, then by mirror.
    groups.sort_by_key(|g| (g.reflect, g.keep_on));
    MirrorPlan { groups }
}

/// Mirror planes for edits: across the shape's own X, Y and/or Z axes,
/// through `centre` (in the shape's own space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mirror {
    /// The axes mirrored across.
    pub axes: [bool; 3],
    /// The point the planes go through.
    pub centre: Vec3,
}

impl Mirror {
    /// Across `axes`, through the shape's origin.
    pub fn through_origin(axes: [bool; 3]) -> Self {
        Self {
            axes,
            centre: Vec3::ZERO,
        }
    }

    /// Every mirror to make an edit's copy across: each combination of
    /// the axes (X and Y gives X, Y and both).
    pub fn reflections(&self) -> Vec<[bool; 3]> {
        reflections(self.axes)
    }

    /// `p` mirrored across `reflect`.
    pub fn point(&self, reflect: [bool; 3], p: Vec3) -> Vec3 {
        self.centre + reflection(reflect).transform_vector3(p - self.centre)
    }

    /// The direction `d` mirrored across `reflect`.
    pub fn vector(reflect: [bool; 3], d: Vec3) -> Vec3 {
        reflection(reflect).transform_vector3(d)
    }
}

/// Each element's mirror across one plane (or more) of a shape, matched by
/// position within [`TOLERANCE`]. A point on the plane is its own mirror.
pub struct Partners<'a> {
    mesh: &'a Mesh,
    index: PointIndex<'a>,
    mirror: Mirror,
    reflect: [bool; 3],
}

impl<'a> Partners<'a> {
    /// The mirrors in `mesh` across `reflect` of `mirror`.
    pub fn new(mesh: &'a Mesh, mirror: Mirror, reflect: [bool; 3]) -> Self {
        Self {
            mesh,
            index: PointIndex::new(mesh),
            mirror,
            reflect,
        }
    }

    /// The point mirroring `v`, if the shape has one there.
    pub fn point(&self, v: VertexId) -> Option<VertexId> {
        let at = self.mirror.point(self.reflect, self.mesh.position(v)?);
        if self.mesh.position(v)?.distance(at) <= TOLERANCE {
            return Some(v);
        }
        self.index.near(at, v)
    }

    /// The edge mirroring `edge`, if the shape has one there.
    pub fn edge(&self, edge: EdgeKey) -> Option<EdgeKey> {
        let (a, b) = edge.ends();
        let mirrored = EdgeKey::new(self.point(a)?, self.point(b)?);
        self.mesh.topology().has_edge(mirrored).then_some(mirrored)
    }

    /// The polygon mirroring `face`, if the shape has one there.
    pub fn face(&self, face: FaceId) -> Option<FaceId> {
        let corners = self.mesh.face(face)?;
        let mirrored: Vec<VertexId> = corners
            .iter()
            .map(|v| self.point(*v))
            .collect::<Option<_>>()?;
        let wanted: BTreeSet<VertexId> = mirrored.iter().copied().collect();
        self.mesh
            .topology()
            .vertex_faces(*mirrored.first()?)
            .iter()
            .copied()
            .find(|f| {
                self.mesh.face(*f).is_some_and(|c| {
                    c.len() == corners.len() && c.iter().copied().collect::<BTreeSet<_>>() == wanted
                })
            })
    }
}

#[cfg(test)]
mod tests;
