//! Tests for the modifier stack and the remesh modifier (decision 0142).

use crate::*;
use glam::Vec3;
use std::sync::Arc;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn remesh(voxel_size: f32) -> ModifierKind {
    ModifierKind::Remesh(RemeshOptions {
        voxel_size,
        smooth: 0,
    })
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// A cube with a remesh at 10 cm.
fn remeshed_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, remesh(10.0));
    (doc, id)
}

/// Whether every polygon of `mesh` is a quad.
fn all_quads(mesh: &Mesh) -> bool {
    mesh.faces().all(|(_, c)| c.len() == 4)
}

#[test]
fn a_modifier_changes_the_shown_shape_not_the_edited_one() {
    let (mut doc, id) = remeshed_cube();
    let scene = doc.scene();
    assert_eq!(scene.modifiers(id).len(), 1);
    assert!(scene.modifiers(id)[0].on && scene.modifiers(id)[0].in_viewport);
    assert_eq!(scene.evaluated(id).unwrap().mesh.face_count(), 6);
    assert_eq!(scene.unmodified_shape(id).unwrap().face_count(), 6);
    let shown = scene.shown_shape(id).unwrap();
    assert!(shown.face_count() > 300 && all_quads(&shown));
    assert_eq!(scene.exported_shape(id).unwrap(), shown);
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(doc.undo_label(), Some("Add Remesh"));
    doc.undo();
    assert!(doc.scene().modifiers(id).is_empty());
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 6);
    doc.redo();
    assert_eq!(doc.scene().shown_shape(id).unwrap(), shown);
}

#[test]
fn settings_change_through_a_command_within_limits() {
    let (mut doc, id) = remeshed_cube();
    let coarse = doc.scene().shown_shape(id).unwrap().face_count();
    let mut changed = doc.scene().modifiers(id)[0].clone();
    changed.kind = remesh(5.0);
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: changed.clone(),
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change Remesh"));
    assert!(doc.scene().shown_shape(id).unwrap().face_count() > 2 * coarse);
    // A size that isn't a real number becomes the standard one.
    changed.kind = remesh(f32::NAN);
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: changed.clone(),
    })
    .unwrap();
    assert_eq!(
        doc.scene().modifiers(id)[0].kind,
        ModifierKind::Remesh(RemeshOptions {
            voxel_size: RemeshOptions::STANDARD_VOXEL,
            smooth: 0
        })
    );
    // Nothing at that place.
    assert_eq!(
        doc.execute(Command::ChangeModifier {
            id,
            index: 3,
            modifier: changed,
        }),
        Err(SceneError::Modifier(ModifierRefusal::NoSuchModifier))
    );
}

#[test]
fn switched_off_or_hidden_in_the_viewport() {
    let (mut doc, id) = remeshed_cube();
    let remeshed = doc.scene().exported_shape(id).unwrap();
    let mut modifier = doc.scene().modifiers(id)[0].clone();
    modifier.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: modifier.clone(),
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Hide Remesh in the viewport"));
    // The viewport skips it; an export still has it.
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 6);
    assert_eq!(doc.scene().exported_shape(id).unwrap(), remeshed);
    modifier.on = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Switch Remesh off"));
    assert_eq!(doc.scene().exported_shape(id).unwrap().face_count(), 6);
    assert!(!doc.scene().has_modifiers_for(id, Purpose::Export));
    // Off modifiers leave nothing to bake.
    assert!(doc.scene().can_make_editable(id));
}

#[test]
fn modifiers_are_worked_out_in_order_and_can_be_moved_and_removed() {
    let (mut doc, id) = remeshed_cube();
    add(&mut doc, id, remesh(25.0));
    let fine_then_coarse = doc.scene().shown_shape(id).unwrap();
    doc.execute(Command::MoveModifier { id, from: 1, to: 0 })
        .unwrap();
    assert_eq!(doc.undo_label(), Some("Move Remesh up"));
    assert_eq!(doc.scene().modifiers(id)[0].kind, remesh(25.0));
    let coarse_then_fine = doc.scene().shown_shape(id).unwrap();
    assert_ne!(fine_then_coarse, coarse_then_fine);
    // The last one decides how fine it is.
    assert!(coarse_then_fine.face_count() > fine_then_coarse.face_count());
    doc.execute(Command::RemoveModifier { id, index: 0 })
        .unwrap();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.scene().modifiers(id)[0].kind, remesh(10.0));
    for bad in [
        Command::RemoveModifier { id, index: 1 },
        Command::MoveModifier { id, from: 0, to: 4 },
    ] {
        assert_eq!(
            doc.execute(bad),
            Err(SceneError::Modifier(ModifierRefusal::NoSuchModifier))
        );
    }
    doc.undo();
    doc.undo();
    assert_eq!(doc.scene().modifiers(id)[0].kind, remesh(10.0));
    assert_eq!(doc.scene().shown_shape(id).unwrap(), fine_then_coarse);
}

