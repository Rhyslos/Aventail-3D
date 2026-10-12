//! The polygon mesh: points in space joined into faces with any number of
//! corners (n-gons), with each edge marked hard or soft.
//!
//! Every point and face has an id that stays the same for its whole life
//! and is never given to another element, even after it is deleted. Ids
//! are handed out in order, so building the same mesh the same way always
//! gives the same ids (DESIGN.md §4.1–4.2).
//!
//! Adjacency (walking from an edge to its faces, loops and rings) lives in
//! [`Topology`], worked out once per set of faces ([`Mesh::topology`]).

use crate::{Bounds, Topology};
use glam::Vec3;
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

/// A point's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(u32);

/// A face's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(u32);

impl VertexId {
    /// The id as a number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// The id with this number, as recorded earlier (by an operation, or
    /// in a saved file). It may not be in a given mesh.
    pub const fn from_raw(number: u32) -> Self {
        Self(number)
    }
}

impl FaceId {
    /// The id as a number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// The id with this number, as recorded earlier. It may not be in a
    /// given mesh.
    pub const fn from_raw(number: u32) -> Self {
        Self(number)
    }
}

/// An edge between two points, whichever way round it is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeKey(VertexId, VertexId);

impl EdgeKey {
    /// The edge between `a` and `b`.
    pub fn new(a: VertexId, b: VertexId) -> Self {
        if a <= b { Self(a, b) } else { Self(b, a) }
    }

    /// Its two ends, lower id first.
    pub fn ends(self) -> (VertexId, VertexId) {
        (self.0, self.1)
    }
}

/// Why a face couldn't be added, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// A face needs at least three corners.
    TooFewCorners,
    /// A face used a point that isn't in the mesh.
    UnknownVertex(VertexId),
    /// A face went through the same point twice.
    RepeatedVertex(VertexId),
    /// The mesh can't hold more elements.
    Full,
    /// A point's position isn't a real number (infinite or not a number).
    NotFinite(VertexId),
    /// A hard edge names a point that isn't in the mesh.
    UnknownEdge(EdgeKey),
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewCorners => f.write_str("a face needs at least three corners"),
            Self::UnknownVertex(v) => write!(f, "point {} is not in the mesh", v.0),
            Self::RepeatedVertex(v) => write!(f, "a face goes through point {} twice", v.0),
            Self::Full => f.write_str("the mesh is too large"),
            Self::NotFinite(v) => write!(f, "point {} has no real position", v.0),
            Self::UnknownEdge(e) => write!(
                f,
                "a hard edge joins points {} and {}, which aren't both in the mesh",
                e.0.0, e.1.0
            ),
        }
    }
}

impl std::error::Error for MeshError {}

/// A polygon mesh.
///
/// Its parts (positions, faces, hard edges) are each shared between copies
/// until one copy changes them, so copying a mesh is cheap, and an edit
/// that only moves points copies only the positions. This matters for
/// heavy meshes ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)):
/// every recorded edit works on a copy of the shape below it.
#[derive(Clone, Default)]
pub struct Mesh {
    /// Point positions by id; `None` once deleted.
    positions: Points,
    /// How many points exist.
    live_points: usize,
    /// Faces by id, as their corners in order (counter-clockwise seen from
    /// outside).
    faces: Arc<Faces>,
    /// Edges marked hard: shading breaks across them.
    hard: Arc<BTreeSet<EdgeKey>>,
}

/// Points per chunk of [`Points`].
const CHUNK: usize = 4096;

/// One chunk of point positions, with the box around them worked out when
/// first asked for.
#[derive(Default)]
struct Chunk {
    items: Vec<Option<Vec3>>,
    bounds: OnceLock<Option<Bounds>>,
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            bounds: self.bounds.clone(),
        }
    }
}

impl Chunk {
    fn bounds(&self) -> Option<Bounds> {
        *self
            .bounds
            .get_or_init(|| Bounds::around(self.items.iter().flatten().copied()))
    }
}

/// Point positions by id, in chunks shared between copies of a mesh until
/// one copy changes them: moving a few points copies only their chunks,
/// and the box around all points is worked out again only for those
/// ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
#[derive(Clone, Default)]
struct Points {
    chunks: Vec<Arc<Chunk>>,
    len: usize,
}

