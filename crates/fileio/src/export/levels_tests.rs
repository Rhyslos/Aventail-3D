//! Tests for levels of detail in exports with the Levels panel's ways of
//! making them (decision 0165): each level's real shape goes out, and a
//! level made by hand never goes out as an object of its own.

use super::*;
use scene::{Command, Lods, ModifierKind, ObjectKind, PrimitiveKind, SubdivideOptions};

fn with_levels() -> ExportOptions {
    ExportFormat::Glb.options(&ExportOptions {
        lod_levels: true,
        ..ExportOptions::default()
    })
}

/// A sphere with Subdivide and two unreduced levels: the first without
/// Subdivide, the second made by hand (and moved up 50 cm).
fn scene() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let sphere = doc.selection()[0];
    doc.execute(Command::AddModifier {
        ids: vec![sphere],
        kind: ModifierKind::Subdivide(SubdivideOptions::default()),
    })
    .unwrap();
    let modifiers = doc.scene().modifiers(sphere).to_vec();
    let mut lods = Lods {
        on: true,
        ..Lods::default()
    }
    .with_count(2);
    for level in &mut lods.levels {
        level.reduce = false;
    }
    lods.levels[0] = lods.levels[0].with_override(&modifiers, modifiers[0].id, false, None);
    doc.execute(Command::SetLods {
        ids: vec![sphere],
        lods,
    })
    .unwrap();
    doc.execute(Command::MakeLevelByHand {
        id: sphere,
        level: 1,
    })
    .unwrap();
    let shape = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        shape,
        scene::Transform {
            position: glam::Vec3::new(0.0, 0.0, 50.0),
            ..scene::Transform::default()
        },
    )]))
    .unwrap();
    (doc, sphere, shape)
}

#[test]
fn each_level_goes_out_as_its_real_shape() {
    let (doc, sphere, _) = scene();
    let objects = prepare(&doc, &with_levels());
    // The hand-made level's shape isn't an object of its own.
    let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["Sphere"]);
    let out = &objects[0];
    let scene = doc.scene();
    let plain = mesh::decimate::triangle_count(&scene.unmodified_shape(sphere).unwrap());
    let smoothed = mesh::decimate::triangle_count(&scene.exported_shape(sphere).unwrap());
    assert_eq!(out.local.triangle_count(), smoothed);
    assert_eq!(out.levels[0].local.triangle_count(), plain);
    assert_eq!(out.levels[1].local.triangle_count(), smoothed);
    // Placed where it was moved: 50 cm up, half a metre in the file.
    let top = |m: &ExportMesh| m.points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    assert!(
        (top(&out.levels[1].local) - top(&out.local) - 0.5).abs() < 1e-3,
        "{} {}",
        top(&out.levels[1].local),
        top(&out.local)
    );
    // Only Export → Selected of the level's shape gives nothing.
    let mut picked = doc.clone();
    picked.set_selection([objects[0].id]);
    assert_eq!(
        prepare(
            &picked,
            &ExportOptions {
                what: ExportWhat::Selected,
                ..with_levels()
            }
        )
        .len(),
        1
    );
    let summary = summary(&doc, &with_levels());
    assert_eq!(summary.objects, 1);
}

#[test]
fn a_hand_made_level_moved_out_of_its_object_goes_out_as_an_ordinary_shape() {
    let (mut doc, _, shape) = scene();
    doc.execute(Command::Move {
        ids: vec![shape],
        parent: None,
        index: 0,
    })
    .unwrap();
    let objects = prepare(&doc, &with_levels());
    let mut names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["Sphere", "Sphere LOD2"]);
}

#[test]
fn a_kit_piece_takes_its_levels_and_not_their_shapes() {
    let (doc, sphere, _) = scene();
    let objects = prepare_piece(&doc, sphere, &with_levels(), Frames::Both);
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].levels.len(), 2);
}
