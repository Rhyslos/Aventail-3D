use super::*;
use crate::material::{set_material, slot_of};
use crate::primitives::{CubeParams, PlaneParams, SphereParams};
use crate::{MeshReport, Primitive};

fn grid(segments: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [segments, segments],
    })
    .mesh()
}

fn cube(size: f32) -> Mesh {
    Primitive::Cube(CubeParams {
        size: Vec3::splat(size),
        segments: [1, 1, 1],
    })
    .mesh()
}

fn sphere() -> Mesh {
    Primitive::Sphere(SphereParams {
        radius: 50.0,
        sides: 16,
        rings: 8,
    })
    .mesh()
}

/// A mesh of the given points and polygons.
fn mesh_of(points: &[[f32; 3]], faces: &[&[u32]]) -> Mesh {
    let mut mesh = Mesh::new();
    let ids: Vec<VertexId> = points
        .iter()
        .map(|p| mesh.add_vertex(Vec3::from(*p)).unwrap())
        .collect();
    for face in faces {
        let corners: Vec<VertexId> = face.iter().map(|i| ids[*i as usize]).collect();
        mesh.add_face(&corners).unwrap();
    }
    mesh
}

fn options(place_on: PlaceOn) -> ScatterOptions {
    ScatterOptions {
        place_on,
        ..Default::default()
    }
}

fn one(mesh: &Mesh) -> [ScatterShape<'_>; 1] {
    [ScatterShape {
        mesh,
        matrix: Mat4::IDENTITY,
    }]
}

const MOST: usize = 30_000_000;

#[test]
fn points_get_one_copy_each_facing_out() {
    let surface = grid(2);
    let rock = cube(2.0);
    let out = scatter(&surface, None, &one(&rock), &options(PlaceOn::Points), MOST).unwrap();
    assert_eq!(out.face_count(), 4 + 9 * 6);
    let spots = spots(&surface, None, &options(PlaceOn::Points), 1).unwrap();
    assert_eq!(spots.len(), 9);
    for spot in &spots {
        assert!(spot.normal.distance(Vec3::Z) < 1e-5, "{:?}", spot.normal);
    }
    // The surface keeps its own polygons and ids, first.
    let first: Vec<FaceId> = surface.faces().map(|(f, _)| f).collect();
    let kept: Vec<FaceId> = out.faces().map(|(f, _)| f).take(4).collect();
    assert_eq!(first, kept);
}

#[test]
fn centres_get_one_copy_each() {
    let surface = grid(3);
    let spots = spots(&surface, None, &options(PlaceOn::Centres), 1).unwrap();
    assert_eq!(spots.len(), 9);
    // The middle polygon's centre is the grid's middle.
    assert!(spots.iter().any(|s| s.position.length() < 1e-4));
}

#[test]
fn a_random_spread_makes_count_copies_on_the_surface() {
    let surface = sphere();
    let spread = ScatterOptions {
        count: 200,
        ..options(PlaceOn::Random)
    };
    let spots = spots(&surface, None, &spread, 1).unwrap();
    assert_eq!(spots.len(), 200);
    // The sphere sits on the ground: its middle is 50 cm up.
    let middle = Vec3::new(0.0, 0.0, 50.0);
    for spot in &spots {
        // On the sphere (its polygons lie a little inside the true ball).
        let r = spot.position.distance(middle);
        assert!((45.0..=50.01).contains(&r), "{r}");
        // Facing out.
        assert!(spot.normal.dot((spot.position - middle).normalize()) > 0.8);
    }
}

#[test]
fn a_random_spread_is_even_by_area() {
    // Two squares side by side, the right one four times the area.
    let surface = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.0, 2.0, 0.0],
            [2.0, 2.0, 0.0],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    );
    let spread = ScatterOptions {
        count: 5_000,
        ..options(PlaceOn::Random)
    };
    let spots = spots(&surface, None, &spread, 1).unwrap();
    let small = spots.iter().filter(|s| s.position.x <= 1.0).count();
    let share = small as f32 / spots.len() as f32;
    assert!((0.17..0.23).contains(&share), "{share}");
}

