//! The mask (decision 0310): how much each point of a sculpt is held
//! still, from 0 (free) to 255 (fully masked), in chunks of [`CHUNK`]
//! points shared between copies, as the moves are. A masked point
//! doesn't move under any brush or the gizmo; a partly masked one moves
//! partly.
//!
//! What changes a whole mask at once is here as plain functions of the
//! shape and the mask: invert, grow and shrink by one ring of points,
//! sharpen and soften, and masks made from the creases (cavity), the open
//! edges and the way the surface faces.

use crate::frames::newell;
use crate::moves::{CHUNK, chunk_len};
use glam::Vec3;
use mesh::{Mesh, Topology, VertexId};
use std::sync::Arc;

/// Fully masked.
pub const FULL: u8 = 255;

/// One chunk's mask values, or `None` when none of its points is masked.
pub type MaskChunk = Option<Arc<[u8]>>;

/// Every point's mask value, by point slot.
#[derive(Clone, Default)]
pub struct Mask {
    len: usize,
    chunks: Vec<MaskChunk>,
}

impl Mask {
    /// Nothing masked, for `len` points.
    pub fn zero(len: usize) -> Self {
        Self {
            len,
            chunks: vec![None; len.div_ceil(CHUNK)],
        }
    }

    /// A mask from every point's value (all zero is kept as nothing).
    pub fn from_values(values: &[u8]) -> Self {
        let mut mask = Self::zero(values.len());
        for (i, part) in values.chunks(CHUNK).enumerate() {
            mask.set_chunk(i, part.to_vec());
        }
        mask
    }

    /// A mask from its chunks (for files). `None` if they don't fit `len`
    /// points.
    pub fn from_chunks(len: usize, chunks: Vec<MaskChunk>) -> Option<Self> {
        if chunks.len() != len.div_ceil(CHUNK) {
            return None;
        }
        for (i, chunk) in chunks.iter().enumerate() {
            if let Some(c) = chunk
                && c.len() != chunk_len(len, i)
            {
                return None;
            }
        }
        let mut mask = Self { len, chunks };
        for i in 0..mask.chunks.len() {
            mask.tidy(i);
        }
        Some(mask)
    }

    /// How many points.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no points.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The chunks, in order.
    pub fn chunks(&self) -> &[MaskChunk] {
        &self.chunks
    }

    /// Point `i`'s value (0 past the end).
    pub fn get(&self, i: usize) -> u8 {
        self.chunks
            .get(i / CHUNK)
            .and_then(|c| c.as_ref())
            .and_then(|c| c.get(i % CHUNK))
            .copied()
            .unwrap_or(0)
    }

    /// How free point `i` is to move: 1 unmasked, 0 fully masked.
    pub fn free(&self, i: usize) -> f32 {
        1.0 - f32::from(self.get(i)) / f32::from(FULL)
    }

    /// Replaces chunk `index` (all zero is kept as nothing). Ignored if
    /// the index or length is wrong.
    pub fn set_chunk(&mut self, index: usize, values: Vec<u8>) {
        if index >= self.chunks.len() || values.len() != chunk_len(self.len, index) {
            return;
        }
        self.chunks[index] = Some(values.into());
        self.tidy(index);
    }

    /// Every point's value, in one list.
    pub fn values(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len);
        for (i, c) in self.chunks.iter().enumerate() {
            match c {
                Some(c) => out.extend_from_slice(c),
                None => out.resize(out.len() + chunk_len(self.len, i), 0),
            }
        }
        out
    }

    /// Whether nothing is masked.
    pub fn is_zero(&self) -> bool {
        self.chunks.iter().all(Option::is_none)
    }

    /// Bytes held by chunks with values.
    pub fn memory_size(&self) -> usize {
        self.chunks.iter().flatten().map(|c| c.len()).sum()
    }

    /// Which chunks differ from `other`'s, comparing only chunks that
    /// aren't shared.
    pub fn differing(&self, other: &Self) -> Vec<usize> {
        if self.len != other.len {
            return (0..self.chunks.len().max(other.chunks.len())).collect();
        }
        self.chunks
            .iter()
            .zip(&other.chunks)
            .enumerate()
            .filter(|(_, (a, b))| !same_chunk(a, b))
            .map(|(i, _)| i)
            .collect()
    }

    fn tidy(&mut self, index: usize) {
        if let Some(Some(c)) = self.chunks.get(index)
            && c.iter().all(|m| *m == 0)
        {
            self.chunks[index] = None;
        }
    }
}

