//! Tests for the modular kit helpers in the window (decision 0160): the
//! pivot presets, Size to kit grid, Kit-sized shapes, the Create menu's
//! Modular kit list and snapping to the kit grid.

use super::modelling_tests::{H, cube, note};
use super::tests::{fresh, main_menu};
use super::*;
use crate::commands::{CommandId, group_menu, menu_for};
use crate::shape_picker::Shape;
use crate::transforming::{SnapSettings, move_step};
use crate::viewport::GridDetail;
use egui_kittest::kittest::{NodeT, Queryable};
use glam::{Quat, Vec3};
use mesh::{CubeParams, CylinderParams};
use scene::{KitGrid, Primitive, PrimitiveKind, Transform};

fn run(harness: &mut H, command: CommandId) {
    let ctx = harness.ctx.clone();
    let mut requests = Vec::new();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut requests);
    harness.state_mut().workbench.carry_out(requests);
    harness.run();
}

fn execute(harness: &mut H, command: scene::Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run();
}

/// The fresh window's cube, selected, resized to `size` and placed at
/// `at`, turned `turn` round Z.
fn sized_cube(size: Vec3, at: Vec3, turn: f32) -> (H, scene::ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    execute(
        &mut harness,
        scene::Command::SetPrimitive {
            ids: vec![id],
            primitive: Primitive::Cube(CubeParams {
                size,
                ..CubeParams::default()
            }),
        },
    );
    execute(
        &mut harness,
        scene::Command::SetTransforms(vec![(
            id,
            Transform {
                position: at,
                rotation: Vec3::new(turn, 0.0, 0.0),
                ..Transform::default()
            },
        )]),
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    (harness, id)
}

/// Opens the submenu `label` of the menu that is open.
fn open_submenu(harness: &mut H, label: &str) {
    harness
        .get_all_by_label_contains(label)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .unwrap_or_else(|| panic!("no submenu {label}"))
        .hover();
    harness.run_steps(8);
}

fn pivot_of(harness: &H, id: scene::ObjectId) -> Vec3 {
    let scene = harness.state().workbench.document().scene();
    scene.world_matrix(id).w_axis.truncate()
}

fn bounds_of(harness: &H, id: scene::ObjectId) -> scene::Bounds {
    let scene = harness.state().workbench.document().scene();
    scene.bounds_of([id]).unwrap()
}

fn kit_grid(harness: &H) -> KitGrid {
    harness.state().workbench.document().scene().kit_grid()
}

fn set_kit_grid(harness: &mut H, step: f32, sized_shapes: bool) {
    execute(
        harness,
        scene::Command::SetKitGrid(KitGrid { step, sized_shapes }),
    );
}

fn primitive_of(harness: &H, id: scene::ObjectId) -> Primitive {
    let scene = harness.state().workbench.document().scene();
    match &scene.object(id).unwrap().kind {
        scene::ObjectKind::Primitive(p) => *p,
        other => panic!("not a primitive: {other:?}"),
    }
}

#[test]
fn the_pivot_presets_move_the_pivot_onto_the_shapes_box_and_the_shape_stays() {
    let size = Vec3::new(200.0, 100.0, 300.0);
    let at = Vec3::new(50.0, -20.0, 10.0);
    for (command, expected, label) in [
        (
            CommandId::PivotToTopCentre,
            at + Vec3::new(0.0, 0.0, 300.0),
            "Pivot to top centre",
        ),
        (
            CommandId::PivotToBottomCorner,
            at + Vec3::new(-100.0, -50.0, 0.0),
            "Pivot to bottom corner",
        ),
        (
            CommandId::PivotToCentre,
            at + Vec3::new(0.0, 0.0, 150.0),
            "Pivot to centre",
        ),
    ] {
        let (mut harness, id) = sized_cube(size, at, 0.0);
        let before = bounds_of(&harness, id);
        run(&mut harness, command);
        assert!(
            pivot_of(&harness, id).abs_diff_eq(expected, 1e-3),
            "{command:?}: {}",
            pivot_of(&harness, id)
        );
        let after = bounds_of(&harness, id);
        assert!(
            after.min.abs_diff_eq(before.min, 1e-3),
            "{command:?}: the shape stays"
        );
        assert!(after.max.abs_diff_eq(before.max, 1e-3), "{command:?}");
        assert_eq!(
            harness.state().workbench.document().undo_label(),
            Some(label)
        );
        // Bottom centre puts it back where a new shape has it.
        run(&mut harness, CommandId::PivotToBottomCentre);
        assert!(pivot_of(&harness, id).abs_diff_eq(at, 1e-3), "{command:?}");
        // Undo takes each back.
        harness.state_mut().workbench.document_mut().undo();
        harness.state_mut().workbench.document_mut().undo();
        harness.run();
        assert!(pivot_of(&harness, id).abs_diff_eq(at, 1e-3), "{command:?}");
    }
}

#[test]
fn a_new_shape_already_has_its_pivot_at_its_bottom_centre() {
    let (mut harness, id) = sized_cube(Vec3::splat(100.0), Vec3::ZERO, 0.0);
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::PivotToBottomCentre);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(
        note(&harness).contains("already there"),
        "{}",
        note(&harness)
    );
    assert!(pivot_of(&harness, id).abs_diff_eq(Vec3::ZERO, 1e-3));
}

