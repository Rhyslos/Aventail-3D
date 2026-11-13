//! Rust: blotchy orange-brown rust eating into metal, starting at edges and
//! crevices where water sits (milestone 25, decision 0288).

use super::super::pattern::smooth;
use super::super::{colour, info, length, number, seed, switch};
use super::{
    Coat, Follow, Toward, coated, covered, follow_material, follow_model, limited, outputs,
};
use super::{patches, plugs, seeded, toward};
use crate::model::ModelMap;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job};
use crate::value::Value;
use words::text;

/// The model's maps Rust reads.
const MAPS: &[ModelMap] = &[ModelMap::Curvature, ModelMap::Occlusion];

/// How high rust's flakes stand, at Flaking 1 (a share of the height's
/// range).
const FLAKES: f32 = 0.06;

/// The Rust box.
#[derive(Debug)]
pub(crate) struct Rust {
    info: BoxInfo,
}

impl Rust {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "rust",
                text!("Rust"),
                Group::Weather,
                text!(
                    "Blotchy orange-brown rust eating into metal, starting at edges and in \
                     crevices where water sits, then spreading in patches. Where it is, the \
                     material turns rough and stops being metal, and the rust stands up a \
                     little in flakes. Plug a whole material into Material. In the painter it \
                     follows the model's edges and crevices; in the Material mode it shows on \
                     the stand-in of rounded bumps. Rust gives where the rust is (Water streaks \
                     can start from it); a shape in Only where keeps it there."
                ),
                plugs(text!("Rust"), Vec::new()),
                vec![
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How far the rust has spread: 0 is none, 1 covers nearly all."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("How gently the rust fades out at its edges."),
                        0.0,
                        1.0,
                        0.2,
                    ),
                    number(
                        "at_edges",
                        text!("Starts at edges"),
                        text!(
                            "How much the rust keeps to edges and crevices: 0 is patches \
                             anywhere."
                        ),
                        0.0,
                        1.0,
                        0.6,
                    ),
                    length(
                        "patch_size",
                        text!("Patch size"),
                        text!("How wide the rust's blotches are, in centimetres."),
                        0.5,
                        200.0,
                        10.0,
                    ),
                    colour(
                        "dark",
                        text!("Dark rust"),
                        text!("The rust's darkest colour, deep in a patch."),
                        [0.24, 0.09, 0.04],
                    ),
                    colour(
                        "light",
                        text!("Light rust"),
                        text!("The rust's brightest colour, fresh orange."),
                        [0.62, 0.29, 0.1],
                    ),
                    number(
                        "roughness",
                        text!("Rust roughness"),
                        text!("How rough the rust is: 1 is fully matt."),
                        0.0,
                        1.0,
                        0.9,
                    ),
                    number(
                        "flaking",
                        text!("Flaking"),
                        text!("How much the rust stands up in flakes and pits."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    switch(
                        "metal_only",
                        text!("Only on metal"),
                        text!(
                            "Rust only where the material is metal (its metal map), as on a \
                             painted machine whose bare parts rust."
                        ),
                    ),
                    follow_model(),
                    follow_material(),
                    seed(),
                ],
                &["water_streaks", "edge_wear", "material_out"],
            ),
        }
    }
}

impl BoxType for Rust {
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
        let soft = (job.number("softness").clamp(0.0, 1.0) * 0.3).max(0.01);
        let edges = job.number("at_edges").clamp(0.0, 1.0);
        let flaking = job.number("flaking").clamp(0.0, 1.0);
        let metal_only = job.switch("metal_only");
        let dark = job.colour("dark").map(|c| c.clamp(0.0, 1.0));
        let light = job.colour("light").map(|c| c.clamp(0.0, 1.0));
        let size = job.length_px("patch_size");
        let blotches = patches(job, size, 5, seeded(job, 1));
        let grain = patches(job, size * 0.15, 3, seeded(job, 2));
        let cover = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            let lie = follow.at(x, y);
            let start = lie.edge.max(lie.crease).max(lie.enclosed);
            let reach = blotches.at(i, j) * (1.0 - 0.5 * edges) + start * edges * 0.8;
            let on_metal = if metal_only {
                smooth(0.2, 0.6, m.metal.at(i, j))
            } else {
                1.0
            };
            covered(reach, amount, soft) * on_metal
        });
        let cover = limited(job, cover, job.grey("where"));
        let colour = job.fill_rgb(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            // Fresh orange in the grain, darker deep in a patch.
            let k = (grain.at(i, j) * 1.4 - 0.2 - cover.at(i, j) * 0.3).clamp(0.0, 1.0);
            toward(dark, light, k)
        });
        let raise = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            flaking * FLAKES * cover.at(i, j) * (0.4 + grain.at(i, j))
        });
        let coat = Coat {
            cover: &cover,
            colour: &colour,
            roughness: Toward::All(job.number("roughness")),
            metal: Some(Toward::All(0.0)),
            raise: (flaking > 0.0).then_some(raise),
        };
        Ok(outputs(coated(job, m, &coat), cover))
    }
}
