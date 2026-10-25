//! Tests that sampling the screen ([`PickScene::seen_all`]) gives the same
//! answers as looking along each point's own line of sight
//! ([`PickScene::can_see`]), on scenes with outlines, one shape in front of
//! another, holes and a flat view.

use super::*;
use mesh::{Primitive, PrimitiveKind, TorusParams};

/// A small screen, so the areas tried hold more points than pixels and
/// the screen is sampled.
const SMALL: Vec2 = Vec2::new(120.0, 80.0);

/// Every point of the editable meshes seen inside `area`, in the world.
fn spots(scene: &PickScene, camera: &Camera, size: Vec2, area: (Vec2, Vec2)) -> Vec<Vec3> {
    let region = Region::Rect {
        a: area.0,
        b: area.1,
    };
    scene
        .objects
        .iter()
        .filter(|placed| placed.editable)
        .filter_map(|placed| placed.pick.as_ref().map(|pick| (placed, pick)))
        .flat_map(|(placed, pick)| {
            pick.mesh
                .vertices()
                .map(|(_, p)| placed.world(p))
                .filter(|p| camera.project(*p, size).is_some_and(|s| region.contains(s)))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Checks the two ways agree on every point inside `area`, and that the
/// screen was really sampled (more points than pixels); returns how many
/// points were seen.
fn agree(scene: &PickScene, camera: &Camera, size: Vec2, area: (Vec2, Vec2)) -> usize {
    let spots = spots(scene, camera, size, area);
    let pixels = (area.1 - area.0 + Vec2::splat(3.0)).min(size);
    assert!(
        spots.len() >= 256 && (pixels.x * pixels.y) as usize <= spots.len() / 2,
        "{} points over {pixels} pixels: not sampled",
        spots.len()
    );
    let exact: Vec<bool> = spots
        .iter()
        .map(|p| scene.can_see(camera, size, *p))
        .collect();
    let sampled = scene.seen_all(camera, size, area, &spots);
    // Where they differ, the exact ray must be one that slips through
    // the crack between two triangles (it runs exactly along an edge of
    // the surface in front): a hair to the side, it agrees with the
    // sampled answer.
    let differ: Vec<(Vec3, bool)> = spots
        .iter()
        .zip(exact.iter().zip(&sampled))
        .filter(|(_, (a, b))| a != b)
        .map(|(p, (_, b))| (*p, *b))
        .collect();
    for (p, sampled) in &differ {
        assert!(
            exact_is_unsteady(scene, camera, size, *p, *sampled),
            "{p} differs: exact {}, sampled {sampled}",
            !sampled
        );
    }
    assert!(
        differ.len() * 200 <= spots.len(),
        "{} of {} differ",
        differ.len(),
        spots.len()
    );
    let seen = exact.iter().filter(|s| **s).count();
    assert!(seen > 0 && seen < spots.len(), "some seen, some hidden");
    seen
}

/// Whether looking at `point` along a line of sight a hair to one side
/// (a hundredth of a pixel) gives `answer`.
fn exact_is_unsteady(
    scene: &PickScene,
    camera: &Camera,
    size: Vec2,
    point: Vec3,
    answer: bool,
) -> bool {
    let Some(screen) = camera.project(point, size) else {
        return false;
    };
    [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y].iter().any(|step| {
        let ray = camera.ray_through(screen + *step * 0.01, size);
        let depth = (point - ray.origin).dot(ray.direction);
        let seen = scene
            .first_hit(&ray)
            .is_none_or(|(distance, ..)| distance >= depth - seen_tolerance(depth));
        seen == answer
    })
}

fn whole(size: Vec2) -> (Vec2, Vec2) {
    (Vec2::ZERO, size)
}

fn ball(polygons: usize) -> Mesh {
    mesh::samples::heavy_ball(polygons, 100.0)
}

#[test]
fn a_ball_with_its_outline_and_back() {
    let mesh = ball(20_000);
    let scene = PickScene::new([(&mesh, Mat4::IDENTITY, true)]);
    let camera = Camera::looking_at(Vec3::new(250.0, -400.0, 250.0), Vec3::new(0.0, 0.0, 100.0));
    agree(&scene, &camera, SMALL, whole(SMALL));
    // A box over the outline only.
    let edge = camera.project(Vec3::new(100.0, 0.0, 100.0), SMALL).unwrap();
    agree(
        &scene,
        &camera,
        SMALL,
        (edge - Vec2::splat(15.0), edge + Vec2::splat(15.0)),
    );
}

#[test]
fn a_shape_in_front_of_another_hides_part_of_it() {
    // A ball, and a box (not being edited) standing in front of its left
    // half.
    let mesh = ball(20_000);
    let block = PrimitiveKind::Cube.standard().mesh();
    let place = Mat4::from_translation(Vec3::new(-60.0, -220.0, 40.0));
    let scene = PickScene::new([(&mesh, Mat4::IDENTITY, true), (&block, place, false)]);
    let camera = Camera::looking_at(Vec3::new(0.0, -500.0, 150.0), Vec3::new(0.0, 0.0, 100.0));
    agree(&scene, &camera, SMALL, whole(SMALL));
}

#[test]
fn a_ring_seen_through_its_hole() {
    let ring = Primitive::Torus(TorusParams {
        radius: 100.0,
        tube_radius: 30.0,
        sides: 256,
        tube_sides: 96,
    })
    .mesh();
    let scene = PickScene::new([(&ring, Mat4::IDENTITY, true)]);
    let camera = Camera::looking_at(Vec3::new(0.0, -260.0, 300.0), Vec3::ZERO);
    agree(&scene, &camera, SMALL, whole(SMALL));
}

#[test]
fn a_flat_view() {
    let mesh = ball(20_000);
    let scene = PickScene::new([(&mesh, Mat4::IDENTITY, true)]);
    let mut camera =
        Camera::looking_at(Vec3::new(300.0, -500.0, 300.0), Vec3::new(0.0, 0.0, 100.0));
    camera.ortho_height = Some(260.0);
    agree(&scene, &camera, SMALL, whole(SMALL));
}

#[test]
fn box_select_finds_the_same_points_either_way() {
    // Through `elements_in`, which samples: the same as each point's own
    // ray.
    let mesh = ball(20_000);
    let scene = PickScene::new([(&mesh, Mat4::IDENTITY, true)]);
    let camera = Camera::looking_at(Vec3::new(250.0, -400.0, 250.0), Vec3::new(0.0, 0.0, 100.0));
    let region = Region::Rect {
        a: Vec2::ZERO,
        b: SMALL,
    };
    let found = scene.elements_in(ElementKind::Points, &camera, SMALL, &region, true);
    let expected: Vec<Hovered> = scene
        .elements_in(ElementKind::Points, &camera, SMALL, &region, false)
        .into_iter()
        .filter(|h| {
            let Element::Point(id) = h.element else {
                return false;
            };
            scene.can_see(&camera, SMALL, mesh.position(id).unwrap())
        })
        .collect();
    assert!(!found.is_empty());
    assert_eq!(found, expected);
}

#[test]
fn few_points_or_a_big_area_use_each_points_own_ray() {
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let scene = PickScene::new([(&mesh, Mat4::IDENTITY, true)]);
    let camera = Camera::looking_at(Vec3::new(300.0, -500.0, 250.0), Vec3::new(0.0, 0.0, 50.0));
    let size = Vec2::new(1000.0, 600.0);
    let spots: Vec<Vec3> = mesh.vertices().map(|(_, p)| p).collect();
    let exact: Vec<bool> = spots
        .iter()
        .map(|p| scene.can_see(&camera, size, *p))
        .collect();
    assert_eq!(scene.seen_all(&camera, size, whole(size), &spots), exact);
    // Seven of the cube's eight corners show.
    assert_eq!(exact.iter().filter(|s| **s).count(), 7);
}
