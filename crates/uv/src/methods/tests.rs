//! Tests for the ways of laying a piece flat, pins and unfolding one way
//! (decision 0109).

use super::*;
use crate::unwrap::{UnfoldAxis, UnwrapOptions, unwrap, unwrap_with};
use mesh::uvs::Uvs;
use mesh::{
    CubeParams, CylinderParams, EdgeKey, FaceId, PlaneParams, Primitive, SphereParams, VertexId,
};
use std::collections::BTreeSet;

const MADE: [UnwrapMethod; 3] = [
    UnwrapMethod::Conformal,
    UnwrapMethod::AngleBased,
    UnwrapMethod::SizeKeeping,
];

fn plane(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [n, n],
        ..Default::default()
    })
    .mesh()
}

fn sphere() -> Mesh {
    Primitive::Sphere(SphereParams {
        sides: 16,
        rings: 12,
        ..Default::default()
    })
    .mesh()
}

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

/// The polygons whose middle is above the middle of the shape on `axis`.
fn upper(mesh: &Mesh, axis: usize) -> Vec<FaceId> {
    mesh.faces()
        .filter(|(_, list)| {
            let sum: Vec3 = list.iter().map(|v| mesh.position(*v).unwrap()).sum();
            sum[axis] / list.len() as f32 > 1e-3
        })
        .map(|(f, _)| f)
        .collect()
}

fn chart_of(mesh: &Mesh, faces: &[FaceId]) -> Chart {
    Chart::of(mesh, faces, &BTreeSet::<EdgeKey>::new())
}

/// How much bigger the biggest triangle is in the layout, for its real
/// size, than the smallest (1 is perfectly even).
fn size_spread(mesh: &Mesh, chart: &Chart, at: &[Vec2]) -> f64 {
    let (triangles, real) = real_shapes(mesh, chart);
    let ratios: Vec<f64> = triangles
        .iter()
        .zip(&real)
        .filter_map(|(t, s)| {
            let s = s.as_ref()?;
            let flat = f64::from((at[t[1]] - at[t[0]]).perp_dot(at[t[2]] - at[t[0]])).abs();
            Some(flat / (2.0 * area(s).abs()))
        })
        .collect();
    let most = ratios.iter().copied().fold(0.0, f64::max);
    let least = ratios.iter().copied().fold(f64::INFINITY, f64::min);
    most / least
}

fn flips(mesh: &Mesh, chart: &Chart, at: &[Vec2]) -> usize {
    let (triangles, real) = real_shapes(mesh, chart);
    triangles
        .iter()
        .zip(&real)
        .filter(|(t, s)| s.is_some() && (at[t[1]] - at[t[0]]).perp_dot(at[t[2]] - at[t[0]]) <= 0.0)
        .count()
}

fn corner_uvs(mesh: &Mesh) -> Vec<Vec2> {
    let count = mesh.face_layout().1.len();
    Uvs::of(mesh).unwrap().into_values(count)
}

