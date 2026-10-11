//! Tests for `.avtl` project files: everything saved comes back exactly,
//! damaged or hostile files are refused in plain words (never a crash),
//! and saving to disk never leaves a half-written project.

use super::*;
use glam::Vec3;
use mesh::{CubeParams, PrimitiveKind, VertexId};
use scene::{Command, Object, ObjectKind, Operation, Place, Primitive, Transform};

/// A document using everything a project can hold: nested groups, every
/// kind of primitive with changed settings, a plain mesh, recorded edits,
/// a hidden object, a moved and turned object, and a selection.
fn rich_document() -> Document {
    let mut doc = Document::new();
    for kind in PrimitiveKind::ALL {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
    }
    let ids = doc.scene().children(None).to_vec();
    // Changed settings on the cube.
    let cube = *ids.last().unwrap();
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: Primitive::Cube(CubeParams {
            size: Vec3::new(120.0, 80.0, 45.5),
            segments: [2, 3, 4],
        }),
    })
    .unwrap();
    // Recorded edits on the cube, one of them naming a missing point.
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::MovePoints {
            points: vec![VertexId::from_raw(0), VertexId::from_raw(4000)],
            offset: Vec3::new(0.0, 0.0, 12.5),
        },
    })
    .unwrap();
    // The sphere becomes a plain mesh.
    let sphere = ids[3];
    doc.execute(Command::MakeEditable(vec![sphere])).unwrap();
    // Two groups, one inside the other, holding the plane.
    doc.execute(Command::Group(vec![ids[5]])).unwrap();
    let outer = doc.selection()[0];
    doc.execute(Command::Group(vec![outer])).unwrap();
    doc.execute(Command::Rename {
        id: outer,
        name: "Inner group – ÆØÅ".into(),
    })
    .unwrap();
    // A hidden object.
    doc.execute(Command::SetVisible {
        ids: vec![ids[0]],
        visible: false,
    })
    .unwrap();
    // An object placed, turned and scaled.
    let mut placed = Object::new(ObjectKind::Group);
    placed.name = "Placed".into();
    placed.transform = Transform {
        position: Vec3::new(10.0, -20.0, 30.0),
        rotation: Vec3::new(0.5, 0.25, -1.0),
        scale: Vec3::new(1.0, 2.0, 0.5),
    };
    doc.execute(Command::Insert {
        object: placed,
        place: Place {
            parent: None,
            index: 2,
        },
    })
    .unwrap();
    // A deleted object, so its id stays taken.
    doc.execute(Command::Delete(vec![ids[1]])).unwrap();
    doc.set_selection([cube, sphere]);
    doc
}

/// Checks two documents hold the same scene, shapes and selection.
fn assert_same(a: &Document, b: &Document) {
    assert_eq!(a.scene(), b.scene());
    assert_eq!(a.selection(), b.selection());
    assert_eq!(a.scene().next_object_id(), b.scene().next_object_id());
    for (id, _) in a.scene().walk() {
        assert_eq!(a.scene().evaluated(id), b.scene().evaluated(id), "{id}");
    }
}

// ----------------------------------------------------------- round trip ---

#[test]
fn everything_saved_comes_back_exactly() {
    let doc = rich_document();
    let bytes = to_bytes(&doc).unwrap();
    let opened = from_bytes(&bytes).unwrap();
    assert_same(&doc, &opened);
    assert_eq!(opened.undo_steps(), 0, "a fresh history");
}

#[test]
fn an_empty_scene_round_trips() {
    let doc = Document::new();
    let opened = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert!(opened.scene().is_empty());
}

#[test]
fn the_same_scene_saves_to_the_same_bytes() {
    let doc = rich_document();
    assert_eq!(to_bytes(&doc).unwrap(), to_bytes(&doc).unwrap());
    let copy = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(to_bytes(&copy).unwrap(), to_bytes(&doc).unwrap());
}

