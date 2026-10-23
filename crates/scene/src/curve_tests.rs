//! Tests for curve objects (decision 0138): their solids, edits through
//! commands, undo, cross-sections from other curves, baking, copies and
//! applying transforms.

use crate::*;
use glam::{Mat4, Vec3};

/// Adds a curve object of sharp corners through `points`; returns its id.
fn add_curve(doc: &mut Document, points: &[Vec3], closed: bool) -> ObjectId {
    let curve = Curve::corners(points, closed).unwrap();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
        CurveShape::Points(curve),
    ))))
    .unwrap();
    doc.selection()[0]
}

fn ready(doc: &mut Document, kind: ReadyCurveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
        CurveShape::Ready(kind.standard()),
    ))))
    .unwrap();
    doc.selection()[0]
}

fn sweep(doc: &mut Document, id: ObjectId, sweep: Sweep) {
    doc.execute(Command::SetCurveSolid {
        ids: vec![id],
        solid: CurveSolid::Sweep(sweep),
    })
    .unwrap();
}

fn curve_of(doc: &Document, id: ObjectId) -> CurveObject {
    doc.scene()
        .object(id)
        .unwrap()
        .kind
        .curve()
        .unwrap()
        .clone()
}

fn points(doc: &Document, id: ObjectId) -> Vec<Vec3> {
    curve_of(doc, id)
        .shape
        .curve()
        .points()
        .iter()
        .map(|p| p.position)
        .collect()
}

fn faces(doc: &Document, id: ObjectId) -> usize {
    doc.scene().evaluated(id).map_or(0, |e| e.mesh.face_count())
}

fn bar() -> Vec<Vec3> {
    vec![Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0)]
}

#[test]
fn a_curve_on_its_own_is_a_line_with_no_shape() {
    let mut doc = Document::new();
    let id = add_curve(&mut doc, &bar(), false);
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.name, "Curve");
    assert!(object.kind.has_shape());
    assert!(doc.scene().evaluated(id).is_none(), "a line has no mesh");
    assert!(doc.scene().shown_shape(id).is_none());
    // Its box is its line's, for framing.
    let bounds = doc.scene().local_bounds(id).unwrap();
    assert_eq!(bounds.size().x, 100.0);
    assert!(!doc.scene().can_make_editable(id));
}

