//! Tests for the bake check in the window (decision 0077): File → Check
//! for baking…, its list, Select and Fix.

use super::modelling_tests::{H, cube, last_operation, note, shape};
use super::tests::{fresh, main_menu};
use egui_kittest::kittest::Queryable;
use scene::Operation;

fn open_check(harness: &mut H) {
    main_menu(harness, "File").click();
    harness.run();
    harness.get_by_label("Check for baking…").click();
    harness.run();
}

/// The fresh window's cube with its top flipped (recorded).
fn with_a_flipped_top() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    let top = shape(&harness)
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| {
            let mesh = shape(&harness);
            mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z)
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: Operation::FlipPolygons {
                faces: vec![top],
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    harness
}

#[test]
fn a_clean_scene_says_so() {
    let mut harness = fresh();
    open_check(&mut harness);
    assert!(
        harness
            .query_by_label("Nothing found that would spoil a bake.")
            .is_some()
    );
}

#[test]
fn a_flipped_polygon_is_listed_selected_and_fixed() {
    let mut harness = with_a_flipped_top();
    open_check(&mut harness);
    assert!(
        harness
            .query_by_label("Cube: 1 polygon facing the wrong way")
            .is_some()
    );
    harness
        .get_by_label("Select Cube: 1 polygon facing the wrong way")
        .click();
    harness.run();
    let workbench = &harness.state().workbench;
    let id = cube(&harness);
    assert_eq!(workbench.mode, crate::palette::Mode::Polygons);
    assert_eq!(workbench.document().selection(), vec![id]);
    assert_eq!(
        workbench
            .document()
            .elements(id)
            .unwrap()
            .polygons()
            .count(),
        1
    );
    harness
        .get_by_label("Fix Cube: 1 polygon facing the wrong way")
        .click();
    harness.run();
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::FixFacing)
    ));
    assert!(note(&harness).contains("Ctrl + Z"));
    // Checked again at once: nothing left.
    assert!(
        harness
            .query_by_label("Nothing found that would spoil a bake.")
            .is_some()
    );
}

#[test]
fn unpaired_low_and_high_objects_are_listed() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Rename {
            id,
            name: "crate_low".into(),
        })
        .unwrap();
    open_check(&mut harness);
    assert!(
        harness
            .query_by_label("crate_low: no partner called crate_high")
            .is_some()
    );
    // No Fix for it, but Select picks the object.
    assert!(
        harness
            .query_by_label("Fix crate_low: no partner called crate_high")
            .is_none()
    );
    harness
        .get_by_label("Select crate_low: no partner called crate_high")
        .click();
    harness.run();
    assert_eq!(harness.state().workbench.mode, crate::palette::Mode::Model);
}

#[test]
fn the_substance_painter_preset_offers_the_check() {
    let mut harness = fresh();
    main_menu(&harness, "File").click();
    harness.run();
    harness.get_by_label("Export…").click();
    harness.run();
    assert!(harness.query_by_label("Check for baking…").is_none());
    harness.get_by_label_contains("Made for:").click();
    harness.run();
    harness.get_by_label("Substance Painter").click();
    harness.run();
    harness.get_by_label("Check for baking…").click();
    harness.run();
    assert!(harness.state().workbench.baking.open);
}

#[test]
fn points_on_the_same_spot_are_welded_by_fix() {
    let mut harness = fresh();
    // Three triangles, one corner doubled on the same spot.
    let text = "o Patch\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 1 1 0\nv 5 5 5\n\
                f 1 2 3\nf 1 5 4\nf 3 5 4\n";
    let import = fileio::obj::read_obj(
        text,
        fileio::export::FileUnits::Metres,
        fileio::export::UpAxis::Y,
    )
    .unwrap();
    harness
        .state_mut()
        .workbench
        .imported(std::path::Path::new("/in/patch.obj"), import);
    harness.run();
    let problems = harness.state_mut().workbench.bake_problems();
    let texts: Vec<String> = problems.iter().map(|p| p.kind.text()).collect();
    assert!(
        texts.contains(&"1 edge with no length".to_string()),
        "{texts:?}"
    );
    for problem in problems
        .iter()
        .filter(|p| p.kind.text().contains("no length"))
    {
        harness.state_mut().workbench.fix_problem(problem);
    }
    harness.run();
    let left: Vec<String> = harness
        .state_mut()
        .workbench
        .bake_problems()
        .iter()
        .map(|p| p.kind.text())
        .collect();
    assert!(!left.iter().any(|t| t.contains("no length")), "{left:?}");
}

