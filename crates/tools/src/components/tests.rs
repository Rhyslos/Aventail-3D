//! Tests for finding points, edges and polygons under the mouse.

use super::*;
use crate::navigation::{Navigator, ViewKind};
use mesh::{Primitive, PrimitiveKind};

const SIZE: Vec2 = Vec2::new(1000.0, 600.0);

fn cube() -> Mesh {
    PrimitiveKind::Cube.standard().mesh()
}

/// Looking at a 1 m cube from the front, to the right and above.
fn camera() -> Camera {
    Camera::looking_at(Vec3::new(300.0, -500.0, 250.0), Vec3::new(0.0, 0.0, 50.0))
}

fn scene_of(meshes: &[(&Mesh, Mat4, bool)]) -> PickScene {
    PickScene::new(meshes.iter().copied())
}

fn point_at(mesh: &Mesh, p: Vec3) -> VertexId {
    mesh.vertices()
        .find(|(_, q)| q.distance(p) < 1e-3)
        .map(|(v, _)| v)
        .unwrap()
}

fn screen(camera: &Camera, p: Vec3) -> Vec2 {
    camera.project(p, SIZE).unwrap()
}

#[test]
fn a_visible_point_is_found_under_the_mouse() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // The corner nearest the camera.
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let mouse = screen(&camera, corner) + Vec2::new(4.0, -3.0);
    let found = scene.under(ComponentKind::Points, &camera, SIZE, mouse);
    assert_eq!(
        found,
        Some(Hovered {
            object: 0,
            component: Component::Point(point_at(&mesh, corner)),
        })
    );
    // Too far away from any point: nothing.
    let away = screen(&camera, corner) + Vec2::new(POINT_RADIUS + 30.0, 0.0);
    assert_ne!(
        scene
            .under(ComponentKind::Points, &camera, SIZE, away)
            .map(|h| h.component),
        Some(Component::Point(point_at(&mesh, corner)))
    );
}

#[test]
fn a_point_behind_the_shape_is_not_found() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // The corner farthest from the camera, hidden behind the cube.
    let hidden = Vec3::new(-50.0, 50.0, 0.0);
    let found = scene.under(
        ComponentKind::Points,
        &camera,
        SIZE,
        screen(&camera, hidden),
    );
    assert_ne!(
        found.map(|h| h.component),
        Some(Component::Point(point_at(&mesh, hidden)))
    );
}

#[test]
fn another_object_in_front_hides_points_and_polygons() {
    let edited = cube();
    let blocker = cube();
    // The blocker sits between the camera and the edited cube.
    let towards_camera = (camera().position - Vec3::new(0.0, 0.0, 50.0)).normalize() * 200.0;
    let in_front = Mat4::from_translation(towards_camera);
    let scene = scene_of(&[(&edited, Mat4::IDENTITY, true), (&blocker, in_front, false)]);
    let camera = camera();
    let middle = screen(&camera, Vec3::new(0.0, 0.0, 50.0));
    assert_eq!(
        scene.under(ComponentKind::Polygons, &camera, SIZE, middle),
        None
    );
    let corner = Vec3::new(50.0, -50.0, 100.0);
    assert_eq!(
        scene.under(
            ComponentKind::Points,
            &camera,
            SIZE,
            screen(&camera, corner)
        ),
        None,
        "covered by the blocker"
    );
}

#[test]
fn an_edge_is_found_where_it_passes_the_mouse() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // The front-top edge, along X.
    let a = Vec3::new(-50.0, -50.0, 100.0);
    let b = Vec3::new(50.0, -50.0, 100.0);
    let mouse = screen(&camera, a.lerp(b, 0.4)) + Vec2::new(0.0, 3.0);
    let found = scene.under(ComponentKind::Edges, &camera, SIZE, mouse);
    assert_eq!(
        found,
        Some(Hovered {
            object: 0,
            component: Component::Edge(EdgeKey::new(point_at(&mesh, a), point_at(&mesh, b))),
        })
    );
}

#[test]
fn a_hidden_edge_is_not_found() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // The back-bottom edge, along X, behind the cube.
    let a = Vec3::new(-50.0, 50.0, 0.0);
    let b = Vec3::new(50.0, 50.0, 0.0);
    let hidden = EdgeKey::new(point_at(&mesh, a), point_at(&mesh, b));
    let found = scene.under(
        ComponentKind::Edges,
        &camera,
        SIZE,
        screen(&camera, a.lerp(b, 0.5)),
    );
    assert_ne!(found.map(|h| h.component), Some(Component::Edge(hidden)));
}

#[test]
fn the_polygon_under_the_mouse_is_the_nearest_one() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // The middle of the front face (facing -Y).
    let front = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).y < -0.9)
        .unwrap();
    let mouse = screen(&camera, Vec3::new(0.0, -50.0, 50.0));
    assert_eq!(
        scene.under(ComponentKind::Polygons, &camera, SIZE, mouse),
        Some(Hovered {
            object: 0,
            component: Component::Polygon(front),
        })
    );
    // Over the background: nothing.
    assert_eq!(
        scene.under(ComponentKind::Polygons, &camera, SIZE, Vec2::new(5.0, 5.0)),
        None
    );
}

