//! Tests for version 100: version 99 with what is drawn on Shape boxes'
//! canvases (decision 0273).

use super::v99_tests::rich_document_v99;
use super::*;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, PlugRef};
use scene::Command;
use shapes::{Basic, Drawing, Knot, Mirror, Path, PieceKind, Reference};
use std::sync::Arc;

/// A leaf drawn along the mirror line, with a bite cut out, a vein, and a
/// small reference image under it.
pub(crate) fn leaf_drawing() -> Drawing {
    let half = Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.1)),
            Knot::smooth(Vec2::new(0.25, 0.5), Vec2::new(0.0, 0.2)),
            Knot::corner(Vec2::new(0.5, 0.9)),
        ],
        true,
    );
    let vein = Path::new(
        vec![
            Knot::corner(Vec2::new(0.5, 0.2)),
            Knot::corner(Vec2::new(0.5, 0.85)),
        ],
        false,
    );
    let r = Reference::new(2, 2, (0..16).collect(), "leaf photo.png")
        .unwrap()
        .with_opacity(0.3);
    Drawing::new()
        .with_piece("Leaf", half, PieceKind::Add)
        .unwrap()
        .with_piece(
            "Bite",
            Basic::Circle.path(Vec2::new(0.3, 0.45), 0.06),
            PieceKind::Cut,
        )
        .unwrap()
        .with_piece("Vein", vein, PieceKind::Line)
        .unwrap()
        .with_mirror(Mirror { on: true, x: 0.5 })
        .unwrap()
        .with_reference(Some(Arc::new(r)))
}

/// Everything version 99 stores, with a graph "Leaf": a Shape box with
/// [`leaf_drawing`] feeding Material from height and Material out. The
/// version 100 sample holds exactly this.
pub(crate) fn rich_document_v100() -> Document {
    let mut doc = rich_document_v99();
    let id = doc.scene().graphs().next_id();
    doc.execute(Command::AddGraph(Graph::new("Leaf"))).unwrap();
    for (kind, x) in [
        ("drawn_shape", 0.0),
        ("material_from_height", 300.0),
        ("material_out", 600.0),
    ] {
        doc.execute(Command::EditGraph {
            id,
            edit: GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(x, 0.0),
            },
        })
        .unwrap();
    }
    let edits = [
        GraphEdit::SetDrawing {
            node: BoxId(1),
            drawing: Arc::new(leaf_drawing()),
        },
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(1), "height"),
            to: PlugRef::new(BoxId(2), "height"),
        },
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(2), "material"),
            to: PlugRef::new(BoxId(3), "material"),
        },
    ];
    for edit in edits {
        doc.execute(Command::EditGraph { id, edit }).unwrap();
    }
    doc
}

fn leaf_of(doc: &Document) -> Arc<Drawing> {
    let (_, graph) = doc
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Leaf")
        .unwrap();
    graph.get(BoxId(1)).unwrap().drawing.clone().unwrap()
}

#[test]
fn drawn_shapes_survive_a_round_trip() {
    let doc = rich_document_v100();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let drawing = leaf_of(&back);
    assert_eq!(*drawing, leaf_drawing());
    let reference = drawing.reference().unwrap();
    assert_eq!(reference.opacity, 0.3);
    assert_eq!(reference.rgba(), (0..16).collect::<Vec<u8>>().as_slice());
    // It still works out.
    let (_, graph) = back
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Leaf")
        .unwrap();
    let registry = back.scene().graphs().registry();
    let material =
        matgraph::Evaluator::with_threads(1).material(registry, graph, matgraph::Size::nearest(32));
    assert!(material.is_ok(), "{material:?}");
}

#[test]
fn damaged_drawings_are_refused_with_a_reason() {
    let good = v100::drawing_of(&leaf_drawing());
    assert_eq!(v100::drawing_from(good.clone()).unwrap(), leaf_drawing());
    let mut kind = good.clone();
    kind.pieces.0[0].kind = 9;
    assert!(
        v100::drawing_from(kind)
            .unwrap_err()
            .contains("drawn shape")
    );
    let mut open = good.clone();
    open.pieces.0[0].closed = false;
    assert!(v100::drawing_from(open).is_err());
    let mut pixels = good.clone();
    pixels.reference.as_mut().unwrap().rgba.0.pop();
    assert!(v100::drawing_from(pixels).is_err());
    let mut far = good.clone();
    far.pieces.0[0].knots.0[0][0] = f32::INFINITY;
    assert!(v100::drawing_from(far).is_err());
    let mut ids = good.clone();
    ids.next_id = 1;
    assert!(v100::drawing_from(ids).is_err());
    // Opacity and the mirror line are brought into range, not refused.
    let mut wild = good;
    wild.reference.as_mut().unwrap().opacity = 7.0;
    wild.mirror_x = -3.0;
    let fixed = v100::drawing_from(wild).unwrap();
    assert_eq!(fixed.reference().unwrap().opacity, 1.0);
    assert_eq!(fixed.mirror().x, 0.0);
}

#[test]
fn flipped_bytes_in_a_file_with_drawings_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v100()).unwrap();
    let mut state = 100u64;
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
                for (_, b) in graph.boxes() {
                    if let Some(d) = &b.drawing {
                        assert_eq!(Drawing::from_parts(d.to_parts()).as_ref(), Ok(&**d));
                    }
                }
            }
        }
    }
}

const SAMPLE_V100: &[u8] = include_bytes!("../../tests/data/sample-v100.avtl");

#[test]
fn a_file_saved_by_format_version_100_opens_with_its_drawings() {
    let doc = from_bytes(SAMPLE_V100).expect("the version 100 sample opens");
    assert_eq!(doc.scene().graphs(), rich_document_v100().scene().graphs());
    assert_eq!(*leaf_of(&doc), leaf_drawing());
}

#[test]
fn a_version_99_file_opens_with_no_drawings() {
    const SAMPLE_V99: &[u8] = include_bytes!("../../tests/data/sample-v99.avtl");
    let doc = from_bytes(SAMPLE_V99).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(graph.boxes().all(|(_, b)| b.drawing.is_none()));
    }
    assert_eq!(doc.scene().graphs(), rich_document_v99().scene().graphs());
}