#[test]
fn a_fix_that_makes_something_else_worse_says_so() {
    let mut harness = fresh();
    // The saved sample's cube has points on the same spot, among its
    // many edits, that weld into edges shared by three polygons.
    let sample = include_bytes!("../../../fileio/tests/data/sample-v19.avtl");
    let document = fileio::project::from_bytes(sample).unwrap();
    harness
        .state_mut()
        .workbench
        .opened(document, "/w/crate.avtl".into());
    harness.run();
    let problems = harness.state_mut().workbench.bake_problems();
    let weld = problems
        .iter()
        .find(|p| p.name == "Cube" && p.kind.text().contains("no length"))
        .unwrap()
        .clone();
    harness.state_mut().workbench.fix_problem(&weld);
    let text = note(&harness);
    assert!(
        text.contains("but now:") && text.contains("shared by three or more polygons"),
        "{text}"
    );
}

/// Records `operation` on `id`.
fn record(harness: &mut H, id: scene::ObjectId, operation: Operation) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record { id, operation })
        .unwrap();
    harness.run();
}

/// Moves (`by`) or otherwise changes the UVs of `faces` of `id`.
fn change_uvs(harness: &mut H, id: scene::ObjectId, faces: Vec<scene::FaceId>, t: glam::Affine2) {
    record(
        harness,
        id,
        Operation::TransformUvs {
            faces,
            points: Vec::new(),
            transform: t,
            anchors: None,
        },
    );
}

fn texts(harness: &mut H) -> Vec<String> {
    harness
        .state_mut()
        .workbench
        .bake_problems()
        .iter()
        .map(|p| format!("{}: {}", p.name, p.kind.text()))
        .collect()
}

/// Fixes the problems whose text contains `what`.
fn fix(harness: &mut H, what: &str) {
    let problems = harness.state_mut().workbench.bake_problems();
    let found: Vec<_> = problems
        .iter()
        .filter(|p| p.kind.text().contains(what))
        .cloned()
        .collect();
    assert!(!found.is_empty(), "nothing with {what}");
    for problem in &found {
        harness.state_mut().workbench.fix_problem(problem);
    }
    harness.run();
}

#[test]
fn uvs_outside_the_square_are_listed_and_packed_back_in() {
    let mut harness = fresh();
    let id = cube(&harness);
    let all: Vec<scene::FaceId> = shape(&harness).faces().map(|(f, _)| f).collect();
    change_uvs(
        &mut harness,
        id,
        all,
        glam::Affine2::from_translation(glam::Vec2::new(0.9, 0.0)),
    );
    open_check(&mut harness);
    let found = texts(&mut harness);
    assert!(
        found
            .iter()
            .any(|t| t.contains("outside the texture square")),
        "{found:?}"
    );
    harness.get_by_label_contains("Fix Cube: ").click();
    harness.run();
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::PackUvs { .. })
    ));
    assert!(
        harness
            .query_by_label("Nothing found that would spoil a bake.")
            .is_some(),
        "{:?}",
        texts(&mut harness)
    );
}

#[test]
fn objects_sharing_a_texture_are_checked_together() {
    let mut harness = fresh();
    // A second cube with the same material: its pieces lie exactly on the
    // first's, stacked.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
    harness.run();
    let found = texts(&mut harness);
    assert_eq!(
        found
            .iter()
            .filter(|t| t.contains("stacked on identical pieces"))
            .count(),
        2,
        "{found:?}"
    );
    // Packing packs them apart; with stacking on in the Packing settings,
    // it's on purpose, and has no Fix.
    let stacked = |harness: &mut H| {
        harness
            .state_mut()
            .workbench
            .bake_problems()
            .into_iter()
            .find(|p| p.kind.text().contains("stacked"))
            .unwrap()
    };
    assert!(stacked(&mut harness).kind.fix().is_some());
    harness.state_mut().workbench.uv_settings.stack = true;
    assert!(stacked(&mut harness).kind.fix().is_none());
    harness.state_mut().workbench.uv_settings.stack = false;
    // Nudged a little, the second cube's pieces overlap the first's: Fix
    // packs both into one texture.
    let second = harness.state().workbench.document().selection()[0];
    let all: Vec<scene::FaceId> = shape(&harness).faces().map(|(f, _)| f).collect();
    change_uvs(
        &mut harness,
        second,
        all,
        glam::Affine2::from_translation(glam::Vec2::new(0.05, 0.0)),
    );
    let found = texts(&mut harness);
    assert!(
        found.iter().any(|t| t.contains("lying over other UVs")),
        "{found:?}"
    );
    fix(&mut harness, "lying over");
    let found = texts(&mut harness);
    assert!(found.is_empty(), "{found:?}");
    // With a texture for each object, they don't share.
    harness.state_mut().workbench.uv_settings.per_object = true;
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let found = texts(&mut harness);
    assert!(!found.iter().any(|t| t.contains("lying over")), "{found:?}");
}

