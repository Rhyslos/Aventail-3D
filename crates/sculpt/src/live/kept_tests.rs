//! Tests of what a dab finds and what the live sculpt keeps between dabs
//! (the heavy check's misses after decision 0313): the points in reach
//! against looking at every point, the tree's boxes and kept facings
//! against working them out afresh after everything that moves points,
//! and a session's result on any number of cores.

use super::*;
use crate::brush::Falloff;
use crate::tests::grid;
use mesh::PrimitiveKind;
use std::collections::BTreeSet;

fn dab(kind: BrushKind, centre: Vec3, radius: f32) -> Dab {
    Dab {
        kind,
        centre,
        radius,
        strength: 0.7,
        falloff: Falloff::Smooth,
        invert: false,
    }
}

fn ball(polygons: usize) -> Arc<Mesh> {
    Arc::new(mesh::samples::heavy_ball(polygons, 100.0))
}

/// The front of the ball.
const FRONT: Vec3 = Vec3::new(0.0, -100.0, 100.0);

/// What a dab should find, by looking at every polygon: the points of
/// polygons within reach, how many leaves hold a polygon with a corner
/// in reach, those polygons' facings added up, and their area (twice).
fn by_hand(live: &LiveSculpt, centre: Vec3, radius: f32) -> (BTreeSet<u32>, usize, Vec3, f32) {
    let mesh = &live.mesh;
    let mut points = BTreeSet::new();
    let mut leaves = BTreeSet::new();
    let mut facing = glam::DVec3::ZERO;
    let mut area = 0.0;
    let mut every = Vec::new();
    live.tree
        .near_leaves(centre, f32::MAX / 4.0, |leaf| every.push(leaf));
    for leaf in every {
        for &f in live.tree.leaf_faces(leaf) {
            let corners = mesh.face(FaceId::from_raw(f)).unwrap();
            let mut any = false;
            for v in corners {
                if mesh.position(*v).unwrap().distance(centre) <= radius {
                    points.insert(v.get());
                    any = true;
                }
            }
            if any {
                leaves.insert(leaf);
                facing += newell(mesh, corners).as_dvec3();
                area += f64::from(newell(mesh, corners).length());
            }
        }
    }
    (points, leaves.len(), facing.as_vec3(), area as f32)
}

fn assert_reach_is_right(live: &LiveSculpt, centre: Vec3, radius: f32) {
    let found = live.reach(centre, radius, true);
    let (points, leaves, facing, area) = by_hand(live, centre, radius);
    let got: BTreeSet<u32> = found.points.iter().map(|r| r.point.get()).collect();
    assert_eq!(got.len(), found.points.len(), "a point came twice");
    assert_eq!(got, points, "{centre} {radius}");
    for r in &found.points {
        assert_eq!(live.mesh.position(r.point), Some(r.at));
        assert_eq!(r.distance, r.at.distance(centre));
    }
    let unique: BTreeSet<u32> = found.leaves.iter().copied().collect();
    assert_eq!(unique.len(), found.leaves.len(), "a leaf came twice");
    assert_eq!(unique.len(), leaves);
    // Against the area added up: round a closed shape the facings add
    // up to nothing but rounding.
    assert!(
        found.facing.distance(facing) <= area.max(1e-6) * 1e-4,
        "{} against {facing}",
        found.facing
    );
    // Not asked for: not added up, the points and leaves the same.
    let plain = live.reach(centre, radius, false);
    assert_eq!(plain.facing, Vec3::ZERO);
    assert_eq!(plain.leaves, found.leaves);
    assert_eq!(plain.points.len(), found.points.len());
}

#[test]
fn a_dab_finds_exactly_the_points_in_reach_each_once() {
    let mut live = LiveSculpt::new(ball(20_000), None);
    let reaches = [
        (FRONT, 0.5),
        (FRONT, 12.0),
        (FRONT, 60.0),
        (Vec3::new(40.0, 30.0, 190.0), 35.0),
        // The whole shape, and nothing.
        (Vec3::new(0.0, 0.0, 100.0), 500.0),
        (Vec3::new(0.0, 0.0, 900.0), 5.0),
    ];
    for (centre, radius) in reaches {
        assert_reach_is_right(&live, centre, radius);
    }
    // After strokes have moved the surface (the boxes and kept facings
    // followed), still so.
    for kind in [BrushKind::Draw, BrushKind::ClayStrips, BrushKind::Smooth] {
        live.dab(&dab(kind, FRONT, 40.0));
        live.dab(&dab(kind, FRONT + Vec3::X * 9.0, 40.0));
    }
    assert!(live.tree.is_exact(&live.mesh));
    for (centre, radius) in reaches {
        assert_reach_is_right(&live, centre, radius);
    }
}