impl Points {
    fn new(list: Vec<Option<Vec3>>) -> Self {
        let len = list.len();
        let chunks = list
            .chunks(CHUNK)
            .map(|c| {
                Arc::new(Chunk {
                    items: c.to_vec(),
                    bounds: OnceLock::new(),
                })
            })
            .collect();
        Self { chunks, len }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&self, i: usize) -> Option<Vec3> {
        self.chunks
            .get(i / CHUNK)?
            .items
            .get(i % CHUNK)
            .copied()
            .flatten()
    }

    fn push(&mut self, p: Option<Vec3>) {
        if self.len.is_multiple_of(CHUNK) {
            self.chunks.push(Arc::new(Chunk::default()));
        }
        if let Some(last) = self.chunks.last_mut() {
            let chunk = Arc::make_mut(last);
            chunk.items.push(p);
            chunk.bounds = OnceLock::new();
        }
        self.len += 1;
    }

    /// Sets slot `i` (which must exist).
    fn set(&mut self, i: usize, p: Option<Vec3>) {
        if let Some(chunk) = self.chunks.get_mut(i / CHUNK) {
            let chunk = Arc::make_mut(chunk);
            if let Some(slot) = chunk.items.get_mut(i % CHUNK) {
                *slot = p;
                chunk.bounds = OnceLock::new();
            }
        }
    }

    fn iter(&self) -> impl ExactSizeIterator<Item = Option<Vec3>> + '_ {
        Slots {
            chunks: &self.chunks,
            at: 0,
            len: self.len,
        }
    }

    fn bounds(&self) -> Option<Bounds> {
        self.chunks
            .iter()
            .filter_map(|c| c.bounds())
            .reduce(|a, b| Bounds {
                min: a.min.min(b.min),
                max: a.max.max(b.max),
            })
    }

    /// The slots that differ from `other` (of the same length), comparing
    /// only chunks that aren't shared.
    fn differing(&self, other: &Self) -> Vec<VertexId> {
        let mut out = Vec::new();
        for (c, (a, b)) in self.chunks.iter().zip(&other.chunks).enumerate() {
            if Arc::ptr_eq(a, b) {
                continue;
            }
            for (i, (x, y)) in a.items.iter().zip(&b.items).enumerate() {
                if x != y {
                    out.push(VertexId((c * CHUNK + i) as u32));
                }
            }
        }
        out
    }

    fn memory_size(&self) -> usize {
        self.chunks
            .iter()
            .map(|c| c.items.capacity() * std::mem::size_of::<Option<Vec3>>())
            .sum::<usize>()
            + self.chunks.capacity() * std::mem::size_of::<Arc<Chunk>>()
    }
}

/// Every slot of [`Points`], in order.
struct Slots<'a> {
    chunks: &'a [Arc<Chunk>],
    at: usize,
    len: usize,
}

impl Iterator for Slots<'_> {
    type Item = Option<Vec3>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.len {
            return None;
        }
        let item = self.chunks[self.at / CHUNK].items[self.at % CHUNK];
        self.at += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.len - self.at;
        (left, Some(left))
    }
}

impl ExactSizeIterator for Slots<'_> {}

impl PartialEq for Points {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self
                .chunks
                .iter()
                .zip(&other.chunks)
                .all(|(a, b)| Arc::ptr_eq(a, b) || a.items == b.items)
    }
}

/// Every face's corners, one after another in a single list, with where
/// each face starts: far less memory than a list per face, and quick to
/// walk. A deleted face has no corners. The faces' topology is worked out
/// when first asked for and kept with them, so copies of a mesh that only
/// differ in where points are share it.
#[derive(Default)]
pub(crate) struct Faces {
    /// Where each face's corners start in `corners`; one more entry than
    /// there are face slots, the last being the end.
    starts: Vec<u32>,
    corners: Vec<VertexId>,
    /// How many faces exist.
    live: usize,
    topology: OnceLock<Topology>,
}

impl Clone for Faces {
    fn clone(&self) -> Self {
        Self {
            starts: self.starts.clone(),
            corners: self.corners.clone(),
            live: self.live,
            // Whoever copies the faces is about to change them.
            topology: OnceLock::new(),
        }
    }
}

impl Faces {
    fn slot_count(&self) -> usize {
        self.starts.len().saturating_sub(1)
    }

