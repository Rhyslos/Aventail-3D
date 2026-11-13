//! Your own boxes (Phase 12, milestone 7): boxes made from a group of
//! boxes, saved as one box with its own plugs and sliders
//! ([decision 0263](../../../docs/decisions/0263-your-own-boxes.md)).
//!
//! An [`OwnBox`] is a definition: a small graph inside (its own boxes,
//! wires and comment frames), the plugs it shows (each input feeds plugs
//! inside, each output comes from a plug inside), and its sliders (inner
//! boxes' settings it shows as its own). A project keeps its own boxes
//! beside its graphs ([`crate::GraphStore`]); a graph places one as a box
//! whose kind is the definition's id (`own:` and sixteen hex digits), so
//! every box placed from one definition is **linked**: change the
//! boxes inside and every one changes. Its sliders are each placed box's
//! own settings.
//!
//! The project's registry ([`registry_with`]) holds the standard boxes
//! and an [`OwnBoxType`] per definition, which works its graph out
//! inside ([`crate::Evaluator::run_given`]) with what is plugged into it,
//! the graph's seed and real-world size, so making boxes into your own
//! box (and opening it back up) never changes the picture.
//!
//! Making one from a selection and opening one back up are in `making`;
//! bringing them from another project (pasting between tabs) in
//! `bringing`.

pub mod bringing;
#[cfg(test)]
mod bringing_tests;
mod making;
#[cfg(test)]
mod tests;

pub use making::{OpenedUp, Proposal, ProposedInput, ProposedOutput, ProposedSlider, open_up};

use crate::eval::Evaluator;
use crate::graph::{BoxId, Graph, GraphParts, MAX_NAME, MAX_NOTE, PlugRef};
use crate::kind::{ValueKind, fit};
use crate::refusal::Refusal;
use crate::registry::{BoxInfo, BoxType, Group, Job, Label, PlugSpec, Registry};
use crate::settings::{SettingKind, SettingSpec, SettingValue};
use crate::value::Value;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use words::{text, tr, trf};

/// The start of every own box's kind id.
pub const OWN_PREFIX: &str = "own:";
/// The most own boxes a project (or your library) may hold.
pub const MAX_OWN_BOXES: usize = 500;
/// The most own boxes inside each other: one inside another inside
/// another, and so on.
pub const MAX_DEPTH: usize = 8;
/// The most plugs on each side of an own box.
pub const MAX_PLUGS: usize = 16;
/// The most sliders on an own box.
pub const MAX_SLIDERS: usize = 32;
/// The most inner plugs one input feeds.
const MAX_TARGETS: usize = 32;
/// An own box's picture is this many pixels across.
pub const PICTURE_SIDE: u32 = 48;
/// The longest plug or slider key, in bytes.
const MAX_KEY: usize = 64;

/// Whether `kind` names one of your own boxes.
pub fn is_own(kind: &str) -> bool {
    kind.starts_with(OWN_PREFIX)
}

/// Whether `id` is a well-made own box id: the prefix and 1 to 32 hex
/// digits.
pub fn is_own_id(id: &str) -> bool {
    id.strip_prefix(OWN_PREFIX).is_some_and(|hex| {
        (1..=32).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

/// An own box id made from `seed` (anything describing what is being
/// made), not in `taken`.
pub fn fresh_id(seed: &str, taken: impl Fn(&str) -> bool) -> String {
    (0u64..)
        .map(|salt| {
            let mut h = DefaultHasher::new();
            (seed, salt).hash(&mut h);
            format!("{OWN_PREFIX}{:016x}", h.finish())
        })
        .find(|id| !taken(id))
        .unwrap_or_default()
}

/// One input of an own box: what is plugged in goes to plugs inside.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnInput {
    /// Its key (wires name it).
    pub key: String,
    /// Its name, in the maker's own words.
    pub name: String,
    /// What it takes.
    pub kind: ValueKind,
    /// The inner inputs it feeds (inputs with no wire inside).
    pub to: Vec<PlugRef>,
}

/// One output of an own box: it gives what a plug inside makes.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnOutput {
    /// Its key (wires name it).
    pub key: String,
    /// Its name, in the maker's own words.
    pub name: String,
    /// What it gives.
    pub kind: ValueKind,
    /// The inner output it gives; `None` when the box that made it was
    /// removed inside (the output then says so when it runs).
    pub from: Option<PlugRef>,
}

/// One slider of an own box: a setting of a box inside, shown as the own
/// box's own setting.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnSlider {
    /// Its key (each placed box's settings name it).
    pub key: String,
    /// Its name, in the maker's own words.
    pub name: String,
    /// The box inside.
    pub node: BoxId,
    /// That box's setting.
    pub setting: String,
}

