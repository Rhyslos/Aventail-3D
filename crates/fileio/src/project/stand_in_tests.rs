//! Tests for objects the older parts of the file hold as empty groups
//! (curves, decision 0138; lattice cages, decision 0144) while the parts
//! that come before them in the file name them: an instance's original, a
//! boolean's cutter or kept shape, levels of detail, collision "Wrapped in
//! parts". Each such project saves (the save reads its bytes back) and
//! opens as it was.

use super::tests::assert_same;
use super::*;
use scene::{
    BooleanKind, Collision, Command, CurveObject, CurveShape, CurveSolid, Lods, ModifierKind,
    ObjectId, ObjectKind, Primitive, ReadyCurve, Sweep, SweepProfile,
};

/// A document with one swept ring (a curve with a solid).
fn with_ring() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject {
        shape: CurveShape::Ready(ReadyCurve::Circle { radius: 40.0 }),
        solid: CurveSolid::Sweep(Sweep {
            profile: SweepProfile::Circle,
            ..Sweep::default()
        }),
    })))
    .unwrap();
    let ring = doc.selection()[0];
    (doc, ring)
}

fn cube(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        Default::default(),
    ))))
    .unwrap();
    doc.selection()[0]
}

/// Saves `doc` (which reads the bytes back) and opens the bytes.
fn round_trip(doc: &Document) -> Document {
    let bytes = to_bytes(doc).expect("the project saves");
    let opened = from_bytes(&bytes).expect("the project opens");
    assert_same(doc, &opened);
    opened
}

#[test]
fn a_swept_curve_that_an_instance_shows_saves_and_opens() {
    let (mut doc, ring) = with_ring();
    doc.execute(Command::Duplicate {
        ids: vec![ring],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    let opened = round_trip(&doc);
    assert_eq!(
        opened.scene().object(instance).unwrap().kind,
        ObjectKind::Instance(ring)
    );
}

#[test]
fn a_swept_curve_as_a_boolean_cutter_saves_and_opens() {
    let (mut doc, ring) = with_ring();
    let kept = cube(&mut doc);
    doc.execute(Command::AddBoolean {
        kept,
        cutters: vec![ring],
        kind: BooleanKind::Subtract,
    })
    .unwrap();
    let opened = round_trip(&doc);
    assert_eq!(opened.scene().booleans(kept)[0].cutters, vec![ring]);
}

#[test]
fn a_swept_curve_as_a_booleans_kept_shape_saves_and_opens() {
    let (mut doc, ring) = with_ring();
    let cutter = cube(&mut doc);
    doc.execute(Command::AddBoolean {
        kept: ring,
        cutters: vec![cutter],
        kind: BooleanKind::Subtract,
    })
    .unwrap();
    let opened = round_trip(&doc);
    assert_eq!(opened.scene().booleans(ring)[0].cutters, vec![cutter]);
}

#[test]
fn a_swept_curve_with_levels_of_detail_saves_and_opens() {
    let (mut doc, ring) = with_ring();
    doc.execute(Command::SetLods {
        ids: vec![ring],
        lods: Lods {
            on: true,
            ..Lods::default()
        },
    })
    .unwrap();
    let opened = round_trip(&doc);
    assert!(opened.scene().object(ring).unwrap().lods.on);
}

#[test]
fn a_swept_curve_wrapped_in_parts_saves_and_opens() {
    let (mut doc, ring) = with_ring();
    doc.execute(Command::SetCollision {
        ids: vec![ring],
        collision: Collision::Parts,
    })
    .unwrap();
    let opened = round_trip(&doc);
    assert_eq!(
        opened.scene().object(ring).unwrap().collision,
        Collision::Parts
    );
}

#[test]
fn a_swept_curve_shaped_by_a_lattice_and_cutting_another_saves_and_opens() {
    // Every kind of stand-in at once: a cage, and a curve that has a
    // cage's Lattice modifier, an instance, levels and collision parts.
    let (mut doc, ring) = with_ring();
    doc.execute(Command::AddLattice {
        ids: vec![ring],
        divisions: [2, 2, 2],
    })
    .unwrap();
    assert!(
        doc.scene()
            .modifiers(ring)
            .iter()
            .any(|m| matches!(m.kind, ModifierKind::Lattice(_)))
    );
    let kept = cube(&mut doc);
    doc.execute(Command::AddBoolean {
        kept,
        cutters: vec![ring],
        kind: BooleanKind::Union,
    })
    .unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![ring],
        instances: true,
    })
    .unwrap();
    doc.execute(Command::SetCollision {
        ids: vec![ring],
        collision: Collision::Parts,
    })
    .unwrap();
    round_trip(&doc);
}

#[test]
fn a_group_still_cant_have_levels_or_collision_parts() {
    // The readers wait for stand-ins to be given back before checking,
    // but a real group is still refused once everything is in.
    let mut doc = Document::new();
    let a = cube(&mut doc);
    doc.execute(Command::SetLods {
        ids: vec![a],
        lods: Lods {
            on: true,
            ..Lods::default()
        },
    })
    .unwrap();
    doc.execute(Command::SetCollision {
        ids: vec![a],
        collision: Collision::Parts,
    })
    .unwrap();
    doc.execute(Command::Group(vec![a])).unwrap();
    let group = doc.selection()[0].get();
    let model = v43::ProjectV43::from_document(&doc);
    let settings = |model: &v43::ProjectV43| {
        model
            .project
            .project
            .project
            .objects
            .iter()
            .find(|o| o.id == a.get())
            .unwrap()
            .clone()
    };
    // The group takes the cube's levels, then its collision parts.
    for keep in [true, false] {
        let mut damaged = model.clone();
        let cube_settings = settings(&model);
        let entry = damaged
            .project
            .project
            .project
            .objects
            .0
            .iter_mut()
            .find(|o| o.id == group)
            .unwrap();
        if keep {
            entry.lods = cube_settings.lods;
        } else {
            entry.collision_parts = cube_settings.collision_parts;
        }
        assert!(matches!(
            from_bytes(&super::tests::current_file(&damaged)),
            Err(ProjectError::Damaged(_))
        ));
    }
}

#[test]
fn a_modifier_reading_an_object_of_the_wrong_kind_is_taken_off_with_a_note() {
    // A project saved before Bake into shape took Bend along curve off: its
    // curve is now a mesh. Here the curve is left out of the file's curves,
    // so it opens as an empty group, which the bend can't read either.
    let (mut doc, ring) = with_ring();
    let bent = cube(&mut doc);
    doc.execute(Command::Rename {
        id: bent,
        name: "Bent box".into(),
    })
    .unwrap();
    doc.execute(Command::AddModifier {
        ids: vec![bent],
        kind: ModifierKind::BendAlongCurve(scene::CurveBendSettings::of(ring)),
    })
    .unwrap();
    doc.execute(Command::AddModifier {
        ids: vec![bent],
        kind: ModifierKind::Triangulate(Default::default()),
    })
    .unwrap();
    let mut model = v43::ProjectV43::from_document(&doc);
    model.project.curves.0.clear();
    let bytes = super::tests::current_file(&model);
    let (opened, notes) = from_bytes_noting(&bytes).expect("it still opens");
    let left: Vec<_> = opened.scene().modifiers(bent).to_vec();
    assert_eq!(left.len(), 1, "only the bend is taken off");
    assert!(matches!(left[0].kind, ModifierKind::Triangulate(_)));
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("Bent box"), "{}", notes[0]);
    assert!(from_bytes(&bytes).is_ok());
    // A save never hands out a file that would lose something on opening.
    assert!(to_bytes(&opened).is_ok());
}
