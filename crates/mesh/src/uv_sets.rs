//! Several UV sets on one shape ([decision 0241](../../../docs/decisions/0241-uv-sets.md)).
//!
//! The **active** set is always the `uv` corner layer, so every UV tool
//! works on it unchanged. The others are kept as corner layers named
//! `uvset:` and their name (`uvset:lightmap`); the active set's own name
//! is kept by an empty polygon layer named `uvname:` and its name (none:
//! [`STANDARD_NAME`]). Making a set active swaps its values with `uv`'s.
//! A set's name is up to [`MAX_NAME`] letters, digits, spaces, `-` and
//! `_`.

use crate::Mesh;
use crate::attributes::{AttributeKind, AttributeValues, Domain, UV, Value};
use crate::edit::Outcome;
use glam::Vec2;

/// The prefix of an inactive set's layer.
const SET_PREFIX: &str = "uvset:";
/// The prefix of the active set's name marker.
const NAME_PREFIX: &str = "uvname:";
/// The active set's name when it hasn't been given one.
pub const STANDARD_NAME: &str = "UVMap";
/// The longest name.
pub const MAX_NAME: usize = 32;
/// The most sets on one shape.
pub const MAX_SETS: usize = 8;
/// The name of the set made for baked light (decision 0242).
pub const LIGHTMAP_NAME: &str = "lightmap";

/// Whether `name` can name a set.
pub fn is_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= MAX_NAME
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
}

/// The active set's name.
pub fn active_name(mesh: &Mesh) -> String {
    mesh.attributes()
        .find_map(|(n, _, _)| n.strip_prefix(NAME_PREFIX).map(str::to_owned))
        .unwrap_or_else(|| STANDARD_NAME.to_owned())
}

/// The other sets' names, in name order.
pub fn other_names(mesh: &Mesh) -> Vec<String> {
    let mut out: Vec<String> = mesh
        .attributes()
        .filter(|(_, d, k)| *d == Domain::Corner && *k == AttributeKind::Vec2)
        .filter_map(|(n, _, _)| n.strip_prefix(SET_PREFIX).map(str::to_owned))
        .collect();
    out.sort();
    out
}

/// Every set's name, the active one first (empty without UVs).
pub fn names(mesh: &Mesh) -> Vec<String> {
    if !crate::uvs::has_uvs(mesh) {
        return Vec::new();
    }
    let mut out = vec![active_name(mesh)];
    out.extend(other_names(mesh));
    out
}

/// A set's UVs, one per corner (the active set's, or another's), if the
/// shape has it.
pub fn values(mesh: &Mesh, name: &str) -> Option<Vec<Vec2>> {
    let layer = if name == active_name(mesh) {
        UV.to_owned()
    } else {
        format!("{SET_PREFIX}{name}")
    };
    let count = mesh.face_layout().1.len();
    match mesh.attribute_values(&layer) {
        Some((Domain::Corner, _, AttributeValues::Vec2(mut list))) => {
            list.resize(count, Vec2::ZERO);
            Some(list)
        }
        _ => None,
    }
}

/// The first set that isn't active, the lightmap first: the one exported
/// as the second UV set (glTF `TEXCOORD_1`, Godot's UV2).
pub fn second(mesh: &Mesh) -> Option<String> {
    let others = other_names(mesh);
    if others.iter().any(|n| n == LIGHTMAP_NAME) {
        return Some(LIGHTMAP_NAME.to_owned());
    }
    others.into_iter().next()
}

fn write_other(mesh: &mut Mesh, name: &str, values: Vec<Vec2>) -> bool {
    let layer = format!("{SET_PREFIX}{name}");
    mesh.set_attribute_values(
        &layer,
        Domain::Corner,
        Value::Vec2(Vec2::ZERO),
        AttributeValues::Vec2(values),
    )
    .is_ok()
}

fn set_active_name(mesh: &mut Mesh, name: &str) {
    let old: Vec<String> = mesh
        .attributes()
        .filter(|(n, _, _)| n.starts_with(NAME_PREFIX))
        .map(|(n, _, _)| n.to_owned())
        .collect();
    for n in old {
        mesh.remove_attribute(&n);
    }
    if name != STANDARD_NAME {
        let _ = mesh.add_attribute(
            &format!("{NAME_PREFIX}{name}"),
            Domain::Face,
            Value::Bool(false),
        );
    }
}

/// Writes `values` (one UV per corner) as set `name`: the active set's
/// UVs if it is active, else that set, made if needed (within
/// [`MAX_SETS`]). Returns whether it was written.
pub fn write(mesh: &mut Mesh, name: &str, values: Vec<Vec2>) -> bool {
    if !is_name(name) || values.len() != mesh.face_layout().1.len() {
        return false;
    }
    if name == active_name(mesh) && crate::uvs::has_uvs(mesh) {
        return crate::uvs::set_uvs(mesh, values);
    }
    if !names(mesh).iter().any(|n| n == name) && names(mesh).len() >= MAX_SETS {
        return false;
    }
    write_other(mesh, name, values)
}

