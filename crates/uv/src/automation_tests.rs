//! Every automatic tool, fully controlled (decision 0110): each choice
//! changes what it should, and the same shape and choices always give
//! exactly the same result.

use crate::pack::{PackOptions, Piece, islands_of, pack, piece_of};
use crate::seams::{HideSeams, SuggestOptions, suggest, suggest_with};
use crate::unwrap::{UnwrapOptions, unwrap_with};
use crate::{UnwrapMethod, relax};
use glam::{Vec2, Vec3};
use mesh::uvs::Uvs;
use mesh::{CubeParams, EdgeKey, FaceId, Mesh, PlaneParams, Primitive, SphereParams};

fn sphere() -> Mesh {
    Primitive::Sphere(SphereParams {
        sides: 16,
        rings: 10,
        ..Default::default()
    })
    .mesh()
}

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

fn corner_uvs(mesh: &Mesh) -> Vec<Vec2> {
    let count = mesh.face_layout().1.len();
    Uvs::of(mesh).unwrap().into_values(count)
}

/// The middle of each edge.
fn middles(mesh: &Mesh, edges: &[EdgeKey]) -> Vec<Vec3> {
    edges
        .iter()
        .map(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap() + mesh.position(b).unwrap()) / 2.0
        })
        .collect()
}

fn average(points: &[Vec3]) -> Vec3 {
    points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32
}

/// A shape with no seams and no UVs: a sphere's seams taken off.
fn bare_sphere() -> Mesh {
    let mut mesh = sphere();
    let seams = mesh::uvs::seams(&mesh);
    mesh::uvs::set_seams(&mut mesh, &seams, false);
    mesh
}

#[test]
fn the_same_shape_and_choices_always_give_the_same_result() {
    for hide in HideSeams::ALL {
        let options = SuggestOptions {
            hide,
            ..SuggestOptions::default()
        };
        let mesh = bare_sphere();
        let first = suggest_with(&mesh, &[], options);
        for _ in 0..3 {
            assert_eq!(
                suggest_with(&bare_sphere(), &[], options),
                first,
                "{hide:?}"
            );
        }
    }
    for method in UnwrapMethod::ALL {
        let options = UnwrapOptions {
            method,
            ..UnwrapOptions::plain(true)
        };
        let run = || {
            let mut mesh = bare_sphere();
            let faces = all_faces(&mesh);
            unwrap_with(&mut mesh, &faces, options);
            relax(&mut mesh, &faces, 30);
            corner_uvs(&mesh)
        };
        let first = run();
        for _ in 0..3 {
            assert_eq!(run(), first, "{method:?}");
        }
    }
    let mesh = sphere();
    let pieces: Vec<Piece> = islands_of(&mesh, &all_faces(&mesh))
        .iter()
        .map(|faces| piece_of(&mesh, faces))
        .collect();
    let first = pack(&pieces, &PackOptions::default());
    for _ in 0..3 {
        assert_eq!(pack(&pieces, &PackOptions::default()), first);
    }
}

#[test]
fn suggest_follows_its_corner_angle_and_hard_edges() {
    let cube = cube();
    let all = suggest_with(&cube, &[], SuggestOptions::default());
    assert_eq!(all.len(), 12, "every 90° corner is sharper than 66°");
    let none = SuggestOptions {
        corner_angle: 180.0,
        ..SuggestOptions::default()
    };
    let fewer = suggest_with(&cube, &[], none);
    assert!(
        !fewer.is_empty() && fewer.len() < all.len(),
        "{}",
        fewer.len()
    );
    // Hard edges are cut only when asked.
    let flat = Primitive::Plane(PlaneParams {
        segments: [3, 3],
        ..Default::default()
    })
    .mesh();
    let topology = flat.topology();
    let inner: Vec<EdgeKey> = topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .take(2)
        .collect();
    assert_eq!(suggest(&flat, &inner), inner);
    let without = SuggestOptions {
        hard_edges: false,
        ..SuggestOptions::default()
    };
    assert!(suggest_with(&flat, &inner, without).is_empty());
}

