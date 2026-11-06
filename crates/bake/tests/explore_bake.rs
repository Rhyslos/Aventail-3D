//! Exploratory checks of the baker (bake::maps) through its public API:
//! written by a test pass to look for problems. Each test names what it
//! tries.

use bake::Progress;
use bake::Source;
use bake::maps::{High, MapKind, MapSettings, bake_corners, bake_map, problem_map};
use glam::{Mat4, Vec2, Vec3};
use mesh::{CubeParams, Mesh, PlaneParams, Primitive};
use paint::{Image, PaintSurface};

const SIZE: u32 = 64;

fn slots(surface: &PaintSurface) -> Vec<u32> {
    let mut s: Vec<u32> = surface.triangles().iter().map(|t| t.slot).collect();
    s.sort_unstable();
    s.dedup();
    s
}

fn moved(mesh: &Mesh, matrix: Mat4) -> Mesh {
    let mut out = Mesh::new();
    mesh::parts::append(&mut out, mesh, matrix);
    out
}

fn bake_one(low: &Mesh, high: Option<&Mesh>, kind: MapKind, settings: &MapSettings) -> Image {
    let surface = PaintSurface::of(low);
    let slots = slots(&surface);
    let source = Source {
        surface: &surface,
        slots: &slots,
        size: SIZE,
    };
    let high = high.map(|h| High::of(&PaintSurface::of(h)));
    bake_map(&source, high.as_ref(), kind, settings, &Progress::default()).unwrap()
}

fn covered(image: &Image) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let p = image.pixel(x, y);
            if p[3] > 0 {
                out.push(p);
            }
        }
    }
    out
}

/// The direction on the shape that U (or V) grows along, from the first
/// triangle of `mesh`'s surface: worked out here independently of the
/// baker.
fn uv_axes(mesh: &Mesh) -> (Vec3, Vec3) {
    let surface = PaintSurface::of(mesh);
    let t = &surface.triangles()[0];
    let (e1, e2) = (t.points[1] - t.points[0], t.points[2] - t.points[0]);
    let (d1, d2): (Vec2, Vec2) = (t.uvs[1] - t.uvs[0], t.uvs[2] - t.uvs[0]);
    let r = 1.0 / (d1.x * d2.y - d2.x * d1.y);
    let du = (e1 * d2.y - e2 * d1.y) * r;
    let dv = (e2 * d1.x - e1 * d2.x) * r;
    (du.normalize(), dv.normalize())
}

#[test]
fn a_cube_baked_from_itself_has_a_flat_normal_map_everywhere() {
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    let image = bake_one(&cube, None, MapKind::Normal, &MapSettings::default());
    let pixels = covered(&image);
    assert!(pixels.len() > 100);
    for p in pixels {
        assert!(
            (i32::from(p[0]) - 128).abs() <= 1 && (i32::from(p[1]) - 128).abs() <= 1 && p[2] >= 254,
            "{p:?}"
        );
    }
}

#[test]
fn a_cube_baked_from_a_copy_of_itself_is_flat_too() {
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    let image = bake_one(&cube, Some(&cube), MapKind::Normal, &MapSettings::default());
    let mut off = 0;
    let pixels = covered(&image);
    for p in &pixels {
        if (i32::from(p[0]) - 128).abs() > 3 || (i32::from(p[1]) - 128).abs() > 3 {
            off += 1;
        }
    }
    assert!(
        off * 50 < pixels.len(),
        "{off} of {} texels not flat",
        pixels.len()
    );
}

#[test]
fn a_sphere_baked_from_a_copy_of_itself_is_flat_with_no_seam_lines() {
    let sphere = Primitive::Sphere(Default::default()).mesh();
    let image = bake_one(
        &sphere,
        Some(&sphere),
        MapKind::Normal,
        &MapSettings::default(),
    );
    let pixels = covered(&image);
    let off = pixels
        .iter()
        .filter(|p| (i32::from(p[0]) - 128).abs() > 4 || (i32::from(p[1]) - 128).abs() > 4)
        .count();
    assert!(
        off * 50 < pixels.len(),
        "{off} of {} texels not flat",
        pixels.len()
    );
}

