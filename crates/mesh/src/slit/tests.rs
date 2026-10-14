//! Tests for Slit.

use super::*;
use crate::attributes::{AttributeElement, Domain, Value};
use crate::{MeshReport, PlaneParams, Primitive};
use glam::Vec2;

fn cube() -> Mesh {
    Primitive::Cube(Default::default()).mesh()
}

/// A square plane, 100 across, lying flat at z = 0, seen from above.
fn plane() -> Mesh {
    Primitive::Plane(PlaneParams::default()).mesh()
}

/// Looking at the cube's front (y = -50) from in front, at half height.
const FRONT: Eye = Eye::At(Vec3::new(0.0, -500.0, 50.0));
const ABOVE: Eye = Eye::At(Vec3::new(0.0, 0.0, 500.0));

fn line(points: &[[f32; 3]], eye: Eye, through: bool) -> SlitLine {
    SlitLine {
        points: points.iter().map(|p| Vec3::from_array(*p)).collect(),
        eye,
        through,
    }
}

/// Across the cube's front, left to right, at half height.
fn across_front(through: bool) -> SlitLine {
    line(&[[-80.0, -50.0, 50.0], [80.0, -50.0, 50.0]], FRONT, through)
}

fn clean(mesh: &Mesh, before: &Mesh) {
    let (r, b) = (MeshReport::new(mesh), MeshReport::new(before));
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.is_closed(), b.is_closed(), "{r:?}");
    assert_eq!(r.pieces, b.pieces, "{r:?}");
}

#[test]
fn a_line_across_cuts_only_the_polygon_seen() {
    let before = cube();
    let mut mesh = before.clone();
    let out = slit(&mut mesh, &across_front(false));
    assert_eq!(out.matched, 1);
    // The front polygon in two; its two side neighbours gain a corner.
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(mesh.vertex_count(), 10);
    clean(&mesh, &before);
    let new: Vec<Vec3> = mesh.vertices().skip(8).map(|(_, p)| p).collect();
    assert!(
        new.iter()
            .all(|p| (p.z - 50.0).abs() < 1e-3 && (p.y + 50.0).abs() < 1e-3)
    );
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 5).count(), 2);
    assert_eq!(slit_size(&before, &across_front(false)), 1);
}

#[test]
fn cutting_through_goes_all_the_way_round() {
    let before = cube();
    let mut mesh = before.clone();
    let out = slit(&mut mesh, &across_front(true));
    // Front, both sides and back.
    assert_eq!(out.matched, 4);
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(mesh.vertex_count(), 12);
    clean(&mesh, &before);
}

#[test]
fn a_flat_view_cuts_the_same() {
    let before = cube();
    let mut mesh = before.clone();
    let mut l = across_front(true);
    l.eye = Eye::Along(Vec3::Y);
    // In a flat view the sides lie edge on: front and back are cut, and the
    // sides take the new points.
    let out = slit(&mut mesh, &l);
    assert!(out.matched >= 2, "{out:?}");
    clean(&mesh, &before);
    let mut seen = before.clone();
    l.through = false;
    assert_eq!(slit(&mut seen, &l).matched, 1);
    clean(&seen, &before);
}

#[test]
fn a_line_can_turn_inside_a_polygon() {
    let before = cube();
    let mut mesh = before.clone();
    let l = line(
        &[
            [-80.0, -50.0, 30.0],
            [0.0, -50.0, 70.0],
            [80.0, -50.0, 30.0],
        ],
        FRONT,
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 1);
    assert_eq!(mesh.face_count(), 7);
    // Two on the front's sides, one inside it.
    assert_eq!(mesh.vertex_count(), 11);
    clean(&mesh, &before);
    // Made in order along the line: in, the turn, out.
    let inner = mesh.position(VertexId::from_raw(9)).unwrap();
    assert!(
        inner.distance(Vec3::new(0.0, -50.0, 70.0)) < 1e-2,
        "{inner}"
    );
}

#[test]
fn corners_snapped_to_cut_straight_between_them() {
    let before = cube();
    let mut mesh = before.clone();
    // Corner to opposite corner of the front.
    let l = line(&[[-50.0, -50.0, 0.0], [50.0, -50.0, 100.0]], FRONT, false);
    assert_eq!(slit(&mut mesh, &l).matched, 1);
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(mesh.vertex_count(), 8);
    assert_eq!(mesh.faces().filter(|(_, c)| c.len() == 3).count(), 2);
    clean(&mesh, &before);
}

