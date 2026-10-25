//! Tests for convex parts (decision 0151).

use super::*;
use crate::Primitive;
use crate::VertexId;
use crate::primitives::{CylinderParams, SphereParams, TorusParams};

/// A mesh of boxes, each from its lowest corner to its highest, its six
/// sides facing out.
fn boxes(list: &[([f32; 3], [f32; 3])]) -> Mesh {
    let mut mesh = Mesh::new();
    for (low, high) in list {
        let (low, high) = (Vec3::from(*low), Vec3::from(*high));
        let ids: Vec<VertexId> = (0..8)
            .map(|i| {
                mesh.add_vertex(Vec3::new(
                    if i & 1 == 0 { low.x } else { high.x },
                    if i & 2 == 0 { low.y } else { high.y },
                    if i & 4 == 0 { low.z } else { high.z },
                ))
                .unwrap()
            })
            .collect();
        for side in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            let corners: Vec<VertexId> = side.iter().map(|i| ids[*i]).collect();
            mesh.add_face(&corners).unwrap();
        }
    }
    mesh
}

/// An L: a long box along X and a tall one at its end, overlapping.
fn l_shape() -> Mesh {
    boxes(&[
        ([0.0, 0.0, 0.0], [200.0, 50.0, 50.0]),
        ([150.0, 0.0, 0.0], [200.0, 50.0, 200.0]),
    ])
}

/// A U: two posts on a base.
fn u_shape() -> Mesh {
    boxes(&[
        ([0.0, 0.0, 0.0], [300.0, 50.0, 50.0]),
        ([0.0, 0.0, 0.0], [50.0, 50.0, 250.0]),
        ([250.0, 0.0, 0.0], [300.0, 50.0, 250.0]),
    ])
}

/// Adds `other`'s points and polygons to `mesh`.
fn add(mesh: &mut Mesh, other: &Mesh) {
    let mut ids = std::collections::HashMap::new();
    for (v, p) in other.vertices() {
        ids.insert(v, mesh.add_vertex(p).unwrap());
    }
    for (_, corners) in other.faces() {
        let corners: Vec<VertexId> = corners.iter().map(|v| ids[v]).collect();
        mesh.add_face(&corners).unwrap();
    }
}

/// A thin ring: 2 m across, its tube 30 cm thick.
fn torus() -> Mesh {
    Primitive::Torus(TorusParams {
        radius: 100.0,
        tube_radius: 15.0,
        ..TorusParams::default()
    })
    .mesh()
}

/// Whether `p` is inside one of `hulls`, give or take `tolerance`.
fn held(hulls: &[Hull], p: Vec3, tolerance: f32) -> bool {
    hulls.iter().any(|h| h.contains(p, tolerance))
}

fn total_volume(hulls: &[Hull]) -> f64 {
    hulls.iter().map(Hull::volume).sum()
}

#[test]
fn a_convex_shape_is_one_part() {
    for mesh in [
        boxes(&[([0.0; 3], [100.0, 60.0, 40.0])]),
        Primitive::Sphere(SphereParams::default()).mesh(),
        Primitive::Cylinder(CylinderParams::default()).mesh(),
    ] {
        let hulls = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
        assert_eq!(hulls.len(), 1);
    }
}

#[test]
fn an_l_is_two_parts_that_hold_the_whole_shape() {
    let mesh = l_shape();
    let hulls = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
    assert_eq!(hulls.len(), 2);
    for (_, p) in mesh.vertices() {
        assert!(held(&hulls, p, 1e-3), "{p} is left out");
    }
    // The L's own volume is 200·50·50 + 50·50·150; the hulls fill
    // little more.
    let own = 200.0 * 50.0 * 50.0 + 50.0 * 50.0 * 150.0;
    let total = total_volume(&hulls);
    assert!(
        total >= own * 0.999 && total < own * 1.25,
        "{total} for {own}"
    );
    // One hull round it all fills in the inside corner.
    let one = wrapped(&mesh, usize::MAX).unwrap().volume();
    assert!(one > own * 1.4);
}

#[test]
fn a_u_is_three_parts() {
    let mesh = u_shape();
    let hulls = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
    assert_eq!(hulls.len(), 3);
    for (_, p) in mesh.vertices() {
        assert!(held(&hulls, p, 1e-3));
    }
}