#[test]
fn open_shapes_polygons_with_many_corners_and_loose_points_are_found_too() {
    // A flat grid (its edge open), a cylinder (caps of many corners: more
    // than are kept at hand) and a point no polygon uses (never found).
    let mut cylinder = PrimitiveKind::Cylinder.standard().mesh();
    cylinder.add_vertex(Vec3::new(0.0, 0.0, 30.0)).unwrap();
    assert!(cylinder.faces().any(|(_, c)| c.len() > AT_HAND));
    for shape in [grid(24), cylinder] {
        let live = LiveSculpt::new(Arc::new(shape), None);
        let bounds = live.mesh.bounds().unwrap();
        let (middle, size) = ((bounds.min + bounds.max) / 2.0, bounds.max - bounds.min);
        for centre in [middle, bounds.min, bounds.max, middle + Vec3::Z * size.z / 2.0] {
            for share in [0.05, 0.3, 0.8, 3.0] {
                assert_reach_is_right(&live, centre, size.max_element() * share);
            }
        }
    }
    // Radii that aren't reaches find nothing.
    let live = LiveSculpt::new(Arc::new(grid(4)), None);
    for radius in [f32::NAN, -1.0] {
        assert!(live.reach(Vec3::ZERO, radius, true).points.is_empty());
    }
    assert!(live.reach(Vec3::NAN, 1.0, true).points.is_empty());
}

