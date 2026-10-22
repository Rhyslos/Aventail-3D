//! Tests for pinning UVs, the unwrap methods and unfolding one way in the
//! window, and the tool menu's groups (decision 0109).

use super::modelling_tests::{H, note, press};
use super::tests::fresh;
use super::unwrapping::{UnwrapWay, UvSettings};
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, VertexId};

fn plane(harness: &mut H) -> scene::ObjectId {
    let doc = harness.state_mut().workbench.document_mut();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::Primitive::Plane(mesh::PlaneParams {
            segments: [2, 2],
            ..Default::default()
        }),
    )))
    .unwrap();
    let id = *doc.selection().first().unwrap();
    harness.run();
    id
}

fn shape(harness: &H, id: scene::ObjectId) -> std::sync::Arc<scene::Mesh> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
}

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

fn last_operation(harness: &H, id: scene::ObjectId) -> Option<scene::Operation> {
    let doc = harness.state().workbench.document();
    let object = doc.scene().object(id)?;
    object
        .stack
        .layers()
        .last()?
        .entries
        .last()
        .map(|e| e.operation.clone())
}

/// The plane in the Points mode with two corners selected.
fn two_corners(harness: &mut H) -> (scene::ObjectId, Vec<VertexId>) {
    let id = plane(harness);
    press(harness, Modifiers::NONE, Key::Num1);
    let points: Vec<VertexId> = shape(harness, id).vertices().map(|(v, _)| v).collect();
    let chosen = vec![points[0], *points.last().unwrap()];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                id,
                ElementSelection::of(chosen.iter().map(|v| Element::Point(*v))),
            )],
            "Select",
            None,
        );
    harness.run();
    (id, chosen)
}

#[test]
fn pin_and_unpin_are_one_step_each_and_say_so() {
    let mut harness = fresh();
    let (id, chosen) = two_corners(&mut harness);
    let before = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::UvPin);
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        before + 1
    );
    match last_operation(&harness, id) {
        Some(scene::Operation::SetUvPins { points, pin, .. }) => {
            assert!(pin);
            assert_eq!(points, chosen);
        }
        other => panic!("{other:?}"),
    }
    assert!(note(&harness).contains("Pin the UVs of 2 points"));
    assert!(mesh::uvs::pins(&shape(&harness, id)).is_some());
    run(&mut harness, CommandId::UvUnpin);
    assert!(mesh::uvs::pins(&shape(&harness, id)).is_none());
}

#[test]
fn pinning_needs_selected_uvs() {
    let mut harness = fresh();
    plane(&mut harness);
    // The Model mode selects objects, not UVs.
    let ready = harness.state().workbench.readiness(CommandId::UvPin);
    assert!(matches!(ready, crate::tool_menu::Ready::NotNow(_)));
    press(&mut harness, Modifiers::NONE, Key::Num1);
    run(&mut harness, CommandId::UvPin);
    assert!(
        note(&harness).contains("select some UVs"),
        "{}",
        note(&harness)
    );
}

#[test]
fn pinned_points_stay_put_when_unwrapping_every_way() {
    for way in UnwrapWay::ALL {
        let mut harness = fresh();
        let (id, chosen) = two_corners(&mut harness);
        run(&mut harness, CommandId::UvPin);
        harness.state_mut().workbench.uv_settings.unwrap_method = way;
        let spots = |harness: &H| {
            let mesh = shape(harness, id);
            let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
            chosen
                .iter()
                .map(|v| {
                    let (f, list) = mesh.faces().find(|(_, l)| l.contains(v)).unwrap();
                    let k = list.iter().position(|x| x == v).unwrap();
                    uvs.corner(&mesh, f, k).unwrap()
                })
                .collect::<Vec<_>>()
        };
        let before = spots(&harness);
        run(&mut harness, CommandId::Unwrap);
        match last_operation(&harness, id) {
            Some(scene::Operation::Unwrap { options, .. }) => {
                assert_eq!(options.method, way.method());
                assert_eq!(options.only, None);
            }
            other => panic!("{other:?}"),
        }
        for (a, b) in spots(&harness).iter().zip(&before) {
            assert!(a.distance(*b) < 1e-5, "{way:?}");
        }
    }
}

