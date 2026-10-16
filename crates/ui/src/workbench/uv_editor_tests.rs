//! Tests for the UV Editor in the window (decision 0096): the UV Edit
//! layout and the Window menu, the UV mode, and selecting in the editor.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, main_menu, quick_harness};
use super::*;
use egui::{Event, PointerButton};
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementKind};

fn select_cube(harness: &mut H) {
    let id = cube(harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
}

fn click_at(harness: &mut H, at: egui::Pos2, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        harness.step();
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

/// The middle, in the texture, of the cube's polygon facing `normal`.
fn middle_of(harness: &H, normal: glam::Vec3) -> (scene::FaceId, glam::Vec2) {
    let mesh = super::modelling_tests::shape(harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (face, corners) = mesh
        .faces()
        .find(|(f, _)| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap();
    let sum: glam::Vec2 = (0..corners.len())
        .map(|k| uvs.corner(&mesh, face, k).unwrap())
        .sum();
    (face, sum / corners.len() as f32)
}

fn selected_polygons(harness: &H) -> usize {
    let doc = harness.state().workbench.document();
    doc.elements(cube(harness))
        .map_or(0, |s| s.count(ElementKind::Polygons))
}

#[test]
fn the_uv_edit_layout_shows_the_editor_in_the_uv_mode() {
    let mut harness = fresh();
    assert!(
        harness
            .query_by_label(crate::uv_editor::UV_EDITOR_LABEL)
            .is_none()
    );
    harness.get_by_label("UV Edit").click();
    harness.run();
    let bench = &harness.state().workbench;
    assert_eq!(bench.layout_choice(), LayoutChoice::UvEdit);
    assert_eq!(bench.mode, Mode::Uv);
    let editor = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect();
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    assert!(editor.min.x > viewport.center().x, "beside the viewport");
    // Back to the standard layout, out of the UV mode, and the editor gone.
    harness.get_by_label("Standard").click();
    harness.run();
    assert_eq!(harness.state().workbench.mode, Mode::Model);
    assert!(
        harness
            .query_by_label(crate::uv_editor::UV_EDITOR_LABEL)
            .is_none()
    );
}

#[test]
fn each_layout_keeps_its_own_arrangement() {
    let mut harness = fresh();
    harness.state_mut().workbench.toggle_uv_editor();
    harness.run();
    assert!(
        harness
            .state()
            .workbench
            .layout()
            .find_tab(&Panel::UvEditor)
            .is_some()
    );
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::Standard);
    harness.run();
    // The standard layout still has the editor it was given.
    assert!(
        harness
            .state()
            .workbench
            .layout()
            .find_tab(&Panel::UvEditor)
            .is_some()
    );
    assert!(
        harness
            .state()
            .workbench
            .uv_layout()
            .find_tab(&Panel::UvEditor)
            .is_some()
    );
}

#[test]
fn the_window_menu_shows_and_puts_away_the_editor() {
    let mut harness = fresh();
    for shown in [true, false] {
        main_menu(&harness, "Window").click();
        harness.run();
        // The menu entry, above the editor's own tab of the same name.
        harness
            .get_all_by_label(UV_EDITOR_ENTRY)
            .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
            .unwrap()
            .click();
        harness.run();
        assert_eq!(
            harness
                .query_by_label(crate::uv_editor::UV_EDITOR_LABEL)
                .is_some(),
            shown
        );
    }
}

#[test]
fn a_click_in_the_uv_mode_selects_a_whole_island() {
    let mut harness = quick_harness();
    select_cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    let (_, uv) = middle_of(&harness, glam::Vec3::Z);
    let at = harness.state().workbench.uv_editor.screen_of(uv).unwrap();
    click_at(&mut harness, at, Modifiers::NONE);
    // A plain cube's top is one polygon, one island.
    assert_eq!(selected_polygons(&harness), 1);
    // Ctrl + click adds the side's island too, and again takes it away.
    let (side, uv) = middle_of(&harness, glam::Vec3::X);
    let at = harness.state().workbench.uv_editor.screen_of(uv).unwrap();
    click_at(&mut harness, at, Modifiers::COMMAND);
    assert_eq!(selected_polygons(&harness), 2);
    let doc = harness.state().workbench.document();
    assert!(
        doc.elements(cube(&harness))
            .unwrap()
            .contains(Element::Polygon(side))
    );
}

#[test]
fn the_polygons_mode_selects_single_polygons_and_box_drags_select_many() {
    let mut harness = quick_harness();
    select_cube(&mut harness);
    // A cube cut into two each way: its islands hold several polygons.
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetPrimitive {
            ids: vec![id],
            primitive: scene::Primitive::Cube(mesh::CubeParams {
                segments: [2, 2, 2],
                ..Default::default()
            }),
        })
        .unwrap();
    harness.state_mut().workbench.toggle_uv_editor();
    harness.state_mut().workbench.mode = Mode::Polygons;
    harness.run();
    harness.run();
    let (_, uv) = middle_of(&harness, glam::Vec3::Z);
    let at = harness.state().workbench.uv_editor.screen_of(uv).unwrap();
    click_at(&mut harness, at, Modifiers::NONE);
    assert_eq!(selected_polygons(&harness), 1);
    // A box around the whole texture square selects every polygon.
    let editor = harness.state().workbench.uv_editor.clone();
    let a = editor.screen_of(glam::Vec2::new(-0.02, -0.02)).unwrap();
    let b = editor.screen_of(glam::Vec2::new(1.02, 1.02)).unwrap();
    harness.event(Event::PointerMoved(a));
    harness.step();
    harness.event(Event::PointerButton {
        pos: a,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(a + (b - a) * (i as f32 / 8.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: b,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.run();
    assert_eq!(selected_polygons(&harness), 24);
}

#[test]
fn h_and_z_frame_in_the_editor_when_the_mouse_is_over_it() {
    let mut harness = quick_harness();
    select_cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.run();
    let camera_before = *harness.state().workbench.viewport().camera();
    let over = harness
        .get_by_label(crate::uv_editor::UV_EDITOR_LABEL)
        .rect()
        .center();
    harness.event(Event::PointerMoved(over));
    harness.run();
    let before = harness
        .state()
        .workbench
        .uv_editor
        .screen_of(glam::Vec2::ONE)
        .unwrap();
    harness.key_press(Key::Z);
    harness.run();
    let after = harness
        .state()
        .workbench
        .uv_editor
        .screen_of(glam::Vec2::ONE)
        .unwrap();
    assert_ne!(before, after, "Z framed the cube's UVs");
    // The 3D view stayed where it was.
    assert_eq!(
        *harness.state().workbench.viewport().camera(),
        camera_before
    );
}

#[test]
fn the_display_menu_switches_the_surface_to_checker_and_stretch() {
    let mut harness = fresh();
    for (label, wanted) in [
        ("Checker", crate::viewport::Surface::Checker),
        ("Stretch", crate::viewport::Surface::Stretch),
        ("Colours", crate::viewport::Surface::Plain),
    ] {
        harness.get_by_label("Display").click();
        harness.run();
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(harness.state().workbench.view_options().surface, wanted);
    }
}
