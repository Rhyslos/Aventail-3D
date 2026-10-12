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
    let found = scene.under(ElementKind::Points, &camera, SIZE, mouse);
    assert_eq!(
        found,
        Some(Hovered {
            object: 0,
            element: Element::Point(point_at(&mesh, corner)),
        })
    );
    // Too far away from any point: nothing.
    let away = screen(&camera, corner) + Vec2::new(POINT_RADIUS + 30.0, 0.0);
    assert_ne!(
        scene
            .under(ElementKind::Points, &camera, SIZE, away)
            .map(|h| h.element),
        Some(Element::Point(point_at(&mesh, corner)))
    );
}

#[test]
fn a_point_behind_the_shape_is_not_found() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // The corner farthest from the camera, hidden behind the cube.
    let hidden = Vec3::new(-50.0, 50.0, 0.0);
    let found = scene.under(ElementKind::Points, &camera, SIZE, screen(&camera, hidden));
    assert_ne!(
        found.map(|h| h.element),
        Some(Element::Point(point_at(&mesh, hidden)))
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
        scene.under(ElementKind::Polygons, &camera, SIZE, middle),
        None
    );
    let corner = Vec3::new(50.0, -50.0, 100.0);
    assert_eq!(
        scene.under(ElementKind::Points, &camera, SIZE, screen(&camera, corner)),
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
    let found = scene.under(ElementKind::Edges, &camera, SIZE, mouse);
    assert_eq!(
        found,
        Some(Hovered {
            object: 0,
            element: Element::Edge(EdgeKey::new(point_at(&mesh, a), point_at(&mesh, b))),
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
        ElementKind::Edges,
        &camera,
        SIZE,
        screen(&camera, a.lerp(b, 0.5)),
    );
    assert_ne!(found.map(|h| h.element), Some(Element::Edge(hidden)));
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
        scene.under(ElementKind::Polygons, &camera, SIZE, mouse),
        Some(Hovered {
            object: 0,
            element: Element::Polygon(front),
        })
    );
    // Over the background: nothing.
    assert_eq!(
        scene.under(ElementKind::Polygons, &camera, SIZE, Vec2::new(5.0, 5.0)),
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
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
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
            ElementKind::Points,
            &camera,
            SIZE,
            screen(&camera, moved.transform_point3(corner)),
        )
        .unwrap();
    assert_eq!(found.object, 1);
    assert_eq!(found.element, Element::Point(point_at(&mesh, corner)));
}

#[test]
fn flat_views_find_elements_too() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = *Navigator::for_view(ViewKind::Front).camera();
    // In the front view the front face's top-right corner is in plain sight.
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let found = scene.under(ElementKind::Points, &camera, SIZE, screen(&camera, corner));
    assert_eq!(
        found.map(|h| h.element),
        Some(Element::Point(point_at(&mesh, corner)))
    );
    let middle = screen(&camera, Vec3::new(0.0, 0.0, 50.0));
    assert!(matches!(
        scene
            .under(ElementKind::Polygons, &camera, SIZE, middle)
            .map(|h| h.element),
        Some(Element::Polygon(_))
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
            ElementKind::Polygons,
            &camera,
            SIZE,
            screen(&camera, offset),
        );
        assert_eq!(found.map(|h| h.element), Some(Element::Polygon(face)));
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
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
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
                ElementKind::Polygons,
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

// ------------------------------------------------------------- regions ---

/// A box around the whole cube on screen.
fn around_cube(camera: &Camera) -> Region {
    let corners: Vec<Vec2> = cube().vertices().map(|(_, p)| screen(camera, p)).collect();
    let min = corners.iter().copied().reduce(Vec2::min).unwrap() - Vec2::splat(5.0);
    let max = corners.iter().copied().reduce(Vec2::max).unwrap() + Vec2::splat(5.0);
    Region::Rect { a: max, b: min }
}

fn kinds_in(scene: &PickScene, kind: ElementKind, region: &Region, visible_only: bool) -> usize {
    scene
        .elements_in(kind, &camera(), SIZE, region, visible_only)
        .len()
}

#[test]
fn a_box_selects_through_the_shape_unless_only_visible_ones_are_wanted() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let region = around_cube(&camera());
    assert_eq!(kinds_in(&scene, ElementKind::Points, &region, false), 8);
    assert_eq!(kinds_in(&scene, ElementKind::Edges, &region, false), 12);
    assert_eq!(kinds_in(&scene, ElementKind::Polygons, &region, false), 6);
    // From the front, right and above, one corner (and its three edges
    // and three polygons) is out of sight.
    assert_eq!(kinds_in(&scene, ElementKind::Points, &region, true), 7);
    assert_eq!(kinds_in(&scene, ElementKind::Edges, &region, true), 9);
    assert_eq!(kinds_in(&scene, ElementKind::Polygons, &region, true), 3);
}

#[test]
fn a_box_needs_both_ends_of_an_edge_and_a_polygon_s_middle() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // A small box around one corner only.
    let corner = screen(&camera, Vec3::new(50.0, -50.0, 100.0));
    let region = Region::Rect {
        a: corner - Vec2::splat(4.0),
        b: corner + Vec2::splat(4.0),
    };
    assert_eq!(kinds_in(&scene, ElementKind::Points, &region, true), 1);
    assert_eq!(kinds_in(&scene, ElementKind::Edges, &region, true), 0);
    assert_eq!(kinds_in(&scene, ElementKind::Polygons, &region, true), 0);
}

