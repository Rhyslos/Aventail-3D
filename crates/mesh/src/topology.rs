//! Topology: how a mesh's points, edges and faces connect, as an index
//! built from the mesh (DESIGN.md §4.1, decision
//! [0025](../../../docs/decisions/0025-mesh-topology.md)).
//!
//! The mesh itself stores only points and faces (as corner lists), which
//! can describe anything, including the awkward cases: n-gons, open
//! borders, edges shared by three or more faces, loose points. This index
//! answers the questions selection and modelling tools ask: which faces
//! use an edge, which edges meet at a point, which faces are neighbours,
//! and how edge loops and rings run. It is built in one pass and never
//! changes the mesh; build it again after the mesh changes.
//!
//! Edges are named by their two points ([`EdgeKey`]), so an edge's name
//! is as stable as its points' ids.

use crate::polygon::sides;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeMap, BTreeSet};

/// A face's use of an edge: which face, and whether it runs along the
/// edge from its lower point to its higher one (the edge's "forward"
/// direction) or the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeUse {
    /// The face.
    pub face: FaceId,
    /// Whether the face goes from the edge's lower point id to its higher.
    pub forward: bool,
}

/// How a mesh's elements connect. Built by [`Topology::new`].
#[derive(Debug, Clone, Default)]
pub struct Topology {
    /// Every edge with the faces using it, in face id order.
    edges: BTreeMap<EdgeKey, Vec<EdgeUse>>,
    /// Every point with the edges meeting there, in edge order.
    vertex_edges: BTreeMap<VertexId, Vec<EdgeKey>>,
    /// Every point with the faces using it, in face id order.
    vertex_faces: BTreeMap<VertexId, Vec<FaceId>>,
    /// Every face with its edges, in corner order (the edge from corner
    /// `i` to corner `i + 1` first).
    face_edges: BTreeMap<FaceId, Vec<EdgeKey>>,
}

impl Topology {
    /// The topology of `mesh`.
    pub fn new(mesh: &Mesh) -> Self {
        let mut topology = Self::default();
        for (face, corners) in mesh.faces() {
            let mut edges = Vec::with_capacity(corners.len());
            for (a, b) in sides(corners) {
                let key = EdgeKey::new(a, b);
                let uses = topology.edges.entry(key).or_default();
                if uses.is_empty() {
                    topology.vertex_edges.entry(a).or_default().push(key);
                    topology.vertex_edges.entry(b).or_default().push(key);
                }
                uses.push(EdgeUse {
                    face,
                    forward: a < b,
                });
                edges.push(key);
            }
            for v in corners {
                topology.vertex_faces.entry(*v).or_default().push(face);
            }
            topology.face_edges.insert(face, edges);
        }
        topology
    }

