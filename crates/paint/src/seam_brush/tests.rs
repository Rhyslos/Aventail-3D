//! Tests for the seam blend brush (decision 0294).

use super::*;
use crate::brush::{Brush, BrushKind};
use crate::seams::seams_of;
use crate::seams::tests::{side_by_side, two_pieces};
use crate::stroke::{Dab, DabJob, DepthMap, StrokeCanvas, Target, View, paint_dabs};
use glam::{Mat4, Vec2};

const RED: [u8; 4] = [220, 20, 20, 255];
const BLUE: [u8; 4] = [20, 20, 220, 255];
const SIZE: u32 = 128;

/// The texture with the left half red and the right half blue: each of
/// the test pieces painted its own colour.
fn halves() -> Image {
    let mut image = Image::new(SIZE).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE {
            image.set_pixel(x, y, if x < SIZE / 2 { RED } else { BLUE });
        }
    }
    image
}

fn texel(uv: Vec2) -> (u32, u32) {
    let s = SIZE as f32;
    ((uv.x * s) as u32, ((1.0 - uv.y) * s) as u32)
}

fn purple(p: [u8; 4]) -> bool {
    p[0] > 70 && p[2] > 70 && p[0].abs_diff(p[2]) < 60
}

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
fn perspective(fov_y: f32, near: f32, far: f32) -> Mat4 {
    let h = 1.0 / (fov_y * 0.5).tan();
    let r = far / (near - far);
    Mat4::from_cols(
        glam::Vec4::new(h, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, h, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, r, -1.0),
        glam::Vec4::new(0.0, 0.0, r * near, 0.0),
    )
}

/// A camera looking straight at the unit square (in z = 0) from in front.
fn front() -> View {
    let eye = Vec3::new(0.5, 0.5, 3.0);
    let view = look_at(eye, Vec3::new(0.5, 0.5, 0.0), Vec3::Y);
    View {
        view_projection: perspective(0.5, 0.1, 100.0) * view,
        size: Vec2::splat(400.0),
        eye,
        forward: -Vec3::Z,
        flat: false,
    }
}

/// A dab at `world` reaching `world_radius` on the surface.
fn dab(view: &View, world: Vec3, world_radius: f32) -> Dab {
    let (screen, _) = view.screen(world).unwrap();
    let edge = view.screen(world + Vec3::X * world_radius).unwrap().0;
    Dab {
        screen,
        world,
        radius: edge.distance(screen),
        world_radius,
        strength: 1.0,
        angle: 0.0,
        seed: 0,
    }
}

/// `image` with seam blend dabs at `spots` on `surface` (radius
/// `world_radius`), `flow` strong; and whether each found a seam under it.
fn stroked(
    surface: &Arc<PaintSurface>,
    image: Image,
    spots: &[Vec3],
    world_radius: f32,
    flow: f32,
) -> (Image, Vec<bool>) {
    let view = front();
    let depth = DepthMap::of(surface, Mat4::IDENTITY, &view);
    let seams = Arc::new(seams_of(surface, &[0]));
    let shape = Arc::new(SeamBrushShape::new(
        Arc::clone(surface),
        &[0],
        seams,
        image.size(),
    ));
    let mut canvas = StrokeCanvas::new(image);
    canvas.set_seams(shape);
    let brush = Brush {
        kind: BrushKind::SeamBlend,
        hardness: 1.0,
        flow,
        ..Brush::default()
    };
    let mut under = Vec::new();
    for &at in spots {
        under.push(canvas.reach_seams(at, world_radius));
        let mut brush = brush;
        let d = dab(&view, at, world_radius);
        brush.radius = d.radius;
        let job = DabJob {
            target: Target {
                surface,
                slots: &[0],
                model: Mat4::IDENTITY,
                view: &view,
                depth: Some(&depth),
            },
            brush,
            dab: d,
        };
        paint_dabs(&mut canvas, &[job]);
    }
    (canvas.image().clone(), under)
}

/// The colour gap across the seam's middle: a texel in from each side.
fn gap(image: &Image) -> u32 {
    let (lx, ly) = texel(Vec2::new(0.25, 0.25));
    let (rx, ry) = texel(Vec2::new(0.75, 0.25));
    let (a, b) = (image.pixel(lx + 1, ly), image.pixel(rx - 1, ry));
    (0..3).map(|c| u32::from(a[c].abs_diff(b[c]))).sum()
}

