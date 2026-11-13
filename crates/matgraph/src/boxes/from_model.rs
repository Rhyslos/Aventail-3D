//! The **From the model** boxes (Phase 12, milestone 5,
//! [decision 0261](../../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! Curvature, Occlusion, Position, Thickness and Facing up. Each brings
//! one of the model's baked maps into the graph, so a graph used in the
//! painter follows the model it is painted on: moss in the crevices,
//! lighter edges, dust on what faces up.
//!
//! In the painter they give the model's maps ([`crate::ModelMaps`]). In
//! the Material mode there is no model, so they give a **stand-in**: a
//! field of rounded bumps, worked out the same way, so a graph can be
//! built and seen before it is painted with. The stand-in repeats four
//! times across the texture and tiles seamlessly.

use super::info;
use crate::kind::ValueKind;
use crate::model::ModelMap;
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, PlugSpec};
use crate::value::Value;
use std::f32::consts::TAU;
use std::sync::Arc;
use words::text;

/// The ids of the From the model boxes, saved in files.
pub const CURVATURE: &str = "model_curvature";
/// Occlusion's id.
pub const OCCLUSION: &str = "model_occlusion";
/// Position's id.
pub const POSITION: &str = "model_position";
/// Thickness's id.
pub const THICKNESS: &str = "model_thickness";
/// Facing up's id.
pub const FACING_UP: &str = "model_facing_up";

/// The model's maps a From the model box of kind `kind` reads (none for
/// other kinds).
pub fn maps_of(kind: &str) -> &'static [ModelMap] {
    match kind {
        CURVATURE => &[ModelMap::Curvature],
        OCCLUSION => &[ModelMap::Occlusion],
        THICKNESS => &[ModelMap::Thickness],
        FACING_UP => &[ModelMap::FacingUp],
        POSITION => &[ModelMap::Up, ModelMap::Across, ModelMap::FrontToBack],
        _ => &[],
    }
}

/// How many bumps the stand-in has across the texture.
const BUMPS: f32 = 4.0;

/// The stand-in model's map `map` at a place on the texture (0 to 1 each
/// way): rounded bumps in a grid, `BUMPS` across, with creases between.
pub(crate) fn stand_in(map: ModelMap, u: f32, v: f32) -> f32 {
    // A smooth bump made from cosines, so it tiles without a step: 1 on
    // top, 0 in the creases.
    let bump = 0.25 * (1.0 + (u * BUMPS * TAU).cos()) * (1.0 + (v * BUMPS * TAU).cos());
    let bump = bump.clamp(0.0, 1.0);
    match map {
        // Edges (the shoulders of the bumps) light, creases dark.
        ModelMap::Curvature => {
            let crease = (1.0 - bump).powi(4);
            let shoulder = (bump * (1.0 - bump) * 4.0).powi(2);
            (0.5 + 0.35 * shoulder - 0.45 * crease).clamp(0.0, 1.0)
        }
        ModelMap::Occlusion => (0.35 + 0.65 * bump.sqrt()).clamp(0.0, 1.0),
        ModelMap::Thickness => (0.25 + 0.5 * (1.0 - bump)).clamp(0.0, 1.0),
        ModelMap::FacingUp => bump,
        // Light at the top and bottom edges, dark across the middle: a
        // smooth stand-in that tiles.
        ModelMap::Up => 0.5 + 0.5 * (v * TAU).cos(),
        ModelMap::Across => 0.5 - 0.5 * (u * TAU).cos(),
        ModelMap::FrontToBack => bump,
    }
}

/// The grey picture of `map`: the model's in the painter, else the
/// stand-in.
pub(super) fn map_value(job: &Job<'_>, map: ModelMap) -> Value {
    if let Some(image) = job.model_map(map) {
        return Value::Height(image.clone());
    }
    let n = job.size().get() as f32;
    let image = job.fill_grey(|x, y| stand_in(map, (x as f32 + 0.5) / n, (y as f32 + 0.5) / n));
    Value::Height(Arc::new(image))
}

/// A From the model box.
#[derive(Debug)]
pub(crate) struct FromModel {
    info: BoxInfo,
}

/// A height output.
fn height(key: &'static str, name: &'static str) -> PlugSpec {
    PlugSpec::new(key, name, ValueKind::Height, false)
}

/// A shape output.
fn shape(key: &'static str, name: &'static str) -> PlugSpec {
    PlugSpec::new(key, name, ValueKind::Shape, false)
}

