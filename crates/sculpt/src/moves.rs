//! How far each point of a sculpt has moved, in chunks of [`CHUNK`]
//! points shared between copies (decision 0304): a stroke copies only the
//! chunks it touches, so its undo step holds those chunks and nothing
//! else, as a paint stroke holds only its tiles.

use glam::Vec3;
use std::sync::Arc;

/// Points per chunk: moves, undo and saving all work in chunks this size.
pub const CHUNK: usize = 16_384;

/// One chunk's moves, or `None` when none of its points moved (a fresh
/// sculpt holds no memory at all).
pub type Chunk = Option<Arc<[Vec3]>>;

/// Every point's move, measured along the surface (decision 0304: out
/// along the point's facing, and sideways along it), by point slot.
#[derive(Clone, Default)]
pub struct Moves {
    len: usize,
    chunks: Vec<Chunk>,
}

impl Moves {
    /// No moves for `len` points.
    pub fn zero(len: usize) -> Self {
        Self {
            len,
            chunks: vec![None; len.div_ceil(CHUNK)],
        }
    }

    /// Moves from their chunks (for files). `None` if the chunks don't
    /// fit `len` points: one too many or too few, or a chunk of the wrong
    /// length (the last may be short).
    pub fn from_chunks(len: usize, chunks: Vec<Chunk>) -> Option<Self> {
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
        let mut moves = Self { len, chunks };
        for i in 0..moves.chunks.len() {
            moves.tidy(i);
        }
        Some(moves)
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
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Point `i`'s move (none past the end).
    pub fn get(&self, i: usize) -> Vec3 {
        self.chunks
            .get(i / CHUNK)
            .and_then(|c| c.as_ref())
            .and_then(|c| c.get(i % CHUNK))
            .copied()
            .unwrap_or(Vec3::ZERO)
    }

    /// Replaces chunk `index` with `moves` (all zero is kept as nothing).
    /// Ignored if the index or length is wrong.
    pub fn set_chunk(&mut self, index: usize, moves: Vec<Vec3>) {
        if index >= self.chunks.len() || moves.len() != chunk_len(self.len, index) {
            return;
        }
        self.chunks[index] = Some(moves.into());
        self.tidy(index);
    }

    /// Whether every move is a real number.
    pub fn is_valid(&self) -> bool {
        self.chunks
            .iter()
            .flatten()
            .all(|c| c.iter().all(|m| m.is_finite()))
    }

    /// Whether no point moved.
    pub fn is_zero(&self) -> bool {
        self.chunks.iter().all(Option::is_none)
    }

    /// Bytes held by chunks with moves.
    pub fn memory_size(&self) -> usize {
        self.chunks
            .iter()
            .flatten()
            .map(|c| std::mem::size_of_val(&**c))
            .sum()
    }

    /// Which chunks differ from `other`'s (of the same length), comparing
    /// only chunks that aren't shared.
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

    /// A chunk of all zeros becomes nothing.
    fn tidy(&mut self, index: usize) {
        if let Some(Some(c)) = self.chunks.get(index)
            && c.iter().all(|m| *m == Vec3::ZERO)
        {
            self.chunks[index] = None;
        }
    }
}

/// How many points chunk `index` of `len` points holds.
pub fn chunk_len(len: usize, index: usize) -> usize {
    len.saturating_sub(index * CHUNK).min(CHUNK)
}

/// Whether two chunks hold the same moves, without comparing shared ones.
pub fn same_chunk(a: &Chunk, b: &Chunk) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => Arc::ptr_eq(x, y) || x == y,
        _ => false,
    }
}

impl PartialEq for Moves {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self
                .chunks
                .iter()
                .zip(&other.chunks)
                .all(|(a, b)| same_chunk(a, b))
    }
}

impl std::fmt::Debug for Moves {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Moves")
            .field("points", &self.len)
            .field("chunks_moved", &self.chunks.iter().flatten().count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_moves_hold_nothing_and_read_zero() {
        let m = Moves::zero(CHUNK * 2 + 5);
        assert_eq!(m.chunks().len(), 3);
        assert!(m.is_zero());
        assert_eq!(m.memory_size(), 0);
        assert_eq!(m.get(CHUNK + 3), Vec3::ZERO);
        assert_eq!(m.get(10 * CHUNK), Vec3::ZERO);
    }

    #[test]
    fn a_chunk_set_reads_back_and_zeros_are_dropped() {
        let mut m = Moves::zero(CHUNK + 5);
        let mut last = vec![Vec3::ZERO; 5];
        last[2] = Vec3::new(1.0, 2.0, 3.0);
        m.set_chunk(1, last);
        assert_eq!(m.get(CHUNK + 2), Vec3::new(1.0, 2.0, 3.0));
        m.set_chunk(0, vec![Vec3::ZERO; CHUNK]);
        assert!(m.chunks()[0].is_none());
        // Wrong lengths are ignored.
        m.set_chunk(1, vec![Vec3::ONE; 4]);
        m.set_chunk(7, vec![Vec3::ONE; CHUNK]);
        assert_eq!(m.get(CHUNK + 2), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn only_unshared_chunks_differ() {
        let mut a = Moves::zero(3 * CHUNK);
        a.set_chunk(0, vec![Vec3::X; CHUNK]);
        let mut b = a.clone();
        assert!(b.differing(&a).is_empty());
        assert_eq!(a, b);
        b.set_chunk(2, vec![Vec3::Y; CHUNK]);
        assert_eq!(b.differing(&a), vec![2]);
        assert_ne!(a, b);
    }

    #[test]
    fn chunks_that_dont_fit_are_refused() {
        assert!(Moves::from_chunks(10, vec![]).is_none());
        assert!(Moves::from_chunks(10, vec![Some(vec![Vec3::ONE; 9].into())]).is_none());
        let ok = Moves::from_chunks(10, vec![Some(vec![Vec3::ONE; 10].into())]).unwrap();
        assert_eq!(ok.get(9), Vec3::ONE);
        let mut bad = vec![Vec3::ONE; 10];
        bad[3] = Vec3::NAN;
        assert!(
            !Moves::from_chunks(10, vec![Some(bad.into())])
                .unwrap()
                .is_valid()
        );
    }
}