#[test]
fn groups_and_instances_take_no_modifiers_but_instances_show_their_originals() {
    let (mut doc, id) = remeshed_cube();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    assert_eq!(
        doc.execute(Command::AddModifier {
            ids: vec![group],
            kind: remesh(10.0)
        }),
        Err(SceneError::Modifier(ModifierRefusal::NotAShape))
    );
    doc.execute(Command::Duplicate {
        ids: vec![id],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert!(doc.scene().modifiers(instance).is_empty());
    assert_eq!(
        doc.execute(Command::AddModifier {
            ids: vec![instance, id],
            kind: remesh(10.0)
        }),
        Err(SceneError::Modifier(ModifierRefusal::Instance))
    );
    assert_eq!(doc.scene().modifiers(id).len(), 1, "refused as a whole");
    assert_eq!(
        doc.scene().shown_shape(instance),
        doc.scene().shown_shape(id)
    );
    // A plain copy keeps its own.
    doc.execute(Command::Duplicate {
        ids: vec![id],
        instances: false,
    })
    .unwrap();
    let copy = doc.selection()[0];
    assert_eq!(doc.scene().modifiers(copy), doc.scene().modifiers(id));
}

#[test]
fn too_many_modifiers_are_refused() {
    let (mut doc, id) = remeshed_cube();
    for _ in 1..MAX_MODIFIERS {
        add(&mut doc, id, remesh(50.0));
    }
    assert_eq!(
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: remesh(50.0)
        }),
        Err(SceneError::Modifier(ModifierRefusal::TooMany))
    );
}

#[test]
fn an_open_shape_is_shown_unchanged_with_the_reason() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(&mut doc, plane, remesh(10.0));
    let scene = doc.scene();
    assert_eq!(
        scene.shown_shape(plane).unwrap(),
        scene.unmodified_shape(plane).unwrap()
    );
    let problems = scene.modifier_problems(plane);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].0, 0);
    assert!(problems[0].1.contains("isn't closed"), "{}", problems[0].1);
    // Baking leaves the modifier that couldn't work in place.
    doc.execute(Command::MakeEditable(vec![plane])).unwrap();
    assert_eq!(doc.scene().modifiers(plane).len(), 1);
}

#[test]
fn the_live_mirror_comes_before_the_modifiers() {
    // A cube open on its -X side, mirrored across that side: closed only
    // with its mirror, so remesh works only after it.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let left = shape
        .faces()
        .map(|(f, _)| f)
        .find(|f| shape.face_normal(*f).x < -0.9)
        .unwrap();
    doc.execute(Command::Record {
        id,
        operation: Operation::DeleteElements {
            points: Vec::new(),
            edges: Vec::new(),
            faces: vec![left],
            anchors: None,
        },
    })
    .unwrap();
    add(&mut doc, id, remesh(10.0));
    assert_eq!(doc.scene().modifier_problems(id).len(), 1, "open");
    doc.execute(Command::SetMirror {
        ids: vec![id],
        mirror: Some(LiveMirror {
            centre: Vec3::new(-50.0, 0.0, 0.0),
            ..LiveMirror::on_object_x()
        }),
    })
    .unwrap();
    assert!(doc.scene().modifier_problems(id).is_empty());
    let shown = doc.scene().shown_shape(id).unwrap();
    assert!(all_quads(&shown));
    let width = shown.bounds().unwrap().size().x;
    assert!((width - 200.0).abs() < 10.0, "both halves: {width}");
    // Apply mirror bakes the shape before the modifiers, which stay.
    let mut applied = doc.clone();
    applied.execute(Command::ApplyMirror(vec![id])).unwrap();
    assert_eq!(applied.scene().modifiers(id).len(), 1);
    assert_eq!(applied.scene().object(id).unwrap().mirror, None);
    assert!(applied.scene().modifier_problems(id).is_empty());
    // Bake into shape bakes the mirror too, then the remesh.
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    assert_eq!(object.mirror, None);
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    assert_eq!(**baked, *shown);
    doc.undo();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert!(doc.scene().object(id).unwrap().mirror.is_some());
}

