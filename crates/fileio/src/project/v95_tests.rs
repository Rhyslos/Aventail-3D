//! Tests for version 95: version 94 with the project's material graphs.

use super::v94_tests::rich_document_v94;
use super::*;
use glam::Vec2;
use matgraph::{BoxId, Frame, Graph, GraphEdit, GraphId, PlugRef, Registry, SettingValue};
use scene::Command;

/// A graph with every part a file stores: boxes of each kind with
/// changed settings of each kind, wires (one converting a shape to a
/// height), a nested comment frame with a note, a seed.
fn stone_tiles() -> Graph {
    let registry = Registry::standard();
    let mut g = Graph::new("Stone tiles");
    let apply = |g: &mut Graph, edit: GraphEdit| g.apply(registry, &edit).unwrap();
    for (i, kind) in [
        "tiles",
        "noise",
        "blend",
        "levels",
        "colour_fill",
        "material_from_height",
        "material_out",
    ]
    .into_iter()
    .enumerate()
    {
        apply(
            &mut g,
            GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(i as f32 * 220.0, 40.0 * (i % 2) as f32),
            },
        );
    }
    let p = |id: u64, plug: &str| PlugRef::new(BoxId(id), plug);
    for (from, to) in [
        (p(1, "shape"), p(3, "bottom")),
        (p(2, "height"), p(3, "top")),
        (p(1, "shape"), p(3, "where")),
        (p(3, "out"), p(4, "in")),
        (p(4, "out"), p(6, "height")),
        (p(5, "colour"), p(6, "colour")),
        (p(6, "material"), p(7, "material")),
    ] {
        apply(&mut g, GraphEdit::Connect { from, to });
    }
    for (node, key, value) in [
        (1, "shift", SettingValue::Number(0.5)),
        (1, "down", SettingValue::Whole(6)),
        (2, "seed", SettingValue::Whole(-3)),
        (3, "mode", SettingValue::Choice(4)),
        (4, "invert", SettingValue::Switch(true)),
        (5, "colour", SettingValue::Colour([0.7, 0.4, 0.2])),
        (7, "size_cm", SettingValue::Number(250.0)),
    ] {
        apply(
            &mut g,
            GraphEdit::SetSetting {
                node: BoxId(node),
                key: key.into(),
                value,
            },
        );
    }
    let mut outer = Frame::new("Shapes", Vec2::new(-20.0, -20.0), Vec2::new(700.0, 300.0));
    outer.note = "Big shapes first: tiles, then noise on top.".into();
    outer.colour = [140, 90, 60];
    apply(&mut g, GraphEdit::AddFrame(outer));
    apply(
        &mut g,
        GraphEdit::AddFrame(Frame::new(
            "Grout",
            Vec2::new(-10.0, -10.0),
            Vec2::new(100.0, 100.0),
        )),
    );
    apply(&mut g, GraphEdit::SetSeed(77));
    g
}

/// Everything version 94 stores, with two material graphs (and a third
/// added and deleted, so the next id is past every graph's). The version
/// 95 sample holds exactly this.
pub(super) fn rich_document_v95() -> Document {
    let mut doc = rich_document_v94();
    doc.execute(Command::AddGraph(stone_tiles())).unwrap();
    doc.execute(Command::AddGraph(Graph::new("Gone"))).unwrap();
    doc.execute(Command::AddGraph(Graph::new("Empty"))).unwrap();
    doc.execute(Command::SetGraph {
        id: GraphId(2),
        graph: None,
    })
    .unwrap();
    doc
}

#[test]
fn graphs_survive_a_round_trip() {
    let doc = rich_document_v95();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().graphs(), doc.scene().graphs());
    assert_eq!(back.scene().graphs().len(), 2);
    assert_eq!(back.scene().graphs().next_id(), GraphId(4));
    assert_eq!(
        **back.scene().graphs().get(GraphId(1)).unwrap(),
        stone_tiles()
    );
    // A project without graphs stores none.
    let plain = rich_document_v94();
    let back = from_bytes(&to_bytes(&plain).unwrap()).unwrap();
    assert!(back.scene().graphs().is_empty());
}

#[test]
fn a_reopened_graph_makes_the_same_material() {
    let doc = rich_document_v95();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    let size = matgraph::Size::new(32).unwrap();
    let make = |d: &Document| {
        let graph = d.scene().graphs().get(GraphId(1)).unwrap().clone();
        matgraph::Evaluator::with_threads(1)
            .material(Registry::standard(), &graph, size)
            .unwrap()
    };
    assert_eq!(*make(&back), *make(&doc));
}

/// The stored graphs of `doc`.
fn stored(doc: &Document) -> super::v95::GraphsV95 {
    super::v95::graphs_of(doc).unwrap()
}

#[test]
fn damaged_graphs_are_refused() {
    let good = stored(&rich_document_v95());
    let mut cases = Vec::new();
    let mut s = good.clone();
    s.graphs[1].id = s.graphs[0].id;
    cases.push(s);
    let mut s = good.clone();
    s.graphs[0].id = 0;
    cases.push(s);
    let mut s = good.clone();
    s.graphs[0].wires[0].to = 999;
    cases.push(s);
    let mut s = good.clone();
    s.graphs[0].boxes[0].at = [f32::NAN, 0.0];
    cases.push(s);
    let mut s = good.clone();
    s.graphs[0].name = "x".repeat(matgraph::MAX_NAME + 1);
    cases.push(s);
    let mut s = good.clone();
    s.graphs[0].frames[0].note = "x".repeat(matgraph::MAX_NOTE + 1);
    cases.push(s);
    let mut s = good.clone();
    let wire = s.graphs[0].wires[0].clone();
    s.graphs[0].wires.push(super::v95::WireV95 {
        from: wire.to,
        from_plug: "out".into(),
        to: wire.from,
        to_plug: "in".into(),
    });
    cases.push(s);
    for (i, case) in cases.into_iter().enumerate() {
        assert!(
            super::v95::with_graphs(Document::new(), Some(case)).is_err(),
            "case {i}"
        );
    }
    assert!(super::v95::with_graphs(Document::new(), Some(good)).is_ok());
}

