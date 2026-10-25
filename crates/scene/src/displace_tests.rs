//! Tests for the Displace modifier in the scene (decision 0157): adding
//! it, its place on the palette and in a list, following the shape, the
//! chosen points, baking, undo and heavy work.

use crate::*;
use glam::Vec3;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn displace() -> ModifierKind {
    ModifierKind::Displace(DisplaceSettings::default())
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

fn change(doc: &mut Document, id: ObjectId, settings: DisplaceSettings) {
    let mut changed = doc.scene().modifiers(id)[0].clone();
    changed.kind = ModifierKind::Displace(settings);
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: changed,
    })
    .unwrap();
}

fn names(doc: &Document, id: ObjectId) -> Vec<&'static str> {
    doc.scene()
        .modifiers(id)
        .iter()
        .map(|m| m.kind.name())
        .collect()
}

/// A plane of 20 × 20 squares, 200 cm wide.
fn ground(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Plane(
        mesh::primitives::PlaneParams {
            width: 200.0,
            depth: 200.0,
            segments: [20, 20],
        },
    ))))
    .unwrap();
    doc.selection()[0]
}

#[test]
fn a_sphere_is_roughened_and_exports_as_shown() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, sphere, displace());
    assert_eq!(doc.undo_label(), Some("Add Displace"));
    let scene = doc.scene();
    let input = scene.unmodified_shape(sphere).unwrap();
    let shown = scene.shown_shape(sphere).unwrap();
    assert!(scene.modifier_problems(sphere).is_empty());
    assert_eq!(scene.exported_shape(sphere), Some(shown.clone()));
    assert!(shown.same_structure(&input), "only the points move");
    assert_ne!(*shown, *input);
    assert_eq!(displace().family(), ModifierFamily::Deform);
    assert!(!displace().differs_by_purpose());
    assert!(!displace().carries_custom_normals());
    assert!(displace().sources().is_empty() && !displace().needs_sources());
    doc.undo();
    assert!(doc.scene().modifiers(sphere).is_empty());
}

#[test]
fn it_follows_shrinkwrap_on_the_palette() {
    let kinds = ModifierKind::standard_kinds();
    let at = |name: &str| kinds.iter().position(|k| k.name() == name).unwrap();
    assert_eq!(at("Displace"), at("Shrinkwrap") + 1);
}

#[test]
fn settings_are_clamped_when_added() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let v = mesh::VertexId::from_raw;
    add(
        &mut doc,
        cube,
        ModifierKind::Displace(DisplaceSettings {
            options: DisplaceOptions {
                strength: f32::NAN,
                detail: 99,
                ..DisplaceOptions::default()
            },
            points: vec![v(3), v(1), v(3)],
        }),
    );
    let ModifierKind::Displace(kept) = &doc.scene().modifiers(cube)[0].kind else {
        panic!("a displace")
    };
    assert_eq!(kept.options.strength, DisplaceOptions::STANDARD_STRENGTH);
    assert_eq!(kept.options.detail, DisplaceOptions::MAX_DETAIL);
    assert_eq!(kept.points, vec![v(1), v(3)], "listed once, in order");
}

#[test]
fn a_new_displace_goes_before_weighted_normals_only() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, id, displace());
    assert_eq!(
        names(&doc, id),
        ["Subdivide", "Displace", "Weighted normals"]
    );
}

#[test]
fn it_follows_the_shape_as_it_is_modelled_and_its_settings() {
    let mut doc = Document::new();
    let plane = ground(&mut doc);
    add(&mut doc, plane, displace());
    let mut settings = DisplaceSettings::default();
    settings.options.direction = DisplaceDirection::AxisZ;
    change(&mut doc, plane, settings.clone());
    assert_eq!(doc.undo_label(), Some("Change Displace"));
    let before = doc.scene().shown_shape(plane).unwrap();
    assert!(before.vertices().all(|(_, p)| p.z.abs() <= 5.0 + 1e-3));
    assert!(before.vertices().any(|(_, p)| p.z.abs() > 0.5));
    // Stronger: twice the height everywhere.
    settings.options.strength = 10.0;
    change(&mut doc, plane, settings.clone());
    let after = doc.scene().shown_shape(plane).unwrap();
    for (id, p) in before.vertices() {
        let q = after.position(id).unwrap();
        assert!((q.z - 2.0 * p.z).abs() < 1e-3);
    }
    // Lifting the plane as modelled lifts the bumps with it, but the noise
    // is read where each point now is, so the bumps change.
    let shape = doc.scene().evaluated(plane).unwrap().mesh;
    let points: Vec<VertexId> = shape.vertices().map(|(v, _)| v).collect();
    doc.execute(Command::Record {
        id: plane,
        operation: Operation::MovePoints {
            points,
            offset: Vec3::new(0.0, 0.0, 30.0),
            anchors: None,
        },
    })
    .unwrap();
    let lifted = doc.scene().shown_shape(plane).unwrap();
    assert!(
        lifted
            .vertices()
            .all(|(_, p)| (p.z - 30.0).abs() <= 10.0 + 1e-3)
    );
}

