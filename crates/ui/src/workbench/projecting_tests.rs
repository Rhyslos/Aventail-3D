//! Tests for projections in the window (decision 0235): placing one round
//! the selection, the gizmo moving it live in one undo step, Enter, Esc,
//! Done and Cancel, from the view, world-aligned, and the menus and ring.

use super::modelling_tests::{H, note, press};
use super::seams_tests::{add, shape, steps};
use super::tests::{fresh, uv_group};
use super::*;
use crate::components::ViewPointer;
use crate::viewport::GizmoEvent;
use egui_kittest::kittest::Queryable;
use scene::{Operation, Projection};
use tools::{Change, Handle};

fn cube(harness: &mut H) -> scene::ObjectId {
    add(harness, scene::Primitive::Cube(Default::default()))
}

/// Every corner's UV of the selected object's shape.
fn uvs(harness: &H) -> Vec<glam::Vec2> {
    let mesh = shape(harness);
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    (0..mesh.face_layout().1.len()).map(|i| uvs.at(i)).collect()
}

/// The last recorded edit of object `id`.
fn last(harness: &H, id: scene::ObjectId) -> Option<Operation> {
    let object = harness.state().workbench.document().scene().object(id)?;
    object
        .stack
        .layers()
        .iter()
        .flat_map(|l| l.entries.iter())
        .last()
        .map(|e| e.operation.clone())
}

fn pointer() -> ViewPointer {
    ViewPointer {
        camera: tools::Camera::default(),
        size: glam::Vec2::new(400.0, 300.0),
        at: glam::Vec2::ZERO,
    }
}

fn drag(harness: &mut H, change: Change) {
    let handle = Handle::Centre;
    let events = [
        GizmoEvent::Begin {
            handle,
            slide: false,
        },
        GizmoEvent::Update {
            change,
            handle,
            pointer: pointer(),
        },
        GizmoEvent::End,
    ];
    assert!(
        harness
            .state_mut()
            .workbench
            .projection_gizmo_events(&events)
    );
    harness.run();
}

#[test]
fn a_box_projection_is_placed_round_the_selection_and_follows_the_gizmo() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    let before = steps(&harness);
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Box, false);
    harness.run();
    let w = &harness.state().workbench;
    assert!(w.projecting());
    assert!(w.projection_gizmo().is_some(), "the gizmo sits on it");
    assert!(w.tool().gizmo().is_some(), "a gizmo tool is chosen");
    assert!(matches!(
        last(&harness, id),
        Some(Operation::ProjectUvs { .. })
    ));
    assert_eq!(steps(&harness), before + 1);
    assert!(note(&harness).contains("Enter"), "{}", note(&harness));
    let placed = uvs(&harness);
    // Every corner lands in the square.
    for uv in &placed {
        assert!(
            (-1e-4..=1.0001).contains(&uv.x) && (-1e-4..=1.0001).contains(&uv.y),
            "{uv}"
        );
    }
    // Moving the projection moves the texture, in the same undo step.
    drag(
        &mut harness,
        Change::Translate(glam::Vec3::new(10.0, 0.0, 0.0)),
    );
    let moved = uvs(&harness);
    assert_ne!(moved, placed);
    // Sizing it up makes the texture smaller on the model.
    drag(&mut harness, Change::Scale(glam::Vec3::splat(2.0)));
    let bigger = uvs(&harness);
    assert_ne!(bigger, moved);
    // And turning it.
    drag(
        &mut harness,
        Change::Rotate {
            axis: glam::Vec3::Z,
            angle: 0.5,
        },
    );
    assert_eq!(steps(&harness), before + 1, "one step for all of it");
    // Enter keeps it.
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!(!harness.state().workbench.projecting());
    assert_eq!(steps(&harness), before + 1);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(!matches!(
        last(&harness, id),
        Some(Operation::ProjectUvs { .. })
    ));
}

