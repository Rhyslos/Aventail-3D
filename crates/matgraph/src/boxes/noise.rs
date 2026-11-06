//! Noise: soft, cloudy greys, the start of most natural surfaces.
//!
//! Gradient noise on a lattice that repeats a whole number of times across
//! the texture (its lattice points are wrapped), layered finer and finer
//! (each layer twice as fine, and Roughness times as strong). The maths
//! is the well-known gradient noise with the quintic ease; nothing is
//! taken from other programs.

use super::{next, number, seed, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::seed::{lattice, unit};
use crate::value::Value;
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Noise box.
#[derive(Debug)]
pub(crate) struct Noise {
    info: BoxInfo,
}

impl Noise {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed("noise"),
                name: Label::Key(text!("Noise")),
                group: Group::Make,
                about: Label::Key(text!(
                    "Makes soft, cloudy greys: the start of stone, ground, rust and most natural \
                     surfaces. Scale sets how many blobs fit across; Detail adds finer and finer \
                     layers on top. It repeats seamlessly."
                )),
                inputs: Vec::new(),
                outputs: vec![PlugSpec::new(
                    "height",
                    text!("Height"),
                    ValueKind::Height,
                    false,
                )],
                settings: vec![
                    whole(
                        "scale",
                        text!("Scale"),
                        text!("How many blobs fit across the texture."),
                        1,
                        64,
                        4,
                    ),
                    whole(
                        "detail",
                        text!("Detail"),
                        text!("How many finer layers are added on top."),
                        1,
                        8,
                        4,
                    ),
                    number(
                        "roughness",
                        text!("Roughness"),
                        text!("How strong each finer layer is: low is smooth, high is gritty."),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    seed(),
                ],
                comes_next: next(&[
                    "levels",
                    "warp",
                    "blend",
                    "gradient_map",
                    "material_from_height",
                ]),
                one_per_graph: false,
            },
        }
    }
}

impl BoxType for Noise {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let scale = job.whole("scale").max(1) as u32;
        let layers = job.whole("detail").clamp(1, 8) as u32;
        let roughness = job.number("roughness");
        let seed = job.seed();
        let n = job.size().get() as f32;
        let image = job.fill_grey(|x, y| {
            let (u, v) = ((x as f32 + 0.5) / n, (y as f32 + 0.5) / n);
            layered(u, v, scale, layers, roughness, seed)
        });
        Ok(vec![Value::Height(Arc::new(image))])
    }
}

/// Layered noise at `(u, v)` (a texture each way), 0 to 1: `layers`
/// layers from `scale` lattice squares across, each twice as fine and
/// `roughness` times as strong as the last. Repeats exactly at whole `u`
/// and `v`.
pub fn layered(u: f32, v: f32, scale: u32, layers: u32, roughness: f32, seed: u64) -> f32 {
    let mut total = 0.0;
    let mut weight = 1.0;
    let mut weights = 0.0;
    for layer in 0..layers {
        let period = scale.saturating_mul(1 << layer.min(16)).min(1 << 20);
        total += gradient(u, v, period, seed, layer) * weight;
        weights += weight;
        weight *= roughness;
    }
    if weights <= 0.0 {
        return 0.5;
    }
    (0.5 + 0.5 * total / weights * 1.4).clamp(0.0, 1.0)
}

/// Gradient noise at `(u, v)` on a lattice of `period` squares across that
/// wraps round, about -0.7 to 0.7.
pub fn gradient(u: f32, v: f32, period: u32, seed: u64, layer: u32) -> f32 {
    let p = i64::from(period.max(1));
    let (x, y) = (f64::from(u) * p as f64, f64::from(v) * p as f64);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let (ix, iy) = (x0 as i64, y0 as i64);
    let corner = |dx: i64, dy: i64| {
        let h = lattice(
            seed,
            layer,
            (ix + dx).rem_euclid(p),
            (iy + dy).rem_euclid(p),
        );
        let angle = unit(h) * std::f32::consts::TAU;
        let (ox, oy) = (fx - dx as f32, fy - dy as f32);
        angle.cos() * ox + angle.sin() * oy
    };
    let (sx, sy) = (ease(fx), ease(fy));
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * sx;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * sx;
    top + (bottom - top) * sy
}

/// The quintic ease: smooth up to the second change.
fn ease(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
