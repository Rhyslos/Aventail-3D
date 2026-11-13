//! The preview's options (decision 0291), used through the real
//! interface: turning, moving and zooming the picture as the 3D view
//! does, Shift + right drag turning the light, the light and backdrop
//! menu, comparing with a picture kept from before or with another
//! graph, the slider and side by side, and Flat zoomed out to see the
//! material from afar. Turning never works the graph out or lays the
//! material again.

use super::material_preview_tests::{H, click, last_preview, page, says};
use crate::material::comparing::{CompareHow, CompareWith};
use crate::material::preview::PREVIEW_PICTURE;
use crate::material::preview_options::{
    COMPARE_MENU, KEEP_BEFORE, LIGHT_MENU, RESET_VIEW, SIDE_BY_SIDE,
};
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use matgraph::{Graph, GraphEdit, Registry};

/// The picture's place on screen.
fn picture(harness: &H) -> egui::Rect {
    harness.get_by_label(PREVIEW_PICTURE).rect()
}

/// A drag with `button` across the picture's middle by `by`, holding
/// `modifiers`.
fn drag(harness: &mut H, button: PointerButton, by: egui::Vec2, modifiers: Modifiers) {
    let from = picture(harness).center();
    drag_from(harness, from, button, by, modifiers);
}

fn drag_from(
    harness: &mut H,
    from: Pos2,
    button: PointerButton,
    by: egui::Vec2,
    modifiers: Modifiers,
) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button,
        pressed: true,
        modifiers,
    });
    harness.step();
    for i in 1..=8 {
        harness.event(Event::PointerMoved(from + by * (i as f32 / 8.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: from + by,
        button,
        pressed: false,
        modifiers,
    });
    harness.step();
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

/// The wheel turned `lines` over the picture (positive: away from you).
fn wheel(harness: &mut H, lines: f32) {
    harness.event(Event::PointerMoved(picture(harness).center()));
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, lines),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

/// Whether two previews' laid textures differ anywhere (their colour or
/// their roughness, metal and height).
fn differ(a: &crate::ViewportView, b: &crate::ViewportView) -> bool {
    let (a, b) = (&a.scene.textures[0], &b.scene.textures[0]);
    let size = a.image.size();
    (0..size).any(|y| {
        (0..size).any(|x| {
            a.image.pixel(x, y) != b.image.pixel(x, y)
                || a.realism.as_ref().map(|r| r.surface.pixel(x, y))
                    != b.realism.as_ref().map(|r| r.surface.pixel(x, y))
        })
    })
}

/// The last picture asked for in renderer slot `slot`.
fn last_in(harness: &H, slot: usize) -> Option<crate::ViewportView> {
    harness
        .state()
        .renderer
        .previews
        .iter()
        .rev()
        .find(|v| v.slot == slot)
        .cloned()
}

fn editor(harness: &mut H) -> &mut crate::material::MaterialEditor {
    harness.state_mut().workbench.material()
}

/// Opens a menu above the picture and clicks `entry` in it.
fn menu(harness: &mut H, title: &str, entry: &str) {
    harness.get_by_label_contains(title).click();
    harness.run();
    harness.get_by_label(entry).click();
    harness.run();
}

#[test]
fn a_right_drag_turns_the_ball_and_makes_no_work() {
    let mut harness = page();
    let before = last_preview(&harness);
    let runs = harness.state().workbench.material.pictures.counts.runs;
    let versions = harness.state().workbench.material.preview.versions;
    let ball = crate::material::preview::PreviewShape::Ball;
    assert!(!editor(&mut harness).preview.turning.moved(ball));
    drag(
        &mut harness,
        PointerButton::Secondary,
        egui::vec2(80.0, 20.0),
        Modifiers::NONE,
    );
    let after = last_preview(&harness);
    assert_ne!(after.view_projection, before.view_projection, "turned");
    assert_ne!(after.camera_position, before.camera_position);
    // The same scene drawn from elsewhere: nothing worked out or laid.
    assert!(std::sync::Arc::ptr_eq(&after.scene, &before.scene));
    assert_eq!(
        harness.state().workbench.material.pictures.counts.runs,
        runs
    );
    assert_eq!(
        harness.state().workbench.material.preview.versions,
        versions
    );
    // Middle drag moves it; Reset the view goes back.
    drag(
        &mut harness,
        PointerButton::Middle,
        egui::vec2(30.0, 0.0),
        Modifiers::NONE,
    );
    assert_ne!(last_preview(&harness).focus, before.focus, "moved");
    click(&mut harness, RESET_VIEW);
    harness.run();
    let reset = last_preview(&harness);
    assert_eq!(reset.view_projection, before.view_projection);
    assert!(std::sync::Arc::ptr_eq(&reset.scene, &before.scene));
}

#[test]
fn alt_and_the_left_button_turn_it_too_when_that_setting_is_on() {
    let mut harness = page();
    let before = last_preview(&harness).view_projection;
    drag(
        &mut harness,
        PointerButton::Primary,
        egui::vec2(60.0, 0.0),
        Modifiers::ALT,
    );
    assert_ne!(last_preview(&harness).view_projection, before);
    // Switched off: Alt + left does nothing to the preview.
    editor(&mut harness).preview.turning.reset();
    harness
        .state_mut()
        .workbench
        .viewport
        .options_mut()
        .navigation
        .alt_everywhere = false;
    harness.run();
    let reset = last_preview(&harness).view_projection;
    assert_eq!(reset, before);
    drag(
        &mut harness,
        PointerButton::Primary,
        egui::vec2(60.0, 0.0),
        Modifiers::ALT,
    );
    assert_eq!(last_preview(&harness).view_projection, before);
}

#[test]
fn the_wheel_zooms_the_picture_not_the_panel_and_says_how_far() {
    let mut harness = page();
    let before = last_preview(&harness).camera_position;
    assert!(!says(&harness, "Seen from"));
    wheel(&mut harness, -3.0);
    let after = last_preview(&harness).camera_position;
    assert!(
        after.length() > before.length(),
        "further away: {before} → {after}"
    );
    assert!(says(&harness, "Seen from"));
    assert!(says(&harness, "m away"));
}

#[test]
fn shift_right_drag_turns_the_light_and_unlit_says_why_not() {
    let mut harness = page();
    assert_eq!(last_preview(&harness).light_turn, 0.0);
    let camera = last_preview(&harness).view_projection;
    drag(
        &mut harness,
        PointerButton::Secondary,
        egui::vec2(50.0, 0.0),
        Modifiers::SHIFT,
    );
    let view = last_preview(&harness);
    assert!((view.light_turn - 30.0).abs() < 0.5, "{}", view.light_turn);
    assert_eq!(view.view_projection, camera, "the light, not the camera");
    // Unlit has no light: nothing turns, and the status bar says why.
    click(&mut harness, "Unlit");
    drag(
        &mut harness,
        PointerButton::Secondary,
        egui::vec2(50.0, 0.0),
        Modifiers::SHIFT,
    );
    assert!((last_preview(&harness).light_turn - 30.0).abs() < 0.5);
    let note = harness.state().workbench.note.clone().unwrap_or_default();
    assert!(note.contains("Unlit has no light to turn"), "{note}");
}

#[test]
fn the_light_and_backdrop_menu_sets_the_view() {
    let mut harness = page();
    menu(&mut harness, LIGHT_MENU, "Dusk");
    let view = last_preview(&harness);
    assert_eq!(view.look, crate::viewport::SceneLook::DUSK);
    assert_eq!(view.sky_top, crate::viewport::SceneLook::DUSK.backdrop()[0]);
    menu(&mut harness, LIGHT_MENU, "Even grey");
    let [top, horizon, ground] = [
        last_preview(&harness).sky_top,
        last_preview(&harness).sky_horizon,
        last_preview(&harness).ground,
    ];
    assert!(top == horizon && horizon == ground);
    // The realistic look's surroundings and exposure.
    click(&mut harness, "Realistic");
    menu(&mut harness, LIGHT_MENU, "Outdoors");
    assert_eq!(
        last_preview(&harness).environment,
        crate::viewport::Environment::Sky
    );
}

#[test]
fn keeping_a_picture_as_before_compares_it_with_the_material_now() {
    let mut harness = page();
    assert!(last_in(&harness, 1).is_none());
    menu(&mut harness, COMPARE_MENU, KEEP_BEFORE);
    harness.run();
    let comparing = &editor(&mut harness).preview.comparing;
    assert_eq!(comparing.with, CompareWith::Before);
    assert!(comparing.before.is_some());
    let kept = last_in(&harness, 1).expect("the kept picture is drawn too");
    assert!(says(&harness, "Compare: Before"));
    // Change the graph: now differs from before, which stays.
    let id = harness.state().workbench.material.graph.unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id,
            edit: GraphEdit::SetSetting {
                node: matgraph::BoxId(1),
                key: "scale".into(),
                value: matgraph::SettingValue::Whole(9),
            },
        })
        .unwrap();
    for _ in 0..4 {
        harness.run();
    }
    let now = last_in(&harness, 0).unwrap();
    let before = last_in(&harness, 1).unwrap();
    assert!(std::sync::Arc::ptr_eq(&before.scene, &kept.scene), "kept");
    assert!(!std::sync::Arc::ptr_eq(&now.scene, &before.scene));
    assert!(differ(&now, &before));
    // Both turn together.
    drag(
        &mut harness,
        PointerButton::Secondary,
        egui::vec2(40.0, 0.0),
        Modifiers::NONE,
    );
    assert_eq!(
        last_in(&harness, 0).unwrap().view_projection,
        last_in(&harness, 1).unwrap().view_projection
    );
    // A left drag moves the slider's line.
    let rect = picture(&harness);
    drag_from(
        &mut harness,
        rect.center(),
        PointerButton::Primary,
        egui::vec2(-rect.width() * 0.3, 0.0),
        Modifiers::NONE,
    );
    let split = editor(&mut harness).preview.comparing.split;
    assert!((split - 0.2).abs() < 0.05, "{split}");
    // Side by side: two pictures, half as wide.
    menu(&mut harness, COMPARE_MENU, SIDE_BY_SIDE);
    assert_eq!(
        editor(&mut harness).preview.comparing.how,
        CompareHow::SideBySide
    );
    let half = last_in(&harness, 0).unwrap().size_px;
    assert!(half[0] < now.size_px[0], "{half:?} < {:?}", now.size_px);
    // Nothing: the second picture is no longer asked for.
    menu(&mut harness, COMPARE_MENU, "Nothing");
    harness.state_mut().renderer.previews.clear();
    harness.run();
    assert!(last_in(&harness, 1).is_none());
    assert!(last_in(&harness, 0).is_some());
}

