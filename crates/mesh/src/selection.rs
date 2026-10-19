//! Selections of a mesh's elements (points, edges and polygons), and the
//! commands that work them out from the mesh's connections: select all,
//! invert, grow, shrink, loops and rings
//! ([decision 0029](../../../docs/decisions/0029-selection.md)).
//!
//! A selection only names elements by their IDs, so it stays valid while
//! points move. When the mesh's shape changes (a primitive's segments,
//! say), IDs that no longer exist are simply ignored: see
//! [`ElementSelection::valid_in`].

use crate::topology::Topology;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::BTreeSet;

/// A kind of element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementKind {
    /// Points (vertices).
    Points,
    /// Edges.
    Edges,
    /// Polygons (faces).
    Polygons,
}

impl ElementKind {
    /// The plural name, for messages: "points", "edges", "polygons".
    pub fn plural(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Edges => "edges",
            Self::Polygons => "polygons",
        }
    }

    /// The name of one, for messages: "point", "edge", "polygon".
    pub fn singular(self) -> &'static str {
        match self {
            Self::Points => "point",
            Self::Edges => "edge",
            Self::Polygons => "polygon",
        }
    }
}

/// One element of a mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Element {
    /// A point.
    Point(VertexId),
    /// An edge, named by its points.
    Edge(EdgeKey),
    /// A polygon.
    Polygon(FaceId),
}

impl Element {
    /// Which kind of element this is.
    pub fn kind(self) -> ElementKind {
        match self {
            Self::Point(_) => ElementKind::Points,
            Self::Edge(_) => ElementKind::Edges,
            Self::Polygon(_) => ElementKind::Polygons,
        }
    }
}

/// How new elements combine with what is already selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combine {
    /// Only the new elements are selected (a plain click or drag).
    Replace,
    /// The new elements are added (Shift + drag).
    Add,
    /// The new elements are unselected (Ctrl + drag).
    Remove,
    /// Each new element flips: selected ones are unselected and the
    /// others selected (Ctrl + click).
    Toggle,
}

/// The selected points, edges and polygons of one mesh. The three kinds
/// are kept apart, so switching between the Points, Edges and Polygons
/// modes keeps each mode's selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ElementSelection {
    points: BTreeSet<VertexId>,
    edges: BTreeSet<EdgeKey>,
    polygons: BTreeSet<FaceId>,
}

impl ElementSelection {
    /// Nothing selected.
    pub fn new() -> Self {
        Self::default()
    }

    /// A selection holding `elements`.
    pub fn of(elements: impl IntoIterator<Item = Element>) -> Self {
        let mut selection = Self::new();
        for element in elements {
            selection.insert(element);
        }
        selection
    }

