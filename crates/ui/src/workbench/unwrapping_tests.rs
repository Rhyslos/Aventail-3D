//! Tests for Unwrap, Relax and Straighten in the window, and unwrapping
//! live as seams change (decision 0099).

use super::modelling_tests::{H, note, press};
use super::tests::{fresh, main_menu};
use super::unwrapping::UvSettings;
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use scene::{EdgeKey, Element, ElementSelection, FaceId};

fn add(harness: &mut H, primitive: scene::Primitive) -> scene::ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        primitive,
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    harness.run();
    id
}

fn shape(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    let id = *doc.selection().first().unwrap();
    doc.scene().evaluated(id).unwrap().mesh
}

/// The last edit recorded on the first selected object.
fn last_operation(harness: &H) -> Option<scene::Operation> {
    let doc = harness.state().workbench.document();
    let id = *doc.selection().first()?;
    let object = doc.scene().object(id)?;
    object
        .stack
        .layers()
        .last()?
        .entries
        .last()
        .map(|e| e.operation.clone())
}

fn plane(harness: &mut H) -> scene::ObjectId {
    add(
        harness,
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 2],
            ..Default::default()
        }),
    )
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

fn steps(harness: &H) -> usize {
    harness.state().workbench.document().undo_steps()
}

/// Whether every polygon side in the texture runs straight across or up.
fn lined_up(mesh: &scene::Mesh) -> bool {
    let uvs = mesh::uvs::Uvs::of(mesh).unwrap();
    mesh.faces().all(|(f, list)| {
        (0..list.len()).all(|k| {
            let a = uvs.corner(mesh, f, k).unwrap();
            let b = uvs.corner(mesh, f, (k + 1) % list.len()).unwrap();
            let d = b - a;
            d.x.abs() < 1e-4 || d.y.abs() < 1e-4
        })
    })
}

/// The edges across the middle of the 2 × 2 plane, along one direction.
fn middle_line(mesh: &scene::Mesh) -> Vec<EdgeKey> {
    let topology = mesh.topology();
    topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .filter(|e| {
            let (a, b) = e.ends();
            let (p, q) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            p.x.abs() < 1e-4 && q.x.abs() < 1e-4
        })
        .collect()
}

#[test]
fn unwrap_lays_a_turned_plane_straight_again_as_one_step() {
    let mut harness = fresh();
    let id = plane(&mut harness);
    let mesh = shape(&harness);
    let faces: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::TransformUvs {
                faces,
                points: Vec::new(),
                transform: glam::Affine2::from_angle(0.5),
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    assert!(!lined_up(&shape(&harness)));
    let before = steps(&harness);
    run(&mut harness, CommandId::Unwrap);
    assert_eq!(steps(&harness), before + 1);
    assert!(lined_up(&shape(&harness)));
    assert!(matches!(
        last_operation(&harness),
        Some(scene::Operation::Unwrap { strips: true, .. })
    ));
    assert!(note(&harness).contains("Ctrl + Z"));
}

#[test]
fn marking_seams_unwraps_the_pieces_around_them_live() {
    for live in [true, false] {
        let mut harness = fresh();
        harness.state_mut().workbench.uv_settings.live_unwrap = live;
        let id = plane(&mut harness);
        press(&mut harness, Modifiers::NONE, Key::Num2);
        let line: Vec<Element> = middle_line(&shape(&harness))
            .into_iter()
            .map(Element::Edge)
            .collect();
        assert_eq!(line.len(), 2);
        harness
            .state_mut()
            .workbench
            .document_mut()
            .select_elements([(id, ElementSelection::of(line))], "Select", None);
        harness.run();
        assert_eq!(mesh::uvs::islands(&shape(&harness)).len(), 1);
        let before = steps(&harness);
        run(&mut harness, CommandId::MarkSeam);
        assert_eq!(steps(&harness), before + 1, "one step, seams and unwrap");
        // The plane is in two pieces now; live, they were laid flat again
        // (a flat piece comes back just where it was).
        let mesh = shape(&harness);
        let seams: std::collections::BTreeSet<EdgeKey> =
            mesh::uvs::seams(&mesh).into_iter().collect();
        assert_eq!(uv::seams::pieces(&mesh, &seams).len(), 2);
        let unwrapped = matches!(
            last_operation(&harness),
            Some(scene::Operation::Unwrap { .. })
        );
        assert_eq!(unwrapped, live, "live {live}");
    }
}

#[test]
fn relax_and_straighten_work_on_the_selected_polygons() {
    let mut harness = fresh();
    let id = plane(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let first = shape(&harness).faces().next().unwrap().0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(first)]))],
            "Select",
            None,
        );
    harness.run();
    run(&mut harness, CommandId::Relax);
    match last_operation(&harness) {
        Some(scene::Operation::RelaxUvs { faces, .. }) => assert_eq!(faces, vec![first]),
        other => panic!("{other:?}"),
    }
    run(&mut harness, CommandId::Straighten);
    assert!(matches!(
        last_operation(&harness),
        Some(scene::Operation::StraightenUvs { .. })
    ));
    // A ball has triangles: Straighten says what it skips.
    let mut harness = fresh();
    add(&mut harness, scene::Primitive::Sphere(Default::default()));
    run(&mut harness, CommandId::Straighten);
    assert!(
        note(&harness).contains("Only pieces made of quads"),
        "{}",
        note(&harness)
    );
}

#[test]
fn nothing_selected_says_so() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    let before = steps(&harness);
    run(&mut harness, CommandId::Unwrap);
    assert_eq!(steps(&harness), before);
    assert!(note(&harness).contains("select the objects"));
}

#[test]
fn the_uv_menu_has_the_tools_and_remembered_switches() {
    let mut harness = fresh();
    plane(&mut harness);
    main_menu(&harness, "UV").click();
    harness.run();
    for label in [
        "Unwrap",
        "Relax",
        "Straighten",
        super::unwrapping::LIVE_UNWRAP_LABEL,
        super::unwrapping::STRAIGHT_STRIPS_LABEL,
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness
        .get_by_label(super::unwrapping::LIVE_UNWRAP_LABEL)
        .click();
    harness.run();
    let settings = harness.state().workbench.uv_settings();
    assert!(!settings.live_unwrap);
    assert!(settings.straighten_strips);
    // Remembered settings come back.
    let back = Workbench::new(super::tests::info(), None).with_uv_settings(UvSettings {
        live_unwrap: false,
        straighten_strips: false,
        ..UvSettings::default()
    });
    assert!(!back.uv_settings().straighten_strips);
}
