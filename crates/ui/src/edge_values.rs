//! The numbers edges can carry, 0 to 1, set in the Edges mode through the
//! Attributes panel's Selected edges section: creases for the Subdivide
//! modifier ([decision 0143](../../../docs/decisions/0143-subdivide-and-creases.md))
//! and bevel weights for the Live bevel modifier
//! ([decision 0149](../../../docs/decisions/0149-live-bevel.md)). Both are
//! edge layers set by recorded edits and work the same way, so the panel
//! and the workbench share their code through [`EdgeValue`].

use mesh::{Anchors, Mesh};
use scene::{EdgeKey, Operation};

/// Which number an edge carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeValue {
    /// How sharply the Subdivide modifier keeps the edge.
    Crease,
    /// Which edges the Live bevel modifier bevels with *Marked edges*, and
    /// how much of its width.
    BevelWeight,
}

/// The Crease field's name.
pub(crate) const CREASE_LABEL: &str = crate::text!("Crease");
/// The Bevel weight field's name.
pub(crate) const BEVEL_WEIGHT_LABEL: &str = crate::text!("Bevel weight");

impl EdgeValue {
    /// Both, in the order the panel shows them.
    pub(crate) const ALL: [Self; 2] = [Self::Crease, Self::BevelWeight];

    /// The field's name, as shown (and as the undo label's verb).
    pub(crate) fn name(self) -> &'static str {
        crate::text::tr(match self {
            Self::Crease => CREASE_LABEL,
            Self::BevelWeight => BEVEL_WEIGHT_LABEL,
        })
    }

    /// The edge's value: 0 for none.
    pub(crate) fn of(self, mesh: &Mesh, edge: EdgeKey) -> f32 {
        match self {
            Self::Crease => mesh::crease::crease_of(mesh, edge),
            Self::BevelWeight => mesh::bevel_weight::bevel_weight_of(mesh, edge),
        }
    }

    /// A value brought within 0 to 1 (0 when it isn't a real number).
    pub(crate) fn usable(self, value: f32) -> f32 {
        match self {
            Self::Crease => mesh::crease::usable(value),
            Self::BevelWeight => mesh::bevel_weight::usable(value),
        }
    }

    /// The recorded edit giving `edges` the value.
    pub(crate) fn operation(
        self,
        edges: Vec<EdgeKey>,
        value: f32,
        anchors: Option<Anchors>,
    ) -> Operation {
        match self {
            Self::Crease => Operation::SetCreases {
                edges,
                crease: value,
                anchors,
            },
            Self::BevelWeight => Operation::SetBevelWeights {
                edges,
                weight: value,
                anchors,
            },
        }
    }

    /// Whether `op` is an edit setting this value.
    pub(crate) fn sets(self, op: &Operation) -> bool {
        matches!(
            (self, op),
            (Self::Crease, Operation::SetCreases { .. })
                | (Self::BevelWeight, Operation::SetBevelWeights { .. })
        )
    }

    /// `op` (an edit setting this value, maybe mirrored) with its value
    /// changed.
    pub(crate) fn with_value(self, op: &Operation, value: f32) -> Option<Operation> {
        match op {
            Operation::SetCreases { edges, anchors, .. } if self == Self::Crease => {
                Some(self.operation(edges.clone(), value, anchors.clone()))
            }
            Operation::SetBevelWeights { edges, anchors, .. } if self == Self::BevelWeight => {
                Some(self.operation(edges.clone(), value, anchors.clone()))
            }
            Operation::Mirrored { op, mirror } => Some(Operation::Mirrored {
                op: Box::new(self.with_value(op, value)?),
                mirror: *mirror,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_value_reads_and_records_its_own_layer() {
        let mut cube = mesh::Primitive::Cube(Default::default()).mesh();
        let edges: Vec<EdgeKey> = cube.topology().edges().take(2).collect();
        mesh::bevel_weight::set_bevel_weights(&mut cube, &edges, 0.5);
        assert_eq!(EdgeValue::BevelWeight.of(&cube, edges[0]), 0.5);
        assert_eq!(EdgeValue::Crease.of(&cube, edges[0]), 0.0);
        for value in EdgeValue::ALL {
            let op = value.operation(edges.clone(), 1.0, None);
            assert!(value.sets(&op));
            let changed = value.with_value(&op, 0.25).unwrap();
            assert!(value.sets(&changed));
            assert!(changed.numbers_are_real());
            assert_eq!(value.usable(f32::NAN), 0.0);
            assert_eq!(value.usable(3.0), 1.0);
        }
        let crease = EdgeValue::Crease.operation(edges, 1.0, None);
        assert!(!EdgeValue::BevelWeight.sets(&crease));
        assert!(EdgeValue::BevelWeight.with_value(&crease, 0.5).is_none());
    }
}
