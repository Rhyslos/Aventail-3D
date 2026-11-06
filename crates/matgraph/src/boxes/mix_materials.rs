//! Mix materials: lays one whole material over another, through a shape
//! or where it stands higher, mixing every map at once.

use super::{choice, info, number};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Mix materials box.
#[derive(Debug)]
pub(crate) struct MixMaterials {
    info: BoxInfo,
}

/// The ways to mix, in the By setting's order.
const BY_SHAPE: u32 = 0;
const BY_HEIGHT: u32 = 1;

impl MixMaterials {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                "mix_materials",
                text!("Mix materials"),
                Group::Material,
                text!(
                    "Lays one whole material (Top) over another (Bottom), mixing every map at \
                     once: colour, height, roughness and the rest. By shape: Top shows where the \
                     shape in Where is white (everywhere, with nothing there). By height: Top \
                     shows where it stands higher than Bottom, as moss fills cracks or snow sits \
                     on stones; Rise lifts it. Amount sets how much of Top shows."
                ),
                (
                    vec![
                        PlugSpec::new("bottom", text!("Bottom"), ValueKind::Material, true),
                        PlugSpec::new("top", text!("Top"), ValueKind::Material, true),
                        PlugSpec::new("where", text!("Where"), ValueKind::Shape, false),
                    ],
                    vec![PlugSpec::new(
                        "material",
                        text!("Material"),
                        ValueKind::Material,
                        false,
                    )],
                ),
                vec![
                    choice(
                        "by",
                        text!("By"),
                        text!("What decides where Top shows."),
                        &[text!("Shape"), text!("Height")],
                        BY_SHAPE,
                    ),
                    number(
                        "amount",
                        text!("Amount"),
                        text!("How much of Top shows."),
                        0.0,
                        1.0,
                        1.0,
                    ),
                    number(
                        "rise",
                        text!("Rise"),
                        text!("By height: how much higher Top sits, so more or less of it shows."),
                        -1.0,
                        1.0,
                        0.0,
                    ),
                    number(
                        "softness",
                        text!("Softness"),
                        text!("By height: how gently Top fades in where the two meet."),
                        0.0,
                        1.0,
                        0.1,
                    ),
                ],
                &["material_out", "mix_materials"],
            ),
        }
    }
}

impl BoxType for MixMaterials {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let bottom = job.needed_material("bottom")?;
        let top = job.needed_material("top")?;
        let mask = job.grey("where");
        let by = job.choice("by");
        let amount = job.number("amount").clamp(0.0, 1.0);
        let rise = job.number("rise");
        let soft = job.number("softness").clamp(0.0, 1.0).max(1e-3) * 0.5;
        let share: Grey = job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            let shape = mask.map_or(1.0, |m| m.at(x, y).clamp(0.0, 1.0));
            let by_height = if by == BY_HEIGHT {
                let above = top.height.at(x, y) + rise - bottom.height.at(x, y);
                (0.5 + above / (2.0 * soft)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            shape * by_height * amount
        });
        let grey = |a: &Grey, b: &Grey| -> Arc<Grey> {
            Arc::new(job.fill_grey(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let k = share.at(x, y);
                a.at(x, y) + (b.at(x, y) - a.at(x, y)) * k
            }))
        };
        let rgb = |a: &Rgb, b: &Rgb, unit: bool| -> Arc<Rgb> {
            Arc::new(job.fill_rgb(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let k = share.at(x, y);
                let (p, q) = (a.at(x, y), b.at(x, y));
                let mixed: [f32; 3] = std::array::from_fn(|i| p[i] + (q[i] - p[i]) * k);
                if unit {
                    glam::Vec3::from_array(mixed)
                        .normalize_or(glam::Vec3::Z)
                        .to_array()
                } else {
                    mixed
                }
            }))
        };
        let maps = MaterialMaps {
            colour: rgb(&bottom.colour, &top.colour, false),
            height: grey(&bottom.height, &top.height),
            roughness: grey(&bottom.roughness, &top.roughness),
            metal: grey(&bottom.metal, &top.metal),
            normal: rgb(&bottom.normal, &top.normal, true),
            occlusion: grey(&bottom.occlusion, &top.occlusion),
            curvature: grey(&bottom.curvature, &top.curvature),
        };
        Ok(vec![Value::Material(Arc::new(maps))])
    }
}
