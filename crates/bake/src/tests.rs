//! Tests for the baker.

use super::*;
use mesh::PrimitiveKind;

fn cube() -> PaintSurface {
    PaintSurface::of(&PrimitiveKind::Cube.standard().mesh())
}

fn source(surface: &PaintSurface) -> Source<'_> {
    Source {
        surface,
        slots: &[0],
        size: 128,
    }
}

/// The colour baked where the cube's surface nearest `at` lies.
fn at(image: &Image, surface: &PaintSurface, at: Vec3) -> [u8; 4] {
    let (i, w) = surface.closest(at, &[0]).unwrap();
    let t = &surface.triangles()[i];
    let uv = t.uvs[0] * w[0] + t.uvs[1] * w[1] + t.uvs[2] * w[2];
    let size = image.size() as f32;
    image.pixel((uv.x * size) as u32, ((1.0 - uv.y) * size) as u32)
}

#[test]
fn position_and_normals_say_where_and_which_way() {
    let s = cube();
    let settings = BakeSettings::default();
    let position = bake(&[source(&s)], BakeKind::Position, &settings).unwrap();
    // The front's middle: half across, at the front (Y least), half up.
    let p = at(&position, &s, Vec3::new(0.0, -50.0, 50.0));
    assert!(
        (p[0] as i32 - 128).abs() < 8 && p[1] < 8 && (p[2] as i32 - 128).abs() < 8,
        "{p:?}"
    );
    let normals = bake(&[source(&s)], BakeKind::Normal, &settings).unwrap();
    let top = at(&normals, &s, Vec3::new(0.0, 0.0, 100.0));
    assert!(
        top[2] > 250 && (top[0] as i32 - 128).abs() < 3,
        "up: {top:?}"
    );
}

#[test]
fn occlusion_is_open_on_a_lone_cube_and_dark_in_a_corner() {
    let s = cube();
    let settings = BakeSettings {
        rays: 16,
        ..BakeSettings::default()
    };
    let open = bake(&[source(&s)], BakeKind::Occlusion, &settings).unwrap();
    let front = at(&open, &s, Vec3::new(0.0, -50.0, 50.0));
    assert!(front[0] > 250, "nothing blocks a lone cube: {front:?}");
    // Two boxes in steps: where the higher one's side meets the lower
    // one's top, it is darker than the outside.
    // Each in its own half of the texture.
    let half = |dx: f32| {
        let mut m = PrimitiveKind::Cube.standard().mesh();
        let faces: Vec<_> = m.faces().map(|(f, _)| f).collect();
        let squeeze = glam::Affine2::from_translation(glam::Vec2::new(dx, 0.0))
            * glam::Affine2::from_scale(glam::Vec2::splat(0.5));
        mesh::uvs::transform_uvs(&mut m, &faces, &[], squeeze);
        m
    };
    let mut l = half(0.0);
    let other = half(0.5);
    mesh::parts::append(
        &mut l,
        &other,
        glam::Mat4::from_translation(Vec3::new(100.0, 0.0, -50.0)),
    );
    let s = PaintSurface::of(&l);
    let settings = BakeSettings {
        rays: 32,
        reach: 0.5,
        ..BakeSettings::default()
    };
    let shaded = bake(&[source(&s)], BakeKind::Occlusion, &settings).unwrap();
    let corner = at(&shaded, &s, Vec3::new(49.0, 0.0, 53.0));
    let outside = at(&shaded, &s, Vec3::new(-50.0, 0.0, 50.0));
    assert!(
        i32::from(corner[0]) + 40 < i32::from(outside[0]),
        "corner {corner:?}, outside {outside:?}"
    );
}

#[test]
fn curvature_lights_outer_edges_and_leaves_faces_grey() {
    let s = cube();
    let settings = BakeSettings {
        edge_width: 0.05,
        ..BakeSettings::default()
    };
    let curvature = bake(&[source(&s)], BakeKind::Curvature, &settings).unwrap();
    let middle = at(&curvature, &s, Vec3::new(0.0, -50.0, 50.0));
    assert!((middle[0] as i32 - 128).abs() < 4, "flat: {middle:?}");
    let edge = at(&curvature, &s, Vec3::new(0.0, -50.0, 99.5));
    assert!(edge[0] > 200, "an outer edge: {edge:?}");
}

