//! Subdivision smoothing ([decision 0143](../../../docs/decisions/0143-subdivide-and-creases.md)):
//! Catmull-Clark subdivision, the smoothing most 3D programs use.
//!
//! Each level splits every polygon of `n` corners into `n` quads (through
//! the middle of each edge and of the polygon) and moves every point
//! towards the average of its neighbours, so a box becomes a rounded
//! pebble and a low-polygon cage becomes a smooth surface. Each level has
//! four times the polygons of the one before (an n-gon gives n quads the
//! first time).
//!
//! **Sharp edges.** An edge with a crease ([`crate::crease`]) of 1 stays
//! sharp at every level; one of 0.5 stays sharp for half the levels and is
//! then smoothed, so it comes out as a tighter curve. With `hard_sharp`,
//! edges marked hard count as fully creased. Open borders stay where they
//! are as a line that is smoothed along itself, and the corner of an open
//! sheet (a point with one polygon) stays put. Edges shared by three or
//! more polygons are kept sharp.
//!
//! **What is carried over.** Each new polygon keeps its old polygon's
//! values (material, UV lock and the rest); hard edges, seams and full
//! creases are carried to both halves of their edges (partial creases are
//! used up by the levels). UVs are smoothed too, as their own surface, so
//! they stay a smooth fit to the shape: where UVs are split (a seam) the
//! border is smoothed along itself and each side keeps to its own, and a
//! UV corner with one polygon stays put (Blender's "keep corners"). Point
//! values and other corner values aren't carried.

mod step;
#[cfg(test)]
mod tests;

use crate::attributes::{AttributeValues, Domain, Value};
use crate::attributes::{SEAM, UV};
use crate::crease::CREASE;
use crate::{EdgeKey, Mesh, VertexId};
use glam::{Vec2, Vec3};
use std::collections::HashMap;
use step::{ALWAYS, Layout, step};
use words::{tr, trf};

/// The Subdivide modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubdivideOptions {
    /// Levels shown in the viewport (0 shows the shape unsmoothed).
    pub levels: u32,
    /// Levels used when exporting and baking.
    pub export_levels: u32,
    /// Whether edges marked hard stay sharp, as if fully creased.
    pub hard_edges_sharp: bool,
}

impl SubdivideOptions {
    /// The standard viewport levels.
    pub const STANDARD_LEVELS: u32 = 1;
    /// The standard export levels.
    pub const STANDARD_EXPORT_LEVELS: u32 = 2;
    /// The most levels: each multiplies the polygons by four.
    pub const MAX_LEVELS: u32 = 5;
    /// The most polygons a subdivision may make: past this it is
    /// refused, with the advice to lower the levels.
    pub const MAX_POLYGONS: u64 = 4_000_000;

    /// The settings brought within limits.
    pub fn clamped(self) -> Self {
        Self {
            levels: self.levels.min(Self::MAX_LEVELS),
            export_levels: self.export_levels.min(Self::MAX_LEVELS),
            hard_edges_sharp: self.hard_edges_sharp,
        }
    }
}

impl Default for SubdivideOptions {
    fn default() -> Self {
        Self {
            levels: Self::STANDARD_LEVELS,
            export_levels: Self::STANDARD_EXPORT_LEVELS,
            hard_edges_sharp: true,
        }
    }
}

/// Why a shape can't be subdivided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubdivideProblem {
    /// A point isn't at a real position.
    NotANumber,
    /// It would make `polygons` polygons, more than
    /// [`SubdivideOptions::MAX_POLYGONS`].
    TooMany {
        /// The polygons it would make.
        polygons: u64,
    },
}

impl std::fmt::Display for SubdivideProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotANumber => f.write_str(tr("a point of the shape isn't at a real position")),
            Self::TooMany { polygons } => f.write_str(&trf!(
                "it would make {} million polygons, and the most is 4 million. Lower the levels",
                format!("{:.1}", *polygons as f64 / 1e6)
            )),
        }
    }
}

