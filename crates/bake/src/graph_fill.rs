//! Graph fills' pixels (Phase 12, milestone 5,
//! [decision 0261](../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! a material graph worked out on the model it is painted on.
//!
//! Two steps, both off the interface's thread:
//!
//! 1. [`model_maps`] bakes the maps the graph's From the model boxes read
//!    (curvature, occlusion, thickness, facing up, position), the same way
//!    the baker and the smart masks do, as `matgraph` greys over the
//!    whole texture square. Texels no polygon covers take their nearest
//!    covered neighbour's value a few texels out (so a blur or warp near a
//!    UV piece's edge doesn't pull in nonsense), then a neutral value.
//! 2. [`graph_fill`] works the graph out on them
//!    (`matgraph::Evaluator::material_on_model`) and turns its material
//!    into the layer's pixels: colour, and roughness, metal and height as
//!    greys for the realism channels. Like a material layer, only texels
//!    the polygons cover are painted; the rest stay see-through, so
//!    padding works as for painted layers.

use crate::curvature::Edges;
use crate::rays::Rays;
use crate::{BakeSettings, Progress, Source, bounds, texels};
use glam::Vec3;
use matgraph::{
    Evaluator, Graph, Grey, MaterialMaps, ModelMap, ModelMaps, OnModel, Refusal, Registry, Size,
};
use paint::Image;
use std::collections::BTreeSet;
use std::sync::atomic::Ordering::Relaxed;

/// How many texels past a UV piece's edge the model's maps reach out.
const SPREAD: usize = 8;

/// The model's maps as baked for a graph, and which texels its polygons
/// cover.
#[derive(Debug, Clone)]
pub struct ModelBake {
    /// The maps, over the whole texture square.
    pub maps: ModelMaps,
    /// For each texel, row by row from the top left, whether a polygon
    /// covers it.
    pub covered: Vec<bool>,
}

/// Bakes `needed` of the model's maps from `sources` (the objects sharing
/// the texture set), with `settings` (how many rays occlusion and
/// thickness cast, how far they reach, how wide edges are). `key` names
/// them for the graph's cache. `None` for a texture size a graph can't be
/// worked out at, or once `progress` is cancelled.
pub fn model_maps(
    sources: &[Source<'_>],
    needed: &BTreeSet<ModelMap>,
    settings: &BakeSettings,
    progress: &Progress,
    key: u64,
) -> Option<ModelBake> {
    let n = sources.first().map_or(64, |s| s.size);
    let size = Size::new(n).ok()?;
    let texels: Vec<Vec<texels::Texel>> = sources
        .iter()
        .map(|s| texels::covered(s.surface, s.slots, n))
        .collect();
    let total: usize = texels.iter().map(Vec::len).sum();
    progress.done.store(0, Relaxed);
    progress
        .total
        .store((total * needed.len().max(1)) as u64, Relaxed);
    let count = (n * n) as usize;
    let mut covered = vec![false; count];
    for t in texels.iter().flatten() {
        covered[(t.y * n + t.x) as usize] = true;
    }
    let mut maps = ModelMaps::new(size, key);
    for map in needed {
        let mut values = vec![f32::NAN; count];
        for (source, texels) in sources.iter().zip(&texels) {
            let surface = source.surface;
            let (lo, hi) = bounds(surface, source.slots);
            let span = (hi - lo).max(Vec3::splat(1e-6));
            let diagonal = (hi - lo).length().max(1e-6);
            let rays =
                matches!(map, ModelMap::Occlusion | ModelMap::Thickness).then(|| Rays::of(surface));
            let edges = (*map == ModelMap::Curvature)
                .then(|| Edges::of(surface, source.slots, diagonal * settings.edge_width));
            let parts = mesh::parallel::in_parallel(texels.len(), |range| {
                let mut out = Vec::with_capacity(range.len());
                for t in &texels[range] {
                    if progress.cancel.load(Relaxed) {
                        break;
                    }
                    let v = match map {
                        ModelMap::Curvature => edges.as_ref().map_or(0.5, |e| e.at(t.at)),
                        ModelMap::Occlusion => rays.as_ref().map_or(1.0, |r| {
                            r.openness(
                                t.at,
                                t.normal,
                                diagonal * settings.reach,
                                settings.rays,
                                t.seed,
                            )
                        }),
                        ModelMap::Thickness => rays.as_ref().map_or(0.0, |r| {
                            r.thinness(
                                t.at,
                                t.normal,
                                diagonal * settings.reach,
                                settings.rays,
                                t.seed,
                            )
                        }),
                        ModelMap::FacingUp => t.shading.z.max(0.0),
                        ModelMap::Up => (t.at.z - lo.z) / span.z,
                        ModelMap::Across => (t.at.x - lo.x) / span.x,
                        ModelMap::FrontToBack => (t.at.y - lo.y) / span.y,
                    };
                    out.push((t.x, t.y, v.clamp(0.0, 1.0)));
                }
                progress.done.fetch_add(out.len() as u64, Relaxed);
                out
            });
            for part in parts {
                for (x, y, v) in part {
                    values[(y * n + x) as usize] = v;
                }
            }
        }
        if progress.cancel.load(Relaxed) {
            return None;
        }
        spread(&mut values, n as usize, SPREAD);
        let neutral = map.neutral();
        for v in &mut values {
            if v.is_nan() {
                *v = neutral;
            }
        }
        maps.insert(*map, Grey::from_pixels(size, values)?);
    }
    Some(ModelBake { maps, covered })
}

/// Fills unknown (not-a-number) texels next to known ones with the
/// average of their known neighbours, ring by ring, `rings` times.
fn spread(values: &mut [f32], n: usize, rings: usize) {
    for _ in 0..rings {
        let before = values.to_vec();
        let mut any = false;
        for y in 0..n {
            for x in 0..n {
                if !before[y * n + x].is_nan() {
                    continue;
                }
                let (mut sum, mut count) = (0.0, 0);
                for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= n as i64 || ny >= n as i64 {
                        continue;
                    }
                    let v = before[ny as usize * n + nx as usize];
                    if !v.is_nan() {
                        sum += v;
                        count += 1;
                    }
                }
                if count > 0 {
                    values[y * n + x] = sum / count as f32;
                    any = true;
                }
            }
        }
        if !any {
            return;
        }
    }
}

