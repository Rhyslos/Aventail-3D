//! Tests for locked pieces and pieces' own settings in the window
//! (decision 0112): locking and unlocking, the tools leaving locked
//! pieces alone, and the Piece settings submenu.

use super::modelling_tests::{H, note};
use super::piece_settings::*;
use super::tests::{fresh, main_menu};
use super::unwrapping::UnwrapWay;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;

fn cube(harness: &mut H) -> scene::ObjectId {
    let id = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    id
}

fn shape(harness: &H, id: scene::ObjectId) -> std::sync::Arc<scene::Mesh> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

fn corner_uvs(mesh: &scene::Mesh) -> Vec<glam::Vec2> {
    let count = mesh.face_layout().1.len();
    mesh::uvs::Uvs::of(mesh).unwrap().into_values(count)
}

#[test]
fn a_locked_object_is_left_alone_by_unwrap_relax_and_pack() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    // Turn its UVs so every tool would change them.
    let faces: Vec<scene::FaceId> = shape(&harness, id).faces().map(|(f, _)| f).collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::TransformUvs {
                faces,
                points: Vec::new(),
                transform: glam::Affine2::from_angle(0.4),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    let before_steps = harness.state().workbench.document().undo_steps();
    harness.state_mut().workbench.run_lock(true);
    harness.run();
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        before_steps + 1
    );
    assert!(
        note(&harness).contains("Lock the UVs of"),
        "{}",
        note(&harness)
    );
    let kept = corner_uvs(&shape(&harness, id));
    for command in [CommandId::Unwrap, CommandId::Relax, CommandId::Pack] {
        run(&mut harness, command);
        assert_eq!(corner_uvs(&shape(&harness, id)), kept, "{command:?}");
    }
    assert!(note(&harness).contains("locked"), "{}", note(&harness));
    // Locking again says so; unlocked, Unwrap changes it.
    harness.state_mut().workbench.run_lock(true);
    harness.run();
    assert!(note(&harness).contains("already"), "{}", note(&harness));
    harness.state_mut().workbench.run_lock(false);
    harness.run();
    run(&mut harness, CommandId::Unwrap);
    assert_ne!(corner_uvs(&shape(&harness, id)), kept);
}

#[test]
fn piece_settings_are_given_and_cleared_as_steps() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    {
        let s = &mut harness.state_mut().workbench.uv_settings;
        s.piece_method = Some(UnwrapWay::KeepSizes);
        s.piece_strips = Some(false);
        s.piece_density = Some(1024.0);
    }
    let settings = harness.state().workbench.chosen_piece_settings();
    assert_eq!(settings.method, Some(scene::UnwrapMethod::SizeKeeping));
    // 1,024 px/m on a 2,048 texture: half a texture width per metre.
    let texture = harness.state().workbench.document().scene().texture();
    let expected = 1024.0 / texture.size as f32 / 100.0;
    assert!((settings.density.unwrap() - expected).abs() < 1e-7);
    harness.state_mut().workbench.run_piece_settings(settings);
    harness.run();
    let mesh = shape(&harness, id);
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    assert_eq!(scene::PieceSettings::of(&mesh, &faces), settings);
    assert!(
        note(&harness).contains("Every unwrap"),
        "{}",
        note(&harness)
    );
    harness
        .state_mut()
        .workbench
        .run_piece_settings(scene::PieceSettings::default());
    harness.run();
    assert!(scene::PieceSettings::of(&shape(&harness, id), &faces).is_empty());
}

#[test]
fn the_uv_menu_holds_locking_and_the_piece_settings() {
    let mut harness = fresh();
    cube(&mut harness);
    main_menu(&harness, "UV").click();
    harness.run();
    for label in [LOCK_LABEL, UNLOCK_LABEL] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.get_by_label_contains(PIECE_SETTINGS_LABEL).hover();
    harness.run();
    for label in [
        SET_PIECE_LABEL,
        CLEAR_PIECE_LABEL,
        OWN_DENSITY_LABEL,
        "Don't straighten",
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.get_by_label(SET_PIECE_LABEL).click();
    harness.run();
    assert!(
        note(&harness).contains("piece settings"),
        "{}",
        note(&harness)
    );
}

#[test]
fn nothing_selected_says_so() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    harness.state_mut().workbench.run_lock(true);
    assert!(note(&harness).contains("select the objects"));
    harness
        .state_mut()
        .workbench
        .run_piece_settings(scene::PieceSettings::default());
    assert!(note(&harness).contains("select the objects"));
}
