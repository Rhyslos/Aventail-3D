//! The **weathering set** (Phase 12, milestone 25,
//! [decision 0288](../../../../docs/decisions/0288-weathering-set.md)):
//! Dirt, Rust, Edge wear, Dust and Water streaks, listed together as
//! **Weathering**. Each takes a whole material and gives it back aged, with
//! where the weathering lies as a shape.
//!
//! They **follow the model**: in the painter each reads the model's own
//! baked maps ([`crate::BoxType::model_maps`]), so dirt gathers in the
//! model's crevices, paint wears off its edges and dust settles on what
//! faces up. In the Material mode there is no model, so they read the From
//! the model boxes' stand-in of rounded bumps, as those boxes show. They
//! also follow the **material's own** bumps and gaps (its curvature,
//! occlusion and normals), so grout lines and plank edges weather too.
//! Two sliders weigh the two: **Follow the model** and **Follow the
//! material**; a texture made for no model sets the first to 0.
//!
//! Everything reads through wrapping pictures and whole-repeat noise, so
//! with no model the result tiles seamlessly.

mod dirt;
mod dust;
mod edge_wear;
mod rust;
mod streaks;
#[cfg(test)]
mod tests;

pub(super) use dirt::Dirt;
pub(super) use dust::Dust;
pub(super) use edge_wear::EdgeWear;
pub(super) use rust::Rust;
pub(super) use streaks::WaterStreaks;

use super::material_from_height::normal;
use super::noise::layered;
use super::pattern::{smooth, whole_repeats};
use super::{from_model, number, uv};
use crate::image::{Grey, Rgb};
use crate::kind::ValueKind;
use crate::model::ModelMap;
use crate::registry::{Job, PlugSpec};
use crate::seed::mix;
use crate::settings::SettingSpec;
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::text;

/// How deep a raise of 1 bends the normals, in hundredths of the
/// texture's width (as Stamps presses its pictures).
const DEPTH: f32 = 2.0;

/// The most noise squares across, for patches.
const MOST_PATCHES: i64 = 256;

/// The Follow the model setting.
fn follow_model() -> SettingSpec {
    number(
        "model",
        text!("Follow the model"),
        text!(
            "How much the model's own shape counts: its crevices, edges or tops in the painter. \
             In the Material mode the stand-in of rounded bumps shows instead; set it to 0 for \
             a texture used without a model."
        ),
        0.0,
        1.0,
        1.0,
    )
}

/// The Follow the material setting.
fn follow_material() -> SettingSpec {
    number(
        "own",
        text!("Follow the material"),
        text!(
            "How much the material's own bumps and gaps count as well: the gaps between \
             stones, the edges of planks."
        ),
        0.0,
        1.0,
        0.5,
    )
}

/// A weathering box's plugs: Material (needed) and Only where in, then
/// `more` inputs; Material and the weathering alone (`cover`, a shape)
/// out.
fn plugs(cover: &'static str, more: Vec<PlugSpec>) -> (Vec<PlugSpec>, Vec<PlugSpec>) {
    let mut inputs = vec![
        PlugSpec::new("material", text!("Material"), ValueKind::Material, true),
        PlugSpec::new("where", text!("Only where"), ValueKind::Shape, false),
    ];
    inputs.extend(more);
    (
        inputs,
        vec![
            PlugSpec::new(
                "material_out",
                text!("Material"),
                ValueKind::Material,
                false,
            ),
            PlugSpec::new("mask", cover, ValueKind::Shape, false),
        ],
    )
}

/// What a place on the model and the material is like, each 0 to 1: how
/// much it is an outer edge, a crease, enclosed (out of the light), and
/// facing up.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Lie {
    pub edge: f32,
    pub crease: f32,
    pub enclosed: f32,
    pub up: f32,
}

