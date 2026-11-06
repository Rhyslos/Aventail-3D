//! The baker's cage painted on a shape ([decision 0251](../../../docs/decisions/0251-high-to-low.md)):
//! each point's extra push, kept in the point layer [`BAKE_CAGE`] (0, the
//! bake's own push, where nothing is painted).

use crate::attributes::{AttributeElement, AttributeValues, BAKE_CAGE};
use crate::{Domain, Mesh, Value, VertexId};

/// The extra push at point `v`.
pub fn push_of(mesh: &Mesh, v: VertexId) -> f32 {
    match mesh.attribute(BAKE_CAGE, AttributeElement::Point(v)) {
        Some(Value::Float(x)) if x.is_finite() => x,
        _ => 0.0,
    }
}

/// Every point with an extra push, and it.
pub fn pushes(mesh: &Mesh) -> Vec<(VertexId, f32)> {
    if mesh.attribute_values(BAKE_CAGE).is_none() {
        return Vec::new();
    }
    mesh.vertices()
        .map(|(v, _)| (v, push_of(mesh, v)))
        .filter(|(_, p)| *p != 0.0)
        .collect()
}

/// Sets the listed points' extra push. Points not in the mesh are
/// counted missing; pushes that aren't numbers are skipped. The layer is
/// added when first needed and removed when every push is 0.
pub fn set_push(mesh: &mut Mesh, points: &[(VertexId, f32)]) -> crate::edit::Outcome {
    let mut outcome = crate::edit::Outcome::default();
    if mesh.attribute_values(BAKE_CAGE).is_none() {
        if points.iter().all(|(_, p)| *p == 0.0 || !p.is_finite()) {
            outcome.missing = points.iter().filter(|(v, _)| mesh.position(*v).is_none()).count();
            outcome.skipped = points.len() - outcome.missing;
            return outcome;
        }
        if mesh.add_attribute(BAKE_CAGE, Domain::Point, Value::Float(0.0)).is_err() {
            outcome.skipped = points.len();
            return outcome;
        }
    }
    for (v, push) in points {
        if mesh.position(*v).is_none() {
            outcome.missing += 1;
        } else if !push.is_finite() || push_of(mesh, *v) == *push {
            outcome.skipped += 1;
        } else if mesh
            .set_attribute(BAKE_CAGE, AttributeElement::Point(*v), Value::Float(*push))
            .is_ok()
        {
            outcome.matched += 1;
        } else {
            outcome.skipped += 1;
        }
    }
    let used = matches!(
        mesh.attribute_values(BAKE_CAGE),
        Some((_, _, AttributeValues::Float(list))) if list.iter().any(|x| *x != 0.0)
    );
    if !used {
        mesh.remove_attribute(BAKE_CAGE);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CubeParams, Primitive};

    #[test]
    fn pushes_are_set_read_and_the_layer_goes_with_the_last() {
        let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
        let v = mesh.vertices().next().unwrap().0;
        assert_eq!(set_push(&mut mesh, &[(v, 2.5)]).matched, 1);
        assert_eq!(push_of(&mesh, v), 2.5);
        assert_eq!(pushes(&mesh), vec![(v, 2.5)]);
        assert_eq!(set_push(&mut mesh, &[(v, 2.5)]).matched, 0);
        set_push(&mut mesh, &[(v, 0.0)]);
        assert!(mesh.attribute_values(BAKE_CAGE).is_none());
    }

    #[test]
    fn missing_points_and_odd_pushes_are_left_out() {
        let mut mesh = Primitive::Cube(CubeParams::default()).mesh();
        let v = mesh.vertices().next().unwrap().0;
        let out = set_push(&mut mesh, &[(VertexId::from_raw(999), 1.0), (v, f32::NAN)]);
        assert_eq!(out.missing, 1);
        assert!(pushes(&mesh).is_empty());
        // Nothing but zeros adds no layer.
        set_push(&mut mesh, &[(v, 0.0)]);
        assert!(mesh.attribute_values(BAKE_CAGE).is_none());
    }
}
