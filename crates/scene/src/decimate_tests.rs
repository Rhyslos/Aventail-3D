//! Tests for the Decimate modifier (decision 0145).

use crate::*;
use mesh::decimate::triangle_count;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn decimate(ratio: f32) -> ModifierKind {
    ModifierKind::Decimate(DecimateOptions {
        ratio,
        ..Default::default()
    })
}

/// A standard sphere with a standard Decimate modifier.
fn decimated_sphere() -> (Document, ObjectId) {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Decimate(DecimateOptions::default()),
    })
    .unwrap();
    (doc, id)
}

#[test]
fn a_sphere_shows_and_exports_half_its_triangles() {
    let (doc, id) = decimated_sphere();
    let scene = doc.scene();
    let before = triangle_count(&scene.evaluated(id).unwrap().mesh);
    let shown = triangle_count(&scene.shown_shape(id).unwrap());
    assert!(
        shown <= before / 2 && shown + 2 >= before / 2,
        "{before} → {shown}"
    );
    assert_eq!(scene.shown_shape(id), scene.exported_shape(id));
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(doc.undo_label(), Some("Add Decimate"));
}

#[test]
fn changing_the_ratio_works_it_out_again_and_undo_brings_it_back() {
    let (mut doc, id) = decimated_sphere();
    let half = doc.scene().shown_shape(id).unwrap();
    let mut changed = doc.scene().modifiers(id)[0].clone();
    changed.kind = decimate(0.2);
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: changed,
    })
    .unwrap();
    let fifth = doc.scene().shown_shape(id).unwrap();
    assert!(triangle_count(&fifth) < triangle_count(&half));
    doc.undo();
    assert_eq!(doc.scene().shown_shape(id).unwrap(), half);
}

#[test]
fn the_kept_hard_edges_are_those_the_shading_shows() {
    // A cylinder smoothed by angle (as new shapes are): its rims are
    // sharper than 30°, so they stay; its side's rings go.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cylinder);
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: decimate(0.05),
    })
    .unwrap();
    let input = doc.scene().evaluated(id).unwrap().mesh;
    let b = input.bounds().unwrap();
    let on_rims = |m: &Mesh| {
        m.vertices()
            .filter(|(_, p)| p.z == b.min.z || p.z == b.max.z)
            .count()
    };
    let by_angle = doc.scene().shown_shape(id).unwrap();
    let kept = by_angle.bounds().unwrap();
    assert_eq!((kept.min.z, kept.max.z), (b.min.z, b.max.z));
    // Shaded by its marks instead (the primitive marks only its rims):
    // still kept, and the modifier is worked out again.
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    let marked = doc.scene().shown_shape(id).unwrap();
    assert!(on_rims(&marked) >= 12);
    assert!(on_rims(&by_angle) >= 12);
    // With hard edges not kept, the rims may lose more points.
    let mut off = doc.scene().modifiers(id)[0].clone();
    off.kind = ModifierKind::Decimate(DecimateOptions {
        ratio: 0.05,
        keep: DecimateKeep {
            hard_edges: false,
            ..DecimateKeep::default()
        },
        ..Default::default()
    });
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: off,
    })
    .unwrap();
    let loose = doc.scene().shown_shape(id).unwrap();
    assert!(triangle_count(&loose) <= triangle_count(&marked));
}

#[test]
fn it_works_on_what_the_modifiers_before_it_give() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    for kind in [
        ModifierKind::Subdivide(SubdivideOptions {
            levels: 3,
            export_levels: 3,
            hard_edges_sharp: false,
        }),
        decimate(0.25),
    ] {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind,
        })
        .unwrap();
    }
    // 6 quads, three levels: 384 quads, 768 triangles; a quarter is 192.
    let shown = doc.scene().shown_shape(id).unwrap();
    assert_eq!(triangle_count(&shown), 192);
    assert!(shown.faces().all(|(_, c)| c.len() == 3));
}

#[test]
fn bake_into_shape_makes_it_permanent() {
    let (mut doc, id) = decimated_sphere();
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    assert_eq!(**baked, *expected);
    doc.undo();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
}

#[test]
fn settings_are_brought_within_limits() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Decimate(DecimateOptions {
            ratio: f32::NAN,
            triangles: 0,
            ..Default::default()
        }),
    })
    .unwrap();
    let ModifierKind::Decimate(o) = doc.scene().modifiers(id)[0].kind else {
        panic!("decimate")
    };
    assert_eq!(o.ratio, DecimateOptions::STANDARD_RATIO);
    assert_eq!(o.triangles, DecimateOptions::MIN_TRIANGLES);
}

#[test]
fn heavy_decimation_is_worked_out_on_another_thread() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    let sphere = doc.scene().unmodified_shape(id).unwrap();
    // The standard sphere is light; nothing to do is no work.
    assert!(decimate(0.5).work(&sphere, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    assert_eq!(decimate(1.0).work(&sphere, Purpose::Viewport), 0);
    let big = mesh::Primitive::Sphere(mesh::primitives::SphereParams {
        radius: 50.0,
        sides: 128,
        rings: 64,
    })
    .mesh();
    assert!(decimate(0.5).work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
