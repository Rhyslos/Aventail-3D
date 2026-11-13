//! The project's colour palettes, as material graphs see them (Phase 12,
//! milestone 19, [decision 0281](../../../docs/decisions/0281-palette-map.md)).
//!
//! The painter keeps a project's palettes (decision 0183); the scene hands
//! a copy to its graphs' registry ([`crate::Registry::palettes`],
//! [`crate::GraphStore::set_palettes`]), so every place a project's graph
//! is worked out (the canvas's pictures, the painter's graph fills,
//! exports) sees the same palettes. A box names a palette by its name
//! ([`crate::SettingKind::Palette`]); no name means the project's first.
//! Boxes using a palette hash [`Palettes::key`] into their result's key,
//! so changing a palette in the painter makes them again.

use crate::graph::{Graph, GraphBox};
use crate::refusal::{Fix, Refusal};
use crate::registry::Registry;
use crate::settings::{SettingKind, SettingValue};
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use words::trf;

/// The most palettes offered as fixes when one can't be found.
const MAX_OFFERED: usize = 6;

/// How deep own boxes are looked into for the palettes they use.
const MAX_DEPTH: usize = 32;

/// One named list of colours (sRGB bytes), in the order the painter
/// lists them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Palette {
    /// Its name in the project.
    pub name: String,
    /// Its colours, in order.
    pub colours: Vec<[u8; 3]>,
}

/// A project's palettes, shared, with a key that changes whenever any of
/// them does.
#[derive(Debug, Clone)]
pub struct Palettes {
    list: Arc<[Palette]>,
    key: u64,
}

impl Default for Palettes {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl PartialEq for Palettes {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.list == other.list
    }
}

impl Palettes {
    /// The palettes `list`, in order.
    pub fn new(list: impl IntoIterator<Item = Palette>) -> Self {
        let list: Arc<[Palette]> = list.into_iter().collect();
        let mut h = DefaultHasher::new();
        list.hash(&mut h);
        Self {
            list,
            key: h.finish(),
        }
    }

    /// Every palette, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Palette> {
        self.list.iter()
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Changes whenever any palette does (a name, a colour, the order).
    pub fn key(&self) -> u64 {
        self.key
    }

    /// The palette called `name`; with no name, the first.
    pub fn find(&self, name: &str) -> Option<&Palette> {
        if name.is_empty() {
            self.list.first()
        } else {
            self.list.iter().find(|p| p.name == name)
        }
    }

    /// The palette a box's setting `key` names (`name`), with at least
    /// one colour; or why not, in plain words, with fixes: the project's
    /// other palettes, or adding the starter palettes when it has none.
    pub fn usable(&self, box_name: &str, key: &str, name: &str) -> Result<&Palette, Refusal> {
        let others = |skip: &str| {
            self.list
                .iter()
                .filter(|p| p.name != skip && !p.colours.is_empty())
                .take(MAX_OFFERED)
                .map(|p| Fix::UsePalette {
                    key: key.to_string(),
                    name: p.name.clone(),
                })
                .collect::<Vec<_>>()
        };
        if self.list.is_empty() {
            return Err(Refusal::new(trf!(
                "{} colours with one of the project's palettes, and this project has none yet. \
                 Add the starter palettes, or make one in the Paint mode's colour picker \
                 (Palette, then New palette).",
                box_name
            ))
            .with_fixes([Fix::AddPalettes]));
        }
        let Some(palette) = self.find(name) else {
            return Err(Refusal::new(trf!(
                "The palette {} isn't in this project: it was deleted or renamed, or the box \
                 came from another project. Choose one this project has.",
                name
            ))
            .with_fixes(others(name)));
        };
        if palette.colours.is_empty() {
            return Err(Refusal::new(trf!(
                "The palette {} has no colours yet. Add some in the Paint mode's colour \
                 picker (+), or choose another.",
                palette.name
            ))
            .with_fixes(others(&palette.name)));
        }
        Ok(palette)
    }
}

/// Whether working out `graph` reads the project's palettes: a box with
/// a palette setting, or one of your own holding one.
pub fn uses_palettes(registry: &Registry, graph: &Graph) -> bool {
    graph
        .boxes()
        .any(|(_, b)| registry.get(&b.kind).is_some_and(|k| k.uses_palettes()))
}