#[test]
fn settings_out_of_range_in_a_file_are_kept_in_range() {
    let mut s = stored(&rich_document_v95());
    let tiles = &mut s.graphs[0].boxes[0];
    tiles.settings.push(super::v95::SettingV95 {
        key: "across".into(),
        value: super::v95::ValueV95::Whole(1_000_000),
    });
    tiles.settings.push(super::v95::SettingV95 {
        key: "gap".into(),
        value: super::v95::ValueV95::Switch(true),
    });
    let doc = super::v95::with_graphs(Document::new(), Some(s)).unwrap();
    let graph = doc.scene().graphs().get(GraphId(1)).unwrap();
    let tiles = graph.get(BoxId(1)).unwrap();
    assert_eq!(tiles.settings.get("across"), Some(SettingValue::Whole(64)));
    assert_eq!(tiles.settings.get("gap"), Some(SettingValue::Number(0.05)));
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

    fn float(&mut self) -> f32 {
        match self.next() % 6 {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => -3e38,
            _ => (self.next() % 4000) as f32 - 2000.0,
        }
    }
}

#[test]
fn hostile_graphs_never_crash_the_reader() {
    // Fuzz: stored graphs with random, often nonsensical values (ids
    // pointing nowhere or repeated, plugs that don't exist, numbers that
    // aren't real, wrong kinds of value). Each opens or is refused; none
    // crashes.
    let good = stored(&rich_document_v95());
    let mut rng = Lcg(3);
    let mut opened = 0;
    for _ in 0..3000 {
        let mut s = good.clone();
        let g = &mut s.graphs[(rng.next() % 2) as usize];
        for _ in 0..(rng.next() % 4 + 1) {
            match rng.next() % 8 {
                0 if !g.boxes.is_empty() => {
                    let at = (rng.next() as usize) % g.boxes.len();
                    g.boxes[at].id = rng.next() % 12;
                }
                1 if !g.wires.is_empty() => {
                    let at = (rng.next() as usize) % g.wires.len();
                    g.wires[at].from = rng.next() % 12;
                    g.wires[at].to_plug = ["in", "top", "x", ""][(rng.next() % 4) as usize].into();
                }
                2 if !g.boxes.is_empty() => {
                    let at = (rng.next() as usize) % g.boxes.len();
                    g.boxes[at].at = [rng.float(), rng.float()];
                    g.boxes[at].kind =
                        ["noise", "levels", "nope", ""][(rng.next() % 4) as usize].into();
                }
                3 if !g.frames.is_empty() => {
                    let at = (rng.next() as usize) % g.frames.len();
                    g.frames[at].min = [rng.float(), rng.float()];
                    g.frames[at].id = rng.next() % 12;
                }
                4 => g.next_id = rng.next() % 12,
                5 if !g.boxes.is_empty() => {
                    let at = (rng.next() as usize) % g.boxes.len();
                    let value = match rng.next() % 3 {
                        0 => super::v95::ValueV95::Number(rng.float()),
                        1 => super::v95::ValueV95::Choice(rng.next() as u32),
                        _ => super::v95::ValueV95::Whole(rng.next() as i32),
                    };
                    let key = ["mode", "scale", "low", "seed"][(rng.next() % 4) as usize].into();
                    g.boxes[at]
                        .settings
                        .push(super::v95::SettingV95 { key, value });
                }
                6 => s.next_id = rng.next(),
                _ => g.seed = rng.next() as u32,
            }
        }
        if super::v95::with_graphs(Document::new(), Some(s)).is_ok() {
            opened += 1;
        }
    }
    assert!(opened > 0, "some harmless changes still open");
}

#[test]
fn flipped_bytes_in_a_file_with_graphs_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v95()).unwrap();
    let mut rng = Lcg(17);
    for _ in 0..300 {
        let mut bytes = full.clone();
        for _ in 0..(rng.next() % 3 + 1) {
            let at = (rng.next() as usize) % bytes.len();
            bytes[at] ^= 1 << (rng.next() % 8);
        }
        let _ = from_bytes(&bytes);
    }
}

const SAMPLE_V95: &[u8] = include_bytes!("../../tests/data/sample-v95.avtl");

#[test]
fn a_file_saved_by_format_version_95_opens_with_its_graphs() {
    let doc = from_bytes(SAMPLE_V95).expect("the version 95 sample opens");
    assert_eq!(doc.scene().graphs().len(), 2);
    assert_eq!(
        **doc.scene().graphs().get(GraphId(1)).unwrap(),
        stone_tiles()
    );
}

#[test]
fn a_version_94_file_holds_no_graphs() {
    const SAMPLE_V94: &[u8] = include_bytes!("../../tests/data/sample-v94.avtl");
    let doc = from_bytes(SAMPLE_V94).unwrap();
    assert!(doc.scene().graphs().is_empty());
    assert_eq!(doc.scene().graphs().next_id(), GraphId(1));
}