#[test]
fn the_bottom_corner_follows_a_turned_objects_own_axes() {
    // Turned a quarter: the shape's own lowest X and Y corner is now at
    // the world's +X, -Y side.
    let turn = std::f32::consts::FRAC_PI_2;
    let (mut harness, id) = sized_cube(Vec3::new(200.0, 100.0, 50.0), Vec3::ZERO, turn);
    run(&mut harness, CommandId::PivotToBottomCorner);
    let expected = Quat::from_rotation_z(turn) * Vec3::new(-100.0, -50.0, 0.0);
    assert!(
        pivot_of(&harness, id).abs_diff_eq(expected, 1e-2),
        "{} against {expected}",
        pivot_of(&harness, id)
    );
}

#[test]
fn the_pivot_presets_need_a_selection_and_work_on_groups() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    for command in CommandId::PivotPresets.members() {
        run(&mut harness, *command);
        assert!(
            note(&harness).starts_with("Nothing is selected"),
            "{command:?}: {}",
            note(&harness)
        );
    }
    // A group's presets use the box round everything in it.
    let id = cube(&harness);
    execute(&mut harness, scene::Command::Group(vec![id]));
    let group = harness.state().workbench.document().selection()[0];
    let around = bounds_of(&harness, group);
    run(&mut harness, CommandId::PivotToTopCentre);
    let expected = Vec3::new(around.centre().x, around.centre().y, around.max.z);
    assert!(pivot_of(&harness, group).abs_diff_eq(expected, 1e-3));
}

#[test]
fn the_edit_pivot_ring_holds_the_presets_as_a_group() {
    let ring = menu_for(Mode::Object);
    let presets = ring
        .iter()
        .find(|s| s.command == CommandId::PivotPresets)
        .expect("the presets are on the ring");
    let inside = group_menu(CommandId::PivotPresets, &ring);
    assert_eq!(inside.len(), 4);
    assert!(inside.iter().any(|s| s.place == presets.place));
    for slot in &inside {
        assert!(slot.command.modes().contains(&Mode::Object));
    }
    // The Model mode's ring is as it was.
    assert!(
        !menu_for(Mode::Model)
            .iter()
            .any(|s| s.command == CommandId::PivotPresets)
    );
    // Choosing the group in the ring opens it.
    let mut harness = fresh();
    harness.state_mut().workbench.mode = Mode::Object;
    run(&mut harness, CommandId::PivotPresets);
    assert_eq!(
        harness.state().workbench.tool_menu.group(),
        Some(CommandId::PivotPresets)
    );
}

#[test]
fn the_tools_menu_lists_the_presets_in_a_list_of_their_own() {
    let (mut harness, id) = sized_cube(Vec3::splat(100.0), Vec3::ZERO, 0.0);
    main_menu(&harness, "Tools").click();
    harness.run();
    open_submenu(&mut harness, "Pivot presets");
    harness.get_by_label("Pivot to top centre").click();
    harness.run();
    assert!(pivot_of(&harness, id).abs_diff_eq(Vec3::new(0.0, 0.0, 100.0), 1e-3));
}

