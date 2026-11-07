//! Working a graph out: every box's pictures at a chosen size.
//!
//! Boxes run in order, each after everything feeding it. A box whose
//! result is in the [`Cache`] under the same key is not run again. Each
//! box shares its pixels over the cores in tiles (see `parallel`), so the
//! pictures are the same however many cores there are.
//!
//! A box that can't run (a needed input empty, a box from a newer app,
//! one fed by a box that can't run) gets a [`Refusal`] saying why, with
//! ways to make it work; the boxes it doesn't feed still run.

use crate::cache::Cache;
use crate::graph::connect::makers_of;
use crate::graph::{BoxId, Graph, PlugRef};
use crate::image::Size;
use crate::model::OnModel;
use crate::parallel;
use crate::refusal::{Fix, Refusal};
use crate::registry::{BoxInfo, Job, Registry, STANDARD_REAL_SIZE_CM};
use crate::settings::{SettingKind, SettingValue};
use crate::value::{MaterialMaps, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use words::{tr, trf};

/// The id of the box whose input is the graph's material.
pub const MATERIAL_OUT: &str = "material_out";

/// What one box made.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxResult {
    /// One value per output, in order.
    pub outputs: Vec<Value>,
    /// What its picture shows: its first output, or for a box with no
    /// outputs (Material out) what it was given.
    pub picture: Option<Value>,
}

impl BoxResult {
    /// How much memory it takes, in bytes (shared pictures counted once
    /// for the picture).
    pub fn memory_size(&self) -> usize {
        let outputs: usize = self.outputs.iter().map(Value::memory_size).sum();
        let picture = match (&self.picture, self.outputs.first()) {
            (Some(p), Some(first)) if p == first => 0,
            (Some(p), _) => p.memory_size(),
            (None, _) => 0,
        };
        outputs + picture
    }
}

/// The outcome of working a graph out.
#[derive(Debug, Clone)]
pub struct Evaluation {
    outcomes: BTreeMap<BoxId, Result<Arc<BoxResult>, Refusal>>,
    made: usize,
    reused: usize,
}

impl Evaluation {
    /// What box `id` made, or why it couldn't run; `None` for a box that
    /// wasn't asked for.
    pub fn outcome(&self, id: BoxId) -> Option<&Result<Arc<BoxResult>, Refusal>> {
        self.outcomes.get(&id)
    }

    /// What box `id`'s picture shows, if it ran.
    pub fn picture(&self, id: BoxId) -> Option<&Value> {
        self.outcomes.get(&id)?.as_ref().ok()?.picture.as_ref()
    }

    /// Every box that couldn't run, with why.
    pub fn problems(&self) -> impl Iterator<Item = (BoxId, &Refusal)> {
        self.outcomes
            .iter()
            .filter_map(|(id, o)| o.as_ref().err().map(|r| (*id, r)))
    }

    /// How many boxes were run.
    pub fn made(&self) -> usize {
        self.made
    }

    /// How many boxes' results were found in the cache.
    pub fn reused(&self) -> usize {
        self.reused
    }
}

/// Works graphs out, keeping each box's result between runs.
#[derive(Debug)]
pub struct Evaluator {
    cache: Cache,
    threads: usize,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    /// An evaluator using every core.
    pub fn new() -> Self {
        Self::with_threads(parallel::cores())
    }

    /// An evaluator using up to `threads` cores (at least one).
    pub fn with_threads(threads: usize) -> Self {
        Self {
            cache: Cache::new(),
            threads: threads.max(1),
        }
    }

    /// Its kept results.
    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    /// Its kept results, to change the budget or clear them.
    pub fn cache_mut(&mut self) -> &mut Cache {
        &mut self.cache
    }

    /// Works out the boxes `wanted` (and everything feeding them; every
    /// box when empty) of `graph` at `size`.
    pub fn run(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        size: Size,
        wanted: &[BoxId],
    ) -> Evaluation {
        self.run_in(registry, graph, size, None, wanted)
    }

    /// Works out the boxes `wanted` of `graph` on a model, in the painter
    /// (see [`crate::model`]): boxes nothing from the model feeds make the
    /// material's repeat at [`OnModel::tile_size`]; the From the model
    /// boxes, and every box they feed, work on the model's whole texture
    /// square at its size, taking repeats laid across it
    /// ([`Value::repeated`]).
    pub fn run_on_model(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        on: &OnModel,
        wanted: &[BoxId],
    ) -> Evaluation {
        self.run_in(registry, graph, on.tile_size(), Some(on), wanted)
    }