#[test]
fn the_header_says_what_the_file_is() {
    let bytes = to_bytes(&rich_document()).unwrap();
    assert_eq!(&bytes[..4], b"AVTL");
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 1);
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        1,
        "zstd"
    );
    let length = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
    assert_eq!(length as usize, bytes.len() - 20);
}

#[test]
fn compression_keeps_meshes_small() {
    let mut doc = Document::new();
    let sphere = mesh::SphereParams {
        sides: 128,
        rings: 64,
        ..Default::default()
    };
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Sphere(
        sphere,
    ))))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let raw = postcard::to_stdvec(&v1::ProjectV1::from_document(&doc))
        .unwrap()
        .len();
    let saved = to_bytes(&doc).unwrap().len();
    // Positions are floating-point numbers, which pack less tightly than
    // text; a plain mesh still saves noticeably smaller.
    assert!(saved * 10 < raw * 7, "{saved} of {raw} bytes");
}

// ------------------------------------------------------ files refused ---

fn header(version: u32, flags: u32, length: u64) -> Vec<u8> {
    let mut bytes = b"AVTL".to_vec();
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes
}

fn refused(bytes: &[u8]) -> ProjectError {
    match from_bytes(bytes) {
        Ok(_) => panic!("opened a broken file"),
        Err(e) => e,
    }
}

#[test]
fn other_files_are_not_projects() {
    for bytes in [
        &b""[..],
        b"AVT",
        b"PK\x03\x04 a zip file",
        b"avtl lower case",
    ] {
        assert!(
            matches!(refused(bytes), ProjectError::NotAProject),
            "{bytes:?}"
        );
    }
    assert_eq!(
        ProjectError::NotAProject.to_string(),
        "it isn't a project file"
    );
}

#[test]
fn newer_formats_are_refused_with_advice() {
    let error = refused(&header(2, 1, 0));
    assert!(matches!(error, ProjectError::TooNew { version: 2 }));
    let text = error.to_string();
    assert!(
        text.contains("newer version") && text.contains("Update"),
        "{text}"
    );
    // Flags this version doesn't know also mean a newer writer.
    assert!(matches!(
        refused(&header(1, 1 | 4, 0)),
        ProjectError::TooNew { .. }
    ));
}

#[test]
fn damaged_headers_are_explained() {
    assert!(matches!(
        refused(&header(0, 1, 0)),
        ProjectError::Damaged(_)
    ));
    let full = to_bytes(&Document::new()).unwrap();
    for cut in [5, 9, 13, 19] {
        let error = refused(&full[..cut]);
        assert!(error.to_string().contains("cut short"), "{cut}: {error}");
    }
    assert!(matches!(
        refused(&header(1, 1, u64::MAX)),
        ProjectError::TooLarge
    ));
}

#[test]
fn a_cut_short_or_padded_body_is_damaged() {
    let full = to_bytes(&rich_document()).unwrap();
    let error = refused(&full[..full.len() - 10]);
    assert!(error.to_string().contains("cut short"), "{error}");
    let mut padded = full.clone();
    padded.extend_from_slice(b"junk");
    assert!(matches!(refused(&padded), ProjectError::Damaged(_)));
}