#[test]
fn a_hard_edge_inside_a_piece_becomes_a_seam() {
    let mut harness = fresh();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::Primitive::Plane(mesh::PlaneParams {
                segments: [2, 1],
                ..Default::default()
            }),
        )))
        .unwrap();
    harness.run();
    harness.state_mut().workbench.uv_settings.per_object = true;
    let id = harness.state().workbench.document().selection()[0];
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh;
    // One half folded up, so the edge between the halves is sharp
    // (Automatic hard edges goes by angle).
    let right: Vec<scene::VertexId> = mesh
        .vertices()
        .filter(|(_, p)| p.x > 1.0)
        .map(|(v, _)| v)
        .collect();
    record(
        &mut harness,
        id,
        Operation::MovePoints {
            points: right,
            offset: glam::Vec3::Z * 100.0,
            anchors: None,
        },
    );
    let topology = mesh.topology();
    let inner: Vec<scene::EdgeKey> = topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect();
    record(
        &mut harness,
        id,
        Operation::SetHardEdges {
            edges: inner,
            hard: true,
            anchors: None,
        },
    );
    let found = texts(&mut harness);
    assert!(
        found.contains(&"Plane: 1 hard edge that isn't UV seams".to_string()),
        "{found:?}"
    );
    fix(&mut harness, "hard edge");
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Unwrap { .. })
    ));
    let found = texts(&mut harness);
    assert!(!found.iter().any(|t| t.contains("hard edge")), "{found:?}");
}

#[test]
fn folded_and_uneven_pieces_are_fixed() {
    let mut harness = fresh();
    let id = cube(&harness);
    let all: Vec<scene::FaceId> = shape(&harness).faces().map(|(f, _)| f).collect();
    // One side squashed flat.
    change_uvs(
        &mut harness,
        id,
        vec![all[0]],
        glam::Affine2::from_scale(glam::Vec2::new(1.0, 0.0)),
    );
    let found = texts(&mut harness);
    assert!(
        found.iter().any(|t| t.contains("squashed flat")),
        "{found:?}"
    );
    fix(&mut harness, "squashed flat");
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::Unwrap { .. })
    ));
    let found = texts(&mut harness);
    assert!(!found.iter().any(|t| t.contains("squashed")), "{found:?}");
    // Another side made a third of the size: uneven.
    let mesh = shape(&harness);
    let centre = mesh::uvs::uv_centre(&mesh, &[all[1]], &[]).unwrap();
    change_uvs(
        &mut harness,
        id,
        vec![all[1]],
        glam::Affine2::from_translation(centre)
            * glam::Affine2::from_scale(glam::Vec2::splat(1.0 / 3.0))
            * glam::Affine2::from_translation(-centre),
    );
    let found = texts(&mut harness);
    assert!(
        found.iter().any(|t| t.contains("uneven texel density")),
        "{found:?}"
    );
    fix(&mut harness, "uneven");
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::SetTexelDensity { .. })
    ));
    let found = texts(&mut harness);
    assert!(!found.iter().any(|t| t.contains("uneven")), "{found:?}");
}

#[test]
fn a_shape_without_uvs_gets_seams_and_an_unwrap() {
    let mut harness = fresh();
    let text = "o Wedge\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\n\
                f 1 4 3 2\nf 1 2 6 5\nf 2 3 6\nf 1 5 4\nf 4 5 6 3\n";
    let import = fileio::obj::read_obj(
        text,
        fileio::export::FileUnits::Metres,
        fileio::export::UpAxis::Z,
    )
    .unwrap();
    harness
        .state_mut()
        .workbench
        .imported(std::path::Path::new("/in/wedge.obj"), import);
    harness.run();
    let found = texts(&mut harness);
    assert!(found.contains(&"Wedge: no UVs".to_string()), "{found:?}");
    fix(&mut harness, "no UVs");
    // Seams, unwrapped, and packed with the cube that shares its texture,
    // as one step.
    let found = texts(&mut harness);
    assert!(found.is_empty(), "{found:?}");
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let found = texts(&mut harness);
    assert!(found.contains(&"Wedge: no UVs".to_string()), "{found:?}");
}

#[test]
fn the_uv_menu_opens_the_check() {
    let mut harness = fresh();
    main_menu(&harness, "UV").click();
    harness.run();
    harness.get_by_label("Check for baking…").click();
    harness.run();
    assert!(harness.state().workbench.baking.open);
}
