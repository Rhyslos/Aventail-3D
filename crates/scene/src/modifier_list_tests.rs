//! Tests for the commands the modifier lists' right-click menu adds
//! (decision 0168): Duplicate, and Bake into shape up to here.

use crate::*;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

fn remesh(voxel_size: f32) -> ModifierKind {
    ModifierKind::Remesh(RemeshOptions {
        voxel_size,
        smooth: 0,
    })
}

fn subdivide(levels: u32) -> ModifierKind {
    ModifierKind::Subdivide(SubdivideOptions {
        levels,
        export_levels: levels,
        ..SubdivideOptions::default()
    })
}

/// Undoes every step and redoes them, checking each scene comes back.
fn check_round_trip(doc: &mut Document) {
    let mut states = vec![doc.scene().clone()];
    while doc.undo().is_some() {
        states.push(doc.scene().clone());
    }
    states.pop();
    while let Some(expected) = states.pop() {
        doc.redo().unwrap();
        assert_eq!(doc.scene(), &expected);
    }
}

#[test]
fn duplicate_puts_a_copy_just_after_with_a_new_id() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, subdivide(1));
    add(&mut doc, id, remesh(20.0));
    let before = doc.scene().modifiers(id).to_vec();
    doc.execute(Command::DuplicateModifier { id, index: 0 })
        .unwrap();
    assert_eq!(doc.undo_label(), Some("Duplicate Subdivide"));
    let after = doc.scene().modifiers(id).to_vec();
    assert_eq!(after.len(), 3);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1].kind, before[0].kind);
    assert_eq!(after[2], before[1]);
    let ids: std::collections::BTreeSet<_> = after.iter().map(|m| m.id).collect();
    assert_eq!(ids.len(), 3, "the copy has its own id");
    // The last one copies to the end.
    doc.execute(Command::DuplicateModifier { id, index: 2 })
        .unwrap();
    assert_eq!(doc.scene().modifiers(id)[3].kind, before[1].kind);
    // Nothing there: refused, nothing changes.
    let count = doc.scene().modifiers(id).len();
    assert!(
        doc.execute(Command::DuplicateModifier { id, index: 9 })
            .is_err()
    );
    assert_eq!(doc.scene().modifiers(id).len(), count);
    check_round_trip(&mut doc);
}

#[test]
fn duplicate_stops_at_the_most_modifiers_a_shape_can_have() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    for _ in 0..MAX_MODIFIERS {
        add(&mut doc, id, subdivide(0));
    }
    assert!(
        doc.execute(Command::DuplicateModifier { id, index: 0 })
            .is_err()
    );
    assert_eq!(doc.scene().modifiers(id).len(), MAX_MODIFIERS);
}

#[test]
fn bake_up_to_here_bakes_the_first_ones_and_keeps_the_rest() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, subdivide(1));
    // The shape with only the first modifier, as the bake should leave it.
    let expected = doc.scene().exported_shape(id).unwrap();
    add(&mut doc, id, remesh(20.0));
    let later = doc.scene().modifiers(id)[1].clone();
    doc.execute(Command::BakeModifiersUpTo { id, index: 0 })
        .unwrap();
    assert_eq!(doc.undo_label(), Some("Bake into shape up to Subdivide"));
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.kind, ObjectKind::Mesh(expected));
    assert_eq!(
        object.modifiers,
        vec![later],
        "the later one stays, id and all"
    );
    check_round_trip(&mut doc);
}

#[test]
fn bake_up_to_here_keeps_the_ones_switched_off() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, remesh(20.0));
    add(&mut doc, id, subdivide(1));
    add(&mut doc, id, subdivide(1));
    let mut off = doc.scene().modifiers(id)[0].clone();
    off.on = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: off.clone(),
    })
    .unwrap();
    let last = doc.scene().modifiers(id)[2].clone();
    doc.execute(Command::BakeModifiersUpTo { id, index: 1 })
        .unwrap();
    let object = doc.scene().object(id).unwrap();
    // The cube smoothed once (the remesh was off): 6 × 4 polygons.
    let ObjectKind::Mesh(mesh) = &object.kind else {
        panic!("baked into a mesh");
    };
    assert_eq!(mesh.face_count(), 24);
    assert_eq!(object.modifiers, vec![off, last]);
}

#[test]
fn bake_up_to_the_last_is_bake_into_shape() {
    let mut a = Document::new();
    let id = create(&mut a, PrimitiveKind::Sphere);
    add(&mut a, id, subdivide(1));
    add(&mut a, id, remesh(25.0));
    let mut b = a.clone();
    a.execute(Command::BakeModifiersUpTo { id, index: 1 })
        .unwrap();
    b.execute(Command::MakeEditable(vec![id])).unwrap();
    assert_eq!(a.scene().object(id), b.scene().object(id));
}

#[test]
fn bake_up_to_here_keeps_a_levels_changes_to_the_later_ones() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, id, subdivide(1));
    add(&mut doc, id, subdivide(1));
    let modifiers = doc.scene().modifiers(id).to_vec();
    let mut lods = Lods {
        on: true,
        ..Lods::default()
    }
    .with_count(1);
    lods.levels[0] = lods.levels[0].with_override(&modifiers, modifiers[1].id, false, None);
    doc.execute(Command::SetLods {
        ids: vec![id],
        lods: lods.clone(),
    })
    .unwrap();
    doc.execute(Command::BakeModifiersUpTo { id, index: 0 })
        .unwrap();
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.modifiers, vec![modifiers[1].clone()]);
    assert_eq!(
        object.lods.levels[0].overrides, lods.levels[0].overrides,
        "the level still switches the later Subdivide off"
    );
}

#[test]
fn bake_up_to_here_is_refused_where_there_is_nothing() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    assert!(
        doc.execute(Command::BakeModifiersUpTo { id, index: 0 })
            .is_err()
    );
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    assert!(
        doc.execute(Command::BakeModifiersUpTo {
            id: group,
            index: 0
        })
        .is_err()
    );
}