    /// Every edge, in order.
    pub fn edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.edges.keys().copied()
    }

    /// How many edges there are.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Whether `edge` is an edge of some face.
    pub fn has_edge(&self, edge: EdgeKey) -> bool {
        self.edges.contains_key(&edge)
    }

    /// The faces using `edge`, and which way each runs along it. Empty
    /// for an edge no face uses.
    pub fn edge_uses(&self, edge: EdgeKey) -> &[EdgeUse] {
        self.edges.get(&edge).map_or(&[], Vec::as_slice)
    }

    /// The faces using `edge`.
    pub fn edge_faces(&self, edge: EdgeKey) -> impl Iterator<Item = FaceId> + '_ {
        self.edge_uses(edge).iter().map(|u| u.face)
    }

    /// The edges meeting at `vertex`. Empty for a loose point.
    pub fn vertex_edges(&self, vertex: VertexId) -> &[EdgeKey] {
        self.vertex_edges.get(&vertex).map_or(&[], Vec::as_slice)
    }

    /// The faces using `vertex`. Empty for a loose point.
    pub fn vertex_faces(&self, vertex: VertexId) -> &[FaceId] {
        self.vertex_faces.get(&vertex).map_or(&[], Vec::as_slice)
    }

    /// The edges of `face`, in corner order. Empty for an unknown face.
    pub fn face_edges(&self, face: FaceId) -> &[EdgeKey] {
        self.face_edges.get(&face).map_or(&[], Vec::as_slice)
    }

    /// The faces sharing an edge with `face`, each once, in id order.
    pub fn face_neighbours(&self, face: FaceId) -> Vec<FaceId> {
        let found: BTreeSet<FaceId> = self
            .face_edges(face)
            .iter()
            .flat_map(|e| self.edge_faces(*e))
            .filter(|f| *f != face)
            .collect();
        found.into_iter().collect()
    }

    /// The points joined to `vertex` by an edge.
    pub fn vertex_neighbours(&self, vertex: VertexId) -> Vec<VertexId> {
        self.vertex_edges(vertex)
            .iter()
            .map(|e| other_end(*e, vertex))
            .collect()
    }

    /// Whether `edge` is on an open border: used by exactly one face.
    pub fn is_border(&self, edge: EdgeKey) -> bool {
        self.edge_uses(edge).len() == 1
    }

    /// Whether `edge` is shared by more than two faces, which no real
    /// solid could have ("non-manifold").
    pub fn is_non_manifold(&self, edge: EdgeKey) -> bool {
        self.edge_uses(edge).len() > 2
    }

    /// The edges on open borders.
    pub fn border_edges(&self) -> Vec<EdgeKey> {
        self.edges().filter(|e| self.is_border(*e)).collect()
    }

    /// The edges shared by more than two faces.
    pub fn non_manifold_edges(&self) -> Vec<EdgeKey> {
        self.edges().filter(|e| self.is_non_manifold(*e)).collect()
    }

    /// Edges where two faces meet but run the same way along it, so one
    /// of them faces the opposite way to its neighbour (its "normal is
    /// flipped").
    pub fn flipped_edges(&self) -> Vec<EdgeKey> {
        self.edges
            .iter()
            .filter(|(_, uses)| uses.len() == 2 && uses[0].forward == uses[1].forward)
            .map(|(e, _)| *e)
            .collect()
    }

    /// The mesh's separate pieces: faces linked through shared points,
    /// each piece's faces in id order, pieces ordered by their first face.
    pub fn pieces(&self) -> Vec<Vec<FaceId>> {
        let mut seen = BTreeSet::new();
        let mut pieces = Vec::new();
        for face in self.face_edges.keys() {
            if !seen.insert(*face) {
                continue;
            }
            let mut piece = vec![*face];
            let mut stack = vec![*face];
            while let Some(at) = stack.pop() {
                for edge in self.face_edges(at) {
                    let (a, b) = edge.ends();
                    for v in [a, b] {
                        for next in self.vertex_faces(v) {
                            if seen.insert(*next) {
                                piece.push(*next);
                                stack.push(*next);
                            }
                        }
                    }
                }
            }
            piece.sort();
            pieces.push(piece);
        }
        pieces
    }

    /// Whether `face` is a quad: four corners.
    fn is_quad(&self, face: FaceId) -> bool {
        self.face_edges(face).len() == 4
    }

    /// The edge loop through `edge`: the chain of edges running straight
    /// on through points where four quads meet (like a belt around the
    /// mesh), or along the open border when `edge` is on one. It stops at
    /// poles (points with more or fewer than four edges), n-gons,
    /// triangles and borders. Starts with `edge`, then runs one way and,
    /// if the loop isn't closed, the other way, each in walking order.
    /// Just `edge` when it can't continue; empty for an unknown edge.
    pub fn edge_loop(&self, edge: EdgeKey) -> Vec<EdgeKey> {
        if !self.has_edge(edge) {
            return Vec::new();
        }
        let next = |from: EdgeKey, at: VertexId| {
            if self.is_border(from) {
                self.next_along_border(from, at)
            } else {
                self.next_through_quads(from, at)
            }
        };
        let (a, b) = edge.ends();
        let mut out = vec![edge];
        let mut seen = BTreeSet::from([edge]);
        let mut closed = false;
        for start in [b, a] {
            let (mut from, mut at) = (edge, start);
            while let Some(step) = next(from, at) {
                if step == edge {
                    closed = true;
                    break;
                }
                if !seen.insert(step) {
                    break;
                }
                out.push(step);
                at = other_end(step, at);
                from = step;
            }
            if closed {
                break;
            }
        }
        out
    }

    /// Straight on through `at`: the edge opposite `from` where four quads
    /// meet around an inside point.
    fn next_through_quads(&self, from: EdgeKey, at: VertexId) -> Option<EdgeKey> {
        let around = self.vertex_edges(at);
        if around.len() != 4 || self.vertex_faces(at).len() != 4 {
            return None;
        }
        if around.iter().any(|e| self.edge_uses(*e).len() != 2) {
            return None;
        }
        if self.vertex_faces(at).iter().any(|f| !self.is_quad(*f)) {
            return None;
        }
        // The one edge at `at` sharing no face with `from`.
        let beside: BTreeSet<FaceId> = self.edge_faces(from).collect();
        let mut candidates = around
            .iter()
            .filter(|e| **e != from && !self.edge_faces(**e).any(|f| beside.contains(&f)));
        let found = *candidates.next()?;
        candidates.next().is_none().then_some(found)
    }

    /// Along the open border through `at`: the other border edge there,
    /// when there is exactly one.
    fn next_along_border(&self, from: EdgeKey, at: VertexId) -> Option<EdgeKey> {
        let mut others = self
            .vertex_edges(at)
            .iter()
            .filter(|e| **e != from && self.is_border(**e));
        let found = *others.next()?;
        others.next().is_none().then_some(found)
    }

    /// The edge ring through `edge`: the edges across a strip of quads,
    /// like the rungs of a ladder (each quad's edge opposite the one
    /// before). It stops at faces that aren't quads and at borders.
    /// Starts with `edge`, then runs one way and, if the ring isn't
    /// closed, the other way. Empty for an unknown edge.
    pub fn edge_ring(&self, edge: EdgeKey) -> Vec<EdgeKey> {
        let uses = self.edge_uses(edge);
        if uses.is_empty() {
            return Vec::new();
        }
        let mut out = vec![edge];
        let mut seen = BTreeSet::from([edge]);
        let faces: Vec<FaceId> = uses.iter().take(2).map(|u| u.face).collect();
        for start in faces {
            let (mut from, mut through) = (edge, start);
            loop {
                if self.is_non_manifold(from) || !self.is_quad(through) {
                    break;
                }
                let Some(opposite) = self.opposite_edge(through, from) else {
                    break;
                };
                if opposite == edge {
                    return out;
                }
                if !seen.insert(opposite) {
                    break;
                }
                out.push(opposite);
                let Some(next) = self.edge_faces(opposite).find(|f| *f != through) else {
                    break;
                };
                from = opposite;
                through = next;
            }
        }
        out
    }

    /// In a quad, the edge opposite `edge`.
    fn opposite_edge(&self, quad: FaceId, edge: EdgeKey) -> Option<EdgeKey> {
        let edges = self.face_edges(quad);
        let at = edges.iter().position(|e| *e == edge)?;
        edges.get((at + 2) % edges.len()).copied()
    }
}

/// The end of `edge` that isn't `vertex`.
pub fn other_end(edge: EdgeKey, vertex: VertexId) -> VertexId {
    let (a, b) = edge.ends();
    if a == vertex { b } else { a }
}

#[cfg(test)]
mod tests;