#[test]
fn an_uncompressed_body_can_be_read() {
    let doc = rich_document();
    let body = postcard::to_stdvec(&v1::ProjectV1::from_document(&doc)).unwrap();
    let mut bytes = header(1, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    assert_same(&doc, &from_bytes(&bytes).unwrap());
    // Data after the scene is refused.
    let mut longer = body.clone();
    longer.push(0);
    let mut bytes = header(1, 0, longer.len() as u64);
    bytes.extend_from_slice(&longer);
    assert!(refused(&bytes).to_string().contains("unexpected data"));
}

/// The bytes of a file holding `model`, uncompressed.
fn file_of(model: &v1::ProjectV1) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(1, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn bad_values_inside_are_refused_naming_the_object() {
    let good = v1::ProjectV1::from_document(&rich_document());
    let cube = good.objects.iter().position(|o| o.name == "Cube").unwrap();

    let mut bad = good.clone();
    bad.objects[cube].transform.position[1] = f32::NAN;
    let text = refused(&file_of(&bad)).to_string();
    assert!(
        text.contains("\"Cube\"") && text.contains("position"),
        "{text}"
    );

    let mut bad = good.clone();
    let sphere = bad.objects.iter().position(|o| o.name == "Sphere").unwrap();
    if let v1::KindV1::Mesh(mesh) = &mut bad.objects[sphere].kind {
        mesh.faces[0] = Some(vec![0, 1, 99_999]);
    }
    let text = refused(&file_of(&bad)).to_string();
    assert!(
        text.contains("\"Sphere\"") && text.contains("mesh"),
        "{text}"
    );

    let mut bad = good.clone();
    bad.objects[cube].layers[0].operations[0] = v1::OperationV1::MovePoints {
        points: vec![0],
        offset: [0.0, f32::INFINITY, 0.0],
    };
    assert!(
        refused(&file_of(&bad))
            .to_string()
            .contains("recorded move")
    );

    let mut bad = good.clone();
    bad.objects[cube].parent = Some(424_242);
    assert!(matches!(refused(&file_of(&bad)), ProjectError::Damaged(_)));

    let mut bad = good.clone();
    bad.objects[cube].name = String::new();
    assert!(matches!(refused(&file_of(&bad)), ProjectError::Damaged(_)));
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let mut model = v1::ProjectV1::from_document(&Document::new());
    model.objects.push(v1::ObjectV1 {
        id: 0,
        parent: None,
        name: "Huge".into(),
        visible: true,
        transform: v1::TransformV1 {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        },
        kind: v1::KindV1::Primitive(v1::PrimitiveV1::Sphere {
            radius: f32::NAN,
            sides: u32::MAX,
            rings: 0,
        }),
        layers: Vec::new(),
    });
    let doc = from_bytes(&file_of(&model)).unwrap();
    let id = doc.scene().children(None)[0];
    match &doc.scene().object(id).unwrap().kind {
        ObjectKind::Primitive(p) => assert_eq!(*p, p.clamped()),
        other => panic!("{other:?}"),
    }
}

// ----------------------------------------------------------------- fuzz ---

/// A small, repeatable random number source.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn every_cut_of_a_file_is_refused_without_a_crash() {
    let full = to_bytes(&rich_document()).unwrap();
    for cut in 0..full.len() {
        assert!(from_bytes(&full[..cut]).is_err(), "cut at {cut}");
    }
}

#[test]
fn flipped_bytes_never_crash_the_reader() {
    let full = to_bytes(&rich_document()).unwrap();
    let raw = file_of(&v1::ProjectV1::from_document(&rich_document()));
    let mut rng = Lcg(7);
    for round in 0..3000 {
        let mut bytes = if round % 2 == 0 {
            full.clone()
        } else {
            raw.clone()
        };
        for _ in 0..(rng.next() % 4 + 1) {
            let at = (rng.next() as usize) % bytes.len();
            bytes[at] ^= 1 << (rng.next() % 8);
        }
        // Either it opens (the flip landed somewhere harmless) or it is
        // refused; it must never panic.
        let _ = from_bytes(&bytes);
    }
}

#[test]
fn random_bytes_after_a_valid_header_never_crash_the_reader() {
    let mut rng = Lcg(11);
    for _ in 0..2000 {
        let length = (rng.next() % 200) as usize;
        let body: Vec<u8> = (0..length).map(|_| rng.next() as u8).collect();
        for flags in [0, 1] {
            let mut bytes = header(1, flags, body.len() as u64);
            bytes.extend_from_slice(&body);
            // Refused, or by chance a valid (tiny) scene; never a crash.
            let _ = from_bytes(&bytes);
        }
    }
}

// ------------------------------------------------------------- on disk ---

/// A fresh folder for one test, removed when dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("avtl-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn saving_and_opening_on_disk_keeps_a_backup_and_no_leftovers() {
    let folder = Folder::new("save");
    let path = folder.0.join("crate.avtl");
    let first = rich_document();
    save(&path, &first).unwrap();
    assert!(
        !backup_path(&path).exists(),
        "nothing to back up the first time"
    );
    assert_same(&first, &open(&path).unwrap());

    let mut second = first.clone();
    second.execute(Command::Create(ObjectKind::Group)).unwrap();
    save(&path, &second).unwrap();
    assert_same(&second, &open(&path).unwrap());
    assert_same(&first, &open(&backup_path(&path)).unwrap());
    let names: Vec<String> = std::fs::read_dir(&folder.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n.ends_with(".tmp")), "{names:?}");
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn saving_where_it_cant_be_written_explains_and_leaves_nothing() {
    let folder = Folder::new("nowhere");
    let path = folder.0.join("missing folder").join("crate.avtl");
    let error = save(&path, &Document::new()).unwrap_err();
    assert!(matches!(
        error,
        ProjectError::Io {
            action: "write",
            ..
        }
    ));
    assert!(error.to_string().contains("crate.avtl"), "{error}");
    assert!(!path.exists());
}

