//! Charts: the pieces of a surface laid flat in one go, and their points as
//! laid flat.
//!
//! A point of the shape can appear more than once in a chart: where a cut
//! runs through it, each side gets its own copy (a **wedge**). Wedges are
//! what the unwrapping tools place; each polygon corner belongs to one.

use mesh::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeSet, HashMap};

/// A simple union-find over `0..n`.
pub(crate) struct Sets {
    parent: Vec<usize>,
}

impl Sets {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    pub(crate) fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Joins the sets of `a` and `b`; whether they were apart.
    pub(crate) fn join(&mut self, a: usize, b: usize) -> bool {
        let (a, b) = (self.find(a), self.find(b));
        if a == b {
            return false;
        }
        self.parent[a] = b;
        true
    }
}

/// A piece of surface to lay flat.
#[derive(Debug, Clone)]
pub(crate) struct Chart {
    /// Its polygons.
    pub(crate) faces: Vec<FaceId>,
    /// For each polygon (in the order of `faces`), the wedge of each of
    /// its corners.
    pub(crate) corners: Vec<Vec<usize>>,
    /// Each wedge's point.
    pub(crate) points: Vec<VertexId>,
}

impl Chart {
    /// The chart of `faces`, cut open along `cuts` (and wherever two of its
    /// polygons don't share an edge cleanly).
    pub(crate) fn of(mesh: &Mesh, faces: &[FaceId], cuts: &BTreeSet<EdgeKey>) -> Self {
        let topology = mesh.topology();
        let inside: BTreeSet<FaceId> = faces.iter().copied().collect();
        let mut index: HashMap<(FaceId, VertexId), usize> = HashMap::new();
        let mut corner_points = Vec::new();
        let mut polygons = Vec::new();
        for f in faces {
            let list = mesh.face(*f).unwrap_or(&[]);
            let mut mine = Vec::new();
            for v in list {
                let next = index.len();
                let i = *index.entry((*f, *v)).or_insert_with(|| {
                    corner_points.push(*v);
                    next
                });
                mine.push(i);
            }
            polygons.push(mine);
        }
        let mut sets = Sets::new(index.len());
        let mut seen = BTreeSet::new();
        for f in faces {
            let list = mesh.face(*f).unwrap_or(&[]);
            for k in 0..list.len() {
                let e = EdgeKey::new(list[k], list[(k + 1) % list.len()]);
                if !seen.insert(e) || cuts.contains(&e) {
                    continue;
                }
                let uses = topology.edge_uses(e);
                let [one, two] = uses else {
                    continue;
                };
                if one.face == two.face
                    || !inside.contains(&one.face)
                    || !inside.contains(&two.face)
                {
                    continue;
                }
                let (a, b) = e.ends();
                for v in [a, b] {
                    if let (Some(x), Some(y)) =
                        (index.get(&(one.face, v)), index.get(&(two.face, v)))
                    {
                        sets.join(*x, *y);
                    }
                }
            }
        }
        // Number the wedges in order of first use.
        let mut wedge_of_root: HashMap<usize, usize> = HashMap::new();
        let mut points = Vec::new();
        let corners = polygons
            .iter()
            .map(|mine| {
                mine.iter()
                    .map(|c| {
                        let root = sets.find(*c);
                        *wedge_of_root.entry(root).or_insert_with(|| {
                            points.push(corner_points[*c]);
                            points.len() - 1
                        })
                    })
                    .collect()
            })
            .collect();
        Self {
            faces: faces.to_vec(),
            corners,
            points,
        }
    }

    /// How many wedges it has.
    pub(crate) fn len(&self) -> usize {
        self.points.len()
    }

    /// Its triangles, as wedges, cut the way the viewport cuts polygons.
    pub(crate) fn triangles(&self, mesh: &Mesh) -> Vec<[usize; 3]> {
        let mut out = Vec::new();
        for (f, wedges) in self.faces.iter().zip(&self.corners) {
            let list = mesh.face(*f).unwrap_or(&[]);
            mesh::triangulate::face_triangles(mesh, list, |[a, b, c]| {
                out.push([wedges[a], wedges[b], wedges[c]]);
            });
        }
        out
    }

    /// For each of its triangles (in the order of [`Self::triangles`]),
    /// which of its polygons (an index into `faces`) it is part of.
    pub(crate) fn triangle_owners(&self, mesh: &Mesh) -> Vec<usize> {
        let mut out = Vec::new();
        for (i, f) in self.faces.iter().enumerate() {
            let list = mesh.face(*f).unwrap_or(&[]);
            mesh::triangulate::face_triangles(mesh, list, |_| out.push(i));
        }
        out
    }

    /// The sides of its polygons, as pairs of wedges, each once.
    pub(crate) fn sides(&self) -> Vec<(usize, usize)> {
        let mut out = BTreeSet::new();
        for wedges in &self.corners {
            for k in 0..wedges.len() {
                let (a, b) = (wedges[k], wedges[(k + 1) % wedges.len()]);
                if a != b {
                    out.insert((a.min(b), a.max(b)));
                }
            }
        }
        out.into_iter().collect()
    }
}
