//! Tests for STL files: what is written, read back by a small reader
//! here (the app itself never reads STL), merging, the print check, bad
//! input, and how long a heavy shape takes.

use super::check::{IssueKind, PrintCheck, print_check};
use super::*;
use crate::export::{
    ExportCorner, ExportFormat, ExportPolygon, ExportPreset, ExportWhat, export_bytes,
};
use scene::{Command, Document, ObjectId, ObjectKind, PrimitiveKind, Transform};
use std::sync::Arc;

/// A triangle read back: its normal and corners.
#[derive(Debug, Clone, Copy)]
struct Read {
    normal: Vec3,
    corners: [Vec3; 3],
}

/// Reads a binary STL file, checking its size against its count.
fn read(bytes: &[u8]) -> Vec<Read> {
    assert!(bytes.len() >= HEADER_SIZE + 4, "too short");
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), HEADER_SIZE + 4 + count * TRIANGLE_SIZE);
    let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let vec = |at: usize| Vec3::new(float(at), float(at + 4), float(at + 8));
    (0..count)
        .map(|i| {
            let at = HEADER_SIZE + 4 + i * TRIANGLE_SIZE;
            assert_eq!(&bytes[at + 48..at + 50], &[0, 0], "spare bytes are zero");
            Read {
                normal: vec(at),
                corners: [vec(at + 12), vec(at + 24), vec(at + 36)],
            }
        })
        .collect()
}

fn bounds(triangles: &[Read]) -> (Vec3, Vec3) {
    triangles.iter().flat_map(|t| t.corners).fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(a, b), p| (a.min(p), b.max(p)),
    )
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-3 * (1.0 + a.length().max(b.length()))
}

/// Every edge of the triangles (by position) used exactly twice, once
/// each way: a closed surface, consistently facing.
fn watertight(triangles: &[Read]) -> bool {
    use std::collections::HashMap;
    let key = |p: Vec3| [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
    let mut uses: HashMap<([u32; 3], [u32; 3]), i32> = HashMap::new();
    for t in triangles {
        for i in 0..3 {
            let (a, b) = (key(t.corners[i]), key(t.corners[(i + 1) % 3]));
            *uses.entry((a, b)).or_default() += 1;
        }
    }
    uses.iter()
        .all(|((a, b), n)| *n == 1 && uses.get(&(*b, *a)) == Some(&1))
}

/// Each normal points away from `middle` (true of a convex shape).
fn facing_out(triangles: &[Read], middle: Vec3) -> bool {
    triangles.iter().all(|t| {
        let centre = (t.corners[0] + t.corners[1] + t.corners[2]) / 3.0;
        t.normal.dot(centre - middle) > 0.0 && (t.normal.length() - 1.0).abs() < 1e-4
    })
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn place(doc: &mut Document, id: ObjectId, transform: Transform) {
    doc.execute(Command::SetTransforms(vec![(id, transform)]))
        .unwrap();
}

fn at(position: Vec3) -> Transform {
    Transform {
        position,
        ..Transform::default()
    }
}

fn options(units: PrintUnits, merge: bool) -> ExportOptions {
    ExportOptions {
        print_units: units,
        merge,
        ..ExportOptions::default()
    }
}

fn stl(doc: &Document, options: &ExportOptions) -> Result<ExportFiles, ExportError> {
    export_bytes(doc, ExportFormat::Stl, options, "Test App 1.0", "part.stl")
}

/// A 100 cm box from 8 points with the six sides listed, less any in
/// `leave_out`.
fn open_box(leave_out: &[usize]) -> Mesh {
    let mut mesh = Mesh::new();
    let v: Vec<VertexId> = (0..8)
        .map(|i| {
            let p = Vec3::new(
                (i & 1) as f32 * 100.0,
                ((i >> 1) & 1) as f32 * 100.0,
                ((i >> 2) & 1) as f32 * 100.0,
            );
            mesh.add_vertex(p).unwrap()
        })
        .collect();
    let sides: [[usize; 4]; 6] = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    for (i, s) in sides.iter().enumerate() {
        if !leave_out.contains(&i) {
            mesh.add_face(&s.map(|k| v[k])).unwrap();
        }
    }
    mesh
}

fn mesh_object(doc: &mut Document, mesh: Mesh) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(mesh))))
        .unwrap();
    doc.selection()[0]
}

