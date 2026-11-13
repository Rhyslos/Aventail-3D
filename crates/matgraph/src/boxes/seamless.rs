//! Make seamless (Phase 12 milestone 21, [decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! makes a picture that doesn't tile (a photo) repeat without a visible
//! join.
//!
//! **How:** a copy of the picture moved half a texture across has its
//! joins in the middle and none at its edges (its edges are the
//! picture's middle, which runs on smoothly). Near the left and right
//! edges the box shows that copy; near the middle, the picture itself;
//! between, a blend of the two over **Blend width**. Then the same again
//! top to bottom, on the result. Both are done with a blend that keeps
//! the contrast (a plain half-and-half mix of two different places would
//! look washed out and blurry): each value's difference from the average
//! is divided by how much the mix shrinks it. The line where the two
//! meet wanders by noise (**Wobble**, **Seed**), so it doesn't run
//! straight down the picture.
//!
//! This is the offset-and-blend way artists make photos tile by hand,
//! done for them: robust (it never fails or leaves a gap), and the same
//! on any picture. Its weak point is a strong pattern (bricks, planks):
//! where the blend crosses them, two rows can show at once. A narrower
//! Blend width helps; so does choosing the square part so the pattern's
//! rows meet up.
//!
//! It takes a height, a colour or a whole material (each plugged in made
//! seamless the same way, a material's normals made a unit long again).

use super::moving::{INPUTS, filled, plugs};
use super::noise::layered;
use super::{info, number, seed};
use crate::image::{Grey, Rgb, Size};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::{text, trf};

/// The box's id.
pub const SEAMLESS: &str = "seamless";

/// The Make seamless box.
#[derive(Debug)]
pub(crate) struct Seamless {
    info: BoxInfo,
}

impl Seamless {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                SEAMLESS,
                text!("Make seamless"),
                Group::Shape,
                text!(
                    "Makes a picture that doesn't repeat cleanly (a photo) tile without a \
                     visible join. Near its edges it shows a copy moved half a texture along, \
                     whose joins are in the middle, and blends the two so the joins vanish, \
                     keeping the contrast. Switch on the preview's seam check to see the joins \
                     before and after."
                ),
                plugs(),
                vec![
                    number(
                        "width",
                        text!("Blend width"),
                        text!(
                            "How far in from each edge the two copies are blended, as a share \
                             of the texture: wider hides a strong change better, narrower keeps \
                             more of the picture as it was."
                        ),
                        0.05,
                        0.5,
                        0.25,
                    ),
                    number(
                        "wobble",
                        text!("Wobble"),
                        text!(
                            "How much the line where the two copies meet wanders, so it never \
                             runs straight down the picture. 0 is straight."
                        ),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    seed(),
                ],
                &["even_light", "guess_height", "guess_roughness"],
            ),
        }
    }
}

impl BoxType for Seamless {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        Ok(filled(made(job)?, job.size()))
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        made(job)
    }

    fn needs_an_input(&self) -> bool {
        true
    }
}

/// How much of the moved copy shows, each way: across (left and right
/// edges) and down (top and bottom).
struct Masks {
    across: Grey,
    down: Grey,
}

/// What the box gives: each plugged-in input made seamless.
fn made(job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
    let masks = masks(job);
    let out: Vec<Option<Value>> = INPUTS
        .iter()
        .map(|key| job.input(key).map(|v| seamless(job, &masks, v)))
        .collect();
    if out.iter().all(Option::is_none) {
        return Err(Refusal::new(trf!(
            "{} has nothing plugged in.",
            job.box_name()
        )));
    }
    Ok(out)
}

/// The two masks: 1 at the edges (the moved copy), 0 from the blend
/// width in (the picture itself), with a wandering edge.
fn masks(job: &Job<'_>) -> Masks {
    let width = job.number("width").clamp(0.05, 0.5);
    let wobble = job.number("wobble").clamp(0.0, 1.0);
    let seed = job.seed();
    let n = job.size().get() as f32;
    // The wandering is gentle, so it is worked out on a small picture
    // and blended up (it repeats, as everything does).
    let small = Size::nearest(job.size().get().min(128));
    let wander_of = |layer: u64| -> Grey {
        let m = small.get() as f32;
        crate::parallel::fill(small, job.threads(), |x, y| {
            let (u, v) = ((x as f32 + 0.5) / m, (y as f32 + 0.5) / m);
            layered(u, v, 4, 3, 0.5, seed ^ layer) * 2.0 - 1.0
        })
    };
    let one = |layer: u64, flip: bool| {
        let wander = wander_of(layer);
        job.fill_grey(|x, y| {
            let (u, v) = ((x as f32 + 0.5) / n, (y as f32 + 0.5) / n);
            let along = if flip { v } else { u };
            // How far from the nearest edge, 0 to 0.5.
            let edge = along.min(1.0 - along);
            // The wobble fades to nothing at the edge and the middle, so
            // the mask is exactly 1 at the one and 0 at the other.
            let fade = (edge * 2.0 * (1.0 - edge * 2.0) * 4.0).clamp(0.0, 1.0);
            let e = (edge + wander.sample(u, v) * wobble * width * 0.5 * fade).max(0.0);
            1.0 - smoothstep(e / width)
        })
    };
    Masks {
        across: one(0x5ea1, false),
        down: one(0x5ea2, true),
    }
}

