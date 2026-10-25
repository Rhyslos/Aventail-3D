//! Bevel weights: which edges the Live bevel modifier bevels, and how
//! wide, when it bevels only marked edges
//! ([decision 0149](../../../docs/decisions/0149-live-bevel.md)).
//!
//! Each edge can hold a weight from 0 (not marked) to 1 (bevelled at the
//! modifier's full width), in the [`BEVEL_WEIGHT`] edge layer; between,
//! the edge is bevelled that share of the width. Only the Live bevel
//! modifier ([`crate::live_bevel`]) reads it, and only with *Marked
//! edges*: the weight changes nothing else.
//!
//! The layer works as creases do ([`crate::crease`]): it is there only
//! while an edge has a weight, and is carried to new edges by tools like
//! hard edges and seams are ([`crate::builder::Builder`]).

use crate::edge_weights;
use crate::edit::Outcome;
use crate::{EdgeKey, Mesh};

/// The name of the edge layer holding bevel weights (0 to 1).
pub const BEVEL_WEIGHT: &str = "bevel_weight";

/// A weight brought within 0 to 1; one that isn't a real number is 0.
pub fn usable(weight: f32) -> f32 {
    edge_weights::usable(weight)
}

/// The bevel weight of `edge`: 0 for none (or an edge not in the mesh).
pub fn bevel_weight_of(mesh: &Mesh, edge: EdgeKey) -> f32 {
    edge_weights::weight_of(mesh, BEVEL_WEIGHT, edge)
}

/// Every edge with a bevel weight, and how much, in edge order. Values
/// left on edges the mesh no longer has are included; callers that care
/// check the topology.
pub fn bevel_weights(mesh: &Mesh) -> Vec<(EdgeKey, f32)> {
    edge_weights::weights(mesh, BEVEL_WEIGHT)
}

/// Gives `edges` the bevel weight `weight` (0 takes it away; brought
/// within 0 to 1, and a value that isn't a real number counts as 0). The
/// layer is added when needed and taken away when no edge has a weight
/// left. Edges not in the mesh are missing; edges already so are skipped.
pub fn set_bevel_weights(mesh: &mut Mesh, edges: &[EdgeKey], weight: f32) -> Outcome {
    edge_weights::set_weights(mesh, BEVEL_WEIGHT, edges, weight)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VertexId;
    use crate::attributes::{Domain, Value};

    fn cube() -> Mesh {
        crate::Primitive::Cube(Default::default()).mesh()
    }

    fn some_edges(mesh: &Mesh, n: usize) -> Vec<EdgeKey> {
        mesh.topology().edges().take(n).collect()
    }

    #[test]
    fn sets_reads_and_clears_weights() {
        let mut mesh = cube();
        assert!(bevel_weights(&mesh).is_empty());
        let edges = some_edges(&mesh, 4);
        assert_eq!(set_bevel_weights(&mut mesh, &edges, 0.25).matched, 4);
        assert_eq!(bevel_weights(&mesh).len(), 4);
        assert_eq!(bevel_weight_of(&mesh, edges[1]), 0.25);
        assert_eq!(set_bevel_weights(&mut mesh, &edges, 0.25).skipped, 4);
        assert_eq!(set_bevel_weights(&mut mesh, &edges, 0.0).matched, 4);
        assert!(mesh.attribute_values(BEVEL_WEIGHT).is_none());
    }

    #[test]
    fn weights_and_creases_are_separate() {
        let mut mesh = cube();
        let edges = some_edges(&mesh, 2);
        set_bevel_weights(&mut mesh, &edges, 1.0);
        assert!(crate::crease::creases(&mesh).is_empty());
        crate::crease::set_creases(&mut mesh, &edges, 0.5);
        assert_eq!(bevel_weight_of(&mesh, edges[0]), 1.0);
        assert_eq!(crate::crease::crease_of(&mesh, edges[0]), 0.5);
    }

    #[test]
    fn values_out_of_range_are_brought_in() {
        let mut mesh = cube();
        let edges = some_edges(&mesh, 2);
        set_bevel_weights(&mut mesh, &edges, 9.0);
        assert_eq!(bevel_weight_of(&mesh, edges[0]), 1.0);
        set_bevel_weights(&mut mesh, &edges, f32::NAN);
        assert!(bevel_weights(&mesh).is_empty());
        set_bevel_weights(&mut mesh, &edges, f32::NEG_INFINITY);
        assert!(bevel_weights(&mesh).is_empty());
        assert_eq!(usable(-2.0), 0.0);
    }

    #[test]
    fn missing_edges_empty_meshes_and_other_layers() {
        let mut mesh = cube();
        let gone = EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(5_000));
        assert_eq!(set_bevel_weights(&mut mesh, &[gone], 1.0).missing, 1);
        assert!(mesh.attribute_values(BEVEL_WEIGHT).is_none());
        let mut empty = Mesh::new();
        assert_eq!(set_bevel_weights(&mut empty, &[], 1.0).matched, 0);
        let mut other = cube();
        other
            .add_attribute(BEVEL_WEIGHT, Domain::Edge, Value::Bool(true))
            .unwrap();
        let edges = some_edges(&other, 1);
        assert_eq!(set_bevel_weights(&mut other, &edges, 1.0).matched, 0);
        assert_eq!(bevel_weight_of(&other, edges[0]), 0.0);
    }
}
