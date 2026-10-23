//! Tests for live booleans (decision 0135): objects cut by other objects,
//! worked out whenever the shape is shown.

use crate::*;
use glam::Vec3;
use std::sync::Arc;

/// Adds a standard cube (100 cm) at `at`; returns its id.
fn cube(doc: &mut Document, at: Vec3) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    move_to(doc, id, at);
    id
}

fn move_to(doc: &mut Document, id: ObjectId, at: Vec3) {
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: at,
            ..Transform::default()
        },
    )]))
    .unwrap();
}

fn subtract(doc: &mut Document, kept: ObjectId, cutters: &[ObjectId]) -> Result<(), SceneError> {
    doc.execute(Command::AddBoolean {
        kept,
        cutters: cutters.to_vec(),
        kind: BooleanKind::Subtract,
    })
}

/// The volume a closed mesh holds.
fn volume(mesh: &Mesh) -> f32 {
    let mut total = 0.0;
    for (_, corners) in mesh.faces() {
        mesh::face_triangles(mesh, corners, |[a, b, c]| {
            let p = |i: usize| mesh.position(corners[i]).unwrap();
            total += p(a).dot(p(b).cross(p(c))) / 6.0;
        });
    }
    total
}

fn shown_volume(doc: &Document, id: ObjectId) -> f32 {
    volume(&doc.scene().shown_shape(id).unwrap())
}

/// A kept cube at the centre and a cutter cube overlapping one corner.
fn notched() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    let kept = cube(&mut doc, Vec3::ZERO);
    let cutter = cube(&mut doc, Vec3::splat(50.0));
    subtract(&mut doc, kept, &[cutter]).unwrap();
    (doc, kept, cutter)
}

#[test]
fn a_subtract_cuts_the_shown_shape_and_undoes() {
    let (mut doc, kept, cutter) = notched();
    assert_eq!(doc.undo_label(), Some("Subtract"));
    assert_eq!(doc.selection(), [kept], "the kept object is selected");
    assert!((shown_volume(&doc, kept) - 875_000.0).abs() < 10.0);
    // Its edits still work on the uncut shape.
    assert_eq!(doc.scene().evaluated(kept).unwrap().mesh.face_count(), 6);
    assert!(doc.scene().is_cutter(cutter));
    assert_eq!(doc.scene().cut_by(cutter), [kept]);
    assert!(!doc.scene().is_cutter(kept));
    doc.undo().unwrap();
    assert!(doc.scene().booleans(kept).is_empty());
    assert!((shown_volume(&doc, kept) - 1_000_000.0).abs() < 10.0);
    assert!(!doc.scene().is_cutter(cutter));
    doc.redo().unwrap();
    assert_eq!(doc.scene().booleans(kept).len(), 1);
    assert!((shown_volume(&doc, kept) - 875_000.0).abs() < 10.0);
}

#[test]
fn moving_the_cutter_moves_the_cut() {
    let (mut doc, kept, cutter) = notched();
    move_to(&mut doc, cutter, Vec3::new(50.0, 0.0, 0.0));
    assert!((shown_volume(&doc, kept) - 500_000.0).abs() < 10.0);
    move_to(&mut doc, cutter, Vec3::new(500.0, 0.0, 0.0));
    assert!((shown_volume(&doc, kept) - 1_000_000.0).abs() < 10.0);
    // Moving the kept object moves it against the cutter too.
    move_to(&mut doc, kept, Vec3::new(450.0, 0.0, 0.0));
    assert!((shown_volume(&doc, kept) - 500_000.0).abs() < 10.0);
}

#[test]
fn editing_the_cutter_changes_the_cut() {
    let (mut doc, kept, cutter) = notched();
    let mut bigger = PrimitiveKind::Cube.standard();
    if let Primitive::Cube(c) = &mut bigger {
        c.size = Vec3::new(100.0, 100.0, 300.0);
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![cutter],
        primitive: bigger,
    })
    .unwrap();
    // Cubes stand on their pivot: lowered, the cutter now runs through
    // the whole height, and a quarter goes.
    move_to(&mut doc, cutter, Vec3::new(50.0, 50.0, -100.0));
    assert!((shown_volume(&doc, kept) - 750_000.0).abs() < 10.0);
}

#[test]
fn hidden_cutters_still_cut() {
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::SetVisible {
        ids: vec![cutter],
        visible: false,
    })
    .unwrap();
    assert!((shown_volume(&doc, kept) - 875_000.0).abs() < 10.0);
}