    /// Whether nothing of any kind is selected.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty() && self.edges.is_empty() && self.polygons.is_empty()
    }

    /// Whether the element is selected.
    pub fn contains(&self, element: Element) -> bool {
        match element {
            Element::Point(v) => self.points.contains(&v),
            Element::Edge(e) => self.edges.contains(&e),
            Element::Polygon(f) => self.polygons.contains(&f),
        }
    }

    /// Selects the element. Returns whether it wasn't selected before.
    pub fn insert(&mut self, element: Element) -> bool {
        match element {
            Element::Point(v) => self.points.insert(v),
            Element::Edge(e) => self.edges.insert(e),
            Element::Polygon(f) => self.polygons.insert(f),
        }
    }

    /// Unselects the element. Returns whether it was selected.
    pub fn remove(&mut self, element: Element) -> bool {
        match element {
            Element::Point(v) => self.points.remove(&v),
            Element::Edge(e) => self.edges.remove(&e),
            Element::Polygon(f) => self.polygons.remove(&f),
        }
    }

    /// How many elements of `kind` are selected.
    pub fn count(&self, kind: ElementKind) -> usize {
        match kind {
            ElementKind::Points => self.points.len(),
            ElementKind::Edges => self.edges.len(),
            ElementKind::Polygons => self.polygons.len(),
        }
    }

    /// How many elements are selected, of every kind together.
    pub fn len(&self) -> usize {
        self.points.len() + self.edges.len() + self.polygons.len()
    }

    /// The selected elements of `kind`, in ID order.
    pub fn of_kind(&self, kind: ElementKind) -> Vec<Element> {
        match kind {
            ElementKind::Points => self.points.iter().map(|v| Element::Point(*v)).collect(),
            ElementKind::Edges => self.edges.iter().map(|e| Element::Edge(*e)).collect(),
            ElementKind::Polygons => self.polygons.iter().map(|f| Element::Polygon(*f)).collect(),
        }
    }

    /// The selected points.
    pub fn points(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.points.iter().copied()
    }

    /// The selected edges.
    pub fn edges(&self) -> impl Iterator<Item = EdgeKey> + '_ {
        self.edges.iter().copied()
    }

    /// The selected polygons.
    pub fn polygons(&self) -> impl Iterator<Item = FaceId> + '_ {
        self.polygons.iter().copied()
    }

    /// Unselects every element of `kind`, leaving the other kinds.
    pub fn clear_kind(&mut self, kind: ElementKind) {
        match kind {
            ElementKind::Points => self.points.clear(),
            ElementKind::Edges => self.edges.clear(),
            ElementKind::Polygons => self.polygons.clear(),
        }
    }

    /// Combines `elements` with the selection of `kind` (elements of other
    /// kinds are ignored). The other kinds are left alone.
    pub fn combine(
        &mut self,
        kind: ElementKind,
        elements: impl IntoIterator<Item = Element>,
        how: Combine,
    ) {
        if how == Combine::Replace {
            self.clear_kind(kind);
        }
        for element in elements.into_iter().filter(|e| e.kind() == kind) {
            match how {
                Combine::Replace | Combine::Add => {
                    self.insert(element);
                }
                Combine::Remove => {
                    self.remove(element);
                }
                Combine::Toggle => {
                    if !self.remove(element) {
                        self.insert(element);
                    }
                }
            }
        }
    }

    /// The selection with only the elements that exist in `mesh`.
    pub fn valid_in(&self, mesh: &Mesh, topology: &Topology) -> Self {
        Self {
            points: self
                .points
                .iter()
                .copied()
                .filter(|v| mesh.position(*v).is_some())
                .collect(),
            edges: self
                .edges
                .iter()
                .copied()
                .filter(|e| topology.has_edge(*e))
                .collect(),
            polygons: self
                .polygons
                .iter()
                .copied()
                .filter(|f| mesh.face(*f).is_some())
                .collect(),
        }
    }

    /// Roughly how many bytes the selection takes, for the undo budget.
    pub fn memory_size(&self) -> usize {
        // A B-tree entry costs about its key plus some bookkeeping.
        const OVERHEAD: usize = 8;
        self.points.len() * (size_of::<VertexId>() + OVERHEAD)
            + self.edges.len() * (size_of::<EdgeKey>() + OVERHEAD)
            + self.polygons.len() * (size_of::<FaceId>() + OVERHEAD)
    }
}

/// Every element of `kind` in the mesh.
pub fn all(mesh: &Mesh, topology: &Topology, kind: ElementKind) -> Vec<Element> {
    match kind {
        ElementKind::Points => mesh.vertices().map(|(v, _)| Element::Point(v)).collect(),
        ElementKind::Edges => topology.edges().map(Element::Edge).collect(),
        ElementKind::Polygons => mesh.faces().map(|(f, _)| Element::Polygon(f)).collect(),
    }
}

/// Every element of `kind` that isn't selected (the inverted selection).
pub fn inverted(
    mesh: &Mesh,
    topology: &Topology,
    selection: &ElementSelection,
    kind: ElementKind,
) -> Vec<Element> {
    all(mesh, topology, kind)
        .into_iter()
        .filter(|e| !selection.contains(*e))
        .collect()
}