#[test]
fn only_the_chosen_points_and_their_surroundings_move_and_lost_ones_say_why() {
    let mut doc = Document::new();
    let plane = ground(&mut doc);
    add(&mut doc, plane, displace());
    let shape = doc.scene().evaluated(plane).unwrap().mesh;
    let corner: Vec<VertexId> = shape
        .vertices()
        .filter(|(_, p)| p.x > 79.0 && p.y > 79.0)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(corner.len(), 9);
    let mut settings = DisplaceSettings::default();
    settings.options.direction = DisplaceDirection::AxisZ;
    settings.options.strength = 20.0;
    settings.points = corner;
    change(&mut doc, plane, settings.clone());
    let shown = doc.scene().shown_shape(plane).unwrap();
    // Beyond the 20 cm falloff from the corner, nothing moves.
    for (_, p) in shown.vertices() {
        if p.x < 55.0 || p.y < 55.0 {
            assert_eq!(p.z, 0.0, "{p}");
        }
    }
    assert!(shown.vertices().any(|(_, p)| p.z.abs() > 0.5));
    // Points that aren't there any more.
    settings.points = vec![VertexId::from_raw(40_000)];
    change(&mut doc, plane, settings);
    let problems = doc.scene().modifier_problems(plane);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].1.contains("points"), "{problems:?}");
    assert_eq!(
        doc.scene().shown_shape(plane),
        doc.scene().unmodified_shape(plane),
        "passed on unchanged"
    );
}

#[test]
fn bake_into_shape_keeps_the_bumps() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, sphere, displace());
    let exported = doc.scene().exported_shape(sphere).unwrap();
    doc.execute(Command::MakeEditable(vec![sphere])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(sphere).is_empty());
    assert_eq!(
        scene.object(sphere).unwrap().kind,
        ObjectKind::Mesh(exported)
    );
}

#[test]
fn the_same_document_gives_the_same_bumps() {
    let shape = || {
        let mut doc = Document::new();
        let sphere = create(&mut doc, PrimitiveKind::Sphere);
        let mut settings = DisplaceSettings::default();
        settings.options.noise = NoiseKind::Cells;
        add(&mut doc, sphere, ModifierKind::Displace(settings));
        doc.scene().exported_shape(sphere).unwrap()
    };
    assert_eq!(shape(), shape());
}

#[test]
fn heavy_displacing_is_done_on_another_thread() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let input = doc.scene().unmodified_shape(sphere).unwrap();
    assert!(displace().work(&input, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = Primitive::Sphere(mesh::primitives::SphereParams {
        radius: 50.0,
        sides: 512,
        rings: 256,
    })
    .mesh();
    assert!(displace().work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}

#[test]
fn chosen_points_still_count_after_a_subdivide_before_it() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, ModifierKind::Subdivide(Default::default()));
    add(&mut doc, cube, displace());
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let top: Vec<VertexId> = shape
        .vertices()
        .filter(|(_, p)| p.z > 50.0)
        .map(|(id, _)| id)
        .collect();
    let settings = DisplaceSettings {
        points: top,
        ..DisplaceSettings::default()
    };
    let mut changed = doc.scene().modifiers(cube)[1].clone();
    changed.kind = ModifierKind::Displace(settings);
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 1,
        modifier: changed,
    })
    .unwrap();
    assert!(doc.scene().modifier_problems(cube).is_empty());
}