#[test]
fn a_cube_writes_twelve_triangles_in_millimetres_facing_out() {
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Cube);
    let files = stl(&doc, &ExportOptions::default()).unwrap();
    assert!(files.beside.is_empty(), "an STL is one file");
    assert_eq!(files.summary.triangles, 12);
    assert_eq!(files.summary.materials, 0);
    let back = read(&files.main);
    assert_eq!(back.len(), 12);
    // The 100 cm cube standing on the grid: 1000 mm, Z up from 0.
    let (min, max) = bounds(&back);
    assert!(close(min, Vec3::new(-500.0, -500.0, 0.0)), "{min}");
    assert!(close(max, Vec3::new(500.0, 500.0, 1000.0)), "{max}");
    assert!(facing_out(&back, Vec3::new(0.0, 0.0, 500.0)));
    assert!(watertight(&back));
    // The header names the app, and isn't taken for a text STL.
    assert!(files.main.starts_with(b"Test App 1.0"));
    // The same scene always writes the same bytes.
    assert_eq!(files.main, stl(&doc, &ExportOptions::default()).unwrap().main);
}

#[test]
fn each_unit_scales_the_print() {
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Cube);
    for (units, height) in [
        (PrintUnits::Millimetres, 1000.0),
        (PrintUnits::Centimetres, 100.0),
        (PrintUnits::Metres, 1.0),
        (PrintUnits::Inches, 100.0 / 2.54),
    ] {
        let back = read(&stl(&doc, &options(units, true)).unwrap().main);
        let (min, max) = bounds(&back);
        assert!((max.z - min.z - height).abs() < 1e-3 * height, "{units:?}");
        // Normals stay unit length whatever the scale.
        assert!(facing_out(&back, (min + max) / 2.0), "{units:?}");
    }
}

#[test]
fn z_stays_up_and_places_are_kept() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    place(&mut doc, cube, at(Vec3::new(10.0, 20.0, 30.0)));
    let back = read(&stl(&doc, &options(PrintUnits::Centimetres, true)).unwrap().main);
    let (min, max) = bounds(&back);
    assert!(close(min, Vec3::new(-40.0, -30.0, 30.0)), "{min}");
    assert!(close(max, Vec3::new(60.0, 70.0, 130.0)), "{max}");
}

#[test]
fn a_mirrored_shape_still_faces_out() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    place(
        &mut doc,
        cube,
        Transform {
            scale: Vec3::new(-1.0, 1.0, 1.0),
            ..Transform::default()
        },
    );
    let back = read(&stl(&doc, &ExportOptions::default()).unwrap().main);
    assert!(facing_out(&back, Vec3::new(0.0, 0.0, 500.0)));
    assert!(watertight(&back));
}

#[test]
fn overlapping_shapes_merge_into_one_closed_solid() {
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Cube);
    let second = create(&mut doc, PrimitiveKind::Cube);
    place(&mut doc, second, at(Vec3::new(50.0, 50.0, 50.0)));
    let merged = stl(&doc, &options(PrintUnits::Millimetres, true)).unwrap();
    let back = read(&merged.main);
    assert!(watertight(&back), "one closed surface");
    assert_eq!(merged.summary.objects, 2);
    assert_eq!(merged.summary.triangles, back.len());
    let (min, max) = bounds(&back);
    assert!(close(min, Vec3::new(-500.0, -500.0, 0.0)));
    assert!(close(max, Vec3::new(1000.0, 1000.0, 1500.0)));
    // Unmerged, both cubes go out whole, inside each other.
    let apart = read(&stl(&doc, &options(PrintUnits::Millimetres, false)).unwrap().main);
    assert_eq!(apart.len(), 24);
    // The scene isn't changed.
    assert_eq!(doc.scene().children(None).len(), 2);
}

