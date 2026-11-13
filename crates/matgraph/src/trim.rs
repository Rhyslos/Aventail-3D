//! The project's trim rows, as material graphs see them (Phase 12,
//! milestone 26, [decision 0289](../../../docs/decisions/0289-trim-sheet-box.md)).
//!
//! A project's trim sheet (decision 0106) is a set of rows across its
//! texture square, made in the UV Editor's Trim menu; strips of a model
//! are laid onto them with Put on trim row. The scene hands a copy of the
//! rows to its graphs' registry ([`crate::Registry::trim_rows`],
//! [`crate::GraphStore::set_trim_rows`]), as it does the palettes, so the
//! **Trim sheet** box can lay one material on each row and the texture
//! lines up with the strips laid on it. Boxes reading them say so
//! ([`crate::BoxType::uses_trim_rows`]) and hash [`TrimRows::key`] into
//! their result's key, so changing a row makes them again.
//!
//! A graph whose Material out is fed by a Trim sheet box is a **trim sheet
//! graph** ([`is_trim_sheet`]): on a model it is laid once across the
//! texture square, never repeated by the real-world size, so its strips
//! stay on the rows.

use crate::graph::{BoxId, Graph, PlugRef};
use crate::registry::Registry;
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// One row of the project's trim sheet: a band across the texture from
/// `from` to `to`, as shares of its height **from the top** (0 the top, 1
/// the bottom), the way the texture's rows of pixels count.
#[derive(Debug, Clone, PartialEq)]
pub struct TrimBand {
    /// What the row holds ("Bolts", "Panel edge").
    pub name: String,
    /// Where it starts, from the top (0 to 1).
    pub from: f32,
    /// Where it ends (below `from`, at most 1).
    pub to: f32,
}

impl TrimBand {
    /// How tall it is, as a share of the texture's height.
    pub fn height(&self) -> f32 {
        self.to - self.from
    }
}

/// The project's trim rows, top to bottom, shared, with a key that
/// changes whenever any of them does.
#[derive(Debug, Clone)]
pub struct TrimRows {
    list: Arc<[TrimBand]>,
    key: u64,
}

impl Default for TrimRows {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl PartialEq for TrimRows {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.list == other.list
    }
}

impl TrimRows {
    /// The rows `list`, sorted top to bottom; rows that aren't usable
    /// (not a number, outside the texture, no height) are left out.
    pub fn new(list: impl IntoIterator<Item = TrimBand>) -> Self {
        let mut list: Vec<TrimBand> = list
            .into_iter()
            .filter(|r| {
                r.from.is_finite()
                    && r.to.is_finite()
                    && r.from >= 0.0
                    && r.to <= 1.0
                    && r.to > r.from
            })
            .collect();
        list.sort_by(|a, b| a.from.total_cmp(&b.from));
        let mut h = DefaultHasher::new();
        for r in &list {
            (&r.name, r.from.to_bits(), r.to.to_bits()).hash(&mut h);
        }
        Self {
            list: list.into(),
            key: h.finish(),
        }
    }

    /// Every row, top to bottom.
    pub fn iter(&self) -> impl Iterator<Item = &TrimBand> {
        self.list.iter()
    }

    /// Row `i` from the top.
    pub fn get(&self, i: usize) -> Option<&TrimBand> {
        self.list.get(i)
    }

    /// How many rows there are.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Changes whenever any row does (a name, where it lies).
    pub fn key(&self) -> u64 {
        self.key
    }
}

/// Whether working out `graph` reads the project's trim rows: a box that
/// says so, or one of your own holding one.
pub fn uses_trim_rows(registry: &Registry, graph: &Graph) -> bool {
    graph
        .boxes()
        .any(|(_, b)| registry.get(&b.kind).is_some_and(|k| k.uses_trim_rows()))
}

/// Whether `graph` is a trim sheet: a Trim sheet box (switched on) feeds
/// its Material out's material, straight or through other boxes. Such a
/// graph is laid once across a model's texture square, so its strips stay
/// on the project's trim rows.
pub fn is_trim_sheet(graph: &Graph) -> bool {
    let Some(out) = graph.first_of(crate::eval::MATERIAL_OUT) else {
        return false;
    };
    let Some(wire) = graph.wire_into(&PlugRef::new(out, "material")) else {
        return false;
    };
    let mut seen: BTreeSet<BoxId> = BTreeSet::new();
    let mut waiting = vec![wire.from.node];
    while let Some(id) = waiting.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(b) = graph.get(id) else { continue };
        if b.kind == crate::boxes::trim_sheet::TRIM_SHEET && !b.bypassed {
            return true;
        }
        waiting.extend(graph.sources_of(id));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(name: &str, from: f32, to: f32) -> TrimBand {
        TrimBand {
            name: name.into(),
            from,
            to,
        }
    }

    #[test]
    fn rows_are_sorted_from_the_top_and_bad_ones_left_out() {
        let rows = TrimRows::new([
            band("Low", 0.5, 1.0),
            band("High", 0.0, 0.25),
            band("Nothing", 0.3, 0.3),
            band("Outside", 0.9, 1.5),
            band("Broken", f32::NAN, 0.2),
        ]);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["High", "Low"]);
        assert_eq!(rows.len(), 2);
        assert!((rows.get(1).unwrap().height() - 0.5).abs() < 1e-6);
        assert!(TrimRows::default().is_empty());
    }

    #[test]
    fn the_key_follows_every_change() {
        let a = TrimRows::new([band("Bolts", 0.0, 0.5)]);
        let same = TrimRows::new([band("Bolts", 0.0, 0.5)]);
        assert_eq!(a, same);
        assert_eq!(a.key(), same.key());
        for other in [
            TrimRows::new([band("Bolt", 0.0, 0.5)]),
            TrimRows::new([band("Bolts", 0.0, 0.25)]),
            TrimRows::default(),
        ] {
            assert_ne!(a.key(), other.key());
        }
    }
}
