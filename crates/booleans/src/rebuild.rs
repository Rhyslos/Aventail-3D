//! Manifold's triangles put back together into polygons, with the values
//! (materials, UVs, hard edges, seams) of the polygons they came from.

use crate::solid::Joined;
use crate::{Options, Problem};
use glam::Vec3;
use manifold_rust::types::MeshGL64;
use mesh::attributes::{HARD, UV};
use mesh::{Builder, CornerFrom, EdgeKey, FaceId, Mesh, PointFrom, VertexId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// A polygon of the result: the polygon it came from (if known) and its
/// corners, as Manifold's point numbers.
struct Piece {
    from: Option<FaceId>,
    corners: Vec<usize>,
}

/// The result as a polygon mesh, built from Manifold's triangles `gl`.
pub(crate) fn rebuild(joined: &Joined, gl: &MeshGL64, options: &Options) -> Result<Mesh, Problem> {
    let tri_count = gl.num_tri();
    if tri_count > options.max_polygons.saturating_mul(4) {
        return Err(Problem::TooBig);
    }
    let stride = usize::try_from(gl.num_prop).unwrap_or(3).max(3);
    let positions: Vec<Vec3> = gl
        .vert_properties
        .chunks_exact(stride)
        .map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32))
        .collect();
    let mut tris: Vec<[usize; 3]> = Vec::with_capacity(tri_count);
    for t in gl.tri_verts.chunks_exact(3) {
        let tri = [t[0], t[1], t[2]].map(|i| usize::try_from(i).unwrap_or(usize::MAX));
        if tri.iter().any(|i| *i >= positions.len()) {
            return Err(Problem::Failed(
                "a triangle names a point that isn't there".into(),
            ));
        }
        tris.push(tri);
    }
    let original = &joined.mesh;
    // The triangles that came from each polygon.
    let mut groups: BTreeMap<Option<FaceId>, Vec<usize>> = BTreeMap::new();
    for (t, _) in tris.iter().enumerate() {
        let from = gl
            .face_id
            .get(t)
            .and_then(|f| u32::try_from(*f).ok())
            .map(FaceId::from_raw)
            .filter(|f| original.face(*f).is_some());
        groups.entry(from).or_default().push(t);
    }
    let mut pieces = Vec::new();
    for (from, group) in &groups {
        for component in components(&tris, group) {
            match outline(&tris, &component) {
                Some(corners) => pieces.push(Piece {
                    from: *from,
                    corners,
                }),
                None => pieces.extend(component.iter().map(|t| Piece {
                    from: *from,
                    corners: tris[*t].to_vec(),
                })),
            }
        }
    }
    if pieces.len() > options.max_polygons {
        return Err(Problem::TooBig);
    }
    Ok(build(original, joined, &positions, &pieces, options))
}