#[test]
fn a_lasso_selects_what_it_surrounds() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    let corner = screen(&camera, Vec3::new(50.0, -50.0, 100.0));
    let lasso = Region::Lasso(vec![
        corner + Vec2::new(-10.0, -10.0),
        corner + Vec2::new(12.0, -6.0),
        corner + Vec2::new(0.0, 12.0),
    ]);
    let found = scene.elements_in(ElementKind::Points, &camera, SIZE, &lasso, true);
    assert_eq!(
        found.iter().map(|h| h.element).collect::<Vec<_>>(),
        vec![Element::Point(point_at(
            &mesh,
            Vec3::new(50.0, -50.0, 100.0)
        ))]
    );
    // An outline of fewer than three points surrounds nothing.
    let line = Region::Lasso(vec![corner, corner + Vec2::X]);
    assert_eq!(kinds_in(&scene, ElementKind::Points, &line, false), 0);
}

#[test]
fn the_brush_catches_edges_it_crosses_and_polygons_under_it() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    // A small brush on the front face, off its middle.
    let spot = screen(&camera, Vec3::new(25.0, -50.0, 30.0));
    let brush = Region::Circle {
        centre: spot,
        radius: 2.0,
    };
    let visible = scene.elements_in(ElementKind::Polygons, &camera, SIZE, &brush, true);
    assert_eq!(visible.len(), 1, "the front polygon, under the brush");
    let through = scene.elements_in(ElementKind::Polygons, &camera, SIZE, &brush, false);
    assert_eq!(through.len(), 2, "the front and the one behind it");
    assert!(through.contains(&visible[0]));
    assert_eq!(kinds_in(&scene, ElementKind::Edges, &brush, false), 0);

    // Over the front-top edge, the brush catches it.
    let a = Vec3::new(-50.0, -50.0, 100.0);
    let b = Vec3::new(50.0, -50.0, 100.0);
    let over_edge = Region::Circle {
        centre: screen(&camera, a.lerp(b, 0.3)) + Vec2::new(0.0, 3.0),
        radius: 5.0,
    };
    let edges = scene.elements_in(ElementKind::Edges, &camera, SIZE, &over_edge, true);
    assert_eq!(
        edges.iter().map(|h| h.element).collect::<Vec<_>>(),
        vec![Element::Edge(EdgeKey::new(
            point_at(&mesh, a),
            point_at(&mesh, b)
        ))]
    );
}

#[test]
fn objects_not_being_edited_offer_nothing_in_a_region() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, false)]);
    let region = around_cube(&camera());
    for kind in [
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
    ] {
        assert_eq!(kinds_in(&scene, kind, &region, false), 0);
    }
}