    fn get(&self, index: usize) -> Option<&[VertexId]> {
        let (start, end) = (*self.starts.get(index)?, *self.starts.get(index + 1)?);
        (end > start).then(|| &self.corners[start as usize..end as usize])
    }

    fn push(&mut self, corners: Option<&[VertexId]>) {
        if self.starts.is_empty() {
            self.starts.push(0);
        }
        if let Some(corners) = corners {
            self.corners.extend_from_slice(corners);
            self.live += 1;
        }
        self.starts.push(self.corners.len() as u32);
    }
}

impl PartialEq for Faces {
    fn eq(&self, other: &Self) -> bool {
        self.starts == other.starts && self.corners == other.corners
    }
}

impl PartialEq for Mesh {
    fn eq(&self, other: &Self) -> bool {
        self.positions == other.positions
            && (Arc::ptr_eq(&self.faces, &other.faces) || self.faces == other.faces)
            && (Arc::ptr_eq(&self.hard, &other.hard) || self.hard == other.hard)
    }
}

impl std::fmt::Debug for Mesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mesh")
            .field("points", &self.live_points)
            .field("faces", &self.faces.live)
            .field("hard_edges", &self.hard.len())
            .finish()
    }
}

impl Mesh {
    /// An empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a point; returns its id.
    pub fn add_vertex(&mut self, position: Vec3) -> Result<VertexId, MeshError> {
        let id = u32::try_from(self.positions.len()).map_err(|_| MeshError::Full)?;
        self.positions.push(Some(position));
        self.live_points += 1;
        Ok(VertexId(id))
    }

    /// Adds a face through `corners`, in order (counter-clockwise seen from
    /// the side it faces). Refused if it has fewer than three corners, uses
    /// an unknown point, or visits a point twice.
    pub fn add_face(&mut self, corners: &[VertexId]) -> Result<FaceId, MeshError> {
        self.check_face(corners)?;
        let id = u32::try_from(self.faces.slot_count()).map_err(|_| MeshError::Full)?;
        if u32::try_from(self.faces.corners.len() + corners.len()).is_err() {
            return Err(MeshError::Full);
        }
        Arc::make_mut(&mut self.faces).push(Some(corners));
        Ok(FaceId(id))
    }