#[test]
fn the_same_settings_give_the_same_copies() {
    let surface = sphere();
    let rock = cube(3.0);
    let settings = ScatterOptions {
        count: 80,
        random_turn: 360.0,
        scale_min: 0.5,
        scale_max: 2.0,
        spacing: 4.0,
        ..options(PlaceOn::Random)
    };
    let a = scatter(&surface, None, &one(&rock), &settings, MOST).unwrap();
    let b = scatter(&surface, None, &one(&rock), &settings, MOST).unwrap();
    assert_eq!(a, b);
    let other_seed = ScatterOptions {
        seed: 2,
        ..settings
    };
    let c = scatter(&surface, None, &one(&rock), &other_seed, MOST).unwrap();
    assert_ne!(a, c);
}

#[test]
fn raising_count_keeps_the_copies_already_there() {
    let surface = sphere();
    let few = ScatterOptions {
        count: 20,
        random_turn: 90.0,
        scale_min: 0.5,
        ..options(PlaceOn::Random)
    };
    let more = ScatterOptions { count: 40, ..few };
    let a = spots(&surface, None, &few, 3).unwrap();
    let b = spots(&surface, None, &more, 3).unwrap();
    assert_eq!(a[..], b[..20]);
}

#[test]
fn spacing_keeps_copies_apart() {
    let surface = grid(1);
    let spread = ScatterOptions {
        count: 500,
        spacing: 10.0,
        ..options(PlaceOn::Random)
    };
    let spots = spots(&surface, None, &spread, 1).unwrap();
    // A 100 cm square holds well under 500 copies 10 cm apart.
    assert!(spots.len() < 200 && spots.len() > 30, "{}", spots.len());
    for (i, a) in spots.iter().enumerate() {
        for b in &spots[i + 1..] {
            assert!(a.position.distance(b.position) >= 10.0);
        }
    }
    // Over points too: a 10 × 10 grid's points are 10 cm apart, so 15 cm
    // skips about every other one.
    let points = ScatterOptions {
        spacing: 15.0,
        ..options(PlaceOn::Points)
    };
    let thinned = super::spots(&grid(10), None, &points, 1).unwrap();
    assert!(
        thinned.len() < 121 && thinned.len() >= 30,
        "{}",
        thinned.len()
    );
}

