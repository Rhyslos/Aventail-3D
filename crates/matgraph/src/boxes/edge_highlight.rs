//! Edge highlight: lightens the raised edges of a height and darkens its
//! hollows, as hand painters pick out the edges of stones, planks and
//! bricks (Phase 12, milestone 10; decision 0272).
//!
//! An edge is where the height stands above the average round it (the
//! height less a soft blur of it, Width wide); a hollow is where it sits
//! below. Both come from the texture's own height, so they suit any
//! model; edge wear that follows a model's own corners is the From the
//! model boxes' job (and the weathering set's, later).

use super::pattern::soft_blur;
use super::{colour, info, length, moving, number};
use crate::image::Rgb;
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Edge highlight box.
#[derive(Debug)]
pub(crate) struct EdgeHighlight {
    info: BoxInfo,
}

/// How strongly a step in height reads as an edge.
const GAIN: f32 = 8.0;

impl EdgeHighlight {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "edge_highlight",
                text!("Edge highlight"),
                Group::Colour,
                text!(
                    "Lightens the raised edges of a height and darkens its hollows, as hand \
                     painters pick out the edges of stones, planks and bricks. Plug the height \
                     into Height and the colour (or a whole material) to light. Width is how far \
                     in from an edge the highlight reaches, in centimetres on the finished \
                     material. Edges gives the highlight alone, to use as a shape elsewhere."
                ),
                (
                    vec![
                        PlugSpec::new("height", text!("Height"), ValueKind::Height, true),
                        PlugSpec::new("colour", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new("material", text!("Material"), ValueKind::Material, false),
                    ],
                    vec![
                        PlugSpec::new("colour_out", text!("Colour"), ValueKind::Colour, false),
                        PlugSpec::new(
                            "material_out",
                            text!("Material"),
                            ValueKind::Material,
                            false,
                        ),
                        PlugSpec::new("edges", text!("Edges"), ValueKind::Height, false),
                    ],
                ),
                vec![
                    length(
                        "width",
                        text!("Width"),
                        text!("How far in from an edge the highlight reaches, in centimetres."),
                        0.05,
                        50.0,
                        1.5,
                    ),
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How strongly the edges are lightened."),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    colour(
                        "highlight",
                        text!("Highlight colour"),
                        text!("The colour the edges are lightened toward."),
                        [1.0, 0.95, 0.82],
                    ),
                    number(
                        "hollows",
                        text!("Darken hollows"),
                        text!("How much the hollows are darkened: 0 leaves them."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                ],
                &["material_from_height", "directional_light", "mix_materials"],
            ),
        }
    }
}

impl BoxType for EdgeHighlight {
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
        let width = job.length_px("width").max(0.5);
        let amount = job.number("amount").clamp(0.0, 1.0);
        let light = job.colour("highlight").map(|c| c.clamp(0.0, 1.0));
        let hollows = job.number("hollows").clamp(0.0, 1.0);
        let average = soft_blur(height, width, job.threads());
        // Above the average round it is an edge (positive), below a
        // hollow (negative), -1 to 1.
        let raised = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            ((height.at(x, y) - average.at(x, y)) * GAIN).clamp(-1.0, 1.0)
        });
        let lit = |c: &Rgb| -> Rgb {
            job.fill_rgb(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let r = raised.at(x, y);
                let (edge, hollow) = (r.max(0.0) * amount, (-r).max(0.0) * hollows * 0.6);
                let base = c.at(x, y);
                [0, 1, 2].map(|i| {
                    let up = base[i] + (light[i] - base[i]) * edge;
                    (up * (1.0 - hollow)).clamp(0.0, 1.0)
                })
            })
        };
        let colour_out = job
            .input("colour")
            .and_then(Value::colour)
            .map(|c| Value::Colour(Arc::new(lit(c))));
        let material_out = job.input("material").and_then(Value::material).map(|m| {
            Value::Material(Arc::new(MaterialMaps {
                colour: Arc::new(lit(&m.colour)),
                ..(**m).clone()
            }))
        });
        let edges = job.map_grey(&raised, |r| r.max(0.0));
        Ok(vec![
            colour_out,
            material_out,
            Some(Value::Height(Arc::new(edges))),
        ])
    }
}
