//! Seam preferences and stretch budgets (decision 0111): "never" edges
//! are left joined, "prefer" edges are cut first, and pieces over budget
//! are slit until within it, the same way every time.

use crate::seams::{HideSeams, SuggestOptions, budget_cuts, pieces, suggest_with};
use crate::unwrap::{UnwrapOptions, unwrap_with};
use mesh::uvs::{SeamPreference, set_budget, set_seam_preference};
use mesh::{CubeParams, EdgeKey, FaceId, Mesh, Primitive, SphereParams};
use std::collections::BTreeSet;

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

/// A sphere with no seams.
fn bare_sphere() -> Mesh {
    let mut mesh = Primitive::Sphere(SphereParams {
        sides: 16,
        rings: 10,
        ..Default::default()
    })
    .mesh();
    let seams = mesh::uvs::seams(&mesh);
    mesh::uvs::set_seams(&mut mesh, &seams, false);
    mesh
}

fn all_faces(mesh: &Mesh) -> Vec<FaceId> {
    mesh.faces().map(|(f, _)| f).collect()
}

fn inner_edges(mesh: &Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect()
}

/// How far off nearly all of `piece` is laid flat along `cuts`.
fn worst(mesh: &Mesh, piece: &[FaceId], cuts: &BTreeSet<EdgeKey>) -> f64 {
    let chart = crate::chart::Chart::of(mesh, piece, cuts);
    let none = vec![None; chart.len()];
    let (at, _) =
        crate::methods::lay_flat(mesh, &chart, crate::UnwrapMethod::Conformal, &none, &[]).unwrap();
    let (triangles, shapes) = crate::methods::real_shapes(mesh, &chart);
    let each = crate::methods::stretch_each(&triangles, &shapes, &at);
    crate::methods::nearly_all(&each, &shapes)
}

#[test]
fn never_edges_are_not_cut_even_as_hard_edges_or_corners() {
    let mut mesh = cube();
    let all = suggest_with(&mesh, &[], SuggestOptions::default());
    assert_eq!(all.len(), 12);
    let protected = all[..4].to_vec();
    set_seam_preference(&mut mesh, &protected, SeamPreference::Never);
    let now = suggest_with(&mesh, &protected, SuggestOptions::default());
    assert!(protected.iter().all(|e| !now.contains(e)), "{now:?}");
    assert!(!now.is_empty());
    // Unwrap's cut-open of a closed piece keeps away from them too.
    let mut sphere = bare_sphere();
    let first = {
        let mut copy = sphere.clone();
        let faces = all_faces(&copy);
        unwrap_with(&mut copy, &faces, UnwrapOptions::plain(false));
        mesh::uvs::seams(&copy)
    };
    set_seam_preference(&mut sphere, &first, SeamPreference::Never);
    let faces = all_faces(&sphere);
    unwrap_with(&mut sphere, &faces, UnwrapOptions::plain(false));
    let cut = mesh::uvs::seams(&sphere);
    assert!(!cut.is_empty());
    assert!(cut.iter().all(|e| !first.contains(e)), "{cut:?}");
}

#[test]
fn preferred_edges_take_the_cuts_that_are_needed() {
    // A cube with no corners or hard edges cut needs a few cuts to lie
    // flat; with some edges preferred, those are used.
    let options = SuggestOptions {
        corner_angle: 180.0,
        hard_edges: false,
        ..SuggestOptions::default()
    };
    let mut mesh = cube();
    let plain: BTreeSet<EdgeKey> = suggest_with(&mesh, &[], options).into_iter().collect();
    let others: Vec<EdgeKey> = inner_edges(&mesh)
        .into_iter()
        .filter(|e| !plain.contains(e))
        .take(3)
        .collect();
    assert!(!others.is_empty());
    set_seam_preference(&mut mesh, &others, SeamPreference::Prefer);
    let preferred: BTreeSet<EdgeKey> = suggest_with(&mesh, &[], options).into_iter().collect();
    assert!(
        others.iter().any(|e| preferred.contains(e)),
        "{others:?} {preferred:?}"
    );
    assert_ne!(preferred, plain);
}

#[test]
fn a_piece_over_its_budget_is_slit_until_it_is_within() {
    // The top half of a sphere (it sits on the ground, its middle 50 up)
    // is a disk that stretches when laid flat.
    let mesh = bare_sphere();
    let half: Vec<FaceId> = mesh
        .faces()
        .filter(|(_, l)| {
            let z: f32 = l.iter().map(|v| mesh.position(*v).unwrap().z).sum();
            z / l.len() as f32 > 50.0
        })
        .map(|(f, _)| f)
        .collect();
    let none = BTreeSet::new();
    let before = worst(&mesh, &half, &none);
    let budget = 1.2;
    assert!(before > f64::from(budget), "{before}");
    let slits = budget_cuts(&mesh, &half, &none, budget, HideSeams::BelowAndBehind);
    assert!(!slits.is_empty());
    let after = worst(&mesh, &half, &slits);
    assert!(after < before, "{before} -> {after}");
    assert!(after <= f64::from(budget) + 1e-9, "{after}");
    // Still one piece, still a disk.
    assert_eq!(
        pieces(&mesh, &slits)
            .iter()
            .filter(|p| p.iter().any(|f| half.contains(f)))
            .count(),
        1
    );
    // Within budget already: nothing; the same every time.
    assert!(budget_cuts(&mesh, &half, &none, 10.0, HideSeams::BelowAndBehind).is_empty());
    assert_eq!(
        budget_cuts(&mesh, &half, &none, budget, HideSeams::BelowAndBehind),
        slits
    );
    // Nonsense budgets do nothing.
    for bad in [f32::NAN, 0.5] {
        assert!(budget_cuts(&mesh, &half, &none, bad, HideSeams::BelowAndBehind).is_empty());
    }
}

#[test]
fn suggest_and_unwrap_obey_the_budget_and_a_pieces_own() {
    let mesh = bare_sphere();
    let loose = suggest_with(&mesh, &[], SuggestOptions::default());
    let strict = suggest_with(
        &mesh,
        &[],
        SuggestOptions {
            budget: Some(1.5),
            ..SuggestOptions::default()
        },
    );
    assert!(
        strict.len() > loose.len(),
        "{} {}",
        strict.len(),
        loose.len()
    );
    // A piece's own budget is used without a budget for every piece.
    let mut own = mesh.clone();
    let faces = all_faces(&own);
    set_budget(&mut own, &faces, Some(1.5));
    assert_eq!(suggest_with(&own, &[], SuggestOptions::default()), strict);
    // Unwrap cuts further, and marks the cuts as seams.
    let mut plain = mesh.clone();
    unwrap_with(&mut plain, &faces, UnwrapOptions::plain(false));
    let mut held = mesh.clone();
    let options = UnwrapOptions {
        budget: Some(1.5),
        ..UnwrapOptions::plain(false)
    };
    unwrap_with(&mut held, &faces, options);
    assert!(mesh::uvs::seams(&held).len() > mesh::uvs::seams(&plain).len());
}
