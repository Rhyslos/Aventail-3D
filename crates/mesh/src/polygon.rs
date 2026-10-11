//! The polygon mesh: points in space joined into faces with any number of
//! corners (n-gons), with each edge marked hard or soft.
//!
//! Every point and face has an id that stays the same for its whole life
//! and is never given to another element, even after it is deleted. Ids
//! are handed out in order, so building the same mesh the same way always
//! gives the same ids (DESIGN.md §4.1–4.2).
//!
//! Full adjacency (walking from an edge to its faces, loops and rings)
//! arrives with mesh editing in Phase 3; this is the part primitives and
//! drawing need.

use crate::Bounds;
use glam::Vec3;
use std::collections::BTreeSet;

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
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    /// Point positions by id; `None` once deleted.
    positions: Vec<Option<Vec3>>,
    /// Faces by id, as their corners in order (counter-clockwise seen from
    /// outside); `None` once deleted.
    faces: Vec<Option<Vec<VertexId>>>,
    /// Edges marked hard: shading breaks across them.
    hard: BTreeSet<EdgeKey>,
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
        Ok(VertexId(id))
    }

    /// Adds a face through `corners`, in order (counter-clockwise seen from
    /// the side it faces). Refused if it has fewer than three corners, uses
    /// an unknown point, or visits a point twice.
    pub fn add_face(&mut self, corners: &[VertexId]) -> Result<FaceId, MeshError> {
        self.check_face(corners)?;
        let id = u32::try_from(self.faces.len()).map_err(|_| MeshError::Full)?;
        self.faces.push(Some(corners.to_vec()));
        Ok(FaceId(id))
    }

    /// Every point slot by id number, `None` where a point was deleted
    /// (its id is never used again). For saving: [`Self::from_slots`]
    /// rebuilds the same mesh, ids included.
    pub fn point_slots(&self) -> &[Option<Vec3>] {
        &self.positions
    }

    /// Every face slot by id number, `None` where a face was deleted.
    pub fn face_slots(&self) -> &[Option<Vec<VertexId>>] {
        &self.faces
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
        let mut mesh = Self {
            positions: points,
            faces: Vec::new(),
            hard: BTreeSet::new(),
        };
        let mut slots = Vec::with_capacity(faces.len());
        for face in faces {
            if let Some(corners) = &face {
                mesh.check_face(corners)?;
            }
            slots.push(face);
        }
        mesh.faces = slots;
        for edge in hard {
            let (a, b) = edge.ends();
            if mesh.position(a).is_none() || mesh.position(b).is_none() || a == b {
                return Err(MeshError::UnknownEdge(edge));
            }
            mesh.hard.insert(edge);
        }
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
        match self.positions.get_mut(id.0 as usize) {
            Some(Some(p)) => {
                *p = position;
                true
            }
            _ => false,
        }
    }

    /// Where a point is, if it exists.
    pub fn position(&self, id: VertexId) -> Option<Vec3> {
        self.positions.get(id.0 as usize).copied().flatten()
    }

    /// A face's corners, in order, if it exists.
    pub fn face(&self, id: FaceId) -> Option<&[VertexId]> {
        self.faces.get(id.0 as usize)?.as_deref()
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
        self.faces
            .iter()
            .enumerate()
            .filter_map(|(i, f)| f.as_deref().map(|f| (FaceId(i as u32), f)))
    }

    /// How many points there are.
    pub fn vertex_count(&self) -> usize {
        self.positions.iter().flatten().count()
    }

    /// How many faces there are.
    pub fn face_count(&self) -> usize {
        self.faces.iter().flatten().count()
    }

    /// Every edge of every face, once each, in order of first appearance.
    pub fn edges(&self) -> Vec<EdgeKey> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for (_, corners) in self.faces() {
            for (a, b) in sides(corners) {
                let key = EdgeKey::new(a, b);
                if seen.insert(key) {
                    out.push(key);
                }
            }
        }
        out
    }

    /// Marks the edge between `a` and `b` hard (shading breaks across it)
    /// or soft (shading flows over it). Edges are soft unless marked.
    pub fn set_hard(&mut self, a: VertexId, b: VertexId, hard: bool) {
        let key = EdgeKey::new(a, b);
        if hard {
            self.hard.insert(key);
        } else {
            self.hard.remove(&key);
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
    pub fn memory_size(&self) -> usize {
        use std::mem::size_of;
        let positions = self.positions.capacity() * size_of::<Option<Vec3>>();
        let faces = self.faces.capacity() * size_of::<Option<Vec<VertexId>>>()
            + self
                .faces
                .iter()
                .flatten()
                .map(|f| f.capacity() * size_of::<VertexId>())
                .sum::<usize>();
        // A B-tree node holds several keys plus links; twice the keys is a
        // fair estimate.
        let hard = self.hard.len() * size_of::<EdgeKey>() * 2;
        size_of::<Self>() + positions + faces + hard
    }

    /// The box around every point, or `None` for an empty mesh.
    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::around(self.vertices().map(|(_, p)| p))
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
        assert_eq!(empty, std::mem::size_of::<Mesh>());
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
        mesh.positions[extra.0 as usize] = None;
        let rebuilt = Mesh::from_slots(
            mesh.point_slots().to_vec(),
            mesh.face_slots().to_vec(),
            mesh.hard_edges(),
        )
        .unwrap();
        assert_eq!(rebuilt, mesh);
        assert_eq!(rebuilt.point_slots().len(), mesh.point_slots().len());
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

    #[test]
    fn an_empty_mesh_has_no_bounds() {
        assert_eq!(Mesh::new().bounds(), None);
        assert!(Mesh::new().edges().is_empty());
    }
}
