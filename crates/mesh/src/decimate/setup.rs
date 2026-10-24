//! Setting a decimation up: the shape cut into triangles, its UV corner
//! groups (wedges), the kept lines, the marks to carry, and the quadrics.

use super::{Class, DecimateProblem, Decimator, Keep, Marks, Quadric, UvQuadric, key};
use crate::attributes::{AttributeValues, Domain, UV, Value};
use crate::material::slot_of;
use crate::{FaceId, Hardness, Mesh};
use glam::{DVec3, Vec2};
use std::collections::{BinaryHeap, HashMap};

/// Edge kinds: an open border.
pub(super) const BORDER: u8 = 1;
/// A UV seam, marked or where the UVs split.
pub(super) const SEAM: u8 = 2;
/// A hard edge, as the shading shows it.
pub(super) const HARD: u8 = 4;
/// A border between materials.
pub(super) const MATERIAL: u8 = 8;
/// An edge of three or more triangles: always kept.
pub(super) const TANGLED: u8 = 16;

/// How much more than a triangle's plane a kept line's own planes weigh.
const LINE_WEIGHT: f64 = 10.0;

impl Keep {
    /// The edge kinds kept as lines.
    fn mask(self) -> u8 {
        let mut mask = TANGLED;
        for (on, bit) in [
            (self.borders, BORDER),
            (self.seams, SEAM),
            (self.hard_edges, HARD),
            (self.materials, MATERIAL),
        ] {
            if on {
                mask |= bit;
            }
        }
        mask
    }
}

/// The shape as triangles of wedges.
struct Triangles {
    places: Vec<glam::Vec3>,
    wedge_point: Vec<u32>,
    wedge_uv: Vec<Vec2>,
    has_uvs: bool,
    tris: Vec<[u32; 3]>,
    tri_face: Vec<u32>,
    /// Each point slot of the mesh's dense number (`u32::MAX`: not used).
    dense: Vec<u32>,
}

/// Cuts `mesh` into triangles, numbering the points its polygons use (in
/// the order the polygons first use them) and grouping each point's
/// corners by UV (a hundred-thousandth apart counts as the same).
fn triangles_of(mesh: &Mesh) -> Result<Triangles, DecimateProblem> {
    let uv_values = match mesh.attribute_values(UV) {
        Some((Domain::Corner, Value::Vec2(default), AttributeValues::Vec2(list))) => {
            Some((default, list))
        }
        _ => None,
    };
    let (starts, _) = mesh.face_layout();
    let mut out = Triangles {
        places: Vec::with_capacity(mesh.vertex_count()),
        wedge_point: Vec::new(),
        wedge_uv: Vec::new(),
        has_uvs: uv_values.is_some(),
        tris: Vec::new(),
        tri_face: Vec::new(),
        dense: vec![u32::MAX; mesh.point_slot_count()],
    };
    let mut wedges: HashMap<(u32, [i64; 2]), u32> = HashMap::new();
    let mut corner_wedges: Vec<u32> = Vec::new();
    for (face, corners) in mesh.faces() {
        corner_wedges.clear();
        let flat = starts.get(face.get() as usize).copied().unwrap_or(0) as usize;
        for (k, v) in corners.iter().enumerate() {
            let slot = v.get() as usize;
            let point = match out.dense.get(slot) {
                Some(p) if *p != u32::MAX => *p,
                _ => {
                    let place = mesh.position(*v).ok_or(DecimateProblem::NotANumber)?;
                    if !place.is_finite() {
                        return Err(DecimateProblem::NotANumber);
                    }
                    let p = out.places.len() as u32;
                    out.places.push(place);
                    if let Some(d) = out.dense.get_mut(slot) {
                        *d = p;
                    }
                    p
                }
            };
            let uv = uv_values.as_ref().map_or(Vec2::ZERO, |(default, list)| {
                let uv = list.get(flat + k).copied().unwrap_or(*default);
                if uv.is_finite() { uv } else { Vec2::ZERO }
            });
            let rounded = [(uv.x * 1e5).round() as i64, (uv.y * 1e5).round() as i64];
            let wedge = *wedges.entry((point, rounded)).or_insert_with(|| {
                out.wedge_point.push(point);
                out.wedge_uv.push(uv);
                (out.wedge_point.len() - 1) as u32
            });
            corner_wedges.push(wedge);
        }
        crate::triangulate::face_triangles(mesh, corners, |[a, b, c]| {
            out.tris
                .push([corner_wedges[a], corner_wedges[b], corner_wedges[c]]);
            out.tri_face.push(face.get());
        });
    }
    Ok(out)
}