#[test]
fn whole_objects_are_found_under_the_mouse_and_in_regions() {
    let front = cube();
    let behind = cube();
    let camera = camera();
    let away = (camera.position - Vec3::new(0.0, 0.0, 50.0)).normalize() * -300.0;
    let scene = scene_of(&[
        (&front, Mat4::IDENTITY, false),
        (&behind, Mat4::from_translation(away), false),
    ]);
    let middle = screen(&camera, Vec3::new(0.0, 0.0, 50.0));
    assert_eq!(scene.object_under(&camera, SIZE, middle), Some(0));
    assert_eq!(scene.object_under(&camera, SIZE, Vec2::new(3.0, 3.0)), None);

    // A small box in the middle of the front cube: the cube behind it is
    // only caught when selecting through.
    let small = Region::Rect {
        a: middle - Vec2::splat(3.0),
        b: middle + Vec2::splat(3.0),
    };
    assert_eq!(scene.objects_in(&camera, SIZE, &small, true), vec![0]);
    assert_eq!(scene.objects_in(&camera, SIZE, &small, false), vec![0, 1]);
    // A box just reaching over the front cube's left edge (the edge of
    // its outline) catches it, though no point and not its middle are
    // inside. The same box moved a little further left catches nothing.
    let a = Vec3::new(-50.0, -50.0, 20.0);
    let b = Vec3::new(-50.0, -50.0, 80.0);
    let on_edge = screen(&camera, a.lerp(b, 0.5));
    let reaching = Region::Rect {
        a: on_edge + Vec2::new(-20.0, -2.0),
        b: on_edge + Vec2::new(1.5, 2.0),
    };
    assert_eq!(scene.objects_in(&camera, SIZE, &reaching, true), vec![0]);
    let outside = Region::Rect {
        a: on_edge + Vec2::new(-20.0, -2.0),
        b: on_edge + Vec2::new(-3.0, 2.0),
    };
    assert!(scene.objects_in(&camera, SIZE, &outside, false).is_empty());
}

#[test]
fn double_clicks_find_the_nearest_edge() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, true)]);
    let camera = camera();
    let a = Vec3::new(-50.0, -50.0, 100.0);
    let b = Vec3::new(50.0, -50.0, 100.0);
    let top_front = EdgeKey::new(point_at(&mesh, a), point_at(&mesh, b));
    let near = screen(&camera, a.lerp(b, 0.5)) + Vec2::new(0.0, 6.0);
    assert_eq!(
        scene.edge_near(&camera, SIZE, near, 12.0),
        Some((0, top_front))
    );
    assert_eq!(
        scene.edge_near(&camera, SIZE, Vec2::new(3.0, 3.0), 12.0),
        None
    );

    // Inside the front polygon, just below its top edge.
    let front = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).y < -0.9)
        .unwrap();
    let inside = screen(&camera, Vec3::new(0.0, -50.0, 90.0));
    assert_eq!(
        scene.nearest_edge_of(0, front, &camera, SIZE, inside),
        Some(top_front)
    );
    assert_eq!(scene.nearest_edge_of(5, front, &camera, SIZE, inside), None);
}

