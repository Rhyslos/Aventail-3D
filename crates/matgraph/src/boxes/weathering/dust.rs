//! Dust: a pale, matt layer settling from above on what faces up, and in
//! corners (milestone 25, decision 0288).

use super::super::{colour, info, length, number, seed};
use super::{
    Coat, Follow, Toward, coated, covered, follow_material, follow_model, limited, outputs,
};
use super::{patches, plugs, seeded};
use crate::model::ModelMap;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::Value;
use words::text;

/// The model's maps Dust reads.
const MAPS: &[ModelMap] = &[ModelMap::FacingUp, ModelMap::Occlusion];

/// The Dust box.
#[derive(Debug)]
pub(crate) struct Dust {
    info: BoxInfo,
}

impl Dust {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "dust",
                text!("Dust"),
                Group::Weather,
                text!(
                    "A pale, matt layer of dust settling from above: on the tops of things and \
                     on ledges, less on the sides, none underneath, with a little more in \
                     corners. Plug a whole material into Material. In the painter it follows \
                     which way the model faces; a material on its own counts as lying flat, \
                     like a floor, so its flat parts gather dust and its slopes less. Dust gives \
                     where the dust is; a shape in Only where keeps it there."
                ),
                plugs(text!("Dust"), Vec::new()),
                vec![
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How much dust has settled: 0 is none, 1 covers even the sides."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("How gently the dust fades out at its edges."),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    number(
                        "corners",
                        text!("In corners"),
                        text!("How much extra dust gathers in corners and crevices."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    number(
                        "patchy",
                        text!("Patchy"),
                        text!("How much thicker the dust lies in some places than others."),
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
                        25.0,
                    ),
                    colour(
                        "dust_colour",
                        text!("Dust colour"),
                        text!("The dust's colour: pale grey, or sandy for a desert."),
                        [0.66, 0.63, 0.58],
                    ),
                    number(
                        "thickness",
                        text!("Thickness"),
                        text!(
                            "How thick the dust is where it lies: low lets the colour under it show through."
                        ),
                        0.0,
                        1.0,
                        0.8,
                    ),
                    number(
                        "roughness",
                        text!("Dust roughness"),
                        text!("How rough the dust is: 1 is fully matt."),
                        0.0,
                        1.0,
                        0.95,
                    ),
                    follow_model(),
                    follow_material(),
                    seed(),
                ],
                &["dirt", "water_streaks", "material_out"],
            ),
        }
    }
}

impl BoxType for Dust {
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
        let soft = (job.number("softness").clamp(0.0, 1.0) * 0.4).max(0.01);
        let corners = job.number("corners").clamp(0.0, 1.0);
        let patchy = job.number("patchy").clamp(0.0, 1.0);
        let thickness = job.number("thickness").clamp(0.0, 1.0);
        let dust = job.colour("dust_colour").map(|c| c.clamp(0.0, 1.0));
        let size = job.length_px("patch_size");
        let big = patches(job, size, 4, seeded(job, 1));
        let fine = patches(job, size * 0.1, 2, seeded(job, 2));
        let cover = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            let lie = follow.at(x, y);
            let reach = lie.up.max(lie.enclosed * 1.4 * corners);
            let patch = (big.at(i, j) - 0.5) + (fine.at(i, j) - 0.5) * 0.3;
            covered(reach + patch * patchy, amount, soft) * thickness
        });
        let cover = limited(job, cover, job.grey("where"));
        let colour = job.fill_rgb(|_, _| dust);
        let coat = Coat {
            cover: &cover,
            colour: &colour,
            roughness: Toward::All(job.number("roughness")),
            // Dust over bare metal isn't metal.
            metal: Some(Toward::All(0.0)),
            raise: None,
        };
        Ok(outputs(coated(job, m, &coat), cover))
    }
}
