//! Tests for getting a scene ready to export.

use super::*;
use scene::{Command, ObjectKind, PrimitiveKind, Transform};

fn with(kind: PrimitiveKind) -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn one(doc: &Document, options: &ExportOptions) -> ExportedObject {
    let mut objects = prepare(doc, options);
    assert_eq!(objects.len(), 1);
    objects.remove(0)
}

#[test]
fn a_cube_exports_shared_points_and_one_normal_per_side() {
    let (doc, _) = with(PrimitiveKind::Cube);
    let cube = one(&doc, &ExportOptions::default());
    assert_eq!(cube.name, "Cube");
    let shape = &cube.world;
    assert_eq!(shape.points.len(), 8);
    assert_eq!(shape.polygons.len(), 6);
    assert_eq!(shape.normals.len(), 6, "crisp sides");
    // Primitives come with UVs (decision 0095): a corner each per side.
    assert!(!shape.uvs.is_empty());
    assert_eq!(shape.triangle_count(), 12);
    // Every polygon's normal matches its winding.
    for polygon in &shape.polygons {
        let p: Vec<Vec3> = polygon
            .corners
            .iter()
            .map(|c| shape.points[c.point as usize])
            .collect();
        let winding = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
        let normal = shape.normals[polygon.corners[0].normal as usize];
        assert!(winding.dot(normal) > 0.99, "{winding} {normal}");
    }
}

#[test]
fn y_up_in_metres_turns_and_scales_the_scene() {
    let (doc, _) = with(PrimitiveKind::Cube);
    let cube = one(&doc, &ExportOptions::default());
    // The cube stands on the ground, 1 m tall: Y from 0 to 1.
    let top = cube
        .world
        .points
        .iter()
        .map(|p| p.y)
        .fold(f32::MIN, f32::max);
    let bottom = cube
        .world
        .points
        .iter()
        .map(|p| p.y)
        .fold(f32::MAX, f32::min);
    assert!((top - 1.0).abs() < 1e-5 && bottom.abs() < 1e-5);
    // Z up, in centimetres: as in the app.
    let options = ExportOptions {
        units: FileUnits::Centimetres,
        up: UpAxis::Z,
        ..ExportOptions::default()
    };
    let same = one(&doc, &options);
    let top = same
        .world
        .points
        .iter()
        .map(|p| p.z)
        .fold(f32::MIN, f32::max);
    assert!((top - 100.0).abs() < 1e-3);
    // A point along +Y in the app goes along -Z in a Y-up file.
    let turned = ExportOptions::default()
        .to_file()
        .transform_point3(Vec3::Y * 100.0);
    assert!(turned.distance(-Vec3::Z) < 1e-5, "{turned}");
}

#[test]
fn triangles_are_cut_the_viewports_way() {
    let (doc, _) = with(PrimitiveKind::Cylinder);
    let options = ExportOptions {
        triangulate: true,
        ..ExportOptions::default()
    };
    let cylinder = one(&doc, &options);
    assert!(cylinder.world.polygons.iter().all(|p| p.corners.len() == 3));
    let shape = doc.scene().evaluated(doc.selection()[0]).unwrap().mesh;
    assert_eq!(
        cylinder.world.polygons.len(),
        mesh::shaded_triangles(&shape).len()
    );
}

#[test]
fn only_shown_shapes_go_and_selected_takes_the_selection() {
    let (mut doc, cube) = with(PrimitiveKind::Cube);
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let sphere = doc.selection()[0];
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    assert_eq!(
        prepare(&doc, &ExportOptions::default()).len(),
        2,
        "no group"
    );
    doc.set_selection([cube]);
    let selected = ExportOptions {
        what: ExportWhat::Selected,
        ..ExportOptions::default()
    };
    let only = prepare(&doc, &selected);
    assert_eq!(only.iter().map(|o| o.id).collect::<Vec<_>>(), vec![cube]);
    doc.execute(Command::SetVisible {
        ids: vec![sphere],
        visible: false,
    })
    .unwrap();
    assert_eq!(prepare(&doc, &ExportOptions::default()).len(), 1);
}