/// The triangles of `group` in pieces that hang together (sharing
/// edges), each in the order of its first triangle.
fn components(tris: &[[usize; 3]], group: &[usize]) -> Vec<Vec<usize>> {
    let mut by_edge: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (k, t) in group.iter().enumerate() {
        let tri = tris[*t];
        for i in 0..3 {
            let (a, b) = (tri[i], tri[(i + 1) % 3]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(k);
        }
    }
    let mut parent: Vec<usize> = (0..group.len()).collect();
    fn root(parent: &mut [usize], mut k: usize) -> usize {
        while parent[k] != k {
            parent[k] = parent[parent[k]];
            k = parent[k];
        }
        k
    }
    for sharing in by_edge.values() {
        for pair in sharing.windows(2) {
            let (a, b) = (root(&mut parent, pair[0]), root(&mut parent, pair[1]));
            if a != b {
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut out: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, t) in group.iter().enumerate() {
        let r = root(&mut parent, k);
        out.entry(r).or_default().push(*t);
    }
    out.into_values().collect()
}

/// The outline of a piece of triangles, as one polygon's corners in
/// order, if the piece is a single patch without holes (a disc). `None`
/// for anything else (a ring round a hole, or an outline that touches
/// itself): those stay triangles.
fn outline(tris: &[[usize; 3]], component: &[usize]) -> Option<Vec<usize>> {
    if component.len() == 1 {
        return Some(tris[component[0]].to_vec());
    }
    let mut half: HashSet<(usize, usize)> = HashSet::new();
    let mut points: HashSet<usize> = HashSet::new();
    for t in component {
        let tri = tris[*t];
        for i in 0..3 {
            half.insert((tri[i], tri[(i + 1) % 3]));
            points.insert(tri[i]);
        }
    }
    let undirected: HashSet<(usize, usize)> =
        half.iter().map(|(a, b)| (*a.min(b), *a.max(b))).collect();
    // A disc: points − edges + triangles = 1.
    if points.len() + component.len() != undirected.len() + 1 {
        return None;
    }
    let mut next: BTreeMap<usize, usize> = BTreeMap::new();
    for (a, b) in &half {
        if !half.contains(&(*b, *a)) && next.insert(*a, *b).is_some() {
            return None;
        }
    }
    let (&start, _) = next.first_key_value()?;
    let mut corners = vec![start];
    let mut at = *next.get(&start)?;
    while at != start {
        if corners.len() > next.len() {
            return None;
        }
        corners.push(at);
        at = *next.get(&at)?;
    }
    (corners.len() == next.len() && corners.len() >= 3).then_some(corners)
}

/// Builds the mesh from `pieces`, carrying values from `original`.
fn build(
    original: &Mesh,
    joined: &Joined,
    positions: &[Vec3],
    pieces: &[Piece],
    options: &Options,
) -> Mesh {
    let mut out = Mesh::new();
    for (name, domain, _) in original.attributes() {
        if name == HARD {
            continue;
        }
        if let Some((_, default, _)) = original.attribute_values(name) {
            // A layer the result can't hold is left out.
            let _ = out.add_attribute(name, domain, default);
        }
    }
    let mut source: HashMap<FaceId, FaceId> = HashMap::new();
    {
        let mut build = Builder::new(original, &mut out);
        let mut points: Vec<Option<VertexId>> = vec![None; positions.len()];
        for piece in pieces {
            let mut corners = Vec::with_capacity(piece.corners.len());
            for i in &piece.corners {
                let spot = positions[*i];
                let id = match points[*i] {
                    Some(id) => id,
                    None => {
                        let from = piece
                            .from
                            .map_or(PointFrom::Nothing, |f| PointFrom::Inside(f, spot));
                        let id = build.point(spot, from);
                        points[*i] = Some(id);
                        id
                    }
                };
                let from = piece
                    .from
                    .map_or(CornerFrom::Nothing, |f| corner_from(original, f, spot));
                corners.push((id, from));
            }
            if let Some(face) = build.face(&corners, piece.from)
                && let Some(from) = piece.from
            {
                source.insert(face, from);
            }
        }
    }
    edges(&mut out, original, joined, &source, options);
    out
}

/// Where a corner at `spot` on polygon `face` (of `original`) takes its
/// values from: a corner it sits on, the edge it sits on, or the polygon
/// as a whole. Points along an edge blend only that edge's two ends, so
/// UVs along a cut edge stay exactly in line.
fn corner_from(original: &Mesh, face: FaceId, spot: Vec3) -> CornerFrom {
    let Some(corners) = original.face(face) else {
        return CornerFrom::Nothing;
    };
    let places: Vec<Vec3> = corners
        .iter()
        .map(|v| original.position(*v).unwrap_or(spot))
        .collect();
    let size = places
        .iter()
        .map(|p| p.distance(places[0]))
        .fold(0.0_f32, f32::max);
    let close = (size * 1e-5).max(1e-6);
    if let Some(i) = places.iter().position(|p| p.distance(spot) <= close) {
        return CornerFrom::Copy(face, corners[i]);
    }
    let n = corners.len();
    for i in 0..n {
        let (a, b) = (places[i], places[(i + 1) % n]);
        let along = b - a;
        let length = along.length_squared();
        if length <= 0.0 {
            continue;
        }
        let t = (spot - a).dot(along) / length;
        if (0.0..=1.0).contains(&t) && (a + along * t).distance(spot) <= close {
            return CornerFrom::Between(face, corners[i], face, corners[(i + 1) % n], t);
        }
    }
    CornerFrom::Inside(face, spot)
}

/// Sets the result's edge values: an edge along an edge of the shape it
/// came from keeps that edge's values (hard, seam); a new edge along the
/// cut is hard where the surfaces meet more sharply than the seam angle,
/// and a seam where the shapes have UVs (they part there).
fn edges(
    out: &mut Mesh,
    original: &Mesh,
    joined: &Joined,
    source: &HashMap<FaceId, FaceId>,
    options: &Options,
) {
    // The original edge between each pair of polygons that share one.
    let topology = original.topology();
    let mut shared: HashMap<(FaceId, FaceId), EdgeKey> = HashMap::new();
    for edge in topology.edges() {
        let faces: Vec<FaceId> = topology.edge_faces(edge).collect();
        if let [a, b] = faces[..] {
            shared.insert((a.min(b), a.max(b)), edge);
        }
    }
    let mut copies: Vec<(EdgeKey, EdgeKey)> = Vec::new();
    let mut cut: Vec<(EdgeKey, bool)> = Vec::new();
    {
        let result = out.topology();
        for edge in result.edges() {
            let faces: Vec<FaceId> = result.edge_faces(edge).collect();
            let [fa, fb] = faces[..] else {
                continue;
            };
            let (Some(sa), Some(sb)) = (source.get(&fa), source.get(&fb)) else {
                continue;
            };
            if sa == sb {
                continue;
            }
            let along = (joined.part_of(*sa) == joined.part_of(*sb))
                .then(|| shared.get(&(*sa.min(sb), *sa.max(sb))))
                .flatten();
            match along {
                Some(from) => copies.push((*from, edge)),
                None => {
                    let angle = out
                        .face_normal(fa)
                        .angle_between(out.face_normal(fb))
                        .to_degrees();
                    cut.push((edge, angle > options.seam_angle));
                }
            }
        }
    }
    {
        let mut build = Builder::new(original, out);
        for (from, edge) in copies {
            let (a, b) = edge.ends();
            build.copy_edge(from, a, b);
        }
    }
    for (edge, hard) in &cut {
        if *hard {
            let (a, b) = edge.ends();
            out.set_hard(a, b, true);
        }
    }
    if original.attribute_values(UV).is_some() {
        let seams: Vec<EdgeKey> = cut.iter().map(|(e, _)| *e).collect();
        mesh::uvs::set_seams(out, &seams, true);
    }
}
