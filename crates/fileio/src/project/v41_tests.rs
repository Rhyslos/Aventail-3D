//! Tests for version 41: the development format with curve objects
//! (decision 0138).

use super::tests::{assert_same, header, refused};
use super::v40_tests::rich_document_v40;
use super::*;
use glam::{Mat4, Vec3};
use scene::{
    Command, Curve, CurveEdit, CurveObject, CurvePart, CurvePointId, CurveShape, CurveSolid,
    LatheAxis, LatheShape, ObjectId, ObjectKind, PointKind, ReadyCurve, ReadyCurveKind, Sweep,
    SweepProfile,
};

/// The id of the object called `name`.
fn named(doc: &Document, name: &str) -> ObjectId {
    doc.scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == name)
        .unwrap()
}

fn add(doc: &mut Document, name: &str, curve: CurveObject) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Curve(curve)))
        .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::Rename {
        id,
        name: name.into(),
    })
    .unwrap();
    id
}

/// Everything version 40 stores, plus curves, with each kind of handle a
/// curve point can have (decision 0139): the version 41 sample holds
/// exactly this.
pub(super) fn rich_document_v41() -> Document {
    let mut doc = rich_document_v41_smooth_only();
    let kinds = add(
        &mut doc,
        "Handle kinds",
        CurveObject::line(CurveShape::Points(
            Curve::corners(
                &[
                    Vec3::ZERO,
                    Vec3::new(40.0, 30.0, 0.0),
                    Vec3::new(80.0, 0.0, 0.0),
                    Vec3::new(120.0, 30.0, 0.0),
                    Vec3::new(160.0, 0.0, 0.0),
                ],
                false,
            )
            .unwrap(),
        )),
    );
    for (point, kind) in [
        (1, PointKind::Smooth),
        (2, PointKind::Broken),
        (3, PointKind::Auto),
    ] {
        doc.execute(Command::EditCurve {
            id: kinds,
            edit: CurveEdit::SetKind {
                points: vec![CurvePointId::from_raw(point)],
                kind,
            },
        })
        .unwrap();
    }
    doc
}

/// Everything version 40 stores, plus curves: a rail swept along a drawn
/// path with another curve as its cross-section, a twisted ring, a lathed
/// vase, a spiral line and an edited arc. The first version 41 sample,
/// saved before curve points had handle kinds (only a smooth switch),
/// holds exactly this.
fn rich_document_v41_smooth_only() -> Document {
    let mut doc = rich_document_v40();
    let profile = add(
        &mut doc,
        "Rail profile",
        CurveObject::line(CurveShape::Points(
            Curve::corners(
                &[
                    Vec3::new(-3.0, 0.0, 0.0),
                    Vec3::new(3.0, 0.0, 0.0),
                    Vec3::new(3.0, 0.0, 2.0),
                    Vec3::new(-3.0, 0.0, 4.0),
                ],
                true,
            )
            .unwrap(),
        )),
    );
    let path = Curve::new(
        [
            (Vec3::ZERO, Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0), false),
            (
                Vec3::new(100.0, 50.0, 20.0),
                Vec3::new(-20.0, -10.0, 0.0),
                Vec3::new(20.0, 10.0, 0.0),
                true,
            ),
            (Vec3::new(200.0, 0.0, 0.0), Vec3::ZERO, Vec3::ZERO, false),
        ],
        false,
    )
    .unwrap();
    add(
        &mut doc,
        "Rail",
        CurveObject {
            shape: CurveShape::Points(path),
            solid: CurveSolid::Sweep(Sweep {
                profile: SweepProfile::Curve(profile),
                twist: 0.0,
                ..Sweep::default()
            }),
        },
    );
    add(
        &mut doc,
        "Ring",
        CurveObject {
            shape: CurveShape::Ready(ReadyCurve::Circle { radius: 40.0 }),
            solid: CurveSolid::Sweep(Sweep {
                profile: SweepProfile::Rectangle,
                width: 6.0,
                height: 3.0,
                twist: 360.0,
                caps: false,
                ..Sweep::default()
            }),
        },
    );
    add(
        &mut doc,
        "Vase",
        CurveObject {
            shape: CurveShape::Ready(ReadyCurve::Line { length: 50.0 }),
            solid: CurveSolid::Lathe(LatheShape {
                axis: LatheAxis::Y,
                angle: 270.0,
                segments: 16,
                ..LatheShape::default()
            }),
        },
    );
    add(
        &mut doc,
        "Spring",
        CurveObject::line(CurveShape::Ready(ReadyCurveKind::Spiral.standard())),
    );
    let arc = add(
        &mut doc,
        "Edited arc",
        CurveObject::line(CurveShape::Ready(ReadyCurveKind::Arc.standard())),
    );
    doc.execute(Command::EditCurve {
        id: arc,
        edit: CurveEdit::Transform {
            parts: vec![CurvePart::point(CurvePointId::from_raw(1))],
            matrix: Mat4::from_translation(Vec3::Z * 10.0),
        },
    })
    .unwrap();
    doc
}

