//! Tests for version 109: version 108 with the small pictures held by
//! each Stamps box (decision 0285).

use super::v108_tests::rich_document_v108;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, Stamp, Stamps};
use scene::Command;
use std::sync::Arc;

/// The graph given a Stamps box in [`rich_document_v109`].
const RIVETS: &str = "Rivets";

/// A small stamp `n` across of a `colour` dot with a soft see-through
/// rim, brought in as a PNG would be.
pub(crate) fn dot_stamp(n: u32, colour: [u8; 3], name: &str) -> Stamp {
    let c = n as f32 / 2.0;
    let image = image::RgbaImage::from_fn(n, n, |x, y| {
        let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
        let a = ((c * 0.8 - (dx * dx + dy * dy).sqrt()) * 100.0).clamp(0.0, 255.0) as u8;
        image::Rgba([colour[0], colour[1], colour[2], a])
    });
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    crate::image_file::read_stamp(&png, name).unwrap()
}

/// Two stamps, the second weighted 3.
pub(crate) fn two_stamps() -> Stamps {
    Stamps::one(dot_stamp(24, [180, 180, 190], "rivet.png"))
        .unwrap()
        .with_added(dot_stamp(16, [200, 120, 40], "bolt.png"))
        .unwrap()
        .with_weight(1, 3)
}

/// Everything version 108 stores, with a graph Rivets holding a Stamps
/// box with two stamps (the second weighted 3), in a grid, and a Stamps
/// box with nothing brought in yet. The version 109 sample holds exactly
/// this.
pub(crate) fn rich_document_v109() -> Document {
    let mut doc = rich_document_v108();
    let registry = doc.scene().graphs().registry().clone();
    let mut graph = Graph::new(RIVETS);
    for edit in [
        GraphEdit::AddBox {
            kind: "stamps".into(),
            at: glam::Vec2::ZERO,
        },
        GraphEdit::SetStamps {
            node: BoxId(1),
            stamps: Some(Arc::new(two_stamps())),
        },
        GraphEdit::SetSetting {
            node: BoxId(1),
            key: "arrange".into(),
            value: matgraph::SettingValue::Choice(1),
        },
        GraphEdit::AddBox {
            kind: "stamps".into(),
            at: glam::Vec2::new(0.0, 300.0),
        },
    ] {
        graph.apply(&registry, &edit).unwrap();
    }
    doc.execute(Command::AddGraph(graph)).unwrap();
    doc
}

fn rivets(doc: &Document) -> (GraphId, Arc<Graph>) {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == RIVETS)
        .map(|(id, g)| (id, g.clone()))
        .unwrap()
}

#[test]
fn stamps_survive_a_round_trip_exactly() {
    let doc = rich_document_v109();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let (_, graph) = rivets(&back);
    let stamps = graph.get(BoxId(1)).unwrap().stamps.clone().unwrap();
    assert_eq!(*stamps, two_stamps());
    assert_eq!(stamps.key(), two_stamps().key());
    assert_eq!(stamps.entries()[0].stamp.name(), "rivet.png");
    assert_eq!(stamps.entries()[1].weight, 3);
    // The see-through rim is kept exactly.
    let rgba = stamps.entries()[0].stamp.rgba();
    assert!(rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
    assert!(graph.get(BoxId(2)).unwrap().stamps.is_none());
    // Saved again, the same bytes: the stamps aren't packed again.
    assert_eq!(to_bytes(&back).unwrap(), bytes);
}

#[test]
fn nothing_new_is_stored_without_stamps() {
    let doc = rich_document_v108();
    for (id, graph) in doc.scene().graphs().iter() {
        let text = serde_json::to_string(&v95::GraphV95::of(id, graph)).unwrap();
        assert!(!text.contains("\"stamps\""), "{text}");
    }
}

#[test]
fn damaged_stamps_are_refused() {
    let good = v109::stamps_of(&two_stamps());
    // Not a picture at all, nothing, cut short.
    for packed in [vec![1, 2, 3, 4], Vec::new(), good[0].packed.0[..12].to_vec()] {
        let mut bad = good.clone();
        bad[0].packed = v107::PackedBytes(packed);
        assert!(v109::stamps_from(bad).is_err());
    }
    // An empty list, and one longer than a box holds.
    assert!(v109::stamps_from(Vec::new()).is_err());
    let long = vec![good[0].clone(); matgraph::MAX_STAMPS + 1];
    assert!(v109::stamps_from(long).is_err());
    // A weight out of range is brought into range.
    let mut heavy = good.clone();
    heavy[0].weight = 1000;
    let read = v109::stamps_from(heavy).unwrap();
    assert_eq!(read.entries()[0].weight, matgraph::stamp::MAX_WEIGHT);
    // Stamps on a box of another kind are dropped when read.
    let doc = rich_document_v109();
    let (id, graph) = rivets(&doc);
    let mut stored = v95::GraphV95::of(id, &graph);
    stored.boxes.0[0].kind = "noise".into();
    let (_, parts) = stored.into_parts().unwrap();
    let back = Graph::from_parts(parts, matgraph::Registry::standard()).unwrap();
    assert!(back.get(BoxId(1)).unwrap().stamps.is_none());
}

#[test]
fn flipped_bytes_in_a_file_with_stamps_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v109()).unwrap();
    let mut state = 109u64;
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
            let registry = doc.scene().graphs().registry().clone();
            for (_, graph) in doc.scene().graphs().iter() {
                let _ = matgraph::Evaluator::with_threads(1).run(
                    &registry,
                    graph,
                    matgraph::Size::new(16).unwrap(),
                    &[],
                );
            }
        }
    }
    // Flipped bytes inside a packed stamp itself.
    let good = v109::stamps_of(&two_stamps());
    for _ in 0..300 {
        let mut bad = good.clone();
        let at = (next() as usize) % bad[1].packed.0.len();
        bad[1].packed.0[at] ^= 1 << (next() % 8);
        if let Ok(s) = v109::stamps_from(bad) {
            assert!(s.len() <= matgraph::MAX_STAMPS);
            for e in s.entries() {
                assert!(e.stamp.width() <= matgraph::MAX_STAMP_SIDE);
            }
        }
    }
}

const SAMPLE_V109: &[u8] = include_bytes!("../../tests/data/sample-v109.avtl");

#[test]
fn a_file_saved_by_format_version_109_opens_with_its_stamps() {
    let doc = from_bytes(SAMPLE_V109).expect("the version 109 sample opens");
    let rich = rich_document_v109();
    assert_eq!(doc.scene().graphs(), rich.scene().graphs());
}

#[test]
fn a_version_108_file_opens_without_stamps() {
    const SAMPLE_V108: &[u8] = include_bytes!("../../tests/data/sample-v108.avtl");
    let doc = from_bytes(SAMPLE_V108).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(graph.boxes().all(|(_, b)| b.stamps.is_none()));
    }
}
