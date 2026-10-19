//! Tests for trim sheets in the window (decision 0106): the Trim menu,
//! Put on trim row, and packing and the bake check leaving trim pieces
//! alone.

use super::modelling_tests::{H, cube, last_operation, note, shape};
use super::tests::{fresh, main_menu};
use egui_kittest::kittest::Queryable;
use scene::Operation;

fn rows(harness: &H) -> scene::TrimSheet {
    harness.state().workbench.document().scene().trim().clone()
}

fn set_trim(harness: &mut H, sheet: scene::TrimSheet) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTrim(sheet))
        .unwrap();
    harness.run();
}

#[test]
fn the_trim_menu_makes_even_rows_as_one_undo_step() {
    let mut harness = fresh();
    main_menu(&harness, "UV").click();
    harness.run();
    harness.get_by_label_contains("Trim sheet").click();
    harness.run();
    assert!(harness.query_by_label("No rows yet.").is_some());
    harness.get_by_label("4").click();
    harness.run();
    assert_eq!(rows(&harness), scene::TrimSheet::even(4));
    // Add a row: there's no room under the lowest of four even rows.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(rows(&harness).rows.is_empty());
}

#[test]
fn put_on_trim_row_lays_the_pieces_along_it() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    set_trim(&mut harness, scene::TrimSheet::even(4));
    let row = rows(&harness).rows[1].clone();
    super::tests::uv_group(&mut harness, super::UV_LAY_OUT_MENU);
    harness.get_by_label_contains("Put on trim row").click();
    harness.run();
    harness.get_by_label("Row 2").click();
    harness.run();
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::PlaceOnTrim { .. })
    ));
    assert!(note(&harness).contains("on Row 2"), "{}", note(&harness));
    let mesh = shape(&harness);
    let islands = mesh::uvs::islands(&mesh);
    for island in &islands {
        assert!(uv::trim::on_row(&mesh, island, row.bottom, row.top));
    }
    // Six sides end to end run past the square's right side, which the
    // bake check accepts for trim pieces (the whole cube is on the trim
    // sheet, so nothing else shares its texture); Pack leaves them on the
    // row.
    let problems: Vec<String> = harness
        .state_mut()
        .workbench
        .bake_problems()
        .iter()
        .map(|p| p.kind.text())
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
    let before = shape(&harness);
    harness.state_mut().workbench.run_pack();
    harness.run();
    assert_eq!(shape(&harness).as_ref(), before.as_ref());
    assert!(
        note(&harness).contains("stays on its row"),
        "{}",
        note(&harness)
    );
}

#[test]
fn trim_pieces_sharing_a_texture_with_others_are_listed() {
    let mut harness = fresh();
    let id = cube(&harness);
    set_trim(&mut harness, scene::TrimSheet::even(4));
    let row = rows(&harness).rows[3].clone();
    let first: Vec<scene::FaceId> = shape(&harness).faces().map(|(f, _)| f).take(1).collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: Operation::PlaceOnTrim {
                faces: first,
                bottom: row.bottom,
                top: row.top,
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    let problems: Vec<String> = harness
        .state_mut()
        .workbench
        .bake_problems()
        .iter()
        .map(|p| p.kind.text())
        .collect();
    assert!(
        problems.contains(
            &"1 polygon on the trim sheet sharing a texture with other pieces".to_string()
        ),
        "{problems:?}"
    );
}

#[test]
fn without_rows_the_menu_says_how_to_make_them() {
    let mut harness = fresh();
    super::tests::uv_group(&mut harness, super::UV_LAY_OUT_MENU);
    harness.get_by_label_contains("Put on trim row").click();
    harness.run();
    assert!(harness.query_by_label("No rows yet").is_some());
}

#[test]
fn rows_change_and_overlapping_rows_are_refused() {
    let mut harness = fresh();
    set_trim(&mut harness, scene::TrimSheet::even(2));
    // The top row made narrower.
    let mut sheet = rows(&harness);
    sheet.rows[0].bottom = 0.75;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTrim(sheet.clone()))
        .unwrap();
    harness.run();
    assert_eq!(rows(&harness), sheet);
    // Overlapping rows are never kept.
    let mut bad = sheet;
    bad.rows[1].top = 0.9;
    assert!(
        harness
            .state_mut()
            .workbench
            .document_mut()
            .execute(scene::Command::SetTrim(bad))
            .is_err()
    );
}