fn point_uv(mesh: &Mesh, v: VertexId) -> Vec2 {
    let uvs = Uvs::of(mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let (f, list) = mesh.faces().find(|(_, l)| l.contains(&v)).unwrap();
    let k = list.iter().position(|x| *x == v).unwrap();
    uvs.at(starts[f.get() as usize] as usize + k)
}

#[test]
fn a_flat_piece_comes_out_perfect_whichever_way() {
    let mesh = plane(4);
    let chart = chart_of(&mesh, &all_faces(&mesh));
    let none = vec![None; chart.len()];
    let (triangles, real) = real_shapes(&mesh, &chart);
    for m in MADE {
        let (at, used) = lay_flat(&mesh, &chart, m, &none, &[]).unwrap();
        assert_eq!(used, m);
        let d = distortion(&triangles, &real, &at);
        assert!(d < 1e-3, "{m:?}: {d}");
        assert_eq!(flips(&mesh, &chart, &at), 0);
    }
}

#[test]
fn angle_based_even_angles_add_up_round_every_inner_point() {
    let mesh = sphere();
    let faces = upper(&mesh, 1);
    let chart = chart_of(&mesh, &faces);
    let (triangles, real) = real_shapes(&mesh, &chart);
    let even = angle_based_shapes(chart.len(), &triangles, &real).unwrap();
    let inner = inner_wedges(chart.len(), &triangles);
    let mut around = vec![0.0f64; chart.len()];
    for (t, s) in triangles.iter().zip(&even) {
        let a = angles(s.as_ref().unwrap());
        for j in 0..3 {
            around[t[j]] += a[j];
        }
    }
    let mut checked = 0;
    for w in 0..chart.len() {
        if inner[w] {
            assert!((around[w] - 2.0 * PI).abs() < 0.02, "{}", around[w]);
            checked += 1;
        }
    }
    assert!(checked > 10);
}

#[test]
fn keeping_sizes_keeps_sizes_better_on_a_dome() {
    let mesh = sphere();
    let chart = chart_of(&mesh, &upper(&mesh, 1));
    let none = vec![None; chart.len()];
    let (conformal, _) = lay_flat(&mesh, &chart, UnwrapMethod::Conformal, &none, &[]).unwrap();
    let (rigid, _) = lay_flat(&mesh, &chart, UnwrapMethod::SizeKeeping, &none, &[]).unwrap();
    let (angle, _) = lay_flat(&mesh, &chart, UnwrapMethod::AngleBased, &none, &[]).unwrap();
    let (c, r) = (
        size_spread(&mesh, &chart, &conformal),
        size_spread(&mesh, &chart, &rigid),
    );
    assert!(r < c, "keep sizes {r} vs keep shapes {c}");
    for at in [&conformal, &rigid, &angle] {
        assert_eq!(flips(&mesh, &chart, at), 0);
    }
}

#[test]
fn automatic_picks_the_most_even_and_always_the_same() {
    let mesh = sphere();
    let chart = chart_of(&mesh, &upper(&mesh, 1));
    let none = vec![None; chart.len()];
    let (triangles, real) = real_shapes(&mesh, &chart);
    let (auto, chosen) = lay_flat(&mesh, &chart, UnwrapMethod::Automatic, &none, &[]).unwrap();
    assert_ne!(chosen, UnwrapMethod::Automatic);
    let best = MADE
        .iter()
        .map(|m| {
            let (at, _) = lay_flat(&mesh, &chart, *m, &none, &[]).unwrap();
            distortion(&triangles, &real, &at)
        })
        .fold(f64::INFINITY, f64::min);
    assert!((distortion(&triangles, &real, &auto) - best).abs() < 1e-9);
    let (again, chosen_again) =
        lay_flat(&mesh, &chart, UnwrapMethod::Automatic, &none, &[]).unwrap();
    assert_eq!(chosen, chosen_again);
    assert_eq!(auto, again);
    // A flat piece is perfect every way: the tie goes to keeping shapes.
    let flat = plane(3);
    let chart = chart_of(&flat, &all_faces(&flat));
    let none = vec![None; chart.len()];
    let (_, chosen) = lay_flat(&flat, &chart, UnwrapMethod::Automatic, &none, &[]).unwrap();
    assert_eq!(chosen, UnwrapMethod::Conformal);
}

#[test]
fn distortion_prices_a_turned_over_triangle_but_not_a_mirrored_layout() {
    let mesh = plane(2);
    let chart = chart_of(&mesh, &all_faces(&mesh));
    let none = vec![None; chart.len()];
    let (triangles, real) = real_shapes(&mesh, &chart);
    let (at, _) = lay_flat(&mesh, &chart, UnwrapMethod::Conformal, &none, &[]).unwrap();
    let even = distortion(&triangles, &real, &at);
    // The whole layout mirrored is just as even (placing turns it back).
    let mirrored: Vec<Vec2> = at.iter().map(|v| Vec2::new(-v.x, v.y)).collect();
    assert!((distortion(&triangles, &real, &mirrored) - even).abs() < 1e-6);
    // A corner folded over its triangle's far side is turned over.
    let corner = (0..chart.len())
        .min_by_key(|w| triangles.iter().filter(|t| t.contains(w)).count())
        .unwrap();
    let t = triangles.iter().find(|t| t.contains(&corner)).unwrap();
    let others: Vec<usize> = t.iter().copied().filter(|w| *w != corner).collect();
    let mut folded = at.clone();
    folded[corner] = at[others[0]] + at[others[1]] - at[corner];
    assert!(distortion(&triangles, &real, &folded) > even + 1.0);
    let worst = stretch_each(&triangles, &real, &folded)
        .into_iter()
        .fold(1.0, f64::max);
    assert!(worst.is_infinite());
}

/// A plane unwrapped, with points `pinned` pinned and then the shape bent
/// (its middle lifted) so a fresh unwrap would differ.
fn pinned_plane(pinned: &[usize]) -> (Mesh, Vec<VertexId>) {
    let mut mesh = plane(4);
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, false);
    let points: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    let chosen: Vec<VertexId> = pinned.iter().map(|i| points[*i]).collect();
    mesh::uvs::set_pins(&mut mesh, &[], &chosen, true);
    let middle = points[12];
    let p = mesh.position(middle).unwrap();
    mesh.set_position(middle, p + Vec3::new(0.0, 10.0, 0.0));
    (mesh, chosen)
}

