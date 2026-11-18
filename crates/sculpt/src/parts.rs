//! Sharing a dab's work between the processor's cores in fixed blocks
//! (decision 0313), so a sum over the points or polygons in reach (the
//! plane under the brush, the way it faces) comes out the same to the
//! last bit whatever the number of cores: each block is added up on its
//! own, and the blocks are added together in order.

use glam::Vec3;
use std::ops::Range;

/// Items in a block. A block is the smallest piece of work one core
/// takes.
pub(crate) const BLOCK: usize = 4096;
/// The fewest blocks a core takes when the work is shared.
const BLOCKS_PER_CORE: usize = 2;

/// `work` on each block of `0..count`, in order: on every core when there
/// are enough blocks, otherwise here.
pub(crate) fn in_blocks<T: Send>(count: usize, work: impl Fn(Range<usize>) -> T + Sync) -> Vec<T> {
    let blocks = count.div_ceil(BLOCK);
    mesh::parallel::in_parallel_from(blocks, BLOCKS_PER_CORE, |range| {
        range
            .map(|b| work(b * BLOCK..((b + 1) * BLOCK).min(count)))
            .collect::<Vec<T>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// The sum of `of` over `0..count`, the same on any number of cores.
pub(crate) fn sum(count: usize, of: impl Fn(usize) -> Vec3 + Sync) -> Vec3 {
    in_blocks(count, |range| range.map(&of).sum::<Vec3>())
        .into_iter()
        .sum()
}

/// `work` on each of `items`, on every core when there are many, the
/// results in order.
pub(crate) fn each<T: Sync, U: Send>(items: &[T], work: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let parts = in_blocks(items.len(), |range| {
        items[range].iter().map(&work).collect::<Vec<U>>()
    });
    if parts.len() == 1 {
        return parts.into_iter().next().unwrap_or_default();
    }
    let mut out = Vec::with_capacity(items.len());
    for part in parts {
        out.extend(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_cover_every_item_once_in_order() {
        for count in [0, 1, BLOCK - 1, BLOCK, BLOCK * 5 + 3] {
            let all: Vec<usize> = in_blocks(count, |r| r.collect::<Vec<_>>())
                .into_iter()
                .flatten()
                .collect();
            assert_eq!(all, (0..count).collect::<Vec<_>>());
            let doubled = each(&all, |i| i * 2);
            assert_eq!(doubled, all.iter().map(|i| i * 2).collect::<Vec<_>>());
        }
    }

    #[test]
    fn sums_are_added_block_by_block() {
        let count = BLOCK * 3 + 17;
        let of = |i: usize| Vec3::new(i as f32 * 0.1, 1.0, -(i as f32).sqrt());
        let mut expected = Vec3::ZERO;
        for b in 0..count.div_ceil(BLOCK) {
            let part: Vec3 = (b * BLOCK..((b + 1) * BLOCK).min(count)).map(of).sum();
            expected += part;
        }
        assert_eq!(sum(count, of), expected);
        assert_eq!(sum(0, of), Vec3::ZERO);
    }
}
