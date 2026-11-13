//! Tests for version 99: version 98 with graph fills' Tiling, and
//! Transform and Repeat moving colours and materials (decision 0270).

use super::v98_tests::rich_document_v98;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, PlugRef};
use scene::Command;

/// The Tiling of the sample's graph fill.
const TILING: f32 = 4.0;

/// Everything version 98 stores, with the bricks graph fill at Tiling 4,
/// and a graph "Moved" whose Transform turns a colour and whose Repeat
/// copies a material (the plugs added in version 99's app). The version
/// 99 sample holds exactly this.
pub(crate) fn rich_document_v99() -> Document {
    let mut doc = rich_document_v98();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let fill = set
        .layers
        .iter_mut()
        .find_map(|l| l.graph.as_mut())
        .unwrap();
    fill.tiling = TILING;
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Moved"))).unwrap();
    for (kind, x) in [
        ("noise", 0.0),
        ("gradient_map", 200.0),
        ("transform", 400.0),
        ("material_from_height", 600.0),
        ("repeat", 800.0),
        ("material_out", 1000.0),
    ] {
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::AddBox {
                kind: kind.into(),
                at: glam::Vec2::new(x, 0.0),
            },
        })
        .unwrap();
    }
    for (from, to) in [
        ((1, "height"), (2, "in")),
        ((2, "colour"), (3, "colour")),
        ((1, "height"), (4, "height")),
        ((3, "colour_out"), (4, "colour")),
        ((4, "material"), (5, "material")),
        ((5, "material_out"), (6, "material")),
    ] {
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::Connect {
                from: PlugRef::new(BoxId(from.0), from.1),
                to: PlugRef::new(BoxId(to.0), to.1),
            },
        })
        .unwrap();
    }
    doc
}

#[test]
fn tiling_and_moved_colours_survive_a_round_trip() {
    let doc = rich_document_v99();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let set = back.scene().paint().set("Default").unwrap();
    let fill = set.layers.iter().find_map(|l| l.graph.as_ref()).unwrap();
    assert_eq!(fill.tiling, TILING);
    // The graph moving a colour and a material still works out.
    let (id, graph) = back
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Moved")
        .unwrap();
    let registry = back.scene().graphs().registry();
    let material =
        matgraph::Evaluator::with_threads(1).material(registry, graph, matgraph::Size::nearest(32));
    assert!(material.is_ok(), "{id:?}: {material:?}");
}

#[test]
fn a_stored_tiling_is_kept_within_its_limits_and_must_be_real() {
    assert_eq!(v99::tiling_from(4.0), Some(4.0));
    assert_eq!(v99::tiling_from(0.0), Some(paint::graph_fill::MIN_TILING));
    assert_eq!(v99::tiling_from(1e9), Some(paint::graph_fill::MAX_TILING));
    assert_eq!(v99::tiling_from(-2.0), Some(paint::graph_fill::MIN_TILING));
    assert_eq!(v99::tiling_from(f32::NAN), None);
    assert_eq!(v99::tiling_from(f32::INFINITY), None);
    let fill = paint::GraphFill::of(matgraph::GraphId(2));
    let mut stored = v97::graph_fill_of(&fill);
    stored.tiling = f32::NAN;
    assert!(v97::graph_fill_from(stored.clone()).is_err());
    stored.tiling = 64.0;
    assert_eq!(
        v97::graph_fill_from(stored).unwrap().tiling,
        paint::graph_fill::MAX_TILING
    );
}

#[test]
fn flipped_bytes_in_a_file_with_tiling_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v99()).unwrap();
    let mut state = 99u64;
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
            for (_, set) in doc.scene().paint().sets() {
                assert!(set.is_valid());
            }
        }
    }
}

const SAMPLE_V99: &[u8] = include_bytes!("../../tests/data/sample-v99.avtl");

#[test]
fn a_file_saved_by_format_version_99_opens_with_its_tiling() {
    let doc = from_bytes(SAMPLE_V99).expect("the version 99 sample opens");
    assert_eq!(doc.scene().paint(), rich_document_v99().scene().paint());
    assert_eq!(doc.scene().graphs(), rich_document_v99().scene().graphs());
}

#[test]
fn a_version_98_file_opens_with_its_graph_fills_at_tiling_one() {
    const SAMPLE_V98: &[u8] = include_bytes!("../../tests/data/sample-v98.avtl");
    let doc = from_bytes(SAMPLE_V98).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    let fills: Vec<_> = set.layers.iter().filter_map(|l| l.graph.as_ref()).collect();
    assert!(!fills.is_empty());
    assert!(fills.iter().all(|f| f.tiling == 1.0));
}