#[test]
fn a_line_crossing_twice_cuts_twice() {
    let before = plane();
    let mut mesh = before.clone();
    let l = line(
        &[
            [-80.0, -20.0, 0.0],
            [80.0, -20.0, 0.0],
            [80.0, 20.0, 0.0],
            [-80.0, 20.0, 0.0],
        ],
        ABOVE,
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 2);
    assert_eq!(mesh.face_count(), 3);
    assert_eq!(mesh.vertex_count(), 8);
    clean(&mesh, &before);
}

#[test]
fn what_is_hidden_is_cut_only_through() {
    // The cube, and a square further back hidden behind it, facing the eye.
    let mut before = cube();
    let behind: Vec<VertexId> = [
        [-40.0, 200.0, 10.0],
        [40.0, 200.0, 10.0],
        [40.0, 200.0, 90.0],
        [-40.0, 200.0, 90.0],
    ]
    .iter()
    .map(|p| before.add_vertex(Vec3::from_array(*p)).unwrap())
    .collect();
    let square = before.add_face(&behind).unwrap();
    assert!(before.face_normal(square).y < -0.9, "faces the eye");
    let mut seen = before.clone();
    slit(&mut seen, &across_front(false));
    assert_eq!(seen.face(square).unwrap().len(), 4, "hidden: not cut");
    assert_eq!(seen.face_count(), before.face_count() + 1);
    let mut through = before.clone();
    slit(&mut through, &across_front(true));
    assert_eq!(through.face_count(), before.face_count() + 5);
}

#[test]
fn values_come_along() {
    let mut before = plane();
    before
        .add_attribute("uv", Domain::Corner, Value::Vec2(Vec2::ZERO))
        .unwrap();
    before
        .add_attribute("w", Domain::Point, Value::Float(0.0))
        .unwrap();
    before
        .add_attribute("seam", Domain::Edge, Value::Bool(false))
        .unwrap();
    let faces: Vec<(FaceId, Vec<VertexId>)> =
        before.faces().map(|(f, c)| (f, c.to_vec())).collect();
    for (f, corners) in faces {
        for (i, c) in corners.iter().enumerate() {
            let p = before.position(*c).unwrap();
            before
                .set_attribute(
                    "uv",
                    AttributeElement::Corner(f, i),
                    Value::Vec2(p.truncate()),
                )
                .unwrap();
            before
                .set_attribute("w", AttributeElement::Point(*c), Value::Float(p.x + p.y))
                .unwrap();
        }
    }
    for edge in before.edges().to_vec() {
        before
            .set_attribute("seam", AttributeElement::Edge(edge), Value::Bool(true))
            .unwrap();
    }
    let mut mesh = before.clone();
    let l = line(
        &[[-80.0, -30.0, 0.0], [10.0, 20.0, 0.0], [80.0, -10.0, 0.0]],
        ABOVE,
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 1);
    for (f, corners) in mesh.faces() {
        for (i, c) in corners.iter().enumerate() {
            let p = mesh.position(*c).unwrap();
            let Some(Value::Vec2(uv)) = mesh.attribute("uv", AttributeElement::Corner(f, i)) else {
                panic!("no uv");
            };
            assert!(uv.distance(p.truncate()) < 1e-3, "{f:?} {i}: {uv} at {p}");
            let Some(Value::Float(w)) = mesh.attribute("w", AttributeElement::Point(*c)) else {
                panic!("no w");
            };
            assert!((w - p.x - p.y).abs() < 1e-3, "{w} at {p}");
        }
    }
    // The border's pieces stay seams; the new edges across aren't.
    let seams = mesh
        .edges()
        .iter()
        .filter(|e| mesh.attribute("seam", AttributeElement::Edge(**e)) == Some(Value::Bool(true)))
        .count();
    assert_eq!(seams, 6);
}

