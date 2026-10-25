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
    // Both checks are made once, before the loop: asking for the layer
    // copies it whole, so asking per edge made this quadratic (50,000
    // edges took seconds).
    let present: Vec<bool> = {
        let topology = mesh.topology();
        edges.iter().map(|e| topology.has_edge(*e)).collect()
    };
    let number_layer = has_number_layer(mesh, layer);
    for (edge, present) in edges.iter().zip(present) {
        if !present {
            outcome.missing += 1;
        } else if !number_layer || weight_of(mesh, layer, *edge) == value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Primitive, SphereParams, VertexId};

    fn sphere(sides: u32, rings: u32) -> Mesh {
        Primitive::Sphere(SphereParams {
            radius: 100.0,
            sides,
            rings,
        })
        .mesh()
    }

    /// The way the values were set before the checks moved out of the
    /// loop, one edge at a time: the result to match.
    fn set_one_by_one(mesh: &mut Mesh, layer: &str, edges: &[EdgeKey], value: f32) -> Outcome {
        let mut total = Outcome::default();
        for edge in edges {
            let outcome = set_weights(mesh, layer, std::slice::from_ref(edge), value);
            total.matched += outcome.matched;
            total.skipped += outcome.skipped;
            total.missing += outcome.missing;
        }
        total
    }

    #[test]
    fn setting_many_edges_at_once_matches_setting_them_one_by_one() {
        let base = sphere(16, 8);
        let mut edges: Vec<EdgeKey> = base.topology().edges().step_by(2).collect();
        // One edge twice, and one the mesh doesn't have.
        edges.push(edges[0]);
        edges.push(EdgeKey::new(
            VertexId::from_raw(0),
            VertexId::from_raw(9_999),
        ));
        for value in [0.25, 1.0, 0.0, 7.0, f32::NAN] {
            let (mut at_once, mut one_by_one) = (base.clone(), base.clone());
            // Start from some values already set, so "already so" counts.
            let first: Vec<EdgeKey> = edges.iter().copied().take(5).collect();
            set_weights(&mut at_once, "w", &first, 0.25);
            set_weights(&mut one_by_one, "w", &first, 0.25);
            let a = set_weights(&mut at_once, "w", &edges, value);
            let b = set_one_by_one(&mut one_by_one, "w", &edges, value);
            assert_eq!(
                weights(&at_once, "w"),
                weights(&one_by_one, "w"),
                "value {value}"
            );
            assert_eq!(
                at_once.attribute_values("w").is_some(),
                one_by_one.attribute_values("w").is_some()
            );
            assert_eq!(a.missing, b.missing);
            assert_eq!(a.matched + a.skipped, b.matched + b.skipped);
        }
    }

    #[test]
    #[ignore = "a timing, for release builds"]
    fn set_weights_timing() {
        // About 50 000 edges: this took about 10 s while the layer was
        // looked up once per edge, and takes a few milliseconds now.
        let mut mesh = sphere(256, 100);
        let edges: Vec<EdgeKey> = mesh.topology().edges().collect();
        let start = std::time::Instant::now();
        let outcome = set_weights(&mut mesh, "w", &edges, 0.5);
        let took = start.elapsed();
        println!("{} edges given a value in {took:?}", edges.len());
        assert_eq!(outcome.matched, edges.len());
        assert!(took.as_secs_f64() < 1.0, "took {took:?}");
    }
}
