//! Tests for version 66: version 65 with fill layers, groups, clipping
//! and every blend mode (decision 0181).

use super::tests::assert_same;
use super::v65_tests::rich_document_v65;
use super::*;
use paint::{Blend, Layer};

/// Everything version 65 stores, with a clipped Multiply fill over the
/// paint and a closed group holding a Hue fill at half opacity. The
/// version 66 sample holds exactly this.
pub(super) fn rich_document_v66() -> Document {
    let mut doc = rich_document_v65();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    let mut shade = Layer::fill("Shade", [90, 80, 70, 255]);
    shade.blend = Blend::Multiply;
    shade.clipped = true;
    set.layers.push(shade);
    let mut tint = Layer::fill("Tint", [200, 20, 20, 255]);
    tint.blend = Blend::Hue;
    let mut group = Layer::group("Group", vec![tint]);
    group.opacity = 0.5;
    group.expanded = false;
    set.layers.push(group);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn fills_groups_clipping_and_blend_modes_survive_a_round_trip() {
    let doc = rich_document_v66();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(
        doc.scene().paint().set("Default"),
        back.scene().paint().set("Default")
    );
}

const SAMPLE_V66: &[u8] = include_bytes!("../../tests/data/sample-v66.avtl");

#[test]
fn a_file_saved_by_format_version_66_opens_with_its_layers() {
    let doc = from_bytes(SAMPLE_V66).expect("the version 66 sample opens");
    assert_same(&rich_document_v66(), &doc);
    assert_eq!(
        rich_document_v66().scene().paint().set("Default"),
        doc.scene().paint().set("Default")
    );
}

#[test]
fn a_version_65_file_opens_with_plain_paint_layers() {
    const SAMPLE_V65: &[u8] = include_bytes!("../../tests/data/sample-v65.avtl");
    let doc = from_bytes(SAMPLE_V65).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(
        set.layers
            .iter()
            .all(|l| l.image().is_some() && !l.clipped && l.blend == Blend::Normal)
    );
}

#[test]
fn damaged_layers_are_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v66());
    let with = |change: &dyn Fn(&mut Vec<super::v64::LayerV64>)| {
        let mut model = good.clone();
        if let Some(p) = model.paint.as_mut() {
            let mut layers: Vec<_> = p.sets[0].layers.iter().cloned().collect();
            change(&mut layers);
            p.sets[0].layers = layers.into_iter().collect();
        }
        model.into_document()
    };
    let last = |l: &mut Vec<super::v64::LayerV64>| l.len() - 1;
    // An unknown blend mode, from a newer version.
    assert!(with(&|l| l[0].blend = 8).is_err());
    // A layer that is a fill and a group at once.
    assert!(
        with(&|l| {
            let i = last(l);
            l[i].fill = Some([0; 4]);
        })
        .is_err()
    );
    // A fill with painted tiles.
    assert!(
        with(&|l| {
            let tiles = l[0].tiles.clone();
            let i = last(l) - 1;
            l[i].tiles = tiles;
        })
        .is_err()
    );
    // Groups nested past the limit.
    assert!(
        with(&|l| {
            let mut layer = l[0].clone();
            for _ in 0..paint::set::MAX_DEPTH + 1 {
                let i = last(l);
                let mut group = l[i].clone();
                group.children = Some(std::iter::once(layer).collect());
                layer = group;
            }
            l.push(layer);
        })
        .is_err()
    );
    assert!(with(&|_| ()).is_ok());
}
