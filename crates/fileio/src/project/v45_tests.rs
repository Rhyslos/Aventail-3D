//! Tests for version 45: version 44 with lattice cages and the Lattice
//! modifier (decision 0144).

use super::tests::{assert_same, header};
use super::v44_tests::rich_document_v44;
use super::*;
use glam::{Mat4, Vec3};
use scene::{
    Command, Interpolation, Lattice, LatticeEdit, LatticeOptions, ModifierKind, ObjectId,
    ObjectKind, PrimitiveKind,
};

/// Every cage in `doc`, in tree order.
fn cages(doc: &Document) -> Vec<(ObjectId, Lattice)> {
    doc.scene()
        .walk()
        .iter()
        .filter_map(|(id, _)| Some((*id, doc.scene().object(*id)?.kind.lattice()?.clone())))
        .collect()
}

/// Every Lattice modifier in `doc`: its object and its cage.
fn lattice_modifiers(doc: &Document) -> Vec<(ObjectId, ObjectId)> {
    doc.scene()
        .walk()
        .iter()
        .flat_map(|(id, _)| {
            doc.scene()
                .modifiers(*id)
                .iter()
                .filter_map(|m| match m.kind {
                    ModifierKind::Lattice(o) => Some((*id, o.cage)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Everything version 44 stores, plus a sphere in a 3×2×4 cage with one
/// point moved, and two cubes sharing a linear 2×2×2 cage at rest: the
/// version 45 sample holds exactly this.
pub(super) fn rich_document_v45() -> Document {
    let mut doc = rich_document_v44();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let sphere = doc.selection()[0];
    doc.execute(Command::AddLattice {
        ids: vec![sphere],
        divisions: [3, 2, 4],
    })
    .unwrap();
    let cage = doc.selection()[0];
    doc.execute(Command::EditLattice {
        id: cage,
        edit: LatticeEdit::Transform {
            points: vec![5],
            matrix: Mat4::from_translation(Vec3::new(10.0, -5.0, 30.0)),
        },
    })
    .unwrap();
    let mut cubes = Vec::new();
    for _ in 0..2 {
        doc.execute(Command::Create(ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        cubes.push(doc.selection()[0]);
    }
    doc.execute(Command::AddLattice {
        ids: cubes,
        divisions: [2, 2, 2],
    })
    .unwrap();
    let shared = doc.selection()[0];
    doc.execute(Command::EditLattice {
        id: shared,
        edit: LatticeEdit::Interpolation(Interpolation::Linear),
    })
    .unwrap();
    doc
}

#[test]
fn lattices_survive_a_round_trip() {
    let doc = rich_document_v45();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(cages(&doc), cages(&back));
    assert_eq!(cages(&back).len(), 2);
    assert_eq!(lattice_modifiers(&doc), lattice_modifiers(&back));
    assert_eq!(lattice_modifiers(&back).len(), 3);
    for (id, _) in lattice_modifiers(&back) {
        assert_eq!(
            back.scene().exported_shape(id),
            doc.scene().exported_shape(id),
            "the bent shape comes out the same"
        );
    }
}

const SAMPLE_V45: &[u8] = include_bytes!("../../tests/data/sample-v45.avtl");

#[test]
fn a_file_saved_by_format_version_45_opens_with_its_cages() {
    let doc = from_bytes(SAMPLE_V45).expect("the version 45 sample opens");
    let expected = rich_document_v45();
    assert_same(&expected, &doc);
    assert_eq!(cages(&expected), cages(&doc));
    assert_eq!(lattice_modifiers(&expected), lattice_modifiers(&doc));
}

const SAMPLE_V44: &[u8] = include_bytes!("../../tests/data/sample-v44.avtl");

#[test]
fn older_files_open_with_no_cages() {
    let doc = from_bytes(SAMPLE_V44).unwrap();
    assert!(cages(&doc).is_empty());
    assert_same(&rich_document_v44(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
}

#[test]
fn damaged_cages_are_refused() {
    let doc = rich_document_v45();
    let good = v43::ProjectV43::from_document(&doc);
    let refused = |change: &dyn Fn(&mut v43::ProjectV43)| {
        let mut model = good.clone();
        change(&mut model);
        model.into_document().is_err()
    };
    assert!(good.clone().into_document().is_ok());
    assert!(refused(&|m| m.lattices[0].divisions = [1, 2, 4]));
    assert!(refused(&|m| m.lattices[0].divisions = [3, 2, 40]));
    assert!(refused(&|m| {
        m.lattices[0].points.pop();
    }));
    assert!(refused(&|m| m.lattices[0].points[0] = [f32::NAN, 0.0, 0.0]));
    assert!(refused(&|m| m.lattices[0].points[0] = [1e9, 0.0, 0.0]));
    assert!(refused(&|m| m.lattices[0].size = [0.0, 1.0, 1.0]));
    assert!(refused(&|m| m.lattices[0].size = [f32::INFINITY, 1.0, 1.0]));
    // Listed twice, or naming an object that isn't an empty group.
    assert!(refused(&|m| {
        let again = m.lattices[0].clone();
        m.lattices.push(again);
    }));
    assert!(refused(&|m| m.lattices[0].id = 999_999));
    assert!(refused(&|m| {
        let shape = m.modifiers[0].id;
        m.lattices[0].id = shape;
    }));
    // A Lattice modifier naming no cage, or a shape.
    assert!(refused(&|m| {
        let last = m.modifiers.len() - 1;
        m.modifiers[last].list[0].kind = v43::KindV43::Lattice { cage: 999_999 };
    }));
    // A Lattice modifier naming a shape, or a cage without its points
    // (which opens as an empty group), is taken off with a note, so the
    // rest of the project still opens: nothing is silently half-read.
    let taken_off = |change: &dyn Fn(&mut v43::ProjectV43)| {
        let mut model = good.clone();
        change(&mut model);
        let mut notes = Vec::new();
        let doc = model.into_document_noting(&mut notes).unwrap();
        let lattices = doc
            .scene()
            .walk()
            .iter()
            .flat_map(|(id, _)| doc.scene().modifiers(*id).to_vec())
            .filter(|m| matches!(m.kind, scene::ModifierKind::Lattice(_)))
            .count();
        (lattices, notes.len())
    };
    let all = taken_off(&|_| {}).0;
    assert!(all > 0);
    let (left, notes) = taken_off(&|m| {
        let last = m.modifiers.len() - 1;
        let shape = m.modifiers[0].id;
        m.modifiers[last].list[0].kind = v43::KindV43::Lattice { cage: shape };
    });
    assert!(left <= all);
    assert_eq!(notes, 1);
    assert_eq!(taken_off(&|m| m.lattices.clear()), (0, all));
}

#[test]
fn random_version_45_bodies_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v45())).unwrap();
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
        let mut bytes = header(45, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}

#[test]
fn a_lattice_modifier_keeps_its_cage_id() {
    let doc = rich_document_v45();
    let model = v43::ProjectV43::from_document(&doc);
    let (_, cage) = lattice_modifiers(&doc)[0];
    assert!(model.modifiers.iter().any(|m| {
        m.list
            .iter()
            .any(|x| x.kind == v43::KindV43::Lattice { cage: cage.get() })
    }));
    assert_eq!(LatticeOptions::default().cage, LatticeOptions::NO_CAGE);
}
