//! Presets and starting layouts (decision 0206): a material with no
//! layers starts from a layout, the Presets row asks for files, and
//! loading a style, a material or brushes does what each holds.

use super::WorkbenchAction;
use super::paint_layers_tests::{painting_the_cube, press};
use crate::paint_layers::presets::PresetKind;
use egui_kittest::kittest::Queryable;
use fileio::preset::{BrushRecipe, Preset};
use paint::{Blend, Layer};
use std::path::Path;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn layer_names(harness: &H) -> Vec<String> {
    let doc = harness.state().workbench.document();
    let Some(set) = doc.scene().paint().set("Default") else {
        return Vec::new();
    };
    set.layers.iter().map(|l| l.name.clone()).collect()
}

#[test]
fn a_hand_painted_start_lays_out_its_layers_in_one_step() {
    let mut harness = painting_the_cube();
    assert!(layer_names(&harness).is_empty());
    press(&mut harness, "Hand-painted");
    // Colours under the light and the edge highlight, details on top
    // (a review found one paint layer on top covered the light).
    assert_eq!(
        layer_names(&harness),
        ["Base colour", "Colours", "Light and shade", "Edge highlight", "Details"]
    );
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    // The base is the material's colour; the made layers were made.
    assert_eq!(
        set.layers[0].content,
        paint::Content::Fill([187, 187, 189, 255])
    );
    assert!(set.layers[2].image().unwrap().painted_tiles() > 0);
    assert!(set.layers[3].mask.as_ref().unwrap().image.painted_tiles() > 0);
    assert_eq!(doc.undo_label(), Some("Start a hand-painted layout"));
    // Colours is chosen to paint on: strokes and fills go under the light.
    let bench = &harness.state().workbench;
    assert_eq!(bench.paint_layers.active("Default", set), Some(vec![1]));
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    assert!(layer_names(&harness).is_empty());
}

#[test]
fn a_realistic_start_has_dirt_wear_and_dust() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Realistic");
    assert_eq!(
        layer_names(&harness),
        ["Base colour", "Cavity dirt", "Edge wear", "Dust", "Paint"]
    );
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    assert_eq!(set.layers[1].blend, Blend::Multiply);
    // Once there are layers, the starting layouts aren't offered.
    assert!(harness.query_by_label("Realistic").is_none());
}

#[test]
fn the_presets_row_asks_for_files() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Hand-painted");
    harness.state_mut().actions.clear();
    press(&mut harness, "Save style…");
    press(&mut harness, "Save material…");
    press(&mut harness, "Load preset…");
    let actions = &harness.state().actions;
    assert!(actions.contains(&WorkbenchAction::SavePreset {
        kind: PresetKind::Style,
        path: None
    }));
    assert!(actions.contains(&WorkbenchAction::SavePreset {
        kind: PresetKind::Material,
        path: None
    }));
    assert!(actions.contains(&WorkbenchAction::LoadPreset { path: None }));
}

#[test]
fn a_saved_style_loads_above_the_chosen_layer() {
    let mut harness = painting_the_cube();
    press(&mut harness, "Hand-painted");
    // Keep Edge highlight as a style.
    {
        let bench = &mut harness.state_mut().workbench;
        bench.paint_layers.choose("Default", vec![3], false);
    }
    let (name, text) = harness
        .state()
        .workbench
        .preset_file(PresetKind::Style)
        .unwrap();
    assert_eq!(name, "Edge highlight");
    harness
        .state_mut()
        .workbench
        .load_preset(Path::new("edge.paintpreset"), text.as_bytes());
    harness.run();
    harness.run();
    assert_eq!(
        layer_names(&harness),
        [
            "Base colour",
            "Colours",
            "Light and shade",
            "Edge highlight",
            "Edge highlight 2",
            "Details"
        ]
    );
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Add the style Edge highlight"));
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(
        set.layers[4].mask.as_ref().unwrap().image.painted_tiles() > 0,
        "made"
    );
}

#[test]
fn a_material_preset_replaces_colour_and_layers_in_one_step() {
    let mut harness = painting_the_cube();
    let preset = Preset::Material {
        name: "Wood".into(),
        colour: [0.5, 0.25, 0.1],
        layers: vec![
            Layer::fill("Base", [128, 64, 25, 255]),
            Layer::fill("Top", [1, 2, 3, 255]),
        ],
    };
    let text = fileio::preset::write_preset(&preset);
    harness
        .state_mut()
        .workbench
        .load_preset(Path::new("wood.paintpreset"), text.as_bytes());
    harness.run();
    assert_eq!(layer_names(&harness), ["Base", "Top"]);
    let doc = harness.state().workbench.document();
    let id = doc.scene().walk()[0].0;
    assert_eq!(doc.scene().material_list(id)[0].color, [0.5, 0.25, 0.1]);
    harness.state_mut().workbench.document_mut().undo().unwrap();
    let doc = harness.state().workbench.document();
    assert!(doc.scene().paint().set("Default").is_none());
    assert_ne!(doc.scene().material_list(id)[0].color, [0.5, 0.25, 0.1]);
}

#[test]
fn brushes_go_out_and_come_back_without_doubles() {
    let mut harness = painting_the_cube();
    assert!(
        harness
            .state()
            .workbench
            .preset_file(PresetKind::Brushes)
            .is_none(),
        "nothing to keep yet"
    );
    let brush = BrushRecipe {
        name: "Chunky".into(),
        tip: 0,
        hardness: 0.7,
        spacing: 0.1,
        flow: 0.8,
        size_pressure: None,
        flow_pressure: None,
        kind: 0,
        alignment: Some(paint::Alignment::Uv.index()),
    };
    let bad = BrushRecipe {
        hardness: 9.0,
        ..brush.clone()
    };
    let text = fileio::preset::write_preset(&Preset::Brushes(vec![brush.clone(), bad]));
    let bench = &mut harness.state_mut().workbench;
    bench.load_preset(Path::new("b.paintpreset"), text.as_bytes());
    assert_eq!(bench.saved_brushes.len(), 1, "the bad one is left out");
    bench.load_preset(Path::new("b.paintpreset"), text.as_bytes());
    assert_eq!(bench.saved_brushes.len(), 1, "no doubles");
    let (_, out) = bench.preset_file(PresetKind::Brushes).unwrap();
    assert_eq!(
        fileio::preset::read_preset(out.as_bytes(), 256).unwrap(),
        Preset::Brushes(vec![brush])
    );
}

#[test]
fn a_damaged_preset_says_why() {
    let mut harness = painting_the_cube();
    let bench = &mut harness.state_mut().workbench;
    bench.load_preset(Path::new("bad.paintpreset"), b"nonsense");
    assert!(bench.note.as_deref().unwrap().contains("bad.paintpreset"));
}

#[test]
fn the_brush_bar_saves_and_loads_brushes() {
    let mut harness = painting_the_cube();
    harness.state_mut().actions.clear();
    press(&mut harness, "Load brushes…");
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::LoadPreset { path: None })
    );
}

#[test]
fn painting_after_a_hand_painted_start_stays_shaded_by_the_light() {
    // A review found the first Fill the layer wiped the light out flat.
    let mut harness = painting_the_cube();
    press(&mut harness, "Hand-painted");
    harness.key_press(egui::Key::Num8);
    harness.run();
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    // The bucket's target is Colours, under Light and shade.
    let bench = &harness.state().workbench;
    assert_eq!(bench.paint_layers.active("Default", set), Some(vec![1]));
    assert_eq!(set.layers[2].name, "Light and shade");
}
