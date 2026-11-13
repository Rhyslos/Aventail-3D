//! Opacity (Phase 12 milestone 14, decision 0276): a whole material made
//! see-through outside a shape, for alpha cards (leaves, grass, fences,
//! chains), with the light that passes through it from behind.
//!
//! The shape's greys become the material's opacity (white solid, black
//! see-through). Its translucency is how much light a thin part lets
//! through: Translucency where the card is solid, less where it is thick
//! (the Thickness input, or the material's own height), none where it is
//! cut away.
//!
//! The colour under the see-through parts is filled from the nearest
//! solid colour (the colour "bleeds" outwards). Games read textures
//! blurred a little and from smaller copies far away, and the colour
//! there leaks into the card's edge: black would leave a dark fringe
//! round every leaf. The fill is a pull-push: the solid colours averaged
//! into ever smaller copies, then laid back from the smallest up under
//! every pixel that has none, so it reaches any distance in a few passes.

use super::{info, number};
use crate::image::{Grey, Image, Rgb, Size};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// The Opacity box's kind id.
pub const OPACITY: &str = "opacity";

/// Where a pixel counts as solid for the colour fill.
const SOLID: f32 = 0.99;

/// The Opacity box.
#[derive(Debug)]
pub(crate) struct Opacity {
    info: BoxInfo,
}

impl Opacity {
    pub(crate) fn new() -> Self {
        Self {
            info: info(
                OPACITY,
                text!("Opacity"),
                Group::Material,
                text!(
                    "Makes a material see-through outside a shape, for alpha cards: leaves, \
                     grass, fences, chains. Plug the Shape box's Mask into Opacity: white stays \
                     solid, black is cut away. Translucency is the light passing through a thin \
                     part from behind, as through a leaf held up to the sun; thicker parts (the \
                     Thickness, or the material's own height) let less through. The colour \
                     under the cut-away parts is filled from the nearest solid colour, so no \
                     dark fringe shows round the edges in a game."
                ),
                (
                    vec![
                        PlugSpec::new("material", text!("Material"), ValueKind::Material, true),
                        PlugSpec::new("shape", text!("Opacity"), ValueKind::Shape, true),
                        PlugSpec::new("thickness", text!("Thickness"), ValueKind::Height, false),
                    ],
                    vec![PlugSpec::new(
                        "material_out",
                        text!("Material"),
                        ValueKind::Material,
                        false,
                    )],
                ),
                vec![
                    number(
                        "translucency",
                        text!("Translucency"),
                        text!(
                            "How much light passes through the thinnest parts from behind: 0 \
                             none (no translucency map), 1 most."
                        ),
                        0.0,
                        1.0,
                        0.5,
                    ),
                    number(
                        "thick",
                        text!("Thick parts block light"),
                        text!(
                            "How much less light passes where the material is thick (high in the \
                             Thickness, or in its own height): veins and stems stay darker."
                        ),
                        0.0,
                        1.0,
                        0.6,
                    ),
                ],
                &["material_out", "mix_materials"],
            ),
        }
    }
}

impl BoxType for Opacity {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let m = job.needed_material("material")?;
        let shape = job.needed_grey("shape")?;
        let opacity = Arc::new(job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            (shape.at(x, y).clamp(0.0, 1.0) * m.opacity_at(x, y)).clamp(0.0, 1.0)
        }));
        let amount = job.number("translucency").clamp(0.0, 1.0);
        let thick = job.number("thick").clamp(0.0, 1.0);
        let thickness = job.grey("thickness").unwrap_or(&m.height);
        let translucency = (amount > 0.0).then(|| {
            Arc::new(job.fill_grey(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let h = thickness.at(x, y).clamp(0.0, 1.0);
                (amount * opacity.at(x, y) * (1.0 - thick * h)).clamp(0.0, 1.0)
            }))
        });
        let colour = Arc::new(bled(&m.colour, &opacity, job.size()));
        Ok(vec![Value::Material(Arc::new(MaterialMaps {
            colour,
            opacity: Some(opacity),
            translucency,
            ..(**m).clone()
        }))])
    }
}

