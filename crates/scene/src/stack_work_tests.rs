//! Tests for how much asking about an object's modifiers costs on the
//! interface thread: a source's modifiers are worked out by the job that
//! reads it, never while gathering it; a heavy job's work estimate is kept
//! while nothing changes; and an export's problems are only reported once
//! worked out.

use crate::*;
use glam::Vec3;
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

fn subdivide(levels: u32) -> ModifierKind {
    ModifierKind::Subdivide(SubdivideOptions {
        levels,
        export_levels: levels,
        hard_edges_sharp: false,
    })
}

/// A cube shrink-wrapped onto a sphere smoothed by Subdivide: the cube and
/// the sphere.
fn cube_wrapped_onto_a_smoothed_sphere() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, sphere, subdivide(1));
    add(
        &mut doc,
        cube,
        ModifierKind::Shrinkwrap(ShrinkwrapSettings::of(sphere)),
    );
    (doc, cube, sphere)
}

fn state(scene: &Scene, id: ObjectId, purpose: Purpose) -> Option<StackState> {
    let input = scene.unmodified_shape(id).unwrap();
    scene.stack_state(id, purpose, input)
}

fn job(scene: &Scene, id: ObjectId) -> StackJob {
    match state(scene, id, Purpose::Viewport) {
        Some(StackState::Needed(job)) => job,
        other => panic!("{other:?}"),
    }
}

#[test]
fn gathering_a_source_never_works_out_its_modifiers() {
    let (doc, cube, sphere) = cube_wrapped_onto_a_smoothed_sphere();
    let scene = doc.scene();
    let sphere_work = job(scene, sphere).work();
    let cube_job = job(scene, cube);
    // The sphere's Subdivide is still to do: asking about the cube didn't
    // do it, and the cube's job counts it.
    assert!(matches!(
        state(scene, sphere, Purpose::Viewport),
        Some(StackState::Needed(_))
    ));
    assert!(cube_job.work() >= sphere_work);
    // Running the cube's job works the sphere out on the way, and both are
    // kept.
    let done = cube_job.run();
    let wrapped = done.result().mesh;
    scene.store_stack(done);
    assert!(matches!(
        state(scene, sphere, Purpose::Viewport),
        Some(StackState::Ready(_))
    ));
    match state(scene, cube, Purpose::Viewport) {
        Some(StackState::Ready(result)) => assert!(Arc::ptr_eq(&result.mesh, &wrapped)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_source_worked_out_by_the_reader_gives_the_same_shape() {
    // Worked out the old way, the sphere first.
    let (doc, cube, sphere) = cube_wrapped_onto_a_smoothed_sphere();
    let scene = doc.scene();
    scene.shown_shape(sphere).unwrap();
    let expected = scene.shown_shape(cube).unwrap();
    // And by the cube's job, the sphere inside it.
    let (doc, cube, _) = cube_wrapped_onto_a_smoothed_sphere();
    let scene = doc.scene();
    let wrapped = scene.shown_shape(cube).unwrap();
    assert_eq!(*wrapped, *expected);
    // And for an export.
    assert_eq!(
        *scene.exported_shape(cube).unwrap(),
        *doc.scene().exported_shape(cube).unwrap()
    );
}

#[test]
fn an_instance_of_a_reader_works_its_source_out_in_the_originals_space() {
    let (mut doc, cube, _) = cube_wrapped_onto_a_smoothed_sphere();
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        instance,
        Transform {
            position: Vec3::new(500.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let scene = doc.scene();
    // The instance first, so its job works the sphere out.
    let done = job(scene, instance).run();
    let shown = done.result().mesh;
    scene.store_stack(done);
    assert_eq!(*shown, *scene.shown_shape(cube).unwrap());
}

#[test]
fn a_heavy_jobs_estimate_is_kept_while_nothing_changes() {
    let mut doc = Document::new();
    let ball = Arc::new(mesh::samples::heavy_ball(40_000, 100.0));
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::clone(&ball))))
        .unwrap();
    let id = doc.selection()[0];
    add(&mut doc, id, subdivide(1));
    let scene = doc.scene();
    let first = job(scene, id).work();
    assert!(first >= MODIFIERS_IN_BACKGROUND_FROM);
    assert_eq!(scene.stacks.estimates_kept(), 1);
    // Asked again (as every frame does while it is worked out elsewhere):
    // the same, from what was kept.
    assert_eq!(job(scene, id).work(), first);
    assert_eq!(scene.stacks.estimates_kept(), 1);
    // Worked out and kept: the estimate goes.
    scene.store_stack(job(scene, id).run());
    assert_eq!(scene.stacks.estimates_kept(), 0);
    // A changed setting is a new estimate.
    let mut changed = scene.modifiers(id)[0].clone();
    changed.kind = subdivide(2);
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: changed,
    })
    .unwrap();
    let scene = doc.scene();
    assert!(job(scene, id).work() > first);
}

#[test]
fn an_exports_problems_are_only_those_worked_out() {
    // Remesh, not shown in the viewport, can't remesh an open plane.
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(
        &mut doc,
        plane,
        ModifierKind::Remesh(RemeshOptions::default()),
    );
    let mut hidden = doc.scene().modifiers(plane)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: plane,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    let scene = doc.scene();
    // Asking doesn't work the export out.
    assert!(scene.modifier_problems(plane).is_empty());
    assert!(matches!(
        state(scene, plane, Purpose::Export),
        Some(StackState::Needed(_))
    ));
    // The light job to do elsewhere; once it's in, the problem shows.
    let job = scene.export_problems_job(plane).expect("light work to do");
    assert_eq!(job.purpose(), Purpose::Export);
    scene.store_stack(job.run());
    let problems = scene.modifier_problems(plane);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, 0);
    // Nothing more to do.
    assert!(scene.export_problems_job(plane).is_none());
}

#[test]
fn an_object_shown_as_exported_needs_no_export_job() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        cube,
        ModifierKind::Triangulate(Default::default()),
    );
    assert!(doc.scene().export_problems_job(cube).is_none());
}

#[test]
fn estimates_count_what_remesh_and_quadify_really_do() {
    // A million-polygon shape remeshed coarsely still measures every
    // triangle: about 2 s, not the few grid points' worth.
    let ball = mesh::samples::heavy_ball(100_000, 100.0);
    let coarse = ModifierKind::Remesh(RemeshOptions {
        voxel_size: 50.0,
        smooth: 0,
    });
    assert!(coarse.work(&ball, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
    // Quadify looks at every polygon, not only the few triangles to join
    // (the ball is quads, but for its poles).
    let quadify = ModifierKind::Quadify(Default::default());
    assert!(mesh::quadify::triangle_count(&ball) / 2 < MODIFIERS_IN_BACKGROUND_FROM);
    assert!(quadify.work(&ball, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
    // A Live bevel heavy for its size alone doesn't look for its edges.
    let bevel = ModifierKind::LiveBevel(Default::default());
    assert_eq!(bevel.work(&ball, Purpose::Viewport), ball.face_count() / 5);
    // Scatter reads its whole surface to find where copies go, however
    // few they are.
    let scatter = ModifierKind::Scatter(ScatterSettings::of(Vec::new()));
    assert!(scatter.work(&ball, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
    // A small one does, and stays light.
    let cube = PrimitiveKind::Cube.standard().mesh();
    assert!(bevel.work(&cube, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    assert!(coarse.work(&cube, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
}