/// The palettes `graph`'s boxes name (not the unnamed "first palette"),
/// looking inside your own boxes: what has to come along when they are
/// pasted into another project.
pub fn names_used(registry: &Registry, graph: &Graph) -> BTreeSet<String> {
    names_in(registry, graph.boxes().map(|(_, b)| b))
}

/// The palettes `boxes` name (not the unnamed "first palette"), looking
/// inside your own boxes: boxes copied on the canvas bring these along.
pub fn names_in<'b>(
    registry: &Registry,
    boxes: impl IntoIterator<Item = &'b GraphBox>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let boxes: Vec<&GraphBox> = boxes.into_iter().collect();
    walk(registry, &boxes, 0, &mut out);
    out
}

fn walk(registry: &Registry, boxes: &[&GraphBox], depth: usize, out: &mut BTreeSet<String>) {
    if depth > MAX_DEPTH {
        return;
    }
    for b in boxes {
        let Some(kind) = registry.get(&b.kind) else {
            continue;
        };
        let info = kind.info();
        for (spec, value) in info
            .settings
            .iter()
            .zip(b.settings.resolved(&info.settings))
        {
            if let (SettingKind::Palette { .. }, SettingValue::Palette(name)) = (&spec.kind, value)
                && !name.is_empty()
            {
                out.insert(name);
            }
        }
        if let Some(own) = kind.own() {
            let inside: Vec<&GraphBox> = own.inner.boxes().map(|(_, b)| b).collect();
            walk(registry, &inside, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette(name: &str, colours: &[[u8; 3]]) -> Palette {
        Palette {
            name: name.into(),
            colours: colours.to_vec(),
        }
    }

    #[test]
    fn a_palette_is_found_by_name_and_no_name_is_the_first() {
        let all = Palettes::new([palette("Wood", &[[1, 2, 3]]), palette("Moss", &[[0, 9, 0]])]);
        assert_eq!(all.find("").unwrap().name, "Wood");
        assert_eq!(all.find("Moss").unwrap().name, "Moss");
        assert!(all.find("moss").is_none());
        assert!(Palettes::default().find("").is_none());
    }

    #[test]
    fn the_key_follows_every_change() {
        let a = Palettes::new([palette("Wood", &[[1, 2, 3]])]);
        let b = Palettes::new([palette("Wood", &[[1, 2, 4]])]);
        let c = Palettes::new([palette("Wod", &[[1, 2, 3]])]);
        let same = Palettes::new([palette("Wood", &[[1, 2, 3]])]);
        assert_eq!(a.key(), same.key());
        assert_eq!(a, same);
        assert_ne!(a.key(), b.key());
        assert_ne!(a.key(), c.key());
        assert_ne!(a.key(), Palettes::default().key());
    }

    #[test]
    fn a_missing_or_empty_palette_is_refused_with_fixes() {
        let none = Palettes::default()
            .usable("Palette map", "palette", "")
            .unwrap_err();
        assert_eq!(none.fixes, [Fix::AddPalettes]);
        let all = Palettes::new([
            palette("Wood", &[[1, 2, 3]]),
            palette("Empty", &[]),
            palette("Moss", &[[0, 9, 0]]),
        ]);
        let gone = all.usable("Palette map", "palette", "Stone").unwrap_err();
        assert!(gone.reason.contains("Stone"), "{}", gone.reason);
        // Empty palettes aren't offered.
        assert_eq!(
            gone.fixes,
            [
                Fix::UsePalette {
                    key: "palette".into(),
                    name: "Wood".into()
                },
                Fix::UsePalette {
                    key: "palette".into(),
                    name: "Moss".into()
                }
            ]
        );
        let empty = all.usable("Palette map", "palette", "Empty").unwrap_err();
        assert!(empty.reason.contains("no colours"), "{}", empty.reason);
        assert_eq!(empty.fixes.len(), 2);
        assert_eq!(all.usable("x", "palette", "Moss").unwrap().name, "Moss");
        assert_eq!(all.usable("x", "palette", "").unwrap().name, "Wood");
        // At most a few are offered.
        let many = Palettes::new((0..20).map(|i| palette(&format!("P{i}"), &[[0, 0, 0]])));
        assert_eq!(
            many.usable("x", "palette", "Gone").unwrap_err().fixes.len(),
            MAX_OFFERED
        );
    }
}