#[test]
fn deleting_a_cutter_ends_its_cut_in_one_undo_step() {
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::Delete(vec![cutter])).unwrap();
    assert!(doc.scene().booleans(kept).is_empty());
    assert!((shown_volume(&doc, kept) - 1_000_000.0).abs() < 10.0);
    doc.undo().unwrap();
    assert_eq!(doc.scene().booleans(kept)[0].cutters, [cutter]);
    assert!((shown_volume(&doc, kept) - 875_000.0).abs() < 10.0);
    // Deleting the kept object leaves the cutter an ordinary object.
    doc.execute(Command::Delete(vec![kept])).unwrap();
    assert!(!doc.scene().is_cutter(cutter));
}

#[test]
fn deleting_one_of_two_cutters_keeps_the_other() {
    let mut doc = Document::new();
    let kept = cube(&mut doc, Vec3::ZERO);
    let a = cube(&mut doc, Vec3::splat(50.0));
    let b = cube(&mut doc, Vec3::splat(-50.0));
    subtract(&mut doc, kept, &[a, b]).unwrap();
    assert!((shown_volume(&doc, kept) - 750_000.0).abs() < 10.0);
    doc.execute(Command::Delete(vec![a])).unwrap();
    assert_eq!(doc.scene().booleans(kept)[0].cutters, [b]);
    assert!((shown_volume(&doc, kept) - 875_000.0).abs() < 10.0);
}

#[test]
fn loops_self_cuts_groups_and_instances_are_refused() {
    let (mut doc, kept, cutter) = notched();
    let refused = |r: Result<(), SceneError>| match r {
        Err(SceneError::Boolean(why)) => why,
        other => panic!("not refused: {other:?}"),
    };
    assert_eq!(
        refused(subtract(&mut doc, cutter, &[kept])),
        BooleanRefusal::Loop
    );
    assert_eq!(
        refused(subtract(&mut doc, kept, &[kept])),
        BooleanRefusal::CutsItself
    );
    assert_eq!(
        refused(subtract(&mut doc, kept, &[])),
        BooleanRefusal::NoCutter
    );
    // A longer loop: C cuts the cutter, then the kept object can't cut C.
    let c = cube(&mut doc, Vec3::new(0.0, 0.0, 300.0));
    subtract(&mut doc, cutter, &[c]).unwrap();
    assert_eq!(
        refused(subtract(&mut doc, c, &[kept])),
        BooleanRefusal::Loop
    );
    // Groups have no shape.
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    assert_eq!(
        refused(subtract(&mut doc, kept, &[group])),
        BooleanRefusal::NotAShape
    );
    assert_eq!(
        refused(subtract(&mut doc, group, &[c])),
        BooleanRefusal::NotAShape
    );
    // An instance can't be cut, and an instance of the kept object can't
    // cut it.
    doc.execute(Command::Duplicate {
        ids: vec![kept],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert_eq!(
        refused(subtract(&mut doc, instance, &[c])),
        BooleanRefusal::Instance
    );
    assert_eq!(
        refused(subtract(&mut doc, kept, &[instance])),
        BooleanRefusal::Loop
    );
    // Refusals change nothing and say why.
    assert_eq!(doc.scene().booleans(kept).len(), 1);
    assert!(
        SceneError::Boolean(BooleanRefusal::Loop)
            .to_string()
            .contains("wait")
    );
}

#[test]
fn an_instance_shows_its_originals_cut() {
    let (mut doc, kept, _) = notched();
    doc.execute(Command::Duplicate {
        ids: vec![kept],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert!(doc.scene().booleans(instance).is_empty());
    assert!((shown_volume(&doc, instance) - 875_000.0).abs() < 10.0);
    // Deleting the original gives the instance its own copy, cut.
    doc.execute(Command::Delete(vec![kept])).unwrap();
    assert!((shown_volume(&doc, instance) - 875_000.0).abs() < 10.0);
}

#[test]
fn the_same_kind_again_adds_to_the_last_boolean() {
    let (mut doc, kept, cutter) = notched();
    let more = cube(&mut doc, Vec3::splat(-50.0));
    subtract(&mut doc, kept, &[more, cutter]).unwrap();
    let list = doc.scene().booleans(kept);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].cutters, [cutter, more]);
    let union = cube(&mut doc, Vec3::new(0.0, 0.0, 100.0));
    doc.execute(Command::AddBoolean {
        kept,
        cutters: vec![union],
        kind: BooleanKind::Union,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Union"));
    let list = doc.scene().booleans(kept);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].kind, BooleanKind::Union);
    assert!((shown_volume(&doc, kept) - 1_750_000.0).abs() < 20.0);
}

#[test]
fn a_boolean_can_change_kind_or_go() {
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::SetBooleans {
        id: kept,
        booleans: vec![Boolean {
            kind: BooleanKind::Intersect,
            cutters: vec![cutter],
        }],
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change booleans"));
    assert!((shown_volume(&doc, kept) - 125_000.0).abs() < 10.0);
    doc.execute(Command::SetBooleans {
        id: kept,
        booleans: vec![Boolean {
            kind: BooleanKind::Union,
            cutters: Vec::new(),
        }],
    })
    .unwrap();
    assert!(doc.scene().booleans(kept).is_empty(), "empty ones go");
    assert!(!doc.scene().is_cutter(cutter));
}

#[test]
fn a_cutter_that_isnt_closed_leaves_the_shape_uncut_and_says_why() {
    let mut doc = Document::new();
    let kept = cube(&mut doc, Vec3::ZERO);
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Plane.standard(),
    )))
    .unwrap();
    let plane = doc.selection()[0];
    subtract(&mut doc, kept, &[plane]).unwrap();
    let problem = doc.scene().boolean_problem(kept).unwrap();
    assert!(problem.contains("Plane"), "{problem}");
    assert!(problem.contains("closed solid"), "{problem}");
    assert!((shown_volume(&doc, kept) - 1_000_000.0).abs() < 10.0);
    // A healthy boolean has no problem to report.
    let (doc, kept, _) = notched();
    assert_eq!(doc.scene().boolean_problem(kept), None);
}