    fn run_in(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        size: Size,
        on: Option<&OnModel>,
        wanted: &[BoxId],
    ) -> Evaluation {
        self.cache.begin();
        let mut out = Evaluation {
            outcomes: BTreeMap::new(),
            made: 0,
            reused: 0,
        };
        let mut keys: HashMap<BoxId, u64> = HashMap::new();
        let real_size = real_size_cm(registry, graph);
        // The boxes working on the model's square (only with a model).
        let mut on_square: HashSet<BoxId> = HashSet::new();
        for id in graph.order(wanted) {
            let square = on.is_some()
                && (graph
                    .get(id)
                    .is_some_and(|b| crate::model::is_from_model(registry, &b.kind))
                    || graph.sources_of(id).any(|s| on_square.contains(&s)));
            if square {
                on_square.insert(id);
            }
            let place = match on.filter(|_| square) {
                Some(on) => Place {
                    size: on.maps.size(),
                    real_size: on.square_cm(real_size),
                    on: Some(on),
                    square: &on_square,
                },
                None => Place {
                    size,
                    real_size,
                    on,
                    square: &on_square,
                },
            };
            let outcome = self.run_box(registry, graph, &place, id, &out, &mut keys);
            match &outcome {
                Ok((_, true)) => out.made += 1,
                Ok((_, false)) => out.reused += 1,
                Err(_) => {}
            }
            out.outcomes.insert(id, outcome.map(|(r, _)| r));
        }
        self.cache.trim();
        out
    }

    /// The graph's material: what its Material out box is given, at
    /// `size`; or why there is none.
    pub fn material(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        size: Size,
    ) -> Result<Arc<MaterialMaps>, Refusal> {
        let Some(id) = graph.first_of(MATERIAL_OUT) else {
            return Err(Refusal::new(tr(
                "This graph has no Material out box, so it doesn't make a material yet.",
            ))
            .with_fixes([Fix::AddBox {
                kind: MATERIAL_OUT.to_string(),
            }]));
        };
        let evaluation = self.run(registry, graph, size, &[id]);
        match evaluation.outcome(id) {
            Some(Ok(result)) => match &result.picture {
                Some(Value::Material(m)) => Ok(m.clone()),
                _ => Err(Refusal::new(tr("Material out was given no material."))),
            },
            Some(Err(why)) => Err(why.clone()),
            None => Err(Refusal::new(tr("Material out couldn't run."))),
        }
    }

    /// The graph's material laid on a model (see [`Self::run_on_model`]):
    /// the model's whole texture square at its size, the material's
    /// repeat laid across it when nothing from the model fed it; or why
    /// there is none.
    pub fn material_on_model(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        on: &OnModel,
    ) -> Result<Arc<MaterialMaps>, Refusal> {
        let Some(id) = graph.first_of(MATERIAL_OUT) else {
            return Err(Refusal::new(tr(
                "This graph has no Material out box, so it doesn't make a material yet.",
            ))
            .with_fixes([Fix::AddBox {
                kind: MATERIAL_OUT.to_string(),
            }]));
        };
        let evaluation = self.run_on_model(registry, graph, on, &[id]);
        let material = match evaluation.outcome(id) {
            Some(Ok(result)) => match &result.picture {
                Some(Value::Material(m)) => m.clone(),
                _ => return Err(Refusal::new(tr("Material out was given no material."))),
            },
            Some(Err(why)) => return Err(why.clone()),
            None => return Err(Refusal::new(tr("Material out couldn't run."))),
        };
        let size = on.maps.size();
        if material.size() == size.get() {
            return Ok(material);
        }
        match Value::Material(material).repeated(on.repeats(), size, self.threads) {
            Value::Material(m) => Ok(m),
            _ => Err(Refusal::new(tr("Material out was given no material."))),
        }
    }