#[test]
fn nonsense_lines_change_nothing() {
    let before = cube();
    for l in [
        line(&[[0.0, -50.0, 50.0]], FRONT, false),
        line(&[[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]], FRONT, false),
        line(
            &[[-80.0, -50.0, 50.0], [80.0, -50.0, 50.0]],
            Eye::Along(Vec3::ZERO),
            false,
        ),
        line(
            &[[-80.0, -50.0, 50.0], [80.0, -50.0, 50.0]],
            Eye::At(Vec3::INFINITY),
            true,
        ),
        SlitLine {
            points: vec![Vec3::ZERO; MAX_LINE_POINTS + 1],
            eye: FRONT,
            through: true,
        },
        // Starting inside the front: it only goes out, never across.
        line(&[[0.0, -50.0, 50.0], [80.0, -50.0, 50.0]], FRONT, false),
        // Off to the side: misses.
        line(&[[200.0, -50.0, 50.0], [300.0, -50.0, 50.0]], FRONT, true),
    ] {
        let mut mesh = before.clone();
        let out = slit(&mut mesh, &l);
        assert_eq!(out.skipped, 1, "{l:?}");
        assert_eq!(mesh, before, "{l:?}");
    }
}

#[test]
fn ids_are_kept_and_the_cut_repeats_exactly() {
    let before = cube();
    let mut a = before.clone();
    let mut b = before.clone();
    slit(&mut a, &across_front(true));
    slit(&mut b, &across_front(true));
    assert_eq!(a, b);
    for (f, _) in before.faces() {
        assert!(a.face(f).is_some());
    }
}

#[test]
fn every_shape_cuts_cleanly() {
    let eyes = [
        Eye::At(Vec3::new(300.0, -400.0, 250.0)),
        Eye::At(Vec3::new(-50.0, 30.0, 600.0)),
        Eye::Along(Vec3::new(0.3, 1.0, -0.2)),
    ];
    for primitive in Primitive::all() {
        let before = primitive.mesh();
        let middle = before.bounds().unwrap().centre();
        let mut cuts = 0;
        for (k, eye) in eyes.iter().enumerate() {
            for through in [false, true] {
                let wobble = Vec3::new(13.0, -7.0, 21.0) * k as f32;
                let l = SlitLine {
                    points: vec![
                        middle + Vec3::new(-120.0, 10.0, -40.0) + wobble,
                        middle + Vec3::new(5.0, -3.0, 7.0),
                        middle + Vec3::new(110.0, -20.0, 45.0) - wobble,
                    ],
                    eye: *eye,
                    through,
                };
                let mut mesh = before.clone();
                cuts += slit(&mut mesh, &l).matched;
                let (r, b) = (MeshReport::new(&mesh), MeshReport::new(&before));
                let what = format!("{} eye {k} through {through}: {r:?}", primitive.name());
                assert_eq!(r.flipped_edges, 0, "{what}");
                assert_eq!(r.non_manifold_edges, 0, "{what}");
                assert_eq!(r.loose_points, 0, "{what}");
                assert_eq!(r.is_closed(), b.is_closed(), "{what}");
                for (v, p) in mesh.vertices() {
                    assert!(p.is_finite(), "{what}: {v:?}");
                }
            }
        }
        assert!(cuts >= 3, "{}: only {cuts} cuts", primitive.name());
    }
}

#[test]
fn a_line_can_turn_from_one_polygon_onto_the_next() {
    // Seen from the front right, as the app first opens: across the front,
    // turning inside it, round the corner and across the right side.
    let before = cube();
    let mut mesh = before.clone();
    let l = line(
        &[
            [-80.0, -50.0, 90.0],
            [30.0, -50.0, 40.0],
            [50.0, 0.0, 60.0],
            [50.0, 120.0, 140.0],
        ],
        Eye::At(Vec3::new(560.0, -980.0, 330.0)),
        false,
    );
    assert_eq!(slit(&mut mesh, &l).matched, 2);
    assert_eq!(mesh.face_count(), 8);
    // In, the turn and the corner on the front; the turn and out on the
    // side.
    assert_eq!(mesh.vertex_count(), 13);
    clean(&mesh, &before);
    // Ending inside the side: it is cut only up to the corner's polygon.
    let mut short = before.clone();
    let mut l = l.clone();
    l.points.pop();
    assert_eq!(slit(&mut short, &l).matched, 1);
    clean(&short, &before);
}

/// A closed line through `points` (the first point again at the end).
fn closed(points: &[[f32; 3]], eye: Eye, through: bool) -> SlitLine {
    let mut l = line(points, eye, through);
    l.points.push(l.points[0]);
    l
}

