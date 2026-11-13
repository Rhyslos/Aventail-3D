//! Tests for version 102: version 101 with edges, varied copies and veins
//! on the shape canvas (decision 0275).

use super::v100_tests::leaf_drawing;
use super::v101_tests::{rich_document_v101, twig_drawing};
use super::*;
use matgraph::{BoxId, GraphEdit};
use scene::Command;
use shapes::{Drawing, Edge, EdgeKind, PieceId, Vary, Vein};
use std::sync::Arc;

/// Version 101's twig with a serrated edge on its linked leaves (shared),
/// varied copies, one leaf given a new seed, a torn bite and the vein made
/// a lighter ridge.
pub(crate) fn varied_drawing() -> Drawing {
    let d = twig_drawing();
    let leaf = PieceId(1);
    let bite = PieceId(2);
    let vein = PieceId(3);
    let copy = d
        .pieces()
        .iter()
        .find(|p| p.id != leaf && p.link == d.piece(leaf).unwrap().link)
        .unwrap()
        .id;
    d.with_edge(
        leaf,
        Edge {
            kind: EdgeKind::Serrated,
            amount: 0.7,
            size: 0.025,
            seed: 11,
        },
    )
    .unwrap()
    .with_vary(
        leaf,
        Vary {
            on: true,
            size: 0.2,
            angle: 0.4,
            colour: 0.3,
        },
    )
    .unwrap()
    .with_new_seeds(&[copy])
    .unwrap()
    .with_edge(
        bite,
        Edge {
            kind: EdgeKind::Torn,
            amount: 1.0,
            size: 0.01,
            seed: 2,
        },
    )
    .unwrap()
    .with_vein(
        vein,
        Vein {
            width: 0.02,
            depth: 0.4,
            shade: 0.3,
        },
    )
    .unwrap()
}

/// Everything version 101 stores, with the Leaf graph's Shape box holding
/// [`varied_drawing`]. The version 102 sample holds exactly this.
pub(crate) fn rich_document_v102() -> Document {
    let mut doc = rich_document_v101();
    let id = doc
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Leaf")
        .map(|(id, _)| id)
        .unwrap();
    doc.execute(Command::EditGraph {
        id,
        edit: GraphEdit::SetDrawing {
            node: BoxId(1),
            drawing: Arc::new(varied_drawing()),
        },
    })
    .unwrap();
    doc
}

fn leaf_of(doc: &Document) -> Arc<Drawing> {
    let (_, graph) = doc
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Leaf")
        .unwrap();
    graph.get(BoxId(1)).unwrap().drawing.clone().unwrap()
}

#[test]
fn edges_varying_and_veins_survive_a_round_trip() {
    let doc = rich_document_v102();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 102u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let d = leaf_of(&back);
    assert_eq!(*d, varied_drawing());
    assert_eq!(d.key(), varied_drawing().key());
    let leaf = d.piece(PieceId(1)).unwrap();
    assert_eq!(leaf.edge.kind, EdgeKind::Serrated);
    assert!(leaf.vary.on);
    assert_eq!(d.piece(PieceId(3)).unwrap().vein.depth, 0.4);
    // The seeds made a different look for each copy, kept as it was.
    let paths: Vec<_> = d.pieces().iter().map(|p| d.drawn_paths(p)).collect();
    let again: Vec<_> = varied_drawing()
        .pieces()
        .iter()
        .map(|p| varied_drawing().drawn_paths(p))
        .collect();
    assert_eq!(paths, again);
}

#[test]
fn damaged_edges_are_refused_and_odd_numbers_brought_in() {
    let good = v100::drawing_of(&varied_drawing());
    assert_eq!(v100::drawing_from(good.clone()).unwrap(), varied_drawing());
    // An edge kind this app doesn't know.
    let mut unknown = good.clone();
    let bite = unknown
        .pieces
        .0
        .iter()
        .position(|p| p.name == "Bite")
        .unwrap();
    unknown.pieces.0[bite].edge.as_mut().unwrap().kind = 9;
    assert!(
        v100::drawing_from(unknown)
            .unwrap_err()
            .contains("drawn shape")
    );
    // Not a number.
    let mut wild = good.clone();
    wild.pieces.0[bite].edge.as_mut().unwrap().amount = f32::NAN;
    assert!(v100::drawing_from(wild).is_err());
    // Linked copies whose edges differ.
    let mut mixed = good.clone();
    let leaf = mixed.pieces.0.iter().position(|p| p.id == 1).unwrap();
    mixed.pieces.0[leaf].edge = None;
    assert!(v100::drawing_from(mixed).is_err());
    // Out of range: brought in.
    let mut big = good;
    let e = big.pieces.0[bite].edge.as_mut().unwrap();
    e.amount = 50.0;
    e.size = -3.0;
    let fixed = v100::drawing_from(big).unwrap();
    let edge = fixed.piece(PieceId(2)).unwrap().edge;
    assert_eq!(edge.amount, 1.0);
    assert_eq!(edge.size, *shapes::edge::SIZE.start());
}

#[test]
fn flipped_bytes_in_a_file_with_edges_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v102()).unwrap();
    let mut state = 102u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 20
    };
    for _ in 0..300 {
        let mut bytes = full.clone();
        for _ in 0..(next() % 3 + 1) {
            let at = (next() as usize) % bytes.len();
            bytes[at] ^= 1 << (next() % 8);
        }
        if let Ok(doc) = from_bytes(&bytes) {
            for (_, graph) in doc.scene().graphs().iter() {
                for (_, b) in graph.boxes() {
                    if let Some(d) = &b.drawing {
                        assert_eq!(Drawing::from_parts(d.to_parts()).as_ref(), Ok(&**d));
                        let _ = shapes::raster::layers(d, 16);
                    }
                }
            }
        }
    }
}

const SAMPLE_V102: &[u8] = include_bytes!("../../tests/data/sample-v102.avtl");

#[test]
fn a_file_saved_by_format_version_102_opens_with_its_edges() {
    let doc = from_bytes(SAMPLE_V102).expect("the version 102 sample opens");
    assert_eq!(doc.scene().graphs(), rich_document_v102().scene().graphs());
    assert_eq!(*leaf_of(&doc), varied_drawing());
}

#[test]
fn a_version_101_file_opens_smooth_unvaried_and_with_no_veins() {
    const SAMPLE_V101: &[u8] = include_bytes!("../../tests/data/sample-v101.avtl");
    let doc = from_bytes(SAMPLE_V101).unwrap();
    let twig = leaf_of(&doc);
    assert_eq!(*twig, twig_drawing());
    for p in twig.pieces() {
        assert_eq!(p.edge, Edge::SMOOTH);
        assert_eq!(p.vary, Vary::OFF);
        assert_eq!(p.vein, Vein::NONE);
        assert_eq!(p.seed, shapes::drawing::first_seed(p.id));
    }
    // So it looks as it did: no vein in the pictures.
    assert!(shapes::raster::veins(&twig, 32).is_empty());
    assert_eq!(leaf_drawing().pieces()[2].vein, Vein::NONE);
}
