//! Regression tests for what the end-to-end reviews of the texture painter
//! found (2026-10-29).

use super::brushing_tests::stroke;
use super::paint_layers_tests::{painting_the_cube, press};
use super::tests::camera;
use crate::attributes::{PaintTool, SizeSpace};
use crate::palette::Mode;
use crate::transforming::Tool;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use paint::BrushKind;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

fn strokes(harness: &H) -> usize {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .paint()
        .sets()
        .map(|(_, s)| s.layers.iter().map(|l| l.strokes.len()).sum::<usize>())
        .sum()
}

#[test]
fn a_press_over_a_panel_never_paints_where_the_mouse_moves_to() {
    // When the app is slow, a press over a panel and the move into the
    // view arrive in one frame: that press mustn't start a stroke.
    let mut harness = painting_the_cube();
    let panel = harness.get_by_label("Realism channels").rect().center();
    let over_cube = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    harness.event(Event::PointerMoved(panel));
    harness.event(Event::PointerButton {
        pos: panel,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(Event::PointerMoved(over_cube));
    harness.step();
    harness.event(Event::PointerButton {
        pos: over_cube,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(strokes(&harness), 0, "no paint");
    // A press over the view still paints.
    stroke(
        &mut harness,
        over_cube,
        over_cube,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert_eq!(strokes(&harness), 1);
}

#[test]
fn esc_closing_a_menu_stays_in_paint_mode() {
    let mut harness = painting_the_cube();
    harness.get_by_label("Edit").click();
    harness.run();
    assert!(egui::Popup::is_any_open(&harness.ctx));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(
        harness.state().workbench.mode,
        Mode::Paint,
        "the menu closed, painting stays"
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(
        harness.state().workbench.mode,
        Mode::Model,
        "a second Esc leaves"
    );
}

#[test]
fn the_fill_tool_fills_even_straight_after_the_eraser() {
    let mut harness = painting_the_cube();
    harness.key_press(egui::Key::Num2);
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.kind,
        BrushKind::Erase
    );
    harness.key_press(egui::Key::Num4);
    harness.run();
    let brush = harness.state().workbench.attributes.brush;
    assert_eq!(brush.tool, PaintTool::Fill);
    assert_eq!(brush.brush.kind, BrushKind::Paint, "fills, doesn't erase");
    // The same from the tool row.
    harness.key_press(egui::Key::Num2);
    harness.run();
    harness.get_by_label("Polygon fill (4)").click();
    harness.run();
    assert_eq!(
        harness.state().workbench.attributes.brush.brush.kind,
        BrushKind::Paint
    );
}

#[test]
fn erasing_on_a_fill_layer_makes_no_empty_layer_and_says_why() {
    let mut harness = painting_the_cube();
    press(&mut harness, "+ Fill");
    let layers = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .map_or(0, |s| s.layers.len())
    };
    let before = layers(&harness);
    harness.key_press(egui::Key::Num2);
    harness.run();
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert_eq!(layers(&harness), before, "no new empty layer");
    let note = harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string();
    assert!(note.starts_with("Nothing to erase here"), "{note}");
}

#[test]
fn painting_puts_a_modelling_tool_down() {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.tool = Tool::Bevel;
    harness.run();
    assert_eq!(harness.state().workbench.tool, Tool::Select);
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let note = harness
        .state()
        .workbench
        .note()
        .unwrap_or_default()
        .to_string();
    assert!(!note.starts_with("Bevel works"), "{note}");
    // The gizmos are fine to keep.
    harness.state_mut().workbench.tool = Tool::Move;
    harness.run();
    assert_eq!(harness.state().workbench.tool, Tool::Move);
}

#[test]
fn the_bars_mirror_list_chooses_the_axis() {
    let mut harness = painting_the_cube();
    harness.get_by_label("Mirror axis").click();
    harness.run();
    harness.get_by_label("Z (up and down)").click();
    harness.run();
    harness.run();
    let brush = harness.state().workbench.attributes.brush;
    assert_eq!(brush.mirror_axis, crate::attributes::MirrorAxis::Z);
    assert!(brush.mirror_on);
}

#[test]
fn a_texture_measured_brush_needs_a_texture_and_falls_back_on_screen() {
    // Vertex colours have no texture: measured on the texture, the brush
    // is measured as on screen there.
    let settings = crate::attributes::BrushSettings {
        size_space: SizeSpace::Texture,
        ..Default::default()
    };
    assert_eq!(
        settings.world_radius(0.5, None),
        settings.brush.radius * 0.5
    );
    assert_eq!(
        settings.world_radius(0.5, Some(10.0)),
        settings.texture_radius / 10.0
    );
}

#[test]
fn subdivide_keeps_the_paint_where_it_was() {
    // Subdivide makes the shape afresh with new polygon numbers: the
    // paint used to be squeezed onto the wrong polygons (the new ones
    // that happened to have the old ones' numbers) and mostly lost.
    let mut harness = painting_the_cube();
    let from = on_screen(&harness, Vec3::new(-30.0, -50.0, 50.0));
    let to = on_screen(&harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(
        &mut harness,
        from,
        to,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    // Whether the paint covers each of many places all over the cube
    // (a grid on each side, pulled in from the corners, which Subdivide
    // rounds off).
    let painted_at = |h: &H| -> Vec<bool> {
        let doc = h.state().workbench.document();
        let id = doc.scene().walk()[0].0;
        let mesh = &doc.scene().evaluated(id).unwrap().mesh;
        let surface = paint::PaintSurface::of(mesh);
        let set = doc.scene().paint().set("Default").unwrap();
        let image = set.layers.last().and_then(|l| l.image()).unwrap();
        let size = image.size() as f32;
        let mut out = Vec::new();
        for axis in 0..3 {
            for side in [-1.0f32, 1.0] {
                for a in 0..7 {
                    for b in 0..7 {
                        let (u, v) = (-30.0 + 10.0 * a as f32, -30.0 + 10.0 * b as f32);
                        let mut p = [u, v, u];
                        p[axis] = 50.0 * side;
                        p[(axis + 1) % 3] = u;
                        p[(axis + 2) % 3] = v;
                        let at = Vec3::new(p[0], p[1], p[2] + 50.0);
                        let Some((i, w)) = surface.closest(at, &[0]) else {
                            out.push(false);
                            continue;
                        };
                        let t = &surface.triangles()[i];
                        let uv = t.uvs[0] * w[0] + t.uvs[1] * w[1] + t.uvs[2] * w[2];
                        let px = ((uv.x * size) as u32).min(image.size() - 1);
                        let py = (((1.0 - uv.y) * size) as u32).min(image.size() - 1);
                        out.push(image.pixel(px, py)[3] > 128);
                    }
                }
            }
        }
        out
    };
    let before = painted_at(&harness);
    assert!(before.iter().filter(|p| **p).count() >= 10);
    let id = harness.state().workbench.document().scene().walk()[0].0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::AddModifier {
            ids: vec![id],
            kind: scene::ModifierKind::Subdivide(Default::default()),
        })
        .unwrap();
    harness.run_steps(8);
    let after = painted_at(&harness);
    let differ = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    assert!(
        differ <= before.len() / 20,
        "{differ} of {} places changed",
        before.len()
    );
}

#[test]
fn subdivide_off_again_gives_the_paint_back_exactly() {
    // Second review: turning Subdivide off moved the paint a second time,
    // which took about a minute on a dense shape and blurred it. Going
    // back to the layout the paint moved from, untouched since, now gives
    // the earlier paint back as it was.
    let mut harness = painting_the_cube();
    let from = on_screen(&harness, Vec3::new(-30.0, -50.0, 50.0));
    let to = on_screen(&harness, Vec3::new(30.0, -50.0, 50.0));
    stroke(
        &mut harness,
        from,
        to,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    harness.run_steps(4);
    let pixels = |h: &H| {
        let set = h
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .set("Default")
            .unwrap();
        set.layers.last().and_then(|l| l.image()).unwrap().clone()
    };
    let before = pixels(&harness);
    let id = harness.state().workbench.document().scene().walk()[0].0;
    let run = |h: &mut H, command| {
        h.state_mut()
            .workbench
            .document_mut()
            .execute(command)
            .unwrap();
        h.run_steps(8);
    };
    run(
        &mut harness,
        scene::Command::AddModifier {
            ids: vec![id],
            kind: scene::ModifierKind::Subdivide(Default::default()),
        },
    );
    assert!(pixels(&harness) != before, "the paint moved with Subdivide");
    run(
        &mut harness,
        scene::Command::RemoveModifier { id, index: 0 },
    );
    assert!(pixels(&harness) == before, "the paint came back as it was");
}

#[test]
fn a_release_and_press_in_one_frame_make_two_strokes() {
    // Second review: when the app was slow, a release and the next press
    // came in one frame, the button looked held throughout, and the two
    // strokes were joined.
    let mut harness = painting_the_cube();
    let at = |x: f32| on_screen(&harness, Vec3::new(x, -50.0, 50.0));
    let (a, b, c, d) = (at(-30.0), at(-10.0), at(10.0), at(30.0));
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.event(Event::PointerMoved(a));
    harness.step();
    harness.event(button(a, true));
    harness.step();
    harness.event(Event::PointerMoved(b));
    harness.step();
    // One frame: let go at b, move to c, press again.
    // (`event` gives each event a frame of its own; these go in together.)
    harness
        .input_mut()
        .events
        .extend([button(b, false), Event::PointerMoved(c), button(c, true)]);
    harness.step();
    harness.event(Event::PointerMoved(d));
    harness.step();
    harness.event(button(d, false));
    harness.step();
    harness.run_steps(4);
    assert_eq!(strokes(&harness), 2);
}
