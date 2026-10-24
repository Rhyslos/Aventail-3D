//! A quick hasher for the hull builder's own maps, whose keys are small
//! numbers it hands out itself (places of points and faces). The standard
//! hasher is built to stand up to keys chosen to collide, which these
//! can't be, and took about a quarter of all the time of splitting a
//! shape into convex parts.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// Mixes each number in with a multiply, as the Rust compiler's own
/// `FxHasher` does.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct QuickHasher(u64);

impl QuickHasher {
    fn add(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl Hasher for QuickHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.add(u64::from(*b));
        }
    }

    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }
}

/// A map with the quick hasher.
pub(super) type QuickMap<K, V> = HashMap<K, V, BuildHasherDefault<QuickHasher>>;
/// A set with the quick hasher.
pub(super) type QuickSet<K> = HashSet<K, BuildHasherDefault<QuickHasher>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_of_places_spread_and_are_found_again() {
        let mut map: QuickMap<(usize, usize), usize> = QuickMap::default();
        for a in 0..200 {
            for b in 0..200 {
                map.insert((a, b), a * 1000 + b);
            }
        }
        assert_eq!(map.len(), 40_000);
        assert_eq!(map.get(&(17, 199)), Some(&17_199));
        assert_eq!(map.get(&(199, 17)), Some(&199_017));
        let hash = |key: (usize, usize)| {
            use std::hash::BuildHasher;
            BuildHasherDefault::<QuickHasher>::default().hash_one(key)
        };
        assert_ne!(hash((1, 2)), hash((2, 1)), "the order counts");
    }
}
