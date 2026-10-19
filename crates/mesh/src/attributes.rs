//! Attribute layers: named values kept per point, edge, polygon or polygon
//! corner ([decision 0043](../../../docs/decisions/0043-attribute-layers-and-the-builder.md)).
//!
//! A mesh is more than where its points are. Hard edges, UV coordinates,
//! seams, material slots, vertex colours and creases are all values that
//! belong to elements. Each is a **layer** with a name, a [`Domain`] (what
//! it belongs to), a kind of value ([`AttributeKind`]) and a default:
//!
//! | Domain | One value per | Examples |
//! |---|---|---|
//! | [`Domain::Point`] | point | vertex colour, skin weights |
//! | [`Domain::Edge`] | edge | hard, seam, crease |
//! | [`Domain::Face`] | polygon | material slot |
//! | [`Domain::Corner`] | corner of a polygon | UVs, split normals |
//!
//! A polygon *corner* is a point as one polygon uses it: a point shared by
//! four polygons has four corners, which can each hold a different UV.
//!
//! **Storage.** Point, polygon and corner values are kept in typed lists by
//! id (corners in the order of the mesh's corner list), filled in only as
//! far as a value was set: anything beyond is the layer's default, so a
//! new point costs nothing until a value is set on it. Edges have no ids,
//! and most edge layers mark only a few edges, so edge values are kept by
//! the edge's two points, and only where they differ from the default.
//!
//! Layers are shared between copies of a mesh until one copy changes them,
//! like the rest of the mesh. Tools carry values over to the elements they
//! make through [`crate::builder::Builder`].

use crate::{EdgeKey, FaceId, VertexId};
use glam::{Vec2, Vec3, Vec4};
use std::collections::BTreeMap;
use std::sync::Arc;

/// What an attribute layer holds one value for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Domain {
    /// Each point.
    Point,
    /// Each edge, by its two points.
    Edge,
    /// Each polygon.
    Face,
    /// Each corner of each polygon.
    Corner,
}

/// The kind of value a layer holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AttributeKind {
    /// On or off (hard, seam).
    Bool,
    /// A whole number (material slot).
    Int,
    /// One number (crease).
    Float,
    /// Two numbers (UVs).
    Vec2,
    /// Three numbers (a direction).
    Vec3,
    /// Four numbers (a colour with transparency).
    Vec4,
}

/// One attribute value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// On or off.
    Bool(bool),
    /// A whole number.
    Int(i32),
    /// One number.
    Float(f32),
    /// Two numbers.
    Vec2(Vec2),
    /// Three numbers.
    Vec3(Vec3),
    /// Four numbers.
    Vec4(Vec4),
}

impl Value {
    /// The kind of value this is.
    pub fn kind(self) -> AttributeKind {
        match self {
            Self::Bool(_) => AttributeKind::Bool,
            Self::Int(_) => AttributeKind::Int,
            Self::Float(_) => AttributeKind::Float,
            Self::Vec2(_) => AttributeKind::Vec2,
            Self::Vec3(_) => AttributeKind::Vec3,
            Self::Vec4(_) => AttributeKind::Vec4,
        }
    }

    /// Whether every number in it is a real number (not infinite or NaN).
    pub fn is_finite(self) -> bool {
        match self {
            Self::Bool(_) | Self::Int(_) => true,
            Self::Float(x) => x.is_finite(),
            Self::Vec2(v) => v.is_finite(),
            Self::Vec3(v) => v.is_finite(),
            Self::Vec4(v) => v.is_finite(),
        }
    }

    /// `t` of the way from `a` to `b`: numbers blend, and on/off values and
    /// whole numbers take whichever end is nearer. Values of different
    /// kinds give `a`.
    pub fn mix(a: Self, b: Self, t: f32) -> Self {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        match (a, b) {
            (Self::Float(x), Self::Float(y)) => Self::Float(x + (y - x) * t),
            (Self::Vec2(x), Self::Vec2(y)) => Self::Vec2(x.lerp(y, t)),
            (Self::Vec3(x), Self::Vec3(y)) => Self::Vec3(x.lerp(y, t)),
            (Self::Vec4(x), Self::Vec4(y)) => Self::Vec4(x.lerp(y, t)),
            (Self::Bool(_), Self::Bool(_)) | (Self::Int(_), Self::Int(_)) => {
                if t < 0.5 {
                    a
                } else {
                    b
                }
            }
            _ => a,
        }
    }
}