#[test]
fn regions_contain_points_and_touch_segments() {
    let rect = Region::Rect {
        a: Vec2::new(10.0, 10.0),
        b: Vec2::ZERO,
    };
    assert!(rect.contains(Vec2::new(5.0, 5.0)));
    assert!(rect.contains(Vec2::new(10.0, 0.0)), "the rim counts");
    assert!(!rect.contains(Vec2::new(11.0, 5.0)));
    assert!(
        rect.touches_segment(Vec2::new(-5.0, 5.0), Vec2::new(15.0, 5.0)),
        "crosses"
    );
    assert!(!rect.touches_segment(Vec2::new(-5.0, 20.0), Vec2::new(15.0, 20.0)));
    assert_eq!(rect.middle(), Vec2::splat(5.0));

    // A bow tie: the two loops are inside, the crossing between them not
    // (the even-odd rule).
    let bow = Region::Lasso(vec![
        Vec2::ZERO,
        Vec2::new(10.0, 10.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(0.0, 10.0),
    ]);
    assert!(bow.contains(Vec2::new(1.0, 5.0)));
    assert!(bow.contains(Vec2::new(9.0, 5.0)));
    assert!(!bow.contains(Vec2::new(5.0, 2.0)));
    assert!(bow.touches_segment(Vec2::new(5.0, -5.0), Vec2::new(5.0, 15.0)));
    assert!(!Region::Lasso(Vec::new()).contains(Vec2::ZERO));
    assert_eq!(Region::Lasso(Vec::new()).middle(), Vec2::ZERO);

    let circle = Region::Circle {
        centre: Vec2::ZERO,
        radius: 5.0,
    };
    assert!(circle.contains(Vec2::new(3.0, 4.0)));
    assert!(circle.touches_segment(Vec2::new(-10.0, 4.0), Vec2::new(10.0, 4.0)));
    assert!(!circle.touches_segment(Vec2::new(-10.0, 6.0), Vec2::new(10.0, 6.0)));

    assert!(
        segments_cross(Vec2::ZERO, Vec2::X, Vec2::X, Vec2::new(1.0, 1.0)),
        "touching"
    );
    assert!(!segments_cross(
        Vec2::ZERO,
        Vec2::X,
        Vec2::Y,
        Vec2::new(1.0, 1.0)
    ));
}

// ------------------------------------------------------------ snapping ---

#[test]
fn snapping_finds_points_then_edges_then_surfaces() {
    let mesh = cube();
    let scene = scene_of(&[(&mesh, Mat4::IDENTITY, false)]);
    let camera = camera();
    let none = |_: usize| false;
    let still = |_: usize, _: VertexId| false;
    let all = SnapTargets::default();
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let near_corner = screen(&camera, corner) + Vec2::new(3.0, 2.0);
    let found = scene
        .snap_target(&camera, SIZE, near_corner, all, &none, &still)
        .unwrap();
    assert!(found.distance(corner) < 1e-3, "the point: {found}");

    // Partway along the front-top edge: a spot on it.
    let a = Vec3::new(-50.0, -50.0, 100.0);
    let on_edge = screen(&camera, a.lerp(corner, 0.4)) + Vec2::new(0.0, 3.0);
    let found = scene
        .snap_target(&camera, SIZE, on_edge, all, &none, &still)
        .unwrap();
    assert!(
        (found.y + 50.0).abs() < 0.5 && (found.z - 100.0).abs() < 0.5,
        "{found}"
    );

    // The middle of the front face: the surface there.
    let middle = screen(&camera, Vec3::new(0.0, -50.0, 50.0));
    let found = scene
        .snap_target(&camera, SIZE, middle, all, &none, &still)
        .unwrap();
    assert!((found.y + 50.0).abs() < 0.5, "{found}");

    // Only what is allowed counts.
    let points_only = SnapTargets {
        points: true,
        edges: false,
        polygons: false,
    };
    assert_eq!(
        scene.snap_target(&camera, SIZE, middle, points_only, &none, &still),
        None
    );
    // What is being moved is never a target.
    let everything = |_: usize| true;
    assert_eq!(
        scene.snap_target(&camera, SIZE, near_corner, all, &everything, &still),
        None
    );
    let corner_id = point_at(&mesh, corner);
    let moving = |_: usize, v: VertexId| v == corner_id;
    let found = scene.snap_target(&camera, SIZE, near_corner, all, &none, &moving);
    assert!(found.is_none_or(|p| p.distance(corner) > 1.0), "{found:?}");
}

// ---------------------------------------------------- heavy shapes, kept ---

/// A bumpy ball and the same ball with a patch of points pulled out.
fn ball_and_pulled() -> (Arc<Mesh>, Arc<Mesh>, Vec<VertexId>) {
    let ball = mesh::samples::heavy_ball(4_000, 50.0);
    let mut pulled = ball.clone();
    let face = ball.faces().nth(1_800).unwrap().1.to_vec();
    for v in &face {
        let p = pulled.position(*v).unwrap();
        pulled.set_position(*v, p + (p - Vec3::new(0.0, 0.0, 50.0)).normalize() * 30.0);
    }
    (Arc::new(ball), Arc::new(pulled), face)
}

/// Everything a scene answers about the view, to compare two scenes.
fn answers(scene: &PickScene, camera: &Camera) -> Vec<String> {
    let mut out = Vec::new();
    for x in (0..1000).step_by(131) {
        for y in (0..600).step_by(83) {
            let at = Vec2::new(x as f32, y as f32);
            for kind in [
                ElementKind::Points,
                ElementKind::Edges,
                ElementKind::Polygons,
            ] {
                out.push(format!("{:?}", scene.under(kind, camera, SIZE, at)));
            }
        }
    }
    let region = Region::Rect {
        a: Vec2::new(300.0, 150.0),
        b: Vec2::new(700.0, 450.0),
    };
    for kind in [
        ElementKind::Points,
        ElementKind::Edges,
        ElementKind::Polygons,
    ] {
        for visible in [false, true] {
            out.push(format!(
                "{:?}",
                scene.elements_in(kind, camera, SIZE, &region, visible)
            ));
        }
    }
    out
}

#[test]
fn an_index_updated_after_a_move_answers_as_a_fresh_one() {
    let (ball, pulled, moved) = ball_and_pulled();
    let camera = Camera::looking_at(Vec3::new(0.0, -260.0, 120.0), Vec3::new(0.0, 0.0, 50.0));
    let kept = MeshPick::new(Arc::clone(&ball));
    // Use every index once, so all of them are updated by the move.
    let before = PickScene::from_picks([(Arc::new(kept), Mat4::IDENTITY, true)]);
    let _ = answers(&before, &camera);
    let kept = before.objects[0].pick.clone().unwrap();
    let updated = kept.moved(Arc::clone(&pulled));
    assert!(Arc::ptr_eq(updated.mesh(), &pulled));
    let fresh = MeshPick::new(Arc::clone(&pulled));
    let a = PickScene::from_picks([(Arc::new(updated), Mat4::IDENTITY, true)]);
    let b = PickScene::from_picks([(Arc::new(fresh), Mat4::IDENTITY, true)]);
    assert_eq!(answers(&a, &camera), answers(&b, &camera));
    // The pulled-out patch is found where it now is.
    let tip = moved
        .iter()
        .map(|v| pulled.position(*v).unwrap())
        .sum::<Vec3>()
        / moved.len() as f32;
    let hovered = a.under(ElementKind::Polygons, &camera, SIZE, screen(&camera, tip));
    assert!(hovered.is_some());
    // A shape with different faces is indexed afresh.
    let other = Arc::new(cube());
    let rebuilt = a.objects[0]
        .pick
        .as_ref()
        .unwrap()
        .moved(Arc::clone(&other));
    assert_eq!(rebuilt.tris.len(), 12);
}

#[test]
fn a_heavy_shape_is_found_through_its_boxes_quickly() {
    let ball = Arc::new(mesh::samples::heavy_ball(60_000, 50.0));
    let camera = Camera::looking_at(Vec3::new(0.0, -260.0, 120.0), Vec3::new(0.0, 0.0, 50.0));
    let scene = PickScene::from_picks([(Arc::new(MeshPick::new(ball)), Mat4::IDENTITY, true)]);
    let middle = Vec2::new(500.0, 300.0);
    let start = std::time::Instant::now();
    for _ in 0..20 {
        assert!(
            scene
                .under(ElementKind::Polygons, &camera, SIZE, middle)
                .is_some()
        );
        assert!(
            scene
                .under(ElementKind::Points, &camera, SIZE, middle)
                .is_some()
        );
    }
    // Generous, even for a slow test machine in debug: a full scan of
    // 120 000 triangles per look would take far longer.
    assert!(start.elapsed().as_secs_f32() < 5.0, "{:?}", start.elapsed());
}
