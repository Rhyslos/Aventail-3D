//! Sharing heavy work between the computer's cores, for meshes of
//! millions of polygons ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).

/// Below this many items per thread, splitting the work isn't worth it.
const PER_THREAD: usize = 100_000;

/// Runs `work` over `0..count` split into parts on several threads when
/// there is enough to do, and gives each part's result in order. Small
/// jobs run on the calling thread.
pub fn in_parallel<T: Send>(
    count: usize,
    work: impl Fn(std::ops::Range<usize>) -> T + Sync,
) -> Vec<T> {
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(count / PER_THREAD)
        .max(1);
    if threads == 1 {
        return vec![work(0..count)];
    }
    let part = count.div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let range = (t * part).min(count)..((t + 1) * part).min(count);
                let work = &work;
                scope.spawn(move || work(range))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    })
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
}
