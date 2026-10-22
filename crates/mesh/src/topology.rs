//! Topology: how a mesh's points, edges and faces connect, as an index
//! built from the mesh (DESIGN.md §4.1, decision
//! [0025](../../../docs/decisions/0025-mesh-topology.md)).
//!
//! The mesh itself stores only points and faces (as corner lists), which
//! can describe anything, including the awkward cases: n-gons, open
//! borders, edges shared by three or more faces, loose points. This index
//! answers the questions selection and modelling tools ask: which faces
//! use an edge, which edges meet at a point, which faces are neighbours,
//! and how edge loops and rings run. It never changes the mesh. It
//! depends only on the faces, so the mesh keeps it with them
//! ([`Mesh::topology`]): moving points doesn't need it worked out again.
//!
//! Edges are named by their two points ([`EdgeKey`]), so an edge's name
//! is as stable as its points' ids.

use crate::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::BTreeSet;

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

/// How a mesh's elements connect. Built by [`Topology::new`], or kept with
/// the mesh by [`Mesh::topology`].
///
/// Everything is stored in flat lists, each element's neighbours one after
/// another with where each element's start, so even a mesh of millions of
/// polygons is indexed in well under a second
/// ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
#[derive(Debug, Clone, Default)]
pub struct Topology {
    /// Every edge, in order.
    edges: Vec<EdgeKey>,
    /// Where each edge's uses start in `uses` (one more than edges).
    use_starts: Vec<u32>,
    /// The faces using each edge, in face id order.
    uses: Vec<EdgeUse>,
    /// Where each point's edges start in `vertex_edges`, by point id.
    vertex_edge_starts: Vec<u32>,
    /// The edges meeting at each point, in the order the faces first use
    /// them.
    vertex_edges: Vec<EdgeKey>,
    /// Where each point's faces start in `vertex_faces`, by point id.
    vertex_face_starts: Vec<u32>,
    /// The faces using each point, in face id order.
    vertex_faces: Vec<FaceId>,
    /// Where each face's edges start in `face_edges`, by face id (the
    /// mesh's own face starts).
    face_starts: Vec<u32>,
    /// Each face's edges, in corner order (the edge from corner `i` to
    /// corner `i + 1` first).
    face_edges: Vec<EdgeKey>,
}

/// Turns per-item counts into where each item's entries start (one more
/// entry than items, the last being the total).
fn starts_from_counts(counts: &[u32]) -> Vec<u32> {
    let mut starts = Vec::with_capacity(counts.len() + 1);
    let mut total = 0u32;
    starts.push(0);
    for c in counts {
        total += c;
        starts.push(total);
    }
    starts
}

/// The entries between `starts[i]` and `starts[i + 1]`, or none.
fn slice_at<'a, T>(starts: &[u32], items: &'a [T], i: usize) -> &'a [T] {
    match (starts.get(i), starts.get(i + 1)) {
        (Some(&a), Some(&b)) => &items[a as usize..b as usize],
        _ => &[],
    }
}