#[test]
fn two_pins_stay_exactly_where_they_were_every_way() {
    for m in UnwrapMethod::ALL {
        let (mut mesh, pinned) = pinned_plane(&[0, 24]);
        let before: Vec<Vec2> = pinned.iter().map(|v| point_uv(&mesh, *v)).collect();
        let faces = all_faces(&mesh);
        let options = UnwrapOptions {
            method: m,
            ..UnwrapOptions::default()
        };
        let outcome = unwrap_with(&mut mesh, &faces, options);
        assert_eq!(outcome.matched, faces.len(), "{m:?}");
        for (v, was) in pinned.iter().zip(&before) {
            assert!(point_uv(&mesh, *v).distance(*was) < 1e-5, "{m:?}");
        }
        assert!(corner_uvs(&mesh).iter().all(|v| v.is_finite()));
        let uvs = Uvs::of(&mesh).unwrap();
        for (f, list) in mesh.faces() {
            let c: Vec<Vec2> = (0..list.len())
                .map(|k| uvs.corner(&mesh, f, k).unwrap())
                .collect();
            assert!((c[1] - c[0]).perp_dot(c[2] - c[0]) > 0.0, "{m:?} {f:?}");
        }
    }
}

#[test]
fn one_pin_moves_the_piece_back_to_it() {
    let (mut mesh, pinned) = pinned_plane(&[6]);
    let was = point_uv(&mesh, pinned[0]);
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, false);
    assert!(point_uv(&mesh, pinned[0]).distance(was) < 1e-5);
}

#[test]
fn a_mirrored_piece_stays_mirrored_around_its_pins() {
    let (mut mesh, pinned) = pinned_plane(&[0, 24]);
    // Mirror the layout across U.
    let mirrored: Vec<Vec2> = corner_uvs(&mesh)
        .iter()
        .map(|v| Vec2::new(1.0 - v.x, v.y))
        .collect();
    mesh::uvs::set_uvs(&mut mesh, mirrored);
    let before: Vec<Vec2> = pinned.iter().map(|v| point_uv(&mesh, *v)).collect();
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, false);
    for (v, was) in pinned.iter().zip(&before) {
        assert!(point_uv(&mesh, *v).distance(*was) < 1e-5);
    }
    // Every polygon still faces the mirrored way.
    let uvs = Uvs::of(&mesh).unwrap();
    for (f, list) in mesh.faces() {
        let c: Vec<Vec2> = (0..list.len())
            .map(|k| uvs.corner(&mesh, f, k).unwrap())
            .collect();
        assert!((c[1] - c[0]).perp_dot(c[2] - c[0]) < 0.0, "{f:?} {c:?}");
    }
}

#[test]
fn unfolding_one_way_keeps_the_other_way() {
    for (axis, kept) in [(UnfoldAxis::Across, 1), (UnfoldAxis::Up, 0)] {
        let (mut mesh, _) = pinned_plane(&[]);
        let before = corner_uvs(&mesh);
        let faces = all_faces(&mesh);
        let options = UnwrapOptions {
            only: Some(axis),
            ..UnwrapOptions::default()
        };
        unwrap_with(&mut mesh, &faces, options);
        let after = corner_uvs(&mesh);
        assert!(
            before
                .iter()
                .zip(&after)
                .all(|(b, a)| (b[kept] - a[kept]).abs() < 1e-6),
            "{axis:?}"
        );
        let other = 1 - kept;
        assert!(
            before
                .iter()
                .zip(&after)
                .any(|(b, a)| (b[other] - a[other]).abs() > 1e-4),
            "{axis:?} changed nothing"
        );
    }
}

#[test]
fn relax_leaves_pinned_points_alone() {
    let (mut mesh, pinned) = pinned_plane(&[0, 6, 12, 18, 24]);
    let before: Vec<Vec2> = pinned.iter().map(|v| point_uv(&mesh, *v)).collect();
    let faces = all_faces(&mesh);
    crate::relax(&mut mesh, &faces, 60);
    for (v, was) in pinned.iter().zip(&before) {
        assert_eq!(point_uv(&mesh, *v), *was);
    }
}

