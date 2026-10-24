//! Tests for spline editing through commands (decision 0139): joining ends
//! and whole curves, splitting, the new point edits, undo and redo, and
//! ids that stay unique and are never reused.

use crate::*;
use glam::{Mat4, Vec3};

fn add_curve(doc: &mut Document, points: &[Vec3], closed: bool) -> ObjectId {
    let curve = Curve::corners(points, closed).unwrap();
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
        CurveShape::Points(curve),
    ))))
    .unwrap();
    doc.selection()[0]
}

fn curve(doc: &Document, id: ObjectId) -> Curve {
    doc.scene()
        .object(id)
        .unwrap()
        .kind
        .curve()
        .unwrap()
        .shape
        .curve()
}

/// Where curve `id`'s points are in the world.
fn world_points(doc: &Document, id: ObjectId) -> Vec<Vec3> {
    let matrix = doc.scene().shape_matrix(id);
    curve(doc, id)
        .points()
        .iter()
        .map(|p| matrix.transform_point3(p.position))
        .collect()
}

fn id(n: u32) -> CurvePointId {
    CurvePointId::from_raw(n)
}

fn ids_unique(curve: &Curve) {
    let mut ids: Vec<u32> = curve.points().iter().map(|p| p.id.get()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), curve.points().len());
}

/// The same objects, settings and places (the scene's count of ids given
/// out never goes back, so whole scenes differ after undo).
fn same_objects(a: &Scene, b: &Scene) {
    assert_eq!(a.walk(), b.walk());
    for (id, _) in a.walk() {
        assert_eq!(a.object(id), b.object(id), "{id:?}");
    }
}

fn moved(doc: &mut Document, id: ObjectId, by: Vec3) {
    doc.execute(Command::SetTransforms(vec![(
        id,
        Transform {
            position: by,
            ..Transform::default()
        },
    )]))
    .unwrap();
}

#[test]
fn welding_two_curves_ends_joins_them_into_the_first() {
    let mut doc = Document::new();
    let a = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X * 100.0], false);
    let b = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X * 100.0], false);
    // The second sits further along, in its own place.
    moved(&mut doc, b, Vec3::new(120.0, 0.0, 0.0));
    let before = doc.scene().clone();
    doc.execute(Command::JoinCurveEnds {
        first: (a, id(1)),
        second: (b, id(0)),
        weld: true,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Weld curve ends"));
    assert!(!doc.scene().contains(b));
    assert_eq!(doc.selection(), vec![a]);
    let world = world_points(&doc, a);
    assert_eq!(world.len(), 3);
    assert!(world[1].distance(Vec3::new(110.0, 0.0, 0.0)) < 1e-3);
    assert!(world[2].distance(Vec3::new(220.0, 0.0, 0.0)) < 1e-3);
    ids_unique(&curve(&doc, a));
    doc.undo();
    same_objects(doc.scene(), &before);
}

#[test]
fn connecting_two_curves_ends_adds_a_straight_stretch() {
    let mut doc = Document::new();
    let a = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X * 100.0], false);
    let b = add_curve(
        &mut doc,
        &[Vec3::Y * 50.0, Vec3::new(100.0, 50.0, 0.0)],
        false,
    );
    doc.execute(Command::JoinCurveEnds {
        first: (a, id(1)),
        second: (b, id(1)),
        weld: false,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Connect curve ends"));
    let world = world_points(&doc, a);
    assert_eq!(world.len(), 4);
    assert!(world[2].distance(Vec3::new(100.0, 50.0, 0.0)) < 1e-3);
    assert!(world[3].distance(Vec3::new(0.0, 50.0, 0.0)) < 1e-3);
}

#[test]
fn joining_a_curves_own_ends_closes_it() {
    let mut doc = Document::new();
    let a = add_curve(
        &mut doc,
        &[Vec3::ZERO, Vec3::X * 100.0, Vec3::new(50.0, 80.0, 0.0)],
        false,
    );
    doc.execute(Command::JoinCurveEnds {
        first: (a, id(0)),
        second: (a, id(2)),
        weld: false,
    })
    .unwrap();
    assert!(curve(&doc, a).closed());
    assert_eq!(curve(&doc, a).points().len(), 3);
    doc.undo();
    // Welding three points' ends would leave two: refused with a reason.
    let err = doc
        .execute(Command::JoinCurveEnds {
            first: (a, id(0)),
            second: (a, id(2)),
            weld: true,
        })
        .unwrap_err();
    assert!(err.to_string().contains("three points"), "{err}");
    // A point in the middle isn't an end; nor is the same end twice.
    for (p, q) in [(id(0), id(1)), (id(0), id(0))] {
        assert!(
            doc.execute(Command::JoinCurveEnds {
                first: (a, p),
                second: (a, q),
                weld: false,
            })
            .is_err()
        );
    }
    assert_eq!(curve(&doc, a).points().len(), 3);
}

#[test]
fn join_objects_on_curves_makes_one_curve_end_to_end() {
    let mut doc = Document::new();
    let a = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X * 100.0], false);
    // Out of order and turned round: joined at the nearest ends.
    let c = add_curve(
        &mut doc,
        &[Vec3::new(300.0, 0.0, 0.0), Vec3::new(200.0, 0.0, 0.0)],
        false,
    );
    let b = add_curve(
        &mut doc,
        &[Vec3::X * 100.0, Vec3::new(150.0, 0.0, 0.0)],
        false,
    );
    let sweep = CurveSolid::Sweep(Sweep::default());
    doc.execute(Command::SetCurveSolid {
        ids: vec![a],
        solid: sweep,
    })
    .unwrap();
    let before = doc.scene().clone();
    doc.execute(Command::Join(vec![a, c, b])).unwrap();
    assert!(!doc.scene().contains(b) && !doc.scene().contains(c));
    let joined = doc.scene().object(a).unwrap().kind.curve().unwrap().clone();
    assert_eq!(joined.solid, sweep, "the first keeps its settings");
    let xs: Vec<f32> = world_points(&doc, a).iter().map(|p| p.x.round()).collect();
    // a's end met b's start on one spot (melted); b's end joined c's.
    assert_eq!(xs, vec![0.0, 100.0, 150.0, 200.0, 300.0]);
    ids_unique(&curve(&doc, a));
    doc.undo();
    same_objects(doc.scene(), &before);
    // A loop has no ends to join.
    let round = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X, Vec3::Y], true);
    let err = doc.execute(Command::Join(vec![a, round])).unwrap_err();
    assert!(err.to_string().contains("no ends"), "{err}");
}

