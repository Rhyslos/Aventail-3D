//! Tests for lattice cages and the Lattice modifier (decision 0144): the
//! first modifier that reads another object, so also the stack's sources
//! (decision 0142): deleting the cage removes the modifier in the same
//! undo step, copies follow copied cages, and loops are refused.

use crate::*;
use glam::{Mat4, Vec3};

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// A cube (100 cm) with a 2×2×2 cage fitted round it: the cube and the cage.
fn caged_cube() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::AddLattice {
        ids: vec![cube],
        divisions: [2, 2, 2],
    })
    .unwrap();
    let cage = doc.selection()[0];
    (doc, cube, cage)
}

fn lattice(doc: &Document, cage: ObjectId) -> Lattice {
    doc.scene()
        .object(cage)
        .and_then(|o| o.kind.lattice())
        .cloned()
        .unwrap()
}

/// Raises the cage's top four points by `height`.
fn raise_top(doc: &mut Document, cage: ObjectId, height: f32) {
    let l = lattice(doc, cage);
    let top: Vec<u32> = (0..l.point_count())
        .filter(|i| l.rest(*i).z > 0.0)
        .map(|i| i as u32)
        .collect();
    doc.execute(Command::EditLattice {
        id: cage,
        edit: LatticeEdit::Transform {
            points: top,
            matrix: Mat4::from_translation(Vec3::Z * height),
        },
    })
    .unwrap();
}

fn height(mesh: &Mesh) -> f32 {
    mesh.bounds().unwrap().size().z
}

#[test]
fn adding_a_lattice_fits_a_cage_and_names_it_in_one_step() {
    let (mut doc, cube, cage) = caged_cube();
    let scene = doc.scene();
    assert_eq!(scene.object(cage).unwrap().name, "Lattice");
    let l = lattice(&doc, cage);
    assert_eq!(l.divisions(), [2, 2, 2]);
    let cube_box = scene.unmodified_shape(cube).unwrap().bounds().unwrap();
    assert!(l.size().distance(cube_box.size()) < 1e-3);
    let cage_box = scene.bounds_of([cage]).unwrap();
    assert!(
        cage_box
            .centre()
            .distance(scene.bounds_of([cube]).unwrap().centre())
            < 1e-3
    );
    assert_eq!(
        scene.modifiers(cube),
        &[Modifier {
            id: ModifierId(1),
            ..Modifier::new(ModifierKind::Lattice(LatticeOptions { cage }))
        }]
    );
    // At rest it changes nothing.
    assert_eq!(scene.shown_shape(cube), scene.unmodified_shape(cube));
    assert_eq!(doc.undo_label(), Some("Add lattice"));
    doc.undo();
    assert!(!doc.scene().contains(cage));
    assert!(doc.scene().modifiers(cube).is_empty());
}

#[test]
fn moving_cage_points_bends_the_shape_and_undoes() {
    let (mut doc, cube, cage) = caged_cube();
    raise_top(&mut doc, cage, 40.0);
    assert_eq!(doc.undo_label(), Some("Move 4 cage points"));
    let shown = doc.scene().shown_shape(cube).unwrap();
    assert!((height(&shown) - 140.0).abs() < 1e-2);
    assert_eq!(doc.scene().exported_shape(cube).unwrap(), shown);
    // The shape itself, as edited, is unchanged; its polygons stay.
    let before = doc.scene().unmodified_shape(cube).unwrap();
    assert!((height(&before) - 100.0).abs() < 1e-3);
    assert!(shown.same_structure(&before));
    assert!(doc.scene().modifier_problems(cube).is_empty());
    doc.undo();
    assert!((height(&doc.scene().shown_shape(cube).unwrap()) - 100.0).abs() < 1e-3);
}

