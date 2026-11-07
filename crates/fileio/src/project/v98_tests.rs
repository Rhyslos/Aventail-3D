//! Tests for version 98: version 97 with the project's own boxes
//! (decision 0263).

use super::v97_tests::rich_document_v97;
use super::*;
use matgraph::own::Proposal;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, SettingValue};
use scene::Command;

/// The id of the sample's own box "Lumpy" (fixed, so the sample always
/// matches), and of "Pebbles", which holds it.
pub(crate) const LUMPY: &str = "own:00000000000000a1";
pub(crate) const PEBBLES: &str = "own:00000000000000b2";

/// Makes `selection` of graph `id` into own box `own_id` called `name`,
/// with the settings `sliders` (by key) chosen and a picture, as the
/// canvas does: one undo step.
fn make(
    doc: &mut Document,
    id: GraphId,
    selection: &[u64],
    name: &str,
    own_id: &str,
    sliders: &[&str],
) {
    let graph = doc.scene().graphs().get(id).unwrap().clone();
    let registry = doc.scene().graphs().registry().clone();
    let selection = selection.iter().map(|b| BoxId(*b)).collect();
    let mut proposal = Proposal::of(&graph, &registry, &selection).unwrap();
    proposal.name = name.into();
    proposal.about = format!("{name}: made for the sample.");
    for s in &mut proposal.sliders {
        s.chosen = sliders.contains(&s.setting.as_str());
    }
    let side = matgraph::own::PICTURE_SIDE as usize;
    let picture = (0..side * side * 3).map(|i| (i % 251) as u8).collect();
    let (own, edits) = proposal
        .make(&graph, &registry, own_id.into(), Some(picture))
        .unwrap();
    let commands = std::iter::once(Command::SetOwnBox {
        id: own_id.into(),
        own: Some(own),
    })
    .chain(
        edits
            .into_iter()
            .map(|edit| Command::EditGraph { id, edit }),
    );
    doc.execute_all_labelled(commands, "Make your own box", 9_800)
        .unwrap();
}

/// Everything version 97 stores, with a graph "Pebbles" made of Noise,
/// Levels and Blur into a material, where Levels became the own box
/// "Lumpy" (Darkest a slider, set to 0.3 on the placed box) and Lumpy and
/// Blur then became "Pebbles", which holds it. The version 98 sample holds
/// exactly this.
pub(crate) fn rich_document_v98() -> Document {
    let mut doc = rich_document_v97();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Pebbles")))
        .unwrap();
    for (kind, x) in [
        ("noise", 0.0),
        ("levels", 200.0),
        ("blur", 400.0),
        ("material_from_height", 600.0),
        ("material_out", 800.0),
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
        ((2, "out"), (3, "in")),
        ((3, "out"), (4, "height")),
        ((4, "material"), (5, "material")),
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
    make(&mut doc, id, &[2], "Lumpy", LUMPY, &["low"]);
    // Lumpy is box 6.
    doc.execute(Command::EditGraph {
        id,
        edit: GraphEdit::SetSetting {
            node: BoxId(6),
            key: "s1".into(),
            value: SettingValue::Number(0.3),
        },
    })
    .unwrap();
    make(&mut doc, id, &[6, 3], "Pebbles", PEBBLES, &["s1"]);
    doc
}

#[test]
fn own_boxes_survive_a_round_trip() {
    let doc = rich_document_v98();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 98u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let graphs = back.scene().graphs();
    assert_eq!(graphs.own_count(), 2);
    let pebbles = graphs.own(PEBBLES).unwrap();
    assert!(pebbles.uses().contains(LUMPY));
    assert!(pebbles.picture.is_some());
    assert_eq!(pebbles.about, "Pebbles: made for the sample.");
    // The graph works out the same material.
    let mut evaluator = matgraph::Evaluator::with_threads(1);
    let size = matgraph::Size::nearest(32);
    let graph = graphs.iter().last().unwrap().1;
    let made = evaluator.material(graphs.registry(), graph, size).unwrap();
    let before = doc.scene().graphs();
    let first = evaluator
        .material(before.registry(), before.iter().last().unwrap().1, size)
        .unwrap();
    assert_eq!(made, first);
}

#[test]
fn own_boxes_with_no_graphs_are_kept() {
    let mut doc = rich_document_v98();
    let ids: Vec<GraphId> = doc.scene().graphs().iter().map(|(id, _)| id).collect();
    for id in ids {
        doc.execute(Command::SetGraph { id, graph: None }).unwrap();
    }
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().graphs().own_count(), 2);
    assert!(back.scene().graphs().is_empty());
}

