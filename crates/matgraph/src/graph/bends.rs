//! Reroute dots (milestone 16): points a wire bends through on its way
//! from an output to an input, to steer long wires round boxes.
//!
//! A wire's dots are kept by the input it goes into (an input takes one
//! wire), in order from the output. They belong to that wire: when it is
//! taken out, replaced by a wire from another output, or one of its boxes
//! is removed, its dots go with it. Moving both of a wire's boxes moves
//! its dots along; the canvas's tidy layout keeps them between the two
//! ends so the wire never runs backwards.

use super::{Graph, MAX_BENDS, MAX_PLACE, PlugRef};
use crate::refusal::Refusal;
use glam::Vec2;
use std::collections::BTreeMap;
use words::{text, tr, trf};

/// Whether `point` is a place a dot may sit.
fn usable(point: Vec2) -> bool {
    point.is_finite() && point.abs().max_element() <= MAX_PLACE
}

impl Graph {
    /// The reroute dots the wire into input `to` bends through, in order
    /// from its output; none for a straight wire (or no wire).
    pub fn bends(&self, to: &PlugRef) -> &[Vec2] {
        self.bends.get(to).map_or(&[], Vec::as_slice)
    }

    /// Every wire's reroute dots, by the input it goes into.
    pub fn all_bends(&self) -> impl Iterator<Item = (&PlugRef, &[Vec2])> {
        self.bends.iter().map(|(to, p)| (to, p.as_slice()))
    }

    /// [`super::GraphEdit::SetBends`]: the wire into `to` bends through
    /// `points` (none: straight).
    pub(super) fn set_bends(&mut self, to: &PlugRef, points: &[Vec2]) -> Result<(), Refusal> {
        if self.wire_into(to).is_none() {
            return Err(Refusal::new(tr(
                "That wire isn't there any more, so it can't have reroute dots.",
            )));
        }
        if points.len() > MAX_BENDS {
            return Err(Refusal::new(trf!(
                "A wire can bend through {} reroute dots at most; remove one first.",
                MAX_BENDS
            )));
        }
        if !points.iter().all(|p| usable(*p)) {
            return Err(Refusal::new(tr(
                "That's too far from the rest of the graph.",
            )));
        }
        if points.is_empty() {
            self.bends.remove(to);
        } else {
            self.bends.insert(to.clone(), points.to_vec());
        }
        Ok(())
    }

    /// Drops the dots of wires no longer there.
    pub(super) fn drop_loose_bends(&mut self) {
        let wires = &self.wires;
        self.bends.retain(|to, _| wires.iter().any(|w| w.to == *to));
    }

    /// The dots of wires running between two of `ids`, moved by `by`; or
    /// why one can't be (nothing changes then).
    pub(super) fn move_bends(&mut self, ids: &[super::BoxId], by: Vec2) -> Result<(), Refusal> {
        let moving: Vec<PlugRef> = self
            .wires
            .iter()
            .filter(|w| ids.contains(&w.from.node) && ids.contains(&w.to.node))
            .filter(|w| self.bends.contains_key(&w.to))
            .map(|w| w.to.clone())
            .collect();
        let far = moving
            .iter()
            .flat_map(|to| self.bends(to))
            .any(|p| !usable(*p + by));
        if far {
            return Err(Refusal::new(tr(
                "That's too far from the rest of the graph.",
            )));
        }
        for to in moving {
            if let Some(points) = self.bends.get_mut(&to) {
                for p in points {
                    *p += by;
                }
            }
        }
        Ok(())
    }
}

/// A graph's dots from a file: each wire's, checked. Dots of a wire that
/// isn't there are dropped; too many, out of place or two lists for one
/// wire are damage.
pub(super) fn from_parts(
    graph: &Graph,
    parts: Vec<(PlugRef, Vec<Vec2>)>,
) -> Result<BTreeMap<PlugRef, Vec<Vec2>>, &'static str> {
    let mut out = BTreeMap::new();
    for (to, points) in parts {
        if points.len() > MAX_BENDS || !points.iter().all(|p| usable(*p)) {
            return Err(text!("a wire's reroute dots are damaged"));
        }
        if out.contains_key(&to) {
            return Err(text!("a wire's reroute dots are damaged"));
        }
        if graph.wire_into(&to).is_some() && !points.is_empty() {
            out.insert(to, points);
        }
    }
    Ok(out)
}