/// `colour` with every pixel that isn't solid in `opacity` filled from
/// the nearest solid colours (pull-push), solid pixels unchanged. With no
/// solid pixel at all, the partly solid ones are the source; with none
/// of those either, it is left as it is.
pub(crate) fn bled(colour: &Rgb, opacity: &Grey, size: Size) -> Rgb {
    let n = size.get() as usize;
    let solid = opacity.pixels().iter().any(|&o| o >= SOLID);
    let weight = |o: f32| {
        if solid {
            if o >= SOLID { 1.0 } else { 0.0 }
        } else {
            o.clamp(0.0, 1.0)
        }
    };
    // Level 0: colour times weight, and the weight.
    let mut sums: Vec<Vec<[f32; 4]>> = vec![
        colour
            .pixels()
            .iter()
            .zip(opacity.pixels())
            .map(|(c, o)| {
                let w = weight(*o);
                [c[0] * w, c[1] * w, c[2] * w, w]
            })
            .collect(),
    ];
    if sums[0].iter().all(|s| s[3] <= 0.0) {
        return colour.clone();
    }
    // Pull: each smaller copy sums its four pixels.
    let mut side = n;
    while side > 1 {
        let half = side / 2;
        let last = &sums[sums.len() - 1];
        let mut next = vec![[0.0f32; 4]; half * half];
        for y in 0..half {
            for x in 0..half {
                let mut s = [0.0f32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = last[(2 * y + dy) * side + 2 * x + dx];
                    for k in 0..4 {
                        s[k] += p[k];
                    }
                }
                next[y * half + x] = s;
            }
        }
        sums.push(next);
        side = half;
    }
    // Push: from the smallest up, each pixel's own average where it has
    // weight, blended over its parent's fill where it has little.
    let average = |s: [f32; 4]| [s[0] / s[3], s[1] / s[3], s[2] / s[3]];
    let mut fill: Vec<[f32; 3]> = vec![
        sums[sums.len() - 1]
            .first()
            .map_or([0.0; 3], |s| average(*s)),
    ];
    for level in (0..sums.len() - 1).rev() {
        let side = n >> level;
        let parent = side / 2;
        let here = &sums[level];
        fill = (0..side * side)
            .map(|i| {
                let (x, y) = (i % side, i / side);
                let up = fill[(y / 2) * parent + x / 2];
                let s = here[i];
                let a = s[3].min(1.0);
                if s[3] <= 0.0 {
                    up
                } else {
                    let own = average(s);
                    std::array::from_fn(|k| own[k] * a + up[k] * (1.0 - a))
                }
            })
            .collect();
    }
    let pixels = colour
        .pixels()
        .iter()
        .zip(opacity.pixels())
        .zip(fill)
        .map(|((c, o), f)| if weight(*o) >= 1.0 { *c } else { f })
        .collect();
    Image::from_pixels(size, pixels).unwrap_or_else(|| colour.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use crate::{BoxId, Evaluator, Graph, GraphEdit, PlugRef};
    use glam::Vec2;

    /// A green disc on black, as the Shape box's colours give it.
    fn disc(size: Size) -> (Rgb, Grey) {
        let n = size.get() as f32;
        let cover = |x: u32, y: u32| {
            let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5).distance(Vec2::splat(n / 2.0));
            (n * 0.3 - d + 0.5).clamp(0.0, 1.0)
        };
        let mask = crate::parallel::fill(size, 1, cover);
        let colour = crate::parallel::fill(size, 1, |x, y| {
            let c = cover(x, y);
            [0.2 * c, 0.6 * c, 0.1 * c]
        });
        (colour, mask)
    }

    #[test]
    fn the_colour_bleeds_out_from_the_solid_parts_and_stays_inside() {
        let size = Size::nearest(64);
        let (colour, mask) = disc(size);
        let out = bled(&colour, &mask, size);
        // Inside: unchanged.
        assert_eq!(out.at(32, 32), colour.at(32, 32));
        // Far outside and on the soft edge: the leaf's green, not black.
        for (x, y) in [(0, 0), (63, 2), (5, 40)] {
            let c = out.at(x, y);
            assert!(
                (c[1] - 0.6).abs() < 0.05 && (c[0] - 0.2).abs() < 0.05,
                "{c:?}"
            );
        }
        let edge = (0..64)
            .find(|x| mask.at(*x, 32) > 0.0 && mask.at(*x, 32) < SOLID)
            .unwrap();
        assert!((out.at(edge, 32)[1] - 0.6).abs() < 0.05);
        assert!(out.pixels().iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn nothing_solid_leaves_the_colour_as_it_was() {
        let size = Size::nearest(16);
        let colour = Image::filled(size, [0.3, 0.2, 0.1]);
        let none = Image::filled(size, 0.0);
        assert_eq!(bled(&colour, &none, size), colour);
        // Only partly solid: those are the source.
        let faint = Image::filled(size, 0.5);
        assert_eq!(bled(&colour, &faint, size).at(3, 3), [0.3, 0.2, 0.1]);
    }

    fn card_graph(translucency: f32) -> (Graph, BoxId) {
        let registry = Registry::standard();
        let mut g = Graph::new("Card");
        for kind in ["basic_shape", "material_from_height", OPACITY] {
            g.apply(
                registry,
                &GraphEdit::AddBox {
                    kind: kind.into(),
                    at: Vec2::ZERO,
                },
            )
            .unwrap();
        }
        let (shape, from_height, cut) = (BoxId(1), BoxId(2), BoxId(3));
        for (from, out, to, input) in [
            (shape, "shape", from_height, "height"),
            (from_height, "material", cut, "material"),
            (shape, "shape", cut, "shape"),
        ] {
            g.apply(
                registry,
                &GraphEdit::Connect {
                    from: PlugRef::new(from, out),
                    to: PlugRef::new(to, input),
                },
            )
            .unwrap();
        }
        g.apply(
            registry,
            &GraphEdit::SetSetting {
                node: cut,
                key: "translucency".into(),
                value: crate::SettingValue::Number(translucency),
            },
        )
        .unwrap();
        (g, cut)
    }

    fn card(translucency: f32) -> Arc<MaterialMaps> {
        let (g, cut) = card_graph(translucency);
        let mut evaluator = Evaluator::with_threads(2);
        let run = evaluator.run(Registry::standard(), &g, Size::nearest(64), &[cut]);
        let out = run.outcome(cut).unwrap().as_ref().unwrap();
        out.outputs[0].clone().unwrap().material().unwrap().clone()
    }

    #[test]
    fn the_card_is_solid_inside_the_shape_and_see_through_outside() {
        let m = card(0.5);
        let opacity = m.opacity.as_ref().unwrap();
        assert_eq!(opacity.at(32, 32), 1.0);
        assert_eq!(opacity.at(0, 0), 0.0);
        assert!(opacity.pixels().iter().all(|v| (0.0..=1.0).contains(v)));
        // Light passes through the solid part, less where it is high, none
        // where it is cut away.
        let t = m.translucency.as_ref().unwrap();
        assert_eq!(t.at(0, 0), 0.0);
        assert!(t.at(32, 32) > 0.0 && t.at(32, 32) <= 0.5);
        assert!(t.pixels().iter().all(|v| (0.0..=0.5 + 1e-6).contains(v)));
        // The rest of the material is kept.
        assert_eq!(m.size(), 64);
    }

    #[test]
    fn no_translucency_gives_no_translucency_map() {
        let m = card(0.0);
        assert!(m.opacity.is_some());
        assert!(m.translucency.is_none());
    }
}
