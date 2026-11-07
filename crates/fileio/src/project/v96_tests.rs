//! Tests for version 96: version 95 with gradient settings in material
//! graphs (Gradient map's colours), and the first full set of boxes.

use super::v95_tests::rich_document_v95;
use super::*;
use glam::Vec2;
use matgraph::{
    BoxId, ColourGradient, GradientStop, Graph, GraphEdit, GraphId, PlugRef, Registry, SettingValue,
};
use scene::Command;

fn stop(at: f32, colour: [f32; 3]) -> GradientStop {
    GradientStop { at, colour }
}

/// A gradient of four colours, added out of order.
fn sunset() -> ColourGradient {
    ColourGradient::new(&[
        stop(0.0, [0.1, 0.0, 0.2]),
        stop(1.0, [1.0, 0.9, 0.5]),
        stop(0.35, [0.7, 0.2, 0.1]),
        stop(0.7, [0.95, 0.6, 0.2]),
    ])
    .unwrap()
}

/// A graph using new boxes: Bricks into Bevel (a length setting), colour
/// by a Gradient map with its own gradient, into a material.
fn coloured_bricks() -> Graph {
    let registry = Registry::standard();
    let mut g = Graph::new("Coloured bricks");
    let apply = |g: &mut Graph, edit: GraphEdit| g.apply(registry, &edit).unwrap();
    for (i, kind) in [
        "bricks",
        "bevel",
        "gradient_map",
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
                at: Vec2::new(i as f32 * 220.0, 0.0),
            },
        );
    }
    let p = |id: u64, plug: &str| PlugRef::new(BoxId(id), plug);
    for (from, to) in [
        (p(1, "shape"), p(2, "shape")),
        (p(2, "height"), p(3, "in")),
        (p(2, "height"), p(4, "height")),
        (p(3, "colour"), p(4, "colour")),
        (p(4, "material"), p(5, "material")),
    ] {
        apply(&mut g, GraphEdit::Connect { from, to });
    }
    for (node, key, value) in [
        (2, "width", SettingValue::Number(3.5)),
        (2, "edge", SettingValue::Choice(2)),
        (3, "colours", SettingValue::Gradient(sunset())),
        (5, "size_cm", SettingValue::Number(120.0)),
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
    g
}

/// Everything version 95 stores, with a third graph holding a gradient.
/// The version 96 sample holds exactly this.
pub(super) fn rich_document_v96() -> Document {
    let mut doc = rich_document_v95();
    doc.execute(Command::AddGraph(coloured_bricks())).unwrap();
    doc
}

/// The id the coloured bricks graph has in [`rich_document_v96`].
const BRICKS: GraphId = GraphId(4);

#[test]
fn gradients_survive_a_round_trip_in_their_order() {
    let doc = rich_document_v96();
    let bytes = to_bytes(&doc).unwrap();
    assert!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) >= 96);
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(back.scene().graphs(), doc.scene().graphs());
    let graph = back.scene().graphs().get(BRICKS).unwrap();
    assert_eq!(
        graph.get(BoxId(3)).unwrap().settings.get("colours"),
        Some(SettingValue::Gradient(sunset()))
    );
    // And the material it makes is the same.
    let size = matgraph::Size::new(32).unwrap();
    let make = |g: &Graph| {
        matgraph::Evaluator::with_threads(1)
            .material(Registry::standard(), g, size)
            .unwrap()
    };
    assert_eq!(*make(graph), *make(&coloured_bricks()));
}

/// The stored graphs of `doc`.
fn stored(doc: &Document) -> super::v95::GraphsV95 {
    super::v95::graphs_of(doc).unwrap()
}

/// The stored gradient setting of the coloured bricks graph.
fn gradient_setting(s: &mut super::v95::GraphsV95) -> &mut super::v96::GradientV96 {
    let graph = s.graphs.iter_mut().find(|g| g.id == BRICKS.0).unwrap();
    let gradient_map = graph.boxes.iter_mut().find(|b| b.id == 3).unwrap();
    let setting = gradient_map
        .settings
        .iter_mut()
        .find(|s| s.key == "colours")
        .unwrap();
    match &mut setting.value {
        super::v95::ValueV95::Gradient(g) => g,
        other => panic!("not a gradient: {other:?}"),
    }
}

#[test]
fn gradients_with_too_few_or_too_many_colours_are_refused() {
    let good = stored(&rich_document_v96());
    for count in [0, 1, matgraph::MAX_STOPS + 1, 200] {
        let mut s = good.clone();
        gradient_setting(&mut s).stops = (0..count)
            .map(|i| super::v96::StopV96 {
                at: i as f32 / 300.0,
                colour: [0.5; 3],
            })
            .collect();
        let why = super::v95::with_graphs(Document::new(), Some(s)).unwrap_err();
        assert!(why.contains("gradient"), "{why}");
    }
    assert!(super::v95::with_graphs(Document::new(), Some(good)).is_ok());
}

