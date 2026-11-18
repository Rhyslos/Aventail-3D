//! Tests for pens, strokes and alphas in Sculpt mode (decision 0308): a
//! Line stroke goes straight whatever way the mouse wanders, a stamp is
//! sized and turned by its drag and matches its alpha, pen pressure
//! makes a lighter stroke, D turns smoothing on and off, the shelf's
//! Alphas tab, and your own alphas kept in the library.

use super::heavy_check::H;
use super::sculpt_tests::{front, shape};
use super::tests::{camera, harness_for, info};
use super::*;
use crate::sculpt_tab::StrokeKind;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use scene::Mesh;
use sculpt::BrushKind;
use sculpt::alpha::Alpha;
use std::sync::Arc;

/// A stroke through `points` with the left button, the first pressed and
/// the last let go.
fn drag(harness: &mut H, points: &[Pos2]) {
    let (first, last) = (points[0], points[points.len() - 1]);
    harness.event(Event::PointerMoved(first));
    harness.step();
    harness.event(Event::PointerButton {
        pos: first,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for pair in points.windows(2) {
        for i in 1..=4 {
            harness.event(Event::PointerMoved(pair[0] + (pair[1] - pair[0]) * (i as f32 / 4.0)));
            harness.step();
        }
    }
    harness.event(Event::PointerButton {
        pos: last,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.run();
}

/// The sphere's point seen at `at` on screen (the nearest of those
/// within a few points of it).
fn under(harness: &H, mesh: &Mesh, at: Pos2) -> scene::VertexId {
    let camera = camera(harness);
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(view.width(), view.height());
    let forward = camera.forward();
    mesh.vertices()
        .filter(|(_, p)| {
            camera
                .project(*p, size)
                .is_some_and(|s| (view.min + egui::vec2(s.x, s.y)).distance(at) < 6.0)
        })
        .min_by(|a, b| a.1.dot(forward).total_cmp(&b.1.dot(forward)))
        .map(|(v, _)| v)
        .expect("a point under the mouse")
}

/// How far the point seen at `at` moved between `before` and now.
fn moved(harness: &H, before: &Mesh, at: Pos2) -> f32 {
    let v = under(harness, before, at);
    let now = shape(harness).position(v).unwrap();
    now.distance(before.position(v).unwrap())
}

/// A window sculpting a dense sphere filling the view, with Draw at full
/// strength, a small brush and stroke `way`.
fn sculpting_with(way: StrokeKind) -> H {
    let mut workbench = Workbench::new(info(), None);
    let sphere = mesh::Primitive::Sphere(mesh::primitives::SphereParams {
        radius: 50.0,
        sides: 160,
        rings: 80,
    });
    workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(sphere)))
        .unwrap();
    let mut harness = harness_for(workbench);
    harness.run();
    harness.state_mut().workbench.mode = Mode::Sculpt;
    harness.run();
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let bounds = shape(&harness).bounds().unwrap();
    let navigator = harness.state().workbench.viewport().navigator().framed(&bounds, view.aspect_ratio());
    *harness.state_mut().workbench.viewport_mut().navigator_mut() = navigator;
    let settings = &mut harness.state_mut().workbench.attributes.sculpt;
    settings.brush = BrushKind::Draw;
    settings.stroke = way;
    let draw = settings.of_mut(BrushKind::Draw);
    draw.radius = 10.0;
    draw.strength = 1.0;
    harness.run();
    harness
}

#[test]
fn a_line_goes_straight_whichever_way_the_mouse_wanders() {
    for way in [StrokeKind::Freehand, StrokeKind::Line] {
        let mut harness = sculpting_with(way);
        let (_, c) = front(&harness);
        let before = shape(&harness);
        let (a, b, detour) = (
            c - egui::vec2(25.0, 0.0),
            c + egui::vec2(25.0, 0.0),
            c - egui::vec2(0.0, 35.0),
        );
        let steps = harness.state().workbench.document().undo_steps();
        drag(&mut harness, &[a, detour, b]);
        assert_eq!(
            harness.state().workbench.document().undo_steps(),
            steps + 1,
            "{way:?}: one undo step"
        );
        let off_line = moved(&harness, &before, detour);
        if way == StrokeKind::Line {
            // Straight from the press to the let go: the detour is
            // untouched and the whole way between is sculpted.
            assert_eq!(off_line, 0.0, "the line wandered");
            for at in [a, c, b] {
                assert!(moved(&harness, &before, at) > 0.0, "{at:?} not sculpted");
            }
        } else {
            assert!(off_line > 0.0, "freehand follows the mouse");
        }
    }
}

#[test]
fn escape_takes_back_a_line_being_dragged() {
    let mut harness = sculpting_with(StrokeKind::Line);
    let (_, c) = front(&harness);
    let before = shape(&harness);
    harness.event(Event::PointerMoved(c));
    harness.step();
    harness.event(Event::PointerButton {
        pos: c,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for i in 1..=4 {
        harness.event(Event::PointerMoved(c + egui::vec2(8.0 * i as f32, 0.0)));
        harness.step();
    }
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().workbench.sculpting.stroking());
    harness.event(Event::PointerButton {
        pos: c,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(shape(&harness), before);
    assert_eq!(harness.state().workbench.mode, Mode::Sculpt);
}

/// White on the left half, black on the right.
fn left_half() -> Arc<Alpha> {
    let mut values = vec![0u8; 16 * 16];
    for y in 0..16 {
        for x in 0..8 {
            values[y * 16 + x] = 255;
        }
    }
    Arc::new(Alpha::new(16, 16, values, "left").unwrap())
}

#[test]
fn a_stamp_is_sized_and_turned_by_its_drag() {
    let mut harness = sculpting_with(StrokeKind::Stamp);
    let (_, c) = front(&harness);
    let before = shape(&harness);
    // A click without a drag lays nothing.
    drag(&mut harness, &[c, c]);
    assert_eq!(shape(&harness), before);
    // Dragged 30 points right: a stamp that big at the press.
    drag(&mut harness, &[c, c + egui::vec2(30.0, 0.0)]);
    assert!(moved(&harness, &before, c) > 0.0);
    assert!(moved(&harness, &before, c + egui::vec2(15.0, 0.0)) > 0.0);
    assert_eq!(moved(&harness, &before, c + egui::vec2(45.0, 0.0)), 0.0);
    // With an alpha, its top along the drag: dragging right puts the
    // picture's (white) left half above on screen.
    let mut harness = sculpting_with(StrokeKind::Stamp);
    harness.state_mut().workbench.attributes.sculpt.alpha = Some(left_half());
    let steps = harness.state().workbench.document().undo_steps();
    drag(&mut harness, &[c, c + egui::vec2(30.0, 0.0)]);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    assert!(moved(&harness, &before, c - egui::vec2(0.0, 12.0)) > 0.0);
    assert_eq!(moved(&harness, &before, c + egui::vec2(0.0, 12.0)), 0.0);
    // Dragged left, it turns over: the white half lies below.
    let mut harness = sculpting_with(StrokeKind::Stamp);
    harness.state_mut().workbench.attributes.sculpt.alpha = Some(left_half());
    drag(&mut harness, &[c, c - egui::vec2(30.0, 0.0)]);
    assert_eq!(moved(&harness, &before, c - egui::vec2(0.0, 12.0)), 0.0);
    assert!(moved(&harness, &before, c + egui::vec2(0.0, 12.0)) > 0.0);
}

#[test]
fn a_freehand_stroke_with_an_alpha_keeps_its_black_parts_still() {
    let mut harness = sculpting_with(StrokeKind::Freehand);
    {
        let settings = &mut harness.state_mut().workbench.attributes.sculpt;
        settings.alpha = Some(left_half());
        settings.of_mut(BrushKind::Draw).radius = 20.0;
    }
    let (_, c) = front(&harness);
    let before = shape(&harness);
    // The picture's top points up the view: its white half is left.
    drag(&mut harness, &[c, c]);
    assert!(moved(&harness, &before, c - egui::vec2(10.0, 0.0)) > 0.0);
    assert_eq!(moved(&harness, &before, c + egui::vec2(10.0, 0.0)), 0.0);
}

fn pen(pressure: f32) -> Option<crate::PenState> {
    Some(crate::PenState {
        pressure,
        tilt: None,
        eraser: false,
    })
}

#[test]
fn a_lighter_press_sculpts_less() {
    // How far all the points moved together, for a dab with the tablet
    // pen at each pressure (or a mouse).
    let pushed: Vec<f32> = [None, pen(1.0), pen(0.3)]
        .into_iter()
        .map(|pen| {
            let mut harness = sculpting_with(StrokeKind::Freehand);
            harness
                .state_mut()
                .workbench
                .attributes
                .sculpt
                .of_mut(BrushKind::Draw)
                .radius = 30.0;
            harness.state_mut().workbench.set_tablet_pen(pen);
            let (_, c) = front(&harness);
            let before = shape(&harness);
            drag(&mut harness, &[c, c]);
            let after = shape(&harness);
            before
                .vertices()
                .map(|(v, p)| after.position(v).unwrap().distance(p))
                .sum()
        })
        .collect();
    assert!(pushed[0] > 0.0);
    assert!(
        (pushed[0] - pushed[1]).abs() < pushed[0] * 1e-4,
        "a full press is a mouse: {pushed:?}"
    );
    // Smaller and weaker: far less than a third.
    assert!(pushed[2] < pushed[1] * 0.3, "{pushed:?}");
    assert!(pushed[2] > 0.0, "{pushed:?}");
}

#[test]
fn d_turns_smoothing_on_and_off_without_extruding() {
    let mut harness = sculpting_with(StrokeKind::Freehand);
    let before = shape(&harness);
    harness.key_press(egui::Key::D);
    harness.run();
    let workbench = &harness.state().workbench;
    assert!(workbench.attributes.sculpt.smoothing_on);
    assert_eq!(workbench.mode, Mode::Sculpt);
    assert!(workbench.note.as_deref().unwrap().contains("Smoothing on"));
    harness.key_press(egui::Key::D);
    harness.run();
    assert!(!harness.state().workbench.attributes.sculpt.smoothing_on);
    assert_eq!(shape(&harness), before);
}

#[test]
fn smoothing_trails_the_mouse() {
    // With a long string, a short wiggle never pulls the brush along:
    // only the press sculpts.
    let mut harness = sculpting_with(StrokeKind::Freehand);
    {
        let settings = &mut harness.state_mut().workbench.attributes.sculpt;
        settings.smoothing_on = true;
        settings.smoothing = 1.0;
    }
    let (_, c) = front(&harness);
    let before = shape(&harness);
    let far = c + egui::vec2(40.0, 0.0);
    drag(&mut harness, &[c, far]);
    assert!(moved(&harness, &before, c) > 0.0);
    assert_eq!(moved(&harness, &before, far), 0.0, "the brush lagged behind");
}

#[test]
fn the_alphas_tab_picks_an_alpha_and_asks_for_your_own() {
    let mut harness = sculpting_with(StrokeKind::Freehand);
    harness.get_by_label(crate::sculpt_tab::ALPHAS_TAB).click();
    harness.run();
    let card = harness.get_by_label("Alpha Stones");
    card.click();
    harness.run();
    let chosen = harness.state().workbench.attributes.sculpt.alpha.clone();
    assert_eq!(
        chosen.and_then(|a| a.made()),
        Some(sculpt::alpha::Made::Stones)
    );
    harness.get_by_label(crate::sculpt_tab::NO_ALPHA).click();
    harness.run();
    assert!(harness.state().workbench.attributes.sculpt.alpha.is_none());
    harness.get_by_label(crate::sculpt_tab::ADD_ALPHA).click();
    harness.run();
    assert!(
        harness
            .state()
            .actions
            .contains(&WorkbenchAction::ChooseAlpha { path: None })
    );
}

/// A 20 × 20 TGA: a white dot on black.
fn dot_picture(path: &std::path::Path) {
    let n = 20u16;
    let mut tga = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    tga.extend_from_slice(&n.to_le_bytes());
    tga.extend_from_slice(&n.to_le_bytes());
    tga.extend_from_slice(&[24, 0x20]);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (f32::from(x) - 9.5, f32::from(y) - 9.5);
            let v = if (dx * dx + dy * dy).sqrt() < 6.0 { 255 } else { 0 };
            tga.extend_from_slice(&[v, v, v]);
        }
    }
    std::fs::write(path, tga).unwrap();
}

#[test]
fn your_own_alphas_are_kept_in_the_library_and_taken_out() {
    let dir = std::env::temp_dir().join(format!("sculpt-alphas-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let picture = dir.join("dot.tga");
    dot_picture(&picture);
    let library = dir.join("alphas");
    let mut harness = sculpting_with(StrokeKind::Freehand);
    harness.state_mut().workbench.info.alpha_library = Some(library.clone());
    harness.state_mut().workbench.bring_in_alpha(&picture);
    harness.run();
    let settings = &harness.state().workbench.attributes.sculpt;
    let chosen = settings.alpha.clone().expect("the new alpha is chosen");
    assert_eq!(chosen.name(), "dot");
    assert_eq!(settings.shelf_tab, crate::sculpt_tab::ShelfTab::Alphas);
    assert!(library.join("dot.png").exists());
    // On the shelf, and found again by a new window.
    assert!(harness.query_by_label("Alpha dot").is_some());
    let mut again = sculpting_with(StrokeKind::Freehand);
    again.state_mut().workbench.info.alpha_library = Some(library.clone());
    again.state_mut().workbench.attributes.sculpt.own_alphas = None;
    again.run();
    let own = again.state().workbench.attributes.sculpt.own_alphas.clone();
    assert_eq!(own.unwrap()[0], chosen);
    // A damaged picture is refused in plain words.
    let junk = dir.join("junk.png");
    std::fs::write(&junk, b"not a picture").unwrap();
    harness.state_mut().workbench.bring_in_alpha(&junk);
    assert!(harness.state().workbench.note.is_some());
    assert_eq!(
        harness
            .state()
            .workbench
            .attributes
            .sculpt
            .own_alphas
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    // Taken out: gone from the shelf, the library and the brush.
    harness.state_mut().workbench.forget_alpha("dot");
    harness.run();
    let settings = &harness.state().workbench.attributes.sculpt;
    assert!(settings.alpha.is_none());
    assert!(settings.own_alphas.as_ref().unwrap().is_empty());
    assert!(!library.join("dot.png").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_grabbing_brush_strokes_freehand_whatever_the_stroke_list_says() {
    let mut harness = sculpting_with(StrokeKind::Stamp);
    harness.state_mut().workbench.attributes.sculpt.brush = BrushKind::Grab;
    harness.run();
    let (_, c) = front(&harness);
    let before = shape(&harness);
    drag(&mut harness, &[c, c + egui::vec2(0.0, -30.0)]);
    let v = under(&harness, &before, c);
    let now = shape(&harness).position(v).unwrap();
    let pulled = (now - before.position(v).unwrap()).dot(camera(&harness).up());
    assert!(pulled > 0.0, "Grab pulled up: {pulled}");
    let _ = Vec3::ZERO;
}



