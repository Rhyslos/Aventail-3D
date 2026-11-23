//! The shading round points that moved, for the hundreds of thousands a
//! sculpting dab moves each frame (decision 0313).
//!
//! [`super::group_normals_each`] works a point's normals out from the polygons
//! round it: each polygon's facing and the angle at its corner. Asked for
//! every point a dab reshades, that works each polygon's facing out once
//! per corner, four times for a quad. [`reshade_moved`] turns it round:
//! each polygon that turned is looked at once (its corners' places, its
//! facing, the angle at every corner), on every core, and each point then
//! only adds up what its polygons hold. The normals are the same to the
//! last bit.

use super::{corner_at, corner_weight};
use crate::polygon::{FaceId, Mesh, VertexId};
use glam::Vec3;

/// Polygons to a block of the work: the smallest piece one core takes,
/// and how the turned polygons' weights are kept (a block's in one list).
const BLOCK: usize = 4096;
/// The fewest blocks a core takes when the work is shared.
const BLOCKS_PER_CORE: usize = 2;
/// Below this many points per core, adding the weights up stays on one
/// core.
const POINTS_PER_CORE: usize = 8192;

/// A list of bits that also says how many are set before any of them.
struct Bits {
    words: Vec<u64>,
    /// How many bits are set in the words before each word.
    before: Vec<u32>,
}

impl Bits {
    fn new(count: usize) -> Self {
        Self {
            words: vec![0; count.div_ceil(64)],
            before: Vec::new(),
        }
    }

    fn set(&mut self, i: usize) {
        if let Some(word) = self.words.get_mut(i / 64) {
            *word |= 1 << (i % 64);
        }
    }

    /// Every set bit, in order; counts them so [`Self::rank`] can answer.
    fn listed(&mut self) -> Vec<u32> {
        let mut out = Vec::new();
        self.before = Vec::with_capacity(self.words.len());
        for (w, word) in self.words.iter().enumerate() {
            self.before.push(out.len() as u32);
            let mut word = *word;
            while word != 0 {
                out.push((w * 64) as u32 + word.trailing_zeros());
                word &= word - 1;
            }
        }
        out
    }

    /// Where bit `i` is among the set bits ([`Self::listed`] first), if
    /// it is set.
    fn rank(&self, i: usize) -> Option<usize> {
        let (word, before) = (*self.words.get(i / 64)?, *self.before.get(i / 64)?);
        let bit = 1u64 << (i % 64);
        (word & bit != 0).then(|| before as usize + (word & (bit - 1)).count_ones() as usize)
    }
}

/// What one block of turned polygons holds: each polygon's facing, where
/// its corners' weights start, and every corner's weight (the facing
/// times the angle the polygon fills there).
struct Block {
    normals: Vec<Vec3>,
    starts: Vec<u32>,
    weights: Vec<Vec3>,
}

/// The new shading normals after `moved` points of `mesh` moved: `found`
/// is given the first corner (as [`super::smooth_groups`] names it, in
/// `groups`) and the normal of every smooth group at every point whose
/// shading changes, the points in id order. Exactly what
/// [`super::group_normals_each`] gives for each point of
/// [`super::shading_reach`]: every corner of every polygon with a moved
/// corner (the polygons turn, and the angles at their corners change).
pub fn reshade_moved(
    mesh: &Mesh,
    groups: &[u32],
    moved: &[VertexId],
    mut found: impl FnMut(u32, Vec3),
) {
    let topology = mesh.topology();
    let (starts, corners) = mesh.face_layout();
    let slots = starts.len().saturating_sub(1);
    // The polygons with a moved corner, each once, in order.
    let mut turned = Bits::new(slots);
    for v in moved {
        for face in topology.vertex_faces(*v) {
            turned.set(face.get() as usize);
        }
    }
    let faces = turned.listed();
    // Each turned polygon looked at once, on every core.
    let blocks: Vec<Block> =
        crate::parallel::in_parallel_from(faces.len().div_ceil(BLOCK), BLOCKS_PER_CORE, |range| {
            range
                .map(|b| block_of(mesh, &faces[b * BLOCK..((b + 1) * BLOCK).min(faces.len())]))
                .collect::<Vec<_>>()
        })
        .into_iter()
        .flatten()
        .collect();
    // The points whose shading changes: every corner of those polygons (a
    // moved point that no polygon uses has no normal to work out).
    let mut reach = Bits::new(mesh.point_slot_count());
    for &f in &faces {
        let f = f as usize;
        for c in &corners[starts[f] as usize..starts[f + 1] as usize] {
            reach.set(c.get() as usize);
        }
    }
    let reach = reach.listed();
    // Each point adds up what its polygons hold, on every core. A polygon
    // round it that didn't turn (at the rim) is worked out as before.
    let turned = &turned;
    let blocks = &blocks;
    let parts = crate::parallel::in_parallel_from(reach.len(), POINTS_PER_CORE, |range| {
        let mut out = Vec::with_capacity(range.len());
        for &point in &reach[range] {
            let v = VertexId::from_raw(point);
            let at_hand = |face: FaceId, corner: usize| {
                let f = face.get() as usize;
                let j = turned.rank(f)?;
                let block = blocks.get(j / BLOCK)?;
                let first = *block.starts.get(j % BLOCK)? as usize;
                let weight = *block.weights.get(first + corner - starts[f] as usize)?;
                Some((weight, block.normals[j % BLOCK]))
            };
            group_normals_with(mesh, groups, v, at_hand, |rep, normal| {
                out.push((rep, normal));
            });
        }
        out
    });
    for (rep, normal) in parts.into_iter().flatten() {
        found(rep, normal);
    }
}