impl Topology {
    /// The topology of `mesh`. Prefer [`Mesh::topology`], which works it
    /// out once and keeps it.
    pub fn new(mesh: &Mesh) -> Self {
        let (face_starts, corners) = mesh.face_layout();
        let face_starts = face_starts.to_vec();
        let points = mesh.point_slot_count();
        let sides = corners.len();
        // Each corner starts a side running to the next corner.
        let mut side_face = vec![0u32; sides];
        let mut side_next = vec![0u32; sides];
        for f in 0..face_starts.len().saturating_sub(1) {
            let (a, b) = (face_starts[f] as usize, face_starts[f + 1] as usize);
            for k in a..b {
                side_face[k] = f as u32;
                side_next[k] = if k + 1 < b { k + 1 } else { a } as u32;
            }
        }
        let ends = |k: usize| (corners[k], corners[side_next[k] as usize]);
        // Group the sides by their lower point, then by their higher one:
        // each group is one edge, and edges come out in order.
        let mut low_counts = vec![0u32; points];
        for k in 0..sides {
            let (a, b) = ends(k);
            low_counts[a.min(b).get() as usize] += 1;
        }
        let low_starts = starts_from_counts(&low_counts);
        let mut fill = low_starts.clone();
        let mut by_low = vec![(0u32, 0u32); sides];
        for k in 0..sides {
            let (a, b) = ends(k);
            let low = a.min(b).get() as usize;
            by_low[fill[low] as usize] = (a.max(b).get(), k as u32);
            fill[low] += 1;
        }
        let mut edges = Vec::new();
        let mut use_starts = vec![0u32];
        let mut uses = Vec::with_capacity(sides);
        let mut side_edge = vec![0u32; sides];
        let mut first_side = Vec::new();
        for low in 0..points {
            let bucket = &mut by_low[low_starts[low] as usize..low_starts[low + 1] as usize];
            // Stable, so each edge's sides stay in face order.
            bucket.sort_by_key(|(high, _)| *high);
            let mut i = 0;
            while i < bucket.len() {
                let high = bucket[i].0;
                let index = edges.len() as u32;
                edges.push(EdgeKey::new(
                    VertexId::from_raw(low as u32),
                    VertexId::from_raw(high),
                ));
                first_side.push(bucket[i].1);
                while i < bucket.len() && bucket[i].0 == high {
                    let k = bucket[i].1 as usize;
                    let (a, b) = ends(k);
                    side_edge[k] = index;
                    uses.push(EdgeUse {
                        face: FaceId::from_raw(side_face[k]),
                        forward: a < b,
                    });
                    i += 1;
                }
                use_starts.push(uses.len() as u32);
            }
        }
        // Edges at each point, in the order faces first use them.
        let mut counts = vec![0u32; points];
        for edge in &edges {
            let (a, b) = edge.ends();
            counts[a.get() as usize] += 1;
            counts[b.get() as usize] += 1;
        }
        let vertex_edge_starts = starts_from_counts(&counts);
        let mut fill = vertex_edge_starts.clone();
        let mut vertex_edges =
            vec![EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(0)); fill[points] as usize];
        for (k, e) in side_edge.iter().enumerate() {
            let e = *e as usize;
            if first_side[e] as usize != k {
                continue;
            }
            let (a, b) = ends(k);
            for v in [a, b] {
                let at = &mut fill[v.get() as usize];
                vertex_edges[*at as usize] = edges[e];
                *at += 1;
            }
        }
        // Faces at each point, in face order.
        let mut counts = vec![0u32; points];
        for v in corners {
            counts[v.get() as usize] += 1;
        }
        let vertex_face_starts = starts_from_counts(&counts);
        let mut fill = vertex_face_starts.clone();
        let mut vertex_faces = vec![FaceId::from_raw(0); sides];
        for (k, v) in corners.iter().enumerate() {
            let at = &mut fill[v.get() as usize];
            vertex_faces[*at as usize] = FaceId::from_raw(side_face[k]);
            *at += 1;
        }
        let face_edges = side_edge.iter().map(|e| edges[*e as usize]).collect();
        Self {
            edges,
            use_starts,
            uses,
            vertex_edge_starts,
            vertex_edges,
            vertex_face_starts,
            vertex_faces,
            face_starts,
            face_edges,
        }
    }

    /// Where `edge` is in the edge list.
    fn edge_index(&self, edge: EdgeKey) -> Option<usize> {
        self.edges.binary_search(&edge).ok()
    }

    /// Roughly how many bytes the topology of a mesh with this many point
    /// slots, face slots and corners will take, before it is worked out:
    /// one edge per two corners (a closed surface; open ones have a few
    /// more), every corner used once by each list that follows corners.
    pub(crate) fn estimate(points: usize, faces: usize, corners: usize) -> usize {
        use std::mem::size_of;
        let edges = corners / 2 + 1;
        size_of::<Self>()
            + edges * (size_of::<EdgeKey>() + size_of::<u32>())
            + corners * size_of::<EdgeUse>()
            + (points + 1) * 2 * size_of::<u32>()
            + edges * 2 * size_of::<EdgeKey>()
            + corners * size_of::<FaceId>()
            + (faces + 1) * size_of::<u32>()
            + corners * size_of::<EdgeKey>()
    }

    /// Roughly how many bytes the topology takes up: what its lists hold
    /// room for. On a big mesh it is several times the mesh itself.
    pub fn memory_size(&self) -> usize {
        use std::mem::size_of;
        size_of::<Self>()
            + self.edges.capacity() * size_of::<EdgeKey>()
            + self.use_starts.capacity() * size_of::<u32>()
            + self.uses.capacity() * size_of::<EdgeUse>()
            + self.vertex_edge_starts.capacity() * size_of::<u32>()
            + self.vertex_edges.capacity() * size_of::<EdgeKey>()
            + self.vertex_face_starts.capacity() * size_of::<u32>()
            + self.vertex_faces.capacity() * size_of::<FaceId>()
            + self.face_starts.capacity() * size_of::<u32>()
            + self.face_edges.capacity() * size_of::<EdgeKey>()
    }

    /// Every edge, in order.
    pub fn edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.edges.iter().copied()
    }

    /// Every edge, in order, as a list.
    pub fn edge_list(&self) -> &[EdgeKey] {
        &self.edges
    }

    /// How many edges there are.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Whether `edge` is an edge of some face.
    pub fn has_edge(&self, edge: EdgeKey) -> bool {
        self.edge_index(edge).is_some()
    }

    /// The faces using `edge`, and which way each runs along it. Empty
    /// for an edge no face uses.
    pub fn edge_uses(&self, edge: EdgeKey) -> &[EdgeUse] {
        self.edge_index(edge)
            .map_or(&[], |i| slice_at(&self.use_starts, &self.uses, i))
    }

    /// The faces using `edge`.
    pub fn edge_faces(&self, edge: EdgeKey) -> impl Iterator<Item = FaceId> + '_ {
        self.edge_uses(edge).iter().map(|u| u.face)
    }

    /// The edges meeting at `vertex`. Empty for a loose point.
    pub fn vertex_edges(&self, vertex: VertexId) -> &[EdgeKey] {
        slice_at(
            &self.vertex_edge_starts,
            &self.vertex_edges,
            vertex.get() as usize,
        )
    }

    /// The faces using `vertex`. Empty for a loose point.
    pub fn vertex_faces(&self, vertex: VertexId) -> &[FaceId] {
        slice_at(
            &self.vertex_face_starts,
            &self.vertex_faces,
            vertex.get() as usize,
        )
    }

    /// The edges of `face`, in corner order. Empty for an unknown face.
    pub fn face_edges(&self, face: FaceId) -> &[EdgeKey] {
        slice_at(&self.face_starts, &self.face_edges, face.get() as usize)
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
        (0..self.edges.len())
            .filter(|i| {
                let uses = slice_at(&self.use_starts, &self.uses, *i);
                uses.len() == 2 && uses[0].forward == uses[1].forward
            })
            .map(|i| self.edges[i])
            .collect()
    }

    /// The mesh's separate pieces: faces linked through shared points,
    /// each piece's faces in id order, pieces ordered by their first face.
    pub fn pieces(&self) -> Vec<Vec<FaceId>> {
        let slots = self.face_starts.len().saturating_sub(1);
        let mut seen = vec![false; slots];
        let mut pieces = Vec::new();
        for index in 0..slots {
            let face = FaceId::from_raw(index as u32);
            if seen[index] || self.face_edges(face).is_empty() {
                continue;
            }
            seen[index] = true;
            let mut piece = vec![face];
            let mut stack = vec![face];
            while let Some(at) = stack.pop() {
                for edge in self.face_edges(at) {
                    let (a, b) = edge.ends();
                    for v in [a, b] {
                        for next in self.vertex_faces(v) {
                            let i = next.get() as usize;
                            if !seen[i] {
                                seen[i] = true;
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
    /// mesh), along the open border when `edge` is on one, or round the
    /// rim of the one n-gon it borders (a cylinder's cap) where it can't go
    /// straight on. Otherwise it stops at poles (points with more or fewer
    /// than four edges), n-gons, triangles and borders. Starts with `edge`, then runs one way and,
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
                    .or_else(|| self.next_around_n_gon(from, at))
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

    /// Round the rim of an n-gon (a polygon of five or more sides) `from`
    /// lies on, where the loop can't go straight on through quads: the
    /// n-gon's other edge at `at`. A cylinder's cap is one n-gon, so a
    /// double-click on its rim selects the rim, as an open border's does.
    /// Only when `from` borders exactly one n-gon.
    fn next_around_n_gon(&self, from: EdgeKey, at: VertexId) -> Option<EdgeKey> {
        let mut n_gons = self
            .edge_faces(from)
            .filter(|f| self.face_edges(*f).len() > 4);
        let n_gon = n_gons.next()?;
        if n_gons.next().is_some() {
            return None;
        }
        let mut others = self.face_edges(n_gon).iter().filter(|e| {
            **e != from && {
                let (a, b) = e.ends();
                a == at || b == at
            }
        });
        let found = *others.next()?;
        others.next().is_none().then_some(found)
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