impl Lie {
    /// From a curvature (0.5 flat), an occlusion (1 open) and how much it
    /// faces up.
    fn of(curvature: f32, occlusion: f32, up: f32) -> Self {
        Self {
            edge: ((curvature - 0.5) * 2.0).clamp(0.0, 1.0),
            crease: ((0.5 - curvature) * 2.0).clamp(0.0, 1.0),
            enclosed: (1.0 - occlusion).clamp(0.0, 1.0),
            up: up.clamp(0.0, 1.0),
        }
    }

    /// Each part the stronger of `self` times `a` and `other` times `b`.
    fn either(self, a: f32, other: Self, b: f32) -> Self {
        Self {
            edge: (self.edge * a).max(other.edge * b),
            crease: (self.crease * a).max(other.crease * b),
            enclosed: (self.enclosed * a).max(other.enclosed * b),
            up: (self.up * a).max(other.up * b),
        }
    }
}

/// What a weathering box follows: the model's maps it reads (the model's
/// in the painter, else the stand-in) and the material's own, weighed by
/// Follow the model and Follow the material.
pub(super) struct Follow<'a> {
    curvature: Option<Arc<Grey>>,
    occlusion: Option<Arc<Grey>>,
    facing_up: Option<Arc<Grey>>,
    material: &'a MaterialMaps,
    model: f32,
    own: f32,
}

impl<'a> Follow<'a> {
    /// For `material`, reading the model's maps `maps`.
    pub(super) fn new(job: &Job<'_>, material: &'a MaterialMaps, maps: &[ModelMap]) -> Self {
        let model = job.number("model").clamp(0.0, 1.0);
        let own = job.number("own").clamp(0.0, 1.0);
        let read = |map: ModelMap| {
            (model > 0.0 && maps.contains(&map))
                .then(|| from_model::map_value(job, map).grey().cloned())
                .flatten()
        };
        Self {
            curvature: read(ModelMap::Curvature),
            occlusion: read(ModelMap::Occlusion),
            facing_up: read(ModelMap::FacingUp),
            material,
            model,
            own,
        }
    }

    /// What pixel (`x`, `y`) is like, the model's and the material's own
    /// weighed together.
    pub(super) fn at(&self, x: u32, y: u32) -> Lie {
        let (x, y) = (i64::from(x), i64::from(y));
        let read = |g: &Option<Arc<Grey>>, neutral: f32| g.as_ref().map_or(neutral, |g| g.at(x, y));
        let model = Lie::of(
            read(&self.curvature, 0.5),
            read(&self.occlusion, 1.0),
            read(&self.facing_up, 0.0),
        );
        let m = self.material;
        // A material on its own lies flat, like a floor: its flat parts
        // face up, its slopes less.
        let facing = smooth(0.6, 1.0, m.normal.at(x, y)[2]);
        let own = Lie::of(m.curvature.at(x, y), m.occlusion.at(x, y), facing);
        model.either(self.model, own, self.own)
    }
}

/// Soft patches from 0 to 1 (about 0.5 on average), `size` pixels across,
/// tiling: weathering is heavier in some places than others.
pub(super) fn patches(job: &Job<'_>, size_px: f32, layers: u32, seed: u64) -> Grey {
    let n = job.size().get() as f32;
    let across = whole_repeats(n, size_px, MOST_PATCHES) as u32;
    job.fill_grey(|x, y| {
        let (u, v) = uv(x, y, n);
        layered(u, v, across, layers, 0.55, seed)
    })
}

/// How much is covered where something reaches `reach` (0 to about 1)
/// for an Amount of `amount`: none at Amount 0, a smooth edge `soft`
/// wide round the level it reaches.
pub(super) fn covered(reach: f32, amount: f32, soft: f32) -> f32 {
    let level = 1.0 - amount;
    smooth(level - soft, level + soft, reach) * smooth(0.0, 0.02, amount)
}

/// The seed for the `k`th pattern of a box.
pub(super) fn seeded(job: &Job<'_>, k: u64) -> u64 {
    mix(job.seed(), k)
}