#[test]
fn cutters_stay_out_and_the_shape_they_cut_goes_cut() {
    let (mut doc, cube) = with(PrimitiveKind::Cube);
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cutter = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(
        cutter,
        Transform {
            position: Vec3::splat(50.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    doc.execute(Command::AddBoolean {
        kept: cube,
        cutters: vec![cutter],
        kind: scene::BooleanKind::Subtract,
    })
    .unwrap();
    let out = prepare(&doc, &ExportOptions::default());
    assert_eq!(out.iter().map(|o| o.id).collect::<Vec<_>>(), vec![cube]);
    // The notched cube: nine polygons (24 triangles), not the plain one's
    // six (12).
    let polygons = out[0].world.polygons.len();
    assert!(matches!(polygons, 9 | 24), "{polygons}");
    let summary = summary(&doc, &ExportOptions::default());
    assert_eq!(summary.objects, 1);
    // Selected on its own, a cutter still stays out.
    doc.set_selection([cutter]);
    let selected = ExportOptions {
        what: ExportWhat::Selected,
        ..ExportOptions::default()
    };
    assert!(prepare(&doc, &selected).is_empty());
}

#[test]
fn a_curve_goes_out_as_its_solid_and_a_plain_line_not_at_all() {
    // Decision 0138: a curve with nothing on it has no shape to export.
    let mut doc = Document::new();
    let line = scene::CurveObject::line(scene::CurveShape::Ready(
        scene::ReadyCurveKind::Circle.standard(),
    ));
    doc.execute(Command::Create(ObjectKind::Curve(line.clone())))
        .unwrap();
    assert!(prepare(&doc, &ExportOptions::default()).is_empty());
    assert_eq!(summary(&doc, &ExportOptions::default()).objects, 0);
    let pipe = scene::CurveObject {
        solid: scene::CurveSolid::Sweep(scene::Sweep::default()),
        ..line
    };
    doc.execute(Command::Create(ObjectKind::Curve(pipe)))
        .unwrap();
    let out = one(&doc, &ExportOptions::default());
    assert_eq!(out.name, "Circle 2");
    // A ring of quads with UVs, every polygon facing the way it winds.
    assert!(out.world.polygons.iter().all(|p| p.corners.len() == 4));
    assert!(!out.world.uvs.is_empty());
    assert_eq!(summary(&doc, &ExportOptions::default()).objects, 1);
}

#[test]
fn a_moved_pivot_keeps_the_shape_and_places_the_local_one_around_it() {
    let (mut doc, cube) = with(PrimitiveKind::Cube);
    doc.execute(Command::MovePivots(vec![(
        cube,
        Transform {
            position: Vec3::new(0.0, 0.0, 100.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let options = ExportOptions {
        up: UpAxis::Z,
        units: FileUnits::Centimetres,
        ..ExportOptions::default()
    };
    let moved = one(&doc, &options);
    let low = moved
        .world
        .points
        .iter()
        .map(|p| p.z)
        .fold(f32::MAX, f32::min);
    assert!(low.abs() < 1e-3, "the shape stays on the ground");
    // Around the pivot, the shape hangs below it.
    let high = moved
        .local
        .points
        .iter()
        .map(|p| p.z)
        .fold(f32::MIN, f32::max);
    assert!(high.abs() < 1e-3, "{high}");
    assert!(moved.pivot.w_axis.truncate().distance(Vec3::Z * 100.0) < 1e-3);
}

#[test]
fn a_mirrored_object_keeps_its_polygons_facing_out() {
    let (mut doc, cube) = with(PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(
        cube,
        Transform {
            scale: Vec3::new(-1.0, 1.0, 1.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let shape = one(&doc, &ExportOptions::default()).world;
    for polygon in &shape.polygons {
        let p: Vec<Vec3> = polygon
            .corners
            .iter()
            .map(|c| shape.points[c.point as usize])
            .collect();
        let winding = (p[1] - p[0]).cross(p[2] - p[0]).normalize();
        let normal = shape.normals[polygon.corners[0].normal as usize];
        assert!(winding.dot(normal) > 0.99);
    }
}

#[test]
fn shading_by_angle_or_by_the_marks() {
    let (doc, _) = with(PrimitiveKind::Cube);
    assert_eq!(one(&doc, &ExportOptions::default()).world.normals.len(), 6);
    // Shaded by an angle wider than the cube's corners: smooth all round,
    // one normal at each point.
    let by_angle = ExportOptions {
        hardness: Some(Hardness::ByAngle(120.0)),
        ..ExportOptions::default()
    };
    assert_eq!(one(&doc, &by_angle).world.normals.len(), 8);
}

#[test]
fn uvs_go_with_their_corners() {
    let mut shape = mesh::Primitive::Plane(Default::default()).mesh();
    shape
        .add_attribute(
            mesh::attributes::UV,
            mesh::Domain::Corner,
            Value::Vec2(Vec2::ZERO),
        )
        .unwrap();
    let (face, corners) = shape.faces().next().map(|(f, c)| (f, c.len())).unwrap();
    for i in 0..corners {
        shape
            .set_attribute(
                mesh::attributes::UV,
                AttributeElement::Corner(face, i),
                Value::Vec2(Vec2::new(i as f32 * 0.25, 0.5)),
            )
            .unwrap();
    }
    let out = export_mesh(&shape, Mat4::IDENTITY, &ExportOptions::default());
    let polygon = &out.polygons[0];
    let uvs: Vec<Vec2> = polygon
        .corners
        .iter()
        .map(|c| out.uvs[c.uv.unwrap() as usize])
        .collect();
    assert_eq!(uvs[2], Vec2::new(0.5, 0.5));
}

#[test]
fn a_dented_polygon_is_cut_exactly_as_the_viewport_cuts_it() {
    // An L-shaped polygon: a fan from its first corner would cut outside.
    let mut shape = Mesh::new();
    let ids: Vec<_> = [
        (0.0, 0.0),
        (2.0, 0.0),
        (2.0, 1.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 2.0),
    ]
    .iter()
    .map(|(x, y)| shape.add_vertex(Vec3::new(*x, *y, 0.0)).unwrap())
    .collect();
    shape.add_face(&ids).unwrap();
    let options = ExportOptions {
        triangulate: true,
        up: UpAxis::Z,
        units: FileUnits::Centimetres,
        ..ExportOptions::default()
    };
    let out = export_mesh(&shape, Mat4::IDENTITY, &options);
    let exported: Vec<[Vec3; 3]> = out
        .polygons
        .iter()
        .map(|p| [0, 1, 2].map(|i| out.points[p.corners[i].point as usize]))
        .collect();
    let viewport: Vec<[Vec3; 3]> = mesh::shaded_triangles(&shape)
        .iter()
        .map(|t| t.positions)
        .collect();
    assert_eq!(exported, viewport);
    assert_eq!(exported.len(), 4);
}

#[test]
fn presets_write_what_their_programs_want() {
    let godot = ExportPreset::Godot.settings();
    assert_eq!(godot.format, ExportFormat::Glb);
    let painter = ExportPreset::SubstancePainter.settings();
    assert_eq!(painter.format, ExportFormat::Obj);
    assert_eq!((painter.units, painter.up), (FileUnits::Metres, UpAxis::Y));
    assert!(painter.triangulate);
    // A Substance Painter export is all triangles, with its materials.
    let mut doc = Document::new();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::PrimitiveKind::Cylinder.standard(),
    )))
    .unwrap();
    let options = ExportOptions {
        units: painter.units,
        up: painter.up,
        triangulate: painter.triangulate,
        ..ExportOptions::default()
    };
    let files = export_bytes(&doc, painter.format, &options, "T", "prop.obj").unwrap();
    let text = String::from_utf8(files.main).unwrap();
    assert!(text.contains("mtllib prop.mtl"));
    for line in text.lines().filter(|l| l.starts_with("f ")) {
        assert_eq!(line.split_whitespace().count(), 4, "{line}");
    }
    assert!(
        ExportPreset::ALL
            .iter()
            .all(|p| !p.description().is_empty())
    );
}

#[test]
fn a_live_mirror_goes_out_as_part_of_its_object() {
    let (mut doc, cube) = with(PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(
        cube,
        Transform {
            position: glam::Vec3::new(100.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let plain = one(&doc, &ExportOptions::default());
    doc.execute(Command::SetMirror {
        ids: vec![cube],
        mirror: Some(scene::LiveMirror::across_x()),
    })
    .unwrap();
    let mirrored = one(&doc, &ExportOptions::default());
    assert_eq!(
        mirrored.world.triangle_count(),
        2 * plain.world.triangle_count()
    );
    // The twin is on the other side of the world centre (the file is in
    // metres: from -1.5 to -0.5).
    let lowest = mirrored
        .world
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::MAX, f32::min);
    assert!((lowest + 1.5).abs() < 1e-4, "{lowest}");
}

#[test]
fn the_solis_preset_is_godots_file_without_godots_names() {
    // Decision 0122: Solis reads plain glTF, in metres with Y up, each
    // object at its pivot. Godot's collision names mean nothing to it.
    let solis = ExportPreset::Solis.settings();
    let godot = ExportPreset::Godot.settings();
    assert_eq!(
        (solis.format, solis.units, solis.up, solis.triangulate),
        (godot.format, godot.units, godot.up, godot.triangulate)
    );
    assert!(godot.godot_names);
    assert!(!solis.godot_names);
    assert!(ExportPreset::ALL.contains(&ExportPreset::Solis));
    // Godot stays first, so it stays the one the app starts with.
    assert_eq!(ExportPreset::ALL[0], ExportPreset::Godot);
    assert!(ExportPreset::Solis.is_engine());
    assert!(!ExportPreset::SubstancePainter.is_engine());
    // A Solis export is a glTF file with each object's own name.
    let (doc, _) = with(PrimitiveKind::Cube);
    let options = ExportOptions {
        godot_names: solis.godot_names,
        ..ExportOptions::default()
    };
    let files = export_bytes(&doc, solis.format, &options, "T", "prop.glb").unwrap();
    assert!(files.main.starts_with(b"glTF"));
    assert_eq!(files.summary.objects, 1);
}

#[test]
fn presets_and_file_settings_survive_being_remembered() {
    for preset in ExportPreset::ALL {
        let text = serde_json::to_string(&preset).unwrap();
        assert_eq!(serde_json::from_str::<ExportPreset>(&text).unwrap(), preset);
    }
    let both = (
        ExportFormat::Obj,
        FileUnits::Centimetres,
        UpAxis::Z,
        ExportWhat::Selected,
    );
    let text = serde_json::to_string(&both).unwrap();
    assert_eq!(
        serde_json::from_str::<(ExportFormat, FileUnits, UpAxis, ExportWhat)>(&text).unwrap(),
        both
    );
    // A name the app doesn't know (a damaged file, or a newer app's
    // preset) is refused rather than guessed.
    assert!(serde_json::from_str::<ExportPreset>("\"Blender\"").is_err());
}
