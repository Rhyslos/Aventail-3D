//! Tests for version 106: version 105 where a graph's setting may name
//! one of the project's palettes (decision 0281).

use super::v105_tests::rich_document_v105;
use super::*;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, SettingValue};
use paint::palette::Palette;
use scene::Command;
use std::sync::Arc;

/// The graph given a Palette map in [`rich_document_v106`].
const PAINTED: &str = "Painted";

/// Everything version 105 stores, with two palettes (Game and Moss) and
/// a graph Painted holding a Palette map set to Moss in Smooth, and a
/// second one on the first palette. The version 106 sample holds exactly
/// this.
pub(crate) fn rich_document_v106() -> Document {
    let mut doc = rich_document_v105();
    doc.execute(Command::SetPalettes(vec![
        Palette {
            name: "Game".into(),
            colours: vec![[230, 220, 200], [60, 50, 40], [150, 120, 90]],
        },
        Palette {
            name: "Moss".into(),
            colours: vec![[40, 90, 30], [120, 170, 60]],
        },
    ]))
    .unwrap();
    let registry = doc.scene().graphs().registry().clone();
    let mut graph = Graph::new(PAINTED);
    for edit in [
        GraphEdit::AddBox {
            kind: "noise".into(),
            at: glam::Vec2::ZERO,
        },
        GraphEdit::AddBox {
            kind: "palette_map".into(),
            at: glam::Vec2::new(300.0, 0.0),
        },
        GraphEdit::AddBox {
            kind: "palette_map".into(),
            at: glam::Vec2::new(300.0, 300.0),
        },
        GraphEdit::Connect {
            from: matgraph::PlugRef::new(BoxId(1), "height"),
            to: matgraph::PlugRef::new(BoxId(2), "in"),
        },
        GraphEdit::SetSetting {
            node: BoxId(2),
            key: "palette".into(),
            value: SettingValue::Palette("Moss".into()),
        },
        GraphEdit::SetSetting {
            node: BoxId(2),
            key: "mode".into(),
            value: SettingValue::Choice(1),
        },
    ] {
        graph.apply(&registry, &edit).unwrap();
    }
    doc.execute(Command::AddGraph(graph)).unwrap();
    doc
}

fn painted(doc: &Document) -> (GraphId, Arc<Graph>) {
    doc.scene()
        .graphs()
        .iter()
        .find(|(_, g)| g.name() == PAINTED)
        .map(|(id, g)| (id, g.clone()))
        .unwrap()
}

#[test]
fn a_palette_setting_survives_a_round_trip() {
    let doc = rich_document_v106();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(doc.scene().graphs(), back.scene().graphs());
    assert_eq!(doc.scene().paint(), back.scene().paint());
    let (_, graph) = painted(&back);
    assert_eq!(
        graph.get(BoxId(2)).unwrap().settings.get("palette"),
        Some(SettingValue::Palette("Moss".into()))
    );
    assert_eq!(graph.get(BoxId(3)).unwrap().settings.get("palette"), None);
    // The opened project's graphs see its palettes, and the box runs.
    let registry = back.scene().graphs().registry();
    assert_eq!(registry.palettes().len(), 2);
    let out = matgraph::Evaluator::with_threads(1).run(
        registry,
        &graph,
        matgraph::Size::new(16).unwrap(),
        &[BoxId(2)],
    );
    assert!(out.outcome(BoxId(2)).unwrap().is_ok());
}

#[test]
fn nothing_new_is_stored_without_a_palette_setting() {
    let doc = rich_document_v105();
    for (id, graph) in doc.scene().graphs().iter() {
        let text = serde_json::to_string(&v95::GraphV95::of(id, graph)).unwrap();
        assert!(!text.contains("Palette"), "{text}");
    }
}

#[test]
fn a_long_or_odd_palette_name_in_a_file_is_kept_within_the_limit() {
    let doc = rich_document_v106();
    let (id, graph) = painted(&doc);
    let mut stored = v95::GraphV95::of(id, &graph);
    let long = "é".repeat(matgraph::MAX_PALETTE_NAME + 50);
    for b in &mut stored.boxes.0 {
        for s in &mut b.settings.0 {
            if s.key == "palette" {
                s.value = v95::ValueV95::Palette(long.clone());
            }
        }
    }
    let (_, parts) = stored.into_parts().unwrap();
    let back = Graph::from_parts(parts, matgraph::Registry::standard()).unwrap();
    let Some(SettingValue::Palette(name)) = back.get(BoxId(2)).unwrap().settings.get("palette")
    else {
        panic!("the palette setting is gone");
    };
    assert_eq!(name.chars().count(), matgraph::MAX_PALETTE_NAME);
    // A palette value on a setting of another kind becomes its standard.
    let mut stored = v95::GraphV95::of(id, &graph);
    for b in &mut stored.boxes.0 {
        for s in &mut b.settings.0 {
            if s.key == "mode" {
                s.value = v95::ValueV95::Palette("Moss".into());
            }
        }
    }
    let (_, parts) = stored.into_parts().unwrap();
    let back = Graph::from_parts(parts, matgraph::Registry::standard()).unwrap();
    assert_ne!(
        back.get(BoxId(2)).unwrap().settings.get("mode"),
        Some(SettingValue::Palette("Moss".into()))
    );
}

#[test]
fn flipped_bytes_in_a_file_with_palette_settings_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v106()).unwrap();
    let mut state = 106u64;
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
                // Working it out never panics, palette or not.
                let _ = matgraph::Evaluator::with_threads(1).run(
                    &registry,
                    graph,
                    matgraph::Size::new(16).unwrap(),
                    &[],
                );
            }
        }
    }
}

const SAMPLE_V106: &[u8] = include_bytes!("../../tests/data/sample-v106.avtl");

#[test]
fn a_file_saved_by_format_version_106_opens_with_its_palette_settings() {
    let doc = from_bytes(SAMPLE_V106).expect("the version 106 sample opens");
    let rich = rich_document_v106();
    assert_eq!(doc.scene().graphs(), rich.scene().graphs());
    assert_eq!(
        doc.scene().paint().palettes(),
        rich.scene().paint().palettes()
    );
}

#[test]
fn a_version_105_file_opens_without_palette_settings() {
    const SAMPLE_V105: &[u8] = include_bytes!("../../tests/data/sample-v105.avtl");
    let doc = from_bytes(SAMPLE_V105).unwrap();
    for (_, graph) in doc.scene().graphs().iter() {
        assert!(!matgraph::palettes::uses_palettes(
            doc.scene().graphs().registry(),
            graph
        ));
    }
}
