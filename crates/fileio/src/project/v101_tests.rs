//! Tests for version 101: version 100 with assemblies on the shape canvas
//! (decision 0274): each piece's place, colour, height, link and group.

use super::v100_tests::{leaf_drawing, rich_document_v100};
use super::*;
use glam::Vec2;
use matgraph::{BoxId, GraphEdit};
use scene::Command;
use shapes::{Change, Drawing, PieceId};
use std::sync::Arc;

/// Version 100's leaf, built into a twig: the leaf coloured green, two
/// linked copies of it (one turned, one flipped, lower and darker), the
/// leaf and its first copy in a group, and a plain copy of the bite.
pub(crate) fn twig_drawing() -> Drawing {
    let leaf = leaf_drawing();
    let first = leaf.pieces()[0].id;
    let bite = leaf.pieces()[1].id;
    let d = leaf.with_colour(first, [0.2, 0.6, 0.1]).unwrap();
    let copies = d.copied(&[first]);
    let (d, a) = d
        .with_pasted(&copies, true, &Change::turned(Vec2::splat(0.5), 0.4))
        .unwrap();
    let (d, b) = d
        .with_pasted(&copies, true, &Change::moved(Vec2::new(0.1, 0.05)))
        .unwrap();
    let d = d
        .flipped(&[b[0]], true)
        .unwrap()
        .with_height(b[0], 0.6)
        .unwrap()
        .with_colour(b[0], [0.1, 0.3, 0.05])
        .unwrap();
    let (d, _) = d
        .with_pasted(
            &d.copied(&[bite]),
            false,
            &Change::moved(Vec2::new(0.0, 0.1)),
        )
        .unwrap();
    let (d, _) = d.with_group(&[first, a[0]], "Twig").unwrap();
    d
}

/// Everything version 100 stores, with the Leaf graph's Shape box holding
/// [`twig_drawing`]. The version 101 sample holds exactly this.
pub(crate) fn rich_document_v101() -> Document {
    let mut doc = rich_document_v100();
    let id = doc
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Leaf")
        .map(|(id, _)| id)
        .unwrap();
    doc.execute(Command::EditGraph {
        id,
        edit: GraphEdit::SetDrawing {
            node: BoxId(1),
            drawing: Arc::new(twig_drawing()),
        },
    })
    .unwrap();
    doc
}

fn twig_of(doc: &Document) -> Arc<Drawing> {
    let (_, graph) = doc
        .scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == "Leaf")
        .unwrap();
    graph.get(BoxId(1)).unwrap().drawing.clone().unwrap()
}

#[test]
fn assemblies_survive_a_round_trip() {
    let doc = rich_document_v101();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], 101u32.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    let twig = twig_of(&back);
    assert_eq!(*twig, twig_drawing());
    let first = twig.pieces().iter().find(|p| p.id == PieceId(1)).unwrap();
    assert_eq!(twig.linked_count(first.id), 3);
    assert_eq!(twig.groups().len(), 1);
    assert!(twig.pieces().iter().any(|p| p.place.flips()));
    // Its linked copies still change together after opening.
    let placed = first.placed();
    let moved = placed.with_knot(0, shapes::Knot::corner(Vec2::new(0.5, 0.05)));
    let changed = twig.with_path(first.id, moved).unwrap();
    let shapes_now: Vec<_> = changed
        .pieces()
        .iter()
        .filter(|p| p.link == first.link)
        .map(|p| p.path.clone())
        .collect();
    assert!(shapes_now.windows(2).all(|w| w[0] == w[1]));
    assert_ne!(shapes_now[0], first.path);
}

#[test]
fn damaged_assemblies_are_refused_with_a_reason() {
    let good = v100::drawing_of(&twig_drawing());
    assert_eq!(v100::drawing_from(good.clone()).unwrap(), twig_drawing());
    // A place squashing a piece flat.
    let mut flat = good.clone();
    flat.pieces.0[0].place = Some([0.0; 6]);
    assert!(
        v100::drawing_from(flat)
            .unwrap_err()
            .contains("drawn shape")
    );
    let mut wild = good.clone();
    wild.pieces.0[0].place = Some([f32::NAN, 0.0, 0.0, 1.0, 0.0, 0.0]);
    assert!(v100::drawing_from(wild).is_err());
    // A link to a piece with another shape.
    let mut mixed = good.clone();
    let bite = mixed
        .pieces
        .0
        .iter()
        .position(|p| p.name == "Bite")
        .unwrap();
    mixed.pieces.0[bite].link = Some(1);
    assert!(v100::drawing_from(mixed).is_err());
    // A group that isn't there, or with its pieces apart.
    let mut missing = good.clone();
    missing.groups.0.clear();
    assert!(v100::drawing_from(missing).is_err());
    let mut apart = good.clone();
    let grouped = apart
        .pieces
        .0
        .iter()
        .position(|p| p.group.is_some())
        .unwrap();
    let piece = apart.pieces.0.remove(grouped);
    apart.pieces.0.push(piece);
    assert!(v100::drawing_from(apart).is_err());
    // Colours and heights out of range are brought in, not refused.
    let mut bright = good;
    bright.pieces.0[0].colour = [4.0, -2.0, 0.5];
    bright.pieces.0[0].height = 3.0;
    let fixed = v100::drawing_from(bright).unwrap();
    assert_eq!(fixed.pieces()[0].colour, [1.0, 0.0, 0.5]);
    assert_eq!(fixed.pieces()[0].height, 1.0);
}

#[test]
fn flipped_bytes_in_a_file_with_assemblies_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v101()).unwrap();
    let mut state = 101u64;
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

const SAMPLE_V101: &[u8] = include_bytes!("../../tests/data/sample-v101.avtl");

#[test]
fn a_file_saved_by_format_version_101_opens_with_its_assemblies() {
    let doc = from_bytes(SAMPLE_V101).expect("the version 101 sample opens");
    assert_eq!(doc.scene().graphs(), rich_document_v101().scene().graphs());
    assert_eq!(*twig_of(&doc), twig_drawing());
}

#[test]
fn a_version_100_file_opens_with_pieces_where_they_were_drawn() {
    const SAMPLE_V100: &[u8] = include_bytes!("../../tests/data/sample-v100.avtl");
    let doc = from_bytes(SAMPLE_V100).unwrap();
    let leaf = twig_of(&doc);
    assert_eq!(*leaf, leaf_drawing());
    for p in leaf.pieces() {
        assert_eq!(p.place, Change::IDENTITY);
        assert_eq!(p.colour, [1.0; 3]);
        assert_eq!(p.height, 1.0);
        assert_eq!(leaf.linked_count(p.id), 1);
        assert_eq!(p.group, None);
    }
    assert!(leaf.groups().is_empty());
}