/// Each curve object, by name.
fn curves_of(doc: &Document) -> Vec<(String, CurveObject)> {
    let scene = doc.scene();
    scene
        .walk()
        .into_iter()
        .filter_map(|(id, _)| {
            let object = scene.object(id)?;
            Some((object.name.clone(), object.kind.curve()?.clone()))
        })
        .collect()
}

#[test]
fn curves_are_saved_and_opened_with_their_solids() {
    let doc = rich_document_v41();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &back);
    assert_eq!(curves_of(&doc), curves_of(&back));
    assert_eq!(curves_of(&back).len(), 7);
    let kinds: Vec<PointKind> = curves_of(&back)
        .iter()
        .find(|(name, _)| name == "Handle kinds")
        .map(|(_, c)| c.shape.curve().points().iter().map(|p| p.kind).collect())
        .unwrap();
    assert_eq!(
        kinds,
        vec![
            PointKind::Sharp,
            PointKind::Smooth,
            PointKind::Broken,
            PointKind::Auto,
            PointKind::Sharp
        ]
    );
    let rail = named(&back, "Rail");
    assert_eq!(
        back.scene().evaluated(rail).map(|e| e.mesh),
        doc.scene().evaluated(rail).map(|e| e.mesh),
        "the sweep comes out the same"
    );
}

const SAMPLE_V41: &[u8] = include_bytes!("../../tests/data/sample-v41.avtl");

#[test]
fn a_file_saved_by_format_version_41_opens_with_its_curves() {
    let doc = from_bytes(SAMPLE_V41).expect("the version 41 sample opens");
    let expected = rich_document_v41();
    assert_same(&expected, &doc);
    assert_eq!(curves_of(&expected), curves_of(&doc));
}

const SAMPLE_V41_SMOOTH_ONLY: &[u8] =
    include_bytes!("../../tests/data/sample-v41-smooth-only.avtl");

#[test]
fn a_version_41_file_from_before_handle_kinds_opens_with_kinds_from_its_switch() {
    let doc = from_bytes(SAMPLE_V41_SMOOTH_ONLY).expect("the first version 41 sample opens");
    let expected = rich_document_v41_smooth_only();
    assert_same(&expected, &doc);
    assert_eq!(curves_of(&expected), curves_of(&doc));
    // The rail's middle point was smooth; its ends sharp, one with a
    // handle (now broken: its handle still moves on its own).
    let rail = curves_of(&doc)
        .into_iter()
        .find(|(name, _)| name == "Rail")
        .map(|(_, c)| c.shape.curve())
        .unwrap();
    let kinds: Vec<PointKind> = rail.points().iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        vec![PointKind::Broken, PointKind::Smooth, PointKind::Sharp]
    );
}

#[test]
fn every_handle_kind_survives_a_round_trip_and_a_missing_one_follows_the_switch() {
    let doc = rich_document_v41();
    let mut model = v41::ProjectV41::from_document(&doc);
    let back = model.clone().into_document().unwrap();
    assert_eq!(curves_of(&doc), curves_of(&back));
    // Without kinds, as the first version 41 files: from the switch.
    for curve in &mut model.curves.0 {
        if let v41::ShapeV41::Points { points, .. } = &mut curve.shape {
            for point in &mut points.0 {
                point.kind = None;
            }
        }
    }
    let back = from_bytes(&file(&model)).unwrap();
    let kinds: Vec<PointKind> = curves_of(&back)
        .iter()
        .find(|(name, _)| name == "Handle kinds")
        .map(|(_, c)| c.shape.curve().points().iter().map(|p| p.kind).collect())
        .unwrap();
    assert_eq!(
        kinds,
        vec![
            PointKind::Sharp,
            PointKind::Smooth,
            PointKind::Broken,
            PointKind::Smooth,
            PointKind::Sharp
        ]
    );
}

