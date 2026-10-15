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
    assert!(shape.uvs.is_empty());
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
        hardness: Hardness::ByAngle(120.0),
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