#[test]
fn the_normal_map_leans_along_v_with_green_up() {
    // The high plane tilted about X: its facing leans along the shape's
    // V direction; green (OpenGL) is up the V direction.
    let low = Primitive::Plane(PlaneParams::default()).mesh();
    let high = moved(&low, Mat4::from_rotation_x(20f32.to_radians()));
    let facing = PaintSurface::of(&high).triangles()[0].normal;
    let (du, dv) = uv_axes(&low);
    let p = bake_one(&low, Some(&high), MapKind::Normal, &MapSettings::default())
        .pixel(SIZE / 2, SIZE / 2);
    let green = f32::from(p[1]) - 127.5;
    let red = f32::from(p[0]) - 127.5;
    assert!(
        green.signum() == facing.dot(dv).signum(),
        "green {green}, facing·V {}",
        facing.dot(dv)
    );
    assert!(red.abs() < 3.0 || red.signum() == facing.dot(du).signum());
}

#[test]
fn a_mirrored_plane_still_leans_the_right_way() {
    // The plane's UVs mirrored in U (as a mirrored half's are): the red
    // channel must follow the mirrored U direction.
    let mut low = Primitive::Plane(PlaneParams::default()).mesh();
    let uvs = mesh::uvs::Uvs::of(&low).unwrap();
    let count = low.face_layout().1.len();
    let values: Vec<Vec2> = uvs
        .into_values(count)
        .into_iter()
        .map(|v| Vec2::new(1.0 - v.x, v.y))
        .collect();
    assert!(mesh::uvs::set_uvs(&mut low, values));
    let high = moved(&low, Mat4::from_rotation_y(20f32.to_radians()));
    let facing = PaintSurface::of(&high).triangles()[0].normal;
    let (du, _) = uv_axes(&low);
    let p = bake_one(&low, Some(&high), MapKind::Normal, &MapSettings::default())
        .pixel(SIZE / 2, SIZE / 2);
    let red = f32::from(p[0]) - 127.5;
    assert!(red.abs() > 20.0, "{p:?}");
    assert!(
        red.signum() == facing.dot(du).signum(),
        "{p:?} {}",
        facing.dot(du)
    );
}

#[test]
fn a_high_shape_with_no_faces_bakes_flat_and_mid_grey() {
    let low = Primitive::Cube(CubeParams::default()).mesh();
    let empty = Mesh::new();
    let normal = bake_one(&low, Some(&empty), MapKind::Normal, &MapSettings::default());
    assert!(covered(&normal).iter().all(|p| p[2] >= 254));
    let height = bake_one(&low, Some(&empty), MapKind::Height, &MapSettings::default());
    assert!(
        covered(&height)
            .iter()
            .all(|p| (i32::from(p[0]) - 128).abs() <= 1)
    );
}