#[test]
fn the_cut_is_kept_until_something_changes() {
    let (mut doc, kept, cutter) = notched();
    let first = doc.scene().shown_shape(kept).unwrap();
    let again = doc.scene().shown_shape(kept).unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    move_to(&mut doc, cutter, Vec3::splat(40.0));
    let moved = doc.scene().shown_shape(kept).unwrap();
    assert!(!Arc::ptr_eq(&first, &moved));
}

#[test]
fn bake_into_shape_makes_the_cut_permanent_and_hides_the_cutter() {
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::MakeEditable(vec![kept])).unwrap();
    let scene = doc.scene();
    assert!(scene.booleans(kept).is_empty());
    assert!(matches!(
        scene.object(kept).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
    assert!((volume(&scene.evaluated(kept).unwrap().mesh) - 875_000.0).abs() < 10.0);
    assert!(
        !scene.object(cutter).unwrap().visible,
        "the cutter is hidden"
    );
    assert!(scene.contains(cutter), "and kept");
    doc.undo().unwrap();
    assert_eq!(doc.scene().booleans(kept).len(), 1);
    assert!(doc.scene().object(cutter).unwrap().visible);
    // A plain mesh with only booleans can be baked too.
    doc.execute(Command::MakeEditable(vec![kept])).unwrap();
    doc.undo().unwrap();
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::MakeEditable(vec![kept])).unwrap();
    doc.undo().unwrap();
    assert!(doc.scene().can_make_editable(kept));
    let _ = cutter;
}

#[test]
fn a_cutter_used_elsewhere_stays_shown_after_a_bake() {
    let (mut doc, kept, cutter) = notched();
    let other = cube(&mut doc, Vec3::new(100.0, 100.0, 0.0));
    subtract(&mut doc, other, &[cutter]).unwrap();
    doc.execute(Command::MakeEditable(vec![kept])).unwrap();
    assert!(doc.scene().object(cutter).unwrap().visible);
    assert_eq!(doc.scene().cut_by(cutter), [other]);
}

#[test]
fn applying_the_mirror_bakes_the_cut_too() {
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::SetMirror {
        ids: vec![kept],
        mirror: Some(LiveMirror::on_object_x()),
    })
    .unwrap();
    let shown = shown_volume(&doc, kept);
    doc.execute(Command::ApplyMirror(vec![kept])).unwrap();
    assert!(doc.scene().booleans(kept).is_empty());
    assert!((shown_volume(&doc, kept) - shown).abs() < 20.0);
    assert!(!doc.scene().object(cutter).unwrap().visible);
}

#[test]
fn duplicating_the_kept_object_and_its_cutter_together_cuts_the_copy_with_the_copy() {
    let (mut doc, kept, cutter) = notched();
    doc.execute(Command::Duplicate {
        ids: vec![kept, cutter],
        instances: false,
    })
    .unwrap();
    let copies = doc.selection();
    let copy = *copies
        .iter()
        .find(|id| !doc.scene().booleans(**id).is_empty())
        .unwrap();
    let cutter_copy = *copies.iter().find(|id| **id != copy).unwrap();
    assert_eq!(doc.scene().booleans(copy)[0].cutters, [cutter_copy]);
    // Only the kept object copied: the same cutter cuts both.
    doc.execute(Command::Duplicate {
        ids: vec![kept],
        instances: false,
    })
    .unwrap();
    let alone = doc.selection()[0];
    assert_eq!(doc.scene().booleans(alone)[0].cutters, [cutter]);
    assert_eq!(doc.scene().cut_by(cutter).len(), 2);
}

