//! Vertex colour painting (decision 0205): Paint on: Vertex colours, a
//! stroke on the model paints the corners there as one undo step, the
//! fill tool fills a polygon's corners, and the views show the colours.

use super::brushing_tests::stroke;
use super::paint_layers_tests::painting_the_cube;
use super::tests::camera;
use crate::attributes::{PaintOn, PaintTool};
use crate::viewport::Surface;
use egui::{Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec3;

type H = egui_kittest::Harness<'static, super::tests::State>;

fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).unwrap();
    rect.min + egui::vec2(at.x, at.y)
}

/// The cube's corner colours that aren't white.
fn coloured(harness: &H) -> Vec<[u8; 4]> {
    let doc = harness.state().workbench.document();
    let id = doc.scene().walk()[0].0;
    let mesh = doc.scene().evaluated(id).unwrap().mesh;
    mesh::colours::colours(&mesh)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| *c != [255; 4])
        .collect()
}

fn painting_vertices() -> H {
    let mut harness = painting_the_cube();
    harness.state_mut().workbench.attributes.brush.paint_on = PaintOn::VertexColours;
    harness.state_mut().workbench.attributes.brush.brush.color = [255, 0, 0, 255];
    harness.state_mut().workbench.attributes.brush.brush.radius = 60.0;
    harness.run();
    harness
}

#[test]
fn a_stroke_paints_the_corners_near_it_in_one_step() {
    let mut harness = painting_vertices();
    assert_eq!(
        harness.state().workbench.view_options().surface,
        Surface::VertexColours,
        "the views show the vertex colours"
    );
    assert!(harness.query_by_label("Vertex colours").is_some());
    let a = on_screen(&harness, Vec3::new(-45.0, -50.0, 95.0));
    let b = on_screen(&harness, Vec3::new(45.0, -50.0, 95.0));
    stroke(&mut harness, a, b, Modifiers::NONE, PointerButton::Primary);
    let painted = coloured(&harness);
    assert!(!painted.is_empty(), "corners near the top edge painted");
    assert!(painted.iter().any(|c| c[0] == 255 && c[1] < 50));
    // No texture painted.
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .paint()
            .is_empty()
    );
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_label(), Some("Paint vertex colours"));
    harness.state_mut().workbench.document_mut().undo().unwrap();
    harness.run();
    assert!(coloured(&harness).is_empty());
    // Back to the texture: the views show the painting again.
    harness.state_mut().workbench.attributes.brush.paint_on = PaintOn::Texture;
    harness.run();
    assert_eq!(
        harness.state().workbench.view_options().surface,
        Surface::Painted
    );
}

#[test]
fn the_fill_tool_fills_a_polygons_corners() {
    let mut harness = painting_vertices();
    harness.state_mut().workbench.attributes.brush.tool = PaintTool::Fill;
    harness.state_mut().workbench.attributes.brush.fill_by = paint::FillBy::Polygon;
    let at = on_screen(&harness, Vec3::new(0.0, -50.0, 50.0));
    stroke(
        &mut harness,
        at,
        at,
        Modifiers::NONE,
        PointerButton::Primary,
    );
    let painted = coloured(&harness);
    assert_eq!(painted.len(), 4, "the front's four corners");
    assert!(painted.iter().all(|c| *c == [255, 0, 0, 255]));
}

#[test]
fn a_painted_texture_turns_into_vertex_colours() {
    let mut harness = painting_vertices();
    {
        let doc = harness.state_mut().workbench.document_mut();
        let material = scene::Material::standard().name;
        let mut set = paint::TextureSet::new(doc.scene().texture().size).unwrap();
        set.layers
            .push(paint::Layer::fill("Fill", [0, 0, 200, 255]));
        doc.execute(scene::Command::SetTextureSet {
            material,
            set: Some(set),
        })
        .unwrap();
        let id = doc.scene().walk()[0].0;
        doc.set_selection([id]);
    }
    harness.run();
    harness.get_by_label("From the texture").click();
    harness.run();
    harness.run();
    let painted = coloured(&harness);
    assert_eq!(painted.len(), 24, "every corner of the cube");
    assert!(painted.iter().all(|c| *c == [0, 0, 200, 255]));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Vertex colours from the texture")
    );
}