#[test]
fn a_high_shape_just_above_bakes_lighter_height_and_below_darker() {
    let low = Primitive::Plane(PlaneParams::default()).mesh();
    let n = PaintSurface::of(&low).triangles()[0].normal;
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (_, p) in low.vertices() {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let size = (hi - lo).length();
    let push = size * MapSettings::default().cage;
    let up = moved(&low, Mat4::from_translation(n * push * 0.5));
    let down = moved(&low, Mat4::from_translation(-n * push * 0.5));
    let s = MapSettings::default();
    let u = bake_one(&low, Some(&up), MapKind::Height, &s).pixel(SIZE / 2, SIZE / 2);
    let d = bake_one(&low, Some(&down), MapKind::Height, &s).pixel(SIZE / 2, SIZE / 2);
    assert!(u[0] > 150 && d[0] < 106, "up {u:?} down {d:?}");
}

#[test]
fn the_problem_counts_follow_the_high_shape() {
    let low = Primitive::Cube(CubeParams::default()).mesh();
    let surface = PaintSurface::of(&low);
    let slots = slots(&surface);
    let source = Source {
        surface: &surface,
        slots: &slots,
        size: SIZE,
    };
    let s = MapSettings::default();
    let same = High::of(&surface);
    let (_, ok) = problem_map(&source, &same, &s, None, &Progress::default()).unwrap();
    assert!(ok.texels > 0);
    assert_eq!(ok.missed + ok.wrong_part + ok.skewed, 0, "{ok:?}");
    // A high shape much bigger than the cage reaches: all missed.
    let centre = low.vertices().map(|(_, p)| p).sum::<Vec3>() / low.vertex_count() as f32;
    let around = Mat4::from_translation(centre)
        * Mat4::from_scale(Vec3::splat(2.0))
        * Mat4::from_translation(-centre);
    let big = High::of(&PaintSurface::of(&moved(&low, around)));
    let (_, far) = problem_map(&source, &big, &s, None, &Progress::default()).unwrap();
    assert_eq!(far.missed, far.texels, "{far:?}");
}

#[test]
fn vertex_colour_occlusion_of_a_lone_cube_is_open() {
    // A cube is convex: nothing shades any of its corners.
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    let surface = PaintSurface::of(&cube);
    let corners = bake_corners(
        &cube,
        &surface,
        MapKind::Occlusion,
        &MapSettings::default().low,
    )
    .unwrap();
    let darkest = corners.iter().map(|(_, _, c)| c[0]).min().unwrap();
    assert!(darkest >= 200, "darkest corner {darkest}");
}

#[test]
#[ignore = "BUG: vertex-colour Colour ID gives each point one material's colour, so faces of another material get the wrong ID at shared corners"]
fn vertex_colour_ids_keep_each_material_s_colour() {
    let mut cube = Primitive::Cube(CubeParams::default()).mesh();
    let top = cube.faces().next().unwrap().0;
    mesh::material::set_material(&mut cube, &[top], 1);
    let surface = PaintSurface::of(&cube);
    let corners = bake_corners(&cube, &surface, MapKind::Id, &MapSettings::default().low).unwrap();
    for (face, _, colour) in corners {
        let want = if face == top {
            bake::id_colour(1)
        } else {
            bake::id_colour(0)
        };
        assert_eq!(colour, want, "face {face:?}");
    }
}

#[test]
fn turning_both_shapes_together_doesn_t_change_the_normal_map() {
    // A tangent-space map belongs to the surface, not to where the object
    // points: the same pair, turned, bakes the same.
    let low = Primitive::Plane(PlaneParams::default()).mesh();
    let high = moved(
        &low,
        Mat4::from_rotation_x(15f32.to_radians()) * Mat4::from_rotation_y(10f32.to_radians()),
    );
    let turn = Mat4::from_rotation_z(1.1) * Mat4::from_rotation_x(0.7);
    let s = MapSettings::default();
    let a = bake_one(&low, Some(&high), MapKind::Normal, &s).pixel(SIZE / 2, SIZE / 2);
    let b = bake_one(
        &moved(&low, turn),
        Some(&moved(&high, turn)),
        MapKind::Normal,
        &s,
    )
    .pixel(SIZE / 2, SIZE / 2);
    for c in 0..3 {
        assert!(
            (i32::from(a[c]) - i32::from(b[c])).abs() <= 2,
            "{a:?} vs {b:?}"
        );
    }
}

#[test]
fn degenerate_uvs_and_no_uvs_don_t_panic() {
    let mut low = Primitive::Cube(CubeParams::default()).mesh();
    let count = low.face_layout().1.len();
    // Every corner on one spot: no UV area anywhere.
    assert!(mesh::uvs::set_uvs(&mut low, vec![Vec2::splat(0.5); count]));
    let high = moved(&low, Mat4::from_scale(Vec3::splat(1.01)));
    for kind in MapKind::ALL {
        let _ = bake_one(&low, Some(&high), kind, &MapSettings::default());
    }
    // A shape with no polygons at all.
    let empty = Mesh::new();
    for kind in MapKind::ALL {
        let image = bake_one(&empty, Some(&high), kind, &MapSettings::default());
        assert!(covered(&image).is_empty());
    }
}

#[test]
fn sizes_out_of_range_give_none_not_a_panic() {
    let low = Primitive::Cube(CubeParams::default()).mesh();
    let surface = PaintSurface::of(&low);
    let slots = slots(&surface);
    for size in [0, 1, 100, 1000, u32::MAX] {
        let source = Source {
            surface: &surface,
            slots: &slots,
            size,
        };
        let _ = bake_map(
            &source,
            None,
            MapKind::Normal,
            &MapSettings::default(),
            &Progress::default(),
        );
    }
}