/// Whether two chunks hold the same values, without comparing shared
/// ones.
pub fn same_chunk(a: &MaskChunk, b: &MaskChunk) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => Arc::ptr_eq(x, y) || x == y,
        _ => false,
    }
}

impl PartialEq for Mask {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self
                .chunks
                .iter()
                .zip(&other.chunks)
                .all(|(a, b)| same_chunk(a, b))
    }
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mask")
            .field("points", &self.len)
            .field("chunks_masked", &self.chunks.iter().flatten().count())
            .finish()
    }
}

/// `mask` with every value turned over: masked becomes free and free
/// masked (Ctrl + I).
pub fn invert(mask: &Mask) -> Mask {
    let values: Vec<u8> = mask.values().into_iter().map(|v| FULL - v).collect();
    Mask::from_values(&values)
}

/// Each point's value and its neighbours' put together by `pick` (the
/// largest to grow, the smallest to shrink), on every core.
fn by_ring(mesh: &Mesh, mask: &Mask, pick: fn(u8, u8) -> u8) -> Mask {
    let values = mask.values();
    let topology = mesh.topology();
    let out = mesh::parallel::in_parallel(values.len(), |range| {
        range
            .map(|i| {
                let v = VertexId::from_raw(i as u32);
                let mut value = values[i];
                if mesh.position(v).is_none() {
                    return value;
                }
                for e in topology.vertex_edges(v) {
                    let other = mesh::topology::other_end(*e, v).get() as usize;
                    if let Some(n) = values.get(other) {
                        value = pick(value, *n);
                    }
                }
                value
            })
            .collect::<Vec<u8>>()
    });
    Mask::from_values(&out.concat())
}

/// `mask` grown by one ring of points (Ctrl + wheel up): each point takes
/// the most masked of itself and its neighbours.
pub fn grow(mesh: &Mesh, mask: &Mask) -> Mask {
    by_ring(mesh, mask, u8::max)
}

/// `mask` shrunk by one ring of points (Ctrl + wheel down): each point
/// takes the least masked of itself and its neighbours.
pub fn shrink(mesh: &Mesh, mask: &Mask) -> Mask {
    by_ring(mesh, mask, u8::min)
}

/// `mask` with its soft edge made harder: values under a half go
/// towards free, over a half towards masked.
pub fn sharpen(mask: &Mask) -> Mask {
    let values: Vec<u8> = mask
        .values()
        .into_iter()
        .map(|v| {
            let x = f32::from(v) / 255.0;
            let y = ((x - 0.5) * 2.0 + 0.5).clamp(0.0, 1.0);
            (y * 255.0).round() as u8
        })
        .collect();
    Mask::from_values(&values)
}

/// `mask` with its edge softened: each point takes the average of itself
/// and its neighbours.
pub fn soften(mesh: &Mesh, mask: &Mask) -> Mask {
    let values = mask.values();
    let topology = mesh.topology();
    let out = mesh::parallel::in_parallel(values.len(), |range| {
        range
            .map(|i| {
                let v = VertexId::from_raw(i as u32);
                let (mut sum, mut n) = (u32::from(values[i]), 1u32);
                if mesh.position(v).is_some() {
                    for e in topology.vertex_edges(v) {
                        let other = mesh::topology::other_end(*e, v).get() as usize;
                        if let Some(x) = values.get(other) {
                            sum += u32::from(*x);
                            n += 1;
                        }
                    }
                }
                ((sum + n / 2) / n) as u8
            })
            .collect::<Vec<u8>>()
    });
    Mask::from_values(&out.concat())
}