#[test]
fn splitting_a_curve_makes_objects_with_the_same_settings() {
    let mut doc = Document::new();
    let a = add_curve(
        &mut doc,
        &[
            Vec3::ZERO,
            Vec3::X * 100.0,
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::Y * 100.0,
        ],
        false,
    );
    moved(&mut doc, a, Vec3::Z * 30.0);
    let lathe = CurveSolid::Lathe(LatheShape::default());
    doc.execute(Command::SetCurveSolid {
        ids: vec![a],
        solid: lathe,
    })
    .unwrap();
    let before = doc.scene().clone();
    doc.execute(Command::SplitCurve {
        id: a,
        points: vec![id(1), id(2)],
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Split curve"));
    let pieces = doc.selection();
    assert_eq!(pieces.len(), 3);
    assert_eq!(pieces[0], a);
    for piece in &pieces {
        let object = doc.scene().object(*piece).unwrap();
        assert_eq!(object.kind.curve().unwrap().solid, lathe);
        assert_eq!(object.transform.position, Vec3::Z * 30.0);
        assert_eq!(curve(&doc, *piece).points().len(), 2);
    }
    doc.undo();
    same_objects(doc.scene(), &before);
    // Cutting at an end splits nothing, and records nothing.
    let steps = doc.undo_steps();
    doc.execute(Command::SplitCurve {
        id: a,
        points: vec![id(0)],
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps);
}

#[test]
fn new_point_edits_undo_and_name_their_step() {
    let mut doc = Document::new();
    let a = add_curve(
        &mut doc,
        &[Vec3::ZERO, Vec3::X * 100.0, Vec3::new(100.0, 100.0, 0.0)],
        false,
    );
    for (edit, label) in [
        (
            CurveEdit::Insert {
                after: id(0),
                t: 0.5,
            },
            "Add curve point",
        ),
        (CurveEdit::Reverse, "Reverse curve"),
        (
            CurveEdit::RoundCorners {
                points: vec![id(1)],
                radius: 10.0,
            },
            "Round corners",
        ),
        (CurveEdit::Resample { count: 6 }, "Space 6 points evenly"),
        (
            CurveEdit::SetKind {
                points: vec![id(1)],
                kind: PointKind::Auto,
            },
            "Give points auto handles",
        ),
        (
            CurveEdit::SetKind {
                points: vec![id(1)],
                kind: PointKind::Broken,
            },
            "Break points' handles",
        ),
        (CurveEdit::CloseEnds { weld: false }, "Connect curve ends"),
    ] {
        let before = doc.scene().clone();
        doc.execute(Command::EditCurve { id: a, edit }).unwrap();
        assert_eq!(doc.undo_label(), Some(label));
        doc.undo();
        same_objects(doc.scene(), &before);
    }
}

#[test]
fn ids_are_never_reused_through_undo_and_redo() {
    let mut doc = Document::new();
    let a = add_curve(&mut doc, &[Vec3::ZERO, Vec3::X * 100.0], false);
    let insert = |doc: &mut Document| {
        doc.execute(Command::EditCurve {
            id: a,
            edit: CurveEdit::Insert {
                after: id(0),
                t: 0.5,
            },
        })
        .unwrap();
    };
    insert(&mut doc);
    assert_eq!(curve(&doc, a).points()[1].id, id(2));
    doc.execute(Command::EditCurve {
        id: a,
        edit: CurveEdit::Delete {
            parts: vec![CurvePart::point(id(2))],
        },
    })
    .unwrap();
    insert(&mut doc);
    assert_eq!(
        curve(&doc, a).points()[1].id,
        id(3),
        "2 is never given again"
    );
    doc.undo();
    doc.undo();
    doc.redo();
    doc.redo();
    assert_eq!(curve(&doc, a).points()[1].id, id(3));
    ids_unique(&curve(&doc, a));
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
fn random_spline_commands_undo_and_redo_exactly() {
    let mut dice = Dice(0xfeed_beef);
    for _ in 0..15 {
        let mut doc = Document::new();
        add_curve(
            &mut doc,
            &[Vec3::ZERO, Vec3::X * 50.0, Vec3::Y * 50.0, Vec3::ONE * 80.0],
            false,
        );
        add_curve(
            &mut doc,
            &[Vec3::Z * 10.0, Vec3::new(30.0, 30.0, 10.0)],
            false,
        );
        let start = doc.scene().clone();
        for _ in 0..25 {
            let live: Vec<ObjectId> = doc
                .scene()
                .walk()
                .into_iter()
                .map(|(id, _)| id)
                .filter(|id| {
                    doc.scene()
                        .object(*id)
                        .is_some_and(|o| o.kind.curve().is_some())
                })
                .collect();
            if live.is_empty() {
                break;
            }
            let pick_object = live[(dice.next() as usize) % live.len()];
            let other = live[(dice.next() as usize) % live.len()];
            let ids: Vec<CurvePointId> = curve(&doc, pick_object)
                .points()
                .iter()
                .map(|p| p.id)
                .collect();
            let other_ids: Vec<CurvePointId> =
                curve(&doc, other).points().iter().map(|p| p.id).collect();
            let pick = ids[(dice.next() as usize) % ids.len()];
            let pick_other = other_ids[(dice.next() as usize) % other_ids.len()];
            let edit = |e| Command::EditCurve {
                id: pick_object,
                edit: e,
            };
            let command = match dice.next() % 9 {
                0 => edit(CurveEdit::Insert {
                    after: pick,
                    t: (dice.next() % 100) as f32 / 100.0,
                }),
                1 => edit(CurveEdit::Reverse),
                2 => edit(CurveEdit::RoundCorners {
                    points: ids.clone(),
                    radius: (dice.next() % 40) as f32,
                }),
                3 => edit(CurveEdit::Resample {
                    count: (dice.next() % 9) as usize,
                }),
                4 => edit(CurveEdit::Transform {
                    parts: vec![CurvePart::point(pick)],
                    matrix: Mat4::from_translation(Vec3::splat((dice.next() % 30) as f32)),
                }),
                5 => Command::SplitCurve {
                    id: pick_object,
                    points: vec![pick],
                },
                6 => Command::JoinCurveEnds {
                    first: (pick_object, pick),
                    second: (other, pick_other),
                    weld: dice.next().is_multiple_of(2),
                },
                7 => Command::Join(live.clone()),
                _ => edit(CurveEdit::SetKind {
                    points: vec![pick],
                    kind: PointKind::ALL[(dice.next() % 4) as usize],
                }),
            };
            let _ = doc.execute(command);
            for id in doc.scene().walk().into_iter().map(|(id, _)| id) {
                if let Some(c) = doc.scene().object(id).and_then(|o| o.kind.curve()) {
                    ids_unique(&c.shape.curve());
                }
            }
        }
        let end = doc.scene().clone();
        while doc.undo().is_some() {}
        doc.redo();
        doc.redo();
        same_objects(doc.scene(), &start);
        while doc.redo().is_some() {}
        same_objects(doc.scene(), &end);
    }
}
