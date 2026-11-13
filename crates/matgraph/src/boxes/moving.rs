//! Moving any kind of picture, for Transform and Repeat
//! ([decision 0270](../../../../docs/decisions/0270-tiling-and-moving-colours.md)):
//! a height, a colour or a whole material, each pixel taken from another
//! place on the input (which wraps, so the result still tiles).
//!
//! A material moves every map the same way. Its normal map also turns
//! with it: a normal is a direction on the texture, so a picture turned a
//! quarter turns its normals a quarter, a flip mirrors them, and copies
//! made smaller (Repeat) have steeper slopes. Occlusion and curvature are
//! moved as they are.

use crate::image::{Image, Size};
use crate::kind::ValueKind;
use crate::refusal::Refusal;
use crate::registry::{Job, PlugSpec};
use crate::value::{MaterialMaps, Value};
use std::sync::Arc;
use words::{text, trf};

/// Where an output pixel takes its value from.
#[derive(Debug, Clone, Copy)]
pub(super) struct Source {
    /// Across the input, in whole textures (wrapping).
    pub(super) u: f32,
    /// Down the input, in whole textures (wrapping).
    pub(super) v: f32,
    /// What the value is multiplied by there (a height made lower, a
    /// colour darker): 1 leaves it.
    pub(super) scale: f32,
}

/// The input keys of a box that moves any kind, with the outputs they
/// give, in order: a height, a colour, a material.
pub(super) const INPUTS: [&str; 3] = ["in", "colour", "material"];

/// `value` moved: each output pixel from `at(x, y)`, and for a
/// material, each normal turned by `normal(n, source)` (then made a unit
/// long). A shape comes out as a height (the plug it goes in takes
/// heights).
pub(super) fn moved(
    job: &Job<'_>,
    value: &Value,
    at: &(impl Fn(u32, u32) -> Source + Sync),
    normal: &(impl Fn([f32; 3], &Source) -> [f32; 3] + Sync),
) -> Value {
    let grey = |g: &crate::image::Grey| {
        Arc::new(job.fill_grey(|x, y| {
            let s = at(x, y);
            g.sample(s.u, s.v) * s.scale
        }))
    };
    let plain = |g: &crate::image::Grey| {
        Arc::new(job.fill_grey(|x, y| {
            let s = at(x, y);
            g.sample(s.u, s.v)
        }))
    };
    let rgb = |c: &crate::image::Rgb| {
        Arc::new(job.fill_rgb(|x, y| {
            let s = at(x, y);
            c.sample_rgb(s.u, s.v).map(|v| v * s.scale)
        }))
    };
    match value {
        Value::Shape(g) | Value::Height(g) => Value::Height(grey(g)),
        Value::Colour(c) => Value::Colour(rgb(c)),
        Value::Material(m) => Value::Material(Arc::new(MaterialMaps {
            colour: rgb(&m.colour),
            height: grey(&m.height),
            roughness: plain(&m.roughness),
            metal: plain(&m.metal),
            normal: Arc::new(job.fill_rgb(|x, y| {
                let s = at(x, y);
                let n = normal(m.normal.sample_rgb(s.u, s.v), &s);
                glam::Vec3::from_array(n)
                    .normalize_or(glam::Vec3::Z)
                    .to_array()
            })),
            occlusion: plain(&m.occlusion),
            curvature: plain(&m.curvature),
            opacity: m.opacity.as_ref().map(|g| plain(g)),
            translucency: m.translucency.as_ref().map(|g| plain(g)),
        })),
    }
}

/// What a box moving any kind gives: for each of its inputs ([`INPUTS`])
/// plugged in, that input moved; or, with nothing plugged in, why not.
pub(super) fn moved_all(
    job: &Job<'_>,
    at: &(impl Fn(u32, u32) -> Source + Sync),
    normal: &(impl Fn([f32; 3], &Source) -> [f32; 3] + Sync),
) -> Result<Vec<Option<Value>>, Refusal> {
    let out: Vec<Option<Value>> = INPUTS
        .iter()
        .map(|key| job.input(key).map(|v| moved(job, v, at, normal)))
        .collect();
    if out.iter().all(Option::is_none) {
        // The evaluator says so first, with ways to fix it.
        return Err(Refusal::new(trf!(
            "{} has nothing plugged in.",
            job.box_name()
        )));
    }
    Ok(out)
}

/// `outputs` with the empty ones made plain, for a caller wanting every
/// output ([`crate::BoxType::run`]): no height, black, and a flat grey
/// material.
pub(super) fn filled(outputs: Vec<Option<Value>>, size: Size) -> Vec<Value> {
    let kinds = [ValueKind::Height, ValueKind::Colour, ValueKind::Material];
    outputs
        .into_iter()
        .zip(kinds)
        .map(|(v, kind)| v.unwrap_or_else(|| plain(kind, size)))
        .collect()
}

/// A plain picture of `kind` at `size`.
pub(super) fn plain(kind: ValueKind, size: Size) -> Value {
    let grey = |v: f32| Arc::new(Image::filled(size, v));
    match kind {
        ValueKind::Shape => Value::Shape(grey(0.0)),
        ValueKind::Height => Value::Height(grey(0.0)),
        ValueKind::Colour => Value::Colour(Arc::new(Image::filled(size, [0.0; 3]))),
        ValueKind::Material => Value::Material(Arc::new(MaterialMaps {
            colour: Arc::new(Image::filled(size, [0.5; 3])),
            height: grey(0.5),
            roughness: grey(0.5),
            metal: grey(0.0),
            normal: Arc::new(Image::filled(size, [0.0, 0.0, 1.0])),
            occlusion: grey(1.0),
            curvature: grey(0.5),
            opacity: None,
            translucency: None,
        })),
    }
}

/// The plugs of a box moving any kind: a height, a colour and a material
/// in, each moved out. The height's keys (`in`, `out`) are the ones
/// released when such boxes moved heights only, so old graphs keep their
/// wires; none is needed on its own ([`crate::BoxType::needs_an_input`]).
pub(super) fn plugs() -> (Vec<PlugSpec>, Vec<PlugSpec>) {
    (
        vec![
            PlugSpec::new(INPUTS[0], text!("Height"), ValueKind::Height, false),
            PlugSpec::new(INPUTS[1], text!("Colour"), ValueKind::Colour, false),
            PlugSpec::new(INPUTS[2], text!("Material"), ValueKind::Material, false),
        ],
        vec![
            PlugSpec::new("out", text!("Height"), ValueKind::Height, false),
            PlugSpec::new("colour_out", text!("Colour"), ValueKind::Colour, false),
            PlugSpec::new(
                "material_out",
                text!("Material"),
                ValueKind::Material,
                false,
            ),
        ],
    )
}
