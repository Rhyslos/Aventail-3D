//! The decimated shape as a mesh: its points in their first order, its
//! triangles (or flat pairs joined into quads), UVs, polygon values and
//! edge marks.

use super::{Decimator, key};
use crate::attributes::{AttributeValues, Domain, SEAM, UV, Value};
use crate::crease::CREASE;
use crate::material::slot_of;
use crate::quadify::pairing::{Candidate, best_pairs, quad_shape};
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::{DVec3, Vec2};
use std::collections::HashMap;

/// How far two triangles may lean from each other and still count as
/// flat together: a degree and a half.
const FLAT_DEGREES: f64 = 1.5;

impl Decimator {
    /// The shape as it is now: triangles, or with `quads_where_flat` flat
    /// pairs of triangles joined into four-sided polygons (see the module
    /// notes).
    pub fn mesh(&self, quads_where_flat: bool) -> Mesh {
        let pairs = if quads_where_flat {
            self.flat_pairs()
        } else {
            HashMap::new()
        };
        // The points still used, in their order.
        let mut used = vec![false; self.places.len()];
        for (t, corners) in self.tris.iter().enumerate() {
            if self.tri_alive[t] {
                for w in corners {
                    used[self.wedge_point[*w as usize] as usize] = true;
                }
            }
        }
        let mut mesh = Mesh::new();
        let mut ids: Vec<Option<VertexId>> = vec![None; self.places.len()];
        for (p, place) in self.places.iter().enumerate() {
            if used[p] {
                ids[p] = mesh.add_vertex(*place).ok();
            }
        }
        let mut from: Vec<usize> = Vec::new();
        let mut uvs: Vec<Vec2> = Vec::new();
        let mut wedges: Vec<u32> = Vec::with_capacity(4);
        let mut corners: Vec<VertexId> = Vec::with_capacity(4);
        for (t, tri) in self.tris.iter().enumerate() {
            if !self.tri_alive[t] {
                continue;
            }
            wedges.clear();
            match pairs.get(&(t as u32)) {
                // The second of a pair is part of the first's quad.
                Some(Pair::Second) => continue,
                Some(Pair::First(quad)) => wedges.extend_from_slice(quad),
                None => wedges.extend_from_slice(tri),
            }
            corners.clear();
            corners.extend(
                wedges
                    .iter()
                    .filter_map(|w| ids[self.wedge_point[*w as usize] as usize]),
            );
            if mesh.add_face(&corners).is_ok() {
                from.push(self.tri_face[t] as usize);
                uvs.extend(wedges.iter().map(|w| self.wedge_uv[*w as usize]));
            }
        }
        if self.has_uvs {
            let _ = mesh.set_attribute_values(
                UV,
                Domain::Corner,
                Value::Vec2(Vec2::ZERO),
                AttributeValues::Vec2(uvs),
            );
        }
        self.put_marks(&mut mesh, &ids);
        crate::subdivide::carry_face_values(&self.original, &mut mesh, &from);
        mesh
    }

    /// Hard edges, seams and creases on the edges the result has.
    fn put_marks(&self, mesh: &mut Mesh, ids: &[Option<VertexId>]) {
        let mut point_of: HashMap<VertexId, u32> = HashMap::new();
        for (p, id) in ids.iter().enumerate() {
            if let Some(id) = id {
                point_of.insert(*id, p as u32);
            }
        }
        let mut seams: Vec<(EdgeKey, Value)> = Vec::new();
        let mut creases: Vec<(EdgeKey, Value)> = Vec::new();
        let mut hard: Vec<EdgeKey> = Vec::new();
        let edges: Vec<EdgeKey> = mesh.topology().edges().collect();
        for edge in edges {
            let (a, b) = edge.ends();
            let (Some(pa), Some(pb)) = (point_of.get(&a), point_of.get(&b)) else {
                continue;
            };
            let Some(marks) = self.marks.get(&key(*pa, *pb)) else {
                continue;
            };
            if marks.hard {
                hard.push(edge);
            }
            if marks.seam {
                seams.push((edge, Value::Bool(true)));
            }
            if marks.crease > 0.0 {
                creases.push((edge, Value::Float(marks.crease)));
            }
        }
        for edge in hard {
            let (a, b) = edge.ends();
            mesh.set_hard(a, b, true);
        }
        if !seams.is_empty() || self.original.attribute_values(SEAM).is_some() {
            let _ = mesh.set_attribute_values(
                SEAM,
                Domain::Edge,
                Value::Bool(false),
                AttributeValues::Edges(seams),
            );
        }
        if !creases.is_empty() {
            let _ = mesh.set_attribute_values(
                CREASE,
                Domain::Edge,
                Value::Float(0.0),
                AttributeValues::Edges(creases),
            );
        }
    }