#[test]
fn older_development_files_open_without_curves() {
    let doc = rich_document_v40();
    let body = v40::encode(&v40::ProjectV40::from_document(&doc)).unwrap();
    let mut bytes = header(40, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert!(curves_of(&back).is_empty());
}

/// A version 41 file holding `model`, uncompressed.
fn file(model: &v41::ProjectV41) -> Vec<u8> {
    let body = v41::encode(model).unwrap();
    let mut bytes = header(41, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn bad_curves_are_refused_in_plain_words() {
    let doc = rich_document_v41();
    let plain = || v41::ProjectV41::from_document(&doc);
    let rail = plain()
        .curves
        .iter()
        .position(|c| matches!(c.solid, v41::SolidV41::Sweep(_)))
        .unwrap();
    // Listed twice.
    let mut model = plain();
    let again = model.curves[0].clone();
    model.curves.push(again);
    assert!(refused(&file(&model)).to_string().contains("twice"));
    // An object that isn't there, or isn't stored as a curve.
    let mut model = plain();
    model.curves[0].id = 999_999;
    assert!(refused(&file(&model)).to_string().contains("isn't there"));
    let cube = named(&doc, "Cube").get();
    let mut model = plain();
    model.curves[0].id = cube;
    assert!(refused(&file(&model)).to_string().contains("curve"));
    // A cross-section that isn't a curve.
    let mut model = plain();
    if let v41::SolidV41::Sweep(s) = &mut model.curves[rail].solid {
        s.profile = v41::ProfileV41::Curve(cube);
    }
    assert!(refused(&file(&model)).to_string().contains("curve"));
    // Points that aren't numbers, repeated ids, too many.
    let mut model = plain();
    if let v41::ShapeV41::Points { points, .. } = &mut model.curves[0].shape {
        points[0].position[0] = f32::NAN;
    }
    assert!(refused(&file(&model)).to_string().contains("number"));
    let mut model = plain();
    if let v41::ShapeV41::Points { points, .. } = &mut model.curves[0].shape {
        points[1].id = points[0].id;
    }
    assert!(refused(&file(&model)).to_string().contains("ids"));
    let mut model = plain();
    if let v41::ShapeV41::Points {
        points, next_id, ..
    } = &mut model.curves[0].shape
    {
        let first = points[0];
        points.0 = vec![first; scene::Curve::default().points().len() + 5000];
        *next_id = u32::MAX;
    }
    assert!(refused(&file(&model)).to_string().contains("curve"));
}

#[test]
fn silly_curve_settings_are_brought_within_limits() {
    let doc = rich_document_v41();
    let mut model = v41::ProjectV41::from_document(&doc);
    for curve in model.curves.iter_mut() {
        match &mut curve.solid {
            v41::SolidV41::Sweep(s) => {
                s.width = f32::NAN;
                s.sides = 0;
                s.segments = 10_000;
                s.twist = f32::INFINITY;
            }
            v41::SolidV41::Lathe(l) => {
                l.angle = -5.0;
                l.segments = 1;
            }
            v41::SolidV41::Line => {}
        }
        if let v41::ShapeV41::Circle { radius } = &mut curve.shape {
            *radius = -1.0;
        }
    }
    let back = from_bytes(&file(&model)).unwrap();
    for (_, curve) in curves_of(&back) {
        assert_eq!(curve.solid, curve.solid.clamped());
        if let CurveShape::Ready(r) = curve.shape {
            assert_eq!(r, r.clamped());
        }
    }
}

#[test]
fn random_version_41_bodies_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let good = v41::encode(&v41::ProjectV41::from_document(&rich_document_v41())).unwrap();
    for round in 0..300 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut body = good.clone();
        // Most changes land in the curves, at the end of the body.
        let tail = body.len() / 4;
        let at = body.len() - 1 - (seed as usize) % tail;
        body[at] ^= (seed >> 32) as u8 | 1;
        if round % 3 == 0 {
            body.truncate(at);
        }
        let mut bytes = header(41, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