/// One of your own boxes: a definition.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnBox {
    /// Its id, the kind of every box placed from it (`own:` and hex).
    pub id: String,
    /// Its name, in the maker's own words.
    pub name: String,
    /// "What does this do?", in the maker's own words.
    pub about: String,
    /// Its picture: [`PICTURE_SIDE`] square, sRGB bytes, row by row.
    pub picture: Option<Vec<u8>>,
    /// The boxes inside.
    pub inner: Arc<Graph>,
    /// Its inputs.
    pub inputs: Vec<OwnInput>,
    /// Its outputs.
    pub outputs: Vec<OwnOutput>,
    /// Its sliders.
    pub sliders: Vec<OwnSlider>,
}

/// An own box as plain parts, from a file ([`OwnBox::from_parts`] checks
/// everything).
#[derive(Debug, Clone, PartialEq)]
pub struct OwnParts {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// "What does this do?".
    pub about: String,
    /// Its picture.
    pub picture: Option<Vec<u8>>,
    /// The boxes inside.
    pub inner: GraphParts,
    /// Its inputs.
    pub inputs: Vec<OwnInput>,
    /// Its outputs.
    pub outputs: Vec<OwnOutput>,
    /// Its sliders.
    pub sliders: Vec<OwnSlider>,
}

impl OwnBox {
    /// Its plain parts, to save.
    pub fn to_parts(&self) -> OwnParts {
        OwnParts {
            id: self.id.clone(),
            name: self.name.clone(),
            about: self.about.clone(),
            picture: self.picture.clone(),
            inner: self.inner.to_parts(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            sliders: self.sliders.clone(),
        }
    }

    /// Rebuilds one from its parts (from a file), checked against
    /// `registry` (which must know the own boxes inside it).
    pub fn from_parts(parts: OwnParts, registry: &Registry) -> Result<Self, String> {
        let inner = Graph::from_parts(parts.inner, registry)?;
        let own = Self {
            id: parts.id,
            name: parts.name,
            about: parts.about,
            picture: parts.picture,
            inner: Arc::new(inner),
            inputs: parts.inputs,
            outputs: parts.outputs,
            sliders: parts.sliders,
        };
        own.check(registry)?;
        Ok(own)
    }

    /// The own boxes it holds inside, by id.
    pub fn uses(&self) -> BTreeSet<String> {
        own_kinds(self.inner.boxes().map(|(_, b)| b.kind.as_str()))
    }

    /// Why it can't be kept, if it can't: its id, name and words, its
    /// picture, and every plug and slider pointing at something inside
    /// that is there and fits.
    pub fn check(&self, registry: &Registry) -> Result<(), String> {
        let damaged = |why: &'static str| Err(tr(why).to_string());
        if !is_own_id(&self.id) {
            return damaged(text!("an own box's id is damaged"));
        }
        if self.name.trim().is_empty() || self.name.len() > MAX_NAME {
            return damaged(text!("an own box's name is missing or too long"));
        }
        if self.about.len() > MAX_NOTE {
            return damaged(text!("an own box's words are too long"));
        }
        if self
            .picture
            .as_ref()
            .is_some_and(|p| p.len() != (PICTURE_SIDE * PICTURE_SIDE * 3) as usize)
        {
            return damaged(text!("an own box's picture is damaged"));
        }
        if self.inputs.len() > MAX_PLUGS
            || self.outputs.len() > MAX_PLUGS
            || self.sliders.len() > MAX_SLIDERS
        {
            return damaged(text!("an own box has too many plugs or sliders"));
        }
        if self.uses().contains(&self.id) {
            return damaged(text!("an own box holds itself"));
        }
        let mut keys = BTreeSet::new();
        let names = self
            .inputs
            .iter()
            .map(|i| (&i.key, &i.name))
            .chain(self.outputs.iter().map(|o| (&o.key, &o.name)))
            .chain(self.sliders.iter().map(|s| (&s.key, &s.name)));
        for (key, name) in names {
            if key.is_empty() || key.len() > MAX_KEY || !keys.insert(key.as_str()) {
                return damaged(text!("an own box's plugs or sliders share a key"));
            }
            if name.len() > MAX_NAME {
                return damaged(text!("an own box's plug or slider name is too long"));
            }
        }
        for input in &self.inputs {
            if input.to.len() > MAX_TARGETS {
                return damaged(text!("an own box's input feeds too many plugs"));
            }
            for to in &input.to {
                match self.inner_plug(registry, to, false) {
                    Some(Some(kind)) if fit(input.kind, kind).is_err() => {
                        return damaged(text!("an own box's input doesn't fit the plug it feeds"));
                    }
                    None => return damaged(text!("an own box's input feeds a plug not inside")),
                    _ => {}
                }
            }
        }
        for output in &self.outputs {
            if let Some(from) = &output.from {
                match self.inner_plug(registry, from, true) {
                    Some(Some(kind)) if kind != output.kind => {
                        return damaged(text!("an own box's output doesn't fit the plug inside"));
                    }
                    None => {
                        return damaged(text!("an own box's output comes from a plug not inside"));
                    }
                    _ => {}
                }
            }
        }
        for slider in &self.sliders {
            let Some(b) = self.inner.get(slider.node) else {
                return damaged(text!("an own box's slider is for a box not inside"));
            };
            if let Some(kind) = registry.get(&b.kind)
                && kind.info().setting(&slider.setting).is_none()
            {
                return damaged(text!("an own box's slider is for a setting not there"));
            }
        }
        Ok(())
    }

    /// The kind of inner plug `plug` (an output when `output`): `None`
    /// when its box isn't inside, `Some(None)` when the box's kind is
    /// unknown (kept as it is), and `None` too when the kind is known but
    /// has no such plug.
    fn inner_plug(
        &self,
        registry: &Registry,
        plug: &PlugRef,
        output: bool,
    ) -> Option<Option<ValueKind>> {
        let b = self.inner.get(plug.node)?;
        let Some(kind) = registry.get(&b.kind) else {
            return (plug.plug.len() <= MAX_KEY).then_some(None);
        };
        let info = kind.info();
        let spec = if output {
            info.output(&plug.plug)
        } else {
            info.input(&plug.plug)
        };
        spec.map(|s| Some(s.kind))
    }

    /// The same box after a change inside: inputs feed only plugs still
    /// there with no wire inside, outputs come only from plugs still
    /// there, and sliders are only for settings still there.
    pub fn normalised(&self, registry: &Registry) -> Self {
        let mut out = self.clone();
        for input in &mut out.inputs {
            input.to.retain(|to| {
                self.inner.wire_into(to).is_none()
                    && matches!(self.inner_plug(registry, to, false), Some(k) if k.is_none_or(|k| fit(input.kind, k).is_ok()))
            });
        }
        for output in &mut out.outputs {
            if let Some(from) = &output.from
                && !matches!(self.inner_plug(registry, from, true), Some(k) if k.is_none_or(|k| k == output.kind))
            {
                output.from = None;
            }
        }
        out.sliders.retain(|s| {
            self.inner.get(s.node).is_some_and(|b| {
                registry
                    .get(&b.kind)
                    .is_none_or(|k| k.info().setting(&s.setting).is_some())
            })
        });
        out
    }

    /// A slider's setting, as the inner box's kind has it, if known.
    fn slider_spec<'r>(
        &self,
        registry: &'r Registry,
        slider: &OwnSlider,
    ) -> Option<&'r SettingSpec> {
        let b = self.inner.get(slider.node)?;
        registry.get(&b.kind)?.info().setting(&slider.setting)
    }

    /// Its value of slider `slider` inside: the inner box's setting.
    fn inner_value(&self, spec: &SettingSpec, slider: &OwnSlider) -> SettingValue {
        let value = self
            .inner
            .get(slider.node)
            .and_then(|b| b.settings.get(&slider.setting))
            .unwrap_or_else(|| spec.standard());
        spec.nearest(value)
    }

    /// What it is, as a kind of box: its plugs, its sliders as settings
    /// (each starting at the inner box's value), in the Your own boxes
    /// group. `registry` must know the boxes inside.
    pub fn info(&self, registry: &Registry) -> BoxInfo {
        let inputs = self
            .inputs
            .iter()
            .map(|i| PlugSpec {
                key: Cow::Owned(i.key.clone()),
                name: Label::Own(i.name.clone()),
                kind: i.kind,
                needed: i.to.iter().any(|to| {
                    self.inner
                        .get(to.node)
                        .and_then(|b| registry.get(&b.kind))
                        .and_then(|k| k.info().input(&to.plug))
                        .is_some_and(|p| p.needed)
                }),
            })
            .collect();
        let outputs = self
            .outputs
            .iter()
            .map(|o| PlugSpec {
                key: Cow::Owned(o.key.clone()),
                name: Label::Own(o.name.clone()),
                kind: o.kind,
                needed: false,
            })
            .collect();
        let settings = self
            .sliders
            .iter()
            .filter_map(|s| {
                let spec = self.slider_spec(registry, s)?;
                let value = self.inner_value(spec, s);
                Some(SettingSpec {
                    key: Cow::Owned(s.key.clone()),
                    name: Label::Own(s.name.clone()),
                    about: spec.about.clone(),
                    kind: with_standard(&spec.kind, value),
                })
            })
            .collect();
        let animated = self
            .inner
            .boxes()
            .any(|(_, b)| !b.bypassed && registry.get(&b.kind).is_some_and(|k| k.info().animated));
        BoxInfo {
            id: Cow::Owned(self.id.clone()),
            name: Label::Own(self.name.clone()),
            group: Group::Own,
            about: Label::Own(if self.about.trim().is_empty() {
                tr("Your own box. Its maker hasn't said what it does yet.").to_string()
            } else {
                self.about.clone()
            }),
            inputs,
            outputs,
            settings,
            comes_next: Vec::new(),
            one_per_graph: false,
            animated,
        }
    }

    /// Its inner graph as a placed box of it runs it: each slider's value
    /// (from `values`, by slider key) set on its inner box, and the
    /// outer graph's seed.
    pub fn inner_with(
        &self,
        registry: &Registry,
        values: impl Fn(&str) -> Option<SettingValue>,
        seed: u32,
    ) -> Graph {
        let mut inner = (*self.inner).clone();
        let _ = inner.apply(registry, &crate::GraphEdit::SetSeed(seed));
        for slider in &self.sliders {
            if let Some(value) = values(&slider.key) {
                let edit = crate::GraphEdit::SetSetting {
                    node: slider.node,
                    key: slider.setting.clone(),
                    value,
                };
                let _ = inner.apply(registry, &edit);
            }
        }
        inner
    }

    /// Roughly how much memory it takes, in bytes (for the undo budget).
    pub fn memory_size(&self) -> usize {
        256 + self.name.len()
            + self.about.len()
            + self.picture.as_ref().map_or(0, Vec::len)
            + self.inner.memory_size()
            + (self.inputs.len() + self.outputs.len() + self.sliders.len()) * 96
    }
}