/// A second graph, Planks, to compare with.
fn with_planks(harness: &mut H) -> matgraph::GraphId {
    let registry = Registry::standard();
    let mut graph = Graph::new("Planks");
    for kind in ["planks", "material_from_height", "material_out"] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: glam::Vec2::ZERO,
        };
        graph.apply(registry, &edit).unwrap();
    }
    for (from, out, to, input) in [(1, "grain", 2, "height"), (2, "material", 3, "material")] {
        graph
            .apply(
                registry,
                &GraphEdit::Connect {
                    from: matgraph::PlugRef::new(matgraph::BoxId(from), out),
                    to: matgraph::PlugRef::new(matgraph::BoxId(to), input),
                },
            )
            .unwrap();
    }
    let document = harness.state_mut().workbench.document_mut();
    document.execute(scene::Command::AddGraph(graph)).unwrap();
    let graphs = document.scene().graphs();
    graphs.iter().map(|(id, _)| id).max().unwrap()
}

#[test]
fn another_graph_is_worked_out_and_shown_beside_this_one() {
    let mut harness = page();
    // One graph only: the menu says there's nothing to compare with.
    harness.get_by_label_contains(COMPARE_MENU).click();
    harness.run();
    assert!(says(&harness, "This project has no other graph"));
    harness.event(Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let planks = with_planks(&mut harness);
    harness.run();
    menu(&mut harness, COMPARE_MENU, "Planks");
    for _ in 0..4 {
        harness.run();
    }
    assert_eq!(
        editor(&mut harness).preview.comparing.with,
        CompareWith::Graph(planks)
    );
    let theirs = last_in(&harness, 1).expect("Planks drawn");
    let mine = last_in(&harness, 0).unwrap();
    assert!(differ(&theirs, &mine));
    assert!(says(&harness, "Compare: Planks"));
    // Working on this graph doesn't work Planks out again.
    let runs = |h: &mut H| editor(h).preview.comparing.pictures.counts.runs;
    let before = runs(&mut harness);
    drag(
        &mut harness,
        PointerButton::Secondary,
        egui::vec2(40.0, 0.0),
        Modifiers::NONE,
    );
    assert_eq!(runs(&mut harness), before);
    // Planks removed (undone): back to the material alone.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    harness.run();
    assert_eq!(
        editor(&mut harness).preview.comparing.with,
        CompareWith::Nothing
    );
}

#[test]
fn flat_zooms_out_to_show_the_material_from_afar() {
    let mut harness = page();
    click(&mut harness, "Flat");
    assert!(!says(&harness, "of the material across"));
    wheel(&mut harness, -6.0);
    let repeats = editor(&mut harness).preview.turning.flat_repeats;
    assert!(repeats > 2.5, "{repeats}");
    assert!(says(&harness, "of the material across"));
    // The seam check keeps its three by three.
    click(&mut harness, crate::material::preview::SEAM_CHECK);
    assert!(!says(&harness, "of the material across"));
    assert!(says(&harness, "No joins show"));
    // Reset the view: two repeats again.
    click(&mut harness, crate::material::preview::SEAM_CHECK);
    click(&mut harness, RESET_VIEW);
    assert_eq!(
        editor(&mut harness).preview.turning.flat_repeats,
        crate::material::turning::FLAT_REPEATS
    );
}

#[test]
fn the_main_viewport_turns_its_light_only_in_realistic() {
    let mut harness = page();
    harness.state_mut().workbench.mode = crate::palette::Mode::Object;
    let options = harness.state_mut().workbench.viewport.options_mut();
    options.surface = crate::viewport::Surface::Painted;
    options.paint_preview = crate::PaintPreview::Lit;
    options.light_turn = 90.0;
    harness.run();
    let view = harness.state().renderer.drawn.last().unwrap();
    assert_eq!(view.light_turn, 0.0, "Lit keeps its light where it was");
}
