//! Tests for version 56: version 55 with each object's levels of detail
//! (decision 0155).

use super::tests::{assert_same, header};
use super::v55_tests::rich_document_v55;
use super::*;
use scene::{Command, LodLevel, Lods, ObjectId, ObjectKind, PrimitiveKind};

/// Every object's levels of detail in `doc` that aren't the standard
/// ones, in tree order.
fn lods(doc: &Document) -> Vec<(ObjectId, Lods)> {
    doc.scene()
        .walk()
        .iter()
        .filter_map(|(id, _)| {
            let o = doc.scene().object(*id)?;
            (o.lods != Lods::default()).then(|| (*id, o.lods.clone()))
        })
        .collect()
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// The sample's odd settings: two levels, the second previewed.
fn odd() -> Lods {
    Lods {
        on: true,
        levels: vec![
            LodLevel {
                ratio: 0.4,
                distance: 1_500.0,
            },
            LodLevel {
                ratio: 0.1,
                distance: 6_000.0,
            },
        ],
        preview: 2,
    }
}

/// Everything version 55 stores, plus a sphere with the standard levels
/// on, a torus with odd ones, and a cone whose odd levels are switched
/// off (kept): the version 56 sample holds exactly this.
pub(super) fn rich_document_v56() -> Document {
    let mut doc = rich_document_v55();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let torus = create(&mut doc, PrimitiveKind::Torus);
    let cone = create(&mut doc, PrimitiveKind::Cone);
    doc.execute(Command::SetLods {
        ids: vec![sphere],
        lods: Lods {
            on: true,
            ..Lods::default()
        },
    })
    .unwrap();
    doc.execute(Command::SetLods {
        ids: vec![torus, cone],
        lods: odd(),
    })
    .unwrap();
    doc.execute(Command::SetLods {
        ids: vec![cone],
        lods: Lods { on: false, ..odd() },
    })
    .unwrap();
    doc
}

#[test]
fn levels_of_detail_survive_a_round_trip() {
    let doc = rich_document_v56();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(lods(&doc), lods(&back));
    assert_eq!(lods(&back).len(), 3);
}

const SAMPLE_V56: &[u8] = include_bytes!("../../tests/data/sample-v56.avtl");

#[test]
fn a_file_saved_by_format_version_56_opens_with_its_levels_of_detail() {
    let doc = from_bytes(SAMPLE_V56).expect("the version 56 sample opens");
    let expected = rich_document_v56();
    assert_same(&expected, &doc);
    let found = lods(&doc);
    assert_eq!(lods(&expected), found);
    let has = |l: &Lods| found.iter().filter(|(_, f)| f == l).count();
    assert_eq!(
        has(&Lods {
            on: true,
            ..Lods::default()
        }),
        1
    );
    assert_eq!(has(&odd()), 1);
    assert_eq!(has(&Lods { on: false, ..odd() }), 1);
}

const SAMPLE_V55: &[u8] = include_bytes!("../../tests/data/sample-v55.avtl");

#[test]
fn older_files_open_with_no_levels_of_detail() {
    let doc = from_bytes(SAMPLE_V55).unwrap();
    assert!(lods(&doc).is_empty());
    assert_same(&rich_document_v55(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The sample's file model, and the place of the torus's settings.
fn model() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v56());
    let at = objects(&model)
        .iter()
        .position(|o| o.lods.as_ref().is_some_and(|l| l.on && l.preview == 2))
        .unwrap();
    (model, at)
}

/// The development settings of every object in `model`.
fn objects(model: &v43::ProjectV43) -> &Vec<super::v39::ObjectV39> {
    &model.project.project.project.objects.0
}

fn objects_mut(model: &mut v43::ProjectV43) -> &mut Vec<super::v39::ObjectV39> {
    &mut model.project.project.project.objects.0
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (mut model, at) = model();
    let id = ObjectId::from_raw(objects(&model)[at].id);
    objects_mut(&mut model)[at].lods = Some(super::v56::LodsV56 {
        on: true,
        levels: (0..40)
            .map(|n| {
                if n % 2 == 0 {
                    (f32::NAN, -5.0)
                } else {
                    (9.0, f32::INFINITY)
                }
            })
            .collect(),
        preview: u32::MAX,
    });
    let back = model.into_document().unwrap();
    let lods = &back.scene().object(id).unwrap().lods;
    assert_eq!(lods.levels.len(), Lods::MAX_LEVELS);
    assert_eq!(lods.preview, Lods::MAX_LEVELS);
    assert_eq!(lods.levels[0].ratio, 0.5, "not a number: the standard");
    assert_eq!(lods.levels[0].distance, 0.0);
    assert_eq!(lods.levels[1].ratio, 1.0);
    assert_eq!(lods.levels[1].distance, 0.0, "not a number is 0");
    // No levels at all: the standard three.
    let (mut model, at) = super::v56_tests::model();
    objects_mut(&mut model)[at].lods = Some(super::v56::LodsV56 {
        on: true,
        levels: Vec::new().into(),
        preview: 1,
    });
    let back = model.into_document().unwrap();
    assert_eq!(back.scene().object(id).unwrap().lods.levels, Lods::STANDARD);
}

#[test]
fn levels_on_for_an_object_with_no_shape_are_refused() {
    let mut doc = rich_document_v56();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    let mut model = v43::ProjectV43::from_document(&doc);
    let list = objects_mut(&mut model);
    let at = list.iter().position(|o| o.id == group.get()).unwrap();
    list[at].lods = Some(super::v56::LodsV56 {
        on: true,
        levels: vec![(0.5, 100.0)].into(),
        preview: 0,
    });
    assert!(model.into_document().is_err());
}

#[test]
fn random_version_56_bodies_never_panic() {
    let mut seed = 0x0056_10d5_0fde_7a11_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v56())).unwrap();
    for round in 0..300 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut body = good.clone();
        let at = (seed as usize) % body.len();
        body[at] ^= (seed >> 32) as u8 | 1;
        if round % 3 == 0 {
            body.truncate(at);
        }
        let mut bytes = header(56, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
