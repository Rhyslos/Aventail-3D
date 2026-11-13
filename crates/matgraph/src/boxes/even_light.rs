//! Even out light (Phase 12 milestone 21, [decision 0283](../../../../docs/decisions/0283-image-to-material.md)):
//! takes the light and shadow a photo was taken in out of it, so the
//! material can be lit by the game instead.
//!
//! A photo of a wall lit from one side is lighter on that side; one
//! taken under a tree has patches of shade. Repeated as a texture, those
//! become stripes and blotches. The box blurs the picture's lightness
//! heavily (over **Light patch size**), which keeps only the large-scale
//! light and shadow and none of the stones or grain, and divides each
//! pixel by it, times the picture's average: dark patches are lifted,
//! light ones lowered, the detail kept (a high-pass against a heavy
//! blur). **Strength** says how much. **Edges** says whether the picture
//! already tiles (the blur wraps round) or is a photo straight from the
//! Image box (its far edges kept apart, `photo_maths::soft_blur_at_edges`).

use super::photo_maths::{lightness_of, mean, soft_blur_at_edges};
use super::{choice, info, number};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::sync::Arc;
use words::text;

/// The box's id.
pub const EVEN_LIGHT: &str = "even_light";

/// The Edges setting's key, and its choice for a photo that doesn't tile
/// yet.
pub const EDGES: &str = "edges";
/// Edges: a photo straight from the Image box.
pub const PHOTO_EDGES: u32 = 1;

/// The least and most each pixel is lightened or darkened by.
const MOST_CHANGE: f32 = 4.0;

/// The Even out light box.
#[derive(Debug)]
pub(crate) struct EvenLight {
    info: BoxInfo,
}

impl EvenLight {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                EVEN_LIGHT,
                text!("Even out light"),
                Group::Colour,
                text!(
                    "Takes the light and shadow a photo was taken in out of it: one side \
                     lighter than the other, or patches of shade, which would show as stripes \
                     and blotches when the texture repeats. It finds the light by blurring the \
                     picture heavily (only big patches of light are left) and evens them out, \
                     keeping the small detail. The game lights the material instead."
                ),
                (
                    vec![PlugSpec::new(
                        "colour",
                        text!("Colour"),
                        ValueKind::Colour,
                        true,
                    )],
                    vec![PlugSpec::new(
                        "colour_out",
                        text!("Colour"),
                        ValueKind::Colour,
                        false,
                    )],
                ),
                vec![
                    number(
                        "strength",
                        text!("Strength"),
                        text!("How much of the light and shadow is taken out: 0 none, 1 all."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    number(
                        "size",
                        text!("Light patch size"),
                        text!(
                            "How big a patch of light or shade has to be to be taken out, as a \
                             share of the texture's width. Bigger keeps more of the picture's \
                             own darker and lighter parts; smaller evens out more, but can wash \
                             out large stones or planks."
                        ),
                        0.05,
                        1.0,
                        0.25,
                    ),
                    choice(
                        EDGES,
                        text!("Edges"),
                        text!(
                            "Tiling: the picture already repeats cleanly, so the light is found \
                             wrapping round its edges. Photo: straight from an Image box, not \
                             made seamless yet, so its edges are kept apart."
                        ),
                        &[text!("Tiling"), text!("Photo")],
                        0,
                    ),
                ],
                &["seamless", "guess_height", "guess_roughness"],
            ),
        }
    }
}

impl BoxType for EvenLight {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let colour = job.needed_colour("colour")?.clone();
        let threads = job.threads();
        let strength = job.number("strength").clamp(0.0, 1.0);
        let n = job.size().get() as f32;
        let radius = job.number("size").clamp(0.05, 1.0) * n * 0.5;
        let wraps = job.choice(EDGES) != PHOTO_EDGES;
        let light = lightness_of(&colour, threads);
        let average = mean(&light).max(1e-3);
        let blurred = soft_blur_at_edges(&light, radius, wraps, threads);
        let out = job.fill_rgb(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let here = blurred.at(x, y).max(0.02);
            let gain = (average / here)
                .powf(strength)
                .clamp(1.0 / MOST_CHANGE, MOST_CHANGE);
            colour.at(x, y).map(|v| (v * gain).clamp(0.0, 1.0))
        });
        Ok(vec![Value::Colour(Arc::new(out))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::tests_each::{registry, with};
    use crate::image::{Rgb, Size};
    use crate::settings::SettingValue;

    /// A pattern of small spots, lit from the left: three times as light
    /// on the right as on the left.
    fn lit_from_one_side(n: u32) -> Rgb {
        let size = Size::new(n).unwrap();
        crate::parallel::fill(size, 1, |x, y| {
            let spots = if (x / 3 + y / 3) % 2 == 0 { 0.8 } else { 1.0 };
            let light = 0.25 + 0.5 * x as f32 / (n - 1) as f32;
            [spots * light; 3]
        })
    }

    fn run(colour: &Rgb, settings: &crate::Settings) -> Rgb {
        let info = registry().get(EVEN_LIGHT).unwrap().info();
        let job = Job::new(
            info,
            Size::new(colour.size()).unwrap(),
            1,
            2,
            settings,
            vec![Some(Value::Colour(Arc::new(colour.clone())))],
        );
        let out = registry().get(EVEN_LIGHT).unwrap().run(&job).unwrap();
        out[0].colour().unwrap().as_ref().clone()
    }

    /// The average lightness of the left and the right quarter.
    fn sides(c: &Rgb) -> (f32, f32) {
        let n = c.size() as i64;
        let (mut left, mut right) = (0.0, 0.0);
        for y in 0..n {
            for x in 0..n / 4 {
                left += c.at(x, y)[0];
                right += c.at(n - 1 - x, y)[0];
            }
        }
        let count = (n * n / 4) as f32;
        (left / count, right / count)
    }

    #[test]
    fn light_from_one_side_is_evened_out() {
        let photo = lit_from_one_side(128);
        let (l, r) = sides(&photo);
        let before = (r - l).abs();
        let s = with(&[(EDGES, SettingValue::Choice(PHOTO_EDGES))]);
        let (l, r) = sides(&run(&photo, &s));
        let after = (r - l).abs();
        assert!(after < before * 0.15, "before {before} after {after}");
        // The spots are kept.
        let out = run(&photo, &s);
        assert!((out.at(64, 1)[0] - out.at(64, 4)[0]).abs() > 0.05);
    }

    #[test]
    fn strength_zero_changes_nothing_and_a_flat_picture_stays() {
        let photo = lit_from_one_side(64);
        let none = run(&photo, &with(&[("strength", SettingValue::Number(0.0))]));
        for (a, b) in none.pixels().iter().zip(photo.pixels()) {
            assert!((a[0] - b[0]).abs() < 1e-5);
        }
        let flat: Rgb = crate::image::Image::filled(Size::new(32).unwrap(), [0.4, 0.5, 0.6]);
        let out = run(&flat, &crate::Settings::new());
        for p in out.pixels() {
            assert!((p[1] - 0.5).abs() < 1e-3, "{p:?}");
        }
    }

    #[test]
    fn black_and_white_never_break_it() {
        let size = Size::new(16).unwrap();
        for v in [0.0, 1.0] {
            let c: Rgb = crate::image::Image::filled(size, [v; 3]);
            for edges in [0, 1] {
                let out = run(&c, &with(&[(EDGES, SettingValue::Choice(edges))]));
                assert!(out.pixels().iter().flatten().all(|p| p.is_finite()));
            }
        }
    }
}