    /// Every point slot by id number, `None` where a point was deleted
    /// (its id is never used again). For saving: [`Self::from_slots`]
    /// rebuilds the same mesh, ids included.
    pub fn point_slots(&self) -> impl ExactSizeIterator<Item = Option<Vec3>> + '_ {
        self.positions.iter()
    }

    /// How many point slots there are (points, and ids of deleted ones):
    /// one more than the highest point id.
    pub fn point_slot_count(&self) -> usize {
        self.positions.len()
    }

    /// Every face slot by id number, `None` where a face was deleted.
    pub fn face_slots(&self) -> impl ExactSizeIterator<Item = Option<&[VertexId]>> + '_ {
        (0..self.faces.slot_count()).map(|i| self.faces.get(i))
    }

    /// The edges marked hard, in order.
    pub fn hard_edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.hard.iter().copied()
    }

    /// Rebuilds a mesh from its slots (as [`Self::point_slots`] and
    /// [`Self::face_slots`] give them) and its hard edges, keeping every
    /// id. Everything is checked, as it may come from a damaged file:
    /// positions must be real numbers, faces need three or more distinct
    /// corners that exist, and hard edges must join existing points.
    pub fn from_slots(
        points: Vec<Option<Vec3>>,
        faces: Vec<Option<Vec<VertexId>>>,
        hard: impl IntoIterator<Item = EdgeKey>,
    ) -> Result<Self, MeshError> {
        if u32::try_from(points.len()).is_err() || u32::try_from(faces.len()).is_err() {
            return Err(MeshError::Full);
        }
        if let Some(i) = points
            .iter()
            .position(|p| p.is_some_and(|p| !p.is_finite()))
        {
            return Err(MeshError::NotFinite(VertexId(i as u32)));
        }
        let live_points = points.iter().flatten().count();
        let mut mesh = Self {
            positions: Points::new(points),
            live_points,
            faces: Arc::default(),
            hard: Arc::default(),
        };
        let mut table = Faces::default();
        for face in &faces {
            if let Some(corners) = face {
                mesh.check_face(corners)?;
                if u32::try_from(table.corners.len() + corners.len()).is_err() {
                    return Err(MeshError::Full);
                }
            }
            table.push(face.as_deref());
        }
        mesh.faces = Arc::new(table);
        let mut hard_edges = BTreeSet::new();
        for edge in hard {
            let (a, b) = edge.ends();
            if mesh.position(a).is_none() || mesh.position(b).is_none() || a == b {
                return Err(MeshError::UnknownEdge(edge));
            }
            hard_edges.insert(edge);
        }
        mesh.hard = Arc::new(hard_edges);
        Ok(mesh)
    }

    /// Checks a face's corners: three or more, all existing, none twice.
    fn check_face(&self, corners: &[VertexId]) -> Result<(), MeshError> {
        if corners.len() < 3 {
            return Err(MeshError::TooFewCorners);
        }
        for (i, v) in corners.iter().enumerate() {
            if self.position(*v).is_none() {
                return Err(MeshError::UnknownVertex(*v));
            }
            if corners[..i].contains(v) {
                return Err(MeshError::RepeatedVertex(*v));
            }
        }
        Ok(())
    }

    /// Adds a point, for builders that know the mesh stays small (the
    /// primitives, whose sizes are capped): the id is never reused even if
    /// the mesh were ever full.
    pub(crate) fn push_vertex(&mut self, position: Vec3) -> VertexId {
        let id = VertexId(u32::try_from(self.positions.len()).unwrap_or(u32::MAX));
        self.positions.push(Some(position));
        self.live_points += 1;
        id
    }

    /// Adds a face for builders that make only valid faces; an invalid one
    /// is skipped rather than causing a crash.
    pub(crate) fn push_face(&mut self, corners: &[VertexId]) {
        let _ = self.add_face(corners);
    }

    /// Moves a point to `position`. Returns `false` (changing nothing) if
    /// the point doesn't exist.
    pub fn set_position(&mut self, id: VertexId, position: Vec3) -> bool {
        if self.position(id).is_none() {
            return false;
        }
        self.positions.set(id.0 as usize, Some(position));
        true
    }

    /// Where a point is, if it exists.
    pub fn position(&self, id: VertexId) -> Option<Vec3> {
        self.positions.get(id.0 as usize)
    }

    /// A face's corners, in order, if it exists.
    pub fn face(&self, id: FaceId) -> Option<&[VertexId]> {
        self.faces.get(id.0 as usize)
    }

    /// Every point, with its id, in id order.
    pub fn vertices(&self) -> impl Iterator<Item = (VertexId, Vec3)> + '_ {
        self.positions
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.map(|p| (VertexId(i as u32), p)))
    }

    /// Every face, with its id, in id order.
    pub fn faces(&self) -> impl Iterator<Item = (FaceId, &[VertexId])> + '_ {
        (0..self.faces.slot_count())
            .filter_map(|i| self.faces.get(i).map(|f| (FaceId(i as u32), f)))
    }

    /// How many points there are.
    pub fn vertex_count(&self) -> usize {
        self.live_points
    }

    /// How many faces there are.
    pub fn face_count(&self) -> usize {
        self.faces.live
    }

    /// Every edge of every face, once each, in order (lower point id
    /// first). Worked out with the topology, once for these faces.
    pub fn edges(&self) -> &[EdgeKey] {
        self.topology().edge_list()
    }

    /// How the mesh's points, edges and faces connect. Worked out when
    /// first asked for, then kept with the faces: copies of the mesh that
    /// only move points share it.
    pub fn topology(&self) -> &Topology {
        self.faces.topology.get_or_init(|| Topology::new(self))
    }

    /// Whether `other` has the very same faces and hard edges as this
    /// mesh (shared, not just equal), so the two can differ only in where
    /// points are: what a recorded move leaves behind. Caches use it to
    /// update only what moved.
    pub fn same_structure(&self, other: &Mesh) -> bool {
        Arc::ptr_eq(&self.faces, &other.faces)
            && Arc::ptr_eq(&self.hard, &other.hard)
            && self.positions.len() == other.positions.len()
    }

    /// The points that are somewhere else in this mesh than in `before`,
    /// if the two have the same structure ([`Self::same_structure`]);
    /// `None` if they don't, so everything may have changed.
    pub fn moved_since(&self, before: &Mesh) -> Option<Vec<VertexId>> {
        if !self.same_structure(before) {
            return None;
        }
        Some(self.positions.differing(&before.positions))
    }

    /// The faces as one list of corners and where each face starts in it
    /// (one more start than there are face slots); a deleted face starts
    /// where it ends.
    pub fn face_layout(&self) -> (&[u32], &[VertexId]) {
        (&self.faces.starts, &self.faces.corners)
    }

    /// Marks the edge between `a` and `b` hard (shading breaks across it)
    /// or soft (shading flows over it). Edges are soft unless marked.
    pub fn set_hard(&mut self, a: VertexId, b: VertexId, hard: bool) {
        let key = EdgeKey::new(a, b);
        if hard == self.hard.contains(&key) {
            return;
        }
        let set = Arc::make_mut(&mut self.hard);
        if hard {
            set.insert(key);
        } else {
            set.remove(&key);
        }
    }

    /// Whether the edge between `a` and `b` is hard.
    pub fn is_hard(&self, a: VertexId, b: VertexId) -> bool {
        self.hard.contains(&EdgeKey::new(a, b))
    }

    /// How many edges are marked hard.
    pub fn hard_edge_count(&self) -> usize {
        self.hard.len()
    }

    /// Roughly how many bytes the mesh takes up in memory, counting what
    /// its lists hold room for. An estimate (allocator overheads aren't
    /// counted), good enough to keep the undo history within a budget.
    /// Parts shared with other copies are counted in full.
    pub fn memory_size(&self) -> usize {
        use std::mem::size_of;
        let positions = self.positions.memory_size();
        let faces = self.faces.starts.capacity() * size_of::<u32>()
            + self.faces.corners.capacity() * size_of::<VertexId>();
        // A B-tree node holds several keys plus links; twice the keys is a
        // fair estimate.
        let hard = self.hard.len() * size_of::<EdgeKey>() * 2;
        size_of::<Self>() + positions + faces + hard
    }

    /// The box around every point, or `None` for an empty mesh. Worked
    /// out once and kept until a point moves.
    pub fn bounds(&self) -> Option<Bounds> {
        self.positions.bounds()
    }

    /// The direction a face looks: unit length, worked out from all its
    /// corners so it is sensible for n-gons that aren't quite flat. Straight
    /// up for a face with no area.
    pub fn face_normal(&self, id: FaceId) -> Vec3 {
        let Some(corners) = self.face(id) else {
            return Vec3::Z;
        };
        // Newell's method: robust for any polygon.
        let mut normal = Vec3::ZERO;
        for (a, b) in sides(corners) {
            let (Some(p), Some(q)) = (self.position(a), self.position(b)) else {
                continue;
            };
            normal += Vec3::new(
                (p.y - q.y) * (p.z + q.z),
                (p.z - q.z) * (p.x + q.x),
                (p.x - q.x) * (p.y + q.y),
            );
        }
        normal.try_normalize().unwrap_or(Vec3::Z)
    }
}