#[test]
fn a_dab_on_a_seam_closes_it_from_both_sides() {
    let surface = Arc::new(side_by_side());
    let (out, under) = stroked(&surface, halves(), &[Vec3::new(0.5, 0.5, 0.0)], 0.2, 1.0);
    assert_eq!(under, [true]);
    let (lx, ly) = texel(Vec2::new(0.25, 0.25));
    let (rx, ry) = texel(Vec2::new(0.75, 0.25));
    assert!(purple(out.pixel(lx + 1, ly)), "{:?}", out.pixel(lx + 1, ly));
    assert!(purple(out.pixel(rx - 1, ry)), "{:?}", out.pixel(rx - 1, ry));
    assert!(gap(&out) < gap(&halves()) / 4, "{}", gap(&out));
    // Farther from the seam inside the dab, less pulled; outside the
    // pieces nothing is written.
    let near = out.pixel(lx + 1, ly);
    let farther = out.pixel(lx + 5, ly);
    assert!(farther[0] > near[0], "{near:?} {farther:?}");
    assert_eq!(out.pixel(2, 2), RED);
    assert_eq!(out.pixel(SIZE - 2, SIZE - 2), BLUE);
}

#[test]
fn a_weaker_flow_closes_it_less() {
    let surface = Arc::new(side_by_side());
    let at = [Vec3::new(0.5, 0.5, 0.0)];
    let (strong, _) = stroked(&surface, halves(), &at, 0.2, 1.0);
    let (weak, _) = stroked(&surface, halves(), &at, 0.2, 0.3);
    assert!(gap(&weak) > gap(&strong), "{} {}", gap(&weak), gap(&strong));
    assert!(gap(&weak) < gap(&halves()));
}

#[test]
fn away_from_seams_it_does_nothing() {
    let surface = Arc::new(side_by_side());
    // The far corner of the square, well away from its diagonal seam.
    let (out, under) = stroked(&surface, halves(), &[Vec3::new(0.95, 0.05, 0.0)], 0.1, 1.0);
    assert_eq!(under, [false]);
    for y in 0..SIZE {
        for x in 0..SIZE {
            assert_eq!(out.pixel(x, y), halves().pixel(x, y), "{x} {y}");
        }
    }
}

#[test]
fn a_stroke_coming_to_a_seam_later_still_blends_what_it_crosses() {
    // The first dab is away from the seam; the next, overlapping it, is
    // on it: texels the first left alone are blended by the second.
    let surface = Arc::new(side_by_side());
    let spots = [Vec3::new(0.75, 0.3, 0.0), Vec3::new(0.6, 0.45, 0.0)];
    let (out, under) = stroked(&surface, halves(), &spots, 0.15, 1.0);
    assert_eq!(under, [false, true]);
    assert!(gap(&out) < gap(&halves()));
}

#[test]
fn matching_colours_and_unpainted_sides_stay_as_they_are() {
    let surface = Arc::new(side_by_side());
    let at = [Vec3::new(0.5, 0.5, 0.0)];
    let mut same = Image::new(SIZE).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE {
            same.set_pixel(x, y, RED);
        }
    }
    let (out, _) = stroked(&surface, same.clone(), &at, 0.2, 1.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            assert_eq!(out.pixel(x, y), RED);
        }
    }
    // Only the left half painted: nothing across to pull towards.
    let mut left = Image::new(SIZE).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE / 2 {
            left.set_pixel(x, y, RED);
        }
    }
    let (out, _) = stroked(&surface, left.clone(), &at, 0.2, 1.0);
    for y in 0..SIZE {
        for x in 0..SIZE {
            assert_eq!(out.pixel(x, y), left.pixel(x, y));
        }
    }
}