#[test]
fn objects_not_being_edited_offer_no_elements() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, false)]);
    let camera = camera();
    let corner = screen(&camera, Vec3::new(50.0, -50.0, 100.0));
    for kind in [
        ComponentKind::Points,
        ComponentKind::Edges,
        ComponentKind::Polygons,
    ] {
        assert_eq!(scene.under(kind, &camera, SIZE, corner), None, "{kind:?}");
    }
}

#[test]
fn moved_objects_are_found_where_they_are() {
    let mesh = cube();
    let moved = Mat4::from_translation(Vec3::new(200.0, 0.0, 0.0));
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, false), (&mesh, moved, true)]);
    let camera = Camera::looking_at(Vec3::new(100.0, -900.0, 300.0), Vec3::new(100.0, 0.0, 50.0));
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let found = scene
        .under(
            ComponentKind::Points,
            &camera,
            SIZE,
            screen(&camera, moved.transform_point3(corner)),
        )
        .unwrap();
    assert_eq!(found.object, 1);
    assert_eq!(found.component, Component::Point(point_at(&mesh, corner)));
}

#[test]
fn flat_views_find_elements_too() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = *Navigator::for_view(ViewKind::Front).camera();
    // In the front view the front face's top-right corner is in plain sight.
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let found = scene.under(
        ComponentKind::Points,
        &camera,
        SIZE,
        screen(&camera, corner),
    );
    assert_eq!(
        found.map(|h| h.component),
        Some(Component::Point(point_at(&mesh, corner)))
    );
    let middle = screen(&camera, Vec3::new(0.0, 0.0, 50.0));
    assert!(matches!(
        scene
            .under(ComponentKind::Polygons, &camera, SIZE, middle)
            .map(|h| h.component),
        Some(Component::Polygon(_))
    ));
}

#[test]
fn an_n_gon_is_one_polygon() {
    let disc = PrimitiveKind::Disc.standard().mesh();
    let scene = scene_of(&[(&disc, Mat4::IDENTITY, true)]);
    let camera = Camera::looking_at(Vec3::new(0.0, -300.0, 300.0), Vec3::ZERO);
    let face = disc.faces().next().unwrap().0;
    for offset in [
        Vec3::ZERO,
        Vec3::new(30.0, 0.0, 0.0),
        Vec3::new(-20.0, 25.0, 0.0),
    ] {
        let found = scene.under(
            ComponentKind::Polygons,
            &camera,
            SIZE,
            screen(&camera, offset),
        );
        assert_eq!(found.map(|h| h.component), Some(Component::Polygon(face)));
    }
}

#[test]
fn broken_placements_and_empty_meshes_are_ignored() {
    let mesh = cube();
    let empty = Mesh::new();
    let broken = Mat4::from_translation(Vec3::NAN);
    let scene = scene_of(&[(&mesh, broken, true), (&empty, Mat4::IDENTITY, true)]);
    assert_eq!(scene.triangle_count(), 0);
    let camera = camera();
    let anywhere = SIZE / 2.0;
    for kind in [
        ComponentKind::Points,
        ComponentKind::Edges,
        ComponentKind::Polygons,
    ] {
        assert_eq!(scene.under(kind, &camera, SIZE, anywhere), None);
    }
}

#[test]
fn every_primitive_can_be_picked_somewhere() {
    let camera = camera();
    for primitive in Primitive::all() {
        let mesh = primitive.mesh();
        let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
        let centre = mesh.bounds().unwrap().centre();
        // Somewhere near the middle of the shape, a polygon is found.
        let found = (0..40).find_map(|i| {
            let offset = Vec2::new((i % 8) as f32 * 6.0 - 21.0, (i / 8) as f32 * 6.0 - 12.0);
            scene.under(
                ComponentKind::Polygons,
                &camera,
                SIZE,
                screen(&camera, centre) + offset,
            )
        });
        assert!(found.is_some(), "{primitive:?}");
    }
}

#[test]
fn segment_distance_and_ray_hits() {
    let (d, t) = distance_to_segment(Vec2::new(5.0, 3.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert_eq!((d, t), (3.0, 0.5));
    let (d, t) = distance_to_segment(Vec2::new(-4.0, 3.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert_eq!((d, t), (5.0, 0.0), "past the end");
    let (d, _) = distance_to_segment(Vec2::new(3.0, 4.0), Vec2::ZERO, Vec2::ZERO);
    assert_eq!(d, 5.0, "a segment with no length");

    let triangle = [Vec3::ZERO, Vec3::X, Vec3::Y];
    let down = Ray {
        origin: Vec3::new(0.2, 0.2, 5.0),
        direction: Vec3::NEG_Z,
    };
    assert_eq!(hit(&down, &triangle), Some(5.0));
    let up = Ray {
        direction: Vec3::Z,
        ..down
    };
    assert_eq!(hit(&up, &triangle), None, "behind the ray's start");
    let sideways = Ray {
        direction: Vec3::X,
        ..down
    };
    assert_eq!(hit(&sideways, &triangle), None, "parallel");
    let outside = Ray {
        origin: Vec3::new(2.0, 2.0, 5.0),
        ..down
    };
    assert_eq!(hit(&outside, &triangle), None);
}
