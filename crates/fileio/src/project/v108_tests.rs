//! Tests for version 108: version 107 with the photo or painting brought
//! in on each Image box (decision 0283).

use super::v107_tests::rich_document_v107;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, Photo};
use scene::Command;
use std::sync::Arc;

/// The graph given an Image box in [`rich_document_v108`].
const FROM_A_PHOTO: &str = "From a photo";

/// A small photo of bark-like stripes, 96 × 64 pixels, brought in as a
/// PNG would be (packed as a JPEG).
pub(crate) fn bark_photo() -> Photo {
    let image = image::RgbImage::from_fn(96, 64, |x, y| {
        let stripe = ((x / 6) % 2) as u8 * 40;
        image::Rgb([90 + stripe, 60 + stripe + (y % 7) as u8, 40])
    });
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    crate::image_file::read_photo(&png, "bark.png").unwrap()
}

/// Everything version 107 stores, with a graph From a photo holding an
/// Image box with the bark photo, its square moved to the right, wired
/// into Make seamless, and an Image box with nothing brought in yet. The
/// version 108 sample holds exactly this.
pub(crate) fn rich_document_v108() -> Document {
    let mut doc = rich_document_v107();
    let registry = doc.scene().graphs().registry().clone();
    let mut graph = Graph::new(FROM_A_PHOTO);
    for edit in [
        GraphEdit::AddBox {
            kind: "image".into(),
            at: glam::Vec2::ZERO,
        },
        GraphEdit::SetPhoto {
            node: BoxId(1),
            photo: Some(Arc::new(bark_photo())),
        },
        GraphEdit::SetSetting {
            node: BoxId(1),
            key: "across".into(),
            value: matgraph::SettingValue::Number(1.0),
        },
        GraphEdit::AddBox {
            kind: "seamless".into(),
            at: glam::Vec2::new(300.0, 0.0),
        },
        GraphEdit::Connect {
            from: matgraph::PlugRef::new(BoxId(1), "colour"),
            to: matgraph::PlugRef::new(BoxId(2), "colour"),
        },
        GraphEdit::AddBox {
            kind: "image".into(),
            at: glam::Vec2::new(0.0, 300.0),
        },
    ] {
        graph.apply(&registry, &edit).unwrap();
    }
    doc.execute(Command::AddGraph(graph)).unwrap();
    doc
}

fn from_a_photo(doc: &Document) -> (GraphId, Arc<Graph>) {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == FROM_A_PHOTO)
        .map(|(id, g)| (id, g.clone()))
        .unwrap()
}

#[test]
fn a_photo_survives_a_round_trip_exactly() {
    let doc = rich_document_v108();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let (_, graph) = from_a_photo(&back);
    let photo = graph.get(BoxId(1)).unwrap().photo.clone().unwrap();
    assert_eq!(*photo, bark_photo());
    assert_eq!(photo.key(), bark_photo().key());
    assert_eq!(photo.name(), "bark.png");
    assert!(graph.get(BoxId(3)).unwrap().photo.is_none());
    // Saved again, the same bytes: the photo isn't packed again.
    assert_eq!(to_bytes(&back).unwrap(), bytes);
}

#[test]
fn nothing_new_is_stored_without_a_photo() {
    let doc = rich_document_v107();
    for (id, graph) in doc.scene().graphs().iter() {
        let text = serde_json::to_string(&v95::GraphV95::of(id, graph)).unwrap();
        assert!(!text.contains("\"photo\""), "{text}");
    }
}

#[test]
fn damaged_photos_are_refused() {
    let good = v108::photo_of(&bark_photo());
    // Not a picture at all, and nothing.
    for packed in [vec![1, 2, 3, 4], Vec::new()] {
        let bad = v108::PhotoV108 {
            packed: v107::PackedBytes(packed),
            ..good.clone()
        };
        assert!(v108::photo_from(bad).is_err());
    }
    // Cut short.
    let mut cut = good.clone();
    cut.packed.0.truncate(8);
    assert!(v108::photo_from(cut).is_err());
    // A photo on a box of another kind is dropped when read.
    let doc = rich_document_v108();
    let (id, graph) = from_a_photo(&doc);
    let mut stored = v95::GraphV95::of(id, &graph);
    stored.boxes.0[0].kind = "noise".into();
    stored.wires.0.clear();
    let (_, parts) = stored.into_parts().unwrap();
    let back = Graph::from_parts(parts, matgraph::Registry::standard()).unwrap();
    assert!(back.get(BoxId(1)).unwrap().photo.is_none());
}

#[test]
fn flipped_bytes_in_a_file_with_photos_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v108()).unwrap();
    let mut state = 108u64;
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
    // Flipped bytes inside the packed picture itself.
    let good = v108::photo_of(&bark_photo());
    for _ in 0..300 {
        let mut bad = good.clone();
        let at = (next() as usize) % bad.packed.0.len();
        bad.packed.0[at] ^= 1 << (next() % 8);
        if let Ok(p) = v108::photo_from(bad) {
            assert!(p.width() <= matgraph::MAX_PHOTO_SIDE);
        }
    }
}

const SAMPLE_V108: &[u8] = include_bytes!("../../tests/data/sample-v108.avtl");

#[test]
fn a_file_saved_by_format_version_108_opens_with_its_photos() {
    let doc = from_bytes(SAMPLE_V108).expect("the version 108 sample opens");
    let rich = rich_document_v108();
    assert_eq!(doc.scene().graphs(), rich.scene().graphs());
}

#[test]
fn a_version_107_file_opens_without_photos() {
    const SAMPLE_V107: &[u8] = include_bytes!("../../tests/data/sample-v107.avtl");
    let doc = from_bytes(SAMPLE_V107).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(graph.boxes().all(|(_, b)| b.photo.is_none()));
    }
}
