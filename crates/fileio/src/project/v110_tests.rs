//! Tests for version 110: version 109 with the tags of each graph and own
//! box (decision 0292).

use super::v98_tests::PEBBLES;
use super::v109_tests::rich_document_v109;
use super::*;
use matgraph::{GraphEdit, GraphId, OwnBox};
use scene::Command;

/// Everything version 109 stores, with the tags "stone" and "hand
/// painted" on the graph Pebbles, and "pebbles" and "scatter" on the own
/// box Pebbles. The version 110 sample holds exactly this.
pub(crate) fn rich_document_v110() -> Document {
    let mut doc = rich_document_v109();
    let id = pebbles_graph(&doc);
    doc.execute(Command::EditGraph {
        id,
        edit: GraphEdit::SetTags(vec!["Stone".into(), "hand  painted".into()]),
    })
    .unwrap();
    let mut own: OwnBox = (**doc.scene().graphs().own(PEBBLES).unwrap()).clone();
    own.tags = vec!["pebbles".into(), "scatter".into()];
    doc.execute(Command::SetOwnBox {
        id: PEBBLES.into(),
        own: Some(own),
    })
    .unwrap();
    doc
}

fn pebbles_graph(doc: &Document) -> GraphId {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Pebbles")
        .map(|(id, _)| id)
        .unwrap()
}

#[test]
fn tags_survive_a_round_trip() {
    let doc = rich_document_v110();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let graphs = back.scene().graphs();
    let graph = graphs.get(pebbles_graph(&back)).unwrap();
    assert_eq!(graph.tags(), ["hand painted", "stone"], "tidied and sorted");
    assert_eq!(graphs.own(PEBBLES).unwrap().tags, ["pebbles", "scatter"]);
}

#[test]
fn nothing_new_is_stored_without_tags() {
    let doc = rich_document_v109();
    let graphs = doc.scene().graphs();
    for (id, graph) in graphs.iter() {
        let text = serde_json::to_string(&v95::GraphV95::of(id, graph)).unwrap();
        assert!(!text.contains("\"tags\""), "{text}");
    }
    for own in graphs.own_boxes() {
        let text = serde_json::to_string(&v98::own_box_of(own)).unwrap();
        assert!(!text.contains("\"tags\""), "{text}");
    }
}

#[test]
fn untidy_tags_in_a_file_are_tidied_not_refused() {
    let doc = rich_document_v110();
    let id = pebbles_graph(&doc);
    let graph = doc.scene().graphs().get(id).unwrap().clone();
    let mut stored = v95::GraphV95::of(id, &graph);
    let mut tags: Vec<String> = (0..40).map(|i| format!("Tag {i:02}")).collect();
    tags.push(String::new());
    tags.push("x".repeat(500));
    tags.push("tag 00".into());
    stored.tags = tags.into();
    let (_, parts) = stored.into_parts().unwrap();
    let registry = doc.scene().graphs().registry().clone();
    let back = matgraph::Graph::from_parts(parts, &registry).unwrap();
    assert_eq!(back.tags().len(), matgraph::tags::MAX_TAGS);
    assert_eq!(back.tags()[0], "tag 00");
    assert!(matgraph::tags::are_tidy(back.tags()));
    // An own box's too.
    let own = doc.scene().graphs().own(PEBBLES).unwrap().clone();
    let mut stored = v98::own_box_of(&own);
    stored.tags = vec!["B".to_string(), "a".into(), "a".into(), " ".into()].into();
    let read = v98::own_boxes_from(
        doc.scene()
            .graphs()
            .own_boxes()
            .filter(|o| o.id != PEBBLES)
            .map(|o| v98::own_box_of(o))
            .chain(std::iter::once(stored))
            .collect(),
    )
    .unwrap();
    let read = read.iter().find(|o| o.id == PEBBLES).unwrap();
    assert_eq!(read.tags, ["a", "b"]);
}

#[test]
fn flipped_bytes_in_a_file_with_tags_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v110()).unwrap();
    let mut state = 110u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 20
    };
    for _ in 0..200 {
        let mut bytes = full.clone();
        for _ in 0..(next() % 3 + 1) {
            let at = (next() as usize) % bytes.len();
            bytes[at] ^= 1 << (next() % 8);
        }
        if let Ok(doc) = from_bytes(&bytes) {
            for (_, graph) in doc.scene().graphs().iter() {
                assert!(matgraph::tags::are_tidy(graph.tags()));
            }
            for own in doc.scene().graphs().own_boxes() {
                assert!(matgraph::tags::are_tidy(&own.tags));
            }
        }
    }
}

const SAMPLE_V110: &[u8] = include_bytes!("../../tests/data/sample-v110.avtl");

#[test]
fn a_file_saved_by_format_version_110_opens_with_its_tags() {
    let doc = from_bytes(SAMPLE_V110).expect("the version 110 sample opens");
    let rich = rich_document_v110();
    assert_eq!(doc.scene().graphs(), rich.scene().graphs());
}

#[test]
fn a_version_109_file_opens_without_tags() {
    const SAMPLE_V109: &[u8] = include_bytes!("../../tests/data/sample-v109.avtl");
    let doc = from_bytes(SAMPLE_V109).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(graph.tags().is_empty());
    }
    assert!(doc.scene().graphs().own_boxes().all(|o| o.tags.is_empty()));
}
