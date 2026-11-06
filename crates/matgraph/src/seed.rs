//! Random numbers that are the same every time: a pattern's randomness
//! comes from hashing where it is and its seed, never from a clock, so a
//! graph with the same seed always makes the same textures (on any
//! machine, with any number of cores).

/// Mixes two numbers into one that looks random (SplitMix64's finaliser).
pub fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_add(0x9E37_79B9_7F4A_7C15).rotate_left(17);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A box's seed: its own seed setting mixed with the graph's seed.
pub fn box_seed(graph_seed: u32, own: i32) -> u64 {
    mix(u64::from(graph_seed), u64::from(own as u32))
}

/// A number from 0 to 1 (not including 1) from a hash.
pub fn unit(hash: u64) -> f32 {
    (hash >> 40) as f32 / (1u64 << 24) as f32
}

/// A hash of a lattice point `(x, y)` on layer `layer` with `seed`.
pub fn lattice(seed: u64, layer: u32, x: i64, y: i64) -> u64 {
    mix(
        mix(seed, u64::from(layer)),
        mix(x as u64, (y as u64).rotate_left(32)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixing_is_repeatable_and_spreads() {
        assert_eq!(mix(1, 2), mix(1, 2));
        assert_ne!(mix(1, 2), mix(2, 1));
        assert_ne!(box_seed(0, 1), box_seed(1, 1));
        // Units cover the range evenly enough.
        let mean: f32 = (0..10_000).map(|i| unit(mix(i, 9))).sum::<f32>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
        assert!((0..1000).all(|i| (0.0..1.0).contains(&unit(mix(i, 3)))));
    }
}
