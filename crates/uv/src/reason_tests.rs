//! Why each suggested seam is where it is (decision 0116): the reasons
//! Suggest seams gives match what it does.

use crate::seams::{SeamReason, SuggestOptions, edge_angle, suggest_explained, suggest_with};
use mesh::uvs::{SeamPreference, set_seam_preference};
use mesh::{CubeParams, EdgeKey, Mesh, PlaneParams, Primitive, PrimitiveKind, SphereParams};
use std::collections::BTreeSet;

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn inner_edges(mesh: &Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect()
}

#[test]
fn the_reasons_cover_exactly_the_suggested_seams() {
    for kind in PrimitiveKind::ALL {
        let mesh = kind.standard().mesh();
        let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
        for options in [
            SuggestOptions::default(),
            SuggestOptions {
                budget: Some(1.5),
                ..SuggestOptions::default()
            },
        ] {
            let plain = suggest_with(&mesh, &hard, options);
            let explained: Vec<EdgeKey> = suggest_explained(&mesh, &hard, options)
                .into_iter()
                .map(|(e, _)| e)
                .collect();
            assert_eq!(plain, explained, "{kind:?}");
        }
    }
}

#[test]
fn hard_edges_and_sharp_corners_say_so() {
    let mesh = cube();
    let hard: Vec<EdgeKey> = mesh.hard_edges().collect();
    assert!(!hard.is_empty());
    let with_hard = suggest_explained(&mesh, &hard, SuggestOptions::default());
    assert!(
        with_hard
            .iter()
            .all(|(e, r)| hard.contains(e) && *r == SeamReason::HardEdge),
        "{with_hard:?}"
    );
    let corners_only = suggest_explained(
        &mesh,
        &[],
        SuggestOptions {
            hard_edges: false,
            ..SuggestOptions::default()
        },
    );
    assert!(!corners_only.is_empty());
    for (_, r) in &corners_only {
        match r {
            SeamReason::SharpCorner(angle) => assert!((angle - 90.0).abs() < 0.1, "{angle}"),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn closed_pieces_are_cut_to_lie_flat_and_preferred_edges_say_so() {
    let options = SuggestOptions {
        corner_angle: 180.0,
        hard_edges: false,
        ..SuggestOptions::default()
    };
    let mut mesh = cube();
    let plain = suggest_explained(&mesh, &[], options);
    assert!(!plain.is_empty());
    assert!(plain.iter().all(|(_, r)| *r == SeamReason::ToLieFlat));
    let cut: BTreeSet<EdgeKey> = plain.iter().map(|(e, _)| *e).collect();
    let others: Vec<EdgeKey> = inner_edges(&mesh)
        .into_iter()
        .filter(|e| !cut.contains(e))
        .take(3)
        .collect();
    set_seam_preference(&mut mesh, &others, SeamPreference::Prefer);
    let preferred = suggest_explained(&mesh, &[], options);
    assert!(
        preferred
            .iter()
            .any(|(e, r)| others.contains(e) && *r == SeamReason::Preferred),
        "{preferred:?}"
    );
    assert!(
        preferred
            .iter()
            .filter(|(e, _)| !others.contains(e))
            .all(|(_, r)| *r == SeamReason::ToLieFlat)
    );
}

#[test]
fn budget_slits_name_their_budget() {
    let mut mesh = Primitive::Sphere(SphereParams {
        sides: 16,
        rings: 10,
        ..Default::default()
    })
    .mesh();
    let seams = mesh::uvs::seams(&mesh);
    mesh::uvs::set_seams(&mut mesh, &seams, false);
    let reasons = suggest_explained(
        &mesh,
        &[],
        SuggestOptions {
            budget: Some(1.5),
            ..SuggestOptions::default()
        },
    );
    assert!(reasons.iter().any(|(_, r)| *r == SeamReason::ToLieFlat));
    assert!(
        reasons
            .iter()
            .any(|(_, r)| *r == SeamReason::StretchBudget(1.5)),
        "{reasons:?}"
    );
}

#[test]
fn edge_angles_are_measured_between_facings() {
    let mesh = cube();
    for e in inner_edges(&mesh) {
        assert!((edge_angle(&mesh, e).unwrap() - 90.0).abs() < 0.01);
    }
    let plane = Primitive::Plane(PlaneParams {
        segments: [2, 1],
        ..Default::default()
    })
    .mesh();
    let topology = plane.topology();
    for e in topology.edges() {
        match topology.edge_uses(e).len() {
            2 => assert!(edge_angle(&plane, e).unwrap() < 0.01),
            _ => assert!(edge_angle(&plane, e).is_none(), "a border edge"),
        }
    }
    assert!(suggest_explained(&Mesh::new(), &[], SuggestOptions::default()).is_empty());
}