#[test]
fn the_shape_follows_the_cage_as_either_moves() {
    let (mut doc, cube, cage) = caged_cube();
    raise_top(&mut doc, cage, 40.0);
    let bent = doc.scene().shown_shape(cube).unwrap();
    // Moving cage and shape together changes nothing.
    let by = Transform {
        position: Vec3::new(300.0, 0.0, 0.0),
        ..Transform::default()
    };
    let cage_at = doc.scene().object(cage).unwrap().transform;
    doc.execute(Command::SetTransforms(vec![
        (cube, by),
        (
            cage,
            Transform {
                position: cage_at.position + by.position,
                ..cage_at
            },
        ),
    ]))
    .unwrap();
    let moved = doc.scene().shown_shape(cube).unwrap();
    assert!(
        moved
            .vertices()
            .zip(bent.vertices())
            .all(|(a, b)| a.1.distance(b.1) < 1e-3)
    );
    // Moving the shape out of the cage: it follows the cage's nearest side.
    doc.execute(Command::SetTransforms(vec![(
        cube,
        Transform {
            position: Vec3::new(300.0, 0.0, 1000.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let above = doc.scene().shown_shape(cube).unwrap();
    assert!(
        (height(&above) - 100.0).abs() < 1e-2,
        "moved as one, unbent"
    );
    let rest = doc.scene().unmodified_shape(cube).unwrap();
    let lift = above.bounds().unwrap().centre().z - rest.bounds().unwrap().centre().z;
    assert!(
        (lift - 40.0).abs() < 1e-2,
        "carried up with the cage's top: {lift}"
    );
}

#[test]
fn deleting_the_cage_removes_its_modifiers_in_the_same_step() {
    let (mut doc, cube, cage) = caged_cube();
    let other = create(&mut doc, PrimitiveKind::Sphere);
    doc.execute(Command::AddModifier {
        ids: vec![other],
        kind: ModifierKind::Lattice(LatticeOptions { cage }),
    })
    .unwrap();
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: ModifierKind::Subdivide(SubdivideOptions::default()),
    })
    .unwrap();
    raise_top(&mut doc, cage, 30.0);
    doc.execute(Command::Delete(vec![cage])).unwrap();
    let scene = doc.scene();
    assert!(!scene.contains(cage));
    assert!(scene.modifiers(other).is_empty());
    assert_eq!(scene.modifiers(cube).len(), 1, "the Subdivide stays");
    assert!(matches!(
        scene.modifiers(cube)[0].kind,
        ModifierKind::Subdivide(_)
    ));
    doc.undo();
    let scene = doc.scene();
    assert!(scene.contains(cage));
    assert_eq!(scene.modifiers(cube).len(), 2);
    assert_eq!(scene.modifiers(other).len(), 1);
    assert!(height(&scene.shown_shape(other).unwrap()) > 100.0);
}

#[test]
fn copies_follow_copied_cages() {
    let (mut doc, cube, cage) = caged_cube();
    // The shape alone: its copy shares the cage.
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: false,
    })
    .unwrap();
    let alone = doc.selection()[0];
    assert_eq!(
        doc.scene().modifiers(alone)[0].kind,
        ModifierKind::Lattice(LatticeOptions { cage })
    );
    // The shape with its cage: the copy follows the copied cage.
    doc.execute(Command::Duplicate {
        ids: vec![cube, cage],
        instances: false,
    })
    .unwrap();
    let copies = doc.selection();
    let (cube_copy, cage_copy) = copies.iter().fold((None, None), |(s, c), id| {
        match doc.scene().object(*id).unwrap().kind {
            ObjectKind::Lattice(_) => (s, Some(*id)),
            _ => (Some(*id), c),
        }
    });
    let (cube_copy, cage_copy) = (cube_copy.unwrap(), cage_copy.unwrap());
    assert_eq!(
        doc.scene().modifiers(cube_copy)[0].kind,
        ModifierKind::Lattice(LatticeOptions { cage: cage_copy })
    );
    // Bending the copied cage bends only its shape.
    raise_top(&mut doc, cage_copy, 50.0);
    assert!(height(&doc.scene().shown_shape(cube_copy).unwrap()) > 149.0);
    assert!((height(&doc.scene().shown_shape(cube).unwrap()) - 100.0).abs() < 1e-3);
}

#[test]
fn loops_and_odd_sources_are_refused() {
    let (mut doc, cube, cage) = caged_cube();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let refused = |doc: &mut Document, id: ObjectId, source: ObjectId| {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: ModifierKind::Lattice(LatticeOptions { cage: source }),
        })
        .unwrap_err()
    };
    let bad = SceneError::Modifier(ModifierRefusal::BadSource);
    // A shape is no cage, nor is the object itself, nor nothing.
    assert_eq!(refused(&mut doc, cube, sphere), bad);
    assert_eq!(refused(&mut doc, cube, cube), bad);
    assert_eq!(refused(&mut doc, cube, LatticeOptions::NO_CAGE), bad);
    // A cage takes no modifiers, so it can't read itself or a shape.
    assert_eq!(
        refused(&mut doc, cage, cage),
        SceneError::Modifier(ModifierRefusal::NotAShape)
    );
    assert_eq!(
        doc.execute(Command::AddModifier {
            ids: vec![cage],
            kind: ModifierKind::Subdivide(SubdivideOptions::default()),
        }),
        Err(SceneError::Modifier(ModifierRefusal::NotAShape))
    );
    // A cage is no cutter either.
    assert!(
        doc.execute(Command::AddBoolean {
            kept: sphere,
            cutters: vec![cage],
            kind: BooleanKind::Subtract,
        })
        .is_err()
    );
    // A cage parented under its own shape still makes no loop: the cage
    // reads no shape, only its place.
    doc.execute(Command::Move {
        ids: vec![cage],
        parent: Some(cube),
        index: 0,
    })
    .unwrap();
    raise_top(&mut doc, cage, 20.0);
    assert!(height(&doc.scene().shown_shape(cube).unwrap()) > 119.0);
    // A shape whose cage it bends can't be the cage's source: nothing
    // reads a shape round in a circle (the cutter case, decision 0135).
    assert!(!doc.scene().shape_uses(cage, cube));
}