/// `a` to `b` by `t`, colour by colour.
pub(super) fn toward(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| (a[i] + (b[i] - a[i]) * t).clamp(0.0, 1.0))
}

/// What a coat takes a map toward: one value, or a picture of them.
#[derive(Clone, Copy)]
pub(super) enum Toward<'a> {
    All(f32),
    Each(&'a Grey),
}

impl Toward<'_> {
    fn at(self, x: i64, y: i64) -> f32 {
        match self {
            Self::All(v) => v,
            Self::Each(g) => g.at(x, y),
        }
        .clamp(0.0, 1.0)
    }
}

/// What a weathering coat lays on a material.
pub(super) struct Coat<'a> {
    /// How much of it covers each place, 0 to 1.
    pub cover: &'a Grey,
    /// Its colour at each place.
    pub colour: &'a Rgb,
    /// How rough it is.
    pub roughness: Toward<'a>,
    /// How metal it is, if it changes the metal.
    pub metal: Option<Toward<'a>>,
    /// How much it raises (or, below 0, lowers) each place, if at all;
    /// the normals bend by it.
    pub raise: Option<Grey>,
}

/// Material `m` with `coat` laid on: its colour, roughness and metal by
/// how much covers each place, its height raised and normals bent; its
/// occlusion, curvature, opacity and light through kept.
pub(super) fn coated(job: &Job<'_>, m: &MaterialMaps, coat: &Coat<'_>) -> MaterialMaps {
    let at = |g: &Grey, x: u32, y: u32| g.at(i64::from(x), i64::from(y));
    let colour = job.fill_rgb(|x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        toward(m.colour.at(x, y), coat.colour.at(x, y), coat.cover.at(x, y))
    });
    let blend = |g: &Grey, to: Toward<'_>| {
        Arc::new(job.fill_grey(|x, y| {
            let v = at(g, x, y);
            let to = to.at(i64::from(x), i64::from(y));
            (v + (to - v) * at(coat.cover, x, y)).clamp(0.0, 1.0)
        }))
    };
    let roughness = blend(&m.roughness, coat.roughness);
    let metal = coat
        .metal
        .map_or_else(|| m.metal.clone(), |to| blend(&m.metal, to));
    let (height, normal) = match &coat.raise {
        Some(raise) => {
            let height =
                job.fill_grey(|x, y| (at(&m.height, x, y) + at(raise, x, y)).clamp(0.0, 1.0));
            let bent = normal(job, raise, DEPTH);
            let normals = job.fill_rgb(|x, y| {
                let (x, y) = (i64::from(x), i64::from(y));
                let [bx, by, bz] = m.normal.at(x, y);
                let [sx, sy, sz] = bent.at(x, y);
                glam::Vec3::new(bx + sx, by + sy, bz * sz)
                    .normalize_or(glam::Vec3::Z)
                    .to_array()
            });
            (Arc::new(height), Arc::new(normals))
        }
        None => (m.height.clone(), m.normal.clone()),
    };
    MaterialMaps {
        colour: Arc::new(colour),
        height,
        roughness,
        metal,
        normal,
        ..m.clone()
    }
}

/// A box's two outputs: the weathered material and where the weathering
/// is.
pub(super) fn outputs(material: MaterialMaps, cover: Grey) -> Vec<Value> {
    vec![
        Value::Material(Arc::new(material)),
        Value::Shape(Arc::new(cover)),
    ]
}

/// `cover` limited to where shape `only` is white, if one is plugged in.
pub(super) fn limited(job: &Job<'_>, cover: Grey, only: Option<&Arc<Grey>>) -> Grey {
    match only {
        Some(only) => job.fill_grey(|x, y| {
            let (x, y) = (i64::from(x), i64::from(y));
            cover.at(x, y) * only.at(x, y).clamp(0.0, 1.0)
        }),
        None => cover,
    }
}
