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
    in_blocks_of(count, BLOCK, work)
}

/// [`in_blocks`] with `block` items to a block, for items that each take
/// more work (a leaf of the tree holds several polygons).
pub(crate) fn in_blocks_of<T: Send>(
    count: usize,
    block: usize,
    work: impl Fn(Range<usize>) -> T + Sync,
) -> Vec<T> {
    let block = block.max(1);
    let blocks = count.div_ceil(block);
    mesh::parallel::in_parallel_from(blocks, BLOCKS_PER_CORE, |range| {
        range
            .map(|b| work(b * block..((b + 1) * block).min(count)))
            .collect::<Vec<T>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// The sum of `values` in order, the same wherever they were worked out:
/// each block of [`BLOCK`] is added up on its own, and the blocks are
/// added together in order. For values worked out on every core
/// beforehand (each polygon's facing as a dab finds it), so adding them
/// needs no pass of its own over the shape.
pub(crate) fn sum_listed(values: impl Iterator<Item = Vec3>) -> Vec3 {
    let (mut total, mut block, mut held) = (Vec3::ZERO, Vec3::ZERO, 0);
    for value in values {
        block += value;
        held += 1;
        if held == BLOCK {
            total += block;
            (block, held) = (Vec3::ZERO, 0);
        }
    }
    if held > 0 {
        total += block;
    }
    total
}

/// `work` on each of `0..count`, on every core when there are many, the
/// results in order.
pub(crate) fn each_of<U: Send>(count: usize, work: impl Fn(usize) -> U + Sync) -> Vec<U> {
    let parts = in_blocks(count, |range| range.map(&work).collect::<Vec<U>>());
    if parts.len() == 1 {
        return parts.into_iter().next().unwrap_or_default();
    }
    let mut out = Vec::with_capacity(count);
    for part in parts {
        out.extend(part);
    }
    out
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
            let by_seven: Vec<usize> = in_blocks_of(count, 7, |r| r.collect::<Vec<_>>())
                .into_iter()
                .flatten()
                .collect();
            assert_eq!(by_seven, all);
        }
    }

    #[test]
    fn a_list_is_added_up_block_by_block() {
        // Values that round differently added in another order.
        let of = |i: usize| {
            Vec3::new(
                1e-3 + i as f32 * 1e4,
                (i as f32).sin() * 1e-5,
                1.0 / 3.0,
            )
        };
        for count in [0, 1, BLOCK - 1, BLOCK, BLOCK + 1, BLOCK * 7 + 19] {
            let mut expected = Vec3::ZERO;
            for b in 0..count.div_ceil(BLOCK) {
                let part: Vec3 = (b * BLOCK..((b + 1) * BLOCK).min(count)).map(of).sum();
                expected += part;
            }
            let listed = sum_listed((0..count).map(of));
            assert_eq!(
                listed.to_array().map(f32::to_bits),
                expected.to_array().map(f32::to_bits),
                "{count}"
            );
        }
    }

    #[test]
    fn each_of_gives_every_result_in_order() {
        for count in [0, 1, BLOCK, BLOCK * 5 + 3] {
            assert_eq!(each_of(count, |i| i * 3), (0..count).map(|i| i * 3).collect::<Vec<_>>());
        }
    }
}
