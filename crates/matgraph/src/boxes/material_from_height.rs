//! Material from height: a whole material from a height and a colour.
//!
//! The "height first" way of working: most graphs only shape a height and
//! colour it; this box works out the rest. The normal comes from how
//! steeply the height changes; occlusion from how far below its
//! surroundings each place sits; curvature from how far above or below
//! its nearest neighbours.

use super::blur::box_blur;
use super::{colour, next, number};
use crate::image::{Grey, Image};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::borrow::Cow;
use std::sync::Arc;
use words::text;

/// The Material from height box.
#[derive(Debug)]
pub(crate) struct MaterialFromHeight {
    info: BoxInfo,
}

impl MaterialFromHeight {
    pub(crate) fn new() -> Self {
        Self {
            info: BoxInfo {
                id: Cow::Borrowed("material_from_height"),
                name: Label::Key(text!("Material from height")),
                group: Group::Material,
                about: Label::Key(text!(
                    "Turns a height into a whole material: it works out the normal (how light \
                     catches the bumps), the occlusion (darker in dips) and the curvature \
                     (lighter on ridges), and adds a colour, roughness and metal. With nothing in \
                     Colour, the Colour setting is used; with nothing in Roughness, the \
                     Roughness setting."
                )),
                inputs: vec![
                    PlugSpec::new("height", text!("Height"), ValueKind::Height, true),
                    PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                    // Milestone 21: a roughness guessed from a photo, or any
                    // greys (0 shiny, 1 matt).
                    PlugSpec::new("roughness", text!("Roughness"), ValueKind::Height, false),
                ],
                outputs: vec![PlugSpec::new(
                    "material",
                    text!("Material"),
                    ValueKind::Material,
                    false,
                )],
                settings: vec![
                    colour(
                        "colour",
                        text!("Colour"),
                        text!("The colour when nothing is plugged into Colour."),
                        [0.6, 0.6, 0.6],
                    ),
                    number(
                        "depth",
                        text!("Depth"),
                        text!(
                            "How deep the bumps look: how far from black to white, as a share \
                             (in hundredths) of the texture's width."
                        ),
                        0.0,
                        20.0,
                        2.0,
                    ),
                    number(
                        "roughness",
                        text!("Roughness"),
                        text!(
                            "0 is shiny, 1 is matt. Used when nothing is plugged into \
                             Roughness."
                        ),
                        0.0,
                        1.0,
                        0.7,
                    ),
                    number(
                        "metal",
                        text!("Metal"),
                        text!("0 is not metal, 1 is bare metal."),
                        0.0,
                        1.0,
                        0.0,
                    ),
                    number(
                        "occlusion",
                        text!("Occlusion"),
                        text!("How much darker the dips are."),
                        0.0,
                        4.0,
                        1.0,
                    ),
                ],
                comes_next: next(&["material_out", "mix_materials", "dirt", "edge_wear"]),
                one_per_graph: false,
                animated: false,
            },
        }
    }
}

impl BoxType for MaterialFromHeight {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let size = job.size();
        let height = job.needed_grey("height")?.clone();
        let colour = match job.input("colour").and_then(Value::colour) {
            Some(c) => c.clone(),
            None => Arc::new(Image::filled(size, job.colour("colour"))),
        };
        let maps = MaterialMaps {
            normal: Arc::new(normal(job, &height, job.number("depth"))),
            occlusion: Arc::new(occlusion(job, &height, job.number("occlusion"))),
            curvature: Arc::new(curvature(job, &height)),
            opacity: None,
            translucency: None,
            roughness: match job.grey("roughness") {
                Some(r) => Arc::new(job.map_grey(r, |v| v.clamp(0.0, 1.0))),
                None => Arc::new(Image::filled(size, job.number("roughness"))),
            },
            metal: Arc::new(Image::filled(size, job.number("metal"))),
            colour,
            height,
        };
        Ok(vec![Value::Material(Arc::new(maps))])
    }
}

/// The normal of `height` with bumps `depth` hundredths of the texture's
/// width deep, OpenGL way (y up the texture).
pub(super) fn normal(job: &Job<'_>, height: &Grey, depth: f32) -> Image<[f32; 3]> {
    let k = 0.5 * job.size().get() as f32 * depth / 100.0;
    job.fill_rgb(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        // Row 0 is the top, so up the texture is towards smaller rows.
        let across = (height.at(x + 1, y) - height.at(x - 1, y)) * k;
        let up = (height.at(x, y - 1) - height.at(x, y + 1)) * k;
        let n = glam::Vec3::new(-across, -up, 1.0).normalize_or(glam::Vec3::Z);
        n.to_array()
    })
}

/// How open each place is: 1 level with or above its surroundings,
/// darker below them.
pub(super) fn occlusion(job: &Job<'_>, height: &Grey, strength: f32) -> Grey {
    let radius = (job.size().get() / 32).max(1);
    let around = box_blur(height, radius, job.threads());
    job.fill_grey(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        let below = (around.at(x, y) - height.at(x, y)).max(0.0);
        (1.0 - below * 4.0 * strength).clamp(0.0, 1.0)
    })
}

/// How each place bends: 0.5 flat, lighter on ridges, darker in grooves.
pub(super) fn curvature(job: &Job<'_>, height: &Grey) -> Grey {
    let radius = (job.size().get() / 256).max(1);
    let around = box_blur(height, radius, job.threads());
    job.fill_grey(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        (0.5 + (height.at(x, y) - around.at(x, y)) * 8.0).clamp(0.0, 1.0)
    })
}