/// Whether `write` could write set `name` on `mesh` now: the set is
/// there already, or there is room for one more (fewer than
/// [`MAX_SETS`]). For saying so before recording an edit that would
/// change nothing.
pub fn has_room_for(mesh: &Mesh, name: &str) -> bool {
    is_name(name) && (has(mesh, name) || names(mesh).len() < MAX_SETS)
}

/// Whether making `change` on `mesh` would change anything (it is tried
/// on a copy).
pub fn would_change(mesh: &Mesh, set_change: &SetChange) -> bool {
    let mut copy = mesh.clone();
    change(&mut copy, set_change).matched > 0
}

/// A change to a shape's UV sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetChange {
    /// A new set called this, a copy of the active one's UVs (so it starts
    /// laid out).
    Add(String),
    /// A copy of set `from` called `to`.
    Copy {
        /// The set copied.
        from: String,
        /// The copy's name.
        to: String,
    },
    /// Set `from` renamed `to`.
    Rename {
        /// Its name now.
        from: String,
        /// Its new name.
        to: String,
    },
    /// Set called this deleted (the active one only when another can take
    /// its place: the first other set).
    Delete(String),
    /// Set called this made the active one, worked on by every UV tool.
    MakeActive(String),
}

impl SetChange {
    /// Whether its names are names.
    pub fn is_real(&self) -> bool {
        match self {
            Self::Add(n) | Self::Delete(n) | Self::MakeActive(n) => is_name(n),
            Self::Copy { from, to } | Self::Rename { from, to } => is_name(from) && is_name(to),
        }
    }
}

fn has(mesh: &Mesh, name: &str) -> bool {
    names(mesh).iter().any(|n| n == name)
}

/// Makes `change` on `mesh`'s UV sets. Nothing changes (and it's
/// counted skipped) when a name is taken or missing, the shape has no UVs,
/// or it would leave too many or no sets.
pub fn change(mesh: &mut Mesh, change: &SetChange) -> Outcome {
    let skipped = Outcome {
        matched: 0,
        missing: 0,
        skipped: 1,
    };
    let done = Outcome {
        matched: 1,
        missing: 0,
        skipped: 0,
    };
    if !change.is_real() || !crate::uvs::has_uvs(mesh) {
        return skipped;
    }
    let active = active_name(mesh);
    match change {
        SetChange::Add(name) => {
            if has(mesh, name) || names(mesh).len() >= MAX_SETS {
                return skipped;
            }
            let Some(uv) = values(mesh, &active) else {
                return skipped;
            };
            if write_other(mesh, name, uv) {
                done
            } else {
                skipped
            }
        }
        SetChange::Copy { from, to } => {
            if has(mesh, to) || names(mesh).len() >= MAX_SETS {
                return skipped;
            }
            let Some(uv) = values(mesh, from) else {
                return skipped;
            };
            if write_other(mesh, to, uv) {
                done
            } else {
                skipped
            }
        }
        SetChange::Rename { from, to } => {
            if !has(mesh, from) || has(mesh, to) {
                return skipped;
            }
            if *from == active {
                set_active_name(mesh, to);
                return done;
            }
            let Some(uv) = values(mesh, from) else {
                return skipped;
            };
            mesh.remove_attribute(&format!("{SET_PREFIX}{from}"));
            if write_other(mesh, to, uv) {
                done
            } else {
                skipped
            }
        }
        SetChange::Delete(name) => {
            if !has(mesh, name) {
                return skipped;
            }
            if *name != active {
                mesh.remove_attribute(&format!("{SET_PREFIX}{name}"));
                return done;
            }
            // The active set goes: the first other takes its place.
            let Some(next) = other_names(mesh).into_iter().next() else {
                return skipped;
            };
            let Some(uv) = values(mesh, &next) else {
                return skipped;
            };
            mesh.remove_attribute(&format!("{SET_PREFIX}{next}"));
            if !crate::uvs::set_uvs(mesh, uv) {
                return skipped;
            }
            set_active_name(mesh, &next);
            done
        }
        SetChange::MakeActive(name) => {
            if *name == active {
                return skipped;
            }
            let (Some(theirs), Some(ours)) = (values(mesh, name), values(mesh, &active)) else {
                return skipped;
            };
            mesh.remove_attribute(&format!("{SET_PREFIX}{name}"));
            if !write_other(mesh, &active, ours) || !crate::uvs::set_uvs(mesh, theirs) {
                return skipped;
            }
            set_active_name(mesh, name);
            done
        }
    }
}

#[cfg(test)]
mod tests;