/// The facings and corner weights of the polygons `faces`.
fn block_of(mesh: &Mesh, faces: &[u32]) -> Block {
    let (starts, corners) = mesh.face_layout();
    let mut block = Block {
        normals: Vec::with_capacity(faces.len()),
        starts: Vec::with_capacity(faces.len()),
        weights: Vec::with_capacity(faces.len() * 4),
    };
    let mut at: Vec<Vec3> = Vec::new();
    for &f in faces {
        let normal = mesh.face_normal(FaceId::from_raw(f));
        let own = &corners[starts[f as usize] as usize..starts[f as usize + 1] as usize];
        at.clear();
        at.extend(
            own.iter()
                .map(|c| mesh.position(*c).unwrap_or(Vec3::ZERO)),
        );
        let n = at.len();
        block.starts.push(block.weights.len() as u32);
        block.normals.push(normal);
        for i in 0..n {
            block
                .weights
                .push(corner_weight(normal, at[(i + n - 1) % n], at[i], at[(i + 1) % n]));
        }
    }
    block
}

/// [`super::group_normals_each`] for point `v`, taking the weight at a polygon's
/// corner (and the polygon's facing) from `at_hand` when it has them.
fn group_normals_with(
    mesh: &Mesh,
    groups: &[u32],
    v: VertexId,
    at_hand: impl Fn(FaceId, usize) -> Option<(Vec3, Vec3)>,
    mut found: impl FnMut(u32, Vec3),
) {
    let topology = mesh.topology();
    // Almost every point has one group: kept here, not in a list. With
    // each group's sum, the facing of its first polygon (for a sum of no
    // length), when it is at hand.
    let mut first: Option<(u32, Vec3, FaceId, Option<Vec3>)> = None;
    let mut sums: Vec<(u32, Vec3, FaceId, Option<Vec3>)> = Vec::new();
    for face in topology.vertex_faces(v) {
        let Some(k) = corner_at(mesh, *face, v) else {
            continue;
        };
        let (weighted, normal) = match at_hand(*face, k) {
            Some((weight, normal)) => (weight, Some(normal)),
            None => (super::weight_at(mesh, *face, v), None),
        };
        let rep = groups.get(k).copied().unwrap_or(k as u32);
        match &mut first {
            Some(entry) if entry.0 == rep => entry.1 += weighted,
            None => first = Some((rep, weighted, *face, normal)),
            Some(_) => match sums.iter_mut().find(|(r, ..)| *r == rep) {
                Some(entry) => entry.1 += weighted,
                None => sums.push((rep, weighted, *face, normal)),
            },
        }
    }
    for (rep, sum, face, normal) in first.into_iter().chain(sums) {
        found(
            rep,
            sum.try_normalize()
                .unwrap_or_else(|| normal.unwrap_or_else(|| mesh.face_normal(face))),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shading::{Hardness, group_normals_each, shading_reach, smooth_groups_with};

    /// What working each point out on its own gives: the reference.
    fn one_by_one(mesh: &Mesh, groups: &[u32], moved: &[VertexId]) -> Vec<(u32, [u32; 3])> {
        let mut out = Vec::new();
        for v in shading_reach(mesh, moved) {
            group_normals_each(mesh, groups, v, |rep, n| {
                out.push((rep, n.to_array().map(f32::to_bits)));
            });
        }
        out
    }

    fn at_once(mesh: &Mesh, groups: &[u32], moved: &[VertexId]) -> Vec<(u32, [u32; 3])> {
        let mut out = Vec::new();
        reshade_moved(mesh, groups, moved, |rep, n| {
            out.push((rep, n.to_array().map(f32::to_bits)));
        });
        out
    }

    /// A deterministic scramble of `i`.
    fn noise(i: u32) -> f32 {
        let x = i.wrapping_mul(0x9E37_79B9).rotate_left(13) ^ 0x5bd1_e995;
        (x % 2001) as f32 / 1000.0 - 1.0
    }

    #[test]
    fn the_normals_are_bit_for_bit_what_each_point_alone_gives() {
        // Enough polygons for several blocks and cores, shaded both ways,
        // with a few points, a patch, and everything moved.
        let mut mesh = crate::samples::heavy_ball(40_000, 100.0);
        let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
        let pushed: Vec<(VertexId, Vec3)> = mesh
            .vertices()
            .map(|(v, p)| (v, p + Vec3::new(noise(v.get()), noise(v.get() + 7), 0.3)))
            .collect();
        mesh.set_positions(&pushed);
        for hardness in [Hardness::Marked, Hardness::ByAngle(30.0)] {
            let groups = smooth_groups_with(&mesh, hardness);
            for moved in [
                &all[..0],
                &all[100..103],
                &all[5_000..17_000],
                &all[..],
            ] {
                assert_eq!(
                    at_once(&mesh, &groups, moved),
                    one_by_one(&mesh, &groups, moved),
                    "{hardness:?}, {} moved",
                    moved.len()
                );
            }
        }
    }

    #[test]
    fn hard_edges_polygons_with_many_corners_and_loose_points_agree_too() {
        // A cube (hard edges all round: several groups a point), a
        // cylinder (caps of many corners) and a point no polygon uses.
        for kind in [crate::PrimitiveKind::Cube, crate::PrimitiveKind::Cylinder] {
            let mut mesh = kind.standard().mesh();
            let loose = mesh.add_vertex(Vec3::splat(3.0)).expect("a point");
            let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
            let groups = smooth_groups_with(&mesh, Hardness::Marked);
            for moved in [&all[..], &all[..1], &[loose][..]] {
                assert_eq!(
                    at_once(&mesh, &groups, moved),
                    one_by_one(&mesh, &groups, moved)
                );
            }
        }
    }

    #[test]
    fn polygons_with_no_area_fall_back_as_before() {
        // Every point of a polygon in one place: its sum has no length.
        let mut mesh = crate::PrimitiveKind::Plane.standard().mesh();
        let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
        let squashed: Vec<(VertexId, Vec3)> = all.iter().map(|v| (*v, Vec3::ONE)).collect();
        mesh.set_positions(&squashed);
        let groups = smooth_groups_with(&mesh, Hardness::Marked);
        assert_eq!(
            at_once(&mesh, &groups, &all),
            one_by_one(&mesh, &groups, &all)
        );
        // Points that aren't in the mesh are passed over.
        let gone = [VertexId::from_raw(9_999_999)];
        assert_eq!(at_once(&mesh, &groups, &gone), Vec::new());
    }

    #[test]
    fn bits_list_and_rank_in_order() {
        let mut bits = Bits::new(200);
        for i in [3usize, 64, 65, 190, 199, 500] {
            bits.set(i);
        }
        assert_eq!(bits.listed(), vec![3, 64, 65, 190, 199]);
        assert_eq!(bits.rank(3), Some(0));
        assert_eq!(bits.rank(65), Some(2));
        assert_eq!(bits.rank(199), Some(4));
        assert_eq!(bits.rank(4), None);
        assert_eq!(bits.rank(5000), None);
    }
}
