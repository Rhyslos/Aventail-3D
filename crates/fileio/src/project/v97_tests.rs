//! Tests for version 97: version 96 with graph fill layers and the
//! settings graphs show in the painter (decision 0261).

use super::v96_tests::rich_document_v96;
use super::*;
use matgraph::{BoxId, GraphEdit, GraphId, SettingValue};
use paint::{GraphFill, Layer};
use scene::Command;

/// The coloured bricks graph of version 96's sample.
const BRICKS: GraphId = GraphId(4);

/// Everything version 96 stores, with the bricks graph showing two
/// settings in the painter and a graph fill layer using it, with values
/// of its own. The version 97 sample holds exactly this.
pub(super) fn rich_document_v97() -> Document {
    let mut doc = rich_document_v96();
    for (node, key) in [(2, "width"), (5, "size_cm")] {
        doc.execute(Command::EditGraph {
            id: BRICKS,
            edit: GraphEdit::ShowInPainter {
                node: BoxId(node),
                key: key.into(),
                shown: true,
            },
        })
        .unwrap();
    }
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut image = paint::Image::new(set.size).unwrap();
    image.set_pixel(5, 5, [180, 90, 60, 255]);
    let mut bricks = Layer::paint("Bricks", image);
    let mut fill = GraphFill::of(BRICKS);
    fill.set_value(BoxId(2), "width", SettingValue::Number(6.0));
    fill.set_value(BoxId(5), "size_cm", SettingValue::Number(80.0));
    fill.made = 0x5eed;
    bricks.graph = Some(fill);
    set.layers.push(bricks);
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn a_graph_fill_and_shown_settings_survive_a_round_trip() {
    let doc = rich_document_v97();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let layer = back
        .scene()
        .paint()
        .set("Default")
        .unwrap()
        .layers
        .last()
        .cloned()
        .unwrap();
    let fill = layer.graph.unwrap();
    assert_eq!(fill.graph, BRICKS);
    assert_eq!(fill.made, 0x5eed);
    assert_eq!(
        fill.value(BoxId(2), "width"),
        Some(SettingValue::Number(6.0))
    );
    let graph = back.scene().graphs().get(BRICKS).unwrap();
    assert_eq!(graph.shown(matgraph::Registry::standard()).len(), 2);
}

#[test]
fn every_kind_of_value_survives() {
    let mut doc = rich_document_v97();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let fill = set.layers.last_mut().unwrap().graph.as_mut().unwrap();
    for (i, value) in [
        SettingValue::Number(0.25),
        SettingValue::Whole(-3),
        SettingValue::Colour([0.1, 0.2, 0.3]),
        SettingValue::Choice(2),
        SettingValue::Switch(true),
    ]
    .into_iter()
    .enumerate()
    {
        fill.set_value(BoxId(10 + i as u64), "any", value);
    }
    doc.execute(Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    // A gradient: its colours, in order (its unused room isn't kept).
    let gradient = matgraph::ColourGradient::two([0.1, 0.0, 0.0], [1.0, 0.9, 0.0]);
    let mut fill = GraphFill::of(BRICKS);
    fill.set_value(BoxId(3), "colours", SettingValue::Gradient(gradient));
    let back = v97::graph_fill_from(v97::graph_fill_of(&fill)).unwrap();
    let Some(SettingValue::Gradient(read)) = back.value(BoxId(3), "colours") else {
        panic!("the gradient is read")
    };
    assert_eq!(read.stops(), gradient.stops());
}

#[test]
fn damaged_graph_fills_are_refused() {
    let mut fill = GraphFill::of(BRICKS);
    fill.set_value(BoxId(1), "a", SettingValue::Number(1.0));
    let good = v97::graph_fill_of(&fill);
    assert_eq!(v97::graph_fill_from(good.clone()).unwrap(), fill);
    // A number that isn't real.
    let mut bad = good.clone();
    bad.values[0].value = v95::ValueV95::Number(f32::NAN);
    assert!(v97::graph_fill_from(bad).is_err());
    // A key far too long.
    let mut bad = good.clone();
    bad.values[0].key = "k".repeat(300);
    assert!(v97::graph_fill_from(bad).is_err());
    // Too many values.
    let mut bad = good.clone();
    bad.values = (0..=paint::graph_fill::MAX_VALUES as u64)
        .map(|i| v97::GraphValueV97 {
            node: i,
            key: "a".into(),
            value: v95::ValueV95::Whole(1),
        })
        .collect();
    assert!(v97::graph_fill_from(bad).is_err());
    // A damaged gradient.
    let mut bad = good;
    bad.values[0].value = v95::ValueV95::Gradient(v96::GradientV96 {
        stops: crate::limits::Bounded::new(),
    });
    assert!(v97::graph_fill_from(bad).is_err());
}

#[test]
fn a_graph_fill_whose_graph_is_gone_still_opens() {
    let mut doc = rich_document_v97();
    doc.execute(Command::SetGraph {
        id: BRICKS,
        graph: None,
    })
    .unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(doc.scene().paint(), back.scene().paint());
    assert!(back.scene().graphs().get(BRICKS).is_none());
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
fn hostile_graph_fills_never_crash_the_reader() {
    // Fuzz: stored graph fills with random ids, keys and values, and
    // shown keys on random boxes.
    let mut rng = Lcg(97);
    let mut opened = 0;
    for _ in 0..1500 {
        let count = (rng.next() % 6) as usize;
        let values = (0..count)
            .map(|_| v97::GraphValueV97 {
                node: rng.next() % 9,
                key: ["width", "size_cm", "", "nope", &"x".repeat(80)][(rng.next() % 5) as usize]
                    .to_string(),
                value: match rng.next() % 5 {
                    0 => v95::ValueV95::Number(
                        [f32::NAN, 1e30, -2.0, 0.5][(rng.next() % 4) as usize],
                    ),
                    1 => v95::ValueV95::Whole(rng.next() as i32),
                    2 => v95::ValueV95::Choice(rng.next() as u32),
                    3 => v95::ValueV95::Colour([f32::INFINITY, 0.3, -1.0]),
                    _ => v95::ValueV95::Switch(rng.next().is_multiple_of(2)),
                },
            })
            .collect();
        let stored = v97::GraphFillV97 {
            graph: rng.next() % 6,
            values,
            made: rng.next(),
            tiling: [1.0, 0.0, -3.0, 1e9, f32::NAN, 2.5][(rng.next() % 6) as usize],
        };
        if let Ok(fill) = v97::graph_fill_from(stored) {
            opened += 1;
            assert!(fill.is_valid());
        }
    }
    assert!(opened > 0);
    let mut graphs = v95::graphs_of(&rich_document_v97()).unwrap();
    for _ in 0..300 {
        let graph = &mut graphs.graphs[(rng.next() as usize) % 3];
        let at = (rng.next() as usize) % graph.boxes.len().max(1);
        if let Some(b) = graph.boxes.get_mut(at) {
            b.shown
                .push(["width", "rows", "", "size_cm"][(rng.next() % 4) as usize].into());
        }
        let _ = v95::with_graphs(Document::new(), Some(graphs.clone()));
    }
}

#[test]
fn flipped_bytes_in_a_file_with_a_graph_fill_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v97()).unwrap();
    let mut rng = Lcg(31);
    for _ in 0..300 {
        let mut bytes = full.clone();
        for _ in 0..(rng.next() % 3 + 1) {
            let at = (rng.next() as usize) % bytes.len();
            bytes[at] ^= 1 << (rng.next() % 8);
        }
        let _ = from_bytes(&bytes);
    }
}

const SAMPLE_V97: &[u8] = include_bytes!("../../tests/data/sample-v97.avtl");

#[test]
fn a_file_saved_by_format_version_97_opens_with_its_graph_fill() {
    let doc = from_bytes(SAMPLE_V97).expect("the version 97 sample opens");
    assert_eq!(doc.scene().paint(), rich_document_v97().scene().paint());
    assert_eq!(doc.scene().graphs(), rich_document_v97().scene().graphs());
}

#[test]
fn a_version_96_file_opens_with_no_graph_fills_or_shown_settings() {
    const SAMPLE_V96: &[u8] = include_bytes!("../../tests/data/sample-v96.avtl");
    let doc = from_bytes(SAMPLE_V96).unwrap();
    let registry = matgraph::Registry::standard();
    assert!(
        doc.scene()
            .graphs()
            .iter()
            .all(|(_, g)| g.shown(registry).is_empty())
    );
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(set.layers.iter().all(|l| l.graph.is_none()));
}