#[test]
fn size_to_kit_grid_rounds_the_selected_shapes_in_one_undo_step() {
    let (mut harness, id) = sized_cube(Vec3::new(130.0, 20.0, 260.0), Vec3::ZERO, 0.0);
    // A second shape, a cylinder 1.6 m across.
    execute(
        &mut harness,
        scene::Command::Create(scene::ObjectKind::Primitive(Primitive::Cylinder(
            CylinderParams {
                radius: 80.0,
                ..Default::default()
            },
        ))),
    );
    let cylinder = harness.state().workbench.document().selection()[0];
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id, cylinder]);
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::SizeToKitGrid);
    let Primitive::Cube(c) = primitive_of(&harness, id) else {
        panic!("a cube")
    };
    assert_eq!(c.size, Vec3::new(100.0, 20.0, 300.0), "the thin side stays");
    let Primitive::Cylinder(r) = primitive_of(&harness, cylinder) else {
        panic!("a cylinder")
    };
    assert_eq!(r.radius, 100.0);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "one undo step");
    assert_eq!(doc.undo_label(), Some("Size to kit grid"));
    assert!(note(&harness).starts_with("2 shapes sized to the 1 m kit grid"));
    // Again: they fit already.
    run(&mut harness, CommandId::SizeToKitGrid);
    assert!(note(&harness).contains("already fit"), "{}", note(&harness));
    // With a 4 m grid, the 3 m height becomes 4 m.
    set_kit_grid(&mut harness, 400.0, false);
    run(&mut harness, CommandId::SizeToKitGrid);
    let Primitive::Cube(c) = primitive_of(&harness, id) else {
        panic!("a cube")
    };
    assert_eq!(c.size.z, 400.0);
    // Undo puts the sizes back.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let Primitive::Cube(c) = primitive_of(&harness, id) else {
        panic!("a cube")
    };
    assert_eq!(c.size.z, 300.0);
}

#[test]
fn size_to_kit_grid_explains_why_it_cant_be_used() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .clear_selection();
    harness.run();
    run(&mut harness, CommandId::SizeToKitGrid);
    assert!(note(&harness).contains("Select one or more shapes"));
    // A baked mesh has no sizes to round.
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    execute(&mut harness, scene::Command::MakeEditable(vec![id]));
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::SizeToKitGrid);
    assert!(
        note(&harness).contains("Only shapes with settings"),
        "{}",
        note(&harness)
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn kit_sized_shapes_makes_new_shapes_at_the_kit_grids_size() {
    let mut harness = fresh();
    let create = |harness: &mut H, kind| {
        let mut requests = Vec::new();
        harness
            .state_mut()
            .workbench
            .create_shape(Shape::Primitive(kind), &mut requests);
        harness.state_mut().workbench.carry_out(requests);
        harness.run();
        let id = harness.state().workbench.document().selection()[0];
        primitive_of(harness, id)
    };
    // Off: the standard 1 m cube, even with a 2 m grid.
    set_kit_grid(&mut harness, 200.0, false);
    assert_eq!(
        create(&mut harness, PrimitiveKind::Cube),
        PrimitiveKind::Cube.standard()
    );
    // On: 2 m.
    set_kit_grid(&mut harness, 200.0, true);
    let Primitive::Cube(c) = create(&mut harness, PrimitiveKind::Cube) else {
        panic!("a cube")
    };
    assert_eq!(c.size, Vec3::splat(200.0));
    // A gear grows to 2 m across, and stays a thin slab.
    let Primitive::Gear(g) = create(&mut harness, PrimitiveKind::Gear) else {
        panic!("a gear")
    };
    assert_eq!(g.radius, 100.0);
    assert!(g.height < 100.0);
    // The palette's shape buttons follow it too.
    let Primitive::Cube(c) = (match harness.state().workbench.new_primitive(PrimitiveKind::Cube) {
        crate::objects::DocRequest::Run(scene::Command::Create(scene::ObjectKind::Primitive(
            p,
        ))) => p,
        _ => panic!("a new cube"),
    }) else {
        panic!("a cube")
    };
    assert_eq!(c.size, Vec3::splat(200.0));
}

#[test]
fn the_modular_kit_list_sets_the_grid_and_the_switches() {
    let mut harness = fresh();
    main_menu(&harness, "Create").click();
    harness.run();
    open_submenu(&mut harness, "Modular kit");
    harness.get_by_label("2 m").click();
    harness.run();
    assert_eq!(kit_grid(&harness).step, 200.0);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Kit grid 2 m")
    );
    harness.get_by_label("Kit-sized shapes").click();
    harness.run();
    assert!(kit_grid(&harness).sized_shapes);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Kit-sized shapes on")
    );
    assert!(!harness.state().workbench.view_options().snap.kit_grid);
    harness.get_by_label("Snap moves to the kit grid").click();
    harness.run();
    assert!(harness.state().workbench.view_options().snap.kit_grid);
    // Undo takes the switch and the grid back in turn.
    harness.state_mut().workbench.document_mut().undo();
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    assert_eq!(kit_grid(&harness), KitGrid::default());
}