#[test]
fn ids_and_thickness() {
    let s = cube();
    let settings = BakeSettings {
        rays: 8,
        ..BakeSettings::default()
    };
    let ids = bake(&[source(&s)], BakeKind::Id, &settings).unwrap();
    assert_eq!(at(&ids, &s, Vec3::new(0.0, -50.0, 50.0)), id_colour(0));
    assert_ne!(id_colour(0), id_colour(1));
    // A thin slab is thinner than a cube.
    let thick = bake(&[source(&s)], BakeKind::Thickness, &settings).unwrap();
    let mut slab = mesh::Mesh::new();
    mesh::parts::append(
        &mut slab,
        &PrimitiveKind::Cube.standard().mesh(),
        glam::Mat4::from_scale(Vec3::new(1.0, 0.05, 1.0)),
    );
    let thin_surface = PaintSurface::of(&slab);
    let thin = bake(&[source(&thin_surface)], BakeKind::Thickness, &settings).unwrap();
    let a = at(&thick, &s, Vec3::new(0.0, -50.0, 50.0))[0];
    let b = at(&thin, &thin_surface, Vec3::new(0.0, -2.5, 50.0))[0];
    assert!(b > a + 60, "thin {b}, thick {a}");
}

#[test]
fn uncovered_texels_stay_empty_and_rays_hit_what_is_there() {
    let s = cube();
    let ids = bake(&[source(&s)], BakeKind::Id, &BakeSettings::default()).unwrap();
    assert_eq!(ids.pixel(0, 0)[3], 0, "outside every piece");
    let rays = Rays::of(&s);
    let d = rays
        .hit(Vec3::new(0.0, -200.0, 50.0), Vec3::Y, 1000.0)
        .unwrap();
    assert!((d - 150.0).abs() < 1e-3, "{d}");
    assert!(
        rays.hit(Vec3::new(0.0, -200.0, 50.0), Vec3::NEG_Y, 1000.0)
            .is_none()
    );
    assert!(
        rays.hit(Vec3::new(0.0, -200.0, 50.0), Vec3::Y, 100.0)
            .is_none()
    );
    // An empty shape: nothing to hit, nothing baked.
    let empty = PaintSurface::default();
    assert!(Rays::of(&empty).hit(Vec3::ZERO, Vec3::X, 10.0).is_none());
    let none = bake(
        &[Source {
            surface: &empty,
            slots: &[0],
            size: 64,
        }],
        BakeKind::Occlusion,
        &BakeSettings::default(),
    )
    .unwrap();
    assert!(none.is_empty());
}

#[test]
fn a_gradient_runs_up_the_shape() {
    let s = cube();
    let g = paint::Gradient {
        from: [0, 0, 0, 255],
        to: [255, 255, 255, 255],
        ..paint::Gradient::default()
    };
    let image = gradient(&[source(&s)], &g).unwrap();
    let low = at(&image, &s, Vec3::new(0.0, -50.0, 5.0))[0];
    let middle = at(&image, &s, Vec3::new(0.0, -50.0, 50.0))[0];
    let high = at(&image, &s, Vec3::new(0.0, -50.0, 95.0))[0];
    assert!(
        low < 30 && (110..150).contains(&middle) && high > 225,
        "{low} {middle} {high}"
    );
    // The top is all the high colour, the bottom all the low.
    assert_eq!(at(&image, &s, Vec3::new(0.0, 0.0, 100.0)), [255; 4]);
    assert_eq!(at(&image, &s, Vec3::new(0.0, 0.0, 0.0)), [0, 0, 0, 255]);
    // Along X instead: left dark, right light.
    let across = paint::Gradient {
        axis: paint::GradientAxis::X,
        ..g
    };
    let image = gradient(&[source(&s)], &across).unwrap();
    assert!(
        at(&image, &s, Vec3::new(-45.0, -50.0, 50.0))[0]
            < at(&image, &s, Vec3::new(45.0, -50.0, 50.0))[0]
    );
}
