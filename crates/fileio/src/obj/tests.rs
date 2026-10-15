//! Tests for OBJ files: writing, reading back, and files from elsewhere.

use super::*;
use crate::export::{ExportOptions, prepare};
use scene::{Command, Document, ObjectKind, PrimitiveKind};

fn scene_of(kinds: &[PrimitiveKind]) -> Document {
    let mut doc = Document::new();
    for kind in kinds {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
    }
    doc
}

fn written(doc: &Document, options: &ExportOptions) -> String {
    write_obj(&prepare(doc, options), options, "Test App 1.0", None)
}

fn read(text: &str) -> ObjImport {
    read_obj(text, FileUnits::Metres, UpAxis::Y).unwrap()
}

#[test]
fn a_cube_writes_as_expected() {
    let doc = scene_of(&[PrimitiveKind::Cube]);
    let text = written(&doc, &ExportOptions::default());
    assert!(text.starts_with("# Test App 1.0\n# Units: metres, Y up\no Cube\n"));
    assert_eq!(text.lines().filter(|l| l.starts_with("v ")).count(), 8);
    assert_eq!(text.lines().filter(|l| l.starts_with("vn ")).count(), 6);
    assert_eq!(text.lines().filter(|l| l.starts_with("f ")).count(), 6);
    assert!(text.lines().any(|l| l == "f 1//1 2//1 3//1 4//1") || text.contains("//"));
    // The same scene always writes the same text.
    assert_eq!(text, written(&doc, &ExportOptions::default()));
}

#[test]
fn what_is_written_reads_back_the_same() {
    let doc = scene_of(&[PrimitiveKind::Cube, PrimitiveKind::Cylinder]);
    let text = written(&doc, &ExportOptions::default());
    let back = read(&text);
    assert_eq!(back.objects.len(), 2);
    assert_eq!((back.skipped_lines, back.skipped_polygons), (0, 0));
    for (object, id) in back.objects.iter().zip(doc.scene().children(None)) {
        let original = doc.scene().evaluated(*id).unwrap().mesh;
        assert_eq!(object.name, doc.scene().object(*id).unwrap().name);
        assert_eq!(object.mesh.face_count(), original.face_count());
        assert_eq!(object.mesh.vertex_count(), original.vertex_count());
        // Back in centimetres, Z up, where they were.
        let (a, b) = (object.mesh.bounds().unwrap(), original.bounds().unwrap());
        assert!(a.min.distance(b.min) < 1e-3 && a.max.distance(b.max) < 1e-3);
        // Hard edges come back where the shading breaks.
        assert_eq!(object.mesh.hard_edge_count(), original.hard_edge_count());
    }
}

#[test]
fn files_from_elsewhere_are_read_kindly() {
    let text = "\
# A quad and a triangle, Y up, metres
mtllib things.mtl
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
vt 0 0
vt 1 0
vt 1 1
vn 0 0 1
g panel
usemtl metal
s off
f 1/1/1 2/2/1 3/3/1 4/1/1
f -4 -3 -2
l 1 2
f 1 1 2
f 1 2 9
curv 0 1 1 2
";
    let back = read(text);
    assert_eq!(back.objects.len(), 1);
    assert_eq!(back.objects[0].name, "panel");
    assert_eq!(back.objects[0].mesh.face_count(), 2);
    // `l` and `curv` aren't read; a polygon on one point twice and one
    // naming a missing point are left out.
    assert_eq!(back.skipped_lines, 2);
    assert_eq!(back.skipped_polygons, 2);
    // The UVs came along: Y up in the file is Z up here, metres are 100 cm.
    let mesh = &back.objects[0].mesh;
    assert!(mesh.attributes().any(|(n, _, _)| n == mesh::attributes::UV));
    let top = mesh.bounds().unwrap().max;
    assert!((top.z - 100.0).abs() < 1e-3, "{top}");
}

#[test]
fn objects_split_at_o_or_else_at_g() {
    let quad = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n";
    let text = format!("{quad}o First\nf 1 2 3\ng part\nf 1 3 4\no Second\nf 2 3 4\n");
    let back = read(&text);
    let names: Vec<&str> = back.objects.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["First", "Second"]);
    assert_eq!(back.objects[0].mesh.face_count(), 2);
    let text = format!("{quad}g One\nf 1 2 3\ng Two\nf 1 3 4\n");
    let names: Vec<String> = read(&text).objects.into_iter().map(|o| o.name).collect();
    assert_eq!(names, ["One", "Two"]);
    // No names at all: one object.
    let back = read(&format!("{quad}f 1 2 3 4\n"));
    assert_eq!(back.objects[0].name, "Object");
}