#[test]
fn a_shape_that_isnt_closed_stops_the_merge_and_says_which() {
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Cube);
    let lid = mesh_object(&mut doc, open_box(&[1]));
    doc.execute(Command::Rename {
        id: lid,
        name: "Box".into(),
    })
    .unwrap();
    let Err(error) = stl(&doc, &options(PrintUnits::Millimetres, true)) else {
        panic!("an open shape can't be merged");
    };
    assert!(matches!(error, ExportError::Merge { .. }), "{error:?}");
    let words = error.to_string();
    assert!(words.contains("couldn't be merged"), "{words}");
    assert!(words.contains("closed solid"), "{words}");
    assert!(words.starts_with("Box"), "names the shape: {words}");
    // Unmerged, it exports.
    let back = read(&stl(&doc, &options(PrintUnits::Millimetres, false)).unwrap().main);
    assert_eq!(back.len(), 12 + 10);
}

#[test]
fn nothing_to_export_is_an_error() {
    let doc = Document::new();
    assert!(matches!(
        stl(&doc, &ExportOptions::default()),
        Err(ExportError::Nothing)
    ));
    // Selected only, with nothing selected.
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Cube);
    doc.clear_selection();
    let selected = ExportOptions {
        what: ExportWhat::Selected,
        ..ExportOptions::default()
    };
    assert!(matches!(stl(&doc, &selected), Err(ExportError::Nothing)));
}

fn corner(point: u32) -> ExportCorner {
    ExportCorner {
        point,
        normal: 0,
        uv: None,
        uv2: None,
        colour: None,
    }
}

fn shape(points: Vec<Vec3>, polygons: &[&[u32]]) -> ExportMesh {
    ExportMesh {
        points,
        polygons: polygons
            .iter()
            .map(|p| ExportPolygon {
                corners: p.iter().map(|k| corner(*k)).collect(),
                material: 0,
            })
            .collect(),
        ..ExportMesh::default()
    }
}

#[test]
fn a_single_triangle_is_134_bytes() {
    let one = shape(vec![Vec3::ZERO, Vec3::X, Vec3::Y], &[&[0, 1, 2]]);
    let (bytes, count) = write_binary([&one].into_iter(), 10.0, "x").unwrap();
    assert_eq!((bytes.len(), count), (134, 1));
    let back = read(&bytes);
    assert!(close(back[0].normal, Vec3::Z));
    assert!(close(back[0].corners[1], Vec3::new(10.0, 0.0, 0.0)));
    // No shapes: a file of no triangles.
    let (empty, count) = write_binary(std::iter::empty(), 1.0, "x").unwrap();
    assert_eq!((empty.len(), count), (84, 0));
}

#[test]
fn degenerate_triangles_are_left_out_or_given_no_normal() {
    let points = vec![Vec3::ZERO, Vec3::X, Vec3::X * 2.0, Vec3::Y];
    // A point twice, a point that isn't there, two corners only: left
    // out. Three points on a line: kept (it joins the surface up), with a
    // zero normal, as STL readers expect of a triangle with no area.
    let weird = shape(
        points,
        &[&[0, 0, 1], &[0, 1, 9], &[0, 1], &[0, 1, 2], &[0, 1, 3]],
    );
    let (bytes, count) = write_binary([&weird].into_iter(), 1.0, "x").unwrap();
    assert_eq!(count, 2);
    let back = read(&bytes);
    assert_eq!(back[0].normal, Vec3::ZERO);
    assert!(close(back[1].normal, Vec3::Z));
    // A polygon of more than three corners goes as a fan.
    let quad = shape(
        vec![Vec3::ZERO, Vec3::X, Vec3::X + Vec3::Y, Vec3::Y],
        &[&[0, 1, 2, 3]],
    );
    assert_eq!(write_binary([&quad].into_iter(), 1.0, "x").unwrap().1, 2);
}

#[test]
fn a_header_never_starts_with_solid() {
    assert!(header("Solid Modeller 2").starts_with(b"binary Solid"));
    assert!(header("Test").starts_with(b"Test\0"));
    let long = "x".repeat(200);
    assert_eq!(header(&long), [b'x'; HEADER_SIZE]);
    // A name cut in the middle of a character still fits.
    let wide = "é".repeat(60);
    assert_eq!(header(&wide).len(), HEADER_SIZE);
}

