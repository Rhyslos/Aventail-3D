//! Tests for editing pieces by hand in the window (decision 0236):
//! Stitch, Cut, Align and spread, Orient by edge, the lattice, and where
//! they are offered.

use super::modelling_tests::{H, note, press};
use super::seams_tests::{add, inner, seams, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use scene::{EdgeKey, Element, ElementSelection};

/// A plane `n` polygons across, selected, in the Edges mode.
fn strip(n: u32) -> (H, scene::ObjectId) {
    let mut harness = fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [n, 1],
            ..Default::default()
        }),
    );
    press(&mut harness, Modifiers::NONE, Key::Num2);
    (harness, id)
}

fn select_edges(harness: &mut H, id: scene::ObjectId, edges: &[EdgeKey]) {
    harness.state_mut().workbench.document_mut().select_elements(
        [(
            id,
            ElementSelection::of(edges.iter().copied().map(Element::Edge)),
        )],
        "Select",
        None,
    );
    harness.run();
}

fn islands(harness: &H) -> usize {
    mesh::uvs::islands(&shape(harness)).len()
}

#[test]
fn cut_parts_the_texture_and_stitch_joins_it_again() {
    let (mut harness, id) = strip(2);
    let edge = inner(&shape(&harness))[0];
    select_edges(&mut harness, id, &[edge]);
    assert_eq!(islands(&harness), 1);
    let before = steps(&harness);
    harness.state_mut().workbench.run_cut();
    harness.run();
    assert_eq!(islands(&harness), 2, "parted");
    assert!(seams(&harness).contains(&edge));
    assert_eq!(steps(&harness), before + 1);
    // Again: nothing more to cut.
    harness.state_mut().workbench.run_cut();
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("cut already"), "{}", note(&harness));
    // Stitch from the ring's command.
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::UvStitch, &ctx, &mut Vec::new());
    harness.run();
    assert_eq!(islands(&harness), 1, "joined");
    assert!(!seams(&harness).contains(&edge), "the seam goes");
    assert_eq!(steps(&harness), before + 2);
    harness.state_mut().workbench.run_stitch();
    assert!(note(&harness).contains("aren't cut"), "{}", note(&harness));
}

#[test]
fn stitch_and_cut_need_edges_or_polygons() {
    let (mut harness, _) = strip(2);
    harness.state_mut().workbench.run_stitch();
    assert!(note(&harness).contains("select edges"), "{}", note(&harness));
    harness.state_mut().workbench.run_cut();
    assert!(note(&harness).contains("select the edges"), "{}", note(&harness));
}

