//! Tests for exporting an STL file for 3D printing in the window
//! (decision 0298): the 3D printing preset, its rows, the print check,
//! remembered settings, and the offer to export unmerged.

use super::tests::{fresh, main_menu};
use super::*;
use egui_kittest::kittest::Queryable;
use fileio::export::{ExportFormat, ExportPreset, PrintUnits};

fn open_export(harness: &mut super::modelling_tests::H) {
    main_menu(harness, "File").click();
    harness.run();
    harness.get_by_label("Export…").click();
    harness.run();
}

fn made_for(harness: &mut super::modelling_tests::H, name: &str) {
    harness.get_by_label_contains("Made for:").click();
    harness.run();
    harness.get_by_label(name).click();
    harness.run();
}

fn last_export(harness: &super::modelling_tests::H) -> Option<ExportRequest> {
    harness.state().actions.iter().rev().find_map(|a| match a {
        WorkbenchAction::Export(request) => Some((**request).clone()),
        _ => None,
    })
}

#[test]
fn three_d_printing_shows_its_own_rows_and_exports_stl() {
    let mut harness = fresh();
    open_export(&mut harness);
    made_for(&mut harness, "3D printing");
    assert!(
        harness
            .query_by_label("STL · binary · Z up · triangles")
            .is_some()
    );
    // Its own rows; none of the texture rows.
    assert!(harness.query_by_label("Print units").is_some());
    assert!(harness.query_by_label("Merge into one solid").is_some());
    assert!(harness.query_by_label("Print check").is_some());
    assert!(harness.query_by_label("Textures").is_none());
    assert!(harness.query_by_label("Levels of detail").is_none());
    // The standard cube, 100 cm: 1,000 mm each way.
    assert!(harness.query_by_label("1,000 × 1,000 × 1,000").is_some());
    assert!(
        harness
            .query_by_label_contains("Nothing found: every shape is closed")
            .is_some()
    );
    harness.get_by_label("Inches").click();
    harness.run();
    assert!(harness.query_by_label("39.4 × 39.4 × 39.4").is_some());
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    let request = last_export(&harness).unwrap();
    assert_eq!(request.format, ExportFormat::Stl);
    assert_eq!(request.preset, Some(ExportPreset::Printing));
    assert_eq!(request.suggested_name, "Untitled.stl");
    assert_eq!(request.options.print_units, PrintUnits::Inches);
    assert!(request.options.merge);
}

#[test]
fn stl_is_a_custom_file_type_too() {
    let mut harness = fresh();
    open_export(&mut harness);
    made_for(&mut harness, "Custom settings");
    harness.get_by_label("STL (.stl)").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.exporting.choice.format,
        ExportFormat::Stl
    );
    assert_eq!(harness.state().workbench.exporting.choice.preset, None);
    assert!(harness.query_by_label("Print units").is_some());
    assert!(
        harness
            .query_by_label_contains("STL is always triangles with Z up")
            .is_some()
    );
}

#[test]
fn the_print_check_warns_about_a_flat_sheet() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Plane.standard(),
        )))
        .unwrap();
    harness.run();
    open_export(&mut harness);
    made_for(&mut harness, "3D printing");
    assert!(
        harness
            .query_by_label_contains("is a flat sheet with no thickness")
            .is_some()
    );
    // Two shapes, merging on: it says the sheet will stop the merge.
    assert!(
        harness
            .query_by_label_contains("Merging needs every shape to be a closed solid")
            .is_some()
    );
    // Still exportable: warnings only.
    harness.get_all_by_label("Export…").last().unwrap().click();
    harness.run();
    assert!(last_export(&harness).is_some());
}

#[test]
fn print_units_and_merge_are_remembered() {
    let mut harness = fresh();
    let mut choice = ExportChoice::default().with_preset(ExportPreset::Printing);
    choice.print_units = PrintUnits::Centimetres;
    choice.merge = false;
    let settings = ExportSettings::of(choice);
    let back: ExportSettings = ron::from_str(&ron::to_string(&settings).unwrap()).unwrap();
    assert_eq!(back, settings);
    let again = back.choice();
    assert_eq!(again.preset, Some(ExportPreset::Printing));
    assert_eq!(again.format, ExportFormat::Stl);
    assert_eq!(
        (again.print_units, again.merge),
        (PrintUnits::Centimetres, false)
    );
    // Kept when Made for changes, as Crisp pixels is.
    let godot = again.with_preset(ExportPreset::Godot);
    assert_eq!(godot.print_units, PrintUnits::Centimetres);
    // Settings saved before STL read as millimetres, merged.
    let old: ExportSettings = ron::from_str("(preset: Some(Godot))").unwrap();
    assert_eq!(
        (old.print_units, old.merge),
        (PrintUnits::Millimetres, true)
    );
    // An export remembers them.
    let request = ExportRequest {
        preset: Some(ExportPreset::Printing),
        format: ExportFormat::Stl,
        options: fileio::export::ExportOptions {
            print_units: PrintUnits::Metres,
            merge: false,
            ..fileio::export::ExportOptions::default()
        },
        path: None,
        suggested_name: "Untitled.stl".into(),
    };
    harness.state_mut().workbench.exported(
        &request,
        "/work/part.stl".into(),
        fileio::export::ExportSummary::default(),
    );
    let remembered = harness.state().workbench.export_settings();
    assert_eq!(
        (remembered.print_units, remembered.merge),
        (PrintUnits::Metres, false)
    );
}

#[test]
fn a_failed_merge_offers_to_export_unmerged() {
    let mut harness = fresh();
    let request = ExportRequest {
        preset: Some(ExportPreset::Printing),
        format: ExportFormat::Stl,
        options: fileio::export::ExportOptions::default(),
        path: None,
        suggested_name: "Untitled.stl".into(),
    };
    harness.state_mut().workbench.exporting.choice =
        ExportChoice::default().with_preset(ExportPreset::Printing);
    harness.state_mut().workbench.merge_failed(
        &request,
        "/work/part.stl".into(),
        "Plane couldn't be merged into one solid: the shape isn't a closed solid",
    );
    harness.run();
    assert!(harness.state().workbench.exporting.open, "the window opens");
    assert!(
        harness
            .query_by_label_contains("The last export stopped: Plane couldn't be merged")
            .is_some()
    );
    harness.get_by_label("Export without merging").click();
    harness.run();
    let unmerged = last_export(&harness).unwrap();
    assert!(!unmerged.options.merge);
    assert_eq!(
        unmerged.path,
        Some("/work/part.stl".into()),
        "the same file"
    );
    assert!(!harness.state().workbench.exporting.open);
    assert!(harness.state().workbench.exporting.merge_failed.is_none());
}