/// `kind` with its standard changed to `value` (a seed becomes a whole
/// number from 0 up, as the graph's seed is mixed in inside).
fn with_standard(kind: &SettingKind, value: SettingValue) -> SettingKind {
    match (kind.clone(), value) {
        (SettingKind::Number { min, max, .. }, SettingValue::Number(n)) => SettingKind::Number {
            min,
            max,
            standard: n,
        },
        (SettingKind::Length { min, max, .. }, SettingValue::Number(n)) => SettingKind::Length {
            min,
            max,
            standard: n,
        },
        (SettingKind::Whole { min, max, .. }, SettingValue::Whole(n)) => SettingKind::Whole {
            min,
            max,
            standard: n,
        },
        (SettingKind::Seed, SettingValue::Whole(n)) => SettingKind::Whole {
            min: 0,
            max: 999_999,
            standard: n.clamp(0, 999_999),
        },
        (SettingKind::Colour { .. }, SettingValue::Colour(c)) => {
            SettingKind::Colour { standard: c }
        }
        (SettingKind::Choice { options, .. }, SettingValue::Choice(c)) => SettingKind::Choice {
            options,
            standard: c,
        },
        (SettingKind::Switch { .. }, SettingValue::Switch(b)) => {
            SettingKind::Switch { standard: b }
        }
        (SettingKind::Gradient { .. }, SettingValue::Gradient(g)) => {
            SettingKind::Gradient { standard: g }
        }
        (other, _) => other,
    }
}