#[test]
fn damaged_own_boxes_are_refused() {
    let stored = v95::graphs_of(&rich_document_v98()).unwrap();
    assert_eq!(stored.own.len(), 2);
    let opens = |s: &v95::GraphsV95| v95::with_graphs(Document::new(), Some(s.clone()));
    assert!(opens(&stored).is_ok());
    // Two sharing an id.
    let mut bad = stored.clone();
    let first = bad.own[0].clone();
    bad.own.push(first);
    assert!(opens(&bad).is_err());
    // A kind number this version doesn't know.
    let mut bad = stored.clone();
    bad.own[0].outputs[0].kind = 9;
    assert!(opens(&bad).is_err());
    // Holding each other in a loop.
    let mut bad = stored.clone();
    let lumpy = bad.own.iter().position(|o| o.id == LUMPY).unwrap();
    bad.own[lumpy].inner.boxes[0].kind = PEBBLES.into();
    assert!(opens(&bad).unwrap_err().contains("loop"));
    // A slider for a box not inside.
    let mut bad = stored.clone();
    bad.own[lumpy].sliders[0].node = 999;
    assert!(opens(&bad).is_err());
    // A picture of the wrong size.
    let mut bad = stored.clone();
    bad.own[0].picture = Some(vec![1, 2, 3].into());
    assert!(opens(&bad).is_err());
    // A graph using an own box the file doesn't hold keeps it as an
    // unknown kind (it refuses to run, with a reason).
    let mut gone = stored;
    gone.own.clear();
    let doc = opens(&gone).unwrap();
    let graph = doc.scene().graphs().iter().last().unwrap().1.clone();
    assert!(graph.boxes().any(|(_, b)| b.kind == PEBBLES));
}

/// A simple repeatable random number source.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 20
    }
}

#[test]
fn hostile_own_boxes_never_crash_the_reader() {
    let stored = v95::graphs_of(&rich_document_v98()).unwrap();
    let mut rng = Lcg(98);
    let mut opened = 0;
    for _ in 0..1500 {
        let mut s = stored.clone();
        for _ in 0..(rng.next() % 4 + 1) {
            let o = (rng.next() as usize) % s.own.len();
            let own = &mut s.own[o];
            match rng.next() % 10 {
                0 => {
                    own.id = ["own:", "own:zz", LUMPY, PEBBLES, "noise"][(rng.next() % 5) as usize]
                        .into()
                }
                1 => own.name = ["", "x", &"n".repeat(300)][(rng.next() % 3) as usize].into(),
                2 => {
                    if let Some(i) = own.inputs.get_mut(0) {
                        i.to.push(v98::PlugV98 {
                            node: rng.next() % 12,
                            plug: ["in", "height", "", "out"][(rng.next() % 4) as usize].into(),
                        });
                    }
                }
                3 => {
                    if let Some(out) = own.outputs.get_mut(0) {
                        out.kind = (rng.next() % 6) as u8;
                    }
                }
                4 => {
                    if let Some(out) = own.outputs.get_mut(0) {
                        out.from = Some(v98::PlugV98 {
                            node: rng.next() % 12,
                            plug: "out".into(),
                        });
                    }
                }
                5 => own.sliders.push(v98::SliderV98 {
                    key: ["s1", "s9", ""][(rng.next() % 3) as usize].into(),
                    name: "x".into(),
                    node: rng.next() % 12,
                    setting: ["low", "high", "nope"][(rng.next() % 3) as usize].into(),
                }),
                6 => {
                    let at = (rng.next() as usize) % own.inner.boxes.len().max(1);
                    if let Some(b) = own.inner.boxes.get_mut(at) {
                        b.kind =
                            [LUMPY, PEBBLES, "levels", "own:1"][(rng.next() % 4) as usize].into();
                    }
                }
                7 => own.inner.wires.clear(),
                8 => own.picture = Some((0..(rng.next() % 9000)).map(|i| i as u8).collect()),
                _ => own.inputs.clear(),
            }
        }
        if let Ok(doc) = v95::with_graphs(Document::new(), Some(s)) {
            opened += 1;
            for own in doc.scene().graphs().own_boxes() {
                assert!(own.check(doc.scene().graphs().registry()).is_ok());
            }
        }
    }
    assert!(opened > 0);
}

#[test]
fn flipped_bytes_in_a_file_with_own_boxes_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v98()).unwrap();
    let mut rng = Lcg(73);
    for _ in 0..300 {
        let mut bytes = full.clone();
        for _ in 0..(rng.next() % 3 + 1) {
            let at = (rng.next() as usize) % bytes.len();
            bytes[at] ^= 1 << (rng.next() % 8);
        }
        let _ = from_bytes(&bytes);
    }
}

const SAMPLE_V98: &[u8] = include_bytes!("../../tests/data/sample-v98.avtl");

#[test]
fn a_file_saved_by_format_version_98_opens_with_its_own_boxes() {
    let doc = from_bytes(SAMPLE_V98).expect("the version 98 sample opens");
    assert_eq!(doc.scene().graphs(), rich_document_v98().scene().graphs());
}

#[test]
fn a_version_97_file_opens_with_no_own_boxes() {
    const SAMPLE_V97: &[u8] = include_bytes!("../../tests/data/sample-v97.avtl");
    let doc = from_bytes(SAMPLE_V97).unwrap();
    assert_eq!(doc.scene().graphs().own_count(), 0);
    assert_eq!(doc.scene().graphs(), rich_document_v97().scene().graphs());
}