/// A mask of the creases (cavity): each point masked by how far it sits
/// below its neighbours' middle, against the length of its edges. Deep
/// creases are fully masked, flat and bulging parts free.
pub fn from_cavity(mesh: &Mesh) -> Mask {
    let topology = mesh.topology();
    let slots = mesh.point_slot_count();
    let out = mesh::parallel::in_parallel(slots, |range| {
        range
            .map(|i| {
                let v = VertexId::from_raw(i as u32);
                cavity(mesh, topology, v).map_or(0, |c| {
                    // A dip of a tenth of the edges' length is full.
                    ((c * 10.0).clamp(0.0, 1.0) * 255.0).round() as u8
                })
            })
            .collect::<Vec<u8>>()
    });
    Mask::from_values(&out.concat())
}

/// How far point `v` dips below its neighbours' middle, along its facing,
/// against its edges' average length (positive in a crease).
fn cavity(mesh: &Mesh, topology: &Topology, v: VertexId) -> Option<f32> {
    let p = mesh.position(v)?;
    let normal = facing(mesh, topology, v)?;
    let (mut sum, mut length, mut n) = (Vec3::ZERO, 0.0, 0.0);
    for e in topology.vertex_edges(v) {
        let q = mesh.position(mesh::topology::other_end(*e, v))?;
        sum += q;
        length += q.distance(p);
        n += 1.0;
    }
    if n == 0.0 || length <= 0.0 {
        return None;
    }
    let middle = sum / n;
    Some((middle - p).dot(normal) / (length / n))
}

/// Point `v`'s area-weighted facing.
fn facing(mesh: &Mesh, topology: &Topology, v: VertexId) -> Option<Vec3> {
    topology
        .vertex_faces(v)
        .iter()
        .filter_map(|f| mesh.face(*f))
        .map(|c| newell(mesh, c))
        .sum::<Vec3>()
        .try_normalize()
}

/// A mask of the open edges: every point on the border of an open shape
/// fully masked (an empty mask for a closed shape).
pub fn from_edges(mesh: &Mesh) -> Mask {
    let topology = mesh.topology();
    let mut values = vec![0u8; mesh.point_slot_count()];
    for e in topology.border_edges() {
        let (a, b) = e.ends();
        for v in [a, b] {
            if let Some(x) = values.get_mut(v.get() as usize) {
                *x = FULL;
            }
        }
    }
    Mask::from_values(&values)
}

/// A mask of the parts facing `way` (in the shape's own space): fully
/// masked where the surface faces it within about 45 degrees, fading out
/// to free where it faces square to it.
pub fn from_facing(mesh: &Mesh, way: Vec3) -> Mask {
    let Some(way) = way.try_normalize() else {
        return Mask::zero(mesh.point_slot_count());
    };
    let topology = mesh.topology();
    let slots = mesh.point_slot_count();
    let out = mesh::parallel::in_parallel(slots, |range| {
        range
            .map(|i| {
                let v = VertexId::from_raw(i as u32);
                facing(mesh, topology, v).map_or(0, |n| {
                    let x = (n.dot(way) / std::f32::consts::FRAC_1_SQRT_2).clamp(0.0, 1.0);
                    (x * 255.0).round() as u8
                })
            })
            .collect::<Vec<u8>>()
    });
    Mask::from_values(&out.concat())
}

/// `mask` with every point for which `inside` holds set to `value`
/// (the box and lasso masks): `inside` is given each point's place.
pub fn set_where(
    mesh: &Mesh,
    mask: &Mask,
    value: u8,
    inside: impl Fn(Vec3) -> bool + Sync,
) -> Mask {
    let values = mask.values();
    let out = mesh::parallel::in_parallel(values.len(), |range| {
        range
            .map(|i| match mesh.position(VertexId::from_raw(i as u32)) {
                Some(p) if inside(p) => value,
                _ => values[i],
            })
            .collect::<Vec<u8>>()
    });
    Mask::from_values(&out.concat())
}
