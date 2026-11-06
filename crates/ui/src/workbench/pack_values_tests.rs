//! Tests for packing control in the window (decision 0240).

use super::modelling_tests::{H, note};
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use egui_kittest::kittest::Queryable;
use mesh::uvs::{PackParts, PackValues};

fn cube(harness: &mut H) -> scene::ObjectId {
    add(harness, scene::Primitive::Cube(Default::default()))
}

#[test]
fn packing_values_are_set_on_whole_pieces_in_one_step() {
    let mut harness = fresh();
    cube(&mut harness);
    let before = steps(&harness);
    harness.state_mut().workbench.run_pack_values(
        PackValues {
            group: 2,
            grain: 1,
            scale: 1.5,
        },
        PackParts {
            group: true,
            grain: true,
            scale: true,
        },
    );
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let mesh = shape(&harness);
    let all: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let v = mesh::uvs::pack_values(&mesh, &all);
    assert_eq!((v.group, v.grain, v.scale), (2, 1, 1.5));
    assert!(
        note(&harness).contains("Pack to see it"),
        "{}",
        note(&harness)
    );
    // Only the group taken away: the grain and scale stay.
    harness.state_mut().workbench.run_pack_values(
        PackValues::default(),
        PackParts {
            group: true,
            ..PackParts::default()
        },
    );
    harness.run();
    let v = mesh::uvs::pack_values(&shape(&harness), &all);
    assert_eq!((v.group, v.grain), (0, 1));
}

#[test]
fn a_packing_group_stays_together_when_packed() {
    let mut harness = fresh();
    cube(&mut harness);
    harness.state_mut().workbench.run_pack_values(
        PackValues {
            group: 1,
            ..PackValues::default()
        },
        PackParts {
            group: true,
            ..PackParts::default()
        },
    );
    harness.run();
    let pieces = mesh::uvs::islands(&shape(&harness));
    let mid = |m: &scene::Mesh, p: &[scene::FaceId]| {
        let (lo, hi) = mesh::uvs::uv_bounds(m, p, &[]).unwrap();
        (lo + hi) / 2.0
    };
    let m = shape(&harness);
    let apart = mid(&m, &pieces[1]) - mid(&m, &pieces[0]);
    harness.state_mut().workbench.run_pack();
    harness.run();
    let m = shape(&harness);
    let now = mid(&m, &pieces[1]) - mid(&m, &pieces[0]);
    // Packed as one block: moved and sized together, the same way round.
    let k = now.length() / apart.length();
    assert!((now - apart * k).length() < 1e-3, "{apart} {now}");
}

#[test]
fn the_packing_menu_and_the_pieces_menu_offer_the_new_settings() {
    let mut harness = fresh();
    cube(&mut harness);
    uv_group(&mut harness, super::UV_PIECES_MENU);
    assert!(
        harness
            .query_by_label_contains(pack_values::PACK_VALUES_LABEL)
            .is_some()
    );
}
