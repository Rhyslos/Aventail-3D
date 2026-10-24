//! Tests for levels of detail in the scene (decision 0155).

use crate::*;
use mesh::decimate::triangle_count;
use std::sync::Arc;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn on() -> Lods {
    Lods {
        on: true,
        ..Lods::default()
    }
}

fn set(doc: &mut Document, id: ObjectId, lods: Lods) {
    doc.execute(Command::SetLods {
        ids: vec![id],
        lods,
    })
    .unwrap();
}

/// The object's levels for its shape as the viewport shows it.
fn levels(doc: &Document, id: ObjectId) -> Option<Arc<LodShapes>> {
    let scene = doc.scene();
    scene.lod_shapes(id, Purpose::Viewport, scene.shown_shape(id)?)
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
fn objects_start_with_no_levels_and_the_standard_settings() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let lods = &doc.scene().object(sphere).unwrap().lods;
    assert!(!lods.on);
    assert_eq!(lods.levels, Lods::STANDARD);
    assert_eq!(lods.preview, 0);
    assert!(levels(&doc, sphere).is_none());
}

#[test]
fn a_sphere_gets_three_nested_levels_of_half_a_quarter_and_an_eighth() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    set(&mut doc, sphere, on());
    let found = levels(&doc, sphere).unwrap();
    assert!(found.problem.is_none());
    let full = triangle_count(&doc.scene().shown_shape(sphere).unwrap());
    assert_eq!(found.full, full);
    let counts = found.triangles();
    assert_eq!(counts.len(), 3);
    // The first level reaches its target; the UV seam and poles, kept as
    // lines, can stop the simplest ones a little short (decision 0145).
    assert!(counts[0] <= full / 2 + 2, "{full} → {counts:?}");
    assert!(counts[1] < counts[0] && counts[2] < counts[1], "{counts:?}");
    assert!(counts[2] <= full / 4, "{full} → {counts:?}");
    // Nested: every point of a level is one of the level before's.
    for pair in found.levels.windows(2) {
        let finer: Vec<_> = pair[0].vertices().map(|(_, p)| p).collect();
        for (_, p) in pair[1].vertices() {
            assert!(finer.iter().any(|q| q.distance(p) < 1e-5));
        }
    }
    // The object itself is unchanged.
    assert_eq!(
        triangle_count(&doc.scene().exported_shape(sphere).unwrap()),
        full
    );
}

#[test]
fn levels_follow_the_modifiers_and_are_kept_while_nothing_changes() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    set(&mut doc, sphere, on());
    let first = levels(&doc, sphere).unwrap();
    let again = levels(&doc, sphere).unwrap();
    assert!(Arc::ptr_eq(&first, &again), "kept");
    assert!(doc.scene().lods_if_ready(sphere).is_some());
    doc.execute(Command::AddModifier {
        ids: vec![sphere],
        kind: ModifierKind::Subdivide(SubdivideOptions::default()),
    })
    .unwrap();
    let smoothed = levels(&doc, sphere).unwrap();
    assert!(smoothed.full > first.full);
    // A distance alone doesn't make them again.
    let mut farther = doc.scene().object(sphere).unwrap().lods.clone();
    farther.levels[0].distance = 5_000.0;
    set(&mut doc, sphere, farther);
    assert!(Arc::ptr_eq(&levels(&doc, sphere).unwrap(), &smoothed));
    // A ratio does.
    let mut fewer = doc.scene().object(sphere).unwrap().lods.clone();
    fewer.levels[0].ratio = 0.3;
    set(&mut doc, sphere, fewer);
    assert!(doc.scene().lods_if_ready(sphere).is_none());
    assert!(!Arc::ptr_eq(&levels(&doc, sphere).unwrap(), &smoothed));
}

#[test]
fn work_done_on_another_thread_is_kept() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    set(&mut doc, sphere, on());
    let scene = doc.scene();
    let shape = scene.shown_shape(sphere).unwrap();
    let Some(LodState::Needed(job)) =
        scene.lod_state(sphere, Purpose::Viewport, Arc::clone(&shape))
    else {
        panic!("nothing made yet");
    };
    assert_eq!(job.work(), triangle_count(&shape));
    let done = std::thread::spawn(move || job.run()).join().unwrap();
    let made = done.shapes();
    scene.store_lods(done);
    let Some(LodState::Ready(kept)) = scene.lod_state(sphere, Purpose::Viewport, shape) else {
        panic!("kept");
    };
    assert!(Arc::ptr_eq(&kept, &made));
}

#[test]
fn ratios_that_grow_give_levels_no_finer_than_the_one_before() {
    let lods = Lods {
        on: true,
        levels: vec![
            LodLevel {
                ratio: 0.25,
                distance: 100.0,
            },
            LodLevel {
                ratio: 0.75,
                distance: 200.0,
            },
        ],
        preview: 0,
    };
    assert_eq!(lods.targets(1_000), [250, 250]);
    assert_eq!(lods.targets(0), [0, 0]);
    assert_eq!(lods.targets(1), [1, 1]);
}

