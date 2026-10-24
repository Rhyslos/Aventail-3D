//! Tests for collision shapes in the scene (decision 0151).

use crate::*;
use glam::Vec3;
use mesh::primitives::DomeParams;
use std::sync::Arc;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn set(doc: &mut Document, id: ObjectId, collision: Collision) {
    doc.execute(Command::SetCollision {
        ids: vec![id],
        collision,
    })
    .unwrap();
}

/// The object's collision shapes for its shape as the viewport shows it.
fn shapes(doc: &Document, id: ObjectId) -> Option<Arc<CollisionShapes>> {
    let scene = doc.scene();
    scene.collision_shapes(id, scene.shown_shape(id)?)
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

#[test]
fn only_wrapped_parts_and_boxes_have_shapes_of_their_own() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    assert!(shapes(&doc, cube).is_none(), "no collision");
    set(&mut doc, cube, Collision::Mesh);
    assert!(shapes(&doc, cube).is_none(), "the shape itself");
    for collision in [Collision::Convex, Collision::Parts, Collision::Box] {
        set(&mut doc, cube, collision);
        let found = shapes(&doc, cube).unwrap();
        assert_eq!(found.hulls.len(), 1, "{collision:?}");
        assert_eq!(found.hulls[0].points.len(), 8);
        assert!(found.problem.is_none());
    }
}

#[test]
fn an_arch_is_wrapped_in_parts_that_leave_its_opening_free() {
    let mut doc = Document::new();
    let arch = create(&mut doc, PrimitiveKind::Arch);
    set(&mut doc, arch, Collision::Parts);
    assert_eq!(
        doc.undo_label(),
        Some("Collision: wrapped in parts (convex)")
    );
    let parts = shapes(&doc, arch).unwrap();
    assert!(parts.hulls.len() >= 3, "{}", parts.hulls.len());
    assert!(parts.hulls.len() <= ConvexPartsOptions::STANDARD_MAX_PARTS as usize);
    // Every point of the arch is in a part.
    let shape = doc.scene().shown_shape(arch).unwrap();
    for (_, p) in shape.vertices() {
        assert!(parts.hulls.iter().any(|h| h.contains(p, 0.01)), "{p}");
    }
    // The middle of the opening, low down, is free.
    let bounds = shape.bounds().unwrap();
    let opening = Vec3::new(bounds.centre().x, bounds.centre().y, bounds.min.z + 1.0);
    assert!(!parts.hulls.iter().any(|h| h.contains(opening, 0.0)));
    // Wrapped as one, it isn't.
    set(&mut doc, arch, Collision::Convex);
    let one = shapes(&doc, arch).unwrap();
    assert!(one.hulls[0].contains(opening, 0.0));
}