#[test]
fn straighten_keeps_a_pinned_point() {
    let mut mesh = Primitive::Cylinder(CylinderParams::default()).mesh();
    let faces = all_faces(&mesh);
    unwrap(&mut mesh, &faces, true);
    let point = mesh.vertices().map(|(v, _)| v).nth(3).unwrap();
    mesh::uvs::set_pins(&mut mesh, &[], &[point], true);
    let was = point_uv(&mesh, point);
    crate::straighten(&mut mesh, &faces);
    assert!(point_uv(&mesh, point).distance(was) < 1e-5);
}

#[test]
fn odd_pieces_give_real_layouts_or_none() {
    // Every point pinned: nothing left to solve.
    let mesh = plane(1);
    let chart = chart_of(&mesh, &all_faces(&mesh));
    let all: Vec<Option<Vec2>> = (0..chart.len())
        .map(|w| Some(Vec2::new(w as f32, (w * w) as f32)))
        .collect();
    for m in MADE {
        assert!(lay_flat(&mesh, &chart, m, &all, &[]).is_none());
    }
    // A piece with no area at all.
    let mut flat = plane(1);
    for (v, _) in flat.vertices().collect::<Vec<_>>() {
        flat.set_position(v, Vec3::ZERO);
    }
    let chart = chart_of(&flat, &all_faces(&flat));
    let none = vec![None; chart.len()];
    for m in UnwrapMethod::ALL {
        assert!(lay_flat(&flat, &chart, m, &none, &[]).is_none());
    }
    // An n-gon, a cube's open box and a cube with a squashed side all come
    // out real and unflipped every way.
    let ngon = Primitive::Cylinder(CylinderParams {
        caps: true,
        ..Default::default()
    })
    .mesh();
    let cap: Vec<FaceId> = ngon
        .faces()
        .filter(|(_, l)| l.len() > 4)
        .map(|(f, _)| f)
        .take(1)
        .collect();
    let mut squashed = Primitive::Cube(CubeParams::default()).mesh();
    let first = squashed.vertices().map(|(v, _)| v).next().unwrap();
    let other = squashed.vertices().map(|(v, _)| v).nth(1).unwrap();
    let at = squashed.position(other).unwrap();
    squashed.set_position(first, at);
    let cube = Primitive::Cube(CubeParams::default()).mesh();
    for (mesh, faces) in [
        (&ngon, cap),
        (&cube, upper(&cube, 1)),
        (&squashed, upper(&squashed, 1)),
    ] {
        let chart = chart_of(mesh, &faces);
        let none = vec![None; chart.len()];
        for m in UnwrapMethod::ALL {
            if let Some((at, _)) = lay_flat(mesh, &chart, m, &none, &[]) {
                assert!(at.iter().all(|v| v.is_finite()), "{m:?}");
            }
        }
    }
    // Pins in one spot count as one.
    assert!(!crate::unwrap::held_apart(&[
        Some(Vec2::ONE),
        Some(Vec2::ONE)
    ]));
    assert!(crate::unwrap::held_apart(&[
        Some(Vec2::ONE),
        None,
        Some(Vec2::ZERO)
    ]));
}

#[test]
fn a_whole_shape_unwraps_every_way_without_folds() {
    for m in UnwrapMethod::ALL {
        let mut mesh = sphere();
        let faces = all_faces(&mesh);
        let options = UnwrapOptions {
            method: m,
            ..UnwrapOptions::default()
        };
        let outcome = unwrap_with(&mut mesh, &faces, options);
        assert_eq!(outcome.matched, faces.len());
        assert!(corner_uvs(&mesh).iter().all(|v| v.is_finite()));
    }
}

#[test]
#[ignore = "benchmark: cargo test -p uv --release -- --ignored methods_benchmark --nocapture"]
fn methods_benchmark() {
    let mesh = Primitive::Sphere(SphereParams {
        sides: 96,
        rings: 64,
        ..Default::default()
    })
    .mesh();
    let chart = chart_of(&mesh, &upper(&mesh, 1));
    let none = vec![None; chart.len()];
    for m in MADE {
        let start = std::time::Instant::now();
        lay_flat(&mesh, &chart, m, &none, &[]).unwrap();
        println!("{m:?}: {} wedges in {:?}", chart.len(), start.elapsed());
    }
}
