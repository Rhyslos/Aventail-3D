//! Tests for the Levels panel's levels (decision 0165): modifier ids,
//! each level's changes to the modifiers, levels made by hand, and undo
//! for all of it.

use crate::*;
use mesh::decimate::triangle_count;
use std::sync::Arc;

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

fn lods(doc: &Document, id: ObjectId) -> Lods {
    doc.scene().object(id).unwrap().lods.clone()
}

fn set(doc: &mut Document, id: ObjectId, lods: Lods) {
    doc.execute(Command::SetLods {
        ids: vec![id],
        lods,
    })
    .unwrap();
}

/// Levels on, none of them reduced, so each shows exactly its own shape.
fn unreduced(count: usize) -> Lods {
    let mut lods = Lods {
        on: true,
        ..Lods::default()
    }
    .with_count(count);
    for level in &mut lods.levels {
        level.reduce = false;
    }
    lods
}

fn counts(doc: &Document, id: ObjectId) -> Vec<usize> {
    let made = doc.scene().lod_shapes(id).unwrap();
    assert!(made.problem.is_none(), "{:?}", made.problem);
    made.triangles()
}

fn check_round_trip(doc: &mut Document) {
    let mut states = vec![doc.scene().clone()];
    while doc.undo().is_some() {
        states.push(doc.scene().clone());
    }
    states.pop();
    while let Some(expected) = states.pop() {
        doc.redo().unwrap();
        assert_eq!(*doc.scene(), expected);
    }
}

/// A sphere smoothed by Subdivide, levels on and unreduced.
fn smoothed_sphere(doc: &mut Document, levels: usize) -> (ObjectId, ModifierId) {
    let sphere = create(doc, PrimitiveKind::Sphere);
    add(
        doc,
        sphere,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    set(doc, sphere, unreduced(levels));
    (sphere, doc.scene().modifiers(sphere)[0].id)
}

#[test]
fn modifiers_get_ids_that_stay_as_they_move_and_change() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        cube,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(
        &mut doc,
        cube,
        ModifierKind::Remesh(RemeshOptions::default()),
    );
    let ids: Vec<ModifierId> = doc.scene().modifiers(cube).iter().map(|m| m.id).collect();
    assert_eq!(ids, [ModifierId(1), ModifierId(2)]);
    doc.execute(Command::MoveModifier {
        id: cube,
        from: 1,
        to: 0,
    })
    .unwrap();
    let moved: Vec<ModifierId> = doc.scene().modifiers(cube).iter().map(|m| m.id).collect();
    assert_eq!(moved, [ModifierId(2), ModifierId(1)]);
    // A change keeps the id, whatever id it is given.
    let mut changed = doc.scene().modifiers(cube)[0].clone();
    changed.on = false;
    changed.id = ModifierId(77);
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: changed,
    })
    .unwrap();
    assert_eq!(doc.scene().modifiers(cube)[0].id, ModifierId(2));
    // Repeated and missing ids are given new ones, in order.
    let list = modifier::with_ids(vec![
        Modifier::new(ModifierKind::Remesh(RemeshOptions::default())),
        Modifier {
            id: ModifierId(4),
            ..Modifier::new(ModifierKind::Remesh(RemeshOptions::default()))
        },
        Modifier {
            id: ModifierId(4),
            ..Modifier::new(ModifierKind::Remesh(RemeshOptions::default()))
        },
    ]);
    let given: Vec<u32> = list.iter().map(|m| m.id.0).collect();
    assert_eq!(given, [5, 4, 6]);
    assert!(modifier::with_ids(Vec::new()).is_empty());
}

