//! Tests for painting strokes on a shape.

use super::*;
use crate::brush::Brush;
use crate::surface::PaintSurface;
use mesh::PrimitiveKind;

/// A right-handed view matrix looking from `eye` at `target`.
fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let f = (target - eye).normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);
    Mat4::from_cols(
        glam::Vec4::new(s.x, u.x, -f.x, 0.0),
        glam::Vec4::new(s.y, u.y, -f.y, 0.0),
        glam::Vec4::new(s.z, u.z, -f.z, 0.0),
        glam::Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
    )
}

/// A right-handed perspective (depth 0 to 1).
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let h = 1.0 / (fov_y * 0.5).tan();
    let w = h / aspect;
    let r = far / (near - far);
    Mat4::from_cols(
        glam::Vec4::new(w, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, h, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, r, -1.0),
        glam::Vec4::new(0.0, 0.0, r * near, 0.0),
    )
}

/// A camera 400 cm in front of the cube (looking along +Y at its front),
/// in a 400 × 400 view.
fn front_view() -> View {
    let eye = Vec3::new(0.0, -400.0, 50.0);
    let target = Vec3::new(0.0, 0.0, 50.0);
    let view = look_at(eye, target, Vec3::Z);
    let projection = perspective(0.8, 1.0, 1.0, 5000.0);
    View {
        view_projection: projection * view,
        size: Vec2::splat(400.0),
        eye,
        forward: Vec3::Y,
        flat: false,
    }
}

fn cube() -> PaintSurface {
    PaintSurface::of(&PrimitiveKind::Cube.standard().mesh())
}

/// Every painted texel's place in the texture (x, y), and the triangles'
/// normals (by which side of the cube) they were painted through.
fn painted(image: &Image) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for y in 0..image.size() {
        for x in 0..image.size() {
            if image.pixel(x, y)[3] > 0 {
                out.push((x, y));
            }
        }
    }
    out
}

/// Which side of the cube each painted texel belongs to, from the cube's
/// own UVs: the side whose UV triangle holds it.
fn sides_painted(surface: &PaintSurface, image: &Image) -> BTreeSet<[i32; 3]> {
    let size = image.size() as f32;
    let mut sides = BTreeSet::new();
    for (x, y) in painted(image) {
        let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        for t in surface.triangles() {
            let texel = t.uvs.map(|uv| Vec2::new(uv.x * size, (1.0 - uv.y) * size));
            if let Some(w) = barycentric(p, texel)
                && w.iter().all(|v| *v >= -0.02)
            {
                sides.insert(t.normal.round().as_ivec3().to_array());
            }
        }
    }
    sides
}

fn dab_at(view: &View, world: Vec3, radius: f32) -> Dab {
    let (screen, _) = view.screen(world).unwrap();
    // How big the dab is on the surface: the world size of a point there.
    let one = view.screen(world + Vec3::X).unwrap().0;
    let per_point = 1.0 / one.distance(screen).max(1e-6);
    Dab {
        strength: 1.0,
        screen,
        world,
        radius,
        world_radius: radius * per_point,
    }
}

fn paint(surface: &PaintSurface, view: &View, depth: bool, brush: &Brush, at: Vec3) -> Image {
    let map = DepthMap::of(surface, Mat4::IDENTITY, view);
    let target = Target {
        surface,
        slots: &[0],
        model: Mat4::IDENTITY,
        view,
        depth: depth.then_some(&map),
    };
    let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
    paint_dab(&mut canvas, &target, brush, &dab_at(view, at, brush.radius));
    canvas.image().clone()
}

#[test]
fn a_dab_paints_the_side_it_lands_on() {
    let surface = cube();
    let view = front_view();
    let brush = Brush {
        radius: 20.0,
        color: [255, 0, 0, 255],
        ..Brush::default()
    };
    let image = paint(&surface, &view, true, &brush, Vec3::new(0.0, -50.0, 50.0));
    assert!(!painted(&image).is_empty());
    let sides = sides_painted(&surface, &image);
    assert_eq!(sides, BTreeSet::from([[0, -1, 0]]), "only the front");
    // Its middle is painted at full strength in its colour.
    let (x, y) = painted(&image)[painted(&image).len() / 2];
    assert_eq!(image.pixel(x, y)[0], 255);
}

#[test]
fn a_dab_on_an_edge_paints_both_sides_of_the_seam() {
    let surface = cube();
    // Looking at the front right edge from the front right.
    let eye = Vec3::new(300.0, -300.0, 50.0);
    let view = View {
        view_projection: perspective(0.8, 1.0, 1.0, 5000.0)
            * look_at(eye, Vec3::new(0.0, 0.0, 50.0), Vec3::Z),
        size: Vec2::splat(400.0),
        eye,
        forward: (Vec3::new(0.0, 0.0, 50.0) - eye).normalize(),
        flat: false,
    };
    let brush = Brush {
        radius: 25.0,
        ..Brush::default()
    };
    let image = paint(&surface, &view, true, &brush, Vec3::new(50.0, -50.0, 50.0));
    let sides = sides_painted(&surface, &image);
    assert!(sides.contains(&[0, -1, 0]), "the front: {sides:?}");
    assert!(sides.contains(&[1, 0, 0]), "the right side: {sides:?}");
}