/// A square of side `2 r` round (`x`, `y`) on the plane, seen from above.
fn square_on_plane(x: f32, y: f32, r: f32) -> SlitLine {
    closed(
        &[
            [x - r, y - r, 0.0],
            [x + r, y - r, 0.0],
            [x + r, y + r, 0.0],
            [x - r, y + r, 0.0],
        ],
        ABOVE,
        false,
    )
}

fn area(mesh: &Mesh, face: FaceId) -> f32 {
    let c: Vec<Vec3> = mesh
        .face(face)
        .unwrap()
        .iter()
        .map(|v| mesh.position(*v).unwrap())
        .collect();
    let n = c.len();
    (0..n)
        .map(|i| c[i].cross(c[(i + 1) % n]))
        .sum::<Vec3>()
        .length()
        / 2.0
}

#[test]
fn closed_lines_are_told_apart() {
    assert!(square_on_plane(0.0, 0.0, 10.0).is_closed());
    // Three points back to the first is a triangle; two and back is not.
    assert!(closed(&[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], ABOVE, false).is_closed());
    assert!(!closed(&[[0.0; 3], [1.0, 0.0, 0.0]], ABOVE, false).is_closed());
    assert!(!across_front(false).is_closed());
}

#[test]
fn a_closed_line_inside_a_polygon_cuts_a_shape_into_it() {
    let before = plane();
    let mut mesh = before.clone();
    let line = square_on_plane(10.0, -5.0, 20.0);
    let out = slit(&mut mesh, &line);
    assert_eq!(out.matched, 1);
    // The square, and the ring round it in two pieces.
    assert_eq!(mesh.face_count(), 3);
    assert_eq!(mesh.vertex_count(), 8);
    let r = MeshReport::new(&mesh);
    assert!(!r.has_problems(), "{r:?}");
    assert_eq!(r.pieces, 1);
    // All facing up, the whole plane's area shared out, the square 40 by 40.
    let total: f32 = mesh.faces().map(|(f, _)| area(&mesh, f)).sum();
    assert!((total - 100.0 * 100.0).abs() < 1e-1, "{total}");
    for (f, _) in mesh.faces() {
        assert!(mesh.face_normal(f).z > 0.99, "{f:?} faces up");
    }
    let square = mesh
        .faces()
        .find(|(_, c)| c.len() == 4 && c.iter().all(|v| v.get() >= 4))
        .map(|(f, _)| f)
        .expect("the square is a polygon");
    assert!((area(&mesh, square) - 1600.0).abs() < 1e-1);
    // Its new points lie where the line was.
    for (_, p) in mesh.vertices().skip(4) {
        assert!((p.x - 10.0).abs() > 19.9 || (p.y + 5.0).abs() > 19.9);
    }
    // Each polygon draws as triangles that lie inside it: the halves of
    // the ring aren't crossed.
    for (f, c) in mesh.faces() {
        let pts: Vec<Vec3> = c.iter().map(|v| mesh.position(*v).unwrap()).collect();
        let mut tris = 0.0;
        crate::triangulate(
            pts.len(),
            |i| pts[i],
            |[a, b, cc]| {
                tris += (pts[b] - pts[a]).cross(pts[cc] - pts[a]).z / 2.0;
            },
        );
        assert!((tris - area(&mesh, f)).abs() < 1e-1, "{f:?}");
    }
    assert_eq!(slit_size(&before, &line), 3);
}

#[test]
fn a_loop_drawn_either_way_round_cuts_the_same() {
    let mut one = plane();
    let mut two = plane();
    let line = square_on_plane(0.0, 0.0, 15.0);
    let mut back = line.clone();
    back.points.reverse();
    slit(&mut one, &line);
    slit(&mut two, &back);
    assert_eq!(one.face_count(), two.face_count());
    for mesh in [&one, &two] {
        assert!(mesh.faces().all(|(f, _)| mesh.face_normal(f).z > 0.99));
    }
}