    /// Box `id`'s result (and whether it was made now rather than found
    /// in the cache), or why it can't run.
    fn run_box(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        place: &Place<'_>,
        id: BoxId,
        done: &Evaluation,
        keys: &mut HashMap<BoxId, u64>,
    ) -> Result<(Arc<BoxResult>, bool), Refusal> {
        let (size, real_size) = (place.size, place.real_size);
        let on_square = place.square.contains(&id);
        let info = graph.info_of(registry, id)?;
        let kind = registry
            .get(&info.id)
            .ok_or_else(|| Refusal::new(tr("That box isn't in the graph any more.")))?;
        let b = graph
            .get(id)
            .ok_or_else(|| Refusal::new(tr("That box isn't in the graph any more.")))?;
        let settings = b.settings.resolved(&info.settings);
        let seed = own_seed(info, &settings).map_or(0, |s| crate::seed::box_seed(graph.seed(), s));

        let mut hasher = DefaultHasher::new();
        info.id.hash(&mut hasher);
        size.hash(&mut hasher);
        seed.hash(&mut hasher);
        for value in &settings {
            value.hash_into(&mut hasher);
        }
        if info.uses_real_size() {
            real_size.to_bits().hash(&mut hasher);
        }
        let from_model = info.group == crate::registry::Group::Model;
        let model = place.on.filter(|_| on_square && from_model);
        if let Some(on) = model {
            (3u8, on.maps.key()).hash(&mut hasher);
        }
        let mut inputs = Vec::with_capacity(info.inputs.len());
        for spec in &info.inputs {
            let to = PlugRef::new(id, spec.key.to_string());
            let Some(wire) = graph.wire_into(&to) else {
                if spec.needed {
                    return Err(Refusal::new(trf!(
                        "{} has nothing plugged into {}: it needs {}.",
                        info.name.show(),
                        spec.name.show(),
                        spec.kind.described()
                    ))
                    .with_fixes(graph.others_that_fit(registry, &to, spec.kind))
                    .with_fixes(makers_of(registry, spec.kind)));
                }
                0u8.hash(&mut hasher);
                inputs.push(None);
                continue;
            };
            let source = wire.from.node;
            let waits = || {
                let name = graph
                    .info_of(registry, source)
                    .map_or_else(|_| tr("a box").to_string(), |i| i.name.show());
                Refusal::new(trf!(
                    "{} waits for {}, which can't run.",
                    info.name.show(),
                    name
                ))
                .with_fixes([Fix::LookAt(source)])
            };
            let (Some(Ok(result)), Some(key)) = (done.outcome(source), keys.get(&source)) else {
                return Err(waits());
            };
            let at = graph
                .info_of(registry, source)?
                .outputs
                .iter()
                .position(|o| o.key == wire.from.plug)
                .ok_or_else(waits)?;
            let value = result
                .outputs
                .get(at)
                .and_then(|v| v.converted(spec.kind))
                .ok_or_else(waits)?;
            (1u8, *key, &wire.from.plug).hash(&mut hasher);
            // A repeat going into a box on the model's square is laid
            // across the square first.
            let value = match place.on {
                Some(on) if on_square && !place.square.contains(&source) => {
                    (2u8, on.repeats().to_bits()).hash(&mut hasher);
                    value.repeated(on.repeats(), size, self.threads)
                }
                _ => value,
            };
            inputs.push(Some(value));
        }
        let key = hasher.finish();
        keys.insert(id, key);
        if let Some(hit) = self.cache.get(key) {
            return Ok((hit, false));
        }
        let job = Job::new(info, size, seed, self.threads, &b.settings, inputs)
            .with_real_size(real_size)
            .with_model(model.map(|on| on.maps.clone()));
        let values = kind.run(&job)?;
        let result = Arc::new(checked(info, size, values)?);
        self.cache.put(key, result.clone());
        Ok((result, true))
    }
}

/// Where one box is worked out: its size and real-world size, and with a
/// model, which boxes work on the model's square.
struct Place<'a> {
    size: Size,
    real_size: f32,
    on: Option<&'a OnModel>,
    square: &'a HashSet<BoxId>,
}

/// How many centimetres the graph's texture covers: its Material out's
/// real-world size, or the standard without one.
pub fn real_size_cm(registry: &Registry, graph: &Graph) -> f32 {
    let size = graph.first_of(MATERIAL_OUT).and_then(|id| {
        let info = registry.get(MATERIAL_OUT)?.info();
        let spec = info.setting(REAL_SIZE)?;
        let value = graph.get(id)?.settings.get(REAL_SIZE)?;
        match spec.nearest(value) {
            SettingValue::Number(cm) => Some(cm),
            _ => None,
        }
    });
    size.unwrap_or(STANDARD_REAL_SIZE_CM)
}

/// The key of Material out's real-world size setting.
pub const REAL_SIZE: &str = "size_cm";

/// The box's own seed setting, if it has one.
fn own_seed(info: &BoxInfo, settings: &[SettingValue]) -> Option<i32> {
    info.settings
        .iter()
        .zip(settings)
        .find_map(|(spec, value)| match (&spec.kind, value) {
            (SettingKind::Seed, SettingValue::Whole(n)) => Some(*n),
            _ => None,
        })
}

/// What a box gave, checked against what it said it makes.
fn checked(info: &BoxInfo, size: Size, values: Vec<Value>) -> Result<BoxResult, Refusal> {
    let wrong = || {
        Refusal::new(trf!(
            "{} made something other than it should; this is a fault in the app.",
            info.name.show()
        ))
    };
    if values.iter().any(|v| v.size() != size.get()) {
        return Err(wrong());
    }
    if info.outputs.is_empty() {
        return Ok(BoxResult {
            picture: values.into_iter().next(),
            outputs: Vec::new(),
        });
    }
    let kinds_match = values.len() == info.outputs.len()
        && values
            .iter()
            .zip(&info.outputs)
            .all(|(v, o)| v.kind() == o.kind);
    if !kinds_match {
        return Err(wrong());
    }
    Ok(BoxResult {
        picture: values.first().cloned(),
        outputs: values,
    })
}

#[cfg(test)]
mod tests;