#[test]
fn a_change_applies_only_to_its_own_level() {
    let mut doc = Document::new();
    let (sphere, subdivide) = smoothed_sphere(&mut doc, 2);
    let plain = triangle_count(&doc.scene().unmodified_shape(sphere).unwrap());
    let smoothed = triangle_count(&doc.scene().exported_shape(sphere).unwrap());
    assert!(smoothed > plain);
    assert_eq!(counts(&doc, sphere), [smoothed, smoothed]);
    // Level 1 without Subdivide.
    let mut changed = lods(&doc, sphere);
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    changed.levels[0] = changed.levels[0].with_override(&modifiers, subdivide, false, None);
    set(&mut doc, sphere, changed);
    assert_eq!(counts(&doc, sphere), [plain, smoothed]);
    // Level 2 smoothed once instead of twice.
    let mut once = lods(&doc, sphere);
    once.levels[1] = once.levels[1].with_override(
        &modifiers,
        subdivide,
        true,
        Some(ModifierKind::Subdivide(SubdivideOptions {
            export_levels: 1,
            ..SubdivideOptions::default()
        })),
    );
    set(&mut doc, sphere, once);
    let [first, second] = counts(&doc, sphere)[..] else {
        panic!("two levels");
    };
    assert_eq!(first, plain);
    assert!(
        plain < second && second < smoothed,
        "{plain} {second} {smoothed}"
    );
    // The model itself is unchanged.
    assert_eq!(
        triangle_count(&doc.scene().exported_shape(sphere).unwrap()),
        smoothed
    );
    // It stays live: a bigger sphere changes every level.
    let mut bigger = PrimitiveKind::Sphere.standard();
    if let Primitive::Sphere(p) = &mut bigger {
        p.sides += 8;
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![sphere],
        primitive: bigger,
    })
    .unwrap();
    let now = counts(&doc, sphere);
    assert!(now[0] > first && now[1] > second, "{now:?}");
    check_round_trip(&mut doc);
}

#[test]
fn a_change_that_changes_nothing_is_not_kept() {
    let mut doc = Document::new();
    let (sphere, subdivide) = smoothed_sphere(&mut doc, 1);
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    let level = lods(&doc, sphere).levels[0].clone();
    let same = level.with_override(&modifiers, subdivide, true, Some(modifiers[0].kind.clone()));
    assert!(same.overrides.is_empty());
    // An unknown modifier, or settings of another kind, mean nothing.
    let unknown = level.with_override(&modifiers, ModifierId(99), false, None);
    assert!(unknown.overrides.is_empty());
    let other_kind = level.with_override(
        &modifiers,
        subdivide,
        true,
        Some(ModifierKind::Remesh(RemeshOptions::default())),
    );
    assert!(other_kind.overrides.is_empty());
    // The command tidies what it is given too.
    let mut wild = lods(&doc, sphere);
    wild.levels[0].overrides = vec![
        ModifierOverride {
            modifier: ModifierId(99),
            on: false,
            settings: None,
        },
        ModifierOverride {
            modifier: subdivide,
            on: false,
            settings: None,
        },
        ModifierOverride {
            modifier: subdivide,
            on: true,
            settings: None,
        },
    ];
    set(&mut doc, sphere, wild);
    assert_eq!(
        lods(&doc, sphere).levels[0].overrides,
        [ModifierOverride {
            modifier: subdivide,
            on: false,
            settings: None,
        }]
    );
}

