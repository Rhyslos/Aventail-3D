//! Tests for version 104: version 103 with bypassed boxes and reroute
//! dots (decision 0278).

use super::v103_tests::rich_document_v103;
use super::*;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef};
use scene::Command;
use std::sync::Arc;

/// The graph changed in [`rich_document_v104`].
const LAVA: &str = "Lava";

/// The wire into Material from height's height, bent through two dots.
fn bent_wire() -> PlugRef {
    PlugRef::new(BoxId(4), "height")
}

/// Everything version 103 stores, with the Lava graph's Pulse bypassed
/// and reroute dots on two of its wires. The version 104 sample holds
/// exactly this.
pub(crate) fn rich_document_v104() -> Document {
    let mut doc = rich_document_v103();
    let (id, _) = lava_of(&doc);
    let edits = [
        GraphEdit::Bypass {
            node: BoxId(3),
            on: true,
        },
        GraphEdit::SetBends {
            to: bent_wire(),
            points: vec![Vec2::new(480.0, 160.0), Vec2::new(560.5, 162.25)],
        },
        GraphEdit::SetBends {
            to: PlugRef::new(BoxId(3), "in"),
            points: vec![Vec2::new(300.0, -80.0)],
        },
    ];
    for edit in edits {
        doc.execute(Command::EditGraph { id, edit }).unwrap();
    }
    doc
}

fn lava_of(doc: &Document) -> (GraphId, Arc<Graph>) {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == LAVA)
        .map(|(id, g)| (id, g.clone()))
        .unwrap()
}

#[test]
fn bypass_and_dots_survive_a_round_trip() {
    let doc = rich_document_v104();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 104u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let (_, lava) = lava_of(&back);
    assert!(lava.get(BoxId(3)).unwrap().bypassed);
    assert!(!lava.get(BoxId(2)).unwrap().bypassed);
    assert_eq!(
        lava.bends(&bent_wire()),
        [Vec2::new(480.0, 160.0), Vec2::new(560.5, 162.25)]
    );
    assert_eq!(lava.all_bends().count(), 2);
}

#[test]
fn a_graph_without_either_stores_nothing_new() {
    let mut graph = Graph::new("Plain");
    graph
        .apply(
            matgraph::Registry::standard(),
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
    let stored = v95::GraphV95::of(GraphId(1), &graph);
    let text = serde_json::to_string(&stored).unwrap();
    assert!(
        !text.contains("bypassed") && !text.contains("bends"),
        "{text}"
    );
}

#[test]
fn odd_dots_from_a_file_are_dropped_or_refused() {
    let (id, lava) = lava_of(&rich_document_v104());
    let registry = matgraph::Registry::standard();
    // Dots of a wire the file doesn't hold are dropped.
    let mut stored = v95::GraphV95::of(id, &lava);
    stored.bends.0.push(v104::BendsV104 {
        to: 1,
        to_plug: "nowhere".into(),
        points: vec![[0.0, 0.0]].into(),
    });
    let (_, parts) = stored.into_parts().unwrap();
    assert_eq!(&Graph::from_parts(parts, registry).unwrap(), &*lava);
    // Too many, or not a number: the graph is damaged.
    for points in [
        vec![[1.0, 1.0]; matgraph::MAX_BENDS + 1],
        vec![[f32::NAN, 0.0]],
    ] {
        let mut stored = v95::GraphV95::of(id, &lava);
        stored.bends.0[0].points = points.into();
        let (_, parts) = stored.into_parts().unwrap();
        assert!(Graph::from_parts(parts, registry).is_err());
    }
}

#[test]
fn flipped_bytes_in_a_file_with_dots_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v104()).unwrap();
    let mut state = 104u64;
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
                for (to, points) in graph.all_bends() {
                    assert!(graph.wire_into(to).is_some());
                    assert!(points.len() <= matgraph::MAX_BENDS);
                    assert!(points.iter().all(|p| p.is_finite()));
                }
            }
        }
    }
}

const SAMPLE_V104: &[u8] = include_bytes!("../../tests/data/sample-v104.avtl");

#[test]
fn a_file_saved_by_format_version_104_opens_with_its_bypass_and_dots() {
    let doc = from_bytes(SAMPLE_V104).expect("the version 104 sample opens");
    assert_eq!(doc.scene().graphs(), rich_document_v104().scene().graphs());
}

#[test]
fn a_version_103_file_opens_with_nothing_bypassed_and_straight_wires() {
    const SAMPLE_V103: &[u8] = include_bytes!("../../tests/data/sample-v103.avtl");
    let doc = from_bytes(SAMPLE_V103).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(graph.boxes().all(|(_, b)| !b.bypassed));
        assert_eq!(graph.all_bends().count(), 0);
    }
}
