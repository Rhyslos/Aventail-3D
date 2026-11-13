//! Edge wear: paint or the top layer worn off outer edges in chips,
//! showing what is under it (milestone 25, decision 0288).

use super::super::{colour, info, length, number, seed};
use super::{Coat, Follow, Toward, coated, covered, follow_material, follow_model, limited};
use super::{outputs, patches, plugs, seeded};
use crate::kind::ValueKind;
use crate::model::ModelMap;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The model's maps Edge wear reads.
const MAPS: &[ModelMap] = &[ModelMap::Curvature];

/// How far the worn parts sink, at Depth 1 (a share of the height's
/// range): the thickness of the paint.
const PAINT: f32 = 0.05;

/// The Edge wear box.
#[derive(Debug)]
pub(crate) struct EdgeWear {
    info: BoxInfo,
}

impl EdgeWear {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "edge_wear",
                text!("Edge wear"),
                Group::Weather,
                text!(
                    "Paint (or any top layer) worn off the outer edges in chips, where things \
                     get knocked and handled, showing what is under it: bare metal to start \
                     with, or a whole material plugged into Under (wood under paint, stone \
                     under plaster). Plug the top material into Material. In the painter it \
                     follows the model's own edges; in the Material mode it shows on the \
                     stand-in of rounded bumps. Wear gives where it is worn; a shape in Only \
                     where keeps it there."
                ),
                plugs(
                    text!("Wear"),
                    vec![PlugSpec::new(
                        "under",
                        text!("Under"),
                        ValueKind::Material,
                        false,
                    )],
                ),
                vec![
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How far in from the edges it is worn: 0 is none."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("How gently the wear fades out: 0 is sharp chips."),
                        0.0,
                        1.0,
                        0.1,
                    ),
                    number(
                        "breakup",
                        text!("Break-up"),
                        text!("How ragged the wear is: 0 follows the edges evenly."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    length(
                        "chip_size",
                        text!("Chip size"),
                        text!("How big the chips are, in centimetres."),
                        0.1,
                        50.0,
                        3.0,
                    ),
                    number(
                        "depth",
                        text!("Depth"),
                        text!("How far the worn parts sink: the paint's thickness."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    colour(
                        "under_colour",
                        text!("Under colour"),
                        text!("What shows through, with nothing in Under: bare steel to start."),
                        [0.56, 0.56, 0.58],
                    ),
                    number(
                        "under_metal",
                        text!("Under metal"),
                        text!("How metal what shows through is, with nothing in Under."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    number(
                        "under_roughness",
                        text!("Under roughness"),
                        text!("How rough what shows through is, with nothing in Under."),
                        0.0,
                        1.0,
                        0.35,
                    ),
                    follow_model(),
                    follow_material(),
                    seed(),
                ],
                &["dirt", "rust", "material_out"],
            ),
        }
    }
}

impl BoxType for EdgeWear {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn model_maps(&self) -> &'static [ModelMap] {
        MAPS
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let m = job.needed_material("material")?;
        let follow = Follow::new(job, m, MAPS);
        let amount = job.number("amount").clamp(0.0, 1.0);
        let soft = (job.number("softness").clamp(0.0, 1.0) * 0.3).max(0.005);
        let breakup = job.number("breakup").clamp(0.0, 1.0);
        let depth = job.number("depth").clamp(0.0, 1.0);
        let chips = patches(job, job.length_px("chip_size"), 4, seeded(job, 1));
        let cover = job.fill_grey(|x, y| {
            let edge = (follow.at(x, y).edge * 1.25).min(1.0);
            let ragged = (chips.at(i64::from(x), i64::from(y)) - 0.5) * breakup * 1.5;
            covered(edge + ragged, amount, soft)
        });
        let cover = limited(job, cover, job.grey("where"));
        let under: Option<&Arc<MaterialMaps>> = job.input("under").and_then(Value::material);
        let plain = job.fill_rgb({
            let c = job.colour("under_colour").map(|c| c.clamp(0.0, 1.0));
            move |_, _| c
        });
        let raise = job.fill_grey(|x, y| -depth * PAINT * cover.at(i64::from(x), i64::from(y)));
        let coat = match under {
            Some(u) => Coat {
                cover: &cover,
                colour: &u.colour,
                roughness: Toward::Each(&u.roughness),
                metal: Some(Toward::Each(&u.metal)),
                raise: (depth > 0.0).then_some(raise),
            },
            None => Coat {
                cover: &cover,
                colour: &plain,
                roughness: Toward::All(job.number("under_roughness")),
                metal: Some(Toward::All(job.number("under_metal"))),
                raise: (depth > 0.0).then_some(raise),
            },
        };
        Ok(outputs(coated(job, m, &coat), cover))
    }
}
