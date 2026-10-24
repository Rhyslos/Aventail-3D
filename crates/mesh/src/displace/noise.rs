//! The noise Displace pushes points by ([decision 0157](../../../../docs/decisions/0157-displace.md)),
//! written by hand so it is the same on every computer and needs no
//! crate.
//!
//! Every kind is built on a grid of whole-number cells. Each cell corner
//! (or cell) gets its random numbers from a hash of its place and the
//! seed ([`crate::names::mix`]), never from a random generator's state,
//! so a point's value depends only on where it is, the seed and the
//! settings: the same shape always gets the same bumps, and moving one
//! point never changes another's.
//!
//! - **Smooth** ([`gradient`]): gradient noise (Perlin's improved noise),
//!   a soft rolling surface, about −1 to 1.
//! - **Ridged**: one minus the smooth noise's size, squared, so its
//!   crossings of 0 become sharp crests; 0 to 1.
//! - **Cells** ([`cells`]): one random point a cell, and how much nearer a
//!   place is to its nearest point than to the next (Worley's F2 − F1),
//!   eased: rounded cells with sharp grooves between them; 0 to 1.
//!
//! **Detail** adds layers (octaves): each layer is half the size and half
//! the height of the one before, with a seed of its own, and the sum is
//! divided by the heights' sum so the result keeps its range.

use super::NoiseKind;
use crate::names::mix;
use glam::Vec3;

/// The hash of cell `(x, y, z)` for `seed`.
fn hash(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    // Each coordinate as its 32 bits, so negative cells hash as well as
    // positive ones.
    let a = mix(seed, u64::from(x as u32));
    let b = mix(a, u64::from(y as u32));
    mix(b, u64::from(z as u32))
}

/// A number from 0 to 1 (1 left out) from 24 bits of `h`, starting at
/// bit `shift`.
fn unit(h: u64, shift: u32) -> f32 {
    ((h >> shift) & 0xff_ffff) as f32 / 16_777_216.0
}

/// The whole cell `x` is in, and how far into it, saturating far out.
fn split(x: f32) -> (i32, f32) {
    let floor = x.floor();
    (floor as i32, x - floor)
}

/// Perlin's ease: 0 at 0, 1 at 1, flat at both ends (to its second
/// slope), so the cells don't show.
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// The dot of a corner's gradient (one of the cube's 12 edge directions,
/// picked by `h`) with the offset `d` from that corner.
fn grad(h: u64, d: Vec3) -> f32 {
    match (h >> 7) % 12 {
        0 => d.x + d.y,
        1 => -d.x + d.y,
        2 => d.x - d.y,
        3 => -d.x - d.y,
        4 => d.x + d.z,
        5 => -d.x + d.z,
        6 => d.x - d.z,
        7 => -d.x - d.z,
        8 => d.y + d.z,
        9 => -d.y + d.z,
        10 => d.y - d.z,
        _ => -d.y - d.z,
    }
}

/// Gradient noise at `p` for `seed`: smooth, 0 at every whole-number
/// corner, about −1 to 1 (clamped to it).
pub(crate) fn gradient(seed: u64, p: Vec3) -> f32 {
    let (ix, fx) = split(p.x);
    let (iy, fy) = split(p.y);
    let (iz, fz) = split(p.z);
    let corner = |dx: i32, dy: i32, dz: i32| {
        let h = hash(
            seed,
            ix.wrapping_add(dx),
            iy.wrapping_add(dy),
            iz.wrapping_add(dz),
        );
        let d = Vec3::new(fx - dx as f32, fy - dy as f32, fz - dz as f32);
        grad(h, d)
    };
    let (u, v, w) = (fade(fx), fade(fy), fade(fz));
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), u);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), u);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), u);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), u);
    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);
    lerp(y0, y1, w).clamp(-1.0, 1.0)
}

/// How far apart a cell's random point may wander from its middle: a
/// little less than the whole cell, so no point sits right on a side.
const JITTER: f32 = 0.9;

/// Cell noise at `p` for `seed`: rounded cells (1 in their middles) with
/// sharp grooves (0) where two cells meet. From the gap between the
/// distances to the nearest and the next nearest random point.
pub(crate) fn cells(seed: u64, p: Vec3) -> f32 {
    let (ix, fx) = split(p.x);
    let (iy, fy) = split(p.y);
    let (iz, fz) = split(p.z);
    let inside = Vec3::new(fx, fy, fz);
    let (mut first, mut second) = (f32::MAX, f32::MAX);
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let h = hash(
                    seed,
                    ix.wrapping_add(dx),
                    iy.wrapping_add(dy),
                    iz.wrapping_add(dz),
                );
                let jitter = Vec3::new(unit(h, 0), unit(h, 20), unit(h, 40)) - 0.5;
                let point = Vec3::new(dx as f32, dy as f32, dz as f32) + 0.5 + jitter * JITTER;
                let d = point.distance(inside);
                if d < first {
                    second = first;
                    first = d;
                } else if d < second {
                    second = d;
                }
            }
        }
    }
    // The gap is 0 on a border and about 0.5 to 1 in a cell's middle;
    // eased so the cells are domed rather than pointed.
    let t = ((second - first) / 0.6).clamp(0.0, 1.0);
    t * (2.0 - t)
}

/// The value of `kind` at `p` (already divided by the scale) for `seed`,
/// summed over `detail` layers (at least one).
pub(crate) fn layered(kind: NoiseKind, seed: u32, detail: u32, p: Vec3) -> f32 {
    let mut sum = 0.0;
    let mut heights = 0.0;
    let mut height = 1.0;
    let mut place = p;
    for layer in 0..detail.max(1) {
        let seed = mix(u64::from(seed), u64::from(layer) + 1);
        let value = match kind {
            NoiseKind::Smooth => gradient(seed, place),
            NoiseKind::Ridged => {
                let crest = 1.0 - gradient(seed, place).abs();
                crest * crest
            }
            NoiseKind::Cells => cells(seed, place),
        };
        sum += value * height;
        heights += height;
        height *= 0.5;
        // Shifted as well as halved, so the layers' grids never line up.
        place = place * 2.0 + Vec3::new(17.31, -5.77, 9.13);
    }
    if heights > 0.0 { sum / heights } else { 0.0 }
}
