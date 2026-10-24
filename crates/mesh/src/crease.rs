//! Creases: how sharply subdivision smoothing keeps an edge
//! ([decision 0143](../../../docs/decisions/0143-subdivide-and-creases.md)).
//!
//! Each edge can hold a crease from 0 (smoothed like any other edge) to 1
//! (kept fully sharp), in the [`CREASE`] edge layer. Only the Subdivide
//! modifier ([`crate::subdivide`]) reads it: a crease changes the shape
//! subdivision makes, not the shading or the base shape. In between, the
//! edge stays sharp for the first few levels of smoothing and is then
//! smoothed, so it comes out as a tighter curve than its neighbours.
//!
//! The layer is there only while an edge has a crease, and is carried to
//! new edges by tools like hard edges and seams are
//! ([`crate::builder::Builder`]).

use crate::edge_weights;
use crate::edit::Outcome;
use crate::{EdgeKey, Mesh};

/// The name of the edge layer holding creases (0 to 1).
pub const CREASE: &str = "crease";

/// A crease brought within 0 to 1; one that isn't a real number is 0.
pub fn usable(crease: f32) -> f32 {
    edge_weights::usable(crease)
}

/// The crease of `edge`: 0 for none (or an edge not in the mesh).
pub fn crease_of(mesh: &Mesh, edge: EdgeKey) -> f32 {
    edge_weights::weight_of(mesh, CREASE, edge)
}

/// Every edge with a crease, and how much, in edge order. Values left on
/// edges the mesh no longer has are included; callers that care check the
/// topology.
pub fn creases(mesh: &Mesh) -> Vec<(EdgeKey, f32)> {
    edge_weights::weights(mesh, CREASE)
}

/// Gives `edges` the crease `crease` (0 takes it away; brought within 0 to
/// 1, and a value that isn't a real number counts as 0). The layer is
/// added when needed and taken away when no edge has a crease left. Edges
/// not in the mesh (no polygon uses them) are missing; edges already so
/// are skipped.
pub fn set_creases(mesh: &mut Mesh, edges: &[EdgeKey], crease: f32) -> Outcome {
    edge_weights::set_weights(mesh, CREASE, edges, crease)
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
    fn sets_reads_and_clears_creases() {
        let mut mesh = cube();
        assert!(creases(&mesh).is_empty());
        let edges = some_edges(&mesh, 3);
        let outcome = set_creases(&mut mesh, &edges, 0.5);
        assert_eq!(outcome.matched, 3);
        assert_eq!(creases(&mesh).len(), 3);
        assert_eq!(crease_of(&mesh, edges[0]), 0.5);
        // Again: nothing changes.
        assert_eq!(set_creases(&mut mesh, &edges, 0.5).skipped, 3);
        // Taken away: the layer goes with the last one.
        assert_eq!(set_creases(&mut mesh, &edges, 0.0).matched, 3);
        assert!(creases(&mesh).is_empty());
        assert!(mesh.attribute_values(CREASE).is_none());
    }

    #[test]
    fn settings_out_of_range_are_brought_in() {
        let mut mesh = cube();
        let edges = some_edges(&mesh, 2);
        set_creases(&mut mesh, &edges, 7.0);
        assert_eq!(crease_of(&mesh, edges[0]), 1.0);
        set_creases(&mut mesh, &edges, -3.0);
        assert!(creases(&mesh).is_empty());
        set_creases(&mut mesh, &edges, f32::NAN);
        assert!(creases(&mesh).is_empty());
        assert_eq!(usable(f32::INFINITY), 0.0);
    }

    #[test]
    fn edges_not_in_the_mesh_are_missing() {
        let mut mesh = cube();
        let gone = EdgeKey::new(VertexId::from_raw(0), VertexId::from_raw(9_999));
        let outcome = set_creases(&mut mesh, &[gone], 1.0);
        assert_eq!(outcome.missing, 1);
        assert!(mesh.attribute_values(CREASE).is_none());
        // Clearing on a mesh without creases changes nothing.
        let edges = some_edges(&mesh, 1);
        assert_eq!(set_creases(&mut mesh, &edges, 0.0).skipped, 1);
    }

    #[test]
    fn a_layer_of_another_kind_is_left_alone() {
        let mut mesh = cube();
        mesh.add_attribute(CREASE, Domain::Edge, Value::Bool(false))
            .unwrap();
        let edges = some_edges(&mesh, 2);
        let outcome = set_creases(&mut mesh, &edges, 1.0);
        assert_eq!(outcome.matched, 0);
        assert_eq!(crease_of(&mesh, edges[0]), 0.0);
        // Clearing leaves it too.
        set_creases(&mut mesh, &edges, 0.0);
        assert!(mesh.attribute_values(CREASE).is_some());
    }

    #[test]
    fn an_empty_mesh_has_no_creases() {
        let mut mesh = Mesh::new();
        assert_eq!(set_creases(&mut mesh, &[], 1.0).matched, 0);
        assert!(creases(&mesh).is_empty());
    }
}