#[test]
fn bad_files_are_refused_in_plain_words() {
    let error = read_obj("v 0 0 nan\nf 1 1 1\n", FileUnits::Metres, UpAxis::Y).unwrap_err();
    assert_eq!(error, ObjError::BadNumber { line: 1 });
    assert!(error.to_string().contains("line 1"));
    let error = read_obj("v 1 two 3\n", FileUnits::Metres, UpAxis::Y).unwrap_err();
    assert_eq!(error, ObjError::BadNumber { line: 1 });
    assert_eq!(
        read_obj("# nothing\nv 0 0 0\n", FileUnits::Metres, UpAxis::Y),
        Err(ObjError::Empty)
    );
    assert_eq!(
        read_obj("", FileUnits::Metres, UpAxis::Y),
        Err(ObjError::Empty)
    );
}

#[test]
fn hard_edges_follow_the_files_normals() {
    // Two quads folded at a right angle along their shared edge.
    let fold = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 1 0 -1\nv 1 1 -1\n";
    let crisp = format!("{fold}vn 0 0 1\nvn 1 0 0\nf 1//1 2//1 3//1 4//1\nf 2//2 5//2 6//2 3//2\n");
    assert_eq!(read(&crisp).objects[0].mesh.hard_edge_count(), 1);
    let smooth = format!("{fold}vn 1 0 1\nf 1//1 2//1 3//1 4//1\nf 2//1 5//1 6//1 3//1\n");
    assert_eq!(read(&smooth).objects[0].mesh.hard_edge_count(), 0);
}

#[test]
fn names_stay_on_one_line() {
    assert_eq!(clean_name("Big\ncrate "), "Big crate");
    assert_eq!(clean_name("  "), "Object");
    assert_eq!(number(-0.0), "0");
    assert_eq!(number(0.25), "0.25");
}

#[test]
fn random_text_never_crashes() {
    let mut seed = 12345u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let words = [
        "v", "vt", "vn", "f", "o", "g", "1", "-1", "2/3", "1//2", "0.5", "x", "#", "\n", "nan",
        "1e40", "/",
    ];
    for _ in 0..300 {
        let text: String = (0..60)
            .map(|_| format!("{} ", words[(next() % words.len() as u64) as usize]))
            .collect();
        let _ = read_obj(&text, FileUnits::Metres, UpAxis::Y);
    }
}

#[test]
fn the_apps_own_units_note_is_followed() {
    let doc = scene_of(&[PrimitiveKind::Cube]);
    let options = ExportOptions {
        units: FileUnits::Centimetres,
        up: UpAxis::Z,
        ..ExportOptions::default()
    };
    let text = written(&doc, &options);
    assert_eq!(
        noted_units(&text),
        Some((FileUnits::Centimetres, UpAxis::Z))
    );
    // Read as metres, Y up, but the note wins: back where it was.
    let back = read(&text);
    let top = back.objects[0].mesh.bounds().unwrap().max;
    assert!((top.z - 100.0).abs() < 1e-3, "{top}");
    assert_eq!(noted_units("# Units: furlongs"), None);
}

#[test]
fn export_bytes_says_what_it_wrote_and_refuses_nothing() {
    use crate::export::{ExportError, ExportFormat, export_bytes};
    let doc = scene_of(&[PrimitiveKind::Cube, PrimitiveKind::Plane]);
    let files = export_bytes(
        &doc,
        ExportFormat::Obj,
        &ExportOptions::default(),
        "T",
        "crate.obj",
    )
    .unwrap();
    let summary = files.summary;
    assert_eq!(summary.objects, 2);
    assert_eq!(summary.triangles, 12 + 2);
    assert_eq!(summary.materials, 1);
    let text = String::from_utf8(files.main).unwrap();
    assert!(text.contains("o Plane"));
    assert!(text.contains("mtllib crate.mtl"));
    assert_eq!(files.beside.len(), 1);
    assert_eq!(files.beside[0].0, "crate.mtl");
    let empty = Document::new();
    assert!(matches!(
        export_bytes(
            &empty,
            ExportFormat::Obj,
            &ExportOptions::default(),
            "T",
            "e.obj"
        ),
        Err(ExportError::Nothing)
    ));
}

