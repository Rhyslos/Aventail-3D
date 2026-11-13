//! Fill the gaps: fills a height's low places up to a level, as moss,
//! dirt, sand or grout settle between stones, tiles and planks (Phase 12,
//! milestone 10; decision 0272).
//!
//! The filling comes up to Level (made patchy by the box's own noise, so
//! moss creeps further in some places than others) and lies flat on top.
//! It gives where the filling is, the filled height, and, when plugged
//! in, the colour or material with the filling's colour and roughness
//! laid into the gaps. Mix materials (by height) lays a whole second
//! material on top; this box is the quicker way when the filling is just
//! a colour, and it gives the gaps as a shape to use elsewhere.

use super::noise::layered;
use super::pattern::{smooth, whole_repeats};
use super::{colour, info, length, moving, number, seed, uv};
use crate::image::Grey;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::seed::mix;
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Fill the gaps box.
#[derive(Debug)]
pub(crate) struct FillGaps {
    info: BoxInfo,
}

/// The most noise squares across, for the patches.
const MOST_PATCHES: i64 = 256;

impl FillGaps {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "fill_gaps",
                text!("Fill the gaps"),
                Group::Detail,
                text!(
                    "Fills the low places of a height up to a level, as moss, dirt, sand or \
                     grout settle between stones, tiles and planks. Level is how high the \
                     filling comes; Patchy lets it creep further in some places than others, in \
                     patches Patch size wide (centimetres on the finished material). It gives \
                     where the filling is (Gaps), the height with the gaps filled, and, when a \
                     colour or a material is plugged in, the same with the filling's colour \
                     (and roughness) laid into the gaps. To fill with a whole second material, \
                     use Mix materials by height."
                ),
                (
                    vec![
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, true),
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new("material", text!("Material"), ValueKind::Material, false),
                    ],
                    vec![
                        PlugSpec::new("out", text!("Height"), ValueKind::Height, false),
                        PlugSpec::new("gaps", text!("Gaps"), ValueKind::Shape, false),
                        PlugSpec::new("colour_out", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new(
                            "material_out",
                            text!("Material"),
                            ValueKind::Material,
                            false,
                        ),
                    ],
                ),
                vec![
                    number(
                        "level",
                        text!("Level"),
                        text!("How high the filling comes: 0 is none, 1 covers everything."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("How gently the filling fades out up the sides."),
                        0.0,
                        1.0,
                        0.15,
                    ),
                    number(
                        "patchy",
                        text!("Patchy"),
                        text!("How much higher or lower the filling comes in patches."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    length(
                        "patch_size",
                        text!("Patch size"),
                        text!("How wide the patches are, in centimetres."),
                        0.5,
                        200.0,
                        20.0,
                    ),
                    colour(
                        "fill_colour",
                        text!("Filling colour"),
                        text!("The filling's colour: moss green, earth brown, grout grey."),
                        [0.33, 0.43, 0.17],
                    ),
                    number(
                        "colour_vary",
                        text!("Colour varies"),
                        text!("How much lighter or darker the filling is in patches."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    number(
                        "roughness",
                        text!("Filling roughness"),
                        text!("How rough the filling is, for a material: 1 is fully matt."),
                        0.0,
                        1.0,
                        0.9,
                    ),
                    seed(),
                ],
                &["material_from_height", "mix_materials", "gradient_map"],
            ),
        }
    }
}

/// The filling at one pixel: how much (0 to 1), how high its top is, and
/// how light it is there.
#[derive(Debug, Clone, Copy, Default)]
struct Fill {
    share: f32,
    top: f32,
    light: f32,
}

impl BoxType for FillGaps {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let size = job.size();
        let kinds = self.info.outputs.iter().map(|o| o.kind);
        Ok(self
            .run_outputs(job)?
            .into_iter()
            .zip(kinds)
            .map(|(v, kind)| v.unwrap_or_else(|| moving::plain(kind, size)))
            .collect())
    }

    fn run_outputs(&self, job: &Job<'_>) -> Result<Vec<Option<Value>>, Refusal> {
        let height = job.needed_grey("height")?;
        let n = job.size().get() as f32;
        let level = job.number("level").clamp(0.0, 1.0);
        let soft = (job.number("softness").clamp(0.0, 1.0) * 0.25).max(0.004);
        let patchy = job.number("patchy").clamp(0.0, 1.0);
        let patches = whole_repeats(n, job.length_px("patch_size"), MOST_PATCHES) as u32;
        let vary = job.number("colour_vary").clamp(0.0, 1.0);
        let fill_colour = job.colour("fill_colour").map(|c| c.clamp(0.0, 1.0));
        let rough = job.number("roughness").clamp(0.0, 1.0);
        let seed = job.seed();
        // Nothing at all at level 0, whatever the patches say.
        let any = smooth(0.0, 0.02, level);
        let fills: crate::image::Image<Fill> =
            crate::parallel::fill(job.size(), job.threads(), |x, y| {
                let (u, v) = uv(x, y, n);
                let h = height.at(i64::from(x), i64::from(y));
                let patch = layered(u, v, patches, 4, 0.5, mix(seed, 1)) - 0.5;
                let top = (level + patch * patchy * 0.6).clamp(0.0, 1.0);
                let share = smooth(-soft, soft, top - h) * any;
                let light = 1.0 + (layered(u, v, patches * 2, 3, 0.5, mix(seed, 2)) - 0.5) * vary;
                Fill { share, top, light }
            });
        let at = |x: u32, y: u32| fills.at(i64::from(x), i64::from(y));
        let filled = |g: &Grey| -> Grey {
            job.fill_grey(|x, y| {
                let f = at(x, y);
                let h = g.at(i64::from(x), i64::from(y));
                (h + (f.top.max(h) - h) * f.share).clamp(0.0, 1.0)
            })
        };
        let gaps = job.fill_grey(|x, y| at(x, y).share);
        let paint = |x: u32, y: u32, under: [f32; 3]| {
            let f = at(x, y);
            let fill = fill_colour.map(|c| (c * f.light).clamp(0.0, 1.0));
            [0, 1, 2].map(|i| (under[i] + (fill[i] - under[i]) * f.share).clamp(0.0, 1.0))
        };
        let colour_out = job.input("colour").and_then(Value::colour).map(|c| {
            Value::Colour(Arc::new(
                job.fill_rgb(|x, y| paint(x, y, c.at(i64::from(x), i64::from(y)))),
            ))
        });
        let material_out = job.input("material").and_then(Value::material).map(|m| {
            let toward = |g: &Grey, to: f32| {
                Arc::new(job.fill_grey(|x, y| {
                    let v = g.at(i64::from(x), i64::from(y));
                    v + (to - v) * at(x, y).share
                }))
            };
            Value::Material(Arc::new(MaterialMaps {
                colour: Arc::new(
                    job.fill_rgb(|x, y| paint(x, y, m.colour.at(i64::from(x), i64::from(y)))),
                ),
                height: Arc::new(filled(&m.height)),
                roughness: toward(&m.roughness, rough),
                metal: toward(&m.metal, 0.0),
                // The filling lies flat.
                normal: Arc::new(job.fill_rgb(|x, y| {
                    let k = at(x, y).share;
                    glam::Vec3::from_array(m.normal.at(i64::from(x), i64::from(y)))
                        .lerp(glam::Vec3::Z, k)
                        .normalize_or(glam::Vec3::Z)
                        .to_array()
                })),
                occlusion: m.occlusion.clone(),
                curvature: toward(&m.curvature, 0.5),
            }))
        });
        Ok(vec![
            Some(Value::Height(Arc::new(filled(height)))),
            Some(Value::Shape(Arc::new(gaps))),
            colour_out,
            material_out,
        ])
    }
}