/// An element a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeElement {
    /// A point.
    Point(VertexId),
    /// An edge, by its two points.
    Edge(EdgeKey),
    /// A polygon.
    Face(FaceId),
    /// A polygon's corner: the polygon, and which corner (0 for the first).
    Corner(FaceId, usize),
}

impl AttributeElement {
    /// The domain of layers that hold values for this element.
    pub fn domain(self) -> Domain {
        match self {
            Self::Point(_) => Domain::Point,
            Self::Edge(_) => Domain::Edge,
            Self::Face(_) => Domain::Face,
            Self::Corner(..) => Domain::Corner,
        }
    }
}

/// Why an attribute couldn't be added or set.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeError {
    /// A layer of that name already exists (with another domain or kind).
    NameTaken(String),
    /// No layer has that name.
    Unknown(String),
    /// The element is of another domain than the layer's.
    WrongDomain {
        /// The layer's domain.
        layer: Domain,
        /// The element's domain.
        element: Domain,
    },
    /// The value is of another kind than the layer's.
    WrongKind {
        /// The layer's kind.
        layer: AttributeKind,
        /// The value's kind.
        value: AttributeKind,
    },
    /// The element isn't in the mesh.
    NoSuchElement,
    /// A value that isn't a real number.
    NotANumber,
}

impl std::fmt::Display for AttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NameTaken(name) => write!(f, "an attribute called \"{name}\" already exists"),
            Self::Unknown(name) => write!(f, "there is no attribute called \"{name}\""),
            Self::WrongDomain { layer, element } => write!(
                f,
                "the attribute holds values for each {layer:?}, not each {element:?}"
            ),
            Self::WrongKind { layer, value } => {
                write!(f, "the attribute holds {layer:?} values, not {value:?}")
            }
            Self::NoSuchElement => write!(f, "that element isn't in the mesh"),
            Self::NotANumber => write!(f, "the value isn't a real number"),
        }
    }
}

impl std::error::Error for AttributeError {}

/// A whole layer's values, for saving and opening: by id (corners in the
/// order of the mesh's corner list) as far as any was set, or by edge.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValues {
    /// On/off values by id.
    Bool(Vec<bool>),
    /// Whole numbers by id.
    Int(Vec<i32>),
    /// Numbers by id.
    Float(Vec<f32>),
    /// Pairs of numbers by id.
    Vec2(Vec<Vec2>),
    /// Three numbers by id.
    Vec3(Vec<Vec3>),
    /// Four numbers by id.
    Vec4(Vec<Vec4>),
    /// Values by edge, where not the default.
    Edges(Vec<(EdgeKey, Value)>),
}

impl AttributeValues {
    /// How many values are listed.
    pub fn len(&self) -> usize {
        match self {
            Self::Bool(v) => v.len(),
            Self::Int(v) => v.len(),
            Self::Float(v) => v.len(),
            Self::Vec2(v) => v.len(),
            Self::Vec3(v) => v.len(),
            Self::Vec4(v) => v.len(),
            Self::Edges(v) => v.len(),
        }
    }

    /// Whether no values are listed.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The kind of the listed values (`None` for edges, which say it per
    /// value).
    fn kind(&self) -> Option<AttributeKind> {
        Some(match self {
            Self::Bool(_) => AttributeKind::Bool,
            Self::Int(_) => AttributeKind::Int,
            Self::Float(_) => AttributeKind::Float,
            Self::Vec2(_) => AttributeKind::Vec2,
            Self::Vec3(_) => AttributeKind::Vec3,
            Self::Vec4(_) => AttributeKind::Vec4,
            Self::Edges(_) => return None,
        })
    }

    /// Whether every listed number is a real number.
    fn is_finite(&self) -> bool {
        match self {
            Self::Bool(_) | Self::Int(_) => true,
            Self::Float(v) => v.iter().all(|x| x.is_finite()),
            Self::Vec2(v) => v.iter().all(|x| x.is_finite()),
            Self::Vec3(v) => v.iter().all(|x| x.is_finite()),
            Self::Vec4(v) => v.iter().all(|x| x.is_finite()),
            Self::Edges(v) => v.iter().all(|(_, x)| x.is_finite()),
        }
    }
}

impl From<Column> for AttributeValues {
    fn from(column: Column) -> Self {
        match column {
            Column::Bool(v) => Self::Bool(v),
            Column::Int(v) => Self::Int(v),
            Column::Float(v) => Self::Float(v),
            Column::Vec2(v) => Self::Vec2(v),
            Column::Vec3(v) => Self::Vec3(v),
            Column::Vec4(v) => Self::Vec4(v),
        }
    }
}

