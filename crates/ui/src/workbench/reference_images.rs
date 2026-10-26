//! Reference images of the main panels and windows (decision 0126,
//! Phase 9 milestone 9): each test draws the window on the GPU and
//! compares it with a picture kept in `crates/ui/tests/snapshots`, so a
//! change in how things look is seen in review, not found by a tester.
//!
//! The pictures are drawn with egui's own GPU renderer (the 3D view shows
//! empty, as the tests use a stand-in for the viewport's renderer). Without
//! They run only with `REQUIRE_GPU` set (CI's Linux window-and-GPU job,
//! with Mesa's software GPU, which made the pictures): other GPUs smooth
//! edges a little differently. When a look
//! changes on purpose, make the pictures again with
//! `UPDATE_SNAPSHOTS=1 REQUIRE_GPU=1 cargo test -p ui reference_images` and
//! look at the new ones before committing them.

use super::tests::{State, TestRenderer, harness_built_sized, info, with_a_cube};
use super::*;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotOptions};

/// The window's size for every picture.
const SIZE: [f32; 2] = [1280.0, 800.0];

/// A GPU-drawn window with a cube, or `None` (the test skips) without a
/// GPU.
fn window() -> Option<Harness<'static, State>> {
    if std::env::var_os("REQUIRE_GPU").is_none() {
        eprintln!("REQUIRE_GPU not set: reference images skipped.");
        return None;
    }
    let gpu = std::panic::catch_unwind(egui_kittest::wgpu::WgpuTestRenderer::new).is_ok();
    assert!(gpu, "REQUIRE_GPU is set but no GPU was found");
    let mut harness = harness_built_sized(
        Harness::builder().wgpu(),
        with_a_cube(Workbench::new(info(), None)),
        TestRenderer::default(),
        SIZE,
    );
    crate::theme::apply(&harness.ctx);
    harness.run();
    Some(harness)
}

/// Compares the window with its reference picture. Small differences in
/// how GPUs smooth edges are allowed; a moved, missing or recoloured part
/// isn't.
fn compare(harness: &mut Harness<'static, State>, name: &str) {
    harness.run();
    harness.snapshot_options(
        name,
        &SnapshotOptions::new().threshold(2.0).max_failed_pixels(400),
    );
}

fn select_the_cube(harness: &mut Harness<'static, State>) {
    let doc = harness.state_mut().workbench.document_mut();
    let id = doc.scene().children(None)[0];
    doc.set_selection([id]);
    harness.run();
}

#[test]
fn reference_standard_window() {
    let Some(mut harness) = window() else { return };
    select_the_cube(&mut harness);
    compare(&mut harness, "standard_window");
}

#[test]
fn reference_select_tool_list() {
    // The select tool list looks like the app's other menus (decision 0140).
    let Some(mut harness) = window() else { return };
    harness
        .get_by_label(crate::palette::SELECT_TOOL_MENU_LABEL)
        .click();
    harness.run();
    compare(&mut harness, "select_tool_list");
}

#[test]
fn reference_uv_edit() {
    let Some(mut harness) = window() else { return };
    select_the_cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    compare(&mut harness, "uv_edit");
}

#[test]
fn reference_settings() {
    let Some(mut harness) = window() else { return };
    harness.state_mut().workbench.open_settings();
    compare(&mut harness, "settings");
}

#[test]
fn reference_command_search() {
    let Some(mut harness) = window() else { return };
    harness.key_press(Key::Space);
    harness.run();
    for c in ["b", "e", "v"] {
        harness.event(egui::Event::Text(c.to_string()));
        harness.run();
    }
    compare(&mut harness, "command_search");
}

#[test]
fn reference_create_menu() {
    let Some(mut harness) = window() else { return };
    super::tests::main_menu(&harness, "Create").click();
    compare(&mut harness, "create_menu");
}

#[test]
fn reference_shape_lists() {
    let Some(mut harness) = window() else { return };
    harness.event(egui::Event::PointerMoved(egui::pos2(500.0, 300.0)));
    harness.key_press(Key::B);
    compare(&mut harness, "shape_list");
    harness.key_press(Key::Escape);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::B);
    compare(&mut harness, "more_shapes_list");
}

#[test]
fn reference_export_window() {
    let Some(mut harness) = window() else { return };
    select_the_cube(&mut harness);
    harness.state_mut().workbench.exporting.open = true;
    compare(&mut harness, "export");
}

#[test]
fn reference_keys_window() {
    let Some(mut harness) = window() else { return };
    harness.state_mut().workbench.shortcuts.open = true;
    compare(&mut harness, "keys");
}