#[test]
fn copies_stand_on_the_surface_turned_to_it() {
    // A tall thin post, its pivot at its foot, on the side of a box
    // facing +X.
    let post = mesh_of(
        &[[0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [0.1, 0.0, 10.0]],
        &[&[0, 1, 2]],
    );
    let wall = mesh_of(
        &[
            [0.0, -1.0, -1.0],
            [0.0, 1.0, -1.0],
            [0.0, 1.0, 1.0],
            [0.0, -1.0, 1.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let settings = options(PlaceOn::Centres);
    let out = scatter(&wall, None, &one(&post), &settings, MOST).unwrap();
    let tip = out
        .vertices()
        .map(|(_, p)| p)
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    assert!((tip.x - 10.0).abs() < 1e-4, "{tip}");
    // Unaligned, it stays upright.
    let upright = ScatterOptions {
        align: false,
        ..settings
    };
    let out = scatter(&wall, None, &one(&post), &upright, MOST).unwrap();
    let top = out.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
    assert!((top - 10.0).abs() < 1e-4);
}

#[test]
fn offset_lifts_copies_along_the_surface() {
    let surface = grid(1);
    let rock = cube(2.0);
    let lifted = ScatterOptions {
        offset: 5.0,
        ..options(PlaceOn::Centres)
    };
    let out = scatter(&surface, None, &one(&rock), &lifted, MOST).unwrap();
    let low = out
        .vertices()
        .skip(surface.vertex_count())
        .map(|(_, p)| p.z)
        .fold(f32::MAX, f32::min);
    // The cube's pivot is at its foot, so its bottom is the offset up.
    assert!((low - 5.0).abs() < 1e-4, "{low}");
}

#[test]
fn random_scale_and_turn_stay_within_their_range() {
    let settings = ScatterOptions {
        count: 300,
        random_turn: 45.0,
        scale_min: 0.5,
        scale_max: 1.5,
        ..options(PlaceOn::Random)
    };
    let spots = spots(&sphere(), None, &settings, 4).unwrap();
    let mut shapes = [0; 4];
    for spot in &spots {
        assert!((0.5..=1.5).contains(&spot.scale));
        assert!((0.0..=45f32.to_radians()).contains(&spot.turn));
        shapes[spot.shape] += 1;
    }
    // Every shape is used, about evenly.
    assert!(shapes.iter().all(|n| *n > 40), "{shapes:?}");
}

#[test]
fn only_the_chosen_polygons_get_copies() {
    let surface = grid(4);
    let chosen: BTreeSet<FaceId> = surface.faces().map(|(f, _)| f).take(2).collect();
    let spots = spots(&surface, Some(&chosen), &options(PlaceOn::Centres), 1).unwrap();
    assert_eq!(spots.len(), 2);
    let random = spots_random(&surface, &chosen);
    for spot in random {
        // The first two polygons are a row along the grid's edge.
        assert!(spot.position.y < -24.9, "{:?}", spot.position);
    }
    let gone: BTreeSet<FaceId> = [FaceId::from_raw(9_999)].into();
    assert_eq!(
        super::spots(&surface, Some(&gone), &options(PlaceOn::Centres), 1),
        Err(ScatterProblem::ChosenGone)
    );
}

fn spots_random(surface: &Mesh, chosen: &BTreeSet<FaceId>) -> Vec<Spot> {
    spots(surface, Some(chosen), &options(PlaceOn::Random), 1).unwrap()
}

#[test]
fn materials_uvs_and_hard_edges_come_along() {
    let surface = grid(1);
    let mut rock = cube(2.0);
    let faces: Vec<FaceId> = rock.faces().map(|(f, _)| f).take(1).collect();
    set_material(&mut rock, &faces, 2);
    let (a, b) = rock.edges()[0].ends();
    rock.set_hard(a, b, true);
    let out = scatter(&surface, None, &one(&rock), &options(PlaceOn::Points), MOST).unwrap();
    let marked = out.faces().filter(|(f, _)| slot_of(&out, *f) == 2).count();
    assert_eq!(marked, 4);
    assert_eq!(out.hard_edge_count(), 4 * rock.hard_edge_count());
    let uv_layers: Vec<&str> = rock.attributes().map(|(n, _, _)| n).collect();
    for name in uv_layers {
        assert!(out.attributes().any(|(n, _, _)| n == name), "{name}");
    }
}

#[test]
fn a_clean_shape_scattered_stays_clean() {
    let out = scatter(
        &cube(100.0),
        None,
        &one(&cube(5.0)),
        &options(PlaceOn::Centres),
        MOST,
    )
    .unwrap();
    let report = MeshReport::new(&out);
    assert_eq!(report.border_edges, 0, "{report:?}");
    assert_eq!(report.non_manifold_edges, 0, "{report:?}");
    assert_eq!(report.flipped_edges, 0, "{report:?}");
    assert_eq!(out.face_count(), 6 + 6 * 6);
}

#[test]
fn degenerate_surfaces_are_refused_or_skipped() {
    let rock = cube(1.0);
    // Empty.
    assert_eq!(
        scatter(
            &Mesh::new(),
            None,
            &one(&rock),
            &options(PlaceOn::Random),
            MOST
        ),
        Err(ScatterProblem::NoSurface)
    );
    // No shapes.
    assert_eq!(
        scatter(&grid(1), None, &[], &options(PlaceOn::Points), MOST),
        Err(ScatterProblem::NoShapes)
    );
    // A polygon with no area: random refuses, centres still place.
    let flat = mesh_of(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        &[&[0, 1, 2]],
    );
    assert_eq!(
        spots(&flat, None, &options(PlaceOn::Random), 1),
        Err(ScatterProblem::NoArea)
    );
    let centred = spots(&flat, None, &options(PlaceOn::Centres), 1).unwrap();
    assert_eq!(centred.len(), 1);
    assert!(centred[0].normal.is_normalized());
    // Zero-length edges: two points at one place.
    let pinched = mesh_of(
        &[
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[&[0, 1, 2, 3]],
    );
    let spread = spots(&pinched, None, &options(PlaceOn::Random), 1).unwrap();
    assert_eq!(spread.len(), 50);
    let points = spots(&pinched, None, &options(PlaceOn::Points), 1).unwrap();
    assert_eq!(points.len(), 4);
    // A point that isn't a number: its polygon is skipped.
    let mut broken = grid(2);
    let v = broken.vertices().next().unwrap().0;
    broken.set_position(v, Vec3::NAN);
    for place_on in PlaceOn::ALL {
        let spots = spots(&broken, None, &options(place_on), 1).unwrap();
        assert!(spots.iter().all(|s| s.position.is_finite()));
        assert!(!spots.is_empty());
    }
}

#[test]
fn n_gons_open_borders_and_tangled_edges_take_copies() {
    // An eight-sided n-gon, and a fin sharing an edge with two others (an
    // edge of three polygons).
    let mut points: Vec<[f32; 3]> = (0..8)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            [a.cos() * 10.0, a.sin() * 10.0, 0.0]
        })
        .collect();
    points.extend([[0.0, 0.0, 5.0], [0.0, 0.0, -5.0], [20.0, 0.0, 5.0]]);
    let surface = mesh_of(
        &points,
        &[
            &[0, 1, 2, 3, 4, 5, 6, 7],
            &[0, 8, 4],
            &[0, 9, 4],
            &[0, 4, 10],
        ],
    );
    let rock = cube(1.0);
    for place_on in PlaceOn::ALL {
        let out = scatter(&surface, None, &one(&rock), &options(place_on), MOST).unwrap();
        assert!(out.face_count() > surface.face_count());
        assert!(out.vertices().all(|(_, p)| p.is_finite()));
    }
}

#[test]
fn too_many_copies_or_polygons_are_refused() {
    let rock = cube(1.0);
    // 100 001 loose triangles.
    let mut big = Mesh::new();
    for i in 0..100_001 {
        let x = i as f32;
        let a = big.add_vertex(Vec3::new(x, 0.0, 0.0)).unwrap();
        let b = big.add_vertex(Vec3::new(x + 0.5, 0.0, 0.0)).unwrap();
        let c = big.add_vertex(Vec3::new(x, 0.5, 0.0)).unwrap();
        big.add_face(&[a, b, c]).unwrap();
    }
    assert!(matches!(
        spots(&big, None, &options(PlaceOn::Centres), 1),
        Err(ScatterProblem::TooManyCopies { copies: 100_001 })
    ));
    assert!(matches!(
        spots(&big, None, &options(PlaceOn::Points), 1),
        Err(ScatterProblem::TooManyCopies { copies: 300_003 })
    ));
    assert!(matches!(
        scatter(&grid(10), None, &one(&rock), &options(PlaceOn::Points), 500),
        Err(ScatterProblem::TooManyPolygons {
            polygons: 826,
            most: 500
        })
    ));
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let wild = ScatterOptions {
        place_on: PlaceOn::Random,
        count: 0,
        seed: u32::MAX,
        spacing: f32::NAN,
        align: true,
        random_turn: f32::INFINITY,
        scale_min: 1e9,
        scale_max: -3.0,
        offset: f32::NEG_INFINITY,
    }
    .clamped();
    assert_eq!(wild.count, 1);
    assert_eq!(wild.spacing, 0.0);
    assert_eq!(wild.random_turn, 0.0);
    assert_eq!(
        (wild.scale_min, wild.scale_max),
        (ScatterOptions::MIN_SCALE, ScatterOptions::MAX_SCALE)
    );
    assert_eq!(wild.offset, 0.0);
    let huge = ScatterOptions {
        count: u32::MAX,
        spacing: 1e30,
        offset: -1e30,
        random_turn: -5.0,
        ..Default::default()
    }
    .clamped();
    assert_eq!(huge.count, ScatterOptions::MAX_COPIES);
    assert_eq!(huge.spacing, ScatterOptions::MAX_DISTANCE);
    assert_eq!(huge.offset, -ScatterOptions::MAX_DISTANCE);
    assert_eq!(huge.random_turn, 0.0);
    // A huge spacing still works: one copy.
    let spread = spots(
        &sphere(),
        None,
        &ScatterOptions {
            spacing: 1e30,
            ..Default::default()
        },
        1,
    )
    .unwrap();
    assert_eq!(spread.len(), 1);
}

#[test]
fn a_tiny_spacing_on_far_off_points_never_panics() {
    let far = mesh_of(
        &[[1e30, 1e30, 0.0], [-1e30, 1e30, 0.0], [-1e30, -1e30, 0.0]],
        &[&[0, 1, 2]],
    );
    let settings = ScatterOptions {
        spacing: 1e-3,
        count: 10,
        ..Default::default()
    };
    for place_on in PlaceOn::ALL {
        let _ = spots(
            &far,
            None,
            &ScatterOptions {
                place_on,
                ..settings
            },
            2,
        );
    }
}

#[test]
fn random_settings_never_give_a_point_that_isnt_a_number() {
    let surface = sphere();
    let rock = cube(2.0);
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let pick = |n: u64, values: &[f32]| values[(n % values.len() as u64) as usize];
    let values = [
        0.0,
        -1.0,
        1.0,
        0.5,
        360.0,
        1e9,
        -1e9,
        f32::NAN,
        f32::INFINITY,
        1e-9,
    ];
    for _ in 0..200 {
        let settings = ScatterOptions {
            place_on: PlaceOn::ALL[(next() % 3) as usize],
            count: (next() % 300) as u32,
            seed: next() as u32,
            spacing: pick(next(), &values),
            align: next() % 2 == 0,
            random_turn: pick(next(), &values),
            scale_min: pick(next(), &values),
            scale_max: pick(next(), &values),
            offset: pick(next(), &values),
        };
        if let Ok(out) = scatter(&surface, None, &one(&rock), &settings, MOST) {
            assert!(out.vertices().all(|(_, p)| p.is_finite()), "{settings:?}");
        }
    }
}

#[test]
fn polygons_added_is_an_upper_bound() {
    let surface = grid(4);
    let rock = cube(1.0);
    for place_on in PlaceOn::ALL {
        let settings = options(place_on);
        let out = scatter(&surface, None, &one(&rock), &settings, MOST).unwrap();
        let added = out.face_count() - surface.face_count();
        assert!(polygons_added(&surface, &settings, &[6]) >= added);
    }
}

#[test]
#[ignore = "timing: run with --release -- --ignored --nocapture"]
fn timings() {
    let surface = Primitive::Sphere(SphereParams {
        radius: 500.0,
        sides: 64,
        rings: 32,
    })
    .mesh();
    let rock = Primitive::Sphere(SphereParams {
        radius: 5.0,
        sides: 12,
        rings: 6,
    })
    .mesh();
    for (count, spacing) in [(1_000, 0.0), (10_000, 0.0), (10_000, 10.0), (100_000, 0.0)] {
        let settings = ScatterOptions {
            count,
            spacing,
            random_turn: 360.0,
            scale_min: 0.5,
            scale_max: 1.5,
            ..Default::default()
        };
        let start = std::time::Instant::now();
        let out = scatter(&surface, None, &one(&rock), &settings, MOST).unwrap();
        eprintln!(
            "{count} copies of {} polygons, spacing {spacing}: {} polygons in {:.1} ms",
            rock.face_count(),
            out.face_count(),
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