/// A graph fill as made: its colour, and its roughness, metal and height
/// as greys (for the realism channels; height 128 is level).
#[derive(Debug, Clone, PartialEq)]
pub struct MadeGraph {
    /// The colour, opaque where the shape's polygons are.
    pub colour: Image,
    /// Roughness, grey.
    pub roughness: Image,
    /// Metal, grey.
    pub metal: Image,
    /// Height, grey.
    pub height: Image,
}

/// `graph` worked out on the model (`bake`, laid `repeats` times across
/// its texture square) by `evaluator` (which keeps each box's result, so
/// a graph worked out again after a small change is quick), as a layer's
/// pixels; or why it can't be (no Material out, a box that can't run).
pub fn graph_fill(
    evaluator: &mut Evaluator,
    registry: &Registry,
    graph: &Graph,
    bake: &ModelBake,
    repeats: f32,
) -> Result<MadeGraph, Refusal> {
    let on = OnModel {
        maps: std::sync::Arc::new(bake.maps.clone()),
        repeats,
    };
    let material = evaluator.material_on_model(registry, graph, &on)?;
    made(&material, &bake.covered)
        .ok_or_else(|| Refusal::new(words::tr("The graph's material is the wrong size.")))
}

/// `material`'s maps as paint images, on the `covered` texels.
fn made(material: &MaterialMaps, covered: &[bool]) -> Option<MadeGraph> {
    let n = material.size();
    if covered.len() != (n * n) as usize {
        return None;
    }
    let mut out = MadeGraph {
        colour: Image::new(n)?,
        roughness: Image::new(n)?,
        metal: Image::new(n)?,
        height: Image::new(n)?,
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let grey = |v: f32| {
        let g = byte(v);
        [g, g, g, 255]
    };
    for y in 0..n {
        for x in 0..n {
            if !covered[(y * n + x) as usize] {
                continue;
            }
            let (i, j) = (i64::from(x), i64::from(y));
            let c = material.colour.at(i, j);
            out.colour
                .set_pixel(x, y, [byte(c[0]), byte(c[1]), byte(c[2]), 255]);
            out.roughness
                .set_pixel(x, y, grey(material.roughness.at(i, j)));
            out.metal.set_pixel(x, y, grey(material.metal.at(i, j)));
            out.height.set_pixel(x, y, grey(material.height.at(i, j)));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests;