#[test]
fn reference_check_for_baking() {
    let Some(mut harness) = window() else { return };
    select_the_cube(&mut harness);
    harness.state_mut().workbench.baking.open = true;
    compare(&mut harness, "check_for_baking");
}

#[test]
fn reference_tool_ring() {
    let Some(mut harness) = window() else { return };
    select_the_cube(&mut harness);
    harness.event(egui::Event::PointerMoved(egui::pos2(500.0, 350.0)));
    harness.run();
    harness.key_press(Key::E);
    compare(&mut harness, "tool_ring");
}

#[test]
fn reference_levels_panel() {
    // The Levels panel (decision 0165): a smoothed, bevelled cube with
    // three levels, the first without Subdivide and previewed, the second
    // with a budget.
    let Some(mut harness) = window() else { return };
    let doc = harness.state_mut().workbench.document_mut();
    let id = doc.scene().children(None)[0];
    for kind in [
        scene::ModifierKind::Subdivide(Default::default()),
        scene::ModifierKind::LiveBevel(Default::default()),
    ] {
        doc.execute(scene::Command::AddModifier {
            ids: vec![id],
            kind,
        })
        .unwrap();
    }
    let modifiers = doc.scene().modifiers(id).to_vec();
    let subdivide = modifiers
        .iter()
        .find(|m| matches!(m.kind, scene::ModifierKind::Subdivide(_)))
        .unwrap()
        .id;
    let mut lods = scene::Lods {
        on: true,
        preview: 1,
        ..scene::Lods::default()
    };
    lods.levels[0] = lods.levels[0].with_override(&modifiers, subdivide, false, None);
    lods.levels[1].budget = 200;
    doc.execute(scene::Command::SetLods {
        ids: vec![id],
        lods,
    })
    .unwrap();
    doc.set_selection([id]);
    crate::layout::show_levels(&mut harness.state_mut().workbench.dock);
    for _ in 0..200 {
        harness.run_steps(1);
        if !harness.state().workbench.drawing.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    compare(&mut harness, "levels_panel");
}

#[test]
fn reference_modifier_list() {
    // A cube with three modifiers listed under it (decision 0168), the
    // Live bevel clicked: the Attributes panel shows only its settings.
    let Some(mut harness) = window() else { return };
    let doc = harness.state_mut().workbench.document_mut();
    let id = doc.scene().children(None)[0];
    for kind in [
        scene::ModifierKind::Array(scene::ArraySettings::default()),
        scene::ModifierKind::Shell(scene::ShellOptions::default()),
        scene::ModifierKind::LiveBevel(scene::LiveBevelOptions::default()),
    ] {
        doc.execute(scene::Command::AddModifier {
            ids: vec![id],
            kind,
        })
        .unwrap();
    }
    doc.set_selection([id]);
    let bevel = doc.scene().modifiers(id)[2].id;
    harness.state_mut().workbench.attributes.picked_modifier = Some((id, bevel));
    for _ in 0..200 {
        harness.run_steps(1);
        if !harness.state().workbench.drawing.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    compare(&mut harness, "modifier_list");
}

#[test]
fn reference_update_offer() {
    // The window offering a downloaded update (decision 0172).
    let Some(mut harness) = window() else { return };
    harness
        .state_mut()
        .workbench
        .offer_update(super::UpdateOffer {
            build: 4,
            notes_url: "https://github.com/Rhyslos/Aventail-3D/releases/tag/build-4".into(),
        });
    compare(&mut harness, "update_offer");
}

#[test]
fn reference_splash_update_question() {
    // The question on the splash screen when a new version is found
    // (decision 0173).
    if std::env::var_os("REQUIRE_GPU").is_none() {
        return;
    }
    let info = info();
    let mut skip = false;
    let mut harness = Harness::builder()
        .wgpu()
        .with_size(SIZE)
        .build_ui(move |ui| {
            crate::theme::apply(ui.ctx());
            crate::splash::show(
                ui,
                &crate::splash::SplashView {
                    info: &info,
                    accent: crate::theme::Accent::default(),
                    doing: "Build 7 is available.",
                    progress: 1.0,
                },
            );
            let ctx = ui.ctx().clone();
            let _ = crate::splash::update_question(
                &ctx,
                7,
                "https://github.com/Rhyslos/Aventail-3D/releases/tag/build-7",
                &mut skip,
            );
        });
    harness.run();
    harness.snapshot_options(
        "splash_update_question",
        &SnapshotOptions::new().threshold(2.0).max_failed_pixels(400),
    );
}
