//! Checking a wire before it is made: plugs that exist, kinds that fit,
//! no loops; and when it can't be made, why, with ways to make it work.

use super::{BoxId, Graph, PlugRef};
use crate::kind::{Fit, ValueKind, fit};
use crate::refusal::{Fix, Refusal};
use crate::registry::{BoxInfo, Registry};
use words::{tr, trf};

/// The most fixes of one sort offered.
const MOST_FIXES: usize = 3;

impl Graph {
    /// The kind of box `id`, if the box is there and `registry` knows it,
    /// or why not.
    pub(crate) fn info_of<'r>(
        &self,
        registry: &'r Registry,
        id: BoxId,
    ) -> Result<&'r BoxInfo, Refusal> {
        let b = self
            .get(id)
            .ok_or_else(|| Refusal::new(tr("That box isn't in the graph any more.")))?;
        registry.get(&b.kind).map(|t| t.info()).ok_or_else(|| {
            let why = if crate::own::is_own(&b.kind) {
                tr("This is one of your own boxes, but the project doesn't hold it any more \
                    (or it holds itself). Bring its box file in again, or remove it.")
                .to_string()
            } else {
                trf!(
                    "This box ({}) is from a newer version of the app, which this one doesn't know.",
                    b.kind
                )
            };
            Refusal::new(why).with_fixes([Fix::RemoveBox(id)])
        })
    }

    /// Whether a wire from output `from` to input `to` can be made, and
    /// how (as it is, or converted with a note); or why not, with ways to
    /// make it work (a box to put between them, other outputs that fit).
    /// A wire already in `to` would be replaced.
    pub fn check_connection(
        &self,
        registry: &Registry,
        from: &PlugRef,
        to: &PlugRef,
    ) -> Result<Fit, Refusal> {
        let (source, target) = (
            self.info_of(registry, from.node)?,
            self.info_of(registry, to.node)?,
        );
        let output = source.output(&from.plug).ok_or_else(|| {
            Refusal::new(trf!(
                "{} has no output called {}.",
                source.name.show(),
                from.plug
            ))
        })?;
        let input = target.input(&to.plug).ok_or_else(|| {
            Refusal::new(trf!(
                "{} has no input called {}.",
                target.name.show(),
                to.plug
            ))
        })?;
        if from.node == to.node {
            return Err(Refusal::new(tr(
                "A box can't feed itself: its picture would wait for itself.",
            ))
            .with_fixes(self.others_that_fit(registry, to, input.kind)));
        }
        let how = fit(output.kind, input.kind).map_err(|why| {
            let reason = trf!(
                "{}'s {} needs {}, and {}.",
                target.name.show(),
                input.name.show(),
                input.kind.described(),
                why
            );
            Refusal::new(reason)
                .with_fixes(boxes_between(registry, output.kind, input.kind))
                .with_fixes(self.others_that_fit(registry, to, input.kind))
        })?;
        if self.feeds(to.node, from.node) {
            return Err(Refusal::new(trf!(
                "That would make a loop: {} already uses what {} makes, so it would wait for \
                 itself.",
                source.name.show(),
                target.name.show()
            ))
            .with_fixes(self.others_that_fit(registry, to, input.kind)));
        }
        Ok(how)
    }

    /// Outputs in the graph that fit input `to` (of kind `kind`) exactly
    /// without making a loop, nearest first.
    pub(crate) fn others_that_fit(
        &self,
        registry: &Registry,
        to: &PlugRef,
        kind: ValueKind,
    ) -> Vec<Fix> {
        let Some(here) = self.get(to.node).map(|b| b.at) else {
            return Vec::new();
        };
        let mut found: Vec<(f32, PlugRef)> = self
            .boxes()
            .filter(|(id, _)| *id != to.node && !self.feeds(to.node, *id))
            .filter_map(|(id, b)| {
                let info = registry.get(&b.kind)?.info();
                let out = info
                    .outputs
                    .iter()
                    .find(|o| fit(o.kind, kind) == Ok(Fit::Fits))?;
                Some((b.at.distance(here), PlugRef::new(id, out.key.to_string())))
            })
            .filter(|(_, from)| self.wire_into(to).is_none_or(|w| w.from != *from))
            .collect();
        found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        found
            .into_iter()
            .take(MOST_FIXES)
            .map(|(_, from)| Fix::ConnectInstead { from })
            .collect()
    }
}

/// Kinds of box that take `from` as it is and give `to` as it is: what
/// could go between two plugs that don't fit. A box that also takes `to`
/// itself only passes that input on (Transform's material, Edge
/// highlight's colour), so it can't turn `from` into `to` and isn't
/// offered. `from` may go into any of a box's inputs (a colour into
/// Material from height's Colour, milestone 27); boxes whose first input
/// takes it come first.
pub(crate) fn boxes_between(registry: &Registry, from: ValueKind, to: ValueKind) -> Vec<Fix> {
    let takes = |i: &crate::registry::PlugSpec| fit(from, i.kind) == Ok(Fit::Fits);
    let mut found: Vec<(bool, &crate::registry::BoxInfo)> = registry
        .iter()
        .map(|t| t.info())
        .filter(|info| !info.one_per_graph)
        .filter(|info| info.inputs.iter().all(|i| fit(to, i.kind) != Ok(Fit::Fits)))
        .filter(|info| {
            info.inputs.iter().any(takes)
                && info
                    .outputs
                    .iter()
                    .any(|o| fit(o.kind, to) == Ok(Fit::Fits))
        })
        .map(|info| (!info.inputs.first().is_some_and(takes), info))
        .collect();
    // Stable: the registry's order within each.
    found.sort_by_key(|(later, _)| *later);
    found
        .into_iter()
        .take(MOST_FIXES)
        .map(|(_, info)| Fix::InsertBox {
            kind: info.id.to_string(),
        })
        .collect()
}

/// Kinds of box that make `kind` from nothing (no needed inputs): what
/// could feed an empty plug.
pub(crate) fn makers_of(registry: &Registry, kind: ValueKind) -> Vec<Fix> {
    registry
        .iter()
        .filter(|t| !t.needs_an_input())
        .map(|t| t.info())
        .filter(|info| !info.one_per_graph && info.inputs.iter().all(|i| !i.needed))
        .filter(|info| {
            info.outputs
                .iter()
                .any(|o| fit(o.kind, kind) == Ok(Fit::Fits))
        })
        .take(MOST_FIXES)
        .map(|info| Fix::AddBox {
            kind: info.id.to_string(),
        })
        .collect()
}