#[test]
fn esc_takes_the_projection_back_and_a_cancelled_drag_goes_back() {
    let mut harness = fresh();
    cube(&mut harness);
    let before = steps(&harness);
    let original = uvs(&harness);
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Cylinder, false);
    harness.run();
    let placed = uvs(&harness);
    // A drag that's cancelled puts the projection back where it was.
    let handle = Handle::Centre;
    harness.state_mut().workbench.projection_gizmo_events(&[
        GizmoEvent::Begin {
            handle,
            slide: false,
        },
        GizmoEvent::Update {
            change: Change::Translate(glam::Vec3::splat(5.0)),
            handle,
            pointer: pointer(),
        },
        GizmoEvent::Cancel,
    ]);
    harness.run();
    assert_eq!(uvs(&harness), placed);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    let w = &harness.state().workbench;
    assert!(!w.projecting());
    assert_eq!(steps(&harness), before, "nothing kept");
    assert_eq!(uvs(&harness), original);
    // Esc only cancelled: the object is still selected.
    assert!(!harness.state().workbench.document().selection().is_empty());
}

#[test]
fn done_and_cancel_buttons_show_while_projecting() {
    let mut harness = fresh();
    cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Planar, false);
    harness.run();
    harness.get_by_label(projecting::DONE_LABEL).click();
    harness.run();
    assert!(!harness.state().workbench.projecting());
    assert!(harness.query_by_label(projecting::DONE_LABEL).is_none());
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Sphere, false);
    harness.run();
    let before = steps(&harness);
    harness.get_by_label(projecting::CANCEL_LABEL).click();
    harness.run();
    assert!(!harness.state().workbench.projecting());
    assert_eq!(steps(&harness), before - 1);
}

#[test]
fn undoing_while_projecting_stops_placing() {
    let mut harness = fresh();
    cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Box, false);
    harness.run();
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert!(!harness.state().workbench.projecting());
}

#[test]
fn projecting_from_the_view_looks_the_way_the_camera_does() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Planar, true);
    harness.run();
    let Some(Operation::ProjectUvs { options, .. }) = last(&harness, id) else {
        panic!("a projection");
    };
    // The projection's depth axis points back at the camera.
    let camera = *harness.state().workbench.viewport.camera();
    let back = options
        .placement
        .inverse()
        .transform_vector3(glam::Vec3::Z)
        .normalize();
    assert!(back.dot(-camera.forward()) > 0.99, "{back}");
}

#[test]
fn world_aligned_uvs_use_the_projects_density_in_one_step() {
    let mut harness = fresh();
    let id = cube(&mut harness);
    let before = steps(&harness);
    harness.state_mut().workbench.run_world_aligned();
    harness.run();
    let Some(Operation::ProjectUvs { options, .. }) = last(&harness, id) else {
        panic!("a projection");
    };
    let want = harness
        .state()
        .workbench
        .document()
        .scene()
        .texture()
        .uv_per_unit();
    assert_eq!(options.world, Some(want));
    assert_eq!(options.kind, Projection::Box);
    assert_eq!(steps(&harness), before + 1);
    assert!(!harness.state().workbench.projecting(), "recorded at once");
    assert!(note(&harness).contains("px/m"), "{}", note(&harness));
}

#[test]
fn projecting_needs_something_selected() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness
        .state_mut()
        .workbench
        .start_projection(Projection::Box, false);
    assert!(!harness.state().workbench.projecting());
    assert!(note(&harness).contains("select"), "{}", note(&harness));
    harness.state_mut().workbench.run_world_aligned();
    assert!(note(&harness).contains("select"), "{}", note(&harness));
}

#[test]
fn the_uv_menu_offers_every_projection() {
    let mut harness = fresh();
    cube(&mut harness);
    uv_group(&mut harness, projecting::PROJECT_LABEL);
    for label in [
        projecting::PLANAR_LABEL,
        projecting::BOX_LABEL,
        projecting::CYLINDER_LABEL,
        projecting::SPHERE_LABEL,
        projecting::FROM_VIEW_LABEL,
        projecting::WORLD_LABEL,
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.get_by_label(projecting::CYLINDER_LABEL).click();
    harness.run();
    assert!(harness.state().workbench.projecting());
}

#[test]
fn the_rings_unwrap_group_holds_unwrap_the_unfolds_and_project() {
    use crate::commands::CommandId;
    assert_eq!(
        CommandId::UvUnwrapGroup.members(),
        &[
            CommandId::Unwrap,
            CommandId::UvUnfoldAcross,
            CommandId::UvUnfoldUp,
            CommandId::UvProject
        ]
    );
    let mut harness = fresh();
    cube(&mut harness);
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(CommandId::UvProject, &ctx, &mut Vec::new());
    harness.run();
    assert!(harness.state().workbench.projecting());
}