#[test]
fn a_closed_line_across_polygons_cuts_back_to_its_start() {
    // A loop round the middle of a plane cut into four: it crosses the
    // inside edges, closing stretch included.
    let before = Primitive::Plane(PlaneParams {
        segments: [2, 2],
        ..Default::default()
    })
    .mesh();
    let mut mesh = before.clone();
    let line = square_on_plane(0.0, 0.0, 25.0);
    let out = slit(&mut mesh, &line);
    assert_eq!(out.matched, 4, "a cut in each quarter");
    // Each quarter is cut in two: a corner piece of the square, and the rest.
    assert_eq!(mesh.face_count(), 8);
    clean(&mesh, &before);
    // Starting in the middle of a stretch inside one quarter changes nothing.
    let mut again = before.clone();
    let shifted = closed(
        &[
            [0.0, -25.0, 0.0],
            [25.0, -25.0, 0.0],
            [25.0, 25.0, 0.0],
            [-25.0, 25.0, 0.0],
            [-25.0, -25.0, 0.0],
        ],
        ABOVE,
        false,
    );
    assert_eq!(slit(&mut again, &shifted).matched, 4);
    assert_eq!(again.face_count(), 8);
    clean(&again, &before);
}

#[test]
fn a_loop_on_the_far_side_is_cut_only_through() {
    // The cube's bottom, seen from above: hidden behind the top.
    let before = cube();
    let mut mesh = before.clone();
    let mut line = closed(
        &[
            [-10.0, -10.0, 0.0],
            [10.0, -10.0, 0.0],
            [10.0, 10.0, 0.0],
            [-10.0, 10.0, 0.0],
        ],
        ABOVE,
        false,
    );
    // Seen: the top gets the shape.
    assert_eq!(slit(&mut mesh, &line).matched, 1);
    assert_eq!(mesh.face_count(), 8);
    clean(&mesh, &before);
    // Through: the bottom too.
    line.through = true;
    let mut both = before.clone();
    assert_eq!(slit(&mut both, &line).matched, 2);
    assert_eq!(both.face_count(), 10);
    clean(&both, &before);
}

#[test]
fn odd_loops_change_nothing_or_stay_clean() {
    // A loop with no area (all on one line) inside a polygon: nothing.
    let mut mesh = plane();
    let flat = closed(
        &[[-10.0, 0.0, 0.0], [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]],
        ABOVE,
        false,
    );
    assert_eq!(slit(&mut mesh, &flat).skipped, 1);
    assert_eq!(mesh, plane());
    // A triangle, and a long thin loop, inside: clean.
    for shape in [
        closed(
            &[[-5.0, -5.0, 0.0], [5.0, -5.0, 0.0], [0.0, 5.0, 0.0]],
            ABOVE,
            false,
        ),
        closed(
            &[
                [-45.0, -1.0, 0.0],
                [45.0, -1.0, 0.0],
                [45.0, 1.0, 0.0],
                [-45.0, 1.0, 0.0],
            ],
            ABOVE,
            false,
        ),
    ] {
        let mut m = plane();
        assert_eq!(slit(&mut m, &shape).matched, 1);
        let r = MeshReport::new(&m);
        assert!(!r.has_problems(), "{r:?}");
        assert_eq!(m.face_count(), 3);
    }
    // A loop that pokes out of the plane cuts where it crosses.
    let mut m = plane();
    let out = slit(&mut m, &square_on_plane(40.0, 0.0, 20.0));
    assert_eq!(out.matched, 1);
    assert!(!MeshReport::new(&m).has_problems());
}

#[test]
fn a_closed_cut_repeats_exactly() {
    let line = square_on_plane(3.0, 7.0, 12.0);
    let (mut a, mut b) = (plane(), plane());
    slit(&mut a, &line);
    slit(&mut b, &line);
    assert_eq!(a, b);
}

#[test]
fn a_star_shaped_loop_still_cuts_cleanly() {
    // A star with deep notches: lines straight to a notch would cross a
    // spike, so other corners and points are tried.
    let pts: Vec<[f32; 3]> = (0..10)
        .map(|i| {
            let a = i as f32 / 10.0 * std::f32::consts::TAU + 0.3;
            let r = if i % 2 == 0 { 40.0 } else { 8.0 };
            [a.cos() * r, a.sin() * r, 0.0]
        })
        .collect();
    let line = closed(&pts, ABOVE, false);
    let mut mesh = plane();
    assert_eq!(slit(&mut mesh, &line).matched, 1);
    let r = MeshReport::new(&mesh);
    assert!(!r.has_problems(), "{r:?}");
    let total: f32 = mesh.faces().map(|(f, _)| area(&mesh, f)).sum();
    assert!((total - 100.0 * 100.0).abs() < 1.0, "{total}");
    for (f, c) in mesh.faces() {
        let flat: Vec<glam::Vec2> = c
            .iter()
            .map(|v| mesh.position(*v).unwrap().truncate())
            .collect();
        assert!(simple(&flat), "{f:?} crosses itself");
    }
}