#[test]
fn groups_and_instances_get_no_lattice() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    let add = |doc: &mut Document, ids: Vec<ObjectId>| {
        doc.execute(Command::AddLattice {
            ids,
            divisions: [2, 2, 2],
        })
    };
    assert_eq!(
        add(&mut doc, vec![group]),
        Err(SceneError::Modifier(ModifierRefusal::NotAShape))
    );
    assert!(add(&mut doc, vec![]).is_err());
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert_eq!(
        add(&mut doc, vec![cube, instance]),
        Err(SceneError::Modifier(ModifierRefusal::Instance))
    );
    assert!(doc.scene().modifiers(cube).is_empty(), "nothing changed");
    assert_eq!(doc.scene().walk().len(), 3, "no cage was left behind");
}

#[test]
fn one_cage_round_several_shapes_lines_up_with_the_world() {
    let mut doc = Document::new();
    let a = create(&mut doc, PrimitiveKind::Cube);
    let b = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(
        b,
        Transform {
            position: Vec3::new(300.0, 0.0, 0.0),
            rotation: Vec3::new(0.3, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    doc.execute(Command::AddLattice {
        ids: vec![a, b],
        divisions: [3, 2, 2],
    })
    .unwrap();
    let cage = doc.selection()[0];
    assert_eq!(doc.undo_label(), Some("Add lattice round 2 objects"));
    let placed = doc.scene().object(cage).unwrap().transform;
    assert_eq!(placed.rotation, Vec3::ZERO);
    let shapes = doc.scene().bounds_of([a, b]).unwrap();
    assert!(lattice(&doc, cage).size().distance(shapes.size()) < 1e-2);
    for id in [a, b] {
        assert_eq!(doc.scene().modifiers(id).len(), 1);
    }
}

#[test]
fn one_cage_round_a_turned_shape_turns_with_it() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let turned = Transform {
        position: Vec3::new(10.0, 20.0, 30.0),
        rotation: Vec3::new(0.5, 0.2, 0.0),
        scale: Vec3::new(2.0, 1.0, 1.0),
    };
    doc.execute(Command::SetTransforms(vec![(cube, turned)]))
        .unwrap();
    doc.execute(Command::AddLattice {
        ids: vec![cube],
        divisions: [2, 2, 2],
    })
    .unwrap();
    let cage = doc.selection()[0];
    let placed = doc.scene().object(cage).unwrap().transform;
    assert!(placed.rotation.distance(turned.rotation) < 1e-4);
    assert!(placed.scale.distance(turned.scale) < 1e-4);
    // Raising the top raises it along the shape's own up.
    raise_top(&mut doc, cage, 30.0);
    let shown = doc.scene().shown_shape(cube).unwrap();
    assert!((height(&shown) - 130.0).abs() < 1e-2);
}

#[test]
fn bake_into_shape_keeps_the_bend_and_the_cage() {
    let (mut doc, cube, cage) = caged_cube();
    raise_top(&mut doc, cage, 25.0);
    let bent = doc.scene().exported_shape(cube).unwrap();
    doc.execute(Command::MakeEditable(vec![cube])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(cube).is_empty());
    assert_eq!(scene.object(cube).unwrap().kind, ObjectKind::Mesh(bent));
    assert!(scene.contains(cage));
    // A cage itself has nothing to bake.
    assert!(!scene.can_make_editable(cage));
}

#[test]
fn a_cage_has_no_shape_and_keeps_its_place_on_apply_transforms() {
    let (mut doc, _, cage) = caged_cube();
    let scene = doc.scene();
    assert!(scene.evaluated(cage).is_none());
    assert!(scene.exported_shape(cage).is_none());
    assert!(!scene.object(cage).unwrap().kind.has_shape());
    assert!(scene.local_bounds(cage).is_some());
    let at = scene.object(cage).unwrap().transform;
    doc.execute(Command::ApplyTransforms {
        ids: vec![cage],
        parts: ApplyParts::ALL,
    })
    .unwrap();
    assert_eq!(doc.scene().object(cage).unwrap().transform, at);
    assert!(
        doc.execute(Command::Record {
            id: cage,
            operation: Operation::MovePoints {
                points: vec![VertexId::from_raw(0)],
                offset: Vec3::X,
                anchors: None,
            },
        })
        .is_err()
    );
}

#[test]
fn cage_edits_are_checked() {
    let (mut doc, cube, cage) = caged_cube();
    let edit = |doc: &mut Document, id, edit| doc.execute(Command::EditLattice { id, edit });
    assert!(
        edit(&mut doc, cube, LatticeEdit::Reset).is_err(),
        "not a cage"
    );
    let nowhere = LatticeEdit::Transform {
        points: vec![0],
        matrix: Mat4::from_translation(Vec3::NAN),
    };
    assert!(edit(&mut doc, cage, nowhere).is_err());
    raise_top(&mut doc, cage, 40.0);
    edit(&mut doc, cage, LatticeEdit::Divisions([4, 4, 4])).unwrap();
    assert_eq!(doc.undo_label(), Some("Change lattice divisions"));
    assert_eq!(lattice(&doc, cage).point_count(), 64);
    let shown = doc.scene().shown_shape(cube).unwrap();
    assert!((height(&shown) - 140.0).abs() < 1.0, "the bend is kept");
    edit(
        &mut doc,
        cage,
        LatticeEdit::Interpolation(Interpolation::Linear),
    )
    .unwrap();
    assert_eq!(lattice(&doc, cage).interpolation(), Interpolation::Linear);
    edit(&mut doc, cage, LatticeEdit::Reset).unwrap();
    assert_eq!(doc.undo_label(), Some("Reset cage points"));
    assert!(lattice(&doc, cage).is_at_rest());
    let count = doc.undo_steps();
    edit(&mut doc, cage, LatticeEdit::Reset).unwrap();
    assert_eq!(doc.undo_steps(), count, "nothing changed, no step");
}

#[test]
fn a_flattened_cage_says_why() {
    let (mut doc, cube, cage) = caged_cube();
    raise_top(&mut doc, cage, 40.0);
    let at = doc.scene().object(cage).unwrap().transform;
    doc.execute(Command::SetTransforms(vec![(
        cage,
        Transform {
            scale: Vec3::new(1.0, 0.0, 1.0),
            ..at
        },
    )]))
    .unwrap();
    let problems = doc.scene().modifier_problems(cube);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].1.contains("flattened"));
    assert_eq!(
        doc.scene().shown_shape(cube),
        doc.scene().unmodified_shape(cube)
    );
}