#[test]
fn materials_are_named_on_usemtl_lines_and_described_in_the_mtl_file() {
    let mut doc = scene_of(&[PrimitiveKind::Cube]);
    let id = doc.scene().children(None)[0];
    doc.execute(scene::Command::SetMaterials {
        id,
        materials: vec![
            scene::Material::standard(),
            scene::Material {
                name: "Rusty metal".into(),
                color: [0.5, 0.25, 0.0],
            },
        ],
    })
    .unwrap();
    doc.execute(scene::Command::Record {
        id,
        operation: scene::Operation::SetMaterial {
            faces: vec![mesh::FaceId::from_raw(5)],
            slot: 1,
            anchors: None,
        },
    })
    .unwrap();
    let options = ExportOptions::default();
    let objects = prepare(&doc, &options);
    let text = write_obj(&objects, &options, "T", Some("crate.mtl"));
    assert!(text.contains("mtllib crate.mtl\n"));
    // One line for each change of material; names are one word.
    assert_eq!(text.matches("usemtl Default\n").count(), 1, "{text}");
    assert_eq!(text.matches("usemtl Rusty_metal\n").count(), 1);
    let mtl = write_mtl(&objects, "T");
    assert!(mtl.contains("newmtl Default\n"));
    assert!(mtl.contains("newmtl Rusty_metal\nKa 0 0 0\nKd 0.5 0.25 0\n"));
    // Reading it back gives the slots and, with the .mtl file, colours.
    let mut back = read_obj(&text, FileUnits::Metres, UpAxis::Y).unwrap();
    assert_eq!(back.material_files, ["crate.mtl"]);
    back.colour_materials(&read_mtl(&mtl));
    let object = &back.objects[0];
    let names: Vec<&str> = object.materials.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["Default", "Rusty_metal"]);
    assert_eq!(object.materials[1].color, [0.5, 0.25, 0.0]);
    assert_eq!(mesh::material::faces_with_slot(&object.mesh, 1).len(), 1);
}

#[test]
fn a_file_without_materials_imports_without_any() {
    let back = read_obj(
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        FileUnits::Metres,
        UpAxis::Y,
    )
    .unwrap();
    assert!(back.objects[0].materials.is_empty());
    assert!(back.material_files.is_empty());
}

#[test]
fn mtl_files_are_read_carefully() {
    let colours =
        read_mtl("newmtl A\nKd 2 -1 0.5\nnewmtl B\nKd nan 0 0\nKd 1\nfoo bar\nKd 0 0 0\n");
    // Held to 0 to 1; numbers that aren't real are passed over.
    assert_eq!(colours["A"], [1.0, 0.0, 0.5]);
    assert_eq!(colours["B"], [0.0, 0.0, 0.0]);
    // A colour before any material belongs to none.
    assert!(read_mtl("Kd 1 1 1\n").is_empty());
}

#[test]
fn material_files_are_found_beside_the_obj_only() {
    let folder = std::env::temp_dir().join(format!("obj-mtl-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("crate.mtl"), "newmtl Wood\nKd 0.4 0.2 0.1\n").unwrap();
    let text = "mtllib ../../crate.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Wood\nf 1 2 3\n";
    let mut back = read_obj(text, FileUnits::Metres, UpAxis::Y).unwrap();
    back.colour_from_files(&folder);
    // The path in the file is cut to its name, found in the OBJ's folder.
    assert_eq!(back.objects[0].materials[0].color, [0.4, 0.2, 0.1]);
    // A missing file leaves the material grey.
    let mut missing = read_obj(
        &text.replace("crate.mtl", "gone.mtl"),
        FileUnits::Metres,
        UpAxis::Y,
    )
    .unwrap();
    missing.colour_from_files(&folder);
    assert_eq!(
        missing.objects[0].materials[0],
        scene::Material {
            name: "Wood".into(),
            ..scene::Material::standard()
        }
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn many_materials_in_one_object_share_the_last_slot() {
    let mut text = String::new();
    let n = mesh::material::MAX_SLOTS as usize + 3;
    for i in 0..n {
        let x = i * 2;
        text.push_str(&format!(
            "v {x} 0 0\nv {} 0 0\nv {x} 1 0\nusemtl M{i}\nf -3 -2 -1\n",
            x + 1
        ));
    }
    let back = read_obj(&text, FileUnits::Metres, UpAxis::Y).unwrap();
    let object = &back.objects[0];
    assert_eq!(object.materials.len(), mesh::material::MAX_SLOTS as usize);
    let last = mesh::material::MAX_SLOTS - 1;
    assert_eq!(mesh::material::faces_with_slot(&object.mesh, last).len(), 4);
}