/// The name of the built-in edge layer marking hard edges.
pub const HARD: &str = "hard";

/// The name of the corner layer holding UV coordinates (two numbers per
/// polygon corner), when a mesh has them: from an imported file now, from
/// the UV tools later.
pub const UV: &str = "uv";

/// The name of the edge layer marking seams: edges the surface is cut open
/// along when it is unwrapped flat (decision 0095).
pub const SEAM: &str = "seam";

/// The name of the corner layer marking pinned UVs: corners that stay
/// where they are when their piece is unwrapped or relaxed (decision
/// 0109).
pub const UV_PIN: &str = "uv_pin";

/// A typed list of values.
#[derive(Debug, Clone, PartialEq)]
enum Column {
    Bool(Vec<bool>),
    Int(Vec<i32>),
    Float(Vec<f32>),
    Vec2(Vec<Vec2>),
    Vec3(Vec<Vec3>),
    Vec4(Vec<Vec4>),
}

impl Column {
    fn new(kind: AttributeKind) -> Self {
        match kind {
            AttributeKind::Bool => Self::Bool(Vec::new()),
            AttributeKind::Int => Self::Int(Vec::new()),
            AttributeKind::Float => Self::Float(Vec::new()),
            AttributeKind::Vec2 => Self::Vec2(Vec::new()),
            AttributeKind::Vec3 => Self::Vec3(Vec::new()),
            AttributeKind::Vec4 => Self::Vec4(Vec::new()),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Bool(v) => v.len(),
            Self::Int(v) => v.len(),
            Self::Float(v) => v.len(),
            Self::Vec2(v) => v.len(),
            Self::Vec3(v) => v.len(),
            Self::Vec4(v) => v.len(),
        }
    }

    fn get(&self, i: usize) -> Option<Value> {
        match self {
            Self::Bool(v) => v.get(i).copied().map(Value::Bool),
            Self::Int(v) => v.get(i).copied().map(Value::Int),
            Self::Float(v) => v.get(i).copied().map(Value::Float),
            Self::Vec2(v) => v.get(i).copied().map(Value::Vec2),
            Self::Vec3(v) => v.get(i).copied().map(Value::Vec3),
            Self::Vec4(v) => v.get(i).copied().map(Value::Vec4),
        }
    }

    /// Sets item `i` (of the column's kind), filling any gap with
    /// `default`. Returns whether the kinds matched.
    fn set(&mut self, i: usize, value: Value, default: Value) -> bool {
        fn put<T: Copy>(list: &mut Vec<T>, i: usize, value: T, default: T) {
            if list.len() <= i {
                list.resize(i + 1, default);
            }
            list[i] = value;
        }
        match (self, value, default) {
            (Self::Bool(l), Value::Bool(v), Value::Bool(d)) => put(l, i, v, d),
            (Self::Int(l), Value::Int(v), Value::Int(d)) => put(l, i, v, d),
            (Self::Float(l), Value::Float(v), Value::Float(d)) => put(l, i, v, d),
            (Self::Vec2(l), Value::Vec2(v), Value::Vec2(d)) => put(l, i, v, d),
            (Self::Vec3(l), Value::Vec3(v), Value::Vec3(d)) => put(l, i, v, d),
            (Self::Vec4(l), Value::Vec4(v), Value::Vec4(d)) => put(l, i, v, d),
            _ => return false,
        }
        true
    }

    /// A new column whose item `i` is this column's item `from[i]`, or
    /// `default` where that is `None` or past the end. Trailing defaults
    /// are left off.
    fn gathered(&self, from: &[Option<usize>], default: Value) -> Self {
        let mut out = Self::new(default.kind());
        let last = from.iter().rposition(|f| f.is_some_and(|i| i < self.len()));
        let Some(last) = last else {
            return out;
        };
        for (i, f) in from[..=last].iter().enumerate() {
            let value = f.and_then(|f| self.get(f)).unwrap_or(default);
            out.set(i, value, default);
        }
        out
    }

    fn memory_size(&self) -> usize {
        use std::mem::size_of;
        match self {
            Self::Bool(v) => v.capacity() * size_of::<bool>(),
            Self::Int(v) => v.capacity() * size_of::<i32>(),
            Self::Float(v) => v.capacity() * size_of::<f32>(),
            Self::Vec2(v) => v.capacity() * size_of::<Vec2>(),
            Self::Vec3(v) => v.capacity() * size_of::<Vec3>(),
            Self::Vec4(v) => v.capacity() * size_of::<Vec4>(),
        }
    }
}

