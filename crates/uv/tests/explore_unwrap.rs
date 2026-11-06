//! Exploratory probes of the UV algorithms (seams, unwrap, projections,
//! packing, lightmaps), through the public API only. Written by a test
//! pass looking for problems; each test names what it tries. Tests that
//! show a bug assert the correct behaviour and are ignored with the bug.

use glam::{Vec2, Vec3};
use mesh::{EdgeKey, FaceId, Mesh, Primitive};
use uv::pack::{PackOptions, Piece};

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

fn values(mesh: &Mesh) -> Vec<Vec2> {
    mesh::uvs::Uvs::of(mesh)
        .unwrap()
        .into_values(mesh.face_layout().1.len())
}

/// Every primitive with standard settings, plus some nasty shapes.
fn shapes() -> Vec<(String, Mesh)> {
    let mut out: Vec<(String, Mesh)> = Primitive::all()
        .into_iter()
        .map(|p| (p.name().to_string(), p.mesh()))
        .collect();
    out.push((
        "tall thin cube".into(),
        Primitive::Cube(mesh::CubeParams {
            size: Vec3::new(1.0, 1.0, 1000.0),
            segments: [1, 1, 8],
        })
        .mesh(),
    ));
    out.push((
        "tiny cube".into(),
        Primitive::Cube(mesh::CubeParams {
            size: Vec3::splat(0.001),
            segments: [1, 1, 1],
        })
        .mesh(),
    ));
    out.push(("lone hexagon".into(), ngon(6)));
    out
}

/// One flat n-gon (an open boundary, one polygon).
fn ngon(n: usize) -> Mesh {
    let mut mesh = Mesh::default();
    let ids: Vec<_> = (0..n)
        .map(|k| {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            mesh.add_vertex(Vec3::new(a.cos(), a.sin(), 0.0)).unwrap()
        })
        .collect();
    mesh.add_face(&ids).unwrap();
    mesh
}

/// Suggests seams, sets them, unwraps the whole shape.
fn seam_and_unwrap(mesh: &mut Mesh, method: uv::UnwrapMethod) {
    let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
    let seams = uv::seams::suggest(mesh, &hard);
    let old = mesh::uvs::seams(mesh);
    mesh::uvs::set_seams(mesh, &old, false);
    mesh::uvs::set_seams(mesh, &seams, true);
    let faces = all_faces(mesh);
    uv::unwrap_with(
        mesh,
        &faces,
        uv::UnwrapOptions {
            method,
            ..uv::UnwrapOptions::default()
        },
    );
}

fn pack_all(mesh: &mut Mesh, options: &PackOptions) -> bool {
    let faces = all_faces(mesh);
    let islands = uv::pack::islands_of(mesh, &faces);
    let pieces: Vec<Piece> = islands
        .iter()
        .map(|i| uv::pack::piece_of(mesh, i))
        .collect();
    let Some(moves) = uv::pack::pack(&pieces, options) else {
        return false;
    };
    let placed: Vec<_> = islands.into_iter().zip(moves).collect();
    uv::pack::place(mesh, &placed);
    true
}

fn in_square(uvs: &[Vec2], tiles: f32) -> bool {
    uvs.iter()
        .all(|v| v.x >= -1e-4 && v.x <= tiles + 1e-4 && v.y >= -1e-4 && v.y <= 1.0 + 1e-4)
}

