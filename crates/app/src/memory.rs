//! Watching the computer's memory, so tabs left for another sleep at once
//! when it runs low ([decision 0267](../../../docs/decisions/0267-sleeping-tabs.md)).
//!
//! The free memory is asked of the system at most every few seconds; it
//! counts as low when less than a tenth of it, or less than 768 MB, is
//! free for programs to use.

use std::time::{Duration, Instant};

/// How often the free memory is looked at, at most.
pub const LOOK_EVERY: Duration = Duration::from_secs(10);

/// Less free than this is low, however big the memory.
const LOW_BYTES: u64 = 768 * 1024 * 1024;

/// Whether `free` bytes of `total` is low: under a tenth, or under
/// 768 MB. Nonsense (nothing in total, more free than there is) is never
/// low, so tabs aren't put to sleep on a misreading.
pub fn is_low(free: u64, total: u64) -> bool {
    if total == 0 || free > total {
        return false;
    }
    free < total / 10 || free < LOW_BYTES
}

/// Free and total memory in bytes from Linux's `/proc/meminfo`:
/// `MemAvailable` (what programs can still have) and `MemTotal`, given in
/// kB. `None` if either is missing or unreadable.
pub fn from_meminfo(text: &str) -> Option<(u64, u64)> {
    let bytes = |rest: &str| -> Option<u64> {
        let kb: u64 = rest.trim().strip_suffix("kB")?.trim().parse().ok()?;
        kb.checked_mul(1024)
    };
    let (mut free, mut total) = (None, None);
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            free = free.or_else(|| bytes(rest));
        } else if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = total.or_else(|| bytes(rest));
        }
    }
    Some((free?, total?))
}

/// Whether memory is low, looked at again at most every [`LOOK_EVERY`].
#[derive(Debug, Default)]
pub struct MemoryWatch {
    looked: Option<Instant>,
    low: bool,
}

impl MemoryWatch {
    /// Whether memory was low when last looked at; looks again first if
    /// that was long enough ago, asking `read` for (free, total).
    pub fn low(&mut self, now: Instant, read: impl FnOnce() -> Option<(u64, u64)>) -> bool {
        let due = self
            .looked
            .is_none_or(|at| now.saturating_duration_since(at) >= LOOK_EVERY);
        if due {
            self.looked = Some(now);
            self.low = read().is_some_and(|(free, total)| is_low(free, total));
        }
        self.low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn low_is_under_a_tenth_or_under_768_megabytes() {
        assert!(!is_low(4 * GB, 16 * GB));
        assert!(is_low(GB, 16 * GB), "under a tenth");
        assert!(is_low(512 * 1024 * 1024, 4 * GB), "under 768 MB");
        assert!(!is_low(2 * GB, 8 * GB));
        // Misreadings never count as low.
        assert!(!is_low(0, 0));
        assert!(!is_low(9 * GB, 8 * GB));
    }

    #[test]
    fn linux_memory_is_read_from_meminfo() {
        let text = "MemTotal:       16303680 kB\nMemFree:          867548 kB\n\
                    MemAvailable:   10489184 kB\nBuffers:          123 kB\n";
        assert_eq!(
            from_meminfo(text),
            Some((10_489_184 * 1024, 16_303_680 * 1024))
        );
        // Old kernels without MemAvailable, or garbled numbers: unknown.
        assert_eq!(from_meminfo("MemTotal: 100 kB\nMemFree: 50 kB\n"), None);
        assert_eq!(from_meminfo("MemTotal: x kB\nMemAvailable: 5 kB"), None);
        assert_eq!(from_meminfo(""), None);
    }

    #[test]
    fn random_meminfo_text_never_panics() {
        // A fuzz test: whatever the file holds, reading it gives numbers
        // or nothing, never a crash or an overflow.
        let pieces = [
            "MemTotal",
            "MemAvailable",
            ":",
            " ",
            "kB",
            "\n",
            "0",
            "18446744073709551615",
            "99",
            "-1",
            "MB",
            "\t",
            "Mem",
            "é",
        ];
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..5000 {
            let mut text = String::new();
            for _ in 0..(next() % 20) {
                text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
            }
            if let Some((free, total)) = from_meminfo(&text) {
                let _ = is_low(free, total);
            }
        }
    }

    #[test]
    fn the_watch_looks_again_only_after_a_while() {
        let mut watch = MemoryWatch::default();
        let start = Instant::now();
        assert!(watch.low(start, || Some((GB / 2, 8 * GB))));
        // Soon after, the last answer stands without asking.
        let mut asked = false;
        assert!(watch.low(start + Duration::from_secs(1), || {
            asked = true;
            None
        }));
        assert!(!asked, "the last answer stands");
        assert!(!watch.low(start + LOOK_EVERY, || Some((6 * GB, 8 * GB))));
        // The system can't say: not low.
        assert!(!watch.low(start + LOOK_EVERY * 3, || None));
    }

    #[test]
    fn this_computer_says_how_much_memory_it_has() {
        if let Some((free, total)) = crate::platform::memory() {
            assert!(total > 0 && free <= total);
        }
    }
}
