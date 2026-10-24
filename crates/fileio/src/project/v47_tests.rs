//! Tests for version 47: version 46 with the Scatter modifier (decision
//! 0146), which names the shapes it scatters.

use super::tests::{assert_same, header};
use super::v46_tests::rich_document_v46;
use super::*;
use scene::{
    Command, Modifier, ModifierKind, ObjectId, ObjectKind, PlaceOn, PrimitiveKind, ScatterOptions,
    ScatterSettings,
};

/// Every object's Scatter modifiers in `doc`, in tree order.
fn scatters(doc: &Document) -> Vec<(ObjectId, Vec<Modifier>)> {
    doc.scene()
        .walk()
        .iter()
        .map(|(id, _)| {
            let list: Vec<Modifier> = doc
                .scene()
                .modifiers(*id)
                .iter()
                .filter(|m| matches!(m.kind, ModifierKind::Scatter(_)))
                .cloned()
                .collect();
            (*id, list)
        })
        .filter(|(_, list)| !list.is_empty())
        .collect()
}

/// The settings the sample's second Scatter modifier has: every one off
/// its standard.
fn odd_options() -> ScatterOptions {
    ScatterOptions {
        place_on: PlaceOn::Centres,
        count: 7,
        seed: 99,
        spacing: 3.5,
        align: false,
        random_turn: 90.0,
        scale_min: 0.5,
        scale_max: 1.5,
        offset: -2.0,
    }
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

/// Everything version 46 stores, plus a plane with a standard scatter of
/// a cube, and a sphere scattering the cube and a cone on two of its
/// polygons with every setting changed, hidden in the viewport: the
/// version 47 sample holds exactly this.
pub(super) fn rich_document_v47() -> Document {
    let mut doc = rich_document_v46();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let cone = create(&mut doc, PrimitiveKind::Cone);
    doc.execute(Command::AddModifier {
        ids: vec![plane],
        kind: ModifierKind::Scatter(ScatterSettings::of(vec![cube])),
    })
    .unwrap();
    let polygons = doc
        .scene()
        .unmodified_shape(sphere)
        .unwrap()
        .faces()
        .map(|(f, _)| f)
        .take(2)
        .collect();
    doc.execute(Command::AddModifier {
        ids: vec![sphere],
        kind: ModifierKind::Scatter(ScatterSettings {
            options: odd_options(),
            shapes: vec![cube, cone],
            polygons,
        }),
    })
    .unwrap();
    let mut hidden = doc.scene().modifiers(sphere)[0].clone();
    hidden.in_viewport = false;
    doc.execute(Command::ChangeModifier {
        id: sphere,
        index: 0,
        modifier: hidden,
    })
    .unwrap();
    doc
}

#[test]
fn scatter_modifiers_survive_a_round_trip() {
    let doc = rich_document_v47();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(scatters(&doc), scatters(&back));
    assert_eq!(scatters(&back).len(), 2);
    for (id, _) in scatters(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the scattered shape comes out the same"
        );
    }
}

const SAMPLE_V47: &[u8] = include_bytes!("../../tests/data/sample-v47.avtl");

#[test]
fn a_file_saved_by_format_version_47_opens_with_its_scatters() {
    let doc = from_bytes(SAMPLE_V47).expect("the version 47 sample opens");
    let expected = rich_document_v47();
    assert_same(&expected, &doc);
    assert_eq!(scatters(&expected), scatters(&doc));
    let odd: Vec<Modifier> = scatters(&doc)
        .into_iter()
        .flat_map(|(_, list)| list)
        .filter(|m| matches!(&m.kind, ModifierKind::Scatter(s) if s.options == odd_options()))
        .collect();
    assert_eq!(odd.len(), 1);
    assert!(!odd[0].in_viewport);
    let ModifierKind::Scatter(settings) = &odd[0].kind else {
        unreachable!()
    };
    assert_eq!(settings.shapes.len(), 2);
    assert_eq!(settings.polygons.len(), 2);
}

const SAMPLE_V46: &[u8] = include_bytes!("../../tests/data/sample-v46.avtl");

#[test]
fn older_files_open_with_no_scatters() {
    let doc = from_bytes(SAMPLE_V46).unwrap();
    assert!(scatters(&doc).is_empty());
    assert_same(&rich_document_v46(), &doc);
    // And save again as version 47.
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

/// The file model of the sample, and the place of its last Scatter
/// modifier's object in it.
fn model_with_last_scatter() -> (v43::ProjectV43, usize) {
    let model = v43::ProjectV43::from_document(&rich_document_v47());
    let last = model
        .modifiers
        .iter()
        .rposition(|m| matches!(m.list[0].kind, v43::KindV43::Scatter { .. }))
        .unwrap();
    (model, last)
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let (model, last) = model_with_last_scatter();
    let mut model = model;
    let v43::KindV43::Scatter { shapes, .. } = model.modifiers[last].list[0].kind.clone() else {
        panic!("scatter")
    };
    model.modifiers[last].list[0].kind = v43::KindV43::Scatter {
        place_on: 200,
        count: u32::MAX,
        seed: 0,
        spacing: f32::NAN,
        align: true,
        random_turn: -4.0,
        scale_min: f32::INFINITY,
        scale_max: -1.0,
        offset: 1e30,
        shapes,
        polygons: crate::limits::Bounded::new(),
    };
    let id = ObjectId::from_raw(model.modifiers[last].id);
    let back = model.into_document().unwrap();
    let ModifierKind::Scatter(s) = &back.scene().modifiers(id)[0].kind else {
        panic!("scatter")
    };
    let o = s.options;
    assert_eq!(o.place_on, PlaceOn::Random);
    assert_eq!(o.count, ScatterOptions::MAX_COPIES);
    assert_eq!(o.spacing, 0.0);
    assert_eq!(o.random_turn, 0.0);
    assert_eq!((o.scale_min, o.scale_max), (ScatterOptions::MIN_SCALE, 1.0));
    assert_eq!(o.offset, ScatterOptions::MAX_DISTANCE);
}

#[test]
fn a_scatter_naming_nothing_or_a_loop_is_refused_as_damaged() {
    let (model, last) = model_with_last_scatter();
    let id = model.modifiers[last].id;
    let with_shapes = |shapes: Vec<u64>| {
        let mut model = model.clone();
        if let v43::KindV43::Scatter { shapes: s, .. } = &mut model.modifiers[last].list[0].kind {
            *s = shapes.into_iter().collect();
        }
        model.into_document()
    };
    // No shapes, a missing one, itself.
    assert!(with_shapes(vec![]).is_err());
    assert!(with_shapes(vec![987_654]).is_err());
    assert!(with_shapes(vec![id]).is_err());
    // The plane scatters the cube: the sphere scattering the plane is fine,
    // but the cube scattering the plane would be a loop.
    let plane = model.modifiers[last - 1].id;
    assert!(with_shapes(vec![plane]).is_ok());
    let v43::KindV43::Scatter { shapes, .. } = &model.modifiers[last - 1].list[0].kind else {
        panic!("scatter")
    };
    let cube = shapes[0];
    let mut looped = model.clone();
    looped.modifiers[last].id = cube;
    if let v43::KindV43::Scatter { shapes, .. } = &mut looped.modifiers[last].list[0].kind {
        *shapes = [plane].into_iter().collect();
        let _ = shapes;
    }
    assert!(looped.into_document().is_err());
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_47_bodies_never_panic() {
    let mut seed = 0xbb67_ae85_84ca_a73b_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v47())).unwrap();
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
        let mut bytes = header(47, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
