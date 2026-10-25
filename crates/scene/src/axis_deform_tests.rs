//! Tests for the *Bend, twist or taper* modifier in the scene (decision
//! 0158): adding it, its name by kind, its place on the palette and in a
//! list, the pivot it works round, the helper, baking, undo and heavy
//! work.

use crate::*;
use glam::Vec3;

/// A pole 20 cm across and 200 cm tall, standing on its pivot, cut into
/// 20 slices: something with points along its length to bend.
fn pole(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cylinder(
        mesh::primitives::CylinderParams {
            radius: 10.0,
            height: 200.0,
            sides: 12,
            height_segments: 20,
            ..Default::default()
        },
    ))))
    .unwrap();
    doc.selection()[0]
}

fn deformer(kind: AxisDeformKind) -> ModifierKind {
    ModifierKind::BendTwistTaper(AxisDeformOptions {
        kind,
        ..AxisDeformOptions::default()
    })
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

fn change(doc: &mut Document, id: ObjectId, index: usize, options: AxisDeformOptions) {
    let mut changed = doc.scene().modifiers(id)[index].clone();
    changed.kind = ModifierKind::BendTwistTaper(options);
    doc.execute(Command::ChangeModifier {
        id,
        index,
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

#[test]
fn a_pole_is_bent_and_exports_as_shown() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    add(&mut doc, id, deformer(AxisDeformKind::Bend));
    assert_eq!(doc.undo_label(), Some("Add Bend"));
    let scene = doc.scene();
    let input = scene.unmodified_shape(id).unwrap();
    let shown = scene.shown_shape(id).unwrap();
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(scene.exported_shape(id), Some(shown.clone()));
    assert!(shown.same_structure(&input), "only the points move");
    // Bent 45° towards X: its top leans over.
    let top = shown.bounds().unwrap();
    assert!(top.max.x > 50.0 && top.max.z < 200.0, "{top:?}");
    let kind = deformer(AxisDeformKind::Bend);
    assert_eq!(kind.family(), ModifierFamily::Deform);
    assert!(!kind.differs_by_purpose() && !kind.carries_custom_normals());
    assert!(kind.sources().is_empty() && !kind.needs_sources());
    assert!(kind.reads_pivot());
    doc.undo();
    assert!(doc.scene().modifiers(id).is_empty());
}

#[test]
fn it_is_named_by_its_kind() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    add(&mut doc, id, deformer(AxisDeformKind::Twist));
    add(&mut doc, id, deformer(AxisDeformKind::Taper));
    assert_eq!(names(&doc, id), ["Twist", "Taper"]);
    assert_eq!(doc.undo_label(), Some("Add Taper"));
    // Switching the kind keeps the other settings and the place.
    let options = AxisDeformOptions {
        kind: AxisDeformKind::Bend,
        angle: 30.0,
        ..AxisDeformOptions::default()
    };
    change(&mut doc, id, 0, options);
    assert_eq!(names(&doc, id), ["Bend", "Taper"]);
    assert_eq!(doc.undo_label(), Some("Change Bend"));
    assert!(
        doc.scene().modifiers(id)[0]
            .kind
            .same_kind(&deformer(AxisDeformKind::Taper))
    );
}

#[test]
fn it_follows_displace_on_the_palette() {
    let kinds = ModifierKind::standard_kinds();
    let at = |name: &str| kinds.iter().position(|k| k.name() == name).unwrap();
    assert_eq!(at("Bend"), at("Displace") + 1);
}

#[test]
fn settings_are_clamped_when_added() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    add(
        &mut doc,
        id,
        ModifierKind::BendTwistTaper(AxisDeformOptions {
            angle: f32::NAN,
            limits: [2.0, -1.0],
            direction: 400.0,
            ..AxisDeformOptions::default()
        }),
    );
    let ModifierKind::BendTwistTaper(kept) = &doc.scene().modifiers(id)[0].kind else {
        panic!("a bend")
    };
    assert_eq!(kept.angle, AxisDeformOptions::STANDARD_ANGLE);
    assert_eq!(kept.limits, [1.0, 0.0]);
    assert_eq!(kept.direction, 40.0);
    assert!(
        doc.scene()
            .shown_shape(id)
            .unwrap()
            .vertices()
            .all(|(_, p)| p.is_finite())
    );
}

#[test]
fn a_new_one_goes_after_subdivide_and_before_weighted_normals() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, id, deformer(AxisDeformKind::Twist));
    assert_eq!(names(&doc, id), ["Subdivide", "Twist", "Weighted normals"]);
}

#[test]
fn it_works_round_the_pivot_and_follows_it() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    add(&mut doc, id, deformer(AxisDeformKind::Bend));
    let foot = |doc: &Document| {
        let shown = doc.scene().shown_shape(id).unwrap();
        let input = doc.scene().unmodified_shape(id).unwrap();
        input
            .vertices()
            .filter(|(_, p)| p.z.abs() < 1e-3)
            .all(|(v, p)| shown.position(v).unwrap().distance(p) < 1e-3)
    };
    assert!(foot(&doc), "bent from its pivot, at its foot");
    // The pivot moved to the top: the top stays, the foot swings.
    let transform = Transform {
        position: Vec3::new(0.0, 0.0, 200.0),
        ..Transform::default()
    };
    doc.execute(Command::MovePivots(vec![(id, transform)]))
        .unwrap();
    assert!(!foot(&doc));
    let shown = doc.scene().shown_shape(id).unwrap();
    let input = doc.scene().unmodified_shape(id).unwrap();
    for (v, p) in input.vertices().filter(|(_, p)| p.z > 199.9) {
        assert!(shown.position(v).unwrap().distance(p) < 1e-2);
    }
}

#[test]
fn the_helper_shows_each_bend_that_is_on() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    assert!(doc.scene().deform_helpers(id).is_empty());
    add(&mut doc, id, deformer(AxisDeformKind::Bend));
    add(&mut doc, id, deformer(AxisDeformKind::Twist));
    let _ = doc.scene().shown_shape(id);
    let helpers = doc.scene().deform_helpers(id);
    assert_eq!(helpers.len(), 2);
    // The bend's axis starts at the foot and leans over towards X.
    let bend = &helpers[0];
    assert!(bend.axis[0].distance(Vec3::ZERO) < 1e-3);
    assert!(bend.axis.last().unwrap().x > 50.0);
    // The twist's is measured on the bent shape: it lies along the bent
    // pole's Z, shorter than the pole.
    let twist = &helpers[1];
    assert!(twist.axis.last().unwrap().z < 200.0);
    // Switched off, it has none.
    let mut off = doc.scene().modifiers(id)[0].clone();
    off.on = false;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: off,
    })
    .unwrap();
    assert_eq!(doc.scene().deform_helpers(id).len(), 1);
}

#[test]
fn bake_into_shape_keeps_the_bend() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    add(&mut doc, id, deformer(AxisDeformKind::Taper));
    let exported = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(id).is_empty());
    assert_eq!(scene.object(id).unwrap().kind, ObjectKind::Mesh(exported));
}

#[test]
fn heavy_bending_is_done_on_another_thread() {
    let mut doc = Document::new();
    let id = pole(&mut doc);
    let input = doc.scene().unmodified_shape(id).unwrap();
    let kind = deformer(AxisDeformKind::Bend);
    assert!(kind.work(&input, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = Primitive::Sphere(mesh::primitives::SphereParams {
        radius: 50.0,
        sides: 512,
        rings: 256,
    })
    .mesh();
    assert!(kind.work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