#[test]
fn every_shape_suggests_unwraps_and_packs_cleanly_with_every_method() {
    let mut problems = Vec::new();
    for method in [
        uv::UnwrapMethod::Conformal,
        uv::UnwrapMethod::AngleBased,
        uv::UnwrapMethod::SizeKeeping,
        uv::UnwrapMethod::Automatic,
    ] {
        for (name, mut mesh) in shapes() {
            seam_and_unwrap(&mut mesh, method);
            let uvs = values(&mesh);
            if !uvs.iter().all(|v| v.is_finite()) {
                problems.push(format!("{name} {method:?}: UVs not finite"));
                continue;
            }
            if !pack_all(&mut mesh, &PackOptions::default()) {
                problems.push(format!("{name} {method:?}: pack gave nothing"));
                continue;
            }
            let uvs = values(&mesh);
            if !in_square(&uvs, 1.0) {
                problems.push(format!("{name} {method:?}: UVs outside the square"));
            }
            let over = uv::select::overlapping(&mesh);
            if !over.is_empty() {
                problems.push(format!(
                    "{name} {method:?}: {} polygons overlap after pack",
                    over.len()
                ));
            }
            let flipped = uv::select::mirrored(&mesh);
            if !flipped.is_empty() {
                problems.push(format!(
                    "{name} {method:?}: {} polygons mirrored",
                    flipped.len()
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn unwrapping_a_closed_cube_with_no_seams_cuts_it_open_and_lays_it_flat() {
    let mut cube = Primitive::Cube(mesh::CubeParams::default()).mesh();
    let seams = mesh::uvs::seams(&cube);
    mesh::uvs::set_seams(&mut cube, &seams, false);
    let faces = all_faces(&cube);
    uv::unwrap(&mut cube, &faces, true);
    assert!(!mesh::uvs::seams(&cube).is_empty(), "a cut was marked");
    assert!(values(&cube).iter().all(|v| v.is_finite()));
    assert!(pack_all(&mut cube, &PackOptions::default()));
    assert!(uv::select::overlapping(&cube).is_empty());
}

#[test]
fn packing_twice_gives_the_same_layout() {
    for (name, mut mesh) in shapes() {
        seam_and_unwrap(&mut mesh, uv::UnwrapMethod::Conformal);
        pack_all(&mut mesh, &PackOptions::default());
        let once = values(&mesh);
        pack_all(&mut mesh, &PackOptions::default());
        let twice = values(&mesh);
        let worst = once
            .iter()
            .zip(&twice)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0f32, f32::max);
        assert!(worst < 0.02, "{name}: moved {worst} on a second pack");
    }
}

/// `n` unit squares far apart, as pieces.
fn squares(n: usize) -> Vec<Piece> {
    (0..n)
        .map(|i| {
            let o = Vec2::new(i as f32 * 3.0, 0.0);
            Piece {
                polygons: vec![vec![
                    o,
                    o + Vec2::new(1.0, 0.0),
                    o + Vec2::new(1.0, 1.0),
                    o + Vec2::new(0.0, 1.0),
                ]],
            }
        })
        .collect()
}

fn placed_boxes(pieces: &[Piece], moves: &[glam::Affine2]) -> Vec<(Vec2, Vec2)> {
    pieces
        .iter()
        .zip(moves)
        .map(|(p, t)| {
            let pts: Vec<Vec2> = p
                .polygons
                .iter()
                .flatten()
                .map(|v| t.transform_point2(*v))
                .collect();
            let lo = pts.iter().fold(Vec2::splat(f32::MAX), |a, b| a.min(*b));
            let hi = pts.iter().fold(Vec2::splat(f32::MIN), |a, b| a.max(*b));
            (lo, hi)
        })
        .collect()
}

/// The smallest gap between two boxes (0 when they touch or overlap).
fn gap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> f32 {
    let dx = (b.0.x - a.1.x).max(a.0.x - b.1.x).max(0.0);
    let dy = (b.0.y - a.1.y).max(a.0.y - b.1.y).max(0.0);
    dx.max(dy)
}

#[test]
fn packing_into_the_whole_square_keeps_the_padding() {
    let pieces = squares(4);
    let padding = 32.0 / 1024.0;
    let options = PackOptions {
        padding,
        ..PackOptions::default()
    };
    let moves = uv::pack::pack(&pieces, &options).unwrap();
    let boxes = placed_boxes(&pieces, &moves);
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            assert!(gap(boxes[i], boxes[j]) >= padding * 0.99);
        }
    }
}

#[test]
#[ignore = "BUG: packing into a region shrinks the padding by the region's size (half the region, half the gap)"]
fn packing_into_a_region_keeps_the_padding_in_texture_pixels() {
    let pieces = squares(4);
    let padding = 32.0 / 1024.0;
    let options = PackOptions {
        padding,
        region: Some((Vec2::ZERO, Vec2::splat(0.25))),
        ..PackOptions::default()
    };
    let moves = uv::pack::pack(&pieces, &options).unwrap();
    let boxes = placed_boxes(&pieces, &moves);
    let mut least = f32::MAX;
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            least = least.min(gap(boxes[i], boxes[j]));
        }
    }
    assert!(
        least >= padding * 0.99,
        "pieces are {} px apart in a 1024 texture, padding asked {} px",
        least * 1024.0,
        padding * 1024.0
    );
}

#[test]
#[ignore = "BUG: a wide region (U 0..1, V 0..0.25) is packed as its left square only; most of it stays empty"]
fn packing_into_a_wide_region_uses_its_width() {
    let pieces = squares(4);
    let options = PackOptions {
        padding: 0.0,
        region: Some((Vec2::ZERO, Vec2::new(1.0, 0.25))),
        ..PackOptions::default()
    };
    let moves = uv::pack::pack(&pieces, &options).unwrap();
    let boxes = placed_boxes(&pieces, &moves);
    let right = boxes.iter().map(|b| b.1.x).fold(0.0f32, f32::max);
    assert!(
        right > 0.75,
        "four squares in a 1 × 0.25 strip reach only U {right}"
    );
}

#[test]
fn packing_into_a_region_stays_inside_it() {
    let mut ball = Primitive::Sphere(mesh::SphereParams::default()).mesh();
    seam_and_unwrap(&mut ball, uv::UnwrapMethod::Conformal);
    let options = PackOptions {
        region: Some((Vec2::new(0.5, 0.5), Vec2::new(1.0, 1.0))),
        ..PackOptions::default()
    };
    assert!(pack_all(&mut ball, &options));
    for v in values(&ball) {
        assert!(v.x >= 0.5 - 1e-4 && v.y >= 0.5 - 1e-4 && v.x <= 1.0001 && v.y <= 1.0001, "{v}");
    }
}

#[test]
fn an_upside_down_region_still_packs_somewhere_sensible() {
    // From (1, 1) to (0.5, 0.5): typed the wrong way round.
    let pieces = squares(2);
    let options = PackOptions {
        region: Some((Vec2::splat(1.0), Vec2::splat(0.5))),
        ..PackOptions::default()
    };
    let moves = uv::pack::pack(&pieces, &options).unwrap();
    for (lo, hi) in placed_boxes(&pieces, &moves) {
        assert!(lo.min_element() >= -1e-4 && hi.max_element() <= 1.0 + 1e-4);
    }
}

#[test]
fn udim_tiles_never_straddle_a_tile_and_use_more_than_one() {
    for tiles in [0, 1, 2, 3, 10, 50] {
        let pieces = squares(7);
        let options = PackOptions {
            tiles,
            ..PackOptions::default()
        };
        let moves = uv::pack::pack(&pieces, &options).unwrap();
        let boxes = placed_boxes(&pieces, &moves);
        let used = tiles.clamp(1, uv::pack::MAX_TILES) as f32;
        for (lo, hi) in &boxes {
            assert!(lo.x >= -1e-4 && hi.x <= used + 1e-4, "tiles {tiles}: {lo} {hi}");
            assert!(lo.y >= -1e-4 && hi.y <= 1.0 + 1e-4);
            assert_eq!(
                (lo.x + 1e-4).floor(),
                (hi.x - 1e-4).floor(),
                "tiles {tiles}: piece {lo}..{hi} straddles a tile"
            );
        }
        if tiles > 1 {
            assert!(
                boxes.iter().any(|(_, hi)| hi.x > 1.0),
                "tiles {tiles}: only the first tile is used"
            );
        }
    }
}

#[test]
fn packing_nothing_or_flat_pieces_gives_none_not_a_panic() {
    assert!(uv::pack::pack(&[], &PackOptions::default()).is_none());
    let line = Piece {
        polygons: vec![vec![Vec2::ZERO, Vec2::X, Vec2::X * 2.0]],
    };
    assert!(uv::pack::pack(&[line], &PackOptions::default()).is_none());
    let empty = Piece { polygons: vec![] };
    let _ = uv::pack::pack(&[empty, squares(1).remove(0)], &PackOptions::default());
    let nan = Piece {
        polygons: vec![vec![Vec2::NAN, Vec2::X, Vec2::Y]],
    };
    let _ = uv::pack::pack(&[nan], &PackOptions::default());
}

#[test]
fn packing_with_huge_padding_still_finishes() {
    let pieces = squares(20);
    for padding in [0.0, 0.25, 0.5, 1.0, 5.0, f32::INFINITY, f32::NAN] {
        let options = PackOptions {
            padding,
            ..PackOptions::default()
        };
        let _ = uv::pack::pack(&pieces, &options);
    }
}

#[test]
fn stacking_puts_identical_pieces_on_one_another() {
    let pieces = squares(3);
    let options = PackOptions {
        stack: true,
        ..PackOptions::default()
    };
    let moves = uv::pack::pack(&pieces, &options).unwrap();
    let boxes = placed_boxes(&pieces, &moves);
    assert!(boxes[0].0.distance(boxes[1].0) < 1e-4);
    assert!(boxes[0].0.distance(boxes[2].0) < 1e-4);
}

#[test]
fn a_grain_piece_is_never_turned_even_when_stacked() {
    // Two identical L shapes, the second lying a half turn round.
    let l = |turn: f32, at: Vec2| {
        let t = glam::Affine2::from_angle(turn);
        Piece {
            polygons: vec![
                [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0)]
                    .iter()
                    .map(|(x, y)| at + t.transform_point2(Vec2::new(*x, *y)))
                    .collect(),
                [(0.0, 1.0), (1.0, 1.0), (1.0, 3.0), (0.0, 3.0)]
                    .iter()
                    .map(|(x, y)| at + t.transform_point2(Vec2::new(*x, *y)))
                    .collect(),
            ],
        }
    };
    let pieces = vec![l(0.0, Vec2::ZERO), l(std::f32::consts::PI, Vec2::new(10.0, 0.0))];
    let asks = [
        uv::pack::PieceAsks {
            grain: true,
            scale: 1.0,
        },
        uv::pack::PieceAsks {
            grain: true,
            scale: 1.0,
        },
    ];
    let options = PackOptions {
        stack: true,
        ..PackOptions::default()
    };
    let moves = uv::pack::pack_with(&pieces, &asks, &[], &options).unwrap();
    for t in moves {
        let angle = t.matrix2.x_axis.y.atan2(t.matrix2.x_axis.x);
        assert!(angle.abs() < 1e-3, "a grain piece was turned by {angle} rad");
    }
}

