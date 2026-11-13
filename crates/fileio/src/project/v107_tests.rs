//! Tests for version 107: version 106 with what is painted on each
//! Painted box (decision 0282).

use super::v106_tests::rich_document_v106;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, Painting, SettingValue};
use scene::Command;
use std::sync::Arc;

/// The graph given Painted boxes in [`rich_document_v107`].
const HAND_PAINTED: &str = "Hand painted";

/// A 512-pixel painting: a soft green blob near the top left corner,
/// reaching over the edges, and a line across.
pub(crate) fn moss_painting() -> Painting {
    let n = 512usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let dx = (x as f32 - 10.0)
                .abs()
                .min(n as f32 - (x as f32 - 10.0).abs());
            let dy = (y as f32 - 12.0)
                .abs()
                .min(n as f32 - (y as f32 - 12.0).abs());
            let d = (dx * dx + dy * dy).sqrt();
            let at = (y * n + x) * 4;
            if d < 40.0 {
                let a = ((40.0 - d) / 40.0 * 255.0) as u8;
                rgba[at..at + 4].copy_from_slice(&[70, 140, 50, a]);
            } else if (300..304).contains(&y) {
                rgba[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    Painting::from_rgba(512, &rgba).unwrap()
}

/// Everything version 106 stores, with a graph Hand painted holding a
/// Painted mask (the moss painting) and a Painted colour over Clouds with
/// nothing painted yet. The version 107 sample holds exactly this.
pub(crate) fn rich_document_v107() -> Document {
    let mut doc = rich_document_v106();
    let registry = doc.scene().graphs().registry().clone();
    let mut graph = Graph::new(HAND_PAINTED);
    for edit in [
        GraphEdit::AddBox {
            kind: "painted".into(),
            at: glam::Vec2::ZERO,
        },
        GraphEdit::SetPainting {
            node: BoxId(1),
            painting: Some(Arc::new(moss_painting())),
        },
        GraphEdit::AddBox {
            kind: "painted".into(),
            at: glam::Vec2::new(0.0, 300.0),
        },
        GraphEdit::SetSetting {
            node: BoxId(2),
            key: "kind".into(),
            value: SettingValue::Choice(2),
        },
    ] {
        graph.apply(&registry, &edit).unwrap();
    }
    doc.execute(Command::AddGraph(graph)).unwrap();
    doc
}

fn hand_painted(doc: &Document) -> (GraphId, Arc<Graph>) {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == HAND_PAINTED)
        .map(|(id, g)| (id, g.clone()))
        .unwrap()
}

#[test]
fn a_painting_survives_a_round_trip() {
    let doc = rich_document_v107();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let (_, graph) = hand_painted(&back);
    let painting = graph.get(BoxId(1)).unwrap().painting.clone().unwrap();
    assert_eq!(*painting, moss_painting());
    assert_eq!(painting.key(), moss_painting().key());
    assert!(graph.get(BoxId(2)).unwrap().painting.is_none());
}

#[test]
fn nothing_new_is_stored_without_a_painting() {
    let doc = rich_document_v106();
    for (id, graph) in doc.scene().graphs().iter() {
        let text = serde_json::to_string(&v95::GraphV95::of(id, graph)).unwrap();
        assert!(!text.contains("painting"), "{text}");
    }
}

#[test]
fn a_painting_is_packed_small() {
    let stored = v107::painting_of(&moss_painting());
    assert!(
        stored.packed.0.len() < 64 * 1024,
        "{} bytes for a 512 painting",
        stored.packed.0.len()
    );
    // An empty 2048 painting packs to almost nothing.
    let empty = v107::painting_of(&Painting::new(2048).unwrap());
    assert!(empty.packed.0.len() < 4096, "{}", empty.packed.0.len());
    assert_eq!(
        v107::painting_from(empty).unwrap(),
        Painting::new(2048).unwrap()
    );
}

#[test]
fn damaged_paintings_are_refused() {
    let good = v107::painting_of(&moss_painting());
    // A size a painting can't be.
    for size in [0, 100, 4096, 1 << 20] {
        let bad = v107::PaintingV107 {
            size,
            ..good.clone()
        };
        assert!(v107::painting_from(bad).is_err(), "{size}");
    }
    // The pixels of another size: too few, too many.
    for size in [256, 1024] {
        let bad = v107::PaintingV107 {
            size,
            ..good.clone()
        };
        assert!(v107::painting_from(bad).is_err(), "{size}");
    }
    // Not packed at all.
    let bad = v107::PaintingV107 {
        size: 512,
        packed: v107::PackedBytes(vec![1, 2, 3, 4]),
    };
    assert!(v107::painting_from(bad).is_err());
    // A painting on a box of another kind is dropped when read.
    let doc = rich_document_v107();
    let (id, graph) = hand_painted(&doc);
    let mut stored = v95::GraphV95::of(id, &graph);
    stored.boxes.0[0].kind = "noise".into();
    let (_, parts) = stored.into_parts().unwrap();
    let back = Graph::from_parts(parts, matgraph::Registry::standard()).unwrap();
    assert!(back.get(BoxId(1)).unwrap().painting.is_none());
}

#[test]
fn flipped_bytes_in_a_file_with_paintings_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v107()).unwrap();
    let mut state = 107u64;
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
                let _ = matgraph::Evaluator::with_threads(1).run(
                    &registry,
                    graph,
                    matgraph::Size::new(16).unwrap(),
                    &[],
                );
            }
        }
    }
    // Flipped bytes inside the packed pixels themselves.
    let good = v107::painting_of(&moss_painting());
    for _ in 0..200 {
        let mut bad = good.clone();
        let at = (next() as usize) % bad.packed.0.len();
        bad.packed.0[at] ^= 1 << (next() % 8);
        let _ = v107::painting_from(bad);
    }
}

const SAMPLE_V107: &[u8] = include_bytes!("../../tests/data/sample-v107.avtl");

#[test]
fn a_file_saved_by_format_version_107_opens_with_its_paintings() {
    let doc = from_bytes(SAMPLE_V107).expect("the version 107 sample opens");
    let rich = rich_document_v107();
    assert_eq!(doc.scene().graphs(), rich.scene().graphs());
}

#[test]
fn a_version_106_file_opens_without_paintings() {
    const SAMPLE_V106: &[u8] = include_bytes!("../../tests/data/sample-v106.avtl");
    let doc = from_bytes(SAMPLE_V106).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(graph.boxes().all(|(_, b)| b.painting.is_none()));
    }
}