/// 0 below 0, 1 above 1, smooth between.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `a` and `b` mixed by `w` (0 all `a`, 1 all `b`) about their shared
/// average `mean`, keeping the contrast: the mix of two unrelated values
/// spreads less than either, so its difference from the average is
/// divided by how much it shrank.
fn kept(a: f32, b: f32, w: f32, mean: f32) -> f32 {
    let spread = ((1.0 - w) * (1.0 - w) + w * w).sqrt().max(1e-3);
    mean + ((a - mean) * (1.0 - w) + (b - mean) * w) / spread
}

/// One grey channel made seamless.
fn grey(job: &Job<'_>, masks: &Masks, g: &Grey) -> Grey {
    let half = i64::from(g.size()) / 2;
    let mean = super::photo_maths::mean(g);
    let first = job.fill_grey(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        kept(g.at(x, y), g.at(x + half, y), masks.across.at(x, y), mean)
    });
    job.fill_grey(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        let v = kept(
            first.at(x, y),
            first.at(x, y + half),
            masks.down.at(x, y),
            mean,
        );
        v.clamp(0.0, 1.0)
    })
}

/// A colour made seamless, channel by channel.
fn colour(job: &Job<'_>, masks: &Masks, c: &Rgb) -> Rgb {
    let size = Size::nearest(c.size());
    let channels: Vec<Grey> = (0..3).map(|k| grey(job, masks, &c.map(|p| p[k]))).collect();
    crate::parallel::fill(size, job.threads(), |x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        [
            channels[0].at(x, y),
            channels[1].at(x, y),
            channels[2].at(x, y),
        ]
    })
}

/// Normals made seamless: blended plainly (a direction has no contrast
/// to keep) and made a unit long.
fn normals(job: &Job<'_>, masks: &Masks, n: &Rgb) -> Rgb {
    let half = i64::from(n.size()) / 2;
    let mix = |a: [f32; 3], b: [f32; 3], w: f32| -> [f32; 3] {
        [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * w)
    };
    let first: Rgb = job.fill_rgb(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        mix(n.at(x, y), n.at(x + half, y), masks.across.at(x, y))
    });
    job.fill_rgb(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        let m = mix(first.at(x, y), first.at(x, y + half), masks.down.at(x, y));
        glam::Vec3::from_array(m)
            .normalize_or(glam::Vec3::Z)
            .to_array()
    })
}

/// `value` made seamless (a shape comes out as a height).
fn seamless(job: &Job<'_>, masks: &Masks, value: &Value) -> Value {
    let g = |image: &Arc<Grey>| Arc::new(grey(job, masks, image));
    match value {
        Value::Shape(h) | Value::Height(h) => Value::Height(g(h)),
        Value::Colour(c) => Value::Colour(Arc::new(colour(job, masks, c))),
        Value::Material(m) => Value::Material(Arc::new(MaterialMaps {
            colour: Arc::new(colour(job, masks, &m.colour)),
            height: g(&m.height),
            roughness: g(&m.roughness),
            metal: g(&m.metal),
            normal: Arc::new(normals(job, masks, &m.normal)),
            occlusion: g(&m.occlusion),
            curvature: g(&m.curvature),
            opacity: m.opacity.as_ref().map(g),
            translucency: m.translucency.as_ref().map(g),
        })),
    }
}

