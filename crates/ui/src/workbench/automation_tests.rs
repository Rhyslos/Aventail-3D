//! Tests for the settings of the automatic UV tools in the window
//! (decision 0110): the UV menu's five settings submenus, the settings
//! reaching what the tools record, and mending as a project setting.

use super::automation::*;
use super::modelling_tests::H;
use super::tests::{fresh, main_menu};
use super::unwrapping::{PackQuality, SeamHiding, UvSettings};
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;

fn plane(harness: &mut H) -> scene::ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 2],
            ..Default::default()
        }),
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    harness.run();
    id
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

fn last_operation(harness: &H, id: scene::ObjectId) -> Option<scene::Operation> {
    let doc = harness.state().workbench.document();
    doc.scene()
        .object(id)?
        .stack
        .layers()
        .last()?
        .entries
        .last()
        .map(|e| e.operation.clone())
}

fn open_submenu(harness: &mut H, label: &str) {
    main_menu(harness, "UV").click();
    harness.run();
    harness.get_by_label_contains(label).hover();
    harness.run();
}

#[test]
fn every_settings_submenu_holds_its_settings() {
    let mut harness = fresh();
    plane(&mut harness);
    for (submenu, inside) in [
        (
            SEAMS_SETTINGS_LABEL,
            vec![CORNER_ANGLE_LABEL, HARD_EDGES_LABEL, HIDING_LABEL],
        ),
        (UNWRAP_SETTINGS_LABEL, vec![METHOD_LABEL, STRIP_ANGLE_LABEL]),
        (RELAX_SETTINGS_LABEL, vec![RELAX_STEPS_LABEL]),
        (PACK_SETTINGS_LABEL, vec![QUALITY_LABEL]),
        (
            MEND_SETTINGS_LABEL,
            vec![MEND_LABEL, UNEVEN_LABEL, SIZE_OFF_LABEL],
        ),
    ] {
        open_submenu(&mut harness, submenu);
        assert!(harness.query_by_label(SETTINGS_HEADING).is_some());
        for label in inside {
            assert!(
                harness.query_by_label(label).is_some(),
                "{label} in {submenu}"
            );
        }
        harness.key_press(egui::Key::Escape);
        harness.run();
    }
}

#[test]
fn the_settings_reach_what_the_tools_record() {
    let mut harness = fresh();
    let id = plane(&mut harness);
    {
        let s = &mut harness.state_mut().workbench.uv_settings;
        s.strip_angle = 40.0;
        s.seam_hiding = SeamHiding::Behind;
        s.relax_steps = 200;
    }
    run(&mut harness, CommandId::Unwrap);
    match last_operation(&harness, id) {
        Some(scene::Operation::Unwrap { options, .. }) => {
            assert_eq!(options.strip_angle, 40.0);
            assert_eq!(options.hide, scene::HideSeams::Behind);
        }
        other => panic!("{other:?}"),
    }
    run(&mut harness, CommandId::Relax);
    assert!(matches!(
        last_operation(&harness, id),
        Some(scene::Operation::RelaxUvs { steps: 200, .. })
    ));
    // Out-of-range settings are kept in range.
    harness.state_mut().workbench.uv_settings.relax_steps = 0;
    harness.state_mut().workbench.uv_settings.strip_angle = 500.0;
    run(&mut harness, CommandId::Relax);
    assert!(matches!(
        last_operation(&harness, id),
        Some(scene::Operation::RelaxUvs { steps: 1, .. })
    ));
    let options = harness.state().workbench.unwrap_options(None);
    assert_eq!(options.strip_angle, uv::unwrap::MAX_STRIP_ANGLE);
    // Suggest seams uses its settings: no corners and no hard edges cut
    // on a cube leaves only what it needs to lie flat.
    let mut harness = fresh();
    let cube = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([cube]);
    {
        let s = &mut harness.state_mut().workbench.uv_settings;
        s.seam_corner_angle = 180.0;
        s.seam_hard_edges = false;
    }
    harness.run();
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(cube)
        .unwrap()
        .mesh;
    let seams = mesh::uvs::seams(&mesh).len();
    assert!(seams > 0 && seams < 12, "{seams}");
}

#[test]
fn pack_quality_sets_the_grid() {
    assert_eq!(PackQuality::Standard.cells(2048, 8), 512);
    assert_eq!(PackQuality::Fast.cells(2048, 8), 256);
    assert_eq!(PackQuality::Best.cells(2048, 8), uv::pack::MAX_CELLS);
    assert_eq!(PackQuality::Standard.cells(256, 8), 128);
    for quality in PackQuality::ALL {
        assert!(!quality.name().is_empty() && !quality.help().is_empty());
    }
    for hiding in SeamHiding::ALL {
        assert!(!hiding.name().is_empty() && !hiding.help().is_empty());
    }
}

#[test]
fn mending_is_the_projects_and_one_step() {
    let mut harness = fresh();
    plane(&mut harness);
    open_submenu(&mut harness, MEND_SETTINGS_LABEL);
    let before = harness.state().workbench.document().undo_steps();
    harness.get_by_label(MEND_LABEL).click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert!(!doc.scene().mending().on);
    assert_eq!(doc.undo_steps(), before + 1);
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert!(harness.state().workbench.document().scene().mending().on);
}

#[test]
fn every_setting_is_remembered_and_old_settings_read_as_standard() {
    let read: UvSettings = ron::from_str("(live_unwrap: false)").unwrap();
    let standard = UvSettings::default();
    assert_eq!(read.strip_angle, standard.strip_angle);
    assert_eq!(read.relax_steps, 60);
    assert_eq!(read.seam_corner_angle, 66.0);
    assert!(read.seam_hard_edges);
    assert_eq!(read.seam_hiding, SeamHiding::BelowAndBehind);
    assert_eq!(read.pack_quality, PackQuality::Standard);
    let changed = UvSettings {
        relax_steps: 120,
        seam_hiding: SeamHiding::Shortest,
        pack_quality: PackQuality::Best,
        ..standard
    };
    let text = ron::to_string(&changed).unwrap();
    let back: UvSettings = ron::from_str(&text).unwrap();
    assert_eq!(back, changed);
}