/// The own box ids among `kinds`.
fn own_kinds<'k>(kinds: impl Iterator<Item = &'k str>) -> BTreeSet<String> {
    kinds.filter(|k| is_own(k)).map(str::to_string).collect()
}

/// An own box as a kind of box: it works its inside out.
#[derive(Debug)]
pub struct OwnBoxType {
    own: Arc<OwnBox>,
    info: BoxInfo,
    /// The kinds its inside may use: the standard boxes and the own boxes
    /// it holds (never itself).
    registry: Registry,
    /// How deep own boxes go inside it (1: none inside).
    depth: usize,
    /// Changes whenever it or an own box inside it changes, so worked
    /// out pictures of the old one are never used again.
    version: u64,
}

impl OwnBoxType {
    /// Its definition.
    pub fn own(&self) -> &Arc<OwnBox> {
        &self.own
    }
}

impl BoxType for OwnBoxType {
    fn info(&self) -> &BoxInfo {
        &self.info
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn own(&self) -> Option<&Arc<OwnBox>> {
        Some(&self.own)
    }

    fn run(&self, job: &Job<'_>) -> Result<Vec<Value>, Refusal> {
        let name = &self.own.name;
        if self.depth > MAX_DEPTH {
            return Err(Refusal::new(trf!(
                "{} holds own boxes more than {} deep inside each other; open some back up.",
                name,
                MAX_DEPTH
            )));
        }
        let values = |key: &str| {
            let at = self.info.settings.iter().position(|s| s.key == key)?;
            job.resolved_settings().get(at).copied()
        };
        let inner = self
            .own
            .inner_with(&self.registry, values, job.graph_seed());
        let mut given = HashMap::new();
        for (index, input) in self.own.inputs.iter().enumerate() {
            if let Some(value) = job.input(&input.key) {
                for to in &input.to {
                    given.insert(to.clone(), (value.clone(), index as u64));
                }
            }
        }
        let wanted: Vec<BoxId> = self
            .own
            .outputs
            .iter()
            .filter_map(|o| o.from.as_ref().map(|f| f.node))
            .collect();
        let mut evaluator = Evaluator::with_threads(job.threads());
        let given = crate::eval::Given {
            values: given,
            real_size_cm: job.real_size_cm(),
            time: job.time(),
        };
        let evaluation = evaluator.run_given(&self.registry, &inner, job.size(), &given, &wanted);
        let mut out = Vec::with_capacity(self.own.outputs.len());
        for output in &self.own.outputs {
            let Some(from) = &output.from else {
                return Err(Refusal::new(trf!(
                    "Nothing inside {} gives its {} output any more. Open it (double-click) \
                     and give the output from a box inside, or undo.",
                    name,
                    output.name
                )));
            };
            let result = match evaluation.outcome(from.node) {
                Some(Ok(result)) => result,
                Some(Err(why)) => {
                    return Err(Refusal::new(trf!("Inside {}: {}", name, why.reason)));
                }
                None => {
                    return Err(Refusal::new(trf!(
                        "Inside {}, the box giving {} couldn't run.",
                        name,
                        output.name
                    )));
                }
            };
            let at = self
                .registry
                .get(
                    &inner
                        .get(from.node)
                        .map_or(String::new(), |b| b.kind.clone()),
                )
                .and_then(|k| k.info().outputs.iter().position(|o| o.key == from.plug));
            let bypassed = inner.get(from.node).is_some_and(|b| b.bypassed);
            let value = at
                .and_then(|at| result.outputs.get(at))
                .and_then(Option::as_ref)
                .and_then(|v| v.converted(output.kind));
            if value.is_none() && bypassed {
                return Err(Refusal::new(trf!(
                    "Inside {}, the box giving {} is bypassed and has nothing to pass on to it. \
                     Open it (double-click) and switch that box back on.",
                    name,
                    output.name
                )));
            }
            let value = value.ok_or_else(|| {
                Refusal::new(trf!(
                    "Inside {}, the box giving {} made something else.",
                    name,
                    output.name
                ))
            })?;
            out.push(value);
        }
        Ok(out)
    }
}

/// A registry of the standard boxes and `own` boxes: each own box is
/// added once every own box inside it is, so a box can never run itself.
/// One in a loop (only in a damaged file) is left out.
pub fn registry_with<'a>(own: impl IntoIterator<Item = &'a Arc<OwnBox>>) -> Registry {
    let mut registry = Registry::standard().clone();
    let mut waiting: Vec<&Arc<OwnBox>> = own.into_iter().collect();
    let all: BTreeSet<String> = waiting.iter().map(|o| o.id.clone()).collect();
    loop {
        let mut later = Vec::new();
        let mut added = false;
        for o in waiting {
            // Wait for the own boxes inside it (one missing altogether is
            // kept as an unknown kind and refuses to run).
            if o.uses()
                .iter()
                .any(|u| all.contains(u) && registry.get(u).is_none())
            {
                later.push(o);
                continue;
            }
            added |= add_own(&mut registry, o.clone()).is_ok();
        }
        if later.is_empty() || !added {
            break;
        }
        waiting = later;
    }
    registry
}

