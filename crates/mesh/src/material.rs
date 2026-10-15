//! Material slots: which material each polygon uses
//! ([decision 0074](../../../docs/decisions/0074-material-slots.md)).
//!
//! Each polygon holds a slot number in the [`MATERIAL`] polygon layer:
//! 0 (the first material, the standard) until it is given another. The
//! object keeps the list of materials the numbers point into; the mesh
//! only knows the numbers. Tools carry the slot over to the polygons they
//! make (walls take the slot of the polygon they grew from).

use crate::attributes::{AttributeElement, Domain, Value};
use crate::edit::Outcome;
use crate::{FaceId, Mesh};
use std::collections::BTreeSet;

/// The name of the polygon layer holding each polygon's material slot.
pub const MATERIAL: &str = "material";

/// The most material slots an object may have.
pub const MAX_SLOTS: u32 = 64;

/// The material slot of `face`: 0 when it has none set (or isn't there).
pub fn slot_of(mesh: &Mesh, face: FaceId) -> u32 {
    match mesh.attribute(MATERIAL, AttributeElement::Face(face)) {
        Some(Value::Int(slot)) => u32::try_from(slot).unwrap_or(0),
        _ => 0,
    }
}

/// The polygons of `mesh` using material slot `slot`, in id order.
pub fn faces_with_slot(mesh: &Mesh, slot: u32) -> Vec<FaceId> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| slot_of(mesh, *f) == slot)
        .collect()
}

/// Gives `faces` material slot `slot` (at most [`MAX_SLOTS`] - 1). Polygons
/// not in the mesh are counted as missing; a slot past the last is
/// refused, changing nothing.
pub fn set_material(mesh: &mut Mesh, faces: &[FaceId], slot: u32) -> Outcome {
    let faces: BTreeSet<FaceId> = faces.iter().copied().collect();
    let present: Vec<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some())
        .collect();
    let missing = faces.len() - present.len();
    if slot >= MAX_SLOTS || present.is_empty() {
        return Outcome {
            matched: 0,
            missing,
            skipped: present.len(),
        };
    }
    let has_layer = mesh
        .attributes()
        .any(|(name, domain, _)| name == MATERIAL && domain == Domain::Face);
    if !has_layer
        && mesh
            .add_attribute(MATERIAL, Domain::Face, Value::Int(0))
            .is_err()
    {
        return Outcome {
            matched: 0,
            missing,
            skipped: present.len(),
        };
    }
    let mut matched = 0;
    for face in present {
        if mesh
            .set_attribute(
                MATERIAL,
                AttributeElement::Face(face),
                Value::Int(slot as i32),
            )
            .is_ok()
        {
            matched += 1;
        }
    }
    Outcome {
        matched,
        missing,
        skipped: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Primitive;

    fn cube() -> Mesh {
        Primitive::Cube(Default::default()).mesh()
    }

    #[test]
    fn polygons_start_in_slot_0_and_can_be_given_another() {
        let mut mesh = cube();
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        assert!(faces.iter().all(|f| slot_of(&mesh, *f) == 0));
        let outcome = set_material(&mut mesh, &faces[..2], 3);
        assert_eq!((outcome.matched, outcome.missing), (2, 0));
        assert_eq!(slot_of(&mesh, faces[0]), 3);
        assert_eq!(faces_with_slot(&mesh, 3), faces[..2].to_vec());
        assert_eq!(faces_with_slot(&mesh, 0).len(), 4);
    }

    #[test]
    fn missing_polygons_and_slots_past_the_last_are_refused() {
        let mut mesh = cube();
        let before = mesh.clone();
        let outcome = set_material(&mut mesh, &[FaceId::from_raw(999)], 1);
        assert_eq!((outcome.matched, outcome.missing), (0, 1));
        let first = mesh.faces().next().unwrap().0;
        assert_eq!(set_material(&mut mesh, &[first], MAX_SLOTS).matched, 0);
        assert_eq!(mesh, before);
    }

    #[test]
    fn walls_of_an_extrude_take_the_slot_of_their_polygon() {
        let mut mesh = cube();
        let top = mesh
            .faces()
            .map(|(f, _)| f)
            .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
            .unwrap();
        set_material(&mut mesh, &[top], 2);
        crate::edit::extrude_polygons(&mut mesh, &[top], 10.0, crate::edit::Grouping::Together);
        assert_eq!(faces_with_slot(&mesh, 2).len(), 1 + 4);
    }

    #[test]
    fn inset_and_loop_cut_keep_the_slots_of_what_they_cut() {
        let mut mesh = cube();
        let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
        set_material(&mut mesh, &faces, 1);
        let top = faces[0];
        crate::edit::inset_polygons(&mut mesh, &[top], 10.0, crate::edit::Grouping::Together);
        assert_eq!(faces_with_slot(&mesh, 1).len(), mesh.face_count());
        let edge = mesh.edges()[0];
        crate::edit::loop_cut(&mut mesh, edge, 2, 0.5);
        assert!(mesh.face_count() > 10);
        assert_eq!(faces_with_slot(&mesh, 1).len(), mesh.face_count());
    }
}