#[test]
fn bake_into_shape_keeps_modifiers_that_are_off() {
    let (mut doc, id) = remeshed_cube();
    add(&mut doc, id, remesh(20.0));
    let mut off = doc.scene().modifiers(id)[1].clone();
    off.on = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 1,
        modifier: off,
    })
    .unwrap();
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.modifiers.len(), 1);
    assert!(!object.modifiers[0].on);
    assert_eq!(object.kind, ObjectKind::Mesh(expected));
}

#[test]
fn changing_a_later_modifier_keeps_the_earlier_ones_work() {
    let (mut doc, id) = remeshed_cube();
    add(&mut doc, id, remesh(20.0));
    let scene = doc.scene();
    scene.shown_shape(id).unwrap();
    let input = scene.unmodified_shape(id).unwrap();
    assert!(matches!(
        scene.stack_state(id, Purpose::Viewport, Arc::clone(&input)),
        Some(StackState::Ready(_))
    ));
    let mut later = scene.modifiers(id)[1].clone();
    later.kind = remesh(15.0);
    doc.execute(Command::ChangeModifier {
        id,
        index: 1,
        modifier: later,
    })
    .unwrap();
    let scene = doc.scene();
    let input = scene.unmodified_shape(id).unwrap();
    match scene.stack_state(id, Purpose::Viewport, input) {
        Some(StackState::Needed(job)) => assert_eq!(job.steps_kept(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn heavy_work_can_be_done_on_another_thread() {
    let (doc, id) = remeshed_cube();
    let scene = doc.scene();
    let input = scene.unmodified_shape(id).unwrap();
    let Some(StackState::Needed(job)) =
        scene.stack_state(id, Purpose::Viewport, Arc::clone(&input))
    else {
        panic!("nothing worked out yet");
    };
    assert_eq!(job.object(), id);
    assert_eq!(job.purpose(), Purpose::Viewport);
    assert!(job.work() > 0);
    let done = std::thread::spawn(move || job.run()).join().unwrap();
    assert_eq!(done.object(), id);
    let result = done.result();
    scene.store_stack(done);
    match scene.stack_state(id, Purpose::Viewport, input) {
        Some(StackState::Ready(ready)) => assert_eq!(ready.mesh, result.mesh),
        other => panic!("{other:?}"),
    }
    // A fine remesh of a big shape counts as heavy.
    let sphere = PrimitiveKind::Sphere.standard().mesh();
    assert!(remesh(1.0).work(&sphere) >= MODIFIERS_IN_BACKGROUND_FROM);
    assert!(remesh(10.0).work(&sphere) < MODIFIERS_IN_BACKGROUND_FROM);
}

#[test]
fn deleting_and_undoing_keeps_modifiers() {
    let (mut doc, id) = remeshed_cube();
    let shown = doc.scene().shown_shape(id).unwrap();
    doc.execute(Command::Delete(vec![id])).unwrap();
    assert!(doc.scene().object(id).is_none());
    doc.undo();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
    assert_eq!(doc.scene().shown_shape(id).unwrap(), shown);
}

#[test]
fn a_saved_scene_with_modifiers_is_checked() {
    let (doc, id) = remeshed_cube();
    let scene = doc.scene().clone();
    let modifiers = scene.modifiers(id).to_vec();
    let mut odd = modifiers.clone();
    odd[0].kind = remesh(-4.0);
    let fixed = scene.clone().with_modifiers(id, odd).unwrap();
    assert_eq!(
        fixed.modifiers(id)[0].kind,
        ModifierKind::Remesh(RemeshOptions {
            voxel_size: RemeshOptions::MIN_VOXEL,
            smooth: 0
        })
    );
    let too_many = vec![modifiers[0].clone(); MAX_MODIFIERS + 1];
    assert_eq!(
        scene.with_modifiers(id, too_many).err(),
        Some(ModifierRefusal::TooMany)
    );
}

#[test]
fn modifier_kinds_describe_themselves() {
    for kind in ModifierKind::standard_kinds() {
        assert!(!kind.name().is_empty());
        assert!(!kind.description().is_empty());
        assert!(ModifierFamily::ALL.contains(&kind.family()));
        assert_eq!(kind.clamped(), kind, "standard settings are within limits");
        assert!(kind.same_kind(&kind.clamped()));
        assert!(kind.sources().is_empty());
    }
    for family in ModifierFamily::ALL {
        assert!(!family.name().is_empty());
    }
    let off = Modifier {
        on: false,
        ..Modifier::new(remesh(2.0))
    };
    assert!(!off.applies(Purpose::Viewport) && !off.applies(Purpose::Export));
    let hidden = Modifier {
        in_viewport: false,
        ..Modifier::new(remesh(2.0))
    };
    assert!(!hidden.applies(Purpose::Viewport) && hidden.applies(Purpose::Export));
}