/// The selection of `kind` grown by one step: every element touching a
/// selected one (sharing a point) is added.
pub fn grown(
    mesh: &Mesh,
    topology: &Topology,
    selection: &ElementSelection,
    kind: ElementKind,
) -> Vec<Element> {
    let selection = selection.valid_in(mesh, topology);
    let mut out: BTreeSet<Element> = selection.of_kind(kind).into_iter().collect();
    match kind {
        ElementKind::Points => {
            for v in selection.points() {
                out.extend(
                    topology
                        .vertex_neighbours(v)
                        .into_iter()
                        .map(Element::Point),
                );
            }
        }
        ElementKind::Edges => {
            for e in selection.edges() {
                let (a, b) = e.ends();
                for end in [a, b] {
                    out.extend(topology.vertex_edges(end).iter().map(|e| Element::Edge(*e)));
                }
            }
        }
        ElementKind::Polygons => {
            for f in selection.polygons() {
                for corner in mesh.face(f).unwrap_or_default() {
                    out.extend(
                        topology
                            .vertex_faces(*corner)
                            .iter()
                            .map(|f| Element::Polygon(*f)),
                    );
                }
            }
        }
    }
    out.into_iter().collect()
}

/// Every element of `kind` joined to a selected one, however far, through
/// edges and polygons ([decision 0118](../../../docs/decisions/0118-select-connected-and-outside-warnings.md)):
/// the whole part, or parts, of the shape the selection is on. Seams and
/// UV splits don't part anything here: it's the shape that counts.
pub fn connected(
    mesh: &Mesh,
    topology: &Topology,
    selection: &ElementSelection,
    kind: ElementKind,
) -> Vec<Element> {
    let selection = selection.valid_in(mesh, topology);
    let mut seeds: Vec<VertexId> = Vec::new();
    for element in selection.of_kind(kind) {
        match element {
            Element::Point(v) => seeds.push(v),
            Element::Edge(e) => {
                let (a, b) = e.ends();
                seeds.extend([a, b]);
            }
            Element::Polygon(f) => seeds.extend(mesh.face(f).unwrap_or_default().iter().copied()),
        }
    }
    let mut reached: BTreeSet<VertexId> = BTreeSet::new();
    while let Some(v) = seeds.pop() {
        if reached.insert(v) {
            seeds.extend(
                topology
                    .vertex_neighbours(v)
                    .into_iter()
                    .filter(|w| !reached.contains(w)),
            );
        }
    }
    match kind {
        ElementKind::Points => reached.into_iter().map(Element::Point).collect(),
        ElementKind::Edges => topology
            .edges()
            .filter(|e| reached.contains(&e.ends().0))
            .map(Element::Edge)
            .collect(),
        ElementKind::Polygons => mesh
            .faces()
            .filter(|(_, corners)| corners.first().is_some_and(|v| reached.contains(v)))
            .map(|(f, _)| Element::Polygon(f))
            .collect(),
    }
}

/// The selection of `kind` shrunk by one step: every selected element
/// touching an unselected one (sharing a point) is dropped, so the
/// selection's rim goes.
pub fn shrunk(
    mesh: &Mesh,
    topology: &Topology,
    selection: &ElementSelection,
    kind: ElementKind,
) -> Vec<Element> {
    let selection = selection.valid_in(mesh, topology);
    match kind {
        ElementKind::Points => selection
            .points()
            .filter(|v| {
                topology
                    .vertex_neighbours(*v)
                    .iter()
                    .all(|n| selection.contains(Element::Point(*n)))
            })
            .map(Element::Point)
            .collect(),
        ElementKind::Edges => {
            // Points where an unselected edge meets the selection.
            let rim = |v: VertexId| {
                topology
                    .vertex_edges(v)
                    .iter()
                    .any(|e| !selection.contains(Element::Edge(*e)))
            };
            selection
                .edges()
                .filter(|e| {
                    let (a, b) = e.ends();
                    !rim(a) && !rim(b)
                })
                .map(Element::Edge)
                .collect()
        }
        ElementKind::Polygons => {
            let rim = |v: &VertexId| {
                topology
                    .vertex_faces(*v)
                    .iter()
                    .any(|f| !selection.contains(Element::Polygon(*f)))
            };
            selection
                .polygons()
                .filter(|f| !mesh.face(*f).unwrap_or_default().iter().any(rim))
                .map(Element::Polygon)
                .collect()
        }
    }
}

