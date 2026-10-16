//! Tests for mending UVs after modelling (decision 0104).

use super::*;
use glam::Affine2;
use mesh::edit::{self, Grouping};
use mesh::{CubeParams, PlaneParams, Primitive, VertexId};

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn plane(n: u32) -> Mesh {
    Primitive::Plane(PlaneParams {
        segments: [n, n],
        ..Default::default()
    })
    .mesh()
}

/// The polygon facing most along `way`.
fn facing(mesh: &Mesh, way: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            mesh.face_normal(*a)
                .dot(way)
                .total_cmp(&mesh.face_normal(*b).dot(way))
        })
        .unwrap()
}

/// Polygon `f`'s corner UVs.
fn uvs_of(mesh: &Mesh, f: FaceId) -> Vec<Vec2> {
    let uvs = Uvs::of(mesh).unwrap();
    let n = mesh.face(f).unwrap().len();
    (0..n).map(|k| uvs.corner(mesh, f, k).unwrap()).collect()
}

/// Runs `edit` on a copy of `mesh` and mends it; the result and how many
/// polygons were mended.
fn edited(mesh: &Mesh, edit: impl FnOnce(&mut Mesh)) -> (Mesh, usize) {
    let mut after = mesh.clone();
    edit(&mut after);
    let mended = mend(mesh, &mut after);
    (after, mended)
}

/// Every polygon with area has room on the texture, none mirrored, none
/// stretched far one way.
fn all_well(mesh: &Mesh) {
    for (f, s) in crate::stretch::stretch_of(mesh).unwrap() {
        assert!(!s.empty, "polygon {} has no room: {s:?}", f.get());
        assert!(!s.flipped, "polygon {} is mirrored: {s:?}", f.get());
        assert!(s.uneven < UNEVEN, "polygon {} is stretched: {s:?}", f.get());
    }
}

/// The polygons both shapes have with the same corners kept their UVs.
fn untouched_kept(before: &Mesh, after: &Mesh) {
    for (f, list) in before.faces() {
        if after.face(f) == Some(list) {
            assert_eq!(uvs_of(before, f), uvs_of(after, f), "polygon {}", f.get());
        }
    }
}

fn islands(mesh: &Mesh) -> usize {
    mesh::uvs::islands(mesh).len()
}

#[test]
fn an_extrudes_walls_get_room_and_the_rest_stays_put() {
    let c = cube();
    let top = facing(&c, Vec3::Z);
    let (m, mended) = edited(&c, |m| {
        edit::extrude_shaped(m, &[top], 50.0, Grouping::Together, Default::default());
    });
    assert_eq!(mended, 4);
    all_well(&m);
    untouched_kept(&c, &m);
    // The top keeps its UVs exactly.
    assert_eq!(uvs_of(&c, top), uvs_of(&m, top));
    // The walls are one more piece, a straight strip, slid inside the
    // square (it is longer than the square is wide at this density, so it
    // starts at its left edge).
    assert_eq!(islands(&m), islands(&c) + 1);
    let strip = mesh::uvs::islands(&m)
        .into_iter()
        .find(|i| i.contains(&FaceId::from_raw(6)))
        .unwrap();
    let corners: Vec<Vec2> = strip.iter().flat_map(|f| uvs_of(&m, *f)).collect();
    let lo = corners.iter().fold(Vec2::INFINITY, |a, v| a.min(*v));
    let hi = corners.iter().fold(Vec2::NEG_INFINITY, |a, v| a.max(*v));
    assert!(lo.x.abs() < 1e-5, "{lo}");
    assert!(lo.y >= -1e-5 && hi.y <= 1.0 + 1e-5, "{lo} {hi}");
}

#[test]
fn an_insets_ring_and_a_filled_hole_get_room() {
    let c = cube();
    let top = facing(&c, Vec3::Z);
    let (m, mended) = edited(&c, |m| {
        edit::inset_polygons(m, &[top], 20.0, Grouping::Together);
    });
    assert_eq!(mended, 4);
    all_well(&m);
    untouched_kept(&c, &m);

    let points: Vec<VertexId> = c.face(top).unwrap().to_vec();
    let mut open = c.clone();
    edit::delete_elements(&mut open, &[], &[], &[top]);
    let (m, mended) = edited(&open, |m| {
        edit::fill_holes(m, &points);
    });
    assert_eq!(mended, 1);
    all_well(&m);
    untouched_kept(&open, &m);
}

