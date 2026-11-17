//! A Sculpt layer's data (decision 0304): how far each point has moved,
//! measured along the surface, and the shape it was sculpted on.
//!
//! A [`Sculpt`] is a cheap handle: copies share everything, and a stroke
//! makes a new one that shares every chunk it didn't touch. It never
//! changes once made.

use crate::frames::{Frame, frame};
use crate::moves::{CHUNK, Chunk, Moves, same_chunk};
use crate::nearest::Grid;
use glam::Vec3;
use mesh::{Mesh, VertexId};
use std::sync::Arc;

/// The most points a sculpt may have (a file claiming more is damaged):
/// ten times the heavy test shape.
pub const MAX_POINTS: usize = 30_000_000;

/// One Sculpt layer's sculpting.
#[derive(Clone)]
pub struct Sculpt(Arc<Inner>);

#[derive(Clone)]
struct Inner {
    /// The structure of the shape it was sculpted on
    /// ([`Mesh::structure_fingerprint`]).
    structure: u64,
    /// Where that shape's points were, by slot (not a number for a slot
    /// with no point): for fitting the sculpt again when the shape under
    /// it changes its points.
    places: Arc<[Vec3]>,
    /// Each point's move, along its frame.
    moves: Moves,
}

/// How a sculpt met the shape it was applied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fit {
    /// The shape's points changed since it was sculpted, so each point took
    /// the moves of the nearest point it was sculpted on (the layer shows
    /// a badge).
    pub found_again: bool,
}

impl Sculpt {
    /// A sculpt with nothing moved yet, made for `mesh`.
    pub fn new(mesh: &Mesh) -> Self {
        let places: Vec<Vec3> = mesh
            .point_slots()
            .map(|p| p.unwrap_or(Vec3::NAN))
            .collect();
        Self(Arc::new(Inner {
            structure: mesh.structure_fingerprint(),
            moves: Moves::zero(places.len()),
            places: places.into(),
        }))
    }

    /// A sculpt from its parts (for files): `None` when they don't fit
    /// together (a move per place), there are too many, or a move isn't a
    /// real number.
    pub fn from_parts(structure: u64, places: Vec<Vec3>, moves: Moves) -> Option<Self> {
        if places.len() != moves.len() || places.len() > MAX_POINTS || !moves.is_valid() {
            return None;
        }
        Some(Self(Arc::new(Inner {
            structure,
            places: places.into(),
            moves,
        })))
    }

    /// The same sculpt with other moves (of the same length).
    pub fn with_moves(&self, moves: Moves) -> Option<Self> {
        (moves.len() == self.0.moves.len() && moves.is_valid()).then(|| {
            Self(Arc::new(Inner {
                structure: self.0.structure,
                places: Arc::clone(&self.0.places),
                moves,
            }))
        })
    }

    /// The structure of the shape it was sculpted on.
    pub fn structure(&self) -> u64 {
        self.0.structure
    }

    /// Where that shape's points were, by slot.
    pub fn places(&self) -> &[Vec3] {
        &self.0.places
    }

    /// Each point's move.
    pub fn moves(&self) -> &Moves {
        &self.0.moves
    }

    /// How many point slots it has.
    pub fn len(&self) -> usize {
        self.0.places.len()
    }

    /// Whether it has no points.
    pub fn is_empty(&self) -> bool {
        self.0.places.is_empty()
    }

    /// Whether every move is a real number.
    pub fn is_valid(&self) -> bool {
        self.0.moves.is_valid()
    }

    /// Whether it was sculpted on a shape with `mesh`'s points and
    /// polygons (the points may have moved since).
    pub fn fits(&self, mesh: &Mesh) -> bool {
        self.0.structure == mesh.structure_fingerprint()
            && self.0.places.len() == mesh.point_slot_count()
    }