#[test]
fn gradient_colours_out_of_range_are_brought_into_range() {
    let mut s = stored(&rich_document_v96());
    let g = gradient_setting(&mut s);
    g.stops[0].at = -4.0;
    g.stops[1].colour = [2.0, -1.0, 0.5];
    let doc = super::v95::with_graphs(Document::new(), Some(s.clone())).unwrap();
    let graph = doc.scene().graphs().get(BRICKS).unwrap();
    let Some(SettingValue::Gradient(read)) = graph.get(BoxId(3)).unwrap().settings.get("colours")
    else {
        panic!("the gradient is read");
    };
    assert!(read.is_allowed());
    assert_eq!(read.stops()[0].at, 0.0);
    assert_eq!(read.stops()[1].colour, [1.0, 0.0, 0.5]);
    // A place that isn't a number: the standard gradient.
    gradient_setting(&mut s).stops[2].at = f32::NAN;
    let doc = super::v95::with_graphs(Document::new(), Some(s)).unwrap();
    let graph = doc.scene().graphs().get(BRICKS).unwrap();
    let standard = Registry::standard()
        .get("gradient_map")
        .unwrap()
        .info()
        .setting("colours")
        .unwrap()
        .standard();
    assert_eq!(
        graph.get(BoxId(3)).unwrap().settings.get("colours"),
        Some(standard)
    );
    // A gradient on a setting that isn't one: the setting's standard.
    let mut s = stored(&rich_document_v96());
    let graph = s.graphs.iter_mut().find(|g| g.id == BRICKS.0).unwrap();
    graph.boxes[0].settings.push(super::v95::SettingV95 {
        key: "rows".into(),
        value: super::v95::ValueV95::Gradient(super::v96::gradient_of(&sunset())),
    });
    let doc = super::v95::with_graphs(Document::new(), Some(s)).unwrap();
    let bricks = doc
        .scene()
        .graphs()
        .get(BRICKS)
        .unwrap()
        .get(BoxId(1))
        .unwrap()
        .clone();
    assert_eq!(bricks.settings.get("rows"), Some(SettingValue::Whole(8)));
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
            1 => f32::NEG_INFINITY,
            2 => 3e38,
            _ => (self.next() % 400) as f32 / 100.0 - 2.0,
        }
    }
}

#[test]
fn hostile_gradients_never_crash_the_reader() {
    // Fuzz: stored gradients with random numbers of colours, places and
    // colours that aren't real, put on any box's settings.
    let good = stored(&rich_document_v96());
    let mut rng = Lcg(5);
    let mut opened = 0;
    for _ in 0..2000 {
        let mut s = good.clone();
        let count = (rng.next() % 12) as usize;
        let stops: Vec<super::v96::StopV96> = (0..count)
            .map(|_| super::v96::StopV96 {
                at: rng.float(),
                colour: [rng.float(), rng.float(), rng.float()],
            })
            .collect();
        let graph = s.graphs.iter_mut().find(|g| g.id == BRICKS.0).unwrap();
        let at = (rng.next() as usize) % graph.boxes.len();
        let key = ["colours", "width", "rows", "nope"][(rng.next() % 4) as usize];
        graph.boxes[at].settings.push(super::v95::SettingV95 {
            key: key.into(),
            value: super::v95::ValueV95::Gradient(super::v96::GradientV96 {
                stops: stops.into_iter().collect(),
            }),
        });
        if let Ok(doc) = super::v95::with_graphs(Document::new(), Some(s)) {
            opened += 1;
            // What opened runs.
            let graph = doc.scene().graphs().get(BRICKS).unwrap().clone();
            let size = matgraph::Size::new(16).unwrap();
            matgraph::Evaluator::with_threads(1)
                .material(Registry::standard(), &graph, size)
                .unwrap();
        }
    }
    assert!(opened > 0);
}

#[test]
fn flipped_bytes_in_a_file_with_a_gradient_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v96()).unwrap();
    let mut rng = Lcg(23);
    for _ in 0..300 {
        let mut bytes = full.clone();
        for _ in 0..(rng.next() % 3 + 1) {
            let at = (rng.next() as usize) % bytes.len();
            bytes[at] ^= 1 << (rng.next() % 8);
        }
        let _ = from_bytes(&bytes);
    }
}

const SAMPLE_V96: &[u8] = include_bytes!("../../tests/data/sample-v96.avtl");

#[test]
fn a_file_saved_by_format_version_96_opens_with_its_gradient() {
    let doc = from_bytes(SAMPLE_V96).expect("the version 96 sample opens");
    assert_eq!(doc.scene().graphs().len(), 3);
    assert_eq!(
        **doc.scene().graphs().get(BRICKS).unwrap(),
        coloured_bricks()
    );
}

#[test]
fn a_version_95_file_still_opens_with_its_graphs() {
    const SAMPLE_V95: &[u8] = include_bytes!("../../tests/data/sample-v95.avtl");
    let doc = from_bytes(SAMPLE_V95).unwrap();
    assert_eq!(doc.scene().graphs().len(), 2);
    // Its Tiles box gained Random and a seed: the graph still runs.
    let graph = doc.scene().graphs().get(GraphId(1)).unwrap().clone();
    let size = matgraph::Size::new(16).unwrap();
    assert!(
        matgraph::Evaluator::with_threads(1)
            .material(Registry::standard(), &graph, size)
            .is_ok()
    );
}