impl std::error::Error for SubdivideProblem {}

/// How many polygons `levels` levels of subdivision make of `mesh`: its
/// polygons for 0 levels; its corners for 1 (an n-gon gives n quads); four
/// times as many for each level more.
pub fn polygon_count(mesh: &Mesh, levels: u32) -> u64 {
    if levels == 0 {
        return mesh.face_count() as u64;
    }
    let corners: u64 = mesh.faces().map(|(_, c)| c.len() as u64).sum();
    corners.saturating_mul(4u64.saturating_pow(levels - 1))
}

/// Which crease an edge has, as a sharpness: how many levels it stays
/// sharp for. A crease of 1 never wears off; below, `crease × levels`.
fn sharpness(crease: f32, levels: u32) -> f32 {
    let crease = crate::crease::usable(crease);
    if crease >= 1.0 {
        ALWAYS
    } else {
        crease * levels as f32
    }
}

/// From this many corners, a level's UVs are worked out on a thread of
/// their own, beside the points.
const PARALLEL_FROM: usize = 20_000;

/// Edge marks carried to the new edges.
const HARD: u8 = 1;
const SEAM_MARK: u8 = 2;
const FULL_CREASE: u8 = 4;

/// `mesh` smoothed by `levels` levels of Catmull-Clark subdivision (see
/// the module notes); `hard_sharp` keeps edges marked hard sharp. Zero
/// levels give the shape as it is.
pub fn subdivide(mesh: &Mesh, levels: u32, hard_sharp: bool) -> Result<Mesh, SubdivideProblem> {
    let levels = levels.min(SubdivideOptions::MAX_LEVELS);
    if levels == 0 || mesh.face_count() == 0 {
        return Ok(mesh.clone());
    }
    let polygons = polygon_count(mesh, levels);
    if polygons > SubdivideOptions::MAX_POLYGONS {
        return Err(SubdivideProblem::TooMany { polygons });
    }
    let mut cage = Cage::of(mesh, levels, hard_sharp)?;
    for _ in 0..levels {
        cage = cage.refined();
    }
    Ok(cage.into_mesh(mesh))
}

/// The shape being subdivided, as plain lists.
struct Cage {
    points: Vec<Vec3>,
    layout: Layout,
    /// Each corner's edge (to the next corner): how sharp, and its marks.
    sharp: Vec<f32>,
    marks: Vec<u8>,
    /// The UVs, welded: their values and each corner's.
    uvs: Option<(Vec<Vec2>, Layout)>,
    /// Each polygon's polygon in the original mesh (its slot number).
    face_from: Vec<u32>,
}

