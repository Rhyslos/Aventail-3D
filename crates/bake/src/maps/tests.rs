//! Tests for the baker's maps (decision 0250).

use super::*;
use glam::Mat4;
use mesh::{Mesh, PlaneParams, Primitive};

const SIZE: u32 = 32;

fn plane() -> Mesh {
    Primitive::Plane(PlaneParams::default()).mesh()
}

/// `mesh` moved by `matrix`.
fn moved(mesh: &Mesh, matrix: Mat4) -> Mesh {
    let mut out = Mesh::new();
    mesh::parts::append(&mut out, mesh, matrix);
    out
}

fn slots(surface: &PaintSurface) -> Vec<u32> {
    let mut s: Vec<u32> = surface.triangles().iter().map(|t| t.slot).collect();
    s.sort_unstable();
    s.dedup();
    s
}

fn bake(low: &Mesh, high: Option<&Mesh>, kind: MapKind, settings: &MapSettings) -> Image {
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

/// The middle texel's colour.
fn middle(image: &Image) -> [u8; 4] {
    image.pixel(SIZE / 2, SIZE / 2)
}

#[test]
fn with_nothing_to_bake_from_the_normal_map_is_flat() {
    let image = bake(&plane(), None, MapKind::Normal, &MapSettings::default());
    let [r, g, b, a] = middle(&image);
    assert!((i32::from(r) - 128).abs() <= 1 && (i32::from(g) - 128).abs() <= 1, "{r} {g}");
    assert!(b >= 254 && a == 255);
}

#[test]
fn a_tilted_high_shape_shows_in_the_normal_map_and_green_down_flips_it() {
    let low = plane();
    let high = moved(&low, Mat4::from_rotation_x(20f32.to_radians()));
    let up = middle(&bake(&low, Some(&high), MapKind::Normal, &MapSettings::default()));
    let down = middle(&bake(
        &low,
        Some(&high),
        MapKind::Normal,
        &MapSettings {
            green_down: true,
            ..Default::default()
        },
    ));
    // Tilted about X: the facing leans along the texture's up and down,
    // not across.
    assert!((i32::from(up[0]) - 128).abs() <= 2, "{up:?}");
    assert!((i32::from(up[1]) - 128).abs() > 20, "{up:?}");
    assert!(up[2] < 250);
    assert!(
        (i32::from(up[1]) + i32::from(down[1]) - 255).abs() <= 2,
        "{up:?} {down:?}"
    );
}

#[test]
fn the_height_map_says_how_far_above_the_high_shape_lies() {
    let low = plane();
    let settings = MapSettings::default();
    let surface = PaintSurface::of(&low);
    let (lo, hi) = crate::bounds(&surface, &slots(&surface));
    let push = (hi - lo).length() * settings.cage;
    // Half the cage's push above: three quarters of the way up.
    let high = moved(&low, Mat4::from_translation(Vec3::Z * push * 0.5));
    let [g, ..] = middle(&bake(&low, Some(&high), MapKind::Height, &settings));
    assert!((i32::from(g) - 191).abs() <= 2, "{g}");
    // Nothing found: the middle.
    let [g, ..] = middle(&bake(&low, None, MapKind::Height, &settings));
    assert_eq!(g, 128);
}

#[test]
fn the_low_shape_maps_come_from_the_painters_bakes() {
    let low = plane();
    let image = bake(&low, None, MapKind::Occlusion, &MapSettings::default());
    // A lone plane is open to the sky.
    assert!(middle(&image)[0] >= 250);
    assert!(!bake(&low, None, MapKind::Id, &MapSettings::default()).is_empty());
}

#[test]
fn a_cancelled_bake_gives_nothing() {
    let low = plane();
    let surface = PaintSurface::of(&low);
    let slots = slots(&surface);
    let source = Source {
        surface: &surface,
        slots: &slots,
        size: SIZE,
    };
    let progress = Progress::default();
    progress.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(bake_map(&source, None, MapKind::Normal, &MapSettings::default(), &progress).is_none());
}

#[test]
fn every_map_has_a_file_name_of_its_own() {
    let mut names: Vec<&str> = MapKind::ALL.iter().map(|k| k.file_suffix()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), MapKind::ALL.len());
    assert!(MapKind::Normal.uses_high() && !MapKind::Id.uses_high());
}