/// One layer's values.
#[derive(Debug, Clone, PartialEq)]
enum Data {
    /// By point, polygon or flat corner index.
    Dense(Column),
    /// By edge, only where they differ from the default.
    Edges(BTreeMap<EdgeKey, Value>),
}

/// One attribute layer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layer {
    domain: Domain,
    default: Value,
    data: Data,
}

impl Layer {
    fn new(domain: Domain, default: Value) -> Self {
        let data = match domain {
            Domain::Edge => Data::Edges(BTreeMap::new()),
            _ => Data::Dense(Column::new(default.kind())),
        };
        Self {
            domain,
            default,
            data,
        }
    }

    pub(crate) fn domain(&self) -> Domain {
        self.domain
    }

    pub(crate) fn kind(&self) -> AttributeKind {
        self.default.kind()
    }

    pub(crate) fn default(&self) -> Value {
        self.default
    }

    /// The value at index `i` (point, polygon or flat corner index).
    pub(crate) fn at(&self, i: usize) -> Value {
        match &self.data {
            Data::Dense(c) => c.get(i).unwrap_or(self.default),
            Data::Edges(_) => self.default,
        }
    }

    /// Sets the value at index `i`. Values matching the default past the
    /// end aren't stored.
    pub(crate) fn set_at(&mut self, i: usize, value: Value) {
        let default = self.default;
        if let Data::Dense(c) = &mut self.data
            && (i < c.len() || value != default)
        {
            c.set(i, value, default);
        }
    }

    pub(crate) fn edge(&self, edge: EdgeKey) -> Value {
        match &self.data {
            Data::Edges(map) => map.get(&edge).copied().unwrap_or(self.default),
            Data::Dense(_) => self.default,
        }
    }

    pub(crate) fn set_edge(&mut self, edge: EdgeKey, value: Value) {
        let default = self.default;
        if let Data::Edges(map) = &mut self.data {
            if value == default {
                map.remove(&edge);
            } else {
                map.insert(edge, value);
            }
        }
    }

    /// The edges holding a value other than the default.
    pub(crate) fn edges(&self) -> impl Iterator<Item = (EdgeKey, Value)> + '_ {
        let map = match &self.data {
            Data::Edges(map) => Some(map),
            Data::Dense(_) => None,
        };
        map.into_iter().flatten().map(|(e, v)| (*e, *v))
    }

    /// How many edges hold a value other than the default.
    pub(crate) fn edge_count(&self) -> usize {
        match &self.data {
            Data::Edges(map) => map.len(),
            Data::Dense(_) => 0,
        }
    }

    /// Forgets edge values for which `gone` says yes.
    pub(crate) fn forget_edges(&mut self, gone: impl Fn(EdgeKey) -> bool) {
        if let Data::Edges(map) = &mut self.data {
            map.retain(|e, _| !gone(*e));
        }
    }

    /// Lays the values out again: new index `i` takes old index `from[i]`
    /// (the default where `None`).
    pub(crate) fn regather(&mut self, from: &[Option<usize>]) {
        let default = self.default;
        if let Data::Dense(c) = &mut self.data {
            *c = c.gathered(from, default);
        }
    }

    /// Its values, for saving.
    pub(crate) fn values(&self) -> AttributeValues {
        match &self.data {
            Data::Dense(c) => c.clone().into(),
            Data::Edges(map) => AttributeValues::Edges(map.iter().map(|(e, v)| (*e, *v)).collect()),
        }
    }

    /// A layer holding `values`, checked against its domain and kind
    /// (where each element exists is checked by the mesh).
    pub(crate) fn from_values(
        domain: Domain,
        default: Value,
        values: AttributeValues,
    ) -> Result<Self, AttributeError> {
        if !default.is_finite() || !values.is_finite() {
            return Err(AttributeError::NotANumber);
        }
        let mut layer = Self::new(domain, default);
        match (domain, values) {
            (Domain::Edge, AttributeValues::Edges(list)) => {
                for (edge, value) in list {
                    if value.kind() != default.kind() {
                        return Err(AttributeError::WrongKind {
                            layer: default.kind(),
                            value: value.kind(),
                        });
                    }
                    layer.set_edge(edge, value);
                }
            }
            // Values by id for an edge layer, or by edge for another.
            (Domain::Edge, _) => {
                return Err(AttributeError::WrongDomain {
                    layer: Domain::Edge,
                    element: Domain::Point,
                });
            }
            (_, AttributeValues::Edges(_)) => {
                return Err(AttributeError::WrongDomain {
                    layer: domain,
                    element: Domain::Edge,
                });
            }
            (_, values) => {
                let kind = values.kind().unwrap_or(default.kind());
                if kind != default.kind() {
                    return Err(AttributeError::WrongKind {
                        layer: default.kind(),
                        value: kind,
                    });
                }
                layer.data = Data::Dense(match values {
                    AttributeValues::Bool(v) => Column::Bool(v),
                    AttributeValues::Int(v) => Column::Int(v),
                    AttributeValues::Float(v) => Column::Float(v),
                    AttributeValues::Vec2(v) => Column::Vec2(v),
                    AttributeValues::Vec3(v) => Column::Vec3(v),
                    AttributeValues::Vec4(v) => Column::Vec4(v),
                    AttributeValues::Edges(_) => Column::new(kind),
                });
            }
        }
        Ok(layer)
    }

    /// How many values are stored by index (dense layers).
    pub(crate) fn stored(&self) -> usize {
        match &self.data {
            Data::Dense(c) => c.len(),
            Data::Edges(_) => 0,
        }
    }

    fn memory_size(&self) -> usize {
        size_of::<Self>()
            + match &self.data {
                Data::Dense(c) => c.memory_size(),
                // A B-tree node holds several entries plus links; twice the
                // entries is a fair estimate.
                Data::Edges(map) => map.len() * size_of::<(EdgeKey, Value)>() * 2,
            }
    }
}