#[test]
fn parts_never_pass_max_parts() {
    let mesh = torus();
    for max_parts in 1..=12 {
        let options = ConvexPartsOptions {
            max_parts,
            ..ConvexPartsOptions::default()
        };
        let hulls = convex_parts(&mesh, &options).unwrap();
        assert!(!hulls.is_empty() && hulls.len() <= max_parts as usize);
        for hull in &hulls {
            assert!(hull.points.len() <= MAX_HULL_CORNERS);
        }
    }
    // A ring needs every part it can get, and they follow its hole.
    let eight = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
    assert_eq!(eight.len(), 8);
    let one = wrapped(&mesh, usize::MAX).unwrap().volume();
    assert!(total_volume(&eight) < one * 0.5);
}

#[test]
fn more_detail_follows_dents_more_closely() {
    let mesh = torus();
    let rough = ConvexPartsOptions {
        max_parts: 32,
        detail: 0.0,
    };
    let fine = ConvexPartsOptions {
        max_parts: 32,
        detail: 1.0,
    };
    let rough = convex_parts(&mesh, &rough).unwrap();
    let fine = convex_parts(&mesh, &fine).unwrap();
    assert!(fine.len() >= rough.len());
}

#[test]
fn neighbouring_parts_meet_with_no_gap() {
    // Every point of the L's surface, sampled across each side, is in a
    // part: the parts meet exactly where they were cut.
    let mesh = l_shape();
    let hulls = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
    for i in 0..=40 {
        let x = i as f32 * 5.0;
        for p in [
            Vec3::new(x, 0.0, 0.0),
            Vec3::new(x, 50.0, 50.0),
            Vec3::new(x, 25.0, 50.0),
        ] {
            assert!(held(&hulls, p, 1e-3), "{p}");
        }
    }
}

#[test]
fn the_same_shape_and_settings_give_the_same_parts() {
    let mesh = torus();
    let options = ConvexPartsOptions::default();
    assert_eq!(convex_parts(&mesh, &options), convex_parts(&mesh, &options));
}

#[test]
fn open_empty_and_flat_shapes_are_refused() {
    let options = ConvexPartsOptions::default();
    // An open box: no lid.
    let mut open = boxes(&[([0.0; 3], [10.0; 3])]);
    let lid = open.faces().last().map(|(f, _)| f).unwrap();
    crate::cleanup::delete_elements(&mut open, &[], &[], &[lid]);
    assert_eq!(convex_parts(&open, &options), Err(PartsProblem::NotClosed));
    assert_eq!(
        convex_parts(&Mesh::new(), &options),
        Err(PartsProblem::Empty)
    );
    // A sheet with both sides: closed, but flat.
    let mut sheet = Mesh::new();
    let ids: Vec<VertexId> = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
        .iter()
        .map(|[x, y]| sheet.add_vertex(Vec3::new(*x, *y, 0.0)).unwrap())
        .collect();
    sheet.add_face(&ids).unwrap();
    let back: Vec<VertexId> = ids.iter().rev().copied().collect();
    sheet.add_face(&back).unwrap();
    assert_eq!(convex_parts(&sheet, &options), Err(PartsProblem::Flat));
    for problem in [
        PartsProblem::Empty,
        PartsProblem::NotANumber,
        PartsProblem::NotClosed,
        PartsProblem::Flat,
    ] {
        assert!(!problem.to_string().is_empty());
    }
}

#[test]
fn points_not_at_real_places_are_refused() {
    let mut mesh = boxes(&[([0.0; 3], [10.0; 3])]);
    let first = mesh.vertices().next().map(|(v, _)| v).unwrap();
    assert!(mesh.set_position(first, Vec3::NAN));
    assert_eq!(
        convex_parts(&mesh, &ConvexPartsOptions::default()),
        Err(PartsProblem::NotANumber)
    );
}

#[test]
fn odd_settings_are_brought_within_limits() {
    let mesh = u_shape();
    for (max_parts, detail) in [
        (0, f32::NAN),
        (u32::MAX, f32::INFINITY),
        (3, f32::NEG_INFINITY),
        (2, -5.0),
    ] {
        let options = ConvexPartsOptions { max_parts, detail };
        let clamped = options.clamped();
        assert!((1..=ConvexPartsOptions::MOST_PARTS).contains(&clamped.max_parts));
        assert!((0.0..=1.0).contains(&clamped.detail));
        let hulls = convex_parts(&mesh, &options).unwrap();
        assert!(hulls.len() <= clamped.max_parts as usize);
    }
}