    /// Pairs of live triangles lying flat together, joined across an
    /// edge that is no kept line and carries no mark, with the same
    /// material and UVs that run on across it, making a quad with no
    /// dent: the squarest quads first. The first of each pair (by number)
    /// holds the quad's corners.
    fn flat_pairs(&self) -> HashMap<u32, Pair> {
        let flat = FLAT_DEGREES.to_radians();
        // Each edge's live triangles, by key, in triangle order.
        let mut on_edge: HashMap<u64, Vec<u32>> = HashMap::new();
        for (t, alive) in self.tri_alive.iter().enumerate() {
            if *alive {
                let p = self.points_of_tri(t);
                for k in 0..3 {
                    on_edge
                        .entry(key(p[k], p[(k + 1) % 3]))
                        .or_default()
                        .push(t as u32);
                }
            }
        }
        let mut candidates: Vec<Candidate<[u32; 4]>> = Vec::new();
        for (t, alive) in self.tri_alive.iter().enumerate() {
            if !*alive {
                continue;
            }
            let p = self.points_of_tri(t);
            for k in 0..3 {
                let (a, b) = (p[k], p[(k + 1) % 3]);
                let edge = key(a, b);
                let Some([t1, t2]) = on_edge
                    .get(&edge)
                    .map(Vec::as_slice)
                    .and_then(|s| <[u32; 2]>::try_from(s).ok())
                else {
                    continue;
                };
                // Seen once, from the first triangle.
                if t1 != t as u32
                    || self.features.contains_key(&edge)
                    || self.marks.contains_key(&edge)
                {
                    continue;
                }
                if let Some((score, quad)) = self.quad(t1, t2, (a, b), flat) {
                    candidates.push(Candidate {
                        score,
                        first: t1,
                        second: t2,
                        item: quad,
                    });
                }
            }
        }
        // The squarest first, as Quadify chooses (decision 0150).
        let mut pairs: HashMap<u32, Pair> = HashMap::new();
        for chosen in best_pairs(candidates) {
            pairs.insert(chosen.first, Pair::First(chosen.item));
            pairs.insert(chosen.second, Pair::Second);
        }
        pairs
    }

    fn points_of_tri(&self, t: usize) -> [u32; 3] {
        self.tris[t].map(|w| self.wedge_point[w as usize])
    }

    /// The quad triangles `t1` (with the edge `a` to `b`) and `t2` make,
    /// with how far its corners are from square (lower is better), if
    /// they may be joined.
    fn quad(&self, t1: u32, t2: u32, (a, b): (u32, u32), flat: f64) -> Option<(f64, [u32; 4])> {
        let (w1, w2) = (self.tris[t1 as usize], self.tris[t2 as usize]);
        // `t1` runs a → b → c, `t2` must run b → a → d.
        let k1 = (0..3).find(|k| self.wedge_point[w1[*k] as usize] == a)?;
        let k2 = (0..3).find(|k| self.wedge_point[w2[*k] as usize] == b)?;
        if self.wedge_point[w1[(k1 + 1) % 3] as usize] != b
            || self.wedge_point[w2[(k2 + 1) % 3] as usize] != a
        {
            return None;
        }
        // The same UV on both sides at both ends, and the same material.
        if w1[k1] != w2[(k2 + 1) % 3] || w1[(k1 + 1) % 3] != w2[k2] {
            return None;
        }
        let face = |t: u32| FaceId::from_raw(self.tri_face[t as usize]);
        if slot_of(&self.original, face(t1)) != slot_of(&self.original, face(t2)) {
            return None;
        }
        let quad = [w1[k1], w2[(k2 + 2) % 3], w1[(k1 + 1) % 3], w1[(k1 + 2) % 3]];
        let p: [DVec3; 4] = quad.map(|w| self.pos[self.wedge_point[w as usize] as usize]);
        // No dent, and flat together (shared with Quadify, decision 0150).
        let shape = quad_shape(p)?;
        if shape.fold > flat {
            return None;
        }
        Some((shape.squareness, quad))
    }
}

/// A triangle's part in a quad.
enum Pair {
    /// The first: the quad's corners (wedges).
    First([u32; 4]),
    /// The second, drawn by the first.
    Second,
}