/// A mesh's attribute layers, by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Attributes {
    layers: BTreeMap<String, Arc<Layer>>,
}

impl Attributes {
    pub(crate) fn get(&self, name: &str) -> Option<&Layer> {
        self.layers.get(name).map(Arc::as_ref)
    }

    /// The layer, to change (copied first if shared with another mesh).
    pub(crate) fn get_mut(&mut self, name: &str) -> Option<&mut Layer> {
        self.layers.get_mut(name).map(Arc::make_mut)
    }

    /// Adds an empty layer. Adding one that exists with the same domain and
    /// kind changes nothing (and keeps its values and default).
    pub(crate) fn add(
        &mut self,
        name: &str,
        domain: Domain,
        default: Value,
    ) -> Result<(), AttributeError> {
        if !default.is_finite() {
            return Err(AttributeError::NotANumber);
        }
        if let Some(layer) = self.get(name) {
            if layer.domain == domain && layer.kind() == default.kind() {
                return Ok(());
            }
            return Err(AttributeError::NameTaken(name.to_owned()));
        }
        self.layers
            .insert(name.to_owned(), Arc::new(Layer::new(domain, default)));
        Ok(())
    }

    /// Puts `layer` in under `name`, replacing any layer of that name.
    pub(crate) fn put(&mut self, name: &str, layer: Layer) {
        self.layers.insert(name.to_owned(), Arc::new(layer));
    }

    pub(crate) fn remove(&mut self, name: &str) -> bool {
        self.layers.remove(name).is_some()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &Layer)> + '_ {
        self.layers.iter().map(|(n, l)| (n.as_str(), l.as_ref()))
    }

    /// Every layer of `domain`, to change.
    pub(crate) fn of_domain_mut(
        &mut self,
        domain: Domain,
    ) -> impl Iterator<Item = &mut Layer> + '_ {
        self.layers
            .values_mut()
            .filter(move |l| l.domain == domain)
            .map(Arc::make_mut)
    }

    pub(crate) fn has_domain(&self, domain: Domain) -> bool {
        self.layers.values().any(|l| l.domain == domain)
    }

    /// Whether any edge layer holds a value.
    pub(crate) fn has_edge_values(&self) -> bool {
        self.layers.values().any(|l| l.edge_count() > 0)
    }

    /// Whether `other` holds the very same layers (shared, not just equal).
    pub(crate) fn shared_with(&self, other: &Self) -> bool {
        self.layers.len() == other.layers.len()
            && self
                .layers
                .iter()
                .zip(&other.layers)
                .all(|((n, a), (m, b))| n == m && Arc::ptr_eq(a, b))
    }

    pub(crate) fn memory_size(&self) -> usize {
        self.layers
            .iter()
            .map(|(n, l)| n.len() + l.memory_size())
            .sum()
    }
}

#[cfg(test)]
mod tests;
