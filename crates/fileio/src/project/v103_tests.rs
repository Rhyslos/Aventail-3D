//! Tests for version 103: version 102 with animated materials (decision
//! 0277): each graph's moment to bake, and settings marked "change in the
//! game".

use super::v102_tests::rich_document_v102;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, GraphId};
use scene::Command;
use std::sync::Arc;
use std::sync::OnceLock;

/// The animated graph's name in [`rich_document_v103`].
const LAVA: &str = "Lava";

/// Everything version 102 stores, with one more graph: Noise → Scroll →
/// Pulse → Material from height → Material out, Scroll's Speed and
/// Pulse's Highest marked "change in the game", baked at 1.5 s. The
/// version 103 sample holds exactly this.
pub(crate) fn rich_document_v103() -> Document {
    let mut doc = rich_document_v102();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new(LAVA))).unwrap();
    let add = |kind: &str, x: f32| GraphEdit::AddBox {
        kind: kind.into(),
        at: glam::Vec2::new(x, 0.0),
    };
    let wire = |from: u64, out: &str, to: u64, input: &str| GraphEdit::Connect {
        from: matgraph::PlugRef::new(BoxId(from), out),
        to: matgraph::PlugRef::new(BoxId(to), input),
    };
    let mark = |node: u64, key: &str| GraphEdit::ChangeInGame {
        node: BoxId(node),
        key: key.into(),
        on: true,
    };
    let edits = [
        add("noise", 0.0),
        add("scroll", 200.0),
        add("pulse", 400.0),
        add("material_from_height", 600.0),
        add("material_out", 800.0),
        wire(1, "height", 2, "in"),
        wire(2, "out", 3, "in"),
        wire(3, "out", 4, "height"),
        wire(4, "material", 5, "material"),
        mark(2, "speed"),
        mark(3, "high"),
        GraphEdit::SetMoment(1.5),
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
fn moments_and_marks_survive_a_round_trip() {
    let doc = rich_document_v103();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let (_, lava) = lava_of(&back);
    assert_eq!(lava.moment(), 1.5);
    let marked: Vec<(BoxId, String)> = lava
        .in_game(lava_registry())
        .iter()
        .map(|s| (s.node, s.spec.key.to_string()))
        .collect();
    assert_eq!(
        marked,
        [(BoxId(2), "speed".to_string()), (BoxId(3), "high".into())]
    );
    assert!(lava.is_animated(lava_registry()));
}

fn lava_registry() -> &'static matgraph::Registry {
    static R: OnceLock<matgraph::Registry> = OnceLock::new();
    R.get_or_init(matgraph::Registry::with_standard_boxes)
}

#[test]
fn odd_moments_and_marks_are_brought_in() {
    let (id, lava) = lava_of(&rich_document_v103());
    let mut stored = v95::GraphV95::of(id, &lava);
    stored.moment = Some(f32::NAN);
    stored.boxes.0[1].in_game.0.push("sparkle".into());
    // Pulse's Shape is a choice: it can't change in the game.
    stored.boxes.0[2].in_game.0.push("shape".into());
    let (_, parts) = stored.clone().into_parts().unwrap();
    let back = Graph::from_parts(parts, lava_registry()).unwrap();
    assert_eq!(back.moment(), 0.0);
    assert_eq!(back.in_game(lava_registry()).len(), 2);
    stored.moment = Some(1.0e9);
    let (_, parts) = stored.into_parts().unwrap();
    let back = Graph::from_parts(parts, lava_registry()).unwrap();
    assert_eq!(back.moment(), matgraph::MAX_MOMENT);
}

#[test]
fn a_still_graph_stores_nothing_new() {
    let graph = Graph::new("Still");
    let stored = v95::GraphV95::of(GraphId(1), &graph);
    assert_eq!(stored.moment, None);
    let text = serde_json::to_string(&stored).unwrap();
    assert!(
        !text.contains("moment") && !text.contains("in_game"),
        "{text}"
    );
}

#[test]
fn flipped_bytes_in_a_file_with_animated_graphs_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v103()).unwrap();
    let mut state = 103u64;
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
                assert!((0.0..=matgraph::MAX_MOMENT).contains(&graph.moment()));
                let registry = doc.scene().graphs().registry();
                for shown in graph.in_game(registry) {
                    assert!(matgraph::changes_in_game(shown.spec));
                }
            }
        }
    }
}

const SAMPLE_V103: &[u8] = include_bytes!("../../tests/data/sample-v103.avtl");

#[test]
fn a_file_saved_by_format_version_103_opens_with_its_animation() {
    let doc = from_bytes(SAMPLE_V103).expect("the version 103 sample opens");
    assert_eq!(doc.scene().graphs(), rich_document_v103().scene().graphs());
    assert_eq!(lava_of(&doc).1.moment(), 1.5);
}

#[test]
fn a_version_102_file_opens_at_the_start_with_nothing_marked() {
    const SAMPLE_V102: &[u8] = include_bytes!("../../tests/data/sample-v102.avtl");
    let doc = from_bytes(SAMPLE_V102).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert_eq!(graph.moment(), 0.0);
        assert!(graph.boxes().all(|(_, b)| b.in_game.is_empty()));
    }
}