    /// This sculpt for `mesh`: itself when it fits; otherwise each of
    /// `mesh`'s points takes the moves of the nearest point it was
    /// sculpted on (decision 0032's promise, kept by position), and the
    /// result is made for `mesh`. The flag says whether it was fitted.
    pub fn fitted_to(&self, mesh: &Mesh) -> (Self, bool) {
        if self.fits(mesh) {
            return (self.clone(), false);
        }
        let mut fitted = Self::new(mesh);
        let Some(grid) = Grid::new(&self.0.places) else {
            return (fitted, true);
        };
        if self.0.moves.is_zero() {
            return (fitted, true);
        }
        let len = fitted.len();
        let mut moves = Moves::zero(len);
        for c in 0..len.div_ceil(CHUNK) {
            let start = c * CHUNK;
            let chunk: Vec<Vec3> = (start..(start + CHUNK).min(len))
                .map(|i| {
                    let at = fitted.0.places[i];
                    grid.nearest(&self.0.places, at)
                        .map_or(Vec3::ZERO, |j| self.0.moves.get(j as usize))
                })
                .collect();
            moves.set_chunk(c, chunk);
        }
        if let Some(inner) = Arc::get_mut(&mut fitted.0) {
            inner.moves = moves;
        }
        (fitted, true)
    }

    /// Moves `mesh`'s points by the sculpt, each along its frame on
    /// `mesh` as it comes in. Points in chunks where nothing moved aren't
    /// looked at.
    pub fn apply(&self, mesh: &mut Mesh) -> Fit {
        let (sculpt, found_again) = self.fitted_to(mesh);
        let moved: Vec<usize> = sculpt
            .0
            .moves
            .chunks()
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_some())
            .flat_map(|(c, _)| c * CHUNK..((c + 1) * CHUNK).min(sculpt.len()))
            .collect();
        let new = sculpted_places(mesh, &moved, |i| sculpt.0.moves.get(i));
        for (i, p) in moved.into_iter().zip(new) {
            if let Some(p) = p {
                mesh.set_position(VertexId::from_raw(i as u32), p);
            }
        }
        Fit { found_again }
    }

    /// The bytes this sculpt holds that `other` doesn't share: what an undo
    /// step changing one into the other keeps.
    pub fn memory_apart(&self, other: &Self) -> usize {
        if Arc::ptr_eq(&self.0, &other.0) {
            return 0;
        }
        let places = if Arc::ptr_eq(&self.0.places, &other.0.places) {
            0
        } else {
            std::mem::size_of_val(&*self.0.places)
        };
        let ours = self.0.moves.chunks();
        let theirs = other.0.moves.chunks();
        let moves: usize = ours
            .iter()
            .enumerate()
            .filter(|(i, c)| !theirs.get(*i).is_some_and(|t| same_chunk(c, t)))
            .map(|(_, c)| chunk_bytes(c))
            .sum();
        places + moves
    }

    /// All the bytes it holds.
    pub fn memory_size(&self) -> usize {
        std::mem::size_of_val(&*self.0.places) + self.0.moves.memory_size()
    }
}

fn chunk_bytes(c: &Chunk) -> usize {
    c.as_ref().map_or(0, |c| std::mem::size_of_val(&**c))
}

/// Where points `which` (slots of `mesh`) go when each moves by `move_of`
/// along its frame on `mesh`; `None` for a slot with no point. Worked out
/// on every core for many points, all from `mesh` as it is.
pub fn sculpted_places(
    mesh: &Mesh,
    which: &[usize],
    move_of: impl Fn(usize) -> Vec3 + Sync,
) -> Vec<Option<Vec3>> {
    let topology = mesh.topology();
    mesh::parallel::in_parallel(which.len(), |range| {
        which[range]
            .iter()
            .map(|&i| {
                let v = VertexId::from_raw(i as u32);
                let p = mesh.position(v)?;
                Some(sculpted(p, frame(mesh, topology, v), move_of(i)))
            })
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// Where a point at `p` goes, moving by `m` along `frame`.
pub(crate) fn sculpted(p: Vec3, frame: Frame, m: Vec3) -> Vec3 {
    if m == Vec3::ZERO {
        return p;
    }
    p + frame.to_object(m)
}

impl PartialEq for Sculpt {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.structure == other.0.structure
                && (Arc::ptr_eq(&self.0.places, &other.0.places)
                    || same_places(&self.0.places, &other.0.places))
                && self.0.moves == other.0.moves)
    }
}

/// Places compared bit for bit (a slot with no point is not a number,
/// and equal to itself here).
fn same_places(a: &[Vec3], b: &[Vec3]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.to_array()
                .iter()
                .zip(y.to_array())
                .all(|(p, q)| p.to_bits() == q.to_bits())
        })
}

impl std::fmt::Debug for Sculpt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sculpt")
            .field("points", &self.len())
            .field("moves", &self.0.moves)
            .finish()
    }
}
