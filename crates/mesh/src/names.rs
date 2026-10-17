//! Stable element names ([decision 0090](../../../docs/decisions/0090-stable-element-names.md)):
//! a name for each point and polygon that says where it came from, not
//! when it was made, so an edit can find its elements again after an
//! earlier edit changes how many elements it makes.
//!
//! Ids are numbers handed out in order: change an earlier bevel's segments
//! and every point made after it gets another number. Names don't count:
//!
//! - **The base shape's points** are named after where they sit in its box
//!   ([`Mesh::name_base`]), so a corner keeps its name when the shape is
//!   resized or given more segments.
//! - **A point an edit makes** is named after the edit (its id, the
//!   "salt") and where the point came from: a copy of a point, a spot
//!   between two points, a spot inside a polygon (the builder's
//!   [`crate::builder::PointFrom`]), by those points' and polygons' own
//!   names. Several points from the same origin in one edit are told
//!   apart by their order among those alone.
//! - **A polygon** is named after its corners' names, in order from the
//!   lowest. A polygon an edit rewrites keeps its name.
//!
//! Names are bookkeeping: two meshes that differ only in their names are
//! equal. They are worked out again whenever a shape is worked out from
//! its edits, never saved with a shape.

use crate::{FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::HashMap;
use std::sync::Arc;

/// The names of a mesh's points and polygons, by slot; 0 where none was
/// given.
#[derive(Debug, Clone, Default)]
pub struct Names {
    points: Arc<Vec<u64>>,
    faces: Arc<Vec<u64>>,
    /// The edit naming new elements now, if any.
    session: Option<Session>,
    /// Whether [`Mesh::name_base`] named it. A mesh never named (a plain
    /// mesh, whose ids never change) has its points named after their ids,
    /// trusted.
    named: bool,
}

impl Names {
    /// Whether an edit is naming the elements it makes.
    pub(crate) fn naming(&self) -> bool {
        self.session.is_some()
    }
}

/// An edit naming the elements it makes.
#[derive(Debug, Clone, Default)]
struct Session {
    salt: u64,
    /// How many elements each origin has named so far.
    seen: HashMap<u64, u32>,
}

/// Where a point an edit makes came from, for its name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Origin {
    /// Made from nothing in particular.
    Plain,
    /// A copy of a point (by its name).
    Copy(u64),
    /// A spot between two points (their names) `t` of the way.
    Between(u64, u64, f32),
    /// A spot inside a polygon (its name).
    Inside(u64, Vec3),
}