/// A picture that doesn't tile, for tests: a slope across and down with
/// fine noise on it, so its left edge is dark and its right light.
#[cfg(test)]
pub(super) fn not_tiling(n: u32) -> Rgb {
    let size = Size::new(n).unwrap();
    crate::parallel::fill(size, 1, |x, y| {
        let (u, v) = ((x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32);
        let fine = layered(u, v, 16, 2, 0.5, 9) * 0.3;
        [0.1 + 0.5 * u + fine, 0.2 + 0.4 * v + fine, 0.5 + fine * 0.5]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{assert_in_range, assert_seamless, registry, with};
    use crate::image::Image;
    use crate::seams::SeamCheck;
    use crate::settings::{SettingValue, Settings};

    fn run(value: Value, settings: &Settings) -> Value {
        let info = registry().get(SEAMLESS).unwrap().info();
        let size = Size::new(value.size()).unwrap();
        let index = match value {
            Value::Height(_) | Value::Shape(_) => 0,
            Value::Colour(_) => 1,
            Value::Material(_) => 2,
        };
        let mut inputs = vec![None, None, None];
        inputs[index] = Some(value);
        let job = Job::new(info, size, 4, 2, settings, inputs);
        registry().get(SEAMLESS).unwrap().run_outputs(&job).unwrap()[index]
            .clone()
            .unwrap()
    }

    #[test]
    fn a_photo_that_doesnt_tile_comes_out_seamless() {
        let photo = Value::Colour(Arc::new(not_tiling(128)));
        assert!(SeamCheck::of(&photo).shows(), "the test picture has joins");
        for settings in [
            Settings::new(),
            with(&[("width", SettingValue::Number(0.05))]),
            with(&[
                ("width", SettingValue::Number(0.5)),
                ("wobble", SettingValue::Number(1.0)),
            ]),
            with(&[("wobble", SettingValue::Number(0.0))]),
        ] {
            let out = run(photo.clone(), &settings);
            let check = SeamCheck::of(&out);
            assert!(!check.shows(), "{}", check.describe());
            assert_seamless("seamless", std::slice::from_ref(&out));
            assert_in_range("seamless", &[out]);
        }
    }

    #[test]
    fn the_middle_is_the_picture_itself() {
        let photo = not_tiling(64);
        let out = run(Value::Colour(Arc::new(photo.clone())), &Settings::new());
        let c = out.colour().unwrap();
        // A quarter in from each edge and further, untouched.
        for (x, y) in [(32, 32), (24, 40)] {
            for k in 0..3 {
                assert!((c.at(x, y)[k] - photo.at(x, y)[k]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn the_contrast_is_kept() {
        // Fine, strong noise: a plain half-and-half mix would halve its
        // spread where the copies blend; the kept mix doesn't.
        let size = Size::new(64).unwrap();
        let noisy: Grey = crate::parallel::fill(size, 1, |x, y| {
            crate::seed::unit(crate::seed::lattice(3, 0, i64::from(x), i64::from(y)))
        });
        let out = run(Value::Height(Arc::new(noisy.clone())), &Settings::new());
        let spread = |g: &Grey, x0: i64| {
            let mean = 0.5;
            let mut s = 0.0;
            for y in 0..64 {
                for x in x0..x0 + 4 {
                    s += (g.at(x, y) - mean).powi(2);
                }
            }
            (s / 256.0f32).sqrt()
        };
        // Columns 6 to 9 are well inside the blend.
        let before = spread(&noisy, 6);
        let after = spread(out.grey().unwrap(), 6);
        assert!(after > before * 0.8, "before {before} after {after}");
    }

    #[test]
    fn heights_and_materials_come_out_seamless_too() {
        let size = Size::new(64).unwrap();
        let ramp: Grey = crate::parallel::fill(size, 1, |x, y| (x + y) as f32 / 126.0);
        let out = run(Value::Height(Arc::new(ramp)), &Settings::new());
        assert!(!SeamCheck::of(&out).shows());
        let material = crate::boxes::tests_each::input(crate::ValueKind::Material, size);
        let out = run(material, &Settings::new());
        assert_seamless("material", std::slice::from_ref(&out));
        assert_in_range("material", &[out]);
        // A flat picture stays flat.
        let flat = Value::Colour(Arc::new(Image::filled(size, [0.3, 0.6, 0.9])));
        let out = run(flat, &Settings::new());
        assert!(
            out.colour()
                .unwrap()
                .pixels()
                .iter()
                .all(|p| (p[2] - 0.9).abs() < 1e-4)
        );
    }

    #[test]
    fn nothing_plugged_in_is_refused() {
        let info = registry().get(SEAMLESS).unwrap().info();
        let job = Job::new(
            info,
            Size::new(16).unwrap(),
            1,
            1,
            &Settings::new(),
            vec![None, None, None],
        );
        assert!(registry().get(SEAMLESS).unwrap().run_outputs(&job).is_err());
    }
}