#[test]
fn without_a_shape_or_at_another_size_it_does_nothing() {
    let surface = Arc::new(side_by_side());
    let view = front();
    let mut canvas = StrokeCanvas::new(halves());
    assert!(!canvas.reach_seams(Vec3::new(0.5, 0.5, 0.0), 0.2));
    let d = dab(&view, Vec3::new(0.5, 0.5, 0.0), 0.2);
    let brush = Brush {
        kind: BrushKind::SeamBlend,
        radius: d.radius,
        ..Brush::default()
    };
    let target = Target {
        surface: &surface,
        slots: &[0],
        model: Mat4::IDENTITY,
        view: &view,
        depth: None,
    };
    crate::stroke::paint_dab(&mut canvas, &target, &brush, &d);
    assert_eq!(gap(canvas.image()), gap(&halves()));
    // Made for another texture size: left alone.
    let seams = Arc::new(seams_of(&surface, &[0]));
    let shape = SeamBrushShape::new(Arc::clone(&surface), &[0], seams, SIZE * 2);
    assert!(shape.is_for(&surface, SIZE * 2) && !shape.is_for(&surface, SIZE));
    canvas.set_seams(Arc::new(shape));
    assert!(!canvas.reach_seams(Vec3::new(0.5, 0.5, 0.0), 0.2));
}

#[test]
fn seams_near_a_point_are_found_by_their_edge() {
    let surface = Arc::new(side_by_side());
    let seams = Arc::new(seams_of(&surface, &[0]));
    let shape = SeamBrushShape::new(Arc::clone(&surface), &[0], seams, SIZE);
    assert_eq!(shape.seams().len(), 1);
    assert_eq!(shape.seams_near(Vec3::new(0.5, 0.5, 0.0), 0.01), [0]);
    assert!(shape.seams_near(Vec3::new(0.9, 0.1, 0.0), 0.1).is_empty());
    assert!(shape.seams_near(Vec3::NAN, 0.1).is_empty());
    assert!(shape.seams_near(Vec3::ZERO, f32::NAN).is_empty());
    assert!(shape.seams_near(Vec3::ZERO, -1.0).is_empty());
}

#[test]
fn odd_uvs_and_sizes_never_crash_the_brush() {
    let mut state = 294u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as f32 / (1u64 << 31) as f32
    };
    let odd = [f32::NAN, f32::INFINITY, -3.0, 5.0, 1e9, 1e-9, 0.0, 1.0];
    for round in 0..150 {
        let mut uv = || {
            let pick = next();
            if pick < 0.2 {
                odd[(next() * odd.len() as f32) as usize % odd.len()]
            } else {
                next() * 1.4 - 0.2
            }
        };
        let mut corners = [Vec2::ZERO; 6];
        for c in &mut corners {
            *c = Vec2::new(uv(), uv());
        }
        let surface = Arc::new(two_pieces(
            [corners[0], corners[1], corners[2]],
            [corners[3], corners[4], corners[5]],
        ));
        let radius = [0.0, 0.05, 0.3, 50.0, f32::NAN][round % 5];
        let (out, _) = stroked(
            &surface,
            halves(),
            &[Vec3::new(0.5, 0.5, 0.0), Vec3::new(0.2, 0.2, 0.0)],
            radius,
            1.0,
        );
        assert_eq!(out.size(), SIZE);
    }
}