impl Cage {
    fn of(mesh: &Mesh, levels: u32, hard_sharp: bool) -> Result<Self, SubdivideProblem> {
        let mut index: Vec<u32> = vec![u32::MAX; mesh.point_slot_count()];
        let mut points = Vec::with_capacity(mesh.vertex_count());
        let mut layout = Layout::default();
        let mut sharp = Vec::new();
        let mut marks = Vec::new();
        let mut face_from = Vec::with_capacity(mesh.face_count());
        let creases: HashMap<EdgeKey, f32> = crate::crease::creases(mesh).into_iter().collect();
        let seams: std::collections::HashSet<EdgeKey> =
            crate::uvs::seams(mesh).into_iter().collect();
        let uv_values = match mesh.attribute_values(UV) {
            Some((Domain::Corner, Value::Vec2(default), AttributeValues::Vec2(list))) => {
                Some((default, list))
            }
            _ => None,
        };
        let (starts, _) = mesh.face_layout();
        let mut uvs: Vec<Vec2> = Vec::new();
        let mut uv_layout = Layout::default();
        let mut uv_index: HashMap<(u32, [i64; 2]), u32> = HashMap::new();
        for (face, corners) in mesh.faces() {
            layout.starts.push(layout.corners.len() as u32);
            uv_layout.starts.push(uv_layout.corners.len() as u32);
            face_from.push(face.get());
            let n = corners.len();
            let flat = starts[face.get() as usize] as usize;
            for k in 0..n {
                let v = corners[k];
                let i = match index.get(v.get() as usize) {
                    Some(i) if *i != u32::MAX => *i,
                    _ => {
                        let p = mesh.position(v).ok_or(SubdivideProblem::NotANumber)?;
                        if !p.is_finite() {
                            return Err(SubdivideProblem::NotANumber);
                        }
                        let i = points.len() as u32;
                        points.push(p);
                        if let Some(slot) = index.get_mut(v.get() as usize) {
                            *slot = i;
                        }
                        i
                    }
                };
                layout.corners.push(i);
                let edge = EdgeKey::new(v, corners[(k + 1) % n]);
                let (a, b) = edge.ends();
                let hard = mesh.is_hard(a, b);
                let crease = creases.get(&edge).copied().unwrap_or(0.0);
                let mut s = sharpness(crease, levels);
                if hard && hard_sharp {
                    s = ALWAYS;
                }
                sharp.push(s);
                marks.push(
                    (u8::from(hard) * HARD)
                        | (u8::from(seams.contains(&edge)) * SEAM_MARK)
                        | (u8::from(crease >= 1.0) * FULL_CREASE),
                );
                if let Some((default, list)) = &uv_values {
                    let uv = list.get(flat + k).copied().unwrap_or(*default);
                    let uv = if uv.is_finite() { uv } else { Vec2::ZERO };
                    // UVs within a hundred-thousandth of each other at the
                    // same point are one: the same place in the texture.
                    let key = (
                        i,
                        [(uv.x * 1e5).round() as i64, (uv.y * 1e5).round() as i64],
                    );
                    let u = *uv_index.entry(key).or_insert_with(|| {
                        uvs.push(uv);
                        (uvs.len() - 1) as u32
                    });
                    uv_layout.corners.push(u);
                }
            }
        }
        layout.starts.push(layout.corners.len() as u32);
        uv_layout.starts.push(uv_layout.corners.len() as u32);
        Ok(Self {
            points,
            layout,
            sharp,
            marks,
            uvs: uv_values.map(|_| (uvs, uv_layout)),
            face_from,
        })
    }

    /// The cage one level on.
    fn refined(self) -> Self {
        let uv_step = |(values, layout): &(Vec<Vec2>, Layout)| {
            let s = step(values, layout, &self.sharp, true);
            (s.values, s.layout)
        };
        // The UVs on a thread of their own when there is enough to do.
        let (stepped, uvs) = match &self.uvs {
            Some(uvs) if self.layout.corners.len() >= PARALLEL_FROM => {
                std::thread::scope(|scope| {
                    let handle = scope.spawn(|| uv_step(uvs));
                    let stepped = step(&self.points, &self.layout, &self.sharp, true);
                    let uvs = handle
                        .join()
                        .unwrap_or_else(|e| std::panic::resume_unwind(e));
                    (stepped, Some(uvs))
                })
            }
            uvs => (
                step(&self.points, &self.layout, &self.sharp, true),
                uvs.as_ref().map(uv_step),
            ),
        };
        let marks = stepped
            .edge_from
            .iter()
            .map(|from| from.map_or(0, |c| self.marks[c as usize]))
            .collect();
        let face_from = (0..self.layout.face_count())
            .flat_map(|f| {
                let n = self.layout.range(f).len();
                std::iter::repeat_n(self.face_from[f], n)
            })
            .collect();
        Self {
            points: stepped.values,
            layout: stepped.layout,
            sharp: stepped.sharp,
            marks,
            uvs,
            face_from,
        }
    }