#[test]
fn an_untouched_cage_is_fitted_again_round_a_resized_shape() {
    let (mut doc, id, cage) = caged_cube();
    assert!(doc.scene().cage_fits(cage, id));
    // The cube made twice as tall: the cage no longer fits, until fitted
    // again, in one undo step.
    let mut tall = PrimitiveKind::Cube.standard();
    if let Primitive::Cube(c) = &mut tall {
        c.size.z *= 2.0;
    }
    doc.execute(Command::SetPrimitive {
        ids: vec![id],
        primitive: tall,
    })
    .unwrap();
    assert!(!doc.scene().cage_fits(cage, id));
    let steps = doc.undo_steps();
    doc.execute(Command::FitCage { cage, shape: id }).unwrap();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Fit cage"));
    assert!(doc.scene().cage_fits(cage, id));
    assert!((lattice(&doc, cage).size().z - 200.0).abs() < 0.01);
    // The shape is as it was: a cage at rest bends nothing.
    let shape = doc.scene().shown_shape(id).unwrap();
    assert!((height(&shape) - 200.0).abs() < 0.01);
    doc.undo();
    assert!((lattice(&doc, cage).size().z - 100.0).abs() < 0.01);
}

#[test]
fn a_moved_or_shared_cage_doesnt_count_as_untouched() {
    let (mut doc, id, cage) = caged_cube();
    raise_top(&mut doc, cage, 30.0);
    assert!(!doc.scene().cage_fits(cage, id), "its points moved");
    let (mut doc, id, cage) = caged_cube();
    let mut moved = doc.scene().object(cage).unwrap().transform;
    moved.position.x += 40.0;
    doc.execute(Command::SetTransforms(vec![(cage, moved)]))
        .unwrap();
    assert!(!doc.scene().cage_fits(cage, id), "the cage moved");
    // Not a cage, or no such object: no, and fitting is refused.
    assert!(!doc.scene().cage_fits(id, id));
    assert!(
        doc.execute(Command::FitCage {
            cage: id,
            shape: id
        })
        .is_err()
    );
    assert!(
        doc.execute(Command::FitCage {
            cage: ObjectId(9999),
            shape: id
        })
        .is_err()
    );
}
