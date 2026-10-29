//! Painting in the UV Editor (decision 0180): the same brush, flat on the
//! texture, only inside the UV pieces, one undo step a stroke.

use super::LayoutChoice;
use super::brushing_tests::{painted_pixels, painting, stroke};
use crate::palette::Mode;
use egui::{Modifiers, PointerButton, Pos2};
use glam::Vec2;
use paint::PaintSurface;

type H = egui_kittest::Harness<'static, super::tests::State>;

/// Paint mode in the UV Edit layout, the cube's UVs in view.
pub(super) fn painting_in_the_editor() -> H {
    let mut harness = painting();
    let cube = harness.state().workbench.document().scene().walk()[0].0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([cube]);
    harness
        .state_mut()
        .workbench
        .switch_layout(LayoutChoice::UvEdit);
    harness.run();
    harness.state_mut().workbench.mode = Mode::Paint;
    harness.run();
    harness.run();
    harness
}

/// The cube's shape as the brush sees it.
pub(super) fn cube_surface(harness: &H) -> PaintSurface {
    let doc = harness.state().workbench.document();
    let id = crate::uv_editor::shown_objects(doc)[0];
    PaintSurface::of(&doc.scene().evaluated(id).unwrap().mesh)
}

/// The middle of the cube's first UV piece, and a place on the texture
/// with no piece.
fn inside_and_outside(harness: &H) -> (Vec2, Vec2) {
    let surface = cube_surface(harness);
    let t = surface.triangles()[0];
    let inside = (t.uvs[0] + t.uvs[1] + t.uvs[2]) / 3.0;
    let outside = (0..40)
        .flat_map(|y| (0..40).map(move |x| Vec2::new(x as f32 + 0.5, y as f32 + 0.5) / 40.0))
        .find(|uv| {
            // Clear of every piece by a good margin.
            [Vec2::ZERO, Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y]
                .iter()
                .all(|d| paint::flat::slot_at(&surface, *uv + *d * 0.04).is_none())
        })
        .expect("the cube's layout leaves some of the texture empty");
    (inside, outside)
}

pub(super) fn on_editor(harness: &H, uv: Vec2) -> Pos2 {
    harness
        .state()
        .workbench
        .uv_editor
        .screen_of(uv)
        .expect("the editor is shown")
}

#[test]
fn a_stroke_in_the_uv_editor_paints_flat_in_one_undo_step() {
    let mut harness = painting_in_the_editor();
    let (inside, _) = inside_and_outside(&harness);
    let from = on_editor(&harness, inside);
    stroke(
        &mut harness,
        from,
        from + egui::vec2(8.0, 0.0),
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let painted = painted_pixels(&harness);
    assert!(painted > 50, "{painted}");
    let doc = harness.state().workbench.document();
    let set = doc.scene().paint().set("Default").unwrap();
    let strokes = &set.layers[0].strokes;
    assert_eq!(strokes.len(), 1);
    assert!(!strokes[0].dabs.is_empty());
    assert!(
        strokes[0]
            .dabs
            .iter()
            .all(|d| d.flat_uv().is_some_and(|uv| uv.distance(inside) < 0.1)),
        "the dabs are kept as places on the texture"
    );
    // Nothing was selected by the drag.
    let id = crate::uv_editor::shown_objects(doc)[0];
    assert!(doc.elements(id).is_none_or(|e| e.is_empty()));
    // One stroke, one step.
    harness.state_mut().workbench.document_mut().undo().unwrap();
    assert_eq!(painted_pixels(&harness), 0);
}

#[test]
fn painting_beside_the_pieces_paints_nothing() {
    let mut harness = painting_in_the_editor();
    let (_, outside) = inside_and_outside(&harness);
    let at = on_editor(&harness, outside);
    stroke(
        &mut harness,
        at,
        at + egui::vec2(2.0, 0.0),
        Modifiers::NONE,
        PointerButton::Primary,
    );
    assert_eq!(painted_pixels(&harness), 0);
    assert!(harness.state().workbench.note.is_some());
}

#[test]
fn ctrl_drags_in_the_uv_editor_change_the_brush_without_moving_the_view() {
    let mut harness = painting_in_the_editor();
    let (inside, _) = inside_and_outside(&harness);
    let at = on_editor(&harness, inside);
    let radius = |h: &H| h.state().workbench.attributes.brush.brush.radius;
    let before = radius(&harness);
    stroke(
        &mut harness,
        at,
        at + egui::vec2(80.0, 0.0),
        Modifiers::COMMAND,
        PointerButton::Secondary,
    );
    assert!(radius(&harness) > before);
    assert_eq!(on_editor(&harness, inside), at, "the view hasn't moved");
    assert_eq!(painted_pixels(&harness), 0);
}

#[test]
fn a_brush_measured_on_the_surface_is_the_same_size_in_the_uv_editor() {
    let mut harness = painting_in_the_editor();
    let (inside, _) = inside_and_outside(&harness);
    {
        let brush = &mut harness.state_mut().workbench.attributes.brush;
        brush.size_space = crate::attributes::SizeSpace::Object;
        brush.surface_radius = 10.0;
        brush.brush.hardness = 1.0;
    }
    let at = on_editor(&harness, inside);
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    // As on the model: 10 cm on a 100 cm side laid out in thirds of the
    // texture.
    let painted = painted_pixels(&harness) as f32;
    let size = harness.state().workbench.document().scene().texture().size as f32;
    let expected = std::f32::consts::PI * (size / 3.0 * 10.0 / 100.0).powi(2);
    assert!(
        painted > expected * 0.6 && painted < expected * 1.6,
        "{painted} against {expected}"
    );
}
