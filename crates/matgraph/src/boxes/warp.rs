//! Warp: pushes a height's pixels about, so straight lines wobble and
//! neat tiles look hand-made or worn.
//!
//! With a height in By, each place is pushed the way By slopes upwards;
//! without one, by two layers of the app's own noise (one across, one
//! down). It reads its input wrapping round, so the texture still tiles.

use super::noise::layered;
use super::{info, length, seed, uv, whole};
use crate::image::Grey;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The Warp box.
#[derive(Debug)]
pub(crate) struct Warp {
    info: BoxInfo,
}

impl Warp {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "warp",
                text!("Warp"),
                Group::Shape,
                text!(
                    "Pushes a height about so straight lines wobble: neat tiles become hand-cut \
                     stone, stripes become wood grain. Amount is how far things move, in \
                     centimetres on the finished material. With a height in By, things move the \
                     way it slopes upwards; with nothing there, the box's own noise moves them \
                     (Scale sets how big its wobbles are)."
                ),
                (
                    vec![
                        PlugSpec::new("in", text!("In"), ValueKind::Height, true),
                        PlugSpec::new("by", text!("By"), ValueKind::Height, false),
                    ],
                    vec![PlugSpec::new("out", text!("Out"), ValueKind::Height, false)],
                ),
                vec![
                    length(
                        "amount",
                        text!("Amount"),
                        text!("How far things move, in centimetres."),
                        0.0,
                        50.0,
                        2.0,
                    ),
                    whole(
                        "scale",
                        text!("Scale"),
                        text!("How many wobbles fit across, when nothing is in By."),
                        1,
                        32,
                        4,
                    ),
                    seed(),
                ],
                &["bevel", "levels", "material_from_height"],
            ),
        }
    }
}

impl BoxType for Warp {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let input = job.needed_grey("in")?;
        let amount = job.length_px("amount");
        let n = job.size().get() as f32;
        let push: Box<dyn Fn(u32, u32) -> (f32, f32) + Sync> = match job.grey("by") {
            Some(by) => {
                let steepest = steepest(by);
                let by = by.clone();
                Box::new(move |x, y| {
                    let (dx, dy) = slope(&by, x, y);
                    if steepest <= 0.0 {
                        (0.0, 0.0)
                    } else {
                        (dx / steepest * amount, dy / steepest * amount)
                    }
                })
            }
            None => {
                let scale = job.whole("scale").max(1) as u32;
                let seed = job.seed();
                Box::new(move |x, y| {
                    let (u, v) = uv(x, y, n);
                    let a = layered(u, v, scale, 3, 0.5, seed) * 2.0 - 1.0;
                    let b = layered(u, v, scale, 3, 0.5, seed ^ 0x5741_5250) * 2.0 - 1.0;
                    (a * amount, b * amount)
                })
            }
        };
        let out = job.fill_grey(|x, y| {
            let (dx, dy) = push(x, y);
            let (u, v) = ((x as f32 + 0.5 + dx) / n, (y as f32 + 0.5 + dy) / n);
            input.sample(u, v)
        });
        Ok(vec![Value::Height(Arc::new(out))])
    }
}

/// How `image` slopes at pixel `(x, y)`: up across and up down, per pixel.
fn slope(image: &Grey, x: u32, y: u32) -> (f32, f32) {
    let (x, y) = (i64::from(x), i64::from(y));
    (
        (image.at(x + 1, y) - image.at(x - 1, y)) * 0.5,
        (image.at(x, y + 1) - image.at(x, y - 1)) * 0.5,
    )
}

/// The steepest slope anywhere in `image`.
fn steepest(image: &Grey) -> f32 {
    let n = image.size();
    let mut most: f32 = 0.0;
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = slope(image, x, y);
            most = most.max((dx * dx + dy * dy).sqrt());
        }
    }
    most
}
