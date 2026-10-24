//! Edge layers holding a number from 0 to 1 per edge: creases
//! ([`crate::crease`], decision 0143) and bevel weights
//! ([`crate::bevel_weight`], decision 0149). Both work the same way, so
//! they share this code.
//!
//! A layer is there only while some edge has a value above 0, and is
//! carried to new edges by tools like hard edges and seams are
//! ([`crate::builder::Builder`]). A layer of the same name but another
//! kind (from an imported file) is left alone and reads as 0.

use crate::attributes::{AttributeElement, AttributeValues, Domain, Value};
use crate::edit::Outcome;
use crate::{EdgeKey, Mesh};

/// A value brought within 0 to 1; one that isn't a real number is 0.
pub(crate) fn usable(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The value of `edge` in `layer`: 0 for none (or an edge not in the
/// mesh).
pub(crate) fn weight_of(mesh: &Mesh, layer: &str, edge: EdgeKey) -> f32 {
    match mesh.attribute(layer, AttributeElement::Edge(edge)) {
        Some(Value::Float(c)) => usable(c),
        _ => 0.0,
    }
}

/// Every edge with a value above 0 in `layer`, in edge order. Values left
/// on edges the mesh no longer has are included; callers that care check
/// the topology.
pub(crate) fn weights(mesh: &Mesh, layer: &str) -> Vec<(EdgeKey, f32)> {
    match mesh.attribute_values(layer) {
        Some((Domain::Edge, _, AttributeValues::Edges(list))) => list
            .into_iter()
            .filter_map(|(e, v)| match v {
                Value::Float(c) if usable(c) > 0.0 => Some((e, usable(c))),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Gives `edges` the value `value` in `layer` (0 takes it away; brought
/// within 0 to 1, and a value that isn't a real number counts as 0). The
/// layer is added when needed and taken away when no edge has a value
/// left. Edges not in the mesh (no polygon uses them) are missing; edges
/// already so are skipped.
pub(crate) fn set_weights(mesh: &mut Mesh, layer: &str, edges: &[EdgeKey], value: f32) -> Outcome {
    let value = usable(value);
    let mut outcome = Outcome::default();
    let has_layer = mesh.attribute_values(layer).is_some();
    if !has_layer
        && value > 0.0
        && mesh
            .add_attribute(layer, Domain::Edge, Value::Float(0.0))
            .is_err()
    {
        // The name is taken by a layer of another kind (an imported file).
        outcome.skipped = edges.len();
        return outcome;
    }
    let topology = mesh.topology().clone();
    for edge in edges {
        if !topology.has_edge(*edge) {
            outcome.missing += 1;
        } else if weight_of(mesh, layer, *edge) == value || !has_number_layer(mesh, layer) {
            outcome.skipped += 1;
        } else if mesh
            .set_attribute(layer, AttributeElement::Edge(*edge), Value::Float(value))
            .is_ok()
        {
            outcome.matched += 1;
        } else {
            outcome.skipped += 1;
        }
    }
    if has_number_layer(mesh, layer) && weights(mesh, layer).is_empty() {
        mesh.remove_attribute(layer);
    }
    outcome
}

/// Whether the mesh has `layer` as an edge layer of numbers.
fn has_number_layer(mesh: &Mesh, layer: &str) -> bool {
    matches!(
        mesh.attribute_values(layer),
        Some((Domain::Edge, Value::Float(_), _))
    )
}
