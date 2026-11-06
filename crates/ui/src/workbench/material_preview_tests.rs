//! The Material mode's preview (decision 0260), used through the real
//! interface: the material on a ball, a cube and the selected model drawn
//! by the views' renderer in the chosen look, the ball drawn here without
//! a graphics card, the seam check, and the texture budget.

use super::tests::{TestRenderer, harness_built_sized, info, with_a_cube};
use super::*;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{Graph, GraphEdit, PlugRef, Registry};

type H = egui_kittest::Harness<'static, super::tests::State>;

/// The Material mode showing Noise → Material from height → Material
/// out, with a cube in the scene (not selected).
fn page() -> H {
    let mut workbench = with_a_cube(Workbench::new(info(), None));
    let registry = Registry::standard();
    let mut graph = Graph::new("Stone");
    for (kind, x) in [
        ("noise", 0.0),
        ("material_from_height", 300.0),
        ("material_out", 600.0),
    ] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, 0.0),
        };
        graph.apply(registry, &edit).unwrap();
    }
    for (from, out, to, input) in [(1, "height", 2, "height"), (2, "material", 3, "material")] {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(matgraph::BoxId(from), out),
            to: PlugRef::new(matgraph::BoxId(to), input),
        };
        graph.apply(registry, &edit).unwrap();
    }
    workbench
        .document_mut()
        .execute(scene::Command::AddGraph(graph))
        .unwrap();
    workbench.mode = Mode::Material;
    // Tall enough that the whole preview panel shows without scrolling.
    let mut harness = harness_built_sized(
        egui_kittest::Harness::builder(),
        workbench,
        TestRenderer::default(),
        [1440.0, 1800.0],
    );
    harness.run();
    harness.run();
    harness
}

fn click(harness: &mut H, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

fn last_preview(harness: &H) -> crate::ViewportView {
    harness
        .state()
        .renderer
        .previews
        .last()
        .expect("the preview asked to be drawn")
        .clone()
}

fn says(harness: &H, words: &str) -> bool {
    harness.query_by_label_contains(words).is_some()
}

#[test]
fn the_material_is_drawn_on_a_ball_by_the_views_renderer_in_the_chosen_look() {
    let mut harness = page();
    let view = last_preview(&harness);
    assert_eq!(view.surface, crate::viewport::Surface::Painted);
    assert_eq!(view.paint_preview, crate::viewport::PaintPreview::Lit);
    assert!(!view.show_grid);
    // One object, the ball, showing the material laid as a texture with
    // its roughness, metal and height.
    assert_eq!(view.scene.items.len(), 1);
    let item = &view.scene.items[0];
    assert_eq!(item.textures[0], crate::material::laying::preview_key());
    let texture = &view.scene.textures[0];
    assert!(texture.realism.is_some(), "a whole material's maps");
    // The ball's object isn't one of the scene's.
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .object(item.id)
            .is_none()
    );
    // Each look draws with its own preview.
    for (name, preview) in [
        ("Toon", crate::viewport::PaintPreview::Toon),
        ("Unlit", crate::viewport::PaintPreview::Unlit),
        ("Realistic", crate::viewport::PaintPreview::Realistic),
    ] {
        click(&mut harness, name);
        assert_eq!(last_preview(&harness).paint_preview, preview, "{name}");
    }
    // The test renderer has no graphics card: the ball is drawn here
    // instead, and says so.
    assert!(says(
        &harness,
        "A simpler ball, drawn without the graphics card"
    ));
}

#[test]
fn an_unchanged_preview_is_the_same_drawing() {
    let mut harness = page();
    harness.run();
    let previews = &harness.state().renderer.previews;
    let n = previews.len();
    assert!(n >= 2);
    // Nothing changed between frames: the very same scene, so the
    // renderer shows its last picture again rather than drawing anew.
    assert!(std::sync::Arc::ptr_eq(
        &previews[n - 1].scene,
        &previews[n - 2].scene
    ));
}

#[test]
fn the_cube_and_the_selected_model_show_the_material_at_its_real_size() {
    let mut harness = page();
    click(&mut harness, "Cube");
    let cube_view = last_preview(&harness);
    assert!(
        !says(&harness, "A simpler ball"),
        "only the ball falls back"
    );
    assert!(says(&harness, "The 3D preview can't be shown"));
    // The model: nothing selected yet, so it says what to do.
    click(&mut harness, "Model");
    assert!(says(&harness, "Select a model"));
    // Select the cube in the scene: the preview shows it.
    let id = harness.state().workbench.document().scene().children(None)[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.run();
    let view = last_preview(&harness);
    assert_eq!(view.scene.items[0].id, id);
    assert!(!says(&harness, "Select a model"));
    // The scene's cube and the preview's are the same 1 m shape, so the
    // material is laid the same way on both.
    let a = &cube_view.scene.textures[0].image;
    let b = &view.scene.textures[0].image;
    assert_eq!(a.pixel(10, 10), b.pixel(10, 10));
}

#[test]
fn a_model_without_uvs_says_why_and_what_to_do() {
    let mut harness = page();
    click(&mut harness, "Model");
    // A shape with no UVs, which becomes the selection.
    let mut bare = (*crate::material::laying::cube()).clone();
    bare.remove_attribute(mesh::attributes::UV);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Mesh(
            std::sync::Arc::new(bare),
        )))
        .unwrap();
    harness.run();
    harness.run();
    assert!(says(&harness, "has no UVs yet"));
}

#[test]
fn the_seam_check_repeats_three_times_and_says_the_joins_dont_show() {
    let mut harness = page();
    click(&mut harness, "Flat");
    assert!(!says(&harness, "No joins show"));
    click(&mut harness, crate::material::preview::SEAM_CHECK);
    assert!(harness.state_mut().workbench.material().preview.seams);
    assert!(says(
        &harness,
        "No joins show: the texture repeats seamlessly"
    ));
}

#[test]
fn the_texture_budget_counts_the_maps_and_warns_about_large_sizes() {
    let mut harness = page();
    assert!(says(&harness, "Texture budget"));
    // The project's 2048: three maps for Godot, 2 m a repeat.
    assert!(says(&harness, "3 maps (colour, ORM, normal map)"));
    assert!(says(
        &harness,
        "One repeat covers 100 cm: 2048 pixels a metre"
    ));
    assert!(!says(&harness, "a lot for one material"));
    assert!(
        says(&harness, "Sharper than the project needs"),
        "2048 a metre against 512"
    );
    harness
        .get_by_label_contains(crate::material::preview::BUDGET_SIZE)
        .click();
    harness.run();
    harness.get_by_label("4096 × 4096").click();
    harness.run();
    assert_eq!(
        harness.state_mut().workbench.material().preview.budget_size,
        Some(4096)
    );
    assert!(says(&harness, "a lot for one material"));
    // Back down to what matches the project: no warnings.
    harness
        .get_by_label_contains(crate::material::preview::BUDGET_SIZE)
        .click();
    harness.run();
    harness.get_by_label("512 × 512").click();
    harness.run();
    assert!(!says(&harness, "Sharper than"));
    assert!(!says(&harness, "Blurrier than"));
}