/// A face's sides as pairs of corners, the last joining back to the first.
pub(crate) fn sides(corners: &[VertexId]) -> impl Iterator<Item = (VertexId, VertexId)> + '_ {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .map(|(a, b)| (*a, *b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> (Mesh, [VertexId; 4]) {
        let mut mesh = Mesh::new();
        let v = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]
        .map(|p| mesh.add_vertex(p).unwrap());
        mesh.add_face(&v).unwrap();
        (mesh, v)
    }

    #[test]
    fn faces_need_three_distinct_known_corners() {
        let (mut mesh, v) = square();
        assert_eq!(mesh.add_face(&v[..2]), Err(MeshError::TooFewCorners));
        assert_eq!(
            mesh.add_face(&[v[0], v[1], v[0]]),
            Err(MeshError::RepeatedVertex(v[0]))
        );
        let ghost = VertexId(99);
        assert_eq!(
            mesh.add_face(&[v[0], v[1], ghost]),
            Err(MeshError::UnknownVertex(ghost))
        );
        assert_eq!(mesh.face_count(), 1, "nothing added");
        assert_eq!(
            MeshError::TooFewCorners.to_string(),
            "a face needs at least three corners"
        );
    }

    #[test]
    fn ids_count_up_in_order() {
        let (mesh, v) = square();
        assert_eq!(v.map(VertexId::get), [0, 1, 2, 3]);
        assert_eq!(mesh.faces().next().unwrap().0.get(), 0);
        assert_eq!(mesh.vertex_count(), 4);
    }

    #[test]
    fn edges_are_listed_once_and_marked_hard_either_way_round() {
        let (mut mesh, v) = square();
        assert_eq!(mesh.edges().len(), 4);
        mesh.set_hard(v[1], v[0], true);
        assert!(mesh.is_hard(v[0], v[1]));
        assert_eq!(mesh.hard_edge_count(), 1);
        mesh.set_hard(v[0], v[1], false);
        assert!(!mesh.is_hard(v[1], v[0]));
    }

    #[test]
    fn a_square_facing_up_has_an_upward_normal_and_flat_bounds() {
        let (mesh, _) = square();
        let face = mesh.faces().next().unwrap().0;
        assert!((mesh.face_normal(face) - Vec3::Z).length() < 1e-6);
        let b = mesh.bounds().unwrap();
        assert_eq!(b.size(), Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn a_face_with_no_area_still_gets_a_usable_normal() {
        let mut mesh = Mesh::new();
        let v = [Vec3::ZERO, Vec3::X, Vec3::X * 2.0].map(|p| mesh.add_vertex(p).unwrap());
        let face = mesh.add_face(&v).unwrap();
        assert_eq!(mesh.face_normal(face), Vec3::Z);
        assert_eq!(mesh.face_normal(FaceId(42)), Vec3::Z, "unknown face");
    }

    #[test]
    fn points_move_only_if_they_exist() {
        let (mut mesh, v) = square();
        assert!(mesh.set_position(v[0], Vec3::splat(5.0)));
        assert_eq!(mesh.position(v[0]), Some(Vec3::splat(5.0)));
        assert!(!mesh.set_position(VertexId::from_raw(40), Vec3::ONE));
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(VertexId::from_raw(3), v[3]);
        assert_eq!(FaceId::from_raw(0).get(), 0);
    }

    #[test]
    fn memory_size_grows_with_the_mesh() {
        let empty = Mesh::new().memory_size();
        assert!(empty >= std::mem::size_of::<Mesh>());
        let (square, _) = square();
        let small = square.memory_size();
        assert!(small > empty);
        let sphere = crate::PrimitiveKind::Sphere.standard().mesh();
        assert!(
            sphere.memory_size() > small * 20,
            "many more points and faces"
        );
        // A sphere's points and faces take at least their raw size.
        let raw = sphere.vertex_count() * 12 + sphere.face_count() * 3 * 4;
        assert!(sphere.memory_size() >= raw);
    }

    #[test]
    fn a_mesh_rebuilt_from_its_slots_is_the_same_ids_and_all() {
        let mut mesh = crate::PrimitiveKind::Cylinder.standard().mesh();
        // A deleted point slot (no face uses the last point of a fresh
        // vertex, so add one and leave it out of any face).
        let extra = mesh.add_vertex(Vec3::splat(9.0)).unwrap();
        mesh.positions.set(extra.0 as usize, None);
        mesh.live_points -= 1;
        let rebuilt = Mesh::from_slots(
            mesh.point_slots().collect(),
            mesh.face_slots().map(|f| f.map(<[_]>::to_vec)).collect(),
            mesh.hard_edges(),
        )
        .unwrap();
        assert_eq!(rebuilt, mesh);
        assert_eq!(rebuilt.point_slot_count(), mesh.point_slot_count());
    }

    #[test]
    fn damaged_slots_are_refused_with_a_reason() {
        let p = |x: f32| Some(Vec3::new(x, 0.0, 0.0));
        let v = VertexId;
        let points = vec![p(0.0), p(1.0), Some(Vec3::Y), None];
        let ok = |faces: Vec<Option<Vec<VertexId>>>| {
            Mesh::from_slots(points.clone(), faces, std::iter::empty())
        };
        assert!(ok(vec![Some(vec![v(0), v(1), v(2)]), None]).is_ok());
        assert_eq!(
            ok(vec![Some(vec![v(0), v(1)])]),
            Err(MeshError::TooFewCorners)
        );
        assert_eq!(
            ok(vec![Some(vec![v(0), v(1), v(3)])]),
            Err(MeshError::UnknownVertex(v(3))),
            "a deleted point"
        );
        assert_eq!(
            ok(vec![Some(vec![v(0), v(1), v(0)])]),
            Err(MeshError::RepeatedVertex(v(0)))
        );
        assert_eq!(
            Mesh::from_slots(vec![p(0.0), Some(Vec3::NAN)], vec![], std::iter::empty()),
            Err(MeshError::NotFinite(v(1)))
        );
        let bad_edge = EdgeKey::new(v(0), v(7));
        assert_eq!(
            Mesh::from_slots(points.clone(), vec![], [bad_edge]),
            Err(MeshError::UnknownEdge(bad_edge))
        );
        let loop_edge = EdgeKey::new(v(1), v(1));
        assert!(Mesh::from_slots(points, vec![], [loop_edge]).is_err());
        assert!(MeshError::NotFinite(v(1)).to_string().contains("point 1"));
    }

    /// A strip of quads long enough to span several chunks of points.
    fn long_strip(quads: usize) -> Mesh {
        let mut mesh = Mesh::new();
        let mut last = None;
        for i in 0..=quads {
            let a = mesh.add_vertex(Vec3::new(i as f32, 0.0, 0.0)).unwrap();
            let b = mesh.add_vertex(Vec3::new(i as f32, 1.0, 0.0)).unwrap();
            if let Some((p, q)) = last {
                mesh.add_face(&[p, a, b, q]).unwrap();
            }
            last = Some((a, b));
        }
        mesh
    }

    #[test]
    fn copies_share_their_parts_until_one_changes() {
        let before = long_strip(5_000);
        assert!(before.vertex_count() > 2 * CHUNK, "several chunks");
        let mut after = before.clone();
        assert!(after.same_structure(&before));
        assert_eq!(after.moved_since(&before), Some(Vec::new()));
        // Moving points in two chunks: only those are found, and the
        // faces are still shared.
        let (a, b) = (VertexId(3), VertexId(CHUNK as u32 + 7));
        after.set_position(a, Vec3::new(0.0, 0.0, 5.0));
        after.set_position(b, Vec3::new(0.0, 0.0, -2.0));
        assert_eq!(after.moved_since(&before), Some(vec![a, b]));
        assert!(after.same_structure(&before));
        assert_ne!(after, before);
        // The box follows the moves, and the original is untouched.
        let bounds = after.bounds().unwrap();
        assert_eq!((bounds.min.z, bounds.max.z), (-2.0, 5.0));
        assert_eq!(before.bounds().unwrap().max.z, 0.0);
        // A new face makes a different structure.
        let mut grown = before.clone();
        grown
            .add_face(&[VertexId(0), VertexId(1), VertexId(3)])
            .unwrap();
        assert!(!grown.same_structure(&before));
        assert_eq!(grown.moved_since(&before), None);
        assert!(grown.topology().edge_count() > before.topology().edge_count());
    }

    #[test]
    fn slots_read_back_across_chunks_and_faces_lay_out_flat() {
        let strip = long_strip(3_000);
        let slots: Vec<Option<Vec3>> = strip.point_slots().collect();
        assert_eq!(slots.len(), strip.point_slot_count());
        assert_eq!(slots[CHUNK + 1], strip.position(VertexId(CHUNK as u32 + 1)));
        let (starts, corners) = strip.face_layout();
        assert_eq!(starts.len(), strip.face_count() + 1);
        assert_eq!(corners.len(), strip.face_count() * 4);
        assert_eq!(strip.face_slots().len(), strip.face_count());
        // A deleted face's slot reads as none, and keeps its id.
        let rebuilt = Mesh::from_slots(
            slots,
            vec![None, Some(strip.face(FaceId(1)).unwrap().to_vec())],
            std::iter::empty(),
        )
        .unwrap();
        assert_eq!(rebuilt.face(FaceId(0)), None);
        assert_eq!(rebuilt.face(FaceId(1)), strip.face(FaceId(1)));
        assert_eq!(rebuilt.face_count(), 1);
        assert_eq!(rebuilt.faces().next().unwrap().0, FaceId(1));
        assert!(format!("{rebuilt:?}").contains("faces: 1"));
    }

    #[test]
    fn an_empty_mesh_has_no_bounds() {
        assert_eq!(Mesh::new().bounds(), None);
        assert!(Mesh::new().edges().is_empty());
    }
}