/// Every polygon is simple, faces up, and together they cover the plane.
fn covers_the_plane(mesh: &Mesh) {
    let r = MeshReport::new(mesh);
    assert!(!r.has_problems(), "{r:?}");
    let total: f32 = mesh.faces().map(|(f, _)| area(mesh, f)).sum();
    assert!((total - 100.0 * 100.0).abs() < 1.0, "{total}");
    for (f, c) in mesh.faces() {
        assert!(mesh.face_normal(f).z > 0.99, "{f:?} faces up");
        let flat: Vec<glam::Vec2> = c
            .iter()
            .map(|v| mesh.position(*v).unwrap().truncate())
            .collect();
        assert!(simple(&flat), "{f:?} crosses itself");
        let mut seen = std::collections::BTreeSet::new();
        assert!(c.iter().all(|v| seen.insert(*v)), "{f:?} repeats a corner");
    }
}

#[test]
fn a_loop_touching_the_border_at_one_point_cuts_its_shape_in() {
    // The first point snapped onto the plane's left edge, the rest inside.
    let line = closed(
        &[
            [-50.0, 0.0, 0.0],
            [0.0, -20.0, 0.0],
            [20.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
        ],
        ABOVE,
        false,
    );
    let mut mesh = plane();
    assert_eq!(slit(&mut mesh, &line).matched, 1);
    // The shape, and the rest cut in two: one new point on the edge.
    assert_eq!(mesh.face_count(), 3);
    assert_eq!(mesh.vertex_count(), 4 + 4);
    covers_the_plane(&mesh);
    // The same with the touching point in the middle of the line.
    let line = closed(
        &[
            [0.0, -20.0, 0.0],
            [20.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
            [-50.0, 0.0, 0.0],
        ],
        ABOVE,
        false,
    );
    let mut mesh = plane();
    assert_eq!(slit(&mut mesh, &line).matched, 1);
    assert_eq!(mesh.face_count(), 3);
    covers_the_plane(&mesh);
}

#[test]
fn a_loop_touching_a_corner_shares_it() {
    let line = closed(
        &[[-50.0, -50.0, 0.0], [10.0, -30.0, 0.0], [-30.0, 10.0, 0.0]],
        ABOVE,
        false,
    );
    let mut mesh = plane();
    assert_eq!(slit(&mut mesh, &line).matched, 1);
    assert_eq!(mesh.vertex_count(), 4 + 2, "the corner is shared");
    assert_eq!(mesh.face_count(), 3);
    covers_the_plane(&mesh);
}

#[test]
fn a_loop_touching_the_border_twice_cuts_in_two_pieces() {
    // From the left edge to the right edge and back, bulging both ways:
    // the two halves of a lens, cut out across the plane.
    let line = closed(
        &[
            [-50.0, 0.0, 0.0],
            [0.0, -20.0, 0.0],
            [50.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
        ],
        ABOVE,
        false,
    );
    let mut mesh = plane();
    let out = slit(&mut mesh, &line);
    assert!(out.matched >= 1);
    assert_eq!(mesh.face_count(), 3);
    covers_the_plane(&mesh);
}

#[test]
fn an_open_line_touching_the_border_from_inside_cuts_on() {
    // In from the bottom, touching the right edge, out at the top.
    let line = line(
        &[
            [0.0, -60.0, 0.0],
            [10.0, -20.0, 0.0],
            [50.0, 0.0, 0.0],
            [10.0, 20.0, 0.0],
            [0.0, 60.0, 0.0],
        ],
        ABOVE,
        false,
    );
    let mut mesh = plane();
    slit(&mut mesh, &line);
    // Left of the line, and the wedge between it and the right edge, cut
    // in two where it touches.
    assert_eq!(mesh.face_count(), 3);
    covers_the_plane(&mesh);
    // Touching from outside cuts nothing.
    let outside = line_from_outside();
    let mut mesh = plane();
    assert_eq!(slit(&mut mesh, &outside).skipped, 1);
}

fn line_from_outside() -> SlitLine {
    line(
        &[[60.0, -20.0, 0.0], [50.0, 0.0, 0.0], [60.0, 20.0, 0.0]],
        ABOVE,
        false,
    )
}
