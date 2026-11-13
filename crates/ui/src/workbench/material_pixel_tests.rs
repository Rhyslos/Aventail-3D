//! Pixel-art output in the Material mode (decision 0287), used through
//! the real interface: a Pixelate box just before Material out makes the
//! preview's budget speak of big pixels, Export textures… writes the set
//! at exactly the pixel count with Godot's material reading it crisp, a
//! material given to a cube says how its pixels land on the cube's
//! texture, and an outline with nothing to find shapes in says why. No
//! keys or mouse actions of its own.

use super::graph_giving_tests::{page, set, wait};
use crate::material::export_set::{EXPORT_TEXTURES, GIVE_TO_SELECTED};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, PlugRef, Registry, SettingValue};

type H = egui_kittest::Harness<'static, super::tests::State>;

const PIXELATE: BoxId = BoxId(4);

/// Colour fill and Noise → Material from height → Pixelate (16 across)
/// → Material out.
fn pixel_stone() -> Graph {
    let registry = Registry::standard();
    let mut graph = Graph::new("Pixel stone");
    for (kind, x) in [
        ("noise", 0.0),
        ("colour_fill", 0.0),
        ("material_from_height", 300.0),
        ("pixelate", 600.0),
        ("material_out", 900.0),
    ] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, 0.0),
        };
        graph.apply(registry, &edit).unwrap();
    }
    for (from, out, to, input) in [
        (1, "height", 3, "height"),
        (2, "colour", 3, "colour"),
        (3, "material", 4, "material"),
        (4, "material_out", 5, "material"),
    ] {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(BoxId(from), out),
            to: PlugRef::new(BoxId(to), input),
        };
        graph.apply(registry, &edit).unwrap();
    }
    let edit = GraphEdit::SetSetting {
        node: PIXELATE,
        key: "pixels".into(),
        value: SettingValue::Choice(0),
    };
    graph.apply(registry, &edit).unwrap();
    graph
}

/// Runs frames until the canvas's pictures are worked out.
fn settle(harness: &mut H) {
    for _ in 0..200 {
        harness.run();
        if !harness.state().workbench.material.pictures.waiting() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    harness.run();
}

#[test]
fn the_budget_speaks_of_big_pixels_and_the_export_writes_exactly_them() {
    let (mut harness, _) = page(pixel_stone());
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("makes this material 16 × 16 big pixels")
            .is_some()
    );
    assert!(
        harness
            .query_by_label_contains("each pixel 6.25 cm across")
            .is_some(),
        "100 cm over 16 pixels"
    );
    assert!(
        harness
            .query_by_label_contains("16 × 16 PNG files made for Godot")
            .is_some()
    );
    assert!(
        harness
            .query_by_label_contains("Godot's material reads them crisp")
            .is_some()
    );
    // Not called blurrier than the project aims for: it is the look.
    assert!(harness.query_by_label_contains("Blurrier than").is_none());
    harness.get_by_label(EXPORT_TEXTURES).click();
    harness.run();
    let folder = std::env::temp_dir().join(format!("pixel-textures-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let workbench = &mut harness.state_mut().workbench;
    workbench.material.preview.budget_size = Some(256);
    workbench.export_graph_textures_into(folder.clone());
    let said = workbench.material.export_set.finish().unwrap();
    assert!(
        said.starts_with("Exported Pixel stone's textures"),
        "{said}"
    );
    for name in [
        "Pixel_stone.png",
        "Pixel_stone_Normal.png",
        "Pixel_stone_Height.png",
    ] {
        let read =
            fileio::image_file::read_image(&std::fs::read(folder.join(name)).unwrap()).unwrap();
        assert_eq!((read.width, read.height), (16, 16), "{name}");
    }
    let tres = std::fs::read_to_string(folder.join("Pixel_stone.tres")).unwrap();
    assert!(tres.contains("texture_filter = 2"), "{tres}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn bypassing_pixelate_goes_back_to_a_smooth_export() {
    let (mut harness, id) = page(pixel_stone());
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id,
            edit: GraphEdit::Bypass {
                node: PIXELATE,
                on: true,
            },
        })
        .unwrap();
    settle(&mut harness);
    assert!(harness.query_by_label_contains("big pixels").is_none());
    let folder = std::env::temp_dir().join(format!("pixel-bypassed-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let workbench = &mut harness.state_mut().workbench;
    workbench.material.preview.budget_size = Some(256);
    workbench.export_graph_textures_into(folder.clone());
    workbench.material.export_set.finish().unwrap();
    let read =
        fileio::image_file::read_image(&std::fs::read(folder.join("Pixel_stone.png")).unwrap())
            .unwrap();
    assert_eq!(read.width, 256);
    let tres = std::fs::read_to_string(folder.join("Pixel_stone.tres")).unwrap();
    assert!(!tres.contains("texture_filter"), "{tres}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_pixel_art_material_on_a_cube_says_how_its_pixels_land() {
    let (mut harness, _) = page(pixel_stone());
    harness.get_by_label(GIVE_TO_SELECTED).click();
    harness.step();
    wait(&mut harness);
    // The notes are worked out again once the making is in.
    harness.run();
    harness.run();
    assert!(set(&harness).is_some());
    let notes = &harness.state().workbench.paint_layers.graph_pixels;
    assert_eq!(notes.len(), 1, "{notes:?}");
    let note = notes.values().next().unwrap();
    assert!(note.starts_with("Pixel art: 16 pixels a repeat"), "{note}");
    assert!(note.contains("64-pixel texture"), "{note}");
}

#[test]
fn an_outline_with_no_shapes_to_follow_says_why_on_the_canvas() {
    let (mut harness, id) = page(pixel_stone());
    let doc = harness.state_mut().workbench.document_mut();
    // Colour fill's colour straight into Pixelate, the material taken out.
    for edit in [
        GraphEdit::Disconnect(PlugRef::new(PIXELATE, "material")),
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(2), "colour"),
            to: PlugRef::new(PIXELATE, "colour"),
        },
        GraphEdit::SetSetting {
            node: PIXELATE,
            key: "outline".into(),
            value: SettingValue::Choice(1),
        },
    ] {
        doc.execute(scene::Command::EditGraph { id, edit }).unwrap();
    }
    settle(&mut harness);
    let workbench = &mut harness.state_mut().workbench;
    let document = workbench.document.clone();
    let graph = workbench.material.current(&document).unwrap().clone();
    let why = workbench
        .material
        .problems(&graph)
        .remove(&PIXELATE)
        .map(|r| r.reason)
        .unwrap_or_default();
    assert!(why.contains("Outline from"), "{why}");
}