#[test]
fn opening_a_missing_or_wrong_file_explains_why() {
    let folder = Folder::new("open");
    let missing = folder.0.join("nothing.avtl");
    let error = open(&missing).unwrap_err();
    assert!(matches!(error, ProjectError::Io { action: "read", .. }));
    let text_file = folder.0.join("notes.avtl");
    std::fs::write(&text_file, "just some text").unwrap();
    assert!(matches!(open(&text_file), Err(ProjectError::NotAProject)));
}

#[test]
fn a_failed_save_keeps_the_previous_file() {
    let folder = Folder::new("keep");
    let path = folder.0.join("crate.avtl");
    let first = rich_document();
    save(&path, &first).unwrap();
    // Block the temporary file's place with a folder, so writing fails.
    std::fs::create_dir_all(folder.0.join("crate.avtl.tmp")).unwrap();
    assert!(save(&path, &Document::new()).is_err());
    assert_same(&first, &open(&path).unwrap());
}

// --------------------------------------------- files from older versions ---

/// A project saved by version 1 of the format, kept in the repository: it
/// must open in every future version of the app.
const SAMPLE_V1: &[u8] = include_bytes!("../../tests/data/sample-v1.avtl");

#[test]
fn a_file_saved_by_format_version_1_still_opens() {
    let doc = from_bytes(SAMPLE_V1).expect("the version 1 sample opens");
    let names: Vec<String> = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, depth)| format!("{depth}:{}", doc.scene().object(id).unwrap().name))
        .collect();
    assert_eq!(
        names,
        [
            "0:Cone",
            "0:Placed",
            "0:Disc",
            "0:Sphere",
            "0:Cylinder",
            "0:Group 2",
            "1:Inner group – ÆØÅ",
            "2:Plane",
            "0:Cube",
        ]
    );
    let cube = *doc.scene().children(None).last().unwrap();
    assert_eq!(doc.scene().object(cube).unwrap().name, "Cube");
    assert_eq!(doc.scene().object(cube).unwrap().stack.operation_count(), 1);
    assert_eq!(doc.selection().len(), 2);
    assert_same(&rich_document(), &doc);
}

/// Writes the sample again. Run only by hand, when a new format version
/// is added and the previous sample must be kept as it is:
/// `WRITE_SAMPLE=1 cargo test -p fileio write_the_sample`.
#[test]
fn write_the_sample() {
    if std::env::var_os("WRITE_SAMPLE").is_some() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/sample-v1.avtl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, to_bytes(&rich_document()).unwrap()).unwrap();
    }
}

#[test]
fn the_backup_sits_beside_the_project() {
    assert_eq!(
        backup_path(Path::new("/work/crate.avtl")),
        PathBuf::from("/work/crate.avtl.bak")
    );
}