#[test]
fn the_writer_never_panics_on_odd_shapes() {
    // A fixed-seed fuzz test: random points (some not numbers, some
    // endless), random corners (some pointing nowhere), random scales.
    let mut seed = 0x5eed_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..300 {
        let n = (next() % 12) as usize;
        let points: Vec<Vec3> = (0..n)
            .map(|_| {
                let pick = |r: u64| match r % 20 {
                    0 => f32::NAN,
                    1 => f32::INFINITY,
                    2 => f32::MAX,
                    _ => (r % 2000) as f32 / 10.0 - 100.0,
                };
                Vec3::new(pick(next()), pick(next()), pick(next()))
            })
            .collect();
        let polygons: Vec<Vec<u32>> = (0..next() % 10)
            .map(|_| (0..next() % 7).map(|_| (next() % 16) as u32).collect())
            .collect();
        let lists: Vec<&[u32]> = polygons.iter().map(Vec::as_slice).collect();
        let odd = shape(points, &lists);
        let scale = match next() % 4 {
            0 => 0.0,
            1 => -3.0,
            2 => f32::NAN,
            _ => 10.0,
        };
        let (bytes, count) = write_binary([&odd, &odd].into_iter(), scale, "fuzz").unwrap();
        assert_eq!(bytes.len(), HEADER_SIZE + 4 + count * TRIANGLE_SIZE);
        let back = read(&bytes);
        assert!(back.iter().all(|t| t.normal.is_finite()));
        // Merging odd shapes never panics either: it works or says why.
        let object = |mesh: ExportMesh| ExportedObject {
            id: ObjectId::from_raw(1),
            name: "Odd".into(),
            pivot: Mat4::IDENTITY,
            world: mesh,
            local: ExportMesh::default(),
            materials: vec![scene::Material::standard()],
            collision: scene::Collision::default(),
            collision_parts: Vec::new(),
            levels: Vec::new(),
            outline: None,
        };
        let _ = solids(&[object(odd.clone()), object(odd)], true);
    }
}

#[test]
fn the_print_check_finds_what_troubles_a_slicer() {
    // A clean cube: nothing to say, and its size.
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Cube);
    let check = print_check(&doc, ExportWhat::Everything);
    assert_eq!(check.issues, Vec::new());
    assert_eq!((check.shapes, check.triangles), (1, 12));
    assert!(close(check.size().unwrap(), Vec3::splat(100.0)));
    assert!(!check.stops_merge());
    // A plane: flat, no thickness.
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Plane);
    let check = print_check(&doc, ExportWhat::Everything);
    assert_eq!(check.issues.len(), 1);
    assert_eq!(check.issues[0].kind, IssueKind::NoThickness);
    assert!(check.stops_merge());
    assert!(check.issues[0].words().contains("Shell"));
    // A box with its lid off: open.
    let mut doc = Document::new();
    mesh_object(&mut doc, open_box(&[1]));
    let check = print_check(&doc, ExportWhat::Everything);
    assert_eq!(check.issues[0].kind, IssueKind::Open { edges: 4 });
    assert!(check.issues[0].words().contains("Fill hole"));
    // A box with one side the wrong way round.
    let mut flipped = open_box(&[0]);
    let v = |k: u32| VertexId::from_raw(k);
    flipped.add_face(&[v(1), v(3), v(2), v(0)]).unwrap();
    let mut doc = Document::new();
    mesh_object(&mut doc, flipped);
    let check = print_check(&doc, ExportWhat::Everything);
    assert_eq!(
        check.issues.iter().map(|i| i.kind).collect::<Vec<_>>(),
        vec![IssueKind::Flipped { polygons: 1 }]
    );
    assert!(check.issues[0].words().contains("Fix normals"));
    assert!(!check.stops_merge(), "facing alone doesn't stop a merge");
    // A closed box with an extra wall inside on one edge: non-manifold.
    let mut fin = open_box(&[]);
    let a = fin.add_vertex(Vec3::new(50.0, 50.0, 0.0)).unwrap();
    let b = fin.add_vertex(Vec3::new(50.0, 50.0, 100.0)).unwrap();
    fin.add_face(&[v(0), v(4), b, a]).unwrap();
    let mut doc = Document::new();
    mesh_object(&mut doc, fin);
    let check = print_check(&doc, ExportWhat::Everything);
    assert!(
        check
            .issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::NonManifold { edges: 1 })),
        "{:?}",
        check.issues
    );
    // A polygon squashed flat: no area.
    let mut squashed = open_box(&[]);
    let c = squashed.add_vertex(Vec3::new(0.0, 0.0, 0.0)).unwrap();
    let d = squashed.add_vertex(Vec3::new(0.0, 0.0, 0.0)).unwrap();
    squashed.add_face(&[v(0), c, d]).unwrap();
    let mut check = PrintCheck::default();
    check.add("Squashed", &squashed, Mat4::IDENTITY);
    assert!(
        check
            .issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::NoArea { polygons: 1 }))
    );
    // Nothing shown: nothing to check, no size.
    let check = print_check(&Document::new(), ExportWhat::Everything);
    assert_eq!((check.shapes, check.size()), (0, None));
}