#[test]
fn cuts_hide_where_they_are_told() {
    let mesh = bare_sphere();
    let place = |hide| {
        let options = SuggestOptions {
            hide,
            ..SuggestOptions::default()
        };
        average(&middles(&mesh, &suggest_with(&mesh, &[], options)))
    };
    // The back is +Y (away from the Front view).
    let behind = place(HideSeams::Behind);
    let below = place(HideSeams::Below);
    assert!(behind.y > 0.0, "{behind}");
    assert!(behind.y > below.y, "{behind} {below}");
    let shortest = suggest_with(
        &mesh,
        &[],
        SuggestOptions {
            hide: HideSeams::Shortest,
            ..SuggestOptions::default()
        },
    );
    assert!(!shortest.is_empty());
}

#[test]
fn how_square_a_strip_must_be_is_the_users() {
    // A strip of quads leaning 30° from square.
    let mut strip = Primitive::Plane(PlaneParams {
        segments: [4, 1],
        ..Default::default()
    })
    .mesh();
    let points: Vec<_> = strip.vertices().collect();
    for (v, p) in points {
        let lean = p.y * 30f32.to_radians().tan();
        strip.set_position(v, p + Vec3::new(lean, 0.0, 0.0));
    }
    let lined_up = |mesh: &Mesh| {
        let uvs = Uvs::of(mesh).unwrap();
        mesh.faces().all(|(f, list)| {
            (0..list.len()).all(|k| {
                let a = uvs.corner(mesh, f, k).unwrap();
                let b = uvs.corner(mesh, f, (k + 1) % list.len()).unwrap();
                let d = b - a;
                d.x.abs() < 1e-4 || d.y.abs() < 1e-4
            })
        })
    };
    for (angle, straight) in [(25.0, false), (40.0, true)] {
        let mut mesh = strip.clone();
        let faces = all_faces(&mesh);
        let options = UnwrapOptions {
            strip_angle: angle,
            ..UnwrapOptions::plain(true)
        };
        unwrap_with(&mut mesh, &faces, options);
        assert_eq!(lined_up(&mesh), straight, "{angle}°");
    }
}

#[test]
fn unwrap_cuts_a_closed_piece_where_it_is_told() {
    let cut_with = |hide| {
        let mut mesh = bare_sphere();
        let faces = all_faces(&mesh);
        let options = UnwrapOptions {
            hide,
            ..UnwrapOptions::plain(false)
        };
        unwrap_with(&mut mesh, &faces, options);
        let seams = mesh::uvs::seams(&mesh);
        assert!(!seams.is_empty(), "{hide:?}");
        average(&middles(&mesh, &seams))
    };
    assert!(cut_with(HideSeams::Behind).y > cut_with(HideSeams::Below).y);
}

#[test]
fn relax_strength_is_how_far_it_goes() {
    // A plane's UVs bunched to one side, then relaxed a little and a lot.
    let mut bent = Primitive::Plane(PlaneParams {
        segments: [4, 4],
        ..Default::default()
    })
    .mesh();
    let squeezed: Vec<Vec2> = corner_uvs(&bent)
        .iter()
        .map(|v| Vec2::new(v.x * v.x, v.y))
        .collect();
    mesh::uvs::set_uvs(&mut bent, squeezed);
    let spread = |steps: usize| {
        let mut mesh = bent.clone();
        let faces = all_faces(&mesh);
        relax(&mut mesh, &faces, steps);
        crate::stretch::stretch_of(&mesh)
            .unwrap()
            .iter()
            .map(|(_, s)| s.uneven)
            .fold(1.0f32, f32::max)
    };
    let (little, lot) = (spread(2), spread(300));
    assert!(lot < little, "{little} {lot}");
}