/// Each edge's triangles, as runs of (edge key, triangle) sorted by key.
fn edge_runs(tris: &[[u32; 3]], wedge_point: &[u32]) -> Vec<(u64, u32)> {
    let mut list: Vec<(u64, u32)> = Vec::with_capacity(tris.len() * 3);
    for (t, corners) in tris.iter().enumerate() {
        for k in 0..3 {
            let a = wedge_point[corners[k] as usize];
            let b = wedge_point[corners[(k + 1) % 3] as usize];
            list.push((key(a, b), t as u32));
        }
    }
    list.sort_unstable();
    list
}

/// The wedge at point `p` in triangle `corners`.
fn wedge_at(corners: &[u32; 3], wedge_point: &[u32], p: u32) -> Option<u32> {
    corners
        .iter()
        .copied()
        .find(|w| wedge_point[*w as usize] == p)
}

impl Decimator {
    /// Sets up the decimation of `mesh` (see the module notes): `keep`
    /// says which lines stay, `hardness` how the object is shaded (the
    /// hard edges kept are those it shows). Refused only for a polygon's
    /// point that isn't at a real position.
    pub fn new(mesh: &Mesh, keep: Keep, hardness: Hardness) -> Result<Self, DecimateProblem> {
        let shape = triangles_of(mesh)?;
        let points = shape.places.len();
        let dense_key = |e: crate::EdgeKey| {
            let (a, b) = e.ends();
            let a = *shape.dense.get(a.get() as usize)?;
            let b = *shape.dense.get(b.get() as usize)?;
            (a != u32::MAX && b != u32::MAX).then(|| key(a, b))
        };
        // Edge kinds from the mesh's own marks.
        let mut kinds: HashMap<u64, u8> = HashMap::new();
        for e in crate::uvs::seams(mesh).into_iter().filter_map(dense_key) {
            *kinds.entry(e).or_default() |= SEAM;
        }
        let hard = crate::hardness::shading_hard_edges(mesh, hardness);
        for e in hard.into_iter().filter_map(dense_key) {
            *kinds.entry(e).or_default() |= HARD;
        }
        let mut marks: HashMap<u64, Marks> = HashMap::new();
        for e in mesh.hard_edges().filter_map(dense_key) {
            marks.entry(e).or_default().hard = true;
        }
        for e in crate::uvs::seams(mesh).into_iter().filter_map(dense_key) {
            marks.entry(e).or_default().seam = true;
        }
        for (e, crease) in crate::crease::creases(mesh) {
            if let Some(e) = dense_key(e) {
                marks.entry(e).or_default().crease = crease;
            }
        }

        let center = crate::Bounds::around(shape.places.iter().copied())
            .map_or(DVec3::ZERO, |b| b.centre().as_dvec3());
        let extent = crate::Bounds::around(shape.places.iter().copied())
            .map_or(1.0, |b| f64::from(b.size().max_element()));
        let scale = if extent > 0.0 && extent.is_finite() {
            1.0 / extent
        } else {
            1.0
        };
        let pos: Vec<DVec3> = shape
            .places
            .iter()
            .map(|p| (p.as_dvec3() - center) * scale)
            .collect();

        // Each edge's kinds, and the kept lines.
        let mask = keep.mask();
        let mut features: HashMap<u64, u8> = HashMap::new();
        let runs = edge_runs(&shape.tris, &shape.wedge_point);
        let mut line_quadrics: Vec<(u32, Quadric)> = Vec::new();
        let mut i = 0;
        while i < runs.len() {
            let edge = runs[i].0;
            let mut j = i;
            while j < runs.len() && runs[j].0 == edge {
                j += 1;
            }
            let run = &runs[i..j];
            let (a, b) = ((edge >> 32) as u32, edge as u32);
            let mut kind = kinds.get(&edge).copied().unwrap_or(0);
            match run {
                [_] => kind |= BORDER,
                [(_, t1), (_, t2)] => {
                    let (c1, c2) = (shape.tris[*t1 as usize], shape.tris[*t2 as usize]);
                    let split = [a, b].iter().any(|p| {
                        wedge_at(&c1, &shape.wedge_point, *p)
                            != wedge_at(&c2, &shape.wedge_point, *p)
                    });
                    if split {
                        kind |= SEAM;
                    }
                    let slot = |t: u32| slot_of(mesh, FaceId::from_raw(shape.tri_face[t as usize]));
                    if slot(*t1) != slot(*t2) {
                        kind |= MATERIAL;
                    }
                }
                _ => kind |= TANGLED,
            }
            let kept = kind & mask;
            if kept != 0 {
                features.insert(edge, kept);
                // Planes standing on the line, one per triangle beside it,
                // keep a point sliding along it from leaving its course.
                let (pa, pb) = (pos[a as usize], pos[b as usize]);
                let along = pb - pa;
                for (_, t) in run {
                    let c = shape.tris[*t as usize]
                        .map(|w| pos[shape.wedge_point[w as usize] as usize]);
                    let facing = (c[1] - c[0]).cross(c[2] - c[0]).normalize_or_zero();
                    let normal = along.cross(facing).normalize_or_zero();
                    if normal != DVec3::ZERO {
                        let q = Quadric::plane(normal, pa, LINE_WEIGHT * along.length_squared());
                        line_quadrics.push((a, q));
                        line_quadrics.push((b, q));
                    }
                }
            }
            i = j;
        }

        // The quadrics: each triangle's plane, weighted by its area, on
        // its points; its UV slopes on its wedges.
        let mut quadrics = vec![Quadric::default(); points];
        let mut uv_quadrics = vec![UvQuadric::default(); shape.wedge_point.len()];
        let mut total_area = 0.0;
        for corners in &shape.tris {
            let p = corners.map(|w| pos[shape.wedge_point[w as usize] as usize]);
            let n = (p[1] - p[0]).cross(p[2] - p[0]);
            let area = n.length() / 2.0;
            if !(area > 0.0 && area.is_finite()) {
                continue;
            }
            total_area += area;
            let q = Quadric::plane(n / (2.0 * area), p[0], area);
            for w in corners {
                quadrics[shape.wedge_point[*w as usize] as usize].add(&q);
            }
            if shape.has_uvs {
                let uv = corners.map(|w| {
                    let uv = shape.wedge_uv[w as usize].as_dvec2();
                    [uv.x, uv.y]
                });
                let q = UvQuadric::triangle(p, uv, area);
                for w in corners {
                    uv_quadrics[*w as usize].add(&q);
                }
            }
        }
        for (p, q) in &line_quadrics {
            quadrics[*p as usize].add(q);
        }

        let mut tris_of: Vec<Vec<u32>> = vec![Vec::new(); points];
        for (t, corners) in shape.tris.iter().enumerate() {
            for w in corners {
                tris_of[shape.wedge_point[*w as usize] as usize].push(t as u32);
            }
        }
        let live = shape.tris.len();
        let mean_area = if live > 0 {
            total_area / live as f64
        } else {
            0.0
        };
        let mut decimator = Self {
            original: mesh.clone(),
            places: shape.places,
            pos,
            wedge_point: shape.wedge_point,
            wedge_uv: shape.wedge_uv,
            has_uvs: shape.has_uvs,
            tri_alive: vec![true; live],
            tris: shape.tris,
            tri_face: shape.tri_face,
            live,
            tris_of,
            point_alive: vec![true; points],
            class: vec![Class::Free; points],
            features,
            marks,
            quadrics,
            uv_quadrics,
            best: vec![None; points],
            version: vec![0; points],
            heap: BinaryHeap::new(),
            tie: 1e-3 * mean_area,
        };
        for p in 0..points as u32 {
            decimator.class[p as usize] = decimator.classify(p);
        }
        for p in 0..points as u32 {
            decimator.consider(p, false);
        }
        Ok(decimator)
    }
}