#[test]
fn a_bevel_keeps_the_pieces_apart_and_gives_its_strip_room() {
    let c = cube();
    let top = facing(&c, Vec3::Z);
    let list = c.face(top).unwrap().to_vec();
    // An edge on a seam (every cube edge is one).
    let edge = mesh::EdgeKey::new(list[0], list[1]);
    assert!(mesh::uvs::is_seam(&c, edge));
    let (m, _) = edited(&c, |m| {
        edit::bevel_edges(m, &[edge], Default::default());
    });
    all_well(&m);
    untouched_kept(&c, &m);
    assert_eq!(islands(&m), islands(&c));
    // A corner bevelled: three pieces meet at the new polygon, which joins
    // one of them (seams part it from the others).
    let (m, mended) = edited(&c, |m| {
        edit::bevel_points(m, &[list[0]], 10.0);
    });
    assert_eq!(mended, 1);
    all_well(&m);
    untouched_kept(&c, &m);
    assert_eq!(islands(&m), islands(&c));
}

#[test]
fn a_loop_cut_needs_no_mending() {
    let c = cube();
    let top = facing(&c, Vec3::Z);
    let list = c.face(top).unwrap().to_vec();
    let (m, mended) = edited(&c, |m| {
        edit::loop_cut(m, mesh::EdgeKey::new(list[0], list[1]), 2, 0.5);
    });
    assert_eq!(mended, 0);
    all_well(&m);
}

#[test]
fn a_piece_turned_by_hand_stays_turned_after_an_extrude_next_to_it() {
    // A plane in one piece, turned 30° and moved by hand, then one of its
    // polygons extruded: the walls carry on from it, and every polygon
    // that was there keeps its UVs.
    let mut p = plane(3);
    let all: Vec<FaceId> = p.faces().map(|(f, _)| f).collect();
    let turn = Affine2::from_translation(Vec2::new(0.1, 0.0)) * Affine2::from_angle(0.5);
    mesh::uvs::transform_uvs(&mut p, &all, &[], turn);
    let middle = all[4];
    let (m, mended) = edited(&p, |m| {
        edit::extrude_shaped(m, &[middle], 20.0, Grouping::Together, Default::default());
    });
    assert_eq!(mended, 4);
    all_well(&m);
    untouched_kept(&p, &m);
}

#[test]
fn a_mirrored_piece_is_mended_mirrored() {
    // A plane mirrored by hand, its middle polygon inset: the ring is laid
    // out around the kept polygons, mirrored like them.
    let mut p = plane(3);
    let all: Vec<FaceId> = p.faces().map(|(f, _)| f).collect();
    mesh::uvs::transform_uvs(&mut p, &all, &[], Affine2::from_scale(Vec2::new(-1.0, 1.0)));
    let middle = all[4];
    let (m, mended) = edited(&p, |m| {
        edit::inset_polygons(m, &[middle], 5.0, Grouping::Together);
    });
    assert_eq!(mended, 4);
    untouched_kept(&p, &m);
    for (f, s) in crate::stretch::stretch_of(&m).unwrap() {
        assert!(!s.empty, "{}", f.get());
        assert!(s.uneven < UNEVEN, "{}: {s:?}", f.get());
        assert!(
            s.flipped,
            "polygon {} isn't mirrored like its piece",
            f.get()
        );
    }
}

#[test]
fn sliding_inside_the_square() {
    assert_eq!(inside(0.2, 0.6), 0.0);
    assert_eq!(inside(-0.3, 0.2), 0.3);
    assert!((inside(0.8, 1.3) - -0.3).abs() < 1e-6);
    // Too big to fit: its lower edge goes to 0.
    assert!((inside(0.5, 2.0) - -0.5).abs() < 1e-6);
}

#[test]
fn shapes_without_uvs_or_changes_are_left_alone() {
    let c = cube();
    let mut same = c.clone();
    assert_eq!(mend(&c, &mut same), 0);
    assert_eq!(same, c);
    let mut bare = cube();
    bare.remove_attribute(mesh::attributes::UV);
    let top = facing(&bare, Vec3::Z);
    let (m, mended) = edited(&bare, |m| {
        edit::extrude_shaped(m, &[top], 50.0, Grouping::Together, Default::default());
    });
    assert_eq!(mended, 0);
    assert!(Uvs::of(&m).is_none());
}

