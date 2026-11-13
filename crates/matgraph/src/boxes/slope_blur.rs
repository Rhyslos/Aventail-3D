//! Slope blur: smears a height, a colour or a whole material along the
//! slopes of another height, as rain wears stone, paint runs and sand
//! drifts (Phase 12, milestone 10; decision 0272).
//!
//! Each pixel looks along the slope of By (uphill, so what is higher
//! slides down over it, or downhill) for Distance, taking Steps samples
//! on the way, and averages them (Smear), keeps the lowest (Wear down)
//! or keeps the highest (Build up). Steeper places smear further, flat
//! ones not at all. With nothing in By, the box's own noise gives the
//! slopes, as Warp's does. It takes the same three inputs as Transform
//! and Repeat ([`super::moving`]): plug in any of them.

use super::moving::{self, INPUTS};
use super::noise::layered;
use super::warp::{slope, steepest};
use super::{choice, info, length, seed, uv, whole};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::{text, trf};

/// The Slope blur box.
#[derive(Debug)]
pub(crate) struct SlopeBlur {
    info: BoxInfo,
}

/// The ways greys are put together, in the Mode setting's order.
const SMEAR: u32 = 0;
const WEAR: u32 = 1;
const BUILD: u32 = 2;

/// The way samples are taken, in the Direction setting's order.
const DOWNHILL: u32 = 0;

impl SlopeBlur {
    pub(crate) fn new() -> Self {
        let (mut inputs, outputs) = moving::plugs();
        inputs.push(PlugSpec::new("by", text!("By"), ValueKind::Height, false));
        Self {
            info: info(
                "slope_blur",
                text!("Slope blur"),
                Group::Shape,
                text!(
                    "Smears a height, a colour or a whole material along the slopes of another \
                     height (By), as rain wears stone, paint runs and sand drifts. Plug in any of \
                     the three and take each from its output. Distance is how far things smear, \
                     in centimetres on the finished material; steeper places smear further, flat \
                     ones not at all. Mode: Smear blurs along the slope, Wear down eats into \
                     what is high, Build up spreads it. With nothing in By, the box's own noise \
                     gives the slopes (Scale sets how big its hills are)."
                ),
                (inputs, outputs),
                vec![
                    length(
                        "distance",
                        text!("Distance"),
                        text!("How far things smear, in centimetres."),
                        0.0,
                        50.0,
                        2.0,
                    ),
                    choice(
                        "mode",
                        text!("Mode"),
                        text!(
                            "Smear blurs along the slope; Wear down keeps the lowest, eating \
                             into what is high; Build up keeps the highest, spreading it."
                        ),
                        &[text!("Smear"), text!("Wear down"), text!("Build up")],
                        SMEAR,
                    ),
                    choice(
                        "direction",
                        text!("Direction"),
                        text!(
                            "Downhill: what is higher slides down over lower places. Uphill: \
                             the other way."
                        ),
                        &[text!("Downhill"), text!("Uphill")],
                        DOWNHILL,
                    ),
                    whole(
                        "steps",
                        text!("Steps"),
                        text!("How many samples along the way: more is smoother, and slower."),
                        2,
                        32,
                        12,
                    ),
                    whole(
                        "scale",
                        text!("Scale"),
                        text!("How many hills fit across, when nothing is in By."),
                        1,
                        32,
                        4,
                    ),
                    seed(),
                ],
                &["levels", "material_from_height", "blend"],
            ),
        }
    }
}

impl BoxType for SlopeBlur {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(moving::filled(self.run_outputs(job)?, job.size()))
    }

    fn needs_an_input(&self) -> bool {
        true
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        if INPUTS.iter().all(|k| job.input(k).is_none()) {
            return Err(Refusal::new(trf!(
                "{} has nothing plugged in.",
                job.box_name()
            )));
        }
        let n = job.size().get() as f32;
        let own;
        let by: &Grey = match job.grey("by") {
            Some(by) => by,
            None => {
                let scale = job.whole("scale").max(1) as u32;
                let seed = job.seed();
                own = job.fill_grey(|x, y| {
                    let (u, v) = uv(x, y, n);
                    layered(u, v, scale, 3, 0.5, seed)
                });
                &own
            }
        };
        let distance = job.length_px("distance").max(0.0);
        let steps = job.whole("steps").clamp(2, 32) as usize;
        let mode = job.choice("mode");
        let sign = if job.choice("direction") == DOWNHILL {
            1.0
        } else {
            -1.0
        };
        // Each pixel's way along the slope, in textures, steepest one
        // `distance` long.
        let steepest = steepest(by);
        let way = |x: u32, y: u32| -> (f32, f32) {
            if steepest <= 0.0 || distance <= 0.0 {
                return (0.0, 0.0);
            }
            let (dx, dy) = slope(by, x, y);
            let k = sign * distance / steepest / n;
            (dx * k, dy * k)
        };
        let places = move |x: u32, y: u32| {
            let (u, v) = uv(x, y, n);
            let (wx, wy) = way(x, y);
            let last = (steps - 1) as f32;
            (0..steps).map(move |i| {
                let t = i as f32 / last;
                (u + wx * t, v + wy * t)
            })
        };
        let grey = |g: &Grey, by_mode: bool| -> Arc<Grey> {
            Arc::new(job.fill_grey(|x, y| {
                let samples = places(x, y).map(|(u, v)| g.sample(u, v));
                let v = match (by_mode, mode) {
                    (true, WEAR) => samples.fold(f32::INFINITY, f32::min),
                    (true, BUILD) => samples.fold(f32::NEG_INFINITY, f32::max),
                    _ => samples.sum::<f32>() / steps as f32,
                };
                v.clamp(0.0, 1.0)
            }))
        };
        let rgb = |c: &Rgb, unit: bool| -> Arc<Rgb> {
            Arc::new(job.fill_rgb(|x, y| {
                let mut total = [0.0f32; 3];
                for (u, v) in places(x, y) {
                    let s = c.sample_rgb(u, v);
                    for i in 0..3 {
                        total[i] += s[i];
                    }
                }
                let mean = total.map(|t| t / steps as f32);
                if unit {
                    glam::Vec3::from_array(mean)
                        .normalize_or(glam::Vec3::Z)
                        .to_array()
                } else {
                    mean.map(|v| v.clamp(0.0, 1.0))
                }
            }))
        };
        let height = job.grey(INPUTS[0]).map(|g| Value::Height(grey(g, true)));
        let colour = job
            .input(INPUTS[1])
            .and_then(Value::colour)
            .map(|c| Value::Colour(rgb(c, false)));
        let material = job.input(INPUTS[2]).and_then(Value::material).map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: rgb(&m.colour, false),
                height: grey(&m.height, true),
                roughness: grey(&m.roughness, false),
                metal: grey(&m.metal, false),
                normal: rgb(&m.normal, true),
                occlusion: grey(&m.occlusion, false),
                curvature: grey(&m.curvature, false),
            }))
        });
        Ok(vec![height, colour, material])
    }
}
