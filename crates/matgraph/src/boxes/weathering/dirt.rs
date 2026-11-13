//! Dirt: grime gathering in a material's crevices and the model's, with a
//! faint film all over (milestone 25, decision 0288).

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

/// The model's maps Dirt reads.
const MAPS: &[ModelMap] = &[ModelMap::Curvature, ModelMap::Occlusion];

/// The Dirt box.
#[derive(Debug)]
pub(crate) struct Dirt {
    info: BoxInfo,
}

impl Dirt {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "dirt",
                text!("Dirt"),
                Group::Weather,
                text!(
                    "Grime gathering in crevices, corners and creases, where rain and hands \
                     don't reach, with a faint film all over. Plug a whole material into \
                     Material. In the painter it follows the model's own crevices (and the \
                     material's gaps, by Follow the material); in the Material mode it shows on \
                     the stand-in of rounded bumps. Dirt gives where the dirt is, to use as a \
                     shape elsewhere; a shape in Only where keeps it there."
                ),
                plugs(text!("Dirt"), Vec::new()),
                vec![
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How far the dirt spreads out of the crevices: 0 is none."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("How gently the dirt fades out at its edges."),
                        0.0,
                        1.0,
                        0.4,
                    ),
                    number(
                        "patchy",
                        text!("Patchy"),
                        text!("How much further the dirt reaches in some places than others."),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    length(
                        "patch_size",
                        text!("Patch size"),
                        text!("How wide the patches are, in centimetres."),
                        0.5,
                        200.0,
                        15.0,
                    ),
                    number(
                        "film",
                        text!("Film all over"),
                        text!("A faint grime over everything, in patches: 0 is none."),
                        0.0,
                        1.0,
                        0.15,
                    ),
                    colour(
                        "dirt_colour",
                        text!("Dirt colour"),
                        text!("The dirt's colour: dark brown for earth, near black for soot."),
                        [0.2, 0.16, 0.11],
                    ),
                    number(
                        "colour_vary",
                        text!("Colour varies"),
                        text!("How much lighter or darker the dirt is in patches."),
                        0.0,
                        1.0,
                        0.3,
                    ),
                    number(
                        "roughness",
                        text!("Dirt roughness"),
                        text!("How rough the dirt is: 1 is fully matt."),
                        0.0,
                        1.0,
                        0.85,
                    ),
                    follow_model(),
                    follow_material(),
                    seed(),
                ],
                &["water_streaks", "dust", "material_out"],
            ),
        }
    }
}

impl BoxType for Dirt {
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
        let patchy = job.number("patchy").clamp(0.0, 1.0);
        let film = job.number("film").clamp(0.0, 1.0);
        let vary = job.number("colour_vary").clamp(0.0, 1.0);
        let dirt = job.colour("dirt_colour").map(|c| c.clamp(0.0, 1.0));
        let size = job.length_px("patch_size");
        let big = patches(job, size, 4, seeded(job, 1));
        let fine = patches(job, size * 0.25, 3, seeded(job, 2));
        let cover = job.fill_grey(|x, y| {
            let (i, j) = (i64::from(x), i64::from(y));
            let lie = follow.at(x, y);
            let crevice = (lie.crease * 0.8).max(lie.enclosed * 1.2).min(1.0);
            let patch = big.at(i, j) - 0.5;
            let deep = covered(crevice + patch * patchy * 0.8, amount, soft);
            let thin = film * (big.at(i, j) * 0.6 + fine.at(i, j) * 0.4);
            1.0 - (1.0 - deep) * (1.0 - thin.clamp(0.0, 1.0))
        });
        let cover = limited(job, cover, job.grey("where"));
        let colour = job.fill_rgb(|x, y| {
            let light = 1.0 + (fine.at(i64::from(x), i64::from(y)) - 0.5) * vary;
            dirt.map(|c| (c * light).clamp(0.0, 1.0))
        });
        let coat = Coat {
            cover: &cover,
            colour: &colour,
            roughness: Toward::All(job.number("roughness")),
            // Grime over bare metal isn't metal.
            metal: Some(Toward::All(0.0)),
            raise: None,
        };
        Ok(outputs(coated(job, m, &coat), cover))
    }
}