#[test]
fn any_other_kit_grid_can_be_typed() {
    let mut harness = fresh();
    main_menu(&harness, "Create").click();
    harness.run();
    open_submenu(&mut harness, "Modular kit");
    harness
        .get_all_by_label("Kit grid")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::SpinButton)
        .expect("the kit grid's field")
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.event(egui::Event::Text("3".into()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(kit_grid(&harness).step, 300.0);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Kit grid 3 m")
    );
}

#[test]
fn moves_snap_to_the_kit_grid_when_asked() {
    let kit = KitGrid {
        step: 400.0,
        ..KitGrid::default()
    };
    let mut snap = SnapSettings::default();
    assert_eq!(move_step(&snap, GridDetail::Metre, kit), 100.0);
    assert_eq!(move_step(&snap, GridDetail::TenCentimetres, kit), 10.0);
    snap.kit_grid = true;
    assert_eq!(move_step(&snap, GridDetail::TenCentimetres, kit), 400.0);
    // A grid that isn't one falls back to the lines shown.
    let broken = KitGrid {
        step: f32::NAN,
        ..kit
    };
    assert_eq!(move_step(&snap, GridDetail::Metre, broken), 100.0);
}

#[test]
fn command_search_finds_the_kit_helpers() {
    let harness = fresh();
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    for name in [
        "Size to kit grid",
        "Kit-sized shapes",
        "Snap moves to the kit grid",
        "Pivot to bottom corner",
    ] {
        assert!(entries.iter().any(|e| e.name == name), "{name}");
    }
    let size = entries
        .iter()
        .find(|e| e.name == "Size to kit grid")
        .unwrap();
    assert_eq!(size.place, "Create");
    let corner = entries
        .iter()
        .find(|e| e.name == "Pivot to bottom corner")
        .unwrap();
    assert_eq!(corner.place, "Tools");
}

#[test]
fn size_to_kit_grid_fits_an_untouched_lattice_cage_again() {
    // A tester sized a shape with a lattice and its cage stayed small.
    let (mut harness, id) = sized_cube(Vec3::new(130.0, 130.0, 130.0), Vec3::ZERO, 0.0);
    execute(
        &mut harness,
        scene::Command::AddLattice {
            ids: vec![id],
            divisions: [2, 2, 2],
        },
    );
    let cage = harness.state().workbench.document().selection()[0];
    let size_of = |harness: &H| match &harness
        .state()
        .workbench
        .document()
        .scene()
        .object(cage)
        .unwrap()
        .kind
    {
        scene::ObjectKind::Lattice(l) => l.size(),
        other => panic!("not a cage: {other:?}"),
    };
    assert!(size_of(&harness).abs_diff_eq(Vec3::splat(130.0), 0.01));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, CommandId::SizeToKitGrid);
    assert!(
        size_of(&harness).abs_diff_eq(Vec3::splat(100.0), 0.01),
        "fitted again"
    );
    assert_eq!(
        harness.state().workbench.document().undo_steps(),
        steps + 1,
        "in the same undo step"
    );
    // A cage whose points were moved stays as the person left it.
    execute(
        &mut harness,
        scene::Command::EditLattice {
            id: cage,
            edit: mesh::lattice::LatticeEdit::Transform {
                points: vec![0],
                matrix: glam::Mat4::from_translation(Vec3::new(0.0, 0.0, -20.0)),
            },
        },
    );
    execute(
        &mut harness,
        scene::Command::SetPrimitive {
            ids: vec![id],
            primitive: Primitive::Cube(CubeParams {
                size: Vec3::splat(260.0),
                ..CubeParams::default()
            }),
        },
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    run(&mut harness, CommandId::SizeToKitGrid);
    let Primitive::Cube(c) = primitive_of(&harness, id) else {
        panic!("a cube: {}", note(&harness))
    };
    assert_eq!(c.size, Vec3::splat(300.0), "{}", note(&harness));
    assert!(
        size_of(&harness).abs_diff_eq(Vec3::splat(100.0), 0.01),
        "left alone"
    );
}
