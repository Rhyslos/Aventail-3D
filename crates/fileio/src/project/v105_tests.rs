//! Tests for version 105: version 104 with variations and the variation
//! a graph fill shows (decision 0280).

use super::v104_tests::rich_document_v104;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, SettingValue, VariationId};
use scene::Command;
use std::sync::Arc;

/// The graph given two variations in [`rich_document_v105`].
const LAVA: &str = "Lava";

/// Everything version 104 stores, with the Lava graph given two
/// variations (Calm, then Fierce with a shown setting changed and new
/// seeds), the Default material's graph given one, and its graph fill
/// showing it. The version 105 sample holds exactly this.
pub(crate) fn rich_document_v105() -> Document {
    let mut doc = rich_document_v104();
    let (lava, graph) = graph_named(&doc, LAVA);
    let registry = doc.scene().graphs().registry().clone();
    let show = GraphEdit::ShowInPainter {
        node: BoxId(3),
        key: "low".into(),
        shown: true,
    };
    doc.execute(Command::EditGraph {
        id: lava,
        edit: show,
    })
    .unwrap();
    let calm = doc
        .scene()
        .graphs()
        .get(lava)
        .unwrap()
        .variation_now(&registry, "Calm");
    let mut edits = vec![GraphEdit::AddVariation(calm)];
    edits.push(GraphEdit::SetSetting {
        node: BoxId(3),
        key: "low".into(),
        value: SettingValue::Number(0.25),
    });
    edits.extend(graph.rolled(&registry, 105, None));
    for edit in edits {
        doc.execute(Command::EditGraph { id: lava, edit }).unwrap();
    }
    let fierce = doc
        .scene()
        .graphs()
        .get(lava)
        .unwrap()
        .variation_now(&registry, "Fierce");
    doc.execute(Command::EditGraph {
        id: lava,
        edit: GraphEdit::AddVariation(fierce),
    })
    .unwrap();
    // The Default material's graph fill shows a variation of its graph.
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let fill = set
        .layers
        .iter_mut()
        .find_map(|l| l.graph.as_mut())
        .unwrap();
    let used = fill.graph;
    let variation = VariationId(doc.scene().graphs().get(used).unwrap().next_id());
    let look = doc
        .scene()
        .graphs()
        .get(used)
        .unwrap()
        .variation_now(&registry, "Worn");
    doc.execute(Command::EditGraph {
        id: used,
        edit: GraphEdit::AddVariation(look),
    })
    .unwrap();
    fill.variation = Some(variation);
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

fn graph_named(doc: &Document, name: &str) -> (GraphId, Arc<Graph>) {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == name)
        .map(|(id, g)| (id, g.clone()))
        .unwrap()
}

fn default_fill(doc: &Document) -> paint::GraphFill {
    let set = doc.scene().paint().set("Default").unwrap();
    set.layers.iter().find_map(|l| l.graph.clone()).unwrap()
}

#[test]
fn variations_and_a_layers_pick_survive_a_round_trip() {
    let doc = rich_document_v105();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    assert_eq!(doc.scene().paint(), back.scene().paint());
    let (_, lava) = graph_named(&back, LAVA);
    let names: Vec<&str> = lava.variations().map(|(_, v)| v.name.as_str()).collect();
    assert_eq!(names, ["Calm", "Fierce"]);
    let registry = back.scene().graphs().registry();
    // Fierce is the look the graph was left in; Calm puts the old one back.
    let fierce = lava.variation_in_use(registry).unwrap();
    assert_eq!(lava.variation(fierce).unwrap().name, "Fierce");
    let (calm, _) = lava.variations().next().unwrap();
    let calmed = lava.with_variation(registry, calm);
    assert_eq!(
        calmed.get(BoxId(3)).unwrap().settings.get("low"),
        lava.variation(calm).unwrap().value(BoxId(3), "low")
    );
    assert_ne!(calmed.seed(), lava.seed());
    let fill = default_fill(&back);
    let picked = fill.variation.unwrap();
    let graph = back.scene().graphs().get(fill.graph).unwrap();
    assert_eq!(graph.variation(picked).unwrap().name, "Worn");
}

#[test]
fn nothing_new_is_stored_without_variations() {
    let mut graph = Graph::new("Plain");
    graph
        .apply(
            matgraph::Registry::standard(),
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: glam::Vec2::ZERO,
            },
        )
        .unwrap();
    let stored = v95::GraphV95::of(GraphId(1), &graph);
    let text = serde_json::to_string(&stored).unwrap();
    assert!(!text.contains("variations"), "{text}");
    let fill = v97::graph_fill_of(&paint::GraphFill::of(GraphId(1)));
    let text = serde_json::to_string(&fill).unwrap();
    assert!(!text.contains("variation"), "{text}");
}

#[test]
fn damaged_variations_in_a_file_are_refused() {
    let doc = rich_document_v105();
    let (id, lava) = graph_named(&doc, LAVA);
    let registry = matgraph::Registry::standard();
    let hurts: [fn(&mut v105::VariationV105); 4] = [
        |v| v.name = String::new(),
        |v| v.id = 1,
        |v| v.moment = Some(f32::NAN),
        |v| v.values.0[0].value = v95::ValueV95::Number(f32::INFINITY),
    ];
    for hurt in hurts {
        let mut stored = v95::GraphV95::of(id, &lava);
        hurt(&mut stored.variations.0[0]);
        let ok = stored
            .into_parts()
            .ok()
            .and_then(|(_, parts)| Graph::from_parts(parts, registry).ok());
        assert!(ok.is_none());
    }
    // A layer naming a variation the graph doesn't have opens, and shows
    // the graph as it is.
    let mut fill = v97::graph_fill_of(&default_fill(&doc));
    fill.variation = Some(9999);
    let back = v97::graph_fill_from(fill).unwrap();
    let graph = doc.scene().graphs().get(back.graph).unwrap();
    assert!(graph.variation(back.variation.unwrap()).is_none());
    assert_eq!(
        &graph.with_variation(registry, back.variation.unwrap()),
        &**graph
    );
}

#[test]
fn flipped_bytes_in_a_file_with_variations_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v105()).unwrap();
    let mut state = 105u64;
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
            let registry = doc.scene().graphs().registry().clone();
            for (_, graph) in doc.scene().graphs().iter() {
                assert!(graph.variations().count() <= matgraph::MAX_VARIATIONS);
                for (id, v) in graph.variations() {
                    assert!(!v.name.trim().is_empty());
                    assert!(v.moment.is_finite());
                    // Using any of them works on what the file holds.
                    let _ = graph.with_variation(&registry, id);
                }
            }
        }
    }
}

const SAMPLE_V105: &[u8] = include_bytes!("../../tests/data/sample-v105.avtl");

#[test]
fn a_file_saved_by_format_version_105_opens_with_its_variations() {
    let doc = from_bytes(SAMPLE_V105).expect("the version 105 sample opens");
    let rich = rich_document_v105();
    assert_eq!(doc.scene().graphs(), rich.scene().graphs());
    assert_eq!(default_fill(&doc).variation, default_fill(&rich).variation);
}

#[test]
fn a_version_104_file_opens_without_variations() {
    const SAMPLE_V104: &[u8] = include_bytes!("../../tests/data/sample-v104.avtl");
    let doc = from_bytes(SAMPLE_V104).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert_eq!(graph.variations().count(), 0);
    }
    assert_eq!(default_fill(&doc).variation, None);
}