/// What usually comes after a map from the model.
const AFTER: &[&str] = &["levels", "mix_materials", "blend", "gradient_map"];

impl FromModel {
    pub(crate) fn curvature() -> Self {
        Self {
            info: info(
                CURVATURE,
                text!("Curvature"),
                Group::Model,
                text!(
                    "The model's edges and creases. Curvature is light on outer edges, dark in \
                     creases and middle grey where the surface is flat; Edges and Creases give \
                     each on its own, white where they are. Use Edges to lighten or wear the \
                     edges, Creases to fill them with dirt or moss. In the Material mode it shows \
                     a stand-in of rounded bumps; in the painter it follows the model."
                ),
                (
                    Vec::new(),
                    vec![
                        height("curvature", text!("Curvature")),
                        shape("edges", text!("Edges")),
                        shape("creases", text!("Creases")),
                    ],
                ),
                Vec::new(),
                AFTER,
            ),
        }
    }

    pub(crate) fn occlusion() -> Self {
        Self {
            info: info(
                OCCLUSION,
                text!("Occlusion"),
                Group::Model,
                text!(
                    "How open the model's surface is to the sky: white where it is open, darker \
                     in crevices and corners that light hardly reaches. Use it for dirt, moss or \
                     shade gathering where the model is enclosed. In the Material mode it shows \
                     a stand-in of rounded bumps; in the painter it follows the model."
                ),
                (Vec::new(), vec![height("occlusion", text!("Occlusion"))]),
                Vec::new(),
                AFTER,
            ),
        }
    }

    pub(crate) fn position() -> Self {
        Self {
            info: info(
                POSITION,
                text!("Position"),
                Group::Model,
                text!(
                    "Where on the model each part is: Up goes from black at its bottom to white \
                     at its top, Across from left to right, Front to back from front to back. \
                     Use Up for dirt rising from the ground or snow settling on top. In the \
                     Material mode it shows a smooth stand-in; in the painter it follows the \
                     model."
                ),
                (
                    Vec::new(),
                    vec![
                        height("up", text!("Up")),
                        height("across", text!("Across")),
                        height("front_to_back", text!("Front to back")),
                    ],
                ),
                Vec::new(),
                AFTER,
            ),
        }
    }

    pub(crate) fn thickness() -> Self {
        Self {
            info: info(
                THICKNESS,
                text!("Thickness"),
                Group::Model,
                text!(
                    "How thin the model is: white where it is thin (leaves, ears, fins), dark \
                     where it is thick. Use it where light would shine through, or to wear thin \
                     parts. In the Material mode it shows a stand-in of rounded bumps; in the \
                     painter it follows the model."
                ),
                (Vec::new(), vec![height("thickness", text!("Thickness"))]),
                Vec::new(),
                AFTER,
            ),
        }
    }

    pub(crate) fn facing_up() -> Self {
        Self {
            info: info(
                FACING_UP,
                text!("Facing up"),
                Group::Model,
                text!(
                    "How much the model's surface faces up: white on top, black on the sides and \
                     underneath. Use it for dust, snow or moss settling on top. In the Material \
                     mode it shows a stand-in of rounded bumps; in the painter it follows the \
                     model."
                ),
                (Vec::new(), vec![height("up", text!("Facing up"))]),
                Vec::new(),
                AFTER,
            ),
        }
    }
}

impl BoxType for FromModel {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn model_maps(&self) -> &'static [ModelMap] {
        maps_of(&self.info.id)
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let maps = maps_of(&self.info.id);
        let mut out: Vec<Value> = maps.iter().map(|m| map_value(job, *m)).collect();
        if self.info.id == CURVATURE {
            // Edges and creases from the curvature: how far above or below
            // flat, made 0 to 1.
            let curvature = out[0]
                .grey()
                .cloned()
                .unwrap_or_else(|| Arc::new(crate::image::Image::filled(job.size(), 0.5)));
            let edges = job.map_grey(&curvature, |c| ((c - 0.5) * 2.0).clamp(0.0, 1.0));
            let creases = job.map_grey(&curvature, |c| ((0.5 - c) * 2.0).clamp(0.0, 1.0));
            out.push(Value::Shape(Arc::new(edges)));
            out.push(Value::Shape(Arc::new(creases)));
        }
        Ok(out)
    }
}
