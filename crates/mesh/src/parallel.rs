//! Sharing heavy work between the computer's cores, for meshes of
//! millions of polygons ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).

use std::cell::Cell;
use std::sync::OnceLock;

/// Below this many items per thread, splitting the work isn't worth it.
const PER_THREAD: usize = 100_000;

thread_local! {
    /// The number of cores work started from this thread is shared
    /// between, while [`with_cores`] runs; 0 otherwise.
    static PRETEND: Cell<usize> = const { Cell::new(0) };
}

/// How many cores heavy work is shared between: the computer's. Asked
/// once and kept: asking takes a moment, and a sculpting dab shares its
/// work out many times a frame.
pub fn cores() -> usize {
    static CORES: OnceLock<usize> = OnceLock::new();
    match PRETEND.get() {
        0 => *CORES.get_or_init(|| {
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        }),
        pretend => pretend,
    }
}

/// Runs `work` with the heavy work it starts shared out as on a computer
/// with `cores` cores (1 or more), whatever this one has: for checking
/// that a result is the same on any number of cores.
pub fn with_cores<T>(cores: usize, work: impl FnOnce() -> T) -> T {
    let before = PRETEND.replace(cores.max(1));
    let out = work();
    PRETEND.set(before);
    out
}

/// Runs `work` over `0..count` split into parts on several threads when
/// there is enough to do, and gives each part's result in order. Small
/// jobs run on the calling thread, which also takes the first part of a
/// big one.
pub fn in_parallel<T: Send>(
    count: usize,
    work: impl Fn(std::ops::Range<usize>) -> T + Sync,
) -> Vec<T> {
    in_parallel_from(count, PER_THREAD, work)
}

/// [`in_parallel`] for items that each take longer (blocks of a grid,
/// say): split once there are at least `per_thread` items for each
/// thread.
pub fn in_parallel_from<T: Send>(
    count: usize,
    per_thread: usize,
    work: impl Fn(std::ops::Range<usize>) -> T + Sync,
) -> Vec<T> {
    let threads = cores().min(count / per_thread.max(1)).max(1);
    if threads == 1 {
        return vec![work(0..count)];
    }
    let part = count.div_ceil(threads);
    std::thread::scope(|scope| {
        // The first part is worked out here while the others run on
        // threads of their own: one thread fewer to start each time, which
        // adds up for work done many times a frame (a sculpting dab).
        let handles: Vec<_> = (1..threads)
            .map(|t| {
                let range = (t * part).min(count)..((t + 1) * part).min(count);
                let work = &work;
                scope.spawn(move || work(range))
            })
            .collect();
        let mut out = Vec::with_capacity(threads);
        out.push(work(0..part.min(count)));
        out.extend(
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))),
        );
        out
    })
}

/// The sum of `of` over `0..count`, worked out in blocks of `block` on
/// several threads: each block added up on its own and the blocks added
/// in order, so the same on any number of cores.
pub fn sum_in_blocks(count: usize, block: usize, of: impl Fn(usize) -> f64 + Sync) -> f64 {
    let block = block.max(1);
    let blocks = count.div_ceil(block);
    in_parallel_from(blocks, 2, |range| {
        range
            .map(|b| {
                (b * block..((b + 1) * block).min(count))
                    .map(&of)
                    .sum::<f64>()
            })
            .collect::<Vec<f64>>()
    })
    .into_iter()
    .flatten()
    .sum()
}

/// `items` sorted, on several threads when there are many: each thread
/// sorts a part, then the parts are merged ([decision 0313](../../../docs/decisions/0313-sculpting-speed.md)).
pub fn sort<T: Ord + Copy + Send>(items: &mut Vec<T>) {
    let threads = cores().min(items.len() / PER_THREAD).max(1);
    if threads == 1 {
        items.sort_unstable();
        return;
    }
    let part = items.len().div_ceil(threads);
    std::thread::scope(|scope| {
        for chunk in items.chunks_mut(part) {
            scope.spawn(move || chunk.sort_unstable());
        }
    });
    // Merged a pair of sorted runs at a time until one is left.
    let parts: Vec<&[T]> = items.chunks(part).collect();
    let mut runs: Vec<Vec<T>> = parts
        .chunks(2)
        .map(|pair| match pair {
            [a, b] => merge(a, b),
            _ => pair.concat(),
        })
        .collect();
    while runs.len() > 1 {
        let mut next = Vec::with_capacity(runs.len().div_ceil(2));
        let mut pairs = runs.into_iter();
        while let Some(a) = pairs.next() {
            match pairs.next() {
                Some(b) => next.push(merge(&a, &b)),
                None => next.push(a),
            }
        }
        runs = next;
    }
    if let Some(sorted) = runs.pop() {
        *items = sorted;
    }
}

/// Two sorted lists merged into one.
fn merge<T: Ord + Copy>(a: &[T], b: &[T]) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if b[j] < a[i] {
            out.push(b[j]);
            j += 1;
        } else {
            out.push(a[i]);
            i += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_is_covered_once_in_order() {
        for count in [0, 1, 99_999, 450_001] {
            let parts = in_parallel(count, |r| r.collect::<Vec<_>>());
            let all: Vec<usize> = parts.into_iter().flatten().collect();
            assert_eq!(all, (0..count).collect::<Vec<_>>());
        }
    }

    #[test]
    fn work_is_shared_as_on_any_number_of_cores_when_asked() {
        let count = PER_THREAD * 6;
        for pretend in [1, 2, 3, 6, 16] {
            let parts = with_cores(pretend, || {
                assert_eq!(cores(), pretend);
                in_parallel(count, |r| r.collect::<Vec<_>>())
            });
            assert_eq!(parts.len(), pretend.min(6));
            let all: Vec<usize> = parts.into_iter().flatten().collect();
            assert_eq!(all, (0..count).collect::<Vec<_>>());
        }
        // Back to the computer's own afterwards, and never none.
        assert!(cores() >= 1);
        assert_eq!(with_cores(0, cores), 1);
    }

    #[test]
    fn sorting_on_several_threads_sorts() {
        for count in [0, 1, 7, PER_THREAD * 2 + 3, PER_THREAD * 5] {
            let mut items: Vec<u64> = (0..count as u64)
                .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15) % 1000)
                .collect();
            let mut expected = items.clone();
            expected.sort_unstable();
            sort(&mut items);
            assert_eq!(items, expected);
        }
    }

    #[test]
    fn sums_in_blocks_add_the_blocks_in_order() {
        let of = |i: usize| (i as f64).sqrt() * 0.1;
        let expected: f64 = (0..10)
            .map(|b| (b * 1000..((b + 1) * 1000).min(9_500)).map(of).sum::<f64>())
            .sum();
        assert_eq!(sum_in_blocks(9_500, 1000, of), expected);
        assert_eq!(sum_in_blocks(0, 1000, of), 0.0);
    }
}