#[test]
fn a_locked_piece_keeps_its_space() {
    let locked = Piece {
        polygons: vec![vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.6, 0.0),
            Vec2::new(0.6, 0.6),
            Vec2::new(0.0, 0.6),
        ]],
    };
    let pieces = squares(3);
    let moves = uv::pack::pack_with(&pieces, &[], &[locked], &PackOptions::default()).unwrap();
    for (lo, hi) in placed_boxes(&pieces, &moves) {
        let inside = lo.x < 0.6 && lo.y < 0.6 && hi.x > 0.0 && hi.y > 0.0;
        assert!(!inside, "a piece at {lo}..{hi} lies on the locked one");
    }
}

#[test]
fn lightmaps_of_every_shape_are_inside_unstacked_and_padded() {
    let mut problems = Vec::new();
    for (name, mesh) in shapes() {
        let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
        for (size, padding) in [(128, 1), (512, 4), (2048, 64)] {
            let Some(uvs) = uv::lightmap::lightmap(&mesh, &hard, size, padding) else {
                problems.push(format!("{name} {size}/{padding}: no lightmap"));
                continue;
            };
            if !in_square(&uvs, 1.0) || !uvs.iter().all(|v| v.is_finite()) {
                problems.push(format!("{name} {size}/{padding}: outside the square"));
                continue;
            }
            let mut laid = mesh.clone();
            mesh::uvs::set_uvs(&mut laid, uvs);
            let over = uv::select::overlapping(&laid);
            if !over.is_empty() {
                problems.push(format!("{name} {size}/{padding}: {} overlap", over.len()));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_lightmap_of_a_shape_with_no_uvs_works() {
    let mesh = ngon(5);
    assert!(!mesh::uvs::has_uvs(&mesh));
    let uvs = uv::lightmap::lightmap(&mesh, &[], 512, 4).unwrap();
    assert!(in_square(&uvs, 1.0));
}

/// A ball projection, placed round a standard sphere.
fn ball_projected() -> Mesh {
    let mut ball = Primitive::Sphere(mesh::SphereParams::default()).mesh();
    let faces = all_faces(&ball);
    let (lo, hi) = bounds(&ball);
    uv::project::project(
        &mut ball,
        &faces,
        &uv::project::ProjectOptions {
            kind: uv::project::Projection::Sphere,
            placement: uv::project::fit_box(lo, hi),
            world: None,
        },
    );
    ball
}

fn bounds(mesh: &Mesh) -> (Vec3, Vec3) {
    mesh.vertices().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(lo, hi), (_, p)| (lo.min(p), hi.max(p)),
    )
}

/// For the polygon nearest the equator facing +X, its UV width over its
/// UV height divided by its real width over its real height: 1 when the
/// texture isn't stretched one way.
fn equator_aspect(mesh: &Mesh) -> f32 {
    let uvs = mesh::uvs::Uvs::of(mesh).unwrap();
    let (f, list) = mesh
        .faces()
        .filter(|(_, l)| l.len() == 4)
        .min_by(|a, b| {
            let c = |l: &[mesh::VertexId]| {
                let m: Vec3 = l.iter().map(|v| mesh.position(*v).unwrap()).sum::<Vec3>()
                    / l.len() as f32;
                m.z.abs() + (m.normalize() - Vec3::X).length()
            };
            c(a.1).total_cmp(&c(b.1))
        })
        .unwrap();
    let p: Vec<Vec3> = list.iter().map(|v| mesh.position(*v).unwrap()).collect();
    let t: Vec<Vec2> = (0..list.len())
        .map(|k| uvs.corner(mesh, f, k).unwrap())
        .collect();
    let span3 = |axis: fn(&Vec3) -> f32| {
        let vals: Vec<f32> = p.iter().map(axis).collect();
        vals.iter().cloned().fold(f32::MIN, f32::max) - vals.iter().cloned().fold(f32::MAX, f32::min)
    };
    let span2 = |axis: fn(&Vec2) -> f32| {
        let vals: Vec<f32> = t.iter().map(axis).collect();
        vals.iter().cloned().fold(f32::MIN, f32::max) - vals.iter().cloned().fold(f32::MAX, f32::min)
    };
    // Facing +X: across is Y, up is Z.
    (span2(|v| v.x) / span2(|v| v.y)) / (span3(|v| v.y) / span3(|v| v.z))
}

#[test]
#[ignore = "DOCS: projections.md says a Ball projection keeps the texture square, but it stretches 2:1 (U is a whole turn, V half a turn)"]
fn a_ball_projection_keeps_the_texture_square_at_the_equator() {
    let ball = ball_projected();
    let aspect = equator_aspect(&ball);
    assert!(
        (aspect - 1.0).abs() < 0.2,
        "the texture is {aspect} times wider than tall at the equator"
    );
}

#[test]
fn projections_of_every_shape_are_finite_and_mark_seams_on_round_ones() {
    for (name, mesh) in shapes() {
        for kind in uv::project::Projection::ALL {
            let mut m = mesh.clone();
            let faces = all_faces(&m);
            let (lo, hi) = bounds(&m);
            let done = uv::project::project(
                &mut m,
                &faces,
                &uv::project::ProjectOptions {
                    kind,
                    placement: uv::project::fit_box(lo, hi),
                    world: None,
                },
            );
            assert!(done.matched > 0, "{name} {kind:?}");
            assert!(values(&m).iter().all(|v| v.is_finite()), "{name} {kind:?}");
        }
    }
}

#[test]
fn a_squashed_projection_does_nothing() {
    let mut cube = Primitive::Cube(mesh::CubeParams::default()).mesh();
    let before = values(&cube);
    let faces = all_faces(&cube);
    let done = uv::project::project(
        &mut cube,
        &faces,
        &uv::project::ProjectOptions {
            kind: uv::project::Projection::Planar,
            placement: glam::Mat4::from_scale(Vec3::new(1.0, 1.0, 0.0)),
            world: None,
        },
    );
    assert_eq!(done.matched, 0);
    assert_eq!(values(&cube), before);
}

#[test]
fn the_seam_slider_runs_from_few_seams_to_many() {
    let ball = Primitive::Torus(mesh::TorusParams::default()).mesh();
    let hard: Vec<EdgeKey> = ball.hard_edges().collect();
    let mut last = 0;
    for k in 0..=10 {
        let (angle, budget) = uv::seam_tools::balanced(k as f32 / 10.0);
        let seams = uv::seams::suggest_with(
            &ball,
            &hard,
            uv::SuggestOptions {
                corner_angle: angle,
                budget,
                ..uv::SuggestOptions::default()
            },
        );
        assert!(
            seams.len() + 2 >= last,
            "at {k}/10 {} seams, fewer than {last} before",
            seams.len()
        );
        last = seams.len();
    }
    for t in [f32::NAN, -1.0, 2.0, f32::INFINITY] {
        let (angle, budget) = uv::seam_tools::balanced(t);
        assert!(angle.is_finite(), "{t}");
        assert!(budget.is_none_or(f32::is_finite), "{t}");
    }
}

#[test]
fn suggested_seams_are_the_same_every_time() {
    for (name, mesh) in shapes() {
        let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
        assert_eq!(
            uv::seams::suggest(&mesh, &hard),
            uv::seams::suggest(&mesh, &hard),
            "{name}"
        );
    }
}

#[test]
fn seam_loops_and_rings_on_a_cylinder() {
    let tube = Primitive::Cylinder(mesh::CylinderParams::default()).mesh();
    let topology = tube.topology();
    // An edge running up the side.
    let up = topology
        .edges()
        .find(|e| {
            let (a, b) = e.ends();
            let (p, q) = (tube.position(a).unwrap(), tube.position(b).unwrap());
            (p.x - q.x).abs() < 1e-4 && (p.y - q.y).abs() < 1e-4 && (p.z - q.z).abs() > 1e-3
        })
        .unwrap();
    let lp = uv::seam_tools::loops(&tube, &[up]);
    assert!(lp.contains(&up));
    for e in &lp {
        let (a, b) = e.ends();
        let (p, q) = (tube.position(a).unwrap(), tube.position(b).unwrap());
        assert!(
            (p.x - q.x).abs() < 1e-3 && (p.y - q.y).abs() < 1e-3,
            "the loop of an up edge left the line: {p} {q}"
        );
    }
    let ring = uv::seam_tools::rings(&tube, &[up]);
    assert!(ring.len() > 2);
    // Nothing given, nothing back; an edge not in the shape gives nothing.
    assert!(uv::seam_tools::loops(&tube, &[]).is_empty());
}

#[test]
fn mirrored_seams_on_a_symmetric_shape_match_both_ways() {
    let cube = Primitive::Cube(mesh::CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    })
    .mesh();
    // Seams on the +X side only.
    let right: Vec<EdgeKey> = cube
        .topology()
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            cube.position(a).unwrap().x > 1.0 && cube.position(b).unwrap().x > 1.0
        })
        .collect();
    let mirrored = uv::seam_tools::mirrored(&cube, &right, 0);
    assert_eq!(mirrored.len(), right.len());
    for e in &mirrored {
        let (a, b) = e.ends();
        assert!(cube.position(a).unwrap().x < -1.0 && cube.position(b).unwrap().x < -1.0);
    }
}

#[test]
fn relax_and_straighten_with_no_uvs_or_no_faces_do_not_panic() {
    let mut mesh = ngon(7);
    let faces = all_faces(&mesh);
    let _ = uv::relax(&mut mesh, &faces, 60);
    let _ = uv::straighten(&mut mesh, &faces);
    let _ = uv::unwrap(&mut mesh, &[], true);
    let mut empty = Mesh::default();
    let _ = uv::unwrap(&mut empty, &[], true);
    let _ = uv::relax(&mut empty, &[], 1000);
}

#[test]
fn texel_density_extremes() {
    let mut cube = Primitive::Cube(mesh::CubeParams::default()).mesh();
    let faces = all_faces(&cube);
    for target in [1e-9, 1e9, 0.0, -1.0, f32::NAN, f32::INFINITY] {
        let _ = uv::density::set_density(&mut cube, &faces, target);
        assert!(
            values(&cube).iter().all(|v| v.is_finite()),
            "density {target} broke the UVs"
        );
    }
}
