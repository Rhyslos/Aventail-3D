//! Bypassed boxes (milestone 16, [decision 0278](../../../../docs/decisions/0278-bypass-and-reroute-dots.md)):
//! a box switched off for a moment, so the graph can be seen with and
//! without it.
//!
//! A bypassed box doesn't run. Each of its outputs passes on the first
//! input of the same kind that has something plugged in; failing that,
//! the first that converts to it (a shape as a height, greys as a
//! colour, a material's height or colour), as a wire would. An output
//! with no such input gives nothing, and a box wired to it says why in
//! plain words, offering to switch the box back on.
//!
//! Its result key comes only from what it passes on, never its settings,
//! seed or the time: a bypassed moving box doesn't fill the cache while
//! the preview plays, and switching bypass on and off finds both results
//! still kept. Nothing it passes on is put in the cache again (it is its
//! input's picture).

use super::{BoxResult, Evaluation, Evaluator, Place};
use crate::graph::{BoxId, Graph};
use crate::kind::{Fit, fit};
use crate::refusal::{Fix, Refusal};
use crate::registry::Registry;
use crate::value::Value;
use crate::{PlugSpec, ValueKind};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use words::{tr, trf};

/// Which input each output of a box passes on while bypassed: for each
/// output (by index), the index of the input whose value it takes.
/// `plugged` says which inputs have something.
pub fn passed_through(
    inputs: &[PlugSpec],
    outputs: &[PlugSpec],
    plugged: &[bool],
) -> Vec<Option<usize>> {
    let takes = |want: ValueKind, ok: &dyn Fn(Result<Fit, &str>) -> bool| {
        inputs
            .iter()
            .zip(plugged)
            .position(|(i, plugged)| *plugged && ok(fit(i.kind, want)))
    };
    outputs
        .iter()
        .map(|o| {
            takes(o.kind, &|f| f == Ok(Fit::Fits))
                .or_else(|| takes(o.kind, &|f| matches!(f, Ok(Fit::Converts(_)))))
        })
        .collect()
}

impl Evaluator {
    /// Box `id`, bypassed: its inputs passed straight to its outputs.
    pub(super) fn run_bypassed(
        &mut self,
        registry: &Registry,
        graph: &Graph,
        place: &Place<'_>,
        id: BoxId,
        done: &Evaluation,
        keys: &mut HashMap<BoxId, u64>,
    ) -> Result<(Arc<BoxResult>, bool), Refusal> {
        let info = graph.info_of(registry, id)?;
        let mut hasher = DefaultHasher::new();
        (9u8, &info.id, place.size).hash(&mut hasher);
        let inputs = self.inputs(registry, graph, place, id, done, keys, &mut hasher, false)?;
        let plugged: Vec<bool> = inputs.iter().map(Option::is_some).collect();
        let outputs: Vec<Option<Value>> = passed_through(&info.inputs, &info.outputs, &plugged)
            .into_iter()
            .zip(&info.outputs)
            .map(|(from, out)| {
                let value = inputs.get(from?)?.as_ref()?;
                value.converted(out.kind)
            })
            .collect();
        keys.insert(id, hasher.finish());
        let picture = outputs.iter().flatten().next().cloned();
        Ok((Arc::new(BoxResult { outputs, picture }), false))
    }
}

/// Why a box wired to output `plug` of the bypassed box `source` can't
/// run: the bypassed box has no input that can pass on to that output.
pub(super) fn nothing_passed(
    registry: &Registry,
    graph: &Graph,
    source: BoxId,
    plug: &str,
) -> Refusal {
    let Ok(info) = graph.info_of(registry, source) else {
        return empty_output_unknown(source);
    };
    let output = info.output(plug);
    let output_name = output.map_or_else(|| plug.to_string(), |o| o.name.show());
    let has_kind = output.is_some_and(|o| {
        info.inputs
            .iter()
            .any(|i| matches!(fit(i.kind, o.kind), Ok(Fit::Fits | Fit::Converts(_))))
    });
    let why = if has_kind {
        trf!(
            "{} is bypassed, so it only passes on what is plugged into it, and nothing that \
             could go out of its {} output is plugged in. Plug one in, or switch {} back on.",
            info.name.show(),
            output_name,
            info.name.show()
        )
    } else {
        trf!(
            "{} is bypassed, so it only passes on what is plugged into it, and it takes \
             nothing that could go out of its {} output ({}): it makes that itself. Switch \
             it back on, or connect a box before it instead.",
            info.name.show(),
            output_name,
            output.map_or("", |o| o.kind.described())
        )
    };
    Refusal::new(why).with_fixes([Fix::StopBypass(source), Fix::LookAt(source)])
}

/// [`nothing_passed`] for a box whose kind is unknown (it can't be
/// bypassed, but a damaged graph may say so).
fn empty_output_unknown(source: BoxId) -> Refusal {
    Refusal::new(tr("A bypassed box passes nothing on there."))
        .with_fixes([Fix::StopBypass(source)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outputs_take_the_first_input_of_their_kind_then_one_that_converts() {
        let registry = Registry::standard();
        let info = |id: &str| registry.get(id).unwrap().info();
        // Transform: height, colour and material in and out.
        let t = info("transform");
        let all = vec![true; t.inputs.len()];
        let through = passed_through(&t.inputs, &t.outputs, &all);
        for (o, from) in t.outputs.iter().zip(&through) {
            assert_eq!(t.inputs[from.unwrap()].kind, o.kind, "{}", o.key);
        }
        // Bevel: a shape in, a height out: the shape converts.
        let bevel = info("bevel");
        let through = passed_through(&bevel.inputs, &bevel.outputs, &[true]);
        assert_eq!(through[0], Some(0));
        // Material from height: nothing takes a material in.
        let mfh = info("material_from_height");
        let plugged = vec![true; mfh.inputs.len()];
        let through = passed_through(&mfh.inputs, &mfh.outputs, &plugged);
        let material = mfh
            .outputs
            .iter()
            .position(|o| o.kind == ValueKind::Material)
            .unwrap();
        assert_eq!(through[material], None);
        // Nothing plugged in: nothing passes.
        let none = vec![false; t.inputs.len()];
        assert!(
            passed_through(&t.inputs, &t.outputs, &none)
                .iter()
                .all(Option::is_none)
        );
    }
}
