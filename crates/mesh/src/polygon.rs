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
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewCorners => f.write_str("a face needs at least three corners"),
            Self::UnknownVertex(v) => write!(f, "point {} is not in the mesh", v.0),
            Self::RepeatedVertex(v) => write!(f, "a face goes through point {} twice", v.0),
            Self::Full => f.write_str("the mesh is too large"),
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
        let id = u32::try_from(self.faces.len()).map_err(|_| MeshError::Full)?;
        self.faces.push(Some(corners.to_vec()));
        Ok(FaceId(id))
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
    fn an_empty_mesh_has_no_bounds() {
        assert_eq!(Mesh::new().bounds(), None);
        assert!(Mesh::new().edges().is_empty());
    }
}