#[test]
fn a_cutter_that_is_itself_cut_cuts_with_its_cut_shape() {
    let (mut doc, kept, cutter) = notched();
    // A third cube takes part of the cutter away, so less of the kept
    // cube goes.
    let inner = cube(&mut doc, Vec3::new(25.0, 25.0, 75.0));
    let mut small = PrimitiveKind::Cube.standard();
    if let Primitive::Cube(c) = &mut small {
        c.size = Vec3::splat(50.0);
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![inner],
        primitive: small,
    })
    .unwrap();
    subtract(&mut doc, cutter, &[inner]).unwrap();
    // Cubes stand on their pivot: the inner one takes the top half of the
    // notch back.
    assert!((shown_volume(&doc, kept) - 937_500.0).abs() < 20.0);
}

#[test]
fn cutter_materials_join_the_kept_objects_by_name() {
    let (mut doc, kept, cutter) = notched();
    let red = Material {
        name: "Red".into(),
        color: [1.0, 0.0, 0.0],
    };
    doc.execute(Command::SetMaterials {
        id: kept,
        materials: vec![Material::standard(), red.clone()],
    })
    .unwrap();
    doc.execute(Command::SetMaterials {
        id: cutter,
        materials: vec![red],
    })
    .unwrap();
    let shown = doc.scene().shown_shape(kept).unwrap();
    let slots: Vec<u32> = shown
        .faces()
        .map(|(f, _)| mesh::material::slot_of(&shown, f))
        .collect();
    assert_eq!(slots.iter().filter(|s| **s == 1).count(), 3, "{slots:?}");
}

#[test]
fn saved_booleans_are_checked_when_a_scene_is_rebuilt() {
    let (doc, kept, cutter) = notched();
    let scene = doc.scene().clone();
    let looped = scene.clone().with_booleans(
        cutter,
        vec![Boolean {
            kind: BooleanKind::Union,
            cutters: vec![kept],
        }],
    );
    assert_eq!(looped.err(), Some(BooleanRefusal::Loop));
    let missing = scene.with_booleans(
        kept,
        vec![Boolean {
            kind: BooleanKind::Union,
            cutters: vec![ObjectId::from_raw(999)],
        }],
    );
    assert_eq!(missing.err(), Some(BooleanRefusal::NotAShape));
}

#[test]
fn the_selection_remembers_its_order() {
    let mut doc = Document::new();
    let a = cube(&mut doc, Vec3::ZERO);
    let b = cube(&mut doc, Vec3::X * 200.0);
    let c = cube(&mut doc, Vec3::X * 400.0);
    // The panel lists the newest first; the order is the clicks'.
    doc.set_selection([b]);
    doc.set_selection([b, a]);
    doc.set_selection([a, b, c]);
    assert_eq!(doc.selection_order(), [b, a, c]);
    doc.toggle_selected(a);
    assert_eq!(doc.selection_order(), [b, c]);
    doc.toggle_selected(a);
    assert_eq!(doc.selection_order(), [b, c, a]);
    doc.undo().unwrap();
    assert_eq!(doc.selection_order(), [b, c]);
    doc.redo().unwrap();
    assert_eq!(doc.selection_order(), [b, c, a]);
    doc.execute(Command::Delete(vec![c])).unwrap();
    assert!(!doc.selection_order().contains(&c));
}

#[test]
fn a_cut_can_be_worked_out_elsewhere_and_handed_back() {
    let (mut doc, kept, cutter) = notched();
    move_to(&mut doc, cutter, Vec3::splat(40.0));
    let Some(CutState::Needed(job)) = doc.scene().cut_state(kept) else {
        panic!("not worked out yet");
    };
    assert_eq!(job.object(), kept);
    assert_eq!(job.polygons(), 12);
    let done = std::thread::spawn(move || job.run()).join().unwrap();
    assert_eq!(done.object(), kept);
    let made = done.result().unwrap();
    doc.scene().store_cut(done);
    match doc.scene().cut_state(kept) {
        Some(CutState::Ready(Ok(ready))) => assert!(Arc::ptr_eq(&ready, &made)),
        other => panic!("not kept: {other:?}"),
    }
    // A cut for older places isn't taken for the new ones.
    move_to(&mut doc, cutter, Vec3::splat(30.0));
    assert!(matches!(
        doc.scene().cut_state(kept),
        Some(CutState::Needed(_))
    ));
    assert!(doc.scene().cut_state(cutter).is_none(), "no booleans");
}