#[test]
fn tangents_follow_the_uvs() {
    let surface = PaintSurface::of(&plane());
    let all = tangents(&surface);
    assert_eq!(all.len(), surface.triangles().len());
    for (t, corners) in surface.triangles().iter().zip(&all) {
        for c in corners {
            // Square to the normal, unit length.
            assert!(c.truncate().dot(t.normal).abs() < 1e-3);
            assert!((c.truncate().length() - 1.0).abs() < 1e-3);
        }
    }
    let _ = texel_uv(0, 0, SIZE);
}

#[test]
fn a_painted_cage_reaches_a_high_shape_the_plain_one_misses() {
    let low = plane();
    let surface = PaintSurface::of(&low);
    let slots = slots(&surface);
    let (lo, hi) = crate::bounds(&surface, &slots);
    let settings = MapSettings::default();
    let push = (hi - lo).length() * settings.cage;
    // Lifted 1.5 pushes up: out of the plain cage's reach.
    let high_mesh = moved(&low, Mat4::from_translation(Vec3::Z * push * 1.5));
    let high = High::of(&PaintSurface::of(&high_mesh));
    let source = Source {
        surface: &surface,
        slots: &slots,
        size: SIZE,
    };
    let plain = bake_map(&source, Some(&high), MapKind::Height, &settings, &Progress::default()).unwrap();
    assert_eq!(middle(&plain)[0], 128, "missed: the middle");
    let mut painted = low.clone();
    let points: Vec<_> = painted.vertices().map(|(v, _)| (v, push)).collect();
    mesh::cage::set_push(&mut painted, &points);
    let extra = cage_pushes(&surface, &painted);
    let caged = bake_map_caged(
        &source,
        Some(&high),
        MapKind::Height,
        &settings,
        Some(&extra),
        &Progress::default(),
    )
    .unwrap();
    assert!(middle(&caged)[0] > 128, "{:?}", middle(&caged));
}

#[test]
fn the_problem_map_counts_misses_far_hits_and_skew() {
    let low = plane();
    let surface = PaintSurface::of(&low);
    let slots = slots(&surface);
    let source = Source {
        surface: &surface,
        slots: &slots,
        size: SIZE,
    };
    let settings = MapSettings::default();
    let (lo, hi) = crate::bounds(&surface, &slots);
    let push = (hi - lo).length() * settings.cage;
    let check = |high: &Mesh| {
        let high = High::of(&PaintSurface::of(high));
        problem_map(&source, &high, &settings, None, &Progress::default()).unwrap().1
    };
    // On the low shape: all well.
    let fine = check(&low);
    assert!(fine.texels > 0);
    assert_eq!(fine.missed + fine.wrong_part + fine.skewed, 0);
    // Out of reach: every texel missed.
    let far = check(&moved(&low, Mat4::from_translation(Vec3::Z * push * 3.0)));
    assert_eq!(far.missed, far.texels);
    assert_eq!(far.share(Problem::Missed), 1.0);
    // Near the end of the reach: another part, probably.
    let deep = check(&moved(&low, Mat4::from_translation(Vec3::Z * -push * 0.95)));
    assert_eq!(deep.wrong_part, deep.texels);
    // Steeply turned: skewed where it is met.
    let steep = check(&moved(&low, Mat4::from_rotation_x(80f32.to_radians())));
    assert!(steep.skewed > 0);
}

#[test]
fn vertex_colours_are_baked_at_every_corner() {
    let cube = Primitive::Cube(mesh::CubeParams::default()).mesh();
    let surface = PaintSurface::of(&cube);
    let corners = bake_corners(&cube, &surface, MapKind::Occlusion, &BakeSettings::default()).unwrap();
    assert_eq!(corners.len(), cube.faces().map(|(_, c)| c.len()).sum::<usize>());
    // A lone cube's corners are open to the sky, more or less.
    assert!(corners.iter().all(|(_, _, c)| c[0] > 100 && c[3] == 255));
    // The same point has the same colour at each of its corners.
    let first = corners[0];
    assert!(corners.iter().filter(|(_, v, _)| *v == first.1).all(|(_, _, c)| *c == first.2));
    assert!(bake_corners(&cube, &surface, MapKind::Normal, &BakeSettings::default()).is_none());
    assert!(bakes_to_points(MapKind::Curvature) && !bakes_to_points(MapKind::Height));
}