#[test]
fn shapes_thinner_than_the_grid_are_one_hull() {
    // A thin plate: 1 mm thick, 2 m across.
    let mesh = boxes(&[([0.0; 3], [200.0, 200.0, 0.1])]);
    let hulls = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
    assert_eq!(hulls.len(), 1);
    assert_eq!(hulls[0].points.len(), 8);
}

#[test]
fn n_gons_and_touching_pieces_are_split() {
    // Boxes touching along an edge (each of its edges used four times),
    // and a cylinder with many-sided caps standing on one.
    let mut mesh = boxes(&[
        ([0.0; 3], [50.0; 3]),
        ([50.0, 50.0, 0.0], [100.0, 100.0, 50.0]),
    ]);
    let cylinder = Primitive::Cylinder(CylinderParams::default()).mesh();
    add(&mut mesh, &cylinder);
    let hulls = convex_parts(&mesh, &ConvexPartsOptions::default()).unwrap();
    assert!(hulls.len() >= 2);
    for (_, p) in mesh.vertices() {
        assert!(held(&hulls, p, 0.5), "{p}");
    }
}

#[test]
fn the_wrapped_hull_holds_every_point() {
    let mesh = torus();
    let hull = wrapped(&mesh, usize::MAX).unwrap();
    for (_, p) in mesh.vertices() {
        assert!(hull.contains(p, 1e-3));
    }
    assert!(wrapped(&mesh, MAX_HULL_CORNERS).unwrap().points.len() <= MAX_HULL_CORNERS);
    assert_eq!(wrapped(&Mesh::new(), 8), Err(HullProblem::Empty));
}

#[test]
#[ignore = "a timing, run by hand: cargo test -p mesh --release convex_parts_timings -- --ignored --nocapture"]
fn convex_parts_timings() {
    for (name, mesh) in [
        ("torus", torus()),
        (
            "dense sphere",
            Primitive::Sphere(SphereParams {
                sides: 256,
                rings: 128,
                ..SphereParams::default()
            })
            .mesh(),
        ),
        ("u", u_shape()),
    ] {
        for detail in [0.0, 0.5, 1.0] {
            let options = ConvexPartsOptions {
                max_parts: 16,
                detail,
            };
            let start = std::time::Instant::now();
            let hulls = convex_parts(&mesh, &options).unwrap();
            eprintln!(
                "{name} ({} polygons), detail {detail}: {} parts in {:?}",
                mesh.face_count(),
                hulls.len(),
                start.elapsed()
            );
        }
    }
}

/// Rings of `around` points round the X axis, radius 50, at each of `xs`.
fn rings(xs: &[f32], around: usize) -> Vec<Vec3> {
    let mut points = Vec::new();
    for x in xs {
        for i in 0..around {
            let a = i as f32 / around as f32 * std::f32::consts::TAU;
            points.push(Vec3::new(*x, 50.0 * a.cos(), 50.0 * a.sin()));
        }
    }
    points
}

#[test]
fn limited_neighbours_still_meet_on_the_plane_between_them() {
    // A rod cut at x = 100 into two parts, each with more points than a
    // part's hull may keep: both hulls keep the same corners on the cut,
    // so they meet there with no gap.
    let lower = rings(&[0.0, 50.0, 100.0], 96);
    let upper = rings(&[100.0, 150.0, 200.0], 96);
    let mut below: split::Sides = [[None; 2]; 3];
    below[0][1] = Some(100.0);
    let mut above: split::Sides = [[None; 2]; 3];
    above[0][0] = Some(100.0);
    let a = part_hull(&lower, &below).unwrap();
    let b = part_hull(&upper, &above).unwrap();
    for hull in [&a, &b] {
        assert!(hull.points.len() <= MAX_HULL_CORNERS);
    }
    let on_cut = |hull: &Hull| {
        let mut corners: Vec<[u32; 3]> = hull
            .points
            .iter()
            .filter(|p| p.x == 100.0)
            .map(|p| p.to_array().map(f32::to_bits))
            .collect();
        corners.sort_unstable();
        corners
    };
    assert!(on_cut(&a).len() >= 3, "a side on the cut");
    assert_eq!(on_cut(&a), on_cut(&b));
    // Each stays inside its part's whole hull.
    let whole = crate::hull::convex_hull(&lower).unwrap();
    for p in &a.points {
        assert!(whole.contains(*p, 1e-3));
    }
}