#[test]
fn hidden_and_far_sides_are_left_alone_unless_painting_through() {
    let surface = cube();
    let view = front_view();
    // A huge dab over the whole cube: the front only.
    let brush = Brush {
        radius: 400.0,
        ..Brush::default()
    };
    let image = paint(&surface, &view, true, &brush, Vec3::new(0.0, -50.0, 50.0));
    let sides = sides_painted(&surface, &image);
    assert!(!sides.contains(&[0, 1, 0]), "not the back: {sides:?}");
    // Painting through: the back too.
    let through = Brush {
        through: true,
        ..brush
    };
    let image = paint(&surface, &view, true, &through, Vec3::new(0.0, -50.0, 50.0));
    let sides = sides_painted(&surface, &image);
    assert!(sides.contains(&[0, 1, 0]), "the back: {sides:?}");
}

#[test]
fn overlapping_dabs_in_one_stroke_dont_pile_up() {
    let surface = cube();
    let view = front_view();
    let brush = Brush {
        radius: 20.0,
        flow: 0.5,
        color: [0, 0, 0, 255],
        ..Brush::default()
    };
    let map = DepthMap::of(&surface, Mat4::IDENTITY, &view);
    let target = Target {
        surface: &surface,
        slots: &[0],
        model: Mat4::IDENTITY,
        view: &view,
        depth: Some(&map),
    };
    let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
    let at = Vec3::new(0.0, -50.0, 50.0);
    for _ in 0..5 {
        paint_dab(&mut canvas, &target, &brush, &dab_at(&view, at, 20.0));
    }
    let image = canvas.image();
    let alphas: Vec<u8> = painted(image)
        .iter()
        .map(|&(x, y)| image.pixel(x, y)[3])
        .collect();
    assert!(alphas.iter().all(|a| *a <= 128), "half flow stays half");
    // A second stroke builds up.
    let mut again = StrokeCanvas::new(image.clone());
    paint_dab(&mut again, &target, &brush, &dab_at(&view, at, 20.0));
    let (x, y) = painted(image)
        .into_iter()
        .max_by_key(|&(x, y)| image.pixel(x, y)[3])
        .unwrap();
    assert!(again.image().pixel(x, y)[3] > 180);
}

#[test]
fn the_eraser_takes_paint_away() {
    let mut base = Image::new(64).unwrap();
    base.set_pixel(3, 3, [10, 20, 30, 255]);
    let mut canvas = StrokeCanvas::new(base);
    let eraser = Brush {
        kind: BrushKind::Erase,
        ..Brush::default()
    };
    canvas.cover(3, 3, 1.0, &eraser);
    assert_eq!(canvas.image().pixel(3, 3)[3], 0);
    assert_eq!(canvas.touched().len(), 1);
}

#[test]
fn only_the_texture_sets_slots_are_painted() {
    let surface = cube();
    let view = front_view();
    let map = DepthMap::of(&surface, Mat4::IDENTITY, &view);
    let target = Target {
        surface: &surface,
        slots: &[3],
        model: Mat4::IDENTITY,
        view: &view,
        depth: Some(&map),
    };
    let mut canvas = StrokeCanvas::new(Image::new(64).unwrap());
    let brush = Brush::default();
    paint_dab(
        &mut canvas,
        &target,
        &brush,
        &dab_at(&view, Vec3::new(0.0, -50.0, 50.0), 20.0),
    );
    assert!(canvas.image().is_empty());
}

#[test]
fn dabs_are_spaced_along_the_path() {
    let mut since = None;
    let first = dabs_along(Vec2::ZERO, Vec2::new(10.0, 0.0), 4.0, &mut since);
    assert_eq!(
        first,
        vec![Vec2::ZERO, Vec2::new(4.0, 0.0), Vec2::new(8.0, 0.0)]
    );
    // The next move carries on from where the last dab was.
    let next = dabs_along(Vec2::new(10.0, 0.0), Vec2::new(13.0, 0.0), 4.0, &mut since);
    assert_eq!(next, vec![Vec2::new(12.0, 0.0)]);
    // A tiny move adds nothing.
    assert!(dabs_along(Vec2::new(13.0, 0.0), Vec2::new(13.5, 0.0), 4.0, &mut since).is_empty());
}

#[test]
fn a_placed_and_mirrored_shape_is_painted_where_it_is() {
    let surface = cube();
    let view = front_view();
    // The cube moved 200 cm to the right and mirrored left to right.
    let model = Mat4::from_translation(Vec3::new(200.0, 0.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let map = DepthMap::of(&surface, model, &view);
    let target = Target {
        surface: &surface,
        slots: &[0],
        model,
        view: &view,
        depth: Some(&map),
    };
    let mut canvas = StrokeCanvas::new(Image::new(256).unwrap());
    let brush = Brush::default();
    paint_dab(
        &mut canvas,
        &target,
        &brush,
        &dab_at(&view, Vec3::new(200.0, -50.0, 50.0), 20.0),
    );
    assert!(!canvas.image().is_empty(), "painted where the cube is now");
}

#[test]
fn the_depth_map_sees_the_front() {
    let surface = cube();
    let view = front_view();
    let map = DepthMap::of(&surface, Mat4::IDENTITY, &view);
    let (front, fd) = view.screen(Vec3::new(0.0, -50.0, 50.0)).unwrap();
    let (_, bd) = view.screen(Vec3::new(0.0, 50.0, 50.0)).unwrap();
    assert!(map.visible(front, fd));
    assert!(!map.visible(front, bd), "the back is behind the front");
}