#[test]
fn settings_are_clamped_never_a_panic() {
    let wild = Lods {
        on: true,
        levels: (0..20)
            .map(|n| LodLevel {
                ratio: [f32::NAN, -1.0, 5.0, f32::INFINITY][n % 4],
                distance: [f32::NAN, -10.0, 1e12, 300.0][n % 4],
            })
            .collect(),
        preview: 99,
    };
    let clamped = wild.clamped();
    assert_eq!(clamped.levels.len(), Lods::MAX_LEVELS);
    assert_eq!(clamped.preview, Lods::MAX_LEVELS);
    for level in &clamped.levels {
        assert!((mesh::decimate::DecimateOptions::MIN_RATIO..=1.0).contains(&level.ratio));
        assert!((0.0..=Lods::MAX_DISTANCE).contains(&level.distance));
    }
    let empty = Lods {
        levels: Vec::new(),
        preview: 4,
        ..on()
    }
    .clamped();
    assert_eq!(empty.levels, Lods::STANDARD);
    assert_eq!(empty.preview, 3);
    // The command clamps too.
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    set(&mut doc, cube, wild);
    assert_eq!(doc.scene().object(cube).unwrap().lods, clamped);
    assert!(levels(&doc, cube).is_some());
}

#[test]
fn more_and_fewer_levels_go_on_halving_and_doubling() {
    let five = on().with_count(5);
    assert_eq!(five.levels.len(), 5);
    assert_eq!(five.levels[3].ratio, 0.06);
    assert_eq!(five.levels[3].distance, 8_000.0);
    assert_eq!(five.levels[4].distance, 16_000.0);
    let previewing = Lods { preview: 5, ..five };
    let one = previewing.with_count(1);
    assert_eq!(one.levels, [Lods::STANDARD[0]]);
    assert_eq!(one.preview, 1);
    assert_eq!(one.with_count(0).levels.len(), 1);
    assert_eq!(one.with_count(100).levels.len(), Lods::MAX_LEVELS);
}

#[test]
fn changing_levels_is_one_undo_step_and_groups_are_left_alone() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::SetLods {
        ids: vec![cube, group],
        lods: on().with_count(2),
    })
    .unwrap();
    assert!(doc.scene().object(cube).unwrap().lods.on);
    assert!(!doc.scene().object(group).unwrap().lods.on);
    assert_eq!(
        doc.undo().as_deref(),
        Some(words::tr("Change the levels of detail"))
    );
    assert!(!doc.scene().object(cube).unwrap().lods.on);
    doc.redo().unwrap();
    // The same settings again change nothing.
    let before = doc.scene().clone();
    doc.execute(Command::SetLods {
        ids: vec![cube],
        lods: on().with_count(2),
    })
    .unwrap();
    assert_eq!(*doc.scene(), before);
    check_round_trip(&mut doc);
}

#[test]
fn an_empty_or_open_shape_gets_levels_and_deleting_forgets_them() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    set(&mut doc, plane, on());
    let found = levels(&doc, plane).unwrap();
    assert!(found.problem.is_none());
    assert_eq!(found.levels.len(), 3);
    let scene = doc.scene();
    let empty = scene
        .lod_shapes(plane, Purpose::Viewport, Arc::new(Mesh::default()))
        .unwrap();
    assert_eq!(empty.full, 0);
    assert!(empty.triangles().iter().all(|t| *t == 0));
    doc.execute(Command::Delete(vec![plane])).unwrap();
    assert!(doc.scene().lods_if_ready(plane).is_none());
}

#[test]
fn a_shape_at_no_real_place_says_why() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    set(&mut doc, cube, on());
    let mut broken = (*doc.scene().shown_shape(cube).unwrap()).clone();
    let first = broken.vertices().next().unwrap().0;
    broken.set_position(first, glam::Vec3::NAN);
    let found = doc
        .scene()
        .lod_shapes(cube, Purpose::Viewport, Arc::new(broken))
        .unwrap();
    assert!(found.levels.is_empty());
    assert!(
        found
            .problem
            .as_deref()
            .unwrap()
            .starts_with("No levels of detail")
    );
}

#[test]
fn the_counts_shown_are_the_viewports_for_the_shape_as_it_is_now() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    set(&mut doc, sphere, on());
    let shown = levels(&doc, sphere).unwrap();
    // An export makes its own (here from a cube's shape): the viewport's
    // counts stay.
    let cube = Arc::new(PrimitiveKind::Cube.standard().mesh());
    let exported = doc
        .scene()
        .lod_shapes(sphere, Purpose::Export, cube)
        .unwrap();
    assert_ne!(exported.full, shown.full);
    assert_eq!(doc.scene().lods_if_ready(sphere).unwrap().full, shown.full);
    // The sphere gets a Subdivide: until its levels are made again, the
    // old counts aren't shown.
    doc.execute(Command::AddModifier {
        ids: vec![sphere],
        kind: ModifierKind::Subdivide(SubdivideOptions::default()),
    })
    .unwrap();
    let scene = doc.scene();
    let now = scene.shown_shape(sphere).unwrap();
    assert!(matches!(
        scene.lod_state(sphere, Purpose::Viewport, now),
        Some(LodState::Needed(_))
    ));
    assert!(scene.lods_if_ready(sphere).is_none(), "stale");
}