/// The loop through `edge`, as elements of `kind` (decision 0029):
/// - **Edges:** the [edge loop](Topology::edge_loop).
/// - **Points:** the points along that loop.
/// - **Polygons:** the strip of polygons the loop's *ring* crosses, so a
///   double-click near a polygon's edge selects the band of polygons
///   running across that edge, through the polygon.
///
/// Empty for an edge the mesh doesn't have.
pub fn loop_through(topology: &Topology, edge: EdgeKey, kind: ElementKind) -> Vec<Element> {
    match kind {
        ElementKind::Edges => topology
            .edge_loop(edge)
            .into_iter()
            .map(Element::Edge)
            .collect(),
        ElementKind::Points => ends_of(&topology.edge_loop(edge)),
        ElementKind::Polygons => strip_across(topology, &topology.edge_ring(edge)),
    }
}

/// The ring through `edge`, as elements of `kind` (decision 0029):
/// - **Edges:** the [edge ring](Topology::edge_ring).
/// - **Points:** the points of the ring's edges.
/// - **Polygons:** the strip of polygons running *along* `edge` on its
///   side in `face` (the polygon loop at right angles to
///   [`loop_through`]'s): found by crossing `face`'s edge next to `edge`.
///   Without `face` (or when `face` doesn't hold `edge`), the polygons on
///   either side of `edge`.
pub fn ring_through(
    mesh: &Mesh,
    topology: &Topology,
    edge: EdgeKey,
    face: Option<FaceId>,
    kind: ElementKind,
) -> Vec<Element> {
    match kind {
        ElementKind::Edges => topology
            .edge_ring(edge)
            .into_iter()
            .map(Element::Edge)
            .collect(),
        ElementKind::Points => ends_of(&topology.edge_ring(edge)),
        ElementKind::Polygons => {
            let beside = face.and_then(|f| next_edge_in_face(mesh, f, edge));
            match beside {
                Some(across) => strip_across(topology, &topology.edge_ring(across)),
                None => topology.edge_faces(edge).map(Element::Polygon).collect(),
            }
        }
    }
}

/// The points at the ends of `edges`, each once.
fn ends_of(edges: &[EdgeKey]) -> Vec<Element> {
    let points: BTreeSet<VertexId> = edges
        .iter()
        .flat_map(|e| {
            let (a, b) = e.ends();
            [a, b]
        })
        .collect();
    points.into_iter().map(Element::Point).collect()
}

/// The polygons a ring of edges runs across: those holding two of its
/// edges. A ring of one edge (it couldn't go on) gives the polygons
/// beside that edge.
fn strip_across(topology: &Topology, ring: &[EdgeKey]) -> Vec<Element> {
    let rungs: BTreeSet<EdgeKey> = ring.iter().copied().collect();
    let mut faces = BTreeSet::new();
    for edge in ring {
        for face in topology.edge_faces(*edge) {
            let held = topology
                .face_edges(face)
                .iter()
                .filter(|e| rungs.contains(e))
                .count();
            if held >= 2 || ring.len() == 1 {
                faces.insert(face);
            }
        }
    }
    faces.into_iter().map(Element::Polygon).collect()
}

/// In `face`, the edge that follows `edge` going round the face.
fn next_edge_in_face(mesh: &Mesh, face: FaceId, edge: EdgeKey) -> Option<EdgeKey> {
    let corners = mesh.face(face)?;
    let n = corners.len();
    (0..n).find_map(|i| {
        let (a, b) = (corners[i], corners[(i + 1) % n]);
        (EdgeKey::new(a, b) == edge).then(|| {
            let c = corners[(i + 2) % n];
            EdgeKey::new(b, c)
        })
    })
}

/// The edges of `face`, going round it (for finding the one nearest the
/// mouse).
pub fn face_edges(mesh: &Mesh, face: FaceId) -> Vec<EdgeKey> {
    let corners = mesh.face(face).unwrap_or_default();
    let n = corners.len();
    (0..n)
        .map(|i| EdgeKey::new(corners[i], corners[(i + 1) % n]))
        .collect()
}

#[cfg(test)]
mod tests;