#[test]
fn the_uv_menu_offers_methods_one_way_unfolds_and_pins() {
    let mut harness = fresh();
    let id = plane(&mut harness);
    super::tests::uv_group(&mut harness, super::UV_PIECES_MENU);
    for label in ["Pin", "Unpin"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.key_press(egui::Key::Escape);
    harness.run();
    super::tests::uv_group(&mut harness, super::UV_LAY_OUT_MENU);
    for label in [
        super::unwrapping::UNFOLD_ACROSS_LABEL,
        super::unwrapping::UNFOLD_UP_LABEL,
    ] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    harness.key_press(egui::Key::Escape);
    harness.run();
    super::tests::uv_group(&mut harness, super::UV_SETTINGS_MENU);
    harness
        .get_by_label_contains(super::automation::UNWRAP_SETTINGS_LABEL)
        .hover();
    harness.run();
    for way in UnwrapWay::ALL {
        assert!(harness.query_by_label(way.name()).is_some(), "{way:?}");
    }
    harness.get_by_label(UnwrapWay::KeepSizes.name()).click();
    harness.run();
    assert_eq!(
        harness.state().workbench.uv_settings().unwrap_method,
        UnwrapWay::KeepSizes
    );
    // Unfold across only: an unwrap that keeps every point's V.
    super::tests::uv_group(&mut harness, super::UV_LAY_OUT_MENU);
    harness
        .get_by_label(super::unwrapping::UNFOLD_ACROSS_LABEL)
        .click();
    harness.run();
    assert!(matches!(
        last_operation(&harness, id),
        Some(scene::Operation::Unwrap {
            options: scene::UnwrapOptions {
                method: scene::UnwrapMethod::SizeKeeping,
                only: Some(scene::UnfoldAxis::Across),
                ..
            },
            ..
        })
    ));
    // The way is remembered, and old settings read as Keep shapes.
    let read: UvSettings = ron::from_str("(live_unwrap: false)").unwrap();
    assert_eq!(read.unwrap_method, UnwrapWay::KeepShapes);
    let back = Workbench::new(super::tests::info(), None).with_uv_settings(UvSettings {
        unwrap_method: UnwrapWay::Automatic,
        ..UvSettings::default()
    });
    assert_eq!(back.uv_settings().unwrap_method, UnwrapWay::Automatic);
    for way in UnwrapWay::ALL {
        assert!(!way.name().is_empty() && !way.help().is_empty());
    }
}

#[test]
fn choosing_a_group_opens_its_tools_in_the_same_place() {
    let mut harness = fresh();
    harness.state_mut().workbench.mode = Mode::Uv;
    let middle = egui::pos2(720.0, 450.0);
    harness.hover_at(middle);
    harness.run();
    press(&mut harness, Modifiers::NONE, crate::TOOL_MENU_KEY);
    assert!(harness.state().workbench.is_tool_menu_open());
    harness.get_by_label("Tool menu: Turn & flip").click();
    harness.run();
    let bench = &harness.state().workbench;
    assert!(bench.is_tool_menu_open(), "the group opens");
    let commands: Vec<CommandId> = bench
        .tool_menu_entries()
        .iter()
        .map(|e| e.slot.command)
        .collect();
    assert_eq!(commands, CommandId::UvTurnFlip.members());
    assert!(
        harness
            .query_by_label("Tool menu: Flip upside down")
            .is_some()
    );
    assert!(harness.query_by_label("Tool menu: Pack").is_none());
    // Esc closes it; E opens the whole ring again.
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!harness.state().workbench.is_tool_menu_open());
    press(&mut harness, Modifiers::NONE, crate::TOOL_MENU_KEY);
    let commands: Vec<CommandId> = harness
        .state()
        .workbench
        .tool_menu_entries()
        .iter()
        .map(|e| e.slot.command)
        .collect();
    assert_eq!(
        commands,
        crate::commands::UV_RING
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_quad_menu_lists_a_groups_tools_in_its_place() {
    let mut harness = fresh();
    harness.state_mut().workbench.mode = Mode::Uv;
    harness.run();
    let ctx = harness.ctx.clone();
    let names: Vec<String> = harness
        .state()
        .workbench
        .quadrants(&ctx)
        .iter()
        .flat_map(|q| q.items.iter().map(|i| i.label.clone()))
        .collect();
    let has = |name: &str| names.iter().any(|n| n == name);
    assert!(has("Turn left 90°"), "{names:?}");
    assert!(!has("Stitch"), "nothing not built yet: {names:?}");
    assert!(!has("Turn & flip"), "{names:?}");
}