#[test]
fn a_sweep_is_live_and_undoes() {
    let mut doc = Document::new();
    let id = add_curve(&mut doc, &bar(), false);
    sweep(&mut doc, id, Sweep::default());
    assert_eq!(doc.undo_label(), Some("Sweep"));
    let first = faces(&doc, id);
    assert!(first > 0);
    // Moving a point updates the shape.
    let end = CurvePart::point(CurvePointId::from_raw(1));
    doc.execute(Command::EditCurve {
        id,
        edit: CurveEdit::Transform {
            parts: vec![end],
            matrix: Mat4::from_translation(Vec3::new(0.0, 0.0, 50.0)),
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Move curve points"));
    let raised = doc.scene().local_bounds(id).unwrap();
    assert!(raised.max.z > 50.0);
    doc.undo();
    assert!(doc.scene().local_bounds(id).unwrap().max.z < 10.0);
    doc.undo();
    assert!(doc.scene().evaluated(id).is_none());
    doc.redo();
    assert_eq!(faces(&doc, id), first);
    // Wider settings are kept within limits.
    sweep(
        &mut doc,
        id,
        Sweep {
            width: -4.0,
            sides: 1000,
            ..Sweep::default()
        },
    );
    match curve_of(&doc, id).solid {
        CurveSolid::Sweep(s) => {
            assert_eq!(s.width, mesh::curve::sweep::MIN_SIZE);
            assert_eq!(s.sides, 64);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_lathe_spins_the_curve() {
    let mut doc = Document::new();
    let id = add_curve(
        &mut doc,
        &[
            Vec3::ZERO,
            Vec3::new(30.0, 0.0, 0.0),
            Vec3::new(20.0, 0.0, 60.0),
        ],
        false,
    );
    doc.execute(Command::SetCurveSolid {
        ids: vec![id],
        solid: CurveSolid::Lathe(LatheShape::default()),
    })
    .unwrap();
    let b = doc.scene().local_bounds(id).unwrap();
    assert!((b.size().y - 60.0).abs() < 1.0, "spun round Z: {b:?}");
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    assert!(mesh.face_count() > 24);
}

#[test]
fn deleting_points_and_too_few_points() {
    let mut doc = Document::new();
    let id = add_curve(
        &mut doc,
        &[Vec3::ZERO, Vec3::X * 10.0, Vec3::X * 20.0],
        false,
    );
    sweep(&mut doc, id, Sweep::default());
    let before = faces(&doc, id);
    doc.execute(Command::EditCurve {
        id,
        edit: CurveEdit::Delete {
            parts: vec![CurvePart::point(CurvePointId::from_raw(1))],
        },
    })
    .unwrap();
    assert_eq!(points(&doc, id).len(), 2);
    assert!(faces(&doc, id) <= before);
    let steps = doc.undo_steps();
    let refused = doc.execute(Command::EditCurve {
        id,
        edit: CurveEdit::Delete {
            parts: vec![CurvePart::point(CurvePointId::from_raw(2))],
        },
    });
    assert!(matches!(refused, Err(SceneError::Curve(_))));
    assert_eq!(doc.undo_steps(), steps, "a refusal records nothing");
    doc.undo();
    assert_eq!(points(&doc, id).len(), 3);
}

#[test]
fn smooth_and_sharp_points() {
    let mut doc = Document::new();
    let id = add_curve(
        &mut doc,
        &[
            Vec3::ZERO,
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
        ],
        false,
    );
    let middle = CurvePointId::from_raw(1);
    let smooth = |doc: &mut Document, smooth| {
        doc.execute(Command::EditCurve {
            id,
            edit: CurveEdit::SetSmooth {
                points: vec![middle],
                smooth,
            },
        })
        .unwrap();
    };
    smooth(&mut doc, true);
    assert_eq!(doc.undo_label(), Some("Make points smooth"));
    let curve = curve_of(&doc, id).shape.curve();
    assert!(curve.point(middle).unwrap().smooth);
    assert!(curve.point(middle).unwrap().has_handles());
    smooth(&mut doc, false);
    assert!(
        !curve_of(&doc, id)
            .shape
            .curve()
            .point(middle)
            .unwrap()
            .has_handles()
    );
    doc.undo();
    assert!(
        curve_of(&doc, id)
            .shape
            .curve()
            .point(middle)
            .unwrap()
            .smooth
    );
    // The same change again records nothing.
    let steps = doc.undo_steps();
    smooth(&mut doc, true);
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn ready_made_curves_keep_their_settings_until_their_points_are_edited() {
    let mut doc = Document::new();
    let id = ready(&mut doc, ReadyCurveKind::Circle);
    assert_eq!(doc.scene().object(id).unwrap().name, "Circle");
    doc.execute(Command::SetReadyCurve {
        ids: vec![id],
        ready: ReadyCurve::Circle { radius: 20.0 },
    })
    .unwrap();
    assert!((doc.scene().local_bounds(id).unwrap().size().x - 40.0).abs() < 1e-3);
    // Another kind's settings are refused.
    assert!(matches!(
        doc.execute(Command::SetReadyCurve {
            ids: vec![id],
            ready: ReadyCurve::Line { length: 5.0 },
        }),
        Err(SceneError::WrongKind(_))
    ));
    doc.execute(Command::EditCurve {
        id,
        edit: CurveEdit::SetClosed(false),
    })
    .unwrap();
    match &curve_of(&doc, id).shape {
        CurveShape::Points(curve) => assert!(!curve.closed()),
        other => panic!("{other:?}"),
    }
    doc.undo();
    assert!(matches!(curve_of(&doc, id).shape, CurveShape::Ready(_)));
    // A line can't be a ready-made curve's settings, nor a cube's.
    let cube = {
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        doc.selection()[0]
    };
    assert!(
        doc.execute(Command::EditCurve {
            id: cube,
            edit: CurveEdit::SetClosed(true),
        })
        .is_err()
    );
}

#[test]
fn edits_are_refused_on_curves() {
    let mut doc = Document::new();
    let id = add_curve(&mut doc, &bar(), false);
    sweep(&mut doc, id, Sweep::default());
    let refused = doc.execute(Command::Record {
        id,
        operation: Operation::FlipPolygons {
            faces: vec![FaceId::from_raw(0)],
            anchors: None,
        },
    });
    assert!(matches!(refused, Err(SceneError::Curve(_))));
    assert!(!refused.unwrap_err().to_string().is_empty());
    // No edit layer either.
    doc.execute(Command::AddLayer(vec![id])).unwrap();
    assert!(doc.scene().object(id).unwrap().stack.is_empty());
}

#[test]
fn a_cross_section_from_another_curve_follows_it() {
    let mut doc = Document::new();
    let path = add_curve(&mut doc, &bar(), false);
    let profile = add_curve(
        &mut doc,
        &[
            Vec3::new(-5.0, -5.0, 0.0),
            Vec3::new(5.0, -5.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
        ],
        true,
    );
    sweep(
        &mut doc,
        path,
        Sweep {
            profile: SweepProfile::Curve(profile),
            ..Sweep::default()
        },
    );
    // A triangle carried along: three sides and two caps.
    assert_eq!(faces(&doc, path), 3 + 2);
    // Moving the profile's point changes the sweep.
    let wide = doc.scene().local_bounds(path).unwrap().size();
    doc.execute(Command::EditCurve {
        id: profile,
        edit: CurveEdit::Transform {
            parts: vec![CurvePart::point(CurvePointId::from_raw(2))],
            matrix: Mat4::from_translation(Vec3::new(0.0, 20.0, 0.0)),
        },
    })
    .unwrap();
    let wider = doc.scene().local_bounds(path).unwrap().size();
    assert!(wider.max_element() > wide.max_element() || wider != wide);
    // A curve can't be its own cross-section, nor a cube one.
    let refused = doc.execute(Command::SetCurveSolid {
        ids: vec![path],
        solid: CurveSolid::Sweep(Sweep {
            profile: SweepProfile::Curve(path),
            ..Sweep::default()
        }),
    });
    assert!(matches!(refused, Err(SceneError::Curve(_))));
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    assert!(
        doc.execute(Command::SetCurveSolid {
            ids: vec![path],
            solid: CurveSolid::Sweep(Sweep {
                profile: SweepProfile::Curve(cube),
                ..Sweep::default()
            }),
        })
        .is_err()
    );
    // Deleting the cross-section gives a round one, in the same step.
    doc.execute(Command::Delete(vec![profile])).unwrap();
    match curve_of(&doc, path).solid {
        CurveSolid::Sweep(s) => assert_eq!(s.profile, SweepProfile::Circle),
        other => panic!("{other:?}"),
    }
    assert!(faces(&doc, path) > 0);
    doc.undo();
    match curve_of(&doc, path).solid {
        CurveSolid::Sweep(s) => assert_eq!(s.profile, SweepProfile::Curve(profile)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn copies_of_a_path_and_its_cross_section_go_together() {
    let mut doc = Document::new();
    let path = add_curve(&mut doc, &bar(), false);
    let profile = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X * 5.0, Vec3::Y * 5.0], true);
    sweep(
        &mut doc,
        path,
        Sweep {
            profile: SweepProfile::Curve(profile),
            ..Sweep::default()
        },
    );
    doc.execute(Command::Duplicate {
        ids: vec![path, profile],
        instances: false,
    })
    .unwrap();
    let copies = doc.selection();
    let copy_path = copies
        .iter()
        .copied()
        .find(|id| curve_of(&doc, *id).solid != CurveSolid::Line)
        .unwrap();
    let copy_profile = copies.iter().copied().find(|id| *id != copy_path).unwrap();
    assert_eq!(
        curve_of(&doc, copy_path).solid.profile_object(),
        Some(copy_profile)
    );
    // A copy of the path alone keeps the original cross-section.
    doc.execute(Command::Duplicate {
        ids: vec![path],
        instances: false,
    })
    .unwrap();
    let alone = doc.selection()[0];
    assert_eq!(curve_of(&doc, alone).solid.profile_object(), Some(profile));
}

#[test]
fn a_pasted_curve_with_a_missing_cross_section_goes_round() {
    let mut doc = Document::new();
    let mut object = Object::new(ObjectKind::Curve(CurveObject {
        shape: CurveShape::Points(Curve::corners(&bar(), false).unwrap()),
        solid: CurveSolid::Sweep(Sweep {
            profile: SweepProfile::Curve(ObjectId::from_raw(999)),
            ..Sweep::default()
        }),
    }));
    object.name = "Pasted".into();
    doc.execute(Command::InsertMany(vec![object])).unwrap();
    let id = doc.selection()[0];
    match curve_of(&doc, id).solid {
        CurveSolid::Sweep(s) => assert_eq!(s.profile, SweepProfile::Circle),
        other => panic!("{other:?}"),
    }
}

#[test]
fn bake_into_shape_makes_a_plain_mesh() {
    let mut doc = Document::new();
    let id = add_curve(&mut doc, &bar(), false);
    // A line has nothing to bake.
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Curve(_)
    ));
    sweep(&mut doc, id, Sweep::default());
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    match &doc.scene().object(id).unwrap().kind {
        ObjectKind::Mesh(mesh) => assert_eq!(mesh.face_count(), shape.face_count()),
        other => panic!("{other:?}"),
    }
    doc.undo();
    assert!(matches!(
        doc.scene().object(id).unwrap().kind,
        ObjectKind::Curve(_)
    ));
}

#[test]
fn applying_transforms_moves_the_points() {
    let mut doc = Document::new();
    let id = add_curve(&mut doc, &bar(), false);
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: Vec3::new(0.0, 0.0, 10.0),
            rotation: Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0),
            scale: Vec3::splat(2.0),
        },
    )]))
    .unwrap();
    let world_before: Vec<Vec3> = points(&doc, id)
        .iter()
        .map(|p| doc.scene().shape_matrix(id).transform_point3(*p))
        .collect();
    doc.execute(Command::ApplyTransforms {
        ids: vec![id],
        parts: ApplyParts::ROTATION_AND_SCALE,
    })
    .unwrap();
    let object = doc.scene().object(id).unwrap();
    assert_eq!(object.transform.scale, Vec3::ONE);
    let world_after: Vec<Vec3> = points(&doc, id)
        .iter()
        .map(|p| doc.scene().shape_matrix(id).transform_point3(*p))
        .collect();
    for (a, b) in world_before.iter().zip(&world_after) {
        assert!(a.distance(*b) < 1e-3, "{a} {b}");
    }
    assert!(doc.scene().object(id).unwrap().stack.is_empty());
}

#[test]
fn instances_and_booleans_use_the_solid() {
    let mut doc = Document::new();
    let ring = ready(&mut doc, ReadyCurveKind::Circle);
    sweep(
        &mut doc,
        ring,
        Sweep {
            width: 20.0,
            ..Sweep::default()
        },
    );
    doc.execute(Command::Duplicate {
        ids: vec![ring],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert_eq!(faces(&doc, instance), faces(&doc, ring));
    // The ring cuts a cube: a closed, outward solid works as a cutter.
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    doc.execute(Command::AddBoolean {
        kept: cube,
        cutters: vec![ring],
        kind: BooleanKind::Subtract,
    })
    .unwrap();
    assert!(
        doc.scene().boolean_problem(cube).is_none(),
        "{:?}",
        doc.scene().boolean_problem(cube)
    );
    let cut = doc.scene().cut_shape(cube).unwrap().unwrap();
    assert!(cut.face_count() > 6);
}

#[test]
fn saved_scenes_with_curves_are_checked() {
    let curve = || CurveObject::line(CurveShape::Points(Curve::corners(&bar(), false).unwrap()));
    let mut with_edit = Object::new(ObjectKind::Curve(curve()));
    with_edit.stack = {
        let mut doc = Document::new();
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        doc.execute(Command::Record {
            id,
            operation: Operation::FlipPolygons {
                faces: vec![FaceId::from_raw(0)],
                anchors: None,
            },
        })
        .unwrap();
        doc.scene().object(id).unwrap().stack.clone()
    };
    assert!(Scene::from_objects([(ObjectId::from_raw(1), None, with_edit)], 2).is_err());
    let lost = Object::new(ObjectKind::Curve(CurveObject {
        solid: CurveSolid::Sweep(Sweep {
            profile: SweepProfile::Curve(ObjectId::from_raw(7)),
            ..Sweep::default()
        }),
        ..curve()
    }));
    assert!(Scene::from_objects([(ObjectId::from_raw(1), None, lost)], 2).is_err());
    // Restoring a curve onto a group stored in its place.
    let group = Object::new(ObjectKind::Group);
    let scene = Scene::from_objects([(ObjectId::from_raw(1), None, group)], 2).unwrap();
    let scene = scene.with_curve(ObjectId::from_raw(1), curve()).unwrap();
    assert!(matches!(
        scene.object(ObjectId::from_raw(1)).unwrap().kind,
        ObjectKind::Curve(_)
    ));
    assert!(scene.with_curve(ObjectId::from_raw(1), curve()).is_err());
}

/// A small random number maker, the same every run.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn random_curve_commands_undo_and_redo_exactly() {
    let mut dice = Dice(0xc0ffee);
    for _ in 0..20 {
        let mut doc = Document::new();
        let a = add_curve(
            &mut doc,
            &[Vec3::ZERO, Vec3::X * 50.0, Vec3::Y * 50.0],
            false,
        );
        let b = ready(&mut doc, ReadyCurveKind::ALL[(dice.next() % 4) as usize]);
        let start = doc.scene().clone();
        for _ in 0..25 {
            let id = if dice.next().is_multiple_of(2) { a } else { b };
            let ids: Vec<CurvePointId> = curve_of(&doc, id)
                .shape
                .curve()
                .points()
                .iter()
                .map(|p| p.id)
                .collect();
            let pick = ids[(dice.next() as usize) % ids.len().max(1)];
            let command = match dice.next() % 6 {
                0 => Command::EditCurve {
                    id,
                    edit: CurveEdit::Transform {
                        parts: vec![CurvePart::point(pick)],
                        matrix: Mat4::from_translation(Vec3::splat((dice.next() % 50) as f32)),
                    },
                },
                1 => Command::EditCurve {
                    id,
                    edit: CurveEdit::SetSmooth {
                        points: vec![pick],
                        smooth: dice.next().is_multiple_of(2),
                    },
                },
                2 => Command::EditCurve {
                    id,
                    edit: CurveEdit::Delete {
                        parts: vec![CurvePart::point(pick)],
                    },
                },
                3 => Command::SetCurveSolid {
                    ids: vec![id],
                    solid: CurveSolid::Sweep(Sweep {
                        profile: if dice.next().is_multiple_of(2) {
                            SweepProfile::Curve(if id == a { b } else { a })
                        } else {
                            SweepProfile::Square
                        },
                        ..Sweep::default()
                    }),
                },
                4 => Command::SetCurveSolid {
                    ids: vec![id],
                    solid: CurveSolid::Lathe(LatheShape::default()),
                },
                _ => Command::EditCurve {
                    id,
                    edit: CurveEdit::SetClosed(dice.next().is_multiple_of(2)),
                },
            };
            let _ = doc.execute(command);
            let _ = doc.scene().evaluated(a);
            let _ = doc.scene().evaluated(b);
        }
        let end = doc.scene().clone();
        while doc.undo().is_some() {}
        // Back to where the two curves were made.
        assert!(doc.undo_steps() == 0);
        doc.redo();
        doc.redo();
        assert_eq!(*doc.scene(), start);
        while doc.redo().is_some() {}
        assert_eq!(*doc.scene(), end);
        assert_eq!(
            doc.scene().evaluated(a).map(|e| e.mesh.face_count()),
            end.evaluated(a).map(|e| e.mesh.face_count())
        );
    }
}