#[test]
fn the_boxes_and_kept_facings_follow_everything_that_moves_points() {
    let input = ball(6_000);
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    assert!(live.tree.is_exact(&live.mesh));
    let empty = live.sculpt().clone();
    // Every brush, a few dabs each (the grabbing ones take hold first),
    // finished as a stroke.
    let mut strokes = Vec::new();
    for (n, kind) in BrushKind::ALL.into_iter().enumerate() {
        let at = FRONT + Vec3::X * (n as f32 - 6.0);
        for step in 0..4 {
            live.dab(&dab(kind, at + Vec3::Z * step as f32 * 3.0, 30.0));
            assert!(live.tree.is_exact(&live.mesh), "{kind:?} dab {step}");
        }
        if let Some(sculpt) = live.finish() {
            strokes.push(sculpt);
        }
        assert!(live.tree.is_exact(&live.mesh), "{kind:?} finished");
    }
    assert!(strokes.len() > 10);
    // A stroke taken back.
    live.dab(&dab(BrushKind::Draw, FRONT, 45.0));
    live.cancel();
    assert!(live.tree.is_exact(&live.mesh), "cancelled");
    // Symmetry: the copies put together, a grab held per copy.
    live.set_symmetry(Symmetry {
        axes: [true, false, true],
        radial: None,
    });
    for kind in [BrushKind::Draw, BrushKind::Grab, BrushKind::SnakeHook] {
        for step in 0..3 {
            live.dab(&dab(kind, FRONT + Vec3::new(4.0, 0.0, step as f32 * 4.0), 25.0));
            assert!(live.tree.is_exact(&live.mesh), "symmetric {kind:?}");
        }
        strokes.extend(live.finish());
    }
    live.set_symmetry(Symmetry::NONE);
    // A mask, a masked dab, and the gizmo moving the unmasked part.
    let half = crate::mask::set_where(&live.mesh, live.mask(), 255, |p| p.x < 0.0);
    strokes.extend(live.set_mask(half));
    live.dab(&dab(BrushKind::Inflate, FRONT, 40.0));
    assert!(live.tree.is_exact(&live.mesh), "masked dab");
    strokes.extend(live.finish());
    live.begin_move();
    live.move_unmasked(Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
    assert!(live.tree.is_exact(&live.mesh), "the gizmo");
    live.move_unmasked(Mat4::from_scale(Vec3::splat(1.1)));
    assert!(live.tree.is_exact(&live.mesh), "the gizmo again");
    strokes.extend(live.finish());
    assert!(live.tree.is_exact(&live.mesh), "the gizmo finished");
    // Undo and redo: the last strokes by their kept places, then far
    // back and forward again (worked out from the moves).
    let last = strokes.len() - 1;
    for at in [last - 1, last, last - 1, last - 2, 3, last, 0] {
        live.follow(&strokes[at]);
        assert!(live.tree.is_exact(&live.mesh), "followed to {at}");
    }
    live.follow(&empty);
    assert!(live.tree.is_exact(&live.mesh), "all undone");
    assert_eq!(live.mesh(), &*input);
    // Make symmetrical, on a shape that is (the standard sphere).
    let mut live = LiveSculpt::new(Arc::new(PrimitiveKind::Sphere.standard().mesh()), None);
    live.dab(&dab(BrushKind::Draw, Vec3::new(50.0, 0.0, 50.0), 20.0));
    live.finish();
    assert!(live.make_symmetrical(0, true).is_ok_and(|s| s.is_some()));
    assert!(live.tree.is_exact(&live.mesh), "made symmetrical");
}

/// A session of strokes with brushes big enough to share their work
/// between cores: the points' places and the mask at the end, as bits.
fn session(input: &Arc<Mesh>) -> (Vec<[u32; 3]>, Vec<u8>, Sculpt) {
    let mut live = LiveSculpt::new(Arc::clone(input), None);
    let stroke = |live: &mut LiveSculpt, kind: BrushKind, from: Vec3, radius: f32| {
        for step in 0..3 {
            live.dab(&dab(kind, from + Vec3::X * step as f32 * 6.0, radius));
        }
        live.finish();
    };
    stroke(&mut live, BrushKind::Draw, FRONT, 70.0);
    stroke(&mut live, BrushKind::ClayStrips, FRONT, 50.0);
    stroke(&mut live, BrushKind::Flatten, FRONT + Vec3::Z * 10.0, 60.0);
    stroke(&mut live, BrushKind::Smooth, FRONT, 60.0);
    stroke(&mut live, BrushKind::Mask, FRONT - Vec3::Z * 30.0, 30.0);
    stroke(&mut live, BrushKind::Inflate, FRONT, 60.0);
    stroke(&mut live, BrushKind::Grab, FRONT, 60.0);
    stroke(&mut live, BrushKind::SnakeHook, FRONT, 50.0);
    // The copies of a symmetric dab overlapping on the middle line.
    live.set_symmetry(Symmetry {
        axes: [true, false, false],
        radial: None,
    });
    stroke(&mut live, BrushKind::Draw, FRONT + Vec3::X * 5.0, 70.0);
    stroke(&mut live, BrushKind::Grab, FRONT + Vec3::X * 5.0, 60.0);
    live.set_symmetry(Symmetry {
        axes: [false, false, false],
        radial: Some((2, 5)),
    });
    stroke(&mut live, BrushKind::Clay, Vec3::new(60.0, 0.0, 180.0), 70.0);
    let places = live
        .mesh
        .point_slots()
        .map(|p| p.unwrap_or(Vec3::ZERO).to_array().map(f32::to_bits))
        .collect();
    (places, live.mask().values(), live.sculpt().clone())
}

#[test]
fn a_session_gives_the_same_bits_on_any_number_of_cores() {
    // Enough points in reach (tens of thousands) for every step of a dab
    // to be shared out when there are cores to share it between.
    let input = ball(120_000);
    let alone = mesh::parallel::with_cores(1, || session(&input));
    assert!(
        alone.0.iter().zip(input.point_slots()).any(|(a, b)| {
            *a != b.unwrap_or(Vec3::ZERO).to_array().map(f32::to_bits)
        }),
        "the session sculpted something"
    );
    for cores in [2, 3, 4, 7, 32] {
        let shared = mesh::parallel::with_cores(cores, || session(&input));
        assert!(shared.0 == alone.0, "places differ on {cores} cores");
        assert!(shared.1 == alone.1, "the mask differs on {cores} cores");
        assert_eq!(shared.2, alone.2, "{cores} cores");
    }
}