#[test]
fn the_printing_preset_writes_stl() {
    let settings = ExportPreset::Printing.settings();
    assert_eq!(settings.format, ExportFormat::Stl);
    assert!(!ExportPreset::Printing.is_engine());
    assert_eq!(ExportFormat::Stl.extension(), "stl");
    let fixed = ExportFormat::Stl.options(&ExportOptions {
        lod_levels: true,
        godot_names: true,
        ..ExportOptions::default()
    });
    assert!(fixed.triangulate && !fixed.lod_levels && !fixed.godot_names);
    assert_eq!(fixed.to_file(), Mat4::IDENTITY, "the scene's own space");
}

#[test]
fn an_export_to_disk_writes_one_file() {
    let folder = std::env::temp_dir().join(format!("stl-export-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("part.stl");
    let mut doc = Document::new();
    create(&mut doc, PrimitiveKind::Sphere);
    let summary = crate::export::export_to(
        &path,
        &doc,
        ExportFormat::Stl,
        &ExportOptions::default(),
        "Test App 1.0",
    )
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(read(&bytes).len(), summary.triangles);
    assert!(watertight(&read(&bytes)));
    let files: Vec<_> = std::fs::read_dir(&folder).unwrap().collect();
    assert_eq!(files.len(), 1, "nothing beside it");
    std::fs::remove_dir_all(&folder).ok();
}

/// Writes a ball of `polygons` polygons, merged with a cube, and says how
/// long each part took.
fn heavy_run(polygons: usize) -> (std::time::Duration, std::time::Duration, usize) {
    let mut doc = Document::new();
    mesh_object(&mut doc, mesh::samples::heavy_ball(polygons, 100.0));
    let start = std::time::Instant::now();
    let files = stl(&doc, &ExportOptions::default()).unwrap();
    let written = start.elapsed();
    let start = std::time::Instant::now();
    let check = print_check(&doc, ExportWhat::Everything);
    let checked = start.elapsed();
    assert_eq!(check.shapes, 1);
    (written, checked, files.summary.triangles)
}

#[test]
fn a_big_shape_writes_quickly() {
    // A benchmark kept small enough for every test run: 100 000 polygons.
    let (written, checked, triangles) = heavy_run(100_000);
    assert!(triangles >= 190_000, "{triangles}");
    assert!(written.as_secs() < 20, "writing took {written:?}");
    assert!(checked.as_secs() < 20, "checking took {checked:?}");
}

/// Part of the heavy-mesh check (DESIGN.md §4.8): an STL of a shape of
/// millions of polygons. Run with `tools/heavy/heavy-check.sh`.
#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn heavy_stl_check() {
    let polygons: usize = std::env::var("HEAVY_POLYGONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000_000);
    let (written, checked, triangles) = heavy_run(polygons);
    println!();
    println!("Heavy-mesh check, STL export: {polygons} polygons, {triangles} triangles");
    println!(
        "{:<58} {:>8.1}ms",
        "Write the STL file (to memory)",
        written.as_secs_f64() * 1000.0
    );
    println!(
        "{:<58} {:>8.1}ms",
        "Print check",
        checked.as_secs_f64() * 1000.0
    );
    assert!(written.as_secs() < 30 && checked.as_secs() < 30);
}
