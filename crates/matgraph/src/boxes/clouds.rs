//! Clouds: big, soft, billowing greys, for weathering, dirt and colour
//! variation, with a choice of looks (soft, billowy, ridged).
//!
//! Value noise (a random grey at each point of a lattice that wraps
//! round, blended smoothly) layered finer and finer. Billowy folds it
//! about the middle grey; Ridged turns the folds into sharp ridges. The
//! maths is the well-known value noise and its folds; nothing is taken
//! from other programs.

use super::{choice, info, number, seed, uv, whole};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::seed::{lattice, unit};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Clouds box.
#[derive(Debug)]
pub(crate) struct Clouds {
    info: BoxInfo,
}

impl Clouds {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "clouds",
                text!("Clouds"),
                Group::Make,
                text!(
                    "Makes big, soft, billowing greys: good for weathering, dirt, and patches of \
                     lighter and darker colour. Billowy puffs it up like cotton wool; Ridged turns \
                     it into sharp ridges like crumpled paper or veins. It repeats seamlessly."
                ),
                (
                    Vec::new(),
                    vec![PlugSpec::new(
                        "height",
                        text!("Height"),
                        ValueKind::Height,
                        false,
                    )],
                ),
                vec![
                    whole(
                        "scale",
                        text!("Scale"),
                        text!("How many clouds fit across the texture."),
                        1,
                        32,
                        2,
                    ),
                    whole(
                        "detail",
                        text!("Detail"),
                        text!("How many finer layers are added on top."),
                        1,
                        10,
                        6,
                    ),
                    number(
                        "roughness",
                        text!("Roughness"),
                        text!("How strong each finer layer is: low is smooth, high is gritty."),
                        0.0,
                        1.0,
                        0.55,
                    ),
                    choice(
                        "look",
                        text!("Look"),
                        text!("Soft clouds, billowy puffs, or sharp ridges."),
                        &[text!("Soft"), text!("Billowy"), text!("Ridged")],
                        0,
                    ),
                    seed(),
                ],
                &["levels", "blend", "gradient_map"],
            ),
        }
    }
}

impl BoxType for Clouds {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let scale = job.whole("scale").max(1) as u32;
        let layers = job.whole("detail").clamp(1, 10) as u32;
        let roughness = job.number("roughness");
        let look = job.choice("look");
        let seed = job.seed();
        let n = job.size().get() as f32;
        let image = job.fill_grey(|x, y| {
            let (u, v) = uv(x, y, n);
            let mut total = 0.0;
            let mut weight = 1.0;
            let mut weights = 0.0;
            for layer in 0..layers {
                let period = scale.saturating_mul(1 << layer.min(16)).min(1 << 20);
                let mut value = value_noise(u, v, period, seed, layer);
                match look {
                    1 => value = (2.0 * value - 1.0).abs(),
                    2 => value = (1.0 - (2.0 * value - 1.0).abs()).powi(2),
                    _ => {}
                }
                total += value * weight;
                weights += weight;
                weight *= roughness;
            }
            let v = if weights > 0.0 { total / weights } else { 0.5 };
            // Layers average towards the middle grey: spread them out again.
            let spread = match look {
                0 => (v - 0.5) * 1.8 + 0.5,
                _ => (v - 0.3) * 1.6 + 0.3,
            };
            spread.clamp(0.0, 1.0)
        });
        Ok(vec![Value::Height(Arc::new(image))])
    }
}

/// Value noise at `(u, v)` on a lattice of `period` squares across that
/// wraps round, 0 to 1.
fn value_noise(u: f32, v: f32, period: u32, seed: u64, layer: u32) -> f32 {
    let p = i64::from(period.max(1));
    let (x, y) = (f64::from(u) * p as f64, f64::from(v) * p as f64);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let (ix, iy) = (x0 as i64, y0 as i64);
    let corner = |dx: i64, dy: i64| {
        unit(lattice(
            seed,
            layer + 100,
            (ix + dx).rem_euclid(p),
            (iy + dy).rem_euclid(p),
        ))
    };
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (ease(fx), ease(fy));
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * sx;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * sx;
    top + (bottom - top) * sy
}