#[test]
fn align_lines_up_pieces_and_spread_needs_three() {
    let (mut harness, id) = strip(3);
    let edges = inner(&shape(&harness));
    select_edges(&mut harness, id, &edges);
    harness.state_mut().workbench.run_cut();
    harness.run();
    assert_eq!(islands(&harness), 3);
    // Move one piece up so they're out of line, by cutting moved it a
    // little already; align their bottoms.
    harness.state_mut().workbench.mode = Mode::Model;
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_align(uv::pieces::Align::Bottom);
    harness.run();
    let mesh = shape(&harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let low: Vec<f32> = mesh::uvs::islands(&mesh)
        .iter()
        .map(|p| {
            p.iter()
                .flat_map(|f| {
                    let s = starts[f.get() as usize] as usize;
                    (0..4).map(move |k| s + k)
                })
                .map(|k| uvs.at(k).y)
                .fold(f32::INFINITY, f32::min)
        })
        .collect();
    assert!(low.iter().all(|y| (y - low[0]).abs() < 1e-4), "{low:?}");
    assert!(steps(&harness) <= before + 1);
    // Spreading three works; one object of one piece says why not.
    harness
        .state_mut()
        .workbench
        .run_align(uv::pieces::Align::SpreadAcross);
    let (mut lone, _) = strip(1);
    lone.state_mut().workbench.mode = Mode::Model;
    lone.state_mut()
        .workbench
        .run_align(uv::pieces::Align::Left);
    assert!(note(&lone).contains("two or more pieces"), "{}", note(&lone));
}

#[test]
fn orient_by_edge_turns_the_piece_square() {
    let (mut harness, id) = strip(2);
    // Turn the whole piece a little in the texture.
    let mesh = shape(&harness);
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::TransformUvs {
                faces,
                points: Vec::new(),
                transform: glam::Affine2::from_angle(0.3),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    let edge = inner(&shape(&harness))[0];
    select_edges(&mut harness, id, &[edge]);
    harness.state_mut().workbench.run_orient();
    harness.run();
    let mesh = shape(&harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (starts, _) = mesh.face_layout();
    let f = mesh.topology().edge_faces(edge).next().unwrap();
    let list = mesh.face(f).unwrap();
    let at = |v| uvs.at(starts[f.get() as usize] as usize + list.iter().position(|w| *w == v).unwrap());
    let (a, b) = edge.ends();
    let d = at(b) - at(a);
    assert!(d.x.abs() < 1e-4 || d.y.abs() < 1e-4, "{d}");
    // Again: it lies straight already.
    harness.state_mut().workbench.run_orient();
    assert!(note(&harness).contains("straight already"), "{}", note(&harness));
    // Outside the Edges mode, it says how.
    harness.state_mut().workbench.mode = Mode::Polygons;
    harness.state_mut().workbench.run_orient();
    assert!(note(&harness).contains("Edges mode"), "{}", note(&harness));
}

#[test]
fn the_lattice_warps_the_pieces_one_drag_a_step_and_esc_puts_it_away() {
    let (mut harness, _) = strip(2);
    harness.state_mut().workbench.mode = Mode::Model;
    let before_uvs = mesh::uvs::Uvs::of(&shape(&harness))
        .unwrap()
        .into_values(8);
    harness.state_mut().workbench.start_lattice();
    harness.run();
    assert!(harness.state().workbench.warping());
    let before = steps(&harness);
    let middle = harness.state().workbench.lattice.as_ref().unwrap().points[4];
    harness
        .state_mut()
        .workbench
        .drag_lattice_handle(4, middle + glam::Vec2::new(0.1, 0.05));
    harness.run();
    assert_eq!(steps(&harness), before + 1);
    let after = mesh::uvs::Uvs::of(&shape(&harness)).unwrap().into_values(8);
    assert_ne!(after, before_uvs, "the UVs bent");
    harness
        .state_mut()
        .workbench
        .drag_lattice_handle(0, glam::Vec2::ZERO);
    assert_eq!(steps(&harness), before + 2, "one step per drag");
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!harness.state().workbench.warping());
    // Esc only put the lattice away.
    assert!(!harness.state().workbench.document().selection().is_empty());
}

#[test]
fn the_lattice_needs_pieces_with_size() {
    let mut harness = fresh();
    harness.state_mut().workbench.document_mut().clear_selection();
    harness.state_mut().workbench.start_lattice();
    assert!(!harness.state().workbench.warping());
    assert!(note(&harness).contains("select the pieces"), "{}", note(&harness));
}

#[test]
fn the_uv_menus_pieces_group_and_the_ring_offer_them() {
    let (mut harness, _) = strip(2);
    uv_group(&mut harness, super::UV_PIECES_MENU);
    for label in [
        uv_pieces::STITCH_LABEL,
        uv_pieces::CUT_LABEL,
        uv_pieces::ALIGN_LABEL,
        uv_pieces::ORIENT_LABEL,
        uv_pieces::LATTICE_LABEL,
    ] {
        assert!(harness.query_by_label_contains(label).is_some(), "{label}");
    }
    assert_eq!(
        CommandId::UvStitchCut.members(),
        &[CommandId::UvStitch, CommandId::UvCut]
    );
    assert!(
        crate::commands::UV_RING.contains(&Some(CommandId::UvStitchCut)),
        "in the ring's last place"
    );
}