    /// The new mesh, with `original`'s polygon values carried over.
    fn into_mesh(self, original: &Mesh) -> Mesh {
        let mut mesh = Mesh::new();
        let ids: Vec<VertexId> = self
            .points
            .iter()
            .filter_map(|p| mesh.add_vertex(*p).ok())
            .collect();
        let mut kept_faces: Vec<usize> = Vec::with_capacity(self.layout.face_count());
        let mut corners: Vec<VertexId> = Vec::new();
        for f in 0..self.layout.face_count() {
            corners.clear();
            corners.extend(
                self.layout.corners[self.layout.range(f)]
                    .iter()
                    .filter_map(|i| ids.get(*i as usize).copied()),
            );
            if mesh.add_face(&corners).is_ok() {
                kept_faces.push(f);
            }
        }
        // Edge marks, on the edges the new polygons have.
        let mut seams: Vec<(EdgeKey, Value)> = Vec::new();
        let mut creases: Vec<(EdgeKey, Value)> = Vec::new();
        for f in &kept_faces {
            let r = self.layout.range(*f);
            for c in r.clone() {
                let marks = self.marks[c];
                if marks == 0 {
                    continue;
                }
                let next = if c + 1 == r.end { r.start } else { c + 1 };
                let (Some(a), Some(b)) = (
                    ids.get(self.layout.corners[c] as usize),
                    ids.get(self.layout.corners[next] as usize),
                ) else {
                    continue;
                };
                if marks & HARD != 0 {
                    mesh.set_hard(*a, *b, true);
                }
                if marks & SEAM_MARK != 0 {
                    seams.push((EdgeKey::new(*a, *b), Value::Bool(true)));
                }
                if marks & FULL_CREASE != 0 {
                    creases.push((EdgeKey::new(*a, *b), Value::Float(1.0)));
                }
            }
        }
        if let Some((values, layout)) = &self.uvs {
            let list: Vec<Vec2> = kept_faces
                .iter()
                .flat_map(|f| layout.range(*f))
                .map(|c| values[layout.corners[c] as usize])
                .collect();
            let _ = mesh.set_attribute_values(
                UV,
                Domain::Corner,
                Value::Vec2(Vec2::ZERO),
                AttributeValues::Vec2(list),
            );
        }
        if !seams.is_empty() || original.attribute_values(SEAM).is_some() {
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
        let from: Vec<usize> = kept_faces
            .iter()
            .map(|f| self.face_from[*f] as usize)
            .collect();
        carry_face_values(original, &mut mesh, &from);
        mesh
    }
}

/// Gives each polygon of `mesh` the polygon values (every polygon layer)
/// of polygon slot `from[i]` of `original`.
pub(crate) fn carry_face_values(original: &Mesh, mesh: &mut Mesh, from: &[usize]) {
    let names: Vec<String> = original
        .attributes()
        .filter(|(_, domain, _)| *domain == Domain::Face)
        .map(|(name, _, _)| name.to_owned())
        .collect();
    for name in names {
        let Some((Domain::Face, default, values)) = original.attribute_values(&name) else {
            continue;
        };
        fn pick<T: Copy>(list: &[T], from: &[usize], default: T) -> Vec<T> {
            from.iter()
                .map(|i| list.get(*i).copied().unwrap_or(default))
                .collect()
        }
        let gathered = match (values, default) {
            (AttributeValues::Bool(l), Value::Bool(d)) => AttributeValues::Bool(pick(&l, from, d)),
            (AttributeValues::Int(l), Value::Int(d)) => AttributeValues::Int(pick(&l, from, d)),
            (AttributeValues::Float(l), Value::Float(d)) => {
                AttributeValues::Float(pick(&l, from, d))
            }
            (AttributeValues::Vec2(l), Value::Vec2(d)) => AttributeValues::Vec2(pick(&l, from, d)),
            (AttributeValues::Vec3(l), Value::Vec3(d)) => AttributeValues::Vec3(pick(&l, from, d)),
            (AttributeValues::Vec4(l), Value::Vec4(d)) => AttributeValues::Vec4(pick(&l, from, d)),
            _ => continue,
        };
        let _ = mesh.set_attribute_values(&name, Domain::Face, default, gathered);
    }
}
