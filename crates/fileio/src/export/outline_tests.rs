//! Outlines in exports (decision 0201): a shell beside each outlined
//! object, in both formats, and a Godot shader beside a Godot export.

use super::*;
use scene::{Command, ObjectKind, Outline, PrimitiveKind};

/// The Godot preset's options.
fn godot() -> ExportOptions {
    let preset = ExportPreset::Godot.settings();
    ExportOptions {
        units: preset.units,
        up: preset.up,
        triangulate: preset.triangulate,
        godot_names: preset.godot_names,
        ..ExportOptions::default()
    }
}

fn outlined_cube() -> Document {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::SetOutline {
        ids: vec![id],
        outline: Some(Outline {
            width: 2.0,
            colour: [10, 20, 30],
        }),
    })
    .unwrap();
    doc
}

#[test]
fn a_gltf_for_godot_gets_the_shell_and_a_shader_beside_it() {
    let doc = outlined_cube();
    let options = godot();
    let files = export_bytes(&doc, ExportFormat::Glb, &options, "App 1.0", "crate.glb").unwrap();
    assert_eq!(files.summary.objects, 2, "the cube and its shell");
    assert_eq!(summary(&doc, &options), files.summary);
    let glb = String::from_utf8_lossy(&files.main);
    assert!(glb.contains("Cube-outline"));
    assert!(glb.contains("outline-0a141e"));
    let (name, shader) = &files.beside[0];
    assert_eq!(name, "crate-outline.gdshader");
    let shader = String::from_utf8_lossy(shader);
    assert!(shader.starts_with("// App 1.0") && shader.contains("0.0200"));
}

#[test]
fn the_shell_is_the_cube_pushed_out_by_the_width_in_file_units() {
    let doc = outlined_cube();
    let options = godot();
    let scale = options.to_file().transform_vector3(Vec3::X).length();
    let objects = prepare(&doc, &options);
    let shells = outline::outline_objects(&objects, scale);
    assert_eq!(shells.len(), 1);
    let cube = &objects[0].world;
    let shell = &shells[0].world;
    assert_eq!(shell.points.len(), cube.points.len());
    // Each corner moved out by 2 cm (0.02 m), diagonally.
    for (a, b) in cube.points.iter().zip(&shell.points) {
        assert!((a.distance(*b) - 0.02).abs() < 1e-5, "{a} {b}");
    }
    assert_eq!(shells[0].materials[0].name, "outline-0a141e");
}

#[test]
fn an_obj_gets_the_shell_but_no_shader() {
    let doc = outlined_cube();
    let files = export_bytes(
        &doc,
        ExportFormat::Obj,
        &ExportOptions::default(),
        "App 1.0",
        "crate.obj",
    )
    .unwrap();
    let obj = String::from_utf8_lossy(&files.main);
    assert!(obj.contains("Cube-outline"));
    assert!(files.beside.iter().all(|(n, _)| !n.ends_with(".gdshader")));
}

#[test]
fn without_an_outline_nothing_is_added() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let options = godot();
    let files = export_bytes(&doc, ExportFormat::Glb, &options, "App 1.0", "crate.glb").unwrap();
    assert_eq!(files.summary.objects, 1);
    assert!(files.beside.is_empty());
}