#[test]
fn the_parts_setting_is_an_undo_step_clamped_and_kept() {
    let mut doc = Document::new();
    let torus = create(&mut doc, PrimitiveKind::Torus);
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    set(&mut doc, torus, Collision::Parts);
    doc.execute(Command::SetCollisionParts {
        ids: vec![torus, group],
        options: ConvexPartsOptions {
            max_parts: 1_000,
            detail: f32::NAN,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change the collision parts"));
    let object = doc.scene().object(torus).unwrap();
    assert_eq!(
        object.collision_parts,
        ConvexPartsOptions {
            max_parts: ConvexPartsOptions::MOST_PARTS,
            detail: ConvexPartsOptions::STANDARD_DETAIL,
        }
    );
    assert_eq!(
        doc.scene().object(group).unwrap().collision_parts,
        ConvexPartsOptions::default()
    );
    // Choosing another collision keeps the settings for later.
    set(&mut doc, torus, Collision::Box);
    assert_eq!(
        doc.scene().object(torus).unwrap().collision_parts.max_parts,
        ConvexPartsOptions::MOST_PARTS
    );
    // The same again is no step.
    let steps = doc.undo_steps();
    doc.execute(Command::SetCollisionParts {
        ids: vec![torus],
        options: doc.scene().object(torus).unwrap().collision_parts,
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
    check_round_trip(&mut doc);
    assert!(
        doc.execute(Command::SetCollisionParts {
            ids: vec![ObjectId::from_raw(999)],
            options: ConvexPartsOptions::default(),
        })
        .is_err()
    );
}

#[test]
fn max_parts_holds_and_changes_are_worked_out_again() {
    let mut doc = Document::new();
    let torus = create(&mut doc, PrimitiveKind::Torus);
    set(&mut doc, torus, Collision::Parts);
    let scene = doc.scene();
    let shape = scene.shown_shape(torus).unwrap();
    // Not kept yet: work to do, heavy enough for another thread.
    let Some(CollisionState::Needed(job)) = scene.collision_state(torus, Arc::clone(&shape)) else {
        panic!("nothing to do");
    };
    assert!(job.work() >= COLLISION_IN_BACKGROUND_FROM);
    let done = job.run();
    assert!(done.shapes().hulls.len() <= 8);
    scene.store_collision(done);
    assert!(matches!(
        scene.collision_state(torus, Arc::clone(&shape)),
        Some(CollisionState::Ready(_))
    ));
    for max_parts in [1, 3, 5] {
        doc.execute(Command::SetCollisionParts {
            ids: vec![torus],
            options: ConvexPartsOptions {
                max_parts,
                ..ConvexPartsOptions::default()
            },
        })
        .unwrap();
        let scene = doc.scene();
        assert!(matches!(
            scene.collision_state(torus, Arc::clone(&shape)),
            Some(CollisionState::Needed(_))
        ));
        let found = shapes(&doc, torus).unwrap();
        assert!(!found.hulls.is_empty() && found.hulls.len() <= max_parts as usize);
    }
}

#[test]
fn an_open_shape_collides_as_one_wrapped_shape_and_says_why() {
    let mut doc = Document::new();
    // A dome without its base is open underneath.
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Dome(
        DomeParams {
            base: false,
            ..DomeParams::default()
        },
    ))))
    .unwrap();
    let dome = doc.selection()[0];
    set(&mut doc, dome, Collision::Parts);
    assert!(
        doc.scene().collision_problem(dome).is_none(),
        "not worked out yet"
    );
    let found = shapes(&doc, dome).unwrap();
    assert_eq!(found.hulls.len(), 1);
    let problem = found.problem.clone().unwrap();
    assert!(problem.contains("isn't closed"), "{problem}");
    assert_eq!(doc.scene().collision_problem(dome), Some(problem));
    // Another setting: the old reason no longer applies.
    set(&mut doc, dome, Collision::Convex);
    assert!(doc.scene().collision_problem(dome).is_none());
}

#[test]
fn a_flat_shape_has_no_wrapping() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    for collision in [Collision::Convex, Collision::Parts] {
        set(&mut doc, plane, collision);
        let found = shapes(&doc, plane).unwrap();
        assert!(found.hulls.is_empty());
        assert!(found.problem.clone().unwrap().contains("flat"));
    }
}

#[test]
fn a_box_is_square_to_the_pivot() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    set(&mut doc, cube, Collision::Box);
    // Turn the pivot 45° round Z: the shape stays, its box turns.
    let transform = Transform {
        rotation: Vec3::new(std::f32::consts::FRAC_PI_4, 0.0, 0.0),
        ..doc.scene().object(cube).unwrap().transform
    };
    doc.execute(Command::MovePivots(vec![(cube, transform)]))
        .unwrap();
    let found = shapes(&doc, cube).unwrap();
    let hull = &found.hulls[0];
    assert_eq!(hull.points.len(), 8);
    // Square to the turned pivot, round a cube 100 cm across: 141 cm
    // across, so seen square to the cube its corners reach 100 cm out.
    let width = hull.points.iter().map(|p| p.x).fold(f32::MIN, f32::max)
        - hull.points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
    assert!((width - 200.0).abs() < 0.1, "{width}");
    assert!((hull.volume() - 141.421_36_f64.powi(2) * 100.0).abs() < 10.0);
}

#[test]
fn deleting_an_object_forgets_its_shapes() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    set(&mut doc, cube, Collision::Convex);
    assert!(shapes(&doc, cube).is_some());
    doc.execute(Command::Delete(vec![cube])).unwrap();
    assert!(doc.scene().collision_problem(cube).is_none());
    doc.undo().unwrap();
    assert!(shapes(&doc, cube).is_some());
}