/// Adds own box `own` to `registry`, which should already know the own
/// boxes inside it; refused when its id is taken.
pub fn add_own(registry: &mut Registry, own: Arc<OwnBox>) -> Result<(), Refusal> {
    let uses = own.uses();
    let depth = 1 + uses
        .iter()
        .map(|u| making::depth(registry, u))
        .max()
        .unwrap_or(0);
    // What it works out, not its name or picture: and the boxes inside.
    let mut h = DefaultHasher::new();
    format!(
        "{:?}{:?}{:?}{:?}",
        own.inner, own.inputs, own.outputs, own.sliders
    )
    .hash(&mut h);
    for u in &uses {
        registry.get(u).map(|k| k.version()).hash(&mut h);
    }
    let kind = OwnBoxType {
        info: own.info(registry),
        own,
        registry: registry.clone(),
        depth,
        version: h.finish(),
    };
    registry.add(Arc::new(kind))
}

/// A picture of `value` for an own box: [`PICTURE_SIDE`] square, sRGB
/// bytes row by row (greys as greys, a material's colour lit by its
/// height).
pub fn picture_of(value: &Value) -> Vec<u8> {
    let side = PICTURE_SIDE;
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut out = Vec::with_capacity((side * side * 3) as usize);
    for y in 0..side {
        for x in 0..side {
            let (u, v) = (
                (x as f32 + 0.5) / side as f32,
                (y as f32 + 0.5) / side as f32,
            );
            let colour = match value {
                Value::Shape(g) | Value::Height(g) => [g.sample(u, v); 3],
                Value::Colour(c) => c.sample_rgb(u, v),
                Value::Material(m) => {
                    let shade = 0.55 + 0.45 * m.height.sample(u, v);
                    m.colour.sample_rgb(u, v).map(|c| c * shade)
                }
            };
            out.extend(colour.map(byte));
        }
    }
    out
}