#[test]
fn a_shape_without_seam_marks_is_cut_where_its_uvs_split() {
    // As an imported shape would be: UVs, no seam layer.
    let mut c = cube();
    c.remove_attribute(mesh::attributes::SEAM);
    let before_islands = islands(&c);
    let top = facing(&c, Vec3::Z);
    let (m, mended) = edited(&c, |m| {
        edit::extrude_shaped(m, &[top], 50.0, Grouping::Together, Default::default());
    });
    assert_eq!(mended, 4);
    all_well(&m);
    untouched_kept(&c, &m);
    assert_eq!(islands(&m), before_islands + 1);
}

/// A modelling edit to try.
type Tool<'a> = Box<dyn Fn(&mut Mesh) + 'a>;

#[test]
fn every_tool_on_every_primitive_mends_without_harm() {
    use mesh::PrimitiveKind;
    for kind in PrimitiveKind::ALL {
        let shape = kind.standard().mesh();
        let first = shape.faces().map(|(f, _)| f).next().unwrap();
        let list = shape.face(first).unwrap().to_vec();
        let edge = mesh::EdgeKey::new(list[0], list[1]);
        let borders = shape.topology().border_edges();
        let tools: Vec<(&str, Tool)> = vec![
            (
                "extrude",
                Box::new(|m: &mut Mesh| {
                    edit::extrude_shaped(m, &[first], 20.0, Grouping::Together, Default::default());
                }),
            ),
            (
                "flat extrude",
                Box::new(|m: &mut Mesh| {
                    edit::extrude_shaped(m, &[first], 0.0, Grouping::Together, Default::default());
                }),
            ),
            (
                "inset",
                Box::new(|m: &mut Mesh| {
                    edit::inset_polygons(m, &[first], 5.0, Grouping::Separate);
                }),
            ),
            (
                "bevel",
                Box::new(|m: &mut Mesh| {
                    edit::bevel_edges(m, &[edge], Default::default());
                }),
            ),
            (
                "bevel point",
                Box::new(|m: &mut Mesh| {
                    edit::bevel_points(m, &[list[0]], 5.0);
                }),
            ),
            (
                "loop cut",
                Box::new(|m: &mut Mesh| {
                    edit::loop_cut(m, edge, 3, 0.5);
                }),
            ),
            (
                "extrude edges",
                Box::new(|m: &mut Mesh| {
                    edit::extrude_edges(m, &borders, 10.0, Grouping::Together);
                }),
            ),
            (
                "delete",
                Box::new(|m: &mut Mesh| {
                    edit::delete_elements(m, &[], &[], &[first]);
                }),
            ),
        ];
        for (name, tool) in &tools {
            let (m, _) = edited(&shape, |m| tool(m));
            let what = format!("{kind:?} {name}");
            let uvs = Uvs::of(&m).unwrap_or_else(|| panic!("{what}: UVs lost"));
            let count = m.face_layout().1.len();
            assert!(
                uvs.into_values(count).iter().all(|v| v.is_finite()),
                "{what}: UVs not finite"
            );
            untouched_kept(&shape, &m);
            // Nothing is left with less room than the edit gave it.
            let mut plain = shape.clone();
            tool(&mut plain);
            let empty = |mesh: &Mesh| {
                crate::stretch::stretch_of(mesh)
                    .unwrap()
                    .iter()
                    .filter(|(_, s)| s.empty)
                    .count()
            };
            assert!(empty(&m) <= empty(&plain), "{what}");
        }
    }
}

/// `cargo test --release -p uv mend_benchmark -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn mend_benchmark() {
    for n in [50u32, 100, 200] {
        let p = plane(n);
        let middle = FaceId::from_raw(n * n / 2 + n / 2);
        let mut after = p.clone();
        edit::extrude_shaped(
            &mut after,
            &[middle],
            20.0,
            Grouping::Together,
            Default::default(),
        );
        let start = std::time::Instant::now();
        let mended = mend(&p, &mut after);
        println!(
            "{} polygons, one extruded: {mended} mended in {:?}",
            n * n,
            start.elapsed()
        );
        // A tenth of the polygons extruded each on their own.
        let some: Vec<FaceId> = p.faces().map(|(f, _)| f).step_by(10).collect();
        let mut after = p.clone();
        edit::extrude_shaped(
            &mut after,
            &some,
            20.0,
            Grouping::Separate,
            Default::default(),
        );
        let start = std::time::Instant::now();
        let mended = mend(&p, &mut after);
        println!(
            "{} polygons, {} extruded: {mended} mended in {:?}",
            n * n,
            some.len(),
            start.elapsed()
        );
    }
}