/// Mixes two numbers into one (SplitMix64's finaliser on their sum): the
/// same on every computer, and a small change gives a quite different
/// result.
pub fn mix(a: u64, b: u64) -> u64 {
    let mut z = a.wrapping_add(b.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    let out = z ^ (z >> 31);
    // 0 means "no name".
    if out == 0 { 1 } else { out }
}

const BASE: u64 = 0xba5e;
const PLAIN: u64 = 0x91a1;
const COPY: u64 = 0xc0b1;
const BETWEEN: u64 = 0xbe7e;
const INSIDE: u64 = 0x1251;
const FACE: u64 = 0xface;

impl Origin {
    fn key(self) -> u64 {
        match self {
            Self::Plain => PLAIN,
            Self::Copy(a) => mix(COPY, a),
            Self::Between(a, b, t) => mix(mix(mix(BETWEEN, a), b), u64::from(t.to_bits())),
            Self::Inside(face, spot) => {
                let bits = |x: f32| u64::from(x.to_bits());
                mix(
                    mix(mix(mix(INSIDE, face), bits(spot.x)), bits(spot.y)),
                    bits(spot.z),
                )
            }
        }
    }
}

/// How finely the base shape's points are told apart by place: a part of
/// its box this small.
const BASE_STEP: f32 = 1e-4;

impl Mesh {
    /// The name of point `v`, or `None` if it isn't in the mesh. A point
    /// never named gets a name from its id alone, which an edit finding
    /// its points again doesn't trust ([`Self::trusted_point_name`]).
    pub fn point_name(&self, v: VertexId) -> Option<u64> {
        self.position(v)?;
        Some(
            self.trusted_point_name(v)
                .unwrap_or_else(|| mix(mix(BASE, u64::from(v.get())), PLAIN)),
        )
    }

    /// The name of point `v` if it was given one (by [`Self::name_base`] or
    /// an edit), so it says where the point came from.
    pub fn trusted_point_name(&self, v: VertexId) -> Option<u64> {
        self.position(v)?;
        match self.names.points.get(v.get() as usize).copied() {
            Some(n) if n != 0 => Some(n),
            _ if self.plain_names() => Some(slot_name(v)),
            _ => None,
        }
    }

    /// The name of polygon `f`, or `None` if it isn't in the mesh: its own
    /// if it was given one, otherwise one from its corners' names.
    pub fn face_name(&self, f: FaceId) -> Option<u64> {
        let corners = self.face(f)?;
        Some(
            self.names
                .faces
                .get(f.get() as usize)
                .copied()
                .filter(|n| *n != 0)
                .unwrap_or_else(|| self.corner_key(corners)),
        )
    }

    /// A polygon's key from its corners' names: in order, starting from
    /// the lowest name, so it doesn't depend on which corner is first.
    fn corner_key(&self, corners: &[VertexId]) -> u64 {
        let names: Vec<u64> = corners
            .iter()
            .map(|v| self.point_name(*v).unwrap_or(0))
            .collect();
        let start = (0..names.len()).min_by_key(|i| names[*i]).unwrap_or(0);
        (0..names.len()).fold(FACE, |key, i| mix(key, names[(start + i) % names.len()]))
    }

    /// Names every point after where it sits in the shape's box, and every
    /// polygon after its corners: what a base shape (a primitive, or a
    /// plain mesh) is named before its edits are worked out. Two points on
    /// the same spot are told apart by their order.
    pub fn name_base(&mut self) {
        let Some(bounds) = self.bounds() else {
            return;
        };
        let size = bounds.size().max(Vec3::splat(1e-6));
        let mut seen: HashMap<u64, u32> = HashMap::new();
        let mut points = vec![0; self.point_slot_count()];
        for (v, p) in self.vertices() {
            let at = ((p - bounds.min) / size / BASE_STEP).round();
            let key = mix(
                mix(mix(BASE, at.x as i64 as u64), at.y as i64 as u64),
                at.z as i64 as u64,
            );
            let n = seen.entry(key).or_insert(0);
            points[v.get() as usize] = mix(key, u64::from(*n));
            *n += 1;
        }
        self.names.points = Arc::new(points);
        self.names.named = true;
        let faces: Vec<u64> = self
            .face_slots()
            .map(|c| c.map_or(0, |c| self.corner_key(c)))
            .collect();
        self.names.faces = Arc::new(faces);
    }

    /// Whether the points are named after their ids: a mesh never named,
    /// whose ids are its points' names.
    fn plain_names(&self) -> bool {
        !self.names.named && self.names.points.is_empty()
    }

    /// Before an edit names a point of a mesh named after its ids: those
    /// names written out, so they stay its points' names.
    fn keep_plain_names(&mut self) {
        if !self.plain_names() {
            return;
        }
        let points: Vec<u64> = (0..self.point_slot_count())
            .map(|i| slot_name(VertexId::from_raw(i as u32)))
            .collect();
        self.names.points = Arc::new(points);
        self.names.named = true;
    }

    /// Starts naming the elements an edit makes, the edit's id mixed into
    /// every name (`salt`).
    pub fn begin_naming(&mut self, salt: u64) {
        self.names.session = Some(Session {
            salt,
            seen: HashMap::new(),
        });
    }

    /// Stops naming new elements.
    pub fn end_naming(&mut self) {
        self.names.session = None;
    }

    /// Names point `v`, just made, after where it came from (while an
    /// edit is naming).
    pub(crate) fn name_new_point(&mut self, v: VertexId, origin: Origin) {
        let Some(name) = self.next_name(origin.key()) else {
            return;
        };
        self.keep_plain_names();
        set(&mut self.names.points, v.get() as usize, name);
    }

    /// Names polygon `f`, just made, after its corners (while an edit is
    /// naming).
    pub(crate) fn name_new_face(&mut self, f: FaceId) {
        let Some(corners) = self.face(f) else {
            return;
        };
        let key = self.corner_key(corners);
        if let Some(name) = self.next_name(key) {
            set(&mut self.names.faces, f.get() as usize, name);
        }
    }

    /// The next name for `key` in the edit naming now: the edit's salt,
    /// the key and how many before it had the same key.
    fn next_name(&mut self, key: u64) -> Option<u64> {
        let session = self.names.session.as_mut()?;
        let n = session.seen.entry(key).or_insert(0);
        let name = mix(mix(session.salt, key), u64::from(*n));
        *n += 1;
        Some(name)
    }
}

/// The name of a point never named, from its id.
fn slot_name(v: VertexId) -> u64 {
    mix(mix(BASE, u64::from(v.get())), PLAIN)
}

/// Sets slot `i` of `names` to `name`, growing it as needed.
fn set(names: &mut Arc<Vec<u64>>, i: usize, name: u64) {
    let names = Arc::make_mut(names);
    if names.len() <= i {
        names.resize(i + 1, 0);
    }
    names[i] = name;
}

#[cfg(test)]
mod tests;