/// How fast seam blend dabs paint against blend dabs, along a seam of the
/// cube on a 4096 texture (an ignored timing: `cargo test --release -p
/// paint seam_brush_speed -- --ignored --nocapture`).
#[test]
#[ignore = "a timing, run by hand"]
fn seam_brush_speed() {
    let mesh = mesh::PrimitiveKind::Cube.standard().mesh();
    let surface = Arc::new(PaintSurface::of(&mesh));
    let size = 4096;
    let mut image = Image::new(size).unwrap();
    for y in (0..size).step_by(3) {
        for x in 0..size {
            image.set_pixel(x, y, [(x % 251) as u8, (y % 241) as u8, 90, 255]);
        }
    }
    let eye = Vec3::new(160.0, -160.0, 160.0);
    let view = View {
        view_projection: perspective(0.8, 1.0, 5000.0)
            * look_at(eye, Vec3::new(0.0, 0.0, 50.0), Vec3::Z),
        size: Vec2::splat(800.0),
        eye,
        forward: (Vec3::new(0.0, 0.0, 50.0) - eye).normalize(),
        flat: false,
    };
    let depth = DepthMap::of(&surface, Mat4::IDENTITY, &view);
    let seams = Arc::new(seams_of(&surface, &[0]));
    for kind in [BrushKind::Blend, BrushKind::SeamBlend] {
        for world_radius in [5.0, 20.0] {
            let started = std::time::Instant::now();
            let shape = Arc::new(SeamBrushShape::new(
                Arc::clone(&surface),
                &[0],
                Arc::clone(&seams),
                size,
            ));
            let ready = started.elapsed();
            let mut canvas = StrokeCanvas::new(image.clone());
            canvas.set_seams(shape);
            let count = 40;
            let start = std::time::Instant::now();
            for i in 0..count {
                // Down the cube's front right edge.
                let at = Vec3::new(50.0, -50.0, 5.0 + 90.0 * i as f32 / count as f32);
                let d = dab(&view, at, world_radius);
                if kind == BrushKind::SeamBlend {
                    canvas.reach_seams(at, world_radius);
                }
                let brush = Brush {
                    kind,
                    radius: d.radius,
                    ..Brush::default()
                };
                let job = DabJob {
                    target: Target {
                        surface: &surface,
                        slots: &[0],
                        model: Mat4::IDENTITY,
                        view: &view,
                        depth: Some(&depth),
                    },
                    brush,
                    dab: d,
                };
                paint_dabs(&mut canvas, &[job]);
            }
            let took = start.elapsed();
            println!(
                "{kind:?}, radius {world_radius} cm: {:.2} ms a dab (shape made in {:.2} ms)",
                took.as_secs_f64() * 1000.0 / count as f64,
                ready.as_secs_f64() * 1000.0
            );
        }
    }
}

/// The square folded along its seam into a corner: the right triangle
/// turned to face along +X, edge on to the camera.
fn folded() -> PaintSurface {
    use crate::surface::PaintTriangle;
    let flat = side_by_side();
    let mut triangles: Vec<PaintTriangle> = flat.triangles().to_vec();
    // Point 3 (0, 1, 0) moved behind, so the second triangle (0, 2, 3)
    // stands edge on, along the seam from (0, 0, 0) to (1, 1, 0).
    triangles[1].points[2] = Vec3::new(0.5, 0.5, -0.7);
    let n = (triangles[1].points[1] - triangles[1].points[0])
        .cross(triangles[1].points[2] - triangles[1].points[0])
        .normalize();
    triangles[1].normal = if n.z > 0.0 { n } else { -n };
    PaintSurface::from_triangles(triangles)
}

#[test]
fn a_side_turned_away_round_a_corner_is_blended_too() {
    // Seen head on from the front, the other side of the seam faces
    // along the screen: it still moves, so both meet.
    let surface = Arc::new(folded());
    let (out, under) = stroked(&surface, halves(), &[Vec3::new(0.5, 0.5, 0.0)], 0.2, 1.0);
    assert_eq!(under, [true]);
    let (rx, ry) = texel(Vec2::new(0.75, 0.25));
    assert!(purple(out.pixel(rx - 1, ry)), "{:?}", out.pixel(rx - 1, ry));
    assert!(gap(&out) < gap(&halves()) / 4, "{}", gap(&out));
}

#[test]
fn a_seam_blend_stroke_resizes_its_layer_as_a_picture() {
    // Effect strokes aren't painted again at a new size (decision 0191):
    // the layer is resized as a picture, as for blend and blur.
    use crate::set::{DabRecord, Layer, StrokeRecord, TextureSet};
    let mut set = TextureSet::new(SIZE).unwrap();
    let mut layer = Layer::paint("Paint", halves());
    layer.strokes.push(Arc::new(StrokeRecord {
        kind: BrushKind::SeamBlend,
        color: [0; 4],
        hardness: 0.5,
        flow: 1.0,
        through: false,
        dabs: vec![DabRecord::flat(Vec2::splat(0.5), 0.1, 1.0)],
        fills: Vec::new(),
        tip: crate::tip::Tip::Round,
        alignment: crate::brush::Alignment::Camera,
    }));
    set.layers.push(layer);
    let bigger = crate::resize::resized(&set, SIZE * 2, &[]).unwrap();
    let image = bigger.layers[0].image().unwrap();
    assert_eq!(image.size(), SIZE * 2);
    assert_eq!(image.pixel(10, 10), RED);
    assert_eq!(image.pixel(SIZE * 2 - 10, 10), BLUE);
}
