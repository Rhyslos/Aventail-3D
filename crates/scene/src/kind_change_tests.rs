//! Tests for commands that change an object's kind (Bake into shape,
//! Apply mirror): a modifier elsewhere that reads the object and can't
//! read its new kind (Bend along curve reads only a curve) is taken off in
//! the same undo step, so the scene always makes sense and saves.

use crate::*;

/// A swept ring (a curve with a solid, so it bakes) and a cube bent along
/// it: the document, the cube and the ring.
fn cube_bent_along_a_ring() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject {
        shape: CurveShape::Ready(ReadyCurve::Circle { radius: 200.0 }),
        solid: CurveSolid::Sweep(Sweep::default()),
    })))
    .unwrap();
    let ring = doc.selection()[0];
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: ModifierKind::BendAlongCurve(CurveBendSettings::of(ring)),
    })
    .unwrap();
    (doc, cube, ring)
}

fn bends(doc: &Document, id: ObjectId) -> usize {
    doc.scene()
        .modifiers(id)
        .iter()
        .filter(|m| matches!(m.kind, ModifierKind::BendAlongCurve(_)))
        .count()
}

fn check_both_ways(mut doc: Document, cube: ObjectId, ring: ObjectId, command: Command) {
    doc.execute(command).unwrap();
    assert!(matches!(
        doc.scene().object(ring).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
    assert_eq!(bends(&doc, cube), 0, "the bend can't read a mesh");
    doc.check_opened().expect("the scene makes sense");
    // One undo step puts both back.
    doc.undo().unwrap();
    assert!(matches!(
        doc.scene().object(ring).unwrap().kind,
        ObjectKind::Curve(_)
    ));
    assert_eq!(bends(&doc, cube), 1);
    doc.redo().unwrap();
    assert_eq!(bends(&doc, cube), 0);
}

#[test]
fn baking_a_curve_another_shape_is_bent_along_takes_the_bend_off() {
    let (doc, cube, ring) = cube_bent_along_a_ring();
    check_both_ways(doc, cube, ring, Command::MakeEditable(vec![ring]));
}

#[test]
fn applying_the_mirror_of_a_curve_another_shape_is_bent_along_takes_the_bend_off() {
    let (mut doc, cube, ring) = cube_bent_along_a_ring();
    doc.execute(Command::SetMirror {
        ids: vec![ring],
        mirror: Some(LiveMirror::across_x()),
    })
    .unwrap();
    check_both_ways(doc, cube, ring, Command::ApplyMirror(vec![ring]));
}

#[test]
fn a_shrinkwrap_onto_a_baked_curve_stays() {
    // Shrinkwrap reads any shape: a curve baked into a mesh is still one.
    let (mut doc, cube, ring) = cube_bent_along_a_ring();
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: ModifierKind::Shrinkwrap(ShrinkwrapSettings::of(ring)),
    })
    .unwrap();
    doc.execute(Command::MakeEditable(vec![ring])).unwrap();
    assert_eq!(bends(&doc, cube), 0);
    assert!(
        doc.scene()
            .modifiers(cube)
            .iter()
            .any(|m| matches!(m.kind, ModifierKind::Shrinkwrap(_)))
    );
}