#[test]
fn changes_follow_their_modifier_and_go_with_it() {
    let mut doc = Document::new();
    let (sphere, subdivide) = smoothed_sphere(&mut doc, 1);
    add(
        &mut doc,
        sphere,
        ModifierKind::Triangulate(TriangulateOptions::default()),
    );
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    let mut off = lods(&doc, sphere);
    off.levels[0] = off.levels[0].with_override(&modifiers, subdivide, false, None);
    set(&mut doc, sphere, off);
    let before = counts(&doc, sphere);
    // Moving Subdivide after Triangulate: the change goes with it.
    doc.execute(Command::MoveModifier {
        id: sphere,
        from: 0,
        to: 1,
    })
    .unwrap();
    assert_eq!(
        lods(&doc, sphere).levels[0].overrides[0].modifier,
        subdivide
    );
    assert_eq!(counts(&doc, sphere), before);
    // Removing it removes the change, in the same step.
    doc.execute(Command::RemoveModifier {
        id: sphere,
        index: 1,
    })
    .unwrap();
    assert!(lods(&doc, sphere).levels[0].overrides.is_empty());
    doc.undo().unwrap();
    assert_eq!(lods(&doc, sphere).levels[0].overrides.len(), 1);
    assert_eq!(doc.scene().modifiers(sphere).len(), 2);
    // A new modifier never takes a removed one's changes.
    doc.redo().unwrap();
    add(
        &mut doc,
        sphere,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    assert!(lods(&doc, sphere).levels[0].overrides.is_empty());
    check_round_trip(&mut doc);
}

#[test]
fn changes_on_an_instance_name_its_originals_modifiers() {
    let mut doc = Document::new();
    let (sphere, subdivide) = smoothed_sphere(&mut doc, 1);
    doc.execute(Command::Duplicate {
        ids: vec![sphere],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    let mut off = lods(&doc, instance);
    off.on = true;
    off.levels[0] = off.levels[0].with_override(&modifiers, subdivide, false, None);
    set(&mut doc, instance, off);
    let plain = triangle_count(&doc.scene().unmodified_shape(sphere).unwrap());
    assert_eq!(counts(&doc, instance), [plain]);
    // The original's Subdivide removed: the instance's change goes too.
    doc.execute(Command::RemoveModifier {
        id: sphere,
        index: 0,
    })
    .unwrap();
    assert!(lods(&doc, instance).levels[0].overrides.is_empty());
}

#[test]
fn a_level_with_changes_reuses_the_exports_steps_before_them() {
    let mut doc = Document::new();
    let (sphere, _) = smoothed_sphere(&mut doc, 1);
    add(
        &mut doc,
        sphere,
        ModifierKind::Triangulate(TriangulateOptions::default()),
    );
    let triangulate = doc.scene().modifiers(sphere)[1].id;
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    let mut off = lods(&doc, sphere);
    off.levels[0] = off.levels[0].with_override(&modifiers, triangulate, false, None);
    set(&mut doc, sphere, off);
    let scene = doc.scene();
    scene.exported_shape(sphere).unwrap();
    let input = scene.unmodified_shape(sphere).unwrap();
    let Some(StackState::Ready(_)) = scene.stack_state(sphere, Purpose::Level(0), input) else {
        panic!("the export's Subdivide is the level's");
    };
}

#[test]
fn heavy_levels_are_work_for_another_thread() {
    let mut doc = Document::new();
    let mut fine = PrimitiveKind::Sphere.standard();
    if let Primitive::Sphere(p) = &mut fine {
        p.sides = 64;
        p.rings = 32;
    }
    doc.execute(Command::Create(ObjectKind::Primitive(fine)))
        .unwrap();
    let sphere = doc.selection()[0];
    add(
        &mut doc,
        sphere,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    let mut changed = Lods {
        on: true,
        ..Lods::default()
    };
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    changed.levels[0] = changed.levels[0].with_override(&modifiers, modifiers[0].id, false, None);
    set(&mut doc, sphere, changed);
    // Asking is quick: nothing is worked out until the job runs.
    let Some(LodState::Needed(job)) = doc.scene().lod_state(sphere) else {
        panic!("work to do");
    };
    assert!(job.work() >= LODS_IN_BACKGROUND_FROM, "{}", job.work());
    let done = std::thread::spawn(move || job.run()).join().unwrap();
    doc.scene().store_lods(done);
    assert!(matches!(
        doc.scene().lod_state(sphere),
        Some(LodState::Ready(_))
    ));
    assert!(doc.scene().lods_if_ready(sphere).is_some());
}

/// A cube with three unreduced levels, the second made by hand.
fn hand_made(doc: &mut Document) -> (ObjectId, ObjectId) {
    let cube = create(doc, PrimitiveKind::Cube);
    set(doc, cube, unreduced(3));
    doc.execute(Command::MakeLevelByHand { id: cube, level: 1 })
        .unwrap();
    let shape = doc.selection()[0];
    (cube, shape)
}

#[test]
fn a_level_made_by_hand_is_a_shape_inside_its_object() {
    let mut doc = Document::new();
    let (cube, shape) = hand_made(&mut doc);
    let scene = doc.scene();
    assert_ne!(shape, cube);
    assert_eq!(scene.parent(shape), Some(cube));
    assert_eq!(scene.object(shape).unwrap().name, "Cube LOD2");
    assert_eq!(scene.level_owner(shape), Some((cube, 1)));
    assert!(scene.is_level_shape(shape));
    assert!(!scene.is_level_shape(cube));
    assert_eq!(scene.level_shape(cube, 1), Some((shape, None)));
    assert_eq!(scene.level_drifted(cube, 1), Some(false));
    assert_eq!(scene.level_drifted(cube, 0), None);
    // A copy of what it showed.
    assert_eq!(
        *scene.evaluated(shape).unwrap().mesh,
        *scene.exported_shape(cube).unwrap()
    );
    assert_eq!(
        doc.undo().as_deref(),
        Some(words::tr("Make level 2 by hand"))
    );
    assert!(!doc.scene().contains(shape));
    assert_eq!(lods(&doc, cube).levels[1].hand, None);
    doc.redo().unwrap();
    // Asking again changes nothing.
    let before = doc.scene().clone();
    doc.execute(Command::MakeLevelByHand { id: cube, level: 1 })
        .unwrap();
    assert_eq!(*doc.scene(), before);
    // Refused without levels, or for a level that isn't there.
    assert!(
        doc.execute(Command::MakeLevelByHand { id: cube, level: 7 })
            .is_err()
    );
    let other = create(&mut doc, PrimitiveKind::Cube);
    assert!(
        doc.execute(Command::MakeLevelByHand {
            id: other,
            level: 0
        })
        .is_err()
    );
    check_round_trip(&mut doc);
}

#[test]
fn editing_a_hand_made_level_changes_only_it_and_the_model_drifts_away() {
    let mut doc = Document::new();
    let (cube, shape) = hand_made(&mut doc);
    let full = counts(&doc, cube);
    // Its polygons on top deleted: the level has fewer, the model as it
    // was.
    let top: Vec<FaceId> = doc
        .scene()
        .evaluated(shape)
        .unwrap()
        .mesh
        .faces()
        .map(|(f, _)| f)
        .take(1)
        .collect();
    doc.execute(Command::Record {
        id: shape,
        operation: Operation::DeleteElements {
            points: Vec::new(),
            edges: Vec::new(),
            faces: top,
            anchors: None,
        },
    })
    .unwrap();
    let now = counts(&doc, cube);
    assert_eq!(now[0], full[0]);
    assert_eq!(now[1], full[1] - 2);
    assert_eq!(now[2], full[2]);
    assert_eq!(doc.scene().level_drifted(cube, 1), Some(false));
    // The model changes: the hand-made level doesn't follow, and says so.
    let mut bigger = PrimitiveKind::Cube.standard();
    if let Primitive::Cube(p) = &mut bigger {
        p.segments[0] += 1;
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: bigger,
    })
    .unwrap();
    let after = counts(&doc, cube);
    assert!(after[0] > full[0]);
    assert_eq!(after[1], now[1]);
    assert_eq!(doc.scene().level_drifted(cube, 1), Some(true));
    // Started again from the model: its edit gone, the note too.
    doc.execute(Command::RestartLevel { id: cube, level: 1 })
        .unwrap();
    assert_eq!(counts(&doc, cube), [after[0], after[0], after[0]]);
    assert_eq!(doc.scene().level_drifted(cube, 1), Some(false));
    assert!(doc.scene().object(shape).unwrap().stack.is_empty());
    assert!(
        doc.execute(Command::RestartLevel { id: cube, level: 0 })
            .is_err()
    );
    check_round_trip(&mut doc);
}

#[test]
fn a_hand_made_level_placed_off_the_model_is_placed_back_into_it() {
    let mut doc = Document::new();
    let (cube, shape) = hand_made(&mut doc);
    let before = doc.scene().lod_shapes(cube).unwrap().levels[1].bounds();
    doc.execute(Command::SetTransforms(vec![(
        shape,
        Transform {
            position: glam::Vec3::new(100.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let (_, place) = doc.scene().level_shape(cube, 1).unwrap();
    assert!(place.is_some());
    let made = doc.scene().lod_shapes(cube).unwrap();
    let moved = made.levels[1].bounds().unwrap();
    assert!((moved.min.x - before.unwrap().min.x - 100.0).abs() < 1e-3);
    // Asked again, the same shapes (the placed one is kept).
    assert!(Arc::ptr_eq(&doc.scene().lod_shapes(cube).unwrap(), &made));
}

#[test]
fn deleting_or_moving_out_a_hand_made_level_makes_it_from_the_model_again() {
    let mut doc = Document::new();
    let (cube, shape) = hand_made(&mut doc);
    doc.execute(Command::Delete(vec![shape])).unwrap();
    assert_eq!(lods(&doc, cube).levels[1].hand, None);
    doc.undo().unwrap();
    assert!(lods(&doc, cube).levels[1].hand.is_some());
    assert!(doc.scene().is_level_shape(shape));
    // Dragged out of its object: an ordinary shape.
    doc.execute(Command::Move {
        ids: vec![shape],
        parent: None,
        index: 0,
    })
    .unwrap();
    assert!(!doc.scene().is_level_shape(shape));
    assert_eq!(doc.scene().level_shape(cube, 1), None);
    doc.undo().unwrap();
    assert!(doc.scene().is_level_shape(shape));
    // Fewer levels take the shape of a level that goes with them.
    let one = lods(&doc, cube).with_count(1);
    set(&mut doc, cube, one);
    assert!(!doc.scene().contains(shape));
    doc.undo().unwrap();
    assert!(doc.scene().is_level_shape(shape));
    // Changing the other settings keeps it.
    let mut farther = lods(&doc, cube);
    farther.levels[1].distance = 9_000.0;
    farther.levels[1].hand = None;
    set(&mut doc, cube, farther);
    assert!(doc.scene().is_level_shape(shape));
    // Deleting the object takes it too.
    doc.execute(Command::Delete(vec![cube])).unwrap();
    assert!(!doc.scene().contains(shape));
    check_round_trip(&mut doc);
}

#[test]
fn copies_have_their_own_hand_made_levels() {
    let mut doc = Document::new();
    let (cube, shape) = hand_made(&mut doc);
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: false,
    })
    .unwrap();
    let copy = doc.selection()[0];
    let (copied_shape, _) = doc.scene().level_shape(copy, 1).unwrap();
    assert_ne!(copied_shape, shape);
    assert_eq!(doc.scene().parent(copied_shape), Some(copy));
    // Pasted as a whole object, its hand-made levels don't come with it.
    let object = doc.scene().object(cube).unwrap().clone();
    doc.execute(Command::Insert {
        object,
        place: scene_top(),
    })
    .unwrap();
    let pasted = doc.selection()[0];
    assert!(lods(&doc, pasted).levels.iter().all(|l| l.hand.is_none()));
}

fn scene_top() -> Place {
    Place {
        parent: None,
        index: 0,
    }
}

#[test]
fn a_level_just_made_by_hand_of_a_smoothed_shape_has_not_drifted() {
    let mut doc = Document::new();
    let (sphere, _) = smoothed_sphere(&mut doc, 3);
    set(
        &mut doc,
        sphere,
        Lods {
            on: true,
            ..Lods::default()
        },
    );
    doc.execute(Command::MakeLevelByHand {
        id: sphere,
        level: 0,
    })
    .unwrap();
    doc.set_selection([sphere]);
    assert_eq!(doc.scene().level_drifted(sphere, 0), Some(false));
}
