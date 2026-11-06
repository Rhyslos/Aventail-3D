//! Exploratory checks of the UV suite (unwrap): written by a test pass to look
//! for problems. Each test names what it tries. Tests that show a bug
//! assert the correct behaviour and are ignored with the bug.

#[allow(unused_imports)]
use super::tests;

use super::auto_unwrap::AutoStep;
use super::modelling_tests::{H, note, press};
use super::recipes::Kept;
use super::seams_tests::{add, steps};
use super::*;
use crate::commands::CommandId;
use egui_kittest::kittest::Queryable;
use scene::{FaceId, ObjectId};

fn uv_values(m: &scene::Mesh) -> Vec<glam::Vec2> {
    mesh::uvs::Uvs::of(m)
        .map(|u| u.into_values(m.face_layout().1.len()))
        .unwrap_or_default()
}

fn close(a: &[glam::Vec2], b: &[glam::Vec2]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| p.distance(*q) < 1e-5)
}

fn mesh_of(harness: &H, id: ObjectId) -> std::sync::Arc<scene::Mesh> {
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

fn select(harness: &mut H, ids: &[ObjectId]) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection(ids.iter().copied());
    harness.run();
}

fn undo(harness: &mut H) {
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
}

fn redo(harness: &mut H) {
    harness.state_mut().workbench.document_mut().redo();
    harness.run();
}

/// Each island's UV box.
fn island_boxes(m: &scene::Mesh) -> Vec<(glam::Vec2, glam::Vec2)> {
    let faces: Vec<FaceId> = m.faces().map(|(f, _)| f).collect();
    uv::pack::islands_of(m, &faces)
        .iter()
        .map(|i| {
            let p = uv::pack::piece_of(m, i);
            let pts: Vec<glam::Vec2> = p.polygons.into_iter().flatten().collect();
            let lo = pts.iter().fold(glam::Vec2::MAX, |a, b| a.min(*b));
            let hi = pts.iter().fold(glam::Vec2::MIN, |a, b| a.max(*b));
            (lo, hi)
        })
        .collect()
}

/// The polygons of `a` and `b` lying over others when shown together in
/// one texture (as the UV Editor's red warning finds them).
fn overlaps_together(harness: &H, a: ObjectId, b: ObjectId) -> usize {
    let (ma, mb) = (mesh_of(harness, a), mesh_of(harness, b));
    uv::warnings::warnings(&[&*ma, &*mb], |_, _| 0u8, &[])
        .iter()
        .map(|w| w.overlapping.len())
        .sum()
}


// ---------------------------------------------------------------- unwrap

#[test]
fn unwrap_from_the_menu_then_undo_and_redo_restore_the_uvs_exactly() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Sphere(mesh::SphereParams::default()),
    );
    let before = uv_values(&mesh_of(&harness, id));
    let seams_before = mesh::uvs::seams(&mesh_of(&harness, id));
    // Make it different first: Suggest seams then Pack.
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    tests::uv_group(&mut harness, "Lay out");
    harness.get_by_label("Pack").click();
    harness.run();
    let packed = uv_values(&mesh_of(&harness, id));
    assert!(!close(&before, &packed), "Pack changed the UVs");
    let n = steps(&harness);
    undo(&mut harness);
    undo(&mut harness);
    assert_eq!(steps(&harness), n - 2);
    assert!(close(&before, &uv_values(&mesh_of(&harness, id))));
    assert_eq!(seams_before, mesh::uvs::seams(&mesh_of(&harness, id)));
    redo(&mut harness);
    redo(&mut harness);
    assert!(close(&packed, &uv_values(&mesh_of(&harness, id))));
}

#[test]
fn unwrap_with_nothing_selected_says_so_and_records_nothing() {
    let mut harness = tests::fresh();
    let n = steps(&harness);
    run(&mut harness, CommandId::Unwrap);
    assert_eq!(steps(&harness), n);
    assert!(note(&harness).contains("select"), "{}", note(&harness));
    run(&mut harness, CommandId::Pack);
    assert_eq!(steps(&harness), n);
    assert!(note(&harness).contains("select"), "{}", note(&harness));
}

#[test]
#[ignore = "BUG: the standard Cone, Suggest seams + Unwrap + Pack: 4 side triangles overlap (folded round the tip)"]
fn every_primitive_unwraps_and_packs_without_overlaps_through_the_menus() {
    let mut problems = Vec::new();
    for primitive in mesh::Primitive::all() {
        let mut harness = tests::fresh();
        let id = add(&mut harness, primitive);
        harness.state_mut().workbench.run_suggest_seams();
        run(&mut harness, CommandId::Unwrap);
        run(&mut harness, CommandId::Pack);
        let m = mesh_of(&harness, id);
        let uvs = uv_values(&m);
        if !uvs
            .iter()
            .all(|v| v.is_finite() && (-1e-4..=1.0001).contains(&v.x) && (-1e-4..=1.0001).contains(&v.y))
        {
            problems.push(format!("{}: UVs outside the square", primitive.name()));
        }
        let over = uv::select::overlapping(&m);
        if !over.is_empty() {
            problems.push(format!("{}: {} overlap", primitive.name(), over.len()));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn packing_twice_from_the_menu_is_stable() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Torus(mesh::TorusParams::default()),
    );
    run(&mut harness, CommandId::Pack);
    let once = uv_values(&mesh_of(&harness, id));
    run(&mut harness, CommandId::Pack);
    let twice = uv_values(&mesh_of(&harness, id));
    let worst = once
        .iter()
        .zip(&twice)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0f32, f32::max);
    assert!(worst < 0.01, "a second Pack moved UVs by {worst}");
}

#[test]
fn two_objects_packed_together_do_not_overlap_each_other() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams::default()),
    );
    select(&mut harness, &[a, b]);
    run(&mut harness, CommandId::Pack);
    assert_eq!(overlaps_together(&harness, a, b), 0);
}

#[test]
fn udim_tiles_from_the_settings_spread_pieces_past_one() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        }),
    );
    harness.state_mut().workbench.uv_settings.udim_tiles = 3;
    run(&mut harness, CommandId::Pack);
    let m = mesh_of(&harness, id);
    let uvs = uv_values(&m);
    assert!(uvs.iter().any(|v| v.x > 1.0), "nothing went past tile 1001");
    assert!(uvs.iter().all(|v| v.x <= 3.0001 && v.y <= 1.0001));
    for (lo, hi) in island_boxes(&m) {
        assert_eq!((lo.x + 1e-4).floor(), (hi.x - 1e-4).floor(), "{lo} {hi}");
    }
}

#[test]
fn pack_with_every_piece_locked_says_so() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_lock(true);
    harness.run();
    let before = uv_values(&mesh_of(&harness, id));
    let n = steps(&harness);
    run(&mut harness, CommandId::Pack);
    assert_eq!(steps(&harness), n);
    assert!(note(&harness).contains("locked"), "{}", note(&harness));
    assert!(close(&before, &uv_values(&mesh_of(&harness, id))));
}

// ---------------------------------------------------------- save and load

fn reopened(harness: &H) -> scene::Document {
    let doc = harness.state().workbench.document();
    fileio::project::from_bytes(&fileio::project::to_bytes(doc).unwrap()).unwrap()
}

#[test]
fn unwrapped_packed_lightmapped_uvs_and_sets_survive_save_and_load() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams::default()),
    );
    harness.state_mut().workbench.run_suggest_seams();
    run(&mut harness, CommandId::Unwrap);
    harness.state_mut().workbench.uv_settings.udim_tiles = 2;
    run(&mut harness, CommandId::Pack);
    harness.state_mut().workbench.run_make_lightmap();
    harness
        .state_mut()
        .workbench
        .run_uv_set(mesh::uv_sets::SetChange::Add("detail".into()));
    harness.run();
    let here = mesh_of(&harness, id);
    let back = reopened(&harness);
    let there = back.scene().evaluated(id).unwrap().mesh;
    assert!(close(&uv_values(&here), &uv_values(&there)));
    assert_eq!(mesh::uvs::seams(&here), mesh::uvs::seams(&there));
    assert_eq!(
        mesh::uv_sets::names(&here),
        mesh::uv_sets::names(&there)
    );
    for name in mesh::uv_sets::names(&here) {
        assert!(
            close(
                &mesh::uv_sets::values(&here, &name).unwrap(),
                &mesh::uv_sets::values(&there, &name).unwrap()
            ),
            "set {name}"
        );
    }
}

// --------------------------------------------------------------- UV sets

#[test]
fn uv_sets_add_switch_rename_delete_undo_back_to_the_start() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let start = uv_values(&mesh_of(&harness, id));
    let w = |h: &mut H, c: mesh::uv_sets::SetChange| {
        h.state_mut().workbench.run_uv_set(c);
        h.run();
    };
    w(&mut harness, mesh::uv_sets::SetChange::Add("second".into()));
    w(
        &mut harness,
        mesh::uv_sets::SetChange::MakeActive("second".into()),
    );
    // Change the second set's UVs.
    run(&mut harness, CommandId::Pack);
    let second = uv_values(&mesh_of(&harness, id));
    w(
        &mut harness,
        mesh::uv_sets::SetChange::Rename {
            from: "second".into(),
            to: "renamed".into(),
        },
    );
    assert_eq!(
        mesh::uv_sets::names(&mesh_of(&harness, id)),
        vec!["renamed".to_string(), "UVMap".to_string()]
    );
    // Switching back gives the first layout; the renamed one kept its own.
    w(
        &mut harness,
        mesh::uv_sets::SetChange::MakeActive("UVMap".into()),
    );
    let m = mesh_of(&harness, id);
    assert!(close(&start, &uv_values(&m)));
    assert!(close(
        &second,
        &mesh::uv_sets::values(&m, "renamed").unwrap()
    ));
    w(&mut harness, mesh::uv_sets::SetChange::Delete("UVMap".into()));
    let m = mesh_of(&harness, id);
    assert_eq!(mesh::uv_sets::names(&m), vec!["renamed".to_string()]);
    assert!(close(&second, &uv_values(&m)));
    for _ in 0..6 {
        undo(&mut harness);
    }
    let m = mesh_of(&harness, id);
    assert_eq!(mesh::uv_sets::names(&m), vec!["UVMap".to_string()]);
    assert!(close(&start, &uv_values(&m)));
}

#[test]
#[ignore = "BUG (minor): a UV set change that changes nothing (deleting the only set) still says it was done"]
fn the_last_uv_set_cannot_be_deleted_and_the_ninth_cannot_be_added() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let n = steps(&harness);
    harness
        .state_mut()
        .workbench
        .run_uv_set(mesh::uv_sets::SetChange::Delete("UVMap".into()));
    harness.run();
    assert_eq!(mesh::uv_sets::names(&mesh_of(&harness, id)).len(), 1);
    // Whatever it says, it must not claim it deleted it.
    assert!(
        !note(&harness).starts_with("Delete the UV set UVMap."),
        "says it deleted the only set: {}",
        note(&harness)
    );
    let _ = n;
    for k in 0..10 {
        harness
            .state_mut()
            .workbench
            .run_uv_set(mesh::uv_sets::SetChange::Add(format!("s{k}")));
        harness.run();
    }
    assert_eq!(
        mesh::uv_sets::names(&mesh_of(&harness, id)).len(),
        mesh::uv_sets::MAX_SETS
    );
}

#[test]
fn the_uv_sets_menu_with_two_objects_with_different_sets() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .run_uv_set(mesh::uv_sets::SetChange::Add("extra".into()));
    harness.run();
    let b = add(&mut harness, scene::Primitive::Cube(Default::default()));
    select(&mut harness, &[a, b]);
    // Switching to "extra" with both selected: only a has it.
    harness
        .state_mut()
        .workbench
        .run_uv_set(mesh::uv_sets::SetChange::MakeActive("extra".into()));
    harness.run();
    let said = note(&harness);
    assert_eq!(
        mesh::uv_sets::names(&mesh_of(&harness, a))[0],
        "extra".to_string()
    );
    assert_eq!(mesh::uv_sets::names(&mesh_of(&harness, b))[0], "UVMap");
    // The note should not claim b was switched without saying it wasn't.
    eprintln!("note with two objects: {said}");
}

// ---------------------------------------------------------- lightmaps

#[test]
fn lightmap_uvs_are_their_own_set_and_undo_removes_it() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Torus(mesh::TorusParams::default()),
    );
    let own = uv_values(&mesh_of(&harness, id));
    tests::uv_group(&mut harness, "Lightmap UVs");
    harness.get_by_label("Make lightmap UVs").click();
    harness.run();
    let m = mesh_of(&harness, id);
    assert!(mesh::uv_sets::names(&m).contains(&"lightmap".to_string()));
    assert!(close(&own, &uv_values(&m)), "the painted UVs stayed");
    let lm = mesh::uv_sets::values(&m, "lightmap").unwrap();
    let mut laid = (*m).clone();
    mesh::uvs::set_uvs(&mut laid, lm);
    assert!(uv::select::overlapping(&laid).is_empty());
    undo(&mut harness);
    assert!(!mesh::uv_sets::names(&mesh_of(&harness, id)).contains(&"lightmap".to_string()));
}

#[test]
#[ignore = "BUG: Make lightmap UVs on an object with 8 UV sets makes nothing but says it made a set called lightmap"]
fn lightmap_uvs_with_eight_sets_already_do_not_claim_success() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    for k in 0..7 {
        harness
            .state_mut()
            .workbench
            .run_uv_set(mesh::uv_sets::SetChange::Add(format!("s{k}")));
        harness.run();
    }
    assert_eq!(mesh::uv_sets::names(&mesh_of(&harness, id)).len(), 8);
    let n = steps(&harness);
    harness.state_mut().workbench.run_make_lightmap();
    harness.run();
    eprintln!("8 sets, lightmap: steps {} -> {}", n, steps(&harness));
    let has = mesh::uv_sets::names(&mesh_of(&harness, id)).contains(&"lightmap".to_string());
    assert!(
        has || !note(&harness).contains("a UV set called lightmap"),
        "no lightmap was made, yet the note says: {}",
        note(&harness)
    );
}

#[test]
fn lightmap_while_working_on_the_lightmap_set_keeps_it_laid_out() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_make_lightmap();
    harness.run();
    harness
        .state_mut()
        .workbench
        .run_uv_set(mesh::uv_sets::SetChange::MakeActive("lightmap".into()));
    harness.run();
    harness.state_mut().workbench.run_make_lightmap();
    harness.run();
    let m = mesh_of(&harness, id);
    let names = mesh::uv_sets::names(&m);
    assert_eq!(names.iter().filter(|n| *n == "lightmap").count(), 1, "{names:?}");
    assert!(uv::select::overlapping(&m).is_empty());
}

// ------------------------------------------------------------- auto-unwrap

#[test]
#[ignore = "DOCS: auto-unwrap.md says Set texel density then Pack leaves pieces at the project's density (too small); Pack rescales everything, so the Density step changes nothing"]
fn auto_unwrap_density_step_changes_the_result() {
    let result = |density: bool| {
        let mut harness = tests::fresh();
        let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
        harness.state_mut().workbench.uv_settings.auto_steps.density = density;
        harness.state_mut().workbench.uv_settings.auto_steps.lightmap = false;
        harness.state_mut().workbench.run_auto_unwrap();
        harness.run();
        uv_values(&mesh_of(&harness, id))
    };
    let with = result(true);
    let without = result(false);
    assert!(
        !close(&with, &without),
        "the Set texel density step made no difference before Pack"
    );
}

#[test]
#[ignore = "BUG: Auto-unwrap's Straighten and Set texel density steps don't set the UV intents the menu tools set (uv-intents.md)"]
fn auto_unwrap_sets_the_same_intents_as_the_tools() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams {
            caps: false,
            ..Default::default()
        }),
    );
    harness.state_mut().workbench.uv_settings.auto_steps.pack = false;
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    let m = mesh_of(&harness, id);
    let faces: Vec<FaceId> = m.faces().map(|(f, _)| f).collect();
    assert!(
        mesh::uv_intents::has_intents(&m),
        "after Straighten and Set texel density, the piece keeps {:?}",
        mesh::uv_intents::piece_intent(&m, &faces)
    );
}

#[test]
fn auto_unwrap_on_two_objects_and_an_empty_one() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(
        &mut harness,
        scene::Primitive::Sphere(mesh::SphereParams::default()),
    );
    select(&mut harness, &[a, b]);
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    for id in [a, b] {
        let m = mesh_of(&harness, id);
        assert!(mesh::uv_sets::names(&m).contains(&"lightmap".to_string()));
        assert!(uv::select::overlapping(&m).is_empty(), "{id:?}");
    }
    assert_eq!(overlaps_together(&harness, a, b), 0);
    // Undo all of it.
    undo(&mut harness);
    let report_steps = harness.state().workbench.auto_report.as_ref().map(|r| r.steps.len());
    eprintln!("report after undo: {report_steps:?}");
}

#[test]
fn switching_an_auto_unwrap_step_off_and_on_is_exact() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    let done = uv_values(&mesh_of(&harness, id));
    // The window's switch for Pack.
    let pack = AutoStep::Pack.label();
    let boxes = harness.get_all_by_label(pack).count();
    assert!(boxes > 0, "the window lists Pack");
    harness
        .get_all_by_label(pack)
        .last()
        .unwrap()
        .click();
    harness.run();
    let off = uv_values(&mesh_of(&harness, id));
    assert!(!close(&done, &off), "switching Pack off changed the UVs");
    harness
        .get_all_by_label(pack)
        .last()
        .unwrap()
        .click();
    harness.run();
    assert!(close(&done, &uv_values(&mesh_of(&harness, id))));
}

// --------------------------------------------------------------- recipes

#[test]
#[ignore = "BUG: applying a recipe with nothing selected says \"Applied the recipe\" when an earlier Auto-unwrap window is still open"]
fn applying_a_recipe_with_nothing_selected_does_not_claim_it_ran() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "Walls");
    select(&mut harness, &[]);
    let n = steps(&harness);
    let before = uv_values(&mesh_of(&harness, id));
    harness
        .state_mut()
        .workbench
        .apply_recipe(Kept::Project, "Walls");
    harness.run();
    assert_eq!(steps(&harness), n);
    assert!(close(&before, &uv_values(&mesh_of(&harness, id))));
    assert!(
        !note(&harness).contains("Applied"),
        "nothing was selected, yet: {}",
        note(&harness)
    );
}

#[test]
#[ignore = "DOCS/BUG: auto-unwrap.md says a recipe keeps the texel density, but it uses the project's density at the time it's applied"]
fn a_recipe_keeps_the_texel_density_it_was_made_with() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.uv_settings.auto_steps.pack = false;
    harness.state_mut().workbench.uv_settings.auto_steps.lightmap = false;
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::App, "Dense");
    let mut texture = harness.state().workbench.document().scene().texture();
    let made_with = texture.uv_per_unit();
    texture.density *= 4.0;
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTexture(texture))
        .unwrap();
    select(&mut harness, &[id]);
    harness.state_mut().workbench.apply_recipe(Kept::App, "Dense");
    harness.run();
    let m = mesh_of(&harness, id);
    let density = uv::density::density(&m, &m.faces().map(|(f, _)| f).collect::<Vec<_>>()).unwrap();
    assert!(
        (density - made_with).abs() < made_with * 0.05,
        "the recipe gave {density}, it was kept at {made_with}"
    );
}

#[test]
fn a_recipe_applied_to_another_object_gives_the_same_layout() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.uv_settings.padding = 40;
    harness.state_mut().workbench.uv_settings.seam_corner_angle = 30.0;
    harness
        .state_mut()
        .workbench
        .keep_recipe(Kept::Project, "Crates");
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    let first = uv_values(&mesh_of(&harness, a));
    // The app's own settings go back to standard.
    harness.state_mut().workbench.uv_settings = UvSettings::default();
    let b = add(&mut harness, scene::Primitive::Cube(Default::default()));
    select(&mut harness, &[b]);
    harness
        .state_mut()
        .workbench
        .apply_recipe(Kept::Project, "Crates");
    harness.run();
    assert!(close(&first, &uv_values(&mesh_of(&harness, b))));
    assert_eq!(harness.state().workbench.uv_settings.padding, 8);
    // Saved with the project, read back the same.
    let back = reopened(&harness);
    assert_eq!(back.scene().recipes().len(), 1);
}

// ------------------------------------------------------------ projections

#[test]
fn a_projection_cancelled_with_esc_leaves_the_uvs_and_no_step() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let before = uv_values(&mesh_of(&harness, id));
    let n = steps(&harness);
    tests::uv_group(&mut harness, "Project");
    harness.get_by_label("Box").click();
    harness.run();
    assert!(harness.state().workbench.projecting());
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!harness.state().workbench.projecting());
    assert_eq!(steps(&harness), n);
    assert!(close(&before, &uv_values(&mesh_of(&harness, id))));
}

#[test]
fn a_projection_kept_with_enter_is_one_step() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Cylinder(mesh::CylinderParams::default()),
    );
    let n = steps(&harness);
    harness
        .state_mut()
        .workbench
        .start_projection(scene::Projection::Cylinder, false);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(steps(&harness), n + 1);
    let m = mesh_of(&harness, id);
    assert!(uv_values(&m).iter().all(|v| v.is_finite()));
    // The cylinder's side wraps once round: U spans the square.
    let us: Vec<f32> = uv_values(&m).iter().map(|v| v.x).collect();
    let span = us.iter().cloned().fold(f32::MIN, f32::max) - us.iter().cloned().fold(f32::MAX, f32::min);
    assert!(span > 0.9, "{span}");
}

#[test]
fn world_aligned_neighbours_share_texture_across_the_join() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::SetTransforms(vec![(
            b,
            scene::Transform {
                position: glam::Vec3::new(100.0, 0.0, 0.0),
                ..Default::default()
            },
        )]))
        .unwrap();
    select(&mut harness, &[a, b]);
    harness.state_mut().workbench.run_world_aligned();
    harness.run();
    // The top polygons' corners at x = 50 (world) match.
    let top_corner = |h: &H, id: ObjectId, x: f32| -> Vec<glam::Vec2> {
        let m = mesh_of(h, id);
        let uvs = mesh::uvs::Uvs::of(&m).unwrap();
        let mut out = Vec::new();
        for (f, list) in m.faces() {
            if m.face_normal(f).z < 0.9 {
                continue;
            }
            for (k, v) in list.iter().enumerate() {
                let p = m.position(*v).unwrap();
                if (p.x - x).abs() < 1e-3 {
                    out.push(uvs.corner(&m, f, k).unwrap());
                }
            }
        }
        out.sort_by(|p, q| p.y.total_cmp(&q.y));
        out
    };
    let ra = top_corner(&harness, a, 50.0);
    let rb = top_corner(&harness, b, -50.0);
    assert_eq!(ra.len(), 2);
    assert!(close(&ra, &rb), "{ra:?} vs {rb:?}");
}

// ---------------------------------------------------------------- seams

#[test]
fn seam_loop_outside_the_edges_mode_says_why() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let n = steps(&harness);
    tests::uv_group(&mut harness, "Seams");
    harness.get_by_label("Mark seam loop").click();
    harness.run();
    assert_eq!(steps(&harness), n);
    assert!(note(&harness).contains("Edges"), "{}", note(&harness));
}

#[test]
fn never_seams_here_is_respected_by_suggest_seams() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    let m = mesh_of(&harness, id);
    let seams = mesh::uvs::seams(&m);
    let never = seams[0];
    press(&mut harness, Modifiers::NONE, Key::Num2);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, scene::ElementSelection::of([scene::Element::Edge(never)]))],
            "Select",
            None,
        );
    harness.run();
    harness
        .state_mut()
        .workbench
        .run_seam_preference(scene::SeamPreference::Never);
    harness.run();
    select(&mut harness, &[id]);
    press(&mut harness, Modifiers::NONE, Key::Num1);
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    let m = mesh_of(&harness, id);
    assert!(!mesh::uvs::seams(&m).contains(&never));
    // The cube still unwraps cleanly without that seam.
    run(&mut harness, CommandId::Pack);
    assert!(uv::select::overlapping(&mesh_of(&harness, id)).is_empty());
}

#[test]
fn every_edge_never_a_seam_still_unwraps_a_closed_cube() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness
        .state_mut()
        .workbench
        .run_seam_preference(scene::SeamPreference::Never);
    harness.run();
    harness.state_mut().workbench.run_suggest_seams();
    harness.run();
    run(&mut harness, CommandId::Unwrap);
    let m = mesh_of(&harness, id);
    assert!(uv_values(&m).iter().all(|v| v.is_finite()));
    eprintln!(
        "all-never cube: {} seams, note {}",
        mesh::uvs::seams(&m).len(),
        note(&harness)
    );
}

#[test]
fn the_seam_slider_drag_is_one_undo_step_and_undo_restores_seams() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Torus(mesh::TorusParams::default()),
    );
    let before = mesh::uvs::seams(&mesh_of(&harness, id));
    let n = steps(&harness);
    for t in [0.2, 0.5, 0.9, 1.0] {
        harness.state_mut().workbench.uv_settings.seam_balance = t;
        harness.state_mut().workbench.apply_seam_balance();
        harness.run();
    }
    assert_eq!(steps(&harness), n + 1);
    undo(&mut harness);
    assert_eq!(before, mesh::uvs::seams(&mesh_of(&harness, id)));
}

#[test]
fn mirror_seams_on_a_cube_from_the_menu() {
    let mut harness = tests::fresh();
    let id = add(
        &mut harness,
        scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 1, 1],
            ..Default::default()
        }),
    );
    tests::uv_group(&mut harness, "Seams");
    harness.get_by_label("Mirror seams").click();
    harness.run();
    let m = mesh_of(&harness, id);
    let seams = mesh::uvs::seams(&m);
    let mirrored = uv::seam_tools::mirrored(&m, &seams, 0);
    for e in mirrored {
        assert!(seams.contains(&e), "{e:?} has no mirrored seam");
    }
}

// --------------------------------------------------------------- intents

#[test]
fn keep_this_way_up_on_a_floor_says_something_true() {
    let mut harness = tests::fresh();
    add(
        &mut harness,
        scene::Primitive::Plane(mesh::PlaneParams::default()),
    );
    harness
        .state_mut()
        .workbench
        .run_set_intent(mesh::uv_intents::UvIntent::Upright);
    harness.run();
    eprintln!("upright on a floor: {}", note(&harness));
}

// --------------------------------------------- after undo, and modelling

#[test]
fn the_auto_unwrap_window_after_ctrl_z() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let start = uv_values(&mesh_of(&harness, id));
    harness.state_mut().workbench.run_auto_unwrap();
    harness.run();
    undo(&mut harness);
    assert!(close(&start, &uv_values(&mesh_of(&harness, id))));
    let n = steps(&harness);
    // The window may still be open: its switches must not bring back
    // edits that were taken back, nor panic.
    let pack = AutoStep::Pack.label();
    if let Some(node) = harness.get_all_by_label(pack).last() {
        node.click();
        harness.run();
    }
    eprintln!(
        "after Ctrl+Z, clicking the window's switch: steps {} -> {}, note {}",
        n,
        steps(&harness),
        note(&harness)
    );
    assert!(close(&start, &uv_values(&mesh_of(&harness, id))));
}

#[test]
#[ignore = "BUG (likely): after Pack, extruding the cube's top lays the new walls partly outside the texture square"]
fn extruding_after_packing_keeps_pieces_off_each_other() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    run(&mut harness, CommandId::Pack);
    let m = mesh_of(&harness, id);
    let top = m
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| m.face_normal(*a).z.total_cmp(&m.face_normal(*b).z))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Record {
            id,
            operation: scene::Operation::ExtrudePolygons {
                faces: vec![top],
                distance: 50.0,
                grouping: scene::Grouping::Together,
                shape: scene::ExtrudeShape::default(),
                stretch: glam::Vec3::ONE,
                anchors: None,
            },
        })
        .unwrap();
    harness.run();
    let m = mesh_of(&harness, id);
    let uvs = mesh::uvs::Uvs::of(&m).unwrap();
    // The new walls: upright polygons above the old top (z 50).
    let mut outside = Vec::new();
    for (f, list) in m.faces() {
        let low = list
            .iter()
            .map(|v| m.position(*v).unwrap().z)
            .fold(f32::MAX, f32::min);
        if m.face_normal(f).z.abs() > 0.1 || low < 49.0 {
            continue;
        }
        for k in 0..list.len() {
            let uv = uvs.corner(&m, f, k).unwrap();
            if uv.min_element() < -1e-3 || uv.max_element() > 1.001 {
                outside.push((f, uv));
            }
        }
    }
    eprintln!(
        "after extrude: {} polygons overlap, walls' corners outside the square {:?}",
        uv::select::overlapping(&m).len(),
        outside
    );
    // Packing again sorts it out.
    run(&mut harness, CommandId::Pack);
    assert!(uv::select::overlapping(&mesh_of(&harness, id)).is_empty());
    assert!(
        outside.is_empty(),
        "uvs-and-modelling.md: an extrude's walls become a new piece inside the texture square"
    );
}

#[test]
#[ignore = "CONFUSING: Pack into a region typed backwards (From above To) silently packs into the whole square"]
fn a_backwards_region_from_the_settings_packs_inside_the_square() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.uv_settings.pack_region = Some([0.8, 0.8, 0.2, 0.2]);
    run(&mut harness, CommandId::Pack);
    let uvs = uv_values(&mesh_of(&harness, id));
    let inside = uvs
        .iter()
        .all(|v| (0.2 - 1e-3..=0.8 + 1e-3).contains(&v.x) && (0.2 - 1e-3..=0.8 + 1e-3).contains(&v.y));
    assert!(uvs.iter().all(|v| v.min_element() > -1e-3 && v.max_element() < 1.001));
    assert!(
        inside,
        "a region typed From 0.8 To 0.2 is ignored without a word: the whole square is used"
    );
}

// ------------------------------------------------- packing for the selection

/// The average UV width of `id`'s pieces.
fn piece_width(harness: &H, id: ObjectId) -> f32 {
    let boxes = island_boxes(&mesh_of(harness, id));
    boxes.iter().map(|(lo, hi)| (hi.x - lo.x).max(hi.y - lo.y)).sum::<f32>() / boxes.len() as f32
}

#[test]
fn a_twice_texture_scale_object_gets_twice_as_wide_pieces() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.run_pack_values(
        mesh::uvs::PackValues {
            scale: 2.0,
            ..Default::default()
        },
        mesh::uvs::PackParts {
            scale: true,
            ..Default::default()
        },
    );
    harness.run();
    let b = add(&mut harness, scene::Primitive::Cube(Default::default()));
    select(&mut harness, &[a, b]);
    run(&mut harness, CommandId::Pack);
    let (wa, wb) = (piece_width(&harness, a), piece_width(&harness, b));
    assert!((wa / wb - 2.0).abs() < 0.05, "{wa} vs {wb}");
    assert_eq!(overlaps_together(&harness, a, b), 0);
}

#[test]
fn a_packing_group_across_two_objects_stays_together() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(&mut harness, scene::Primitive::Cube(Default::default()));
    select(&mut harness, &[a, b]);
    run(&mut harness, CommandId::Pack);
    harness.state_mut().workbench.run_pack_values(
        mesh::uvs::PackValues {
            group: 1,
            ..Default::default()
        },
        mesh::uvs::PackParts {
            group: true,
            ..Default::default()
        },
    );
    harness.run();
    let before: Vec<glam::Vec2> = [a, b]
        .iter()
        .flat_map(|id| uv_values(&mesh_of(&harness, *id)))
        .collect();
    harness.state_mut().workbench.uv_settings.padding = 64;
    run(&mut harness, CommandId::Pack);
    let after: Vec<glam::Vec2> = [a, b]
        .iter()
        .flat_map(|id| uv_values(&mesh_of(&harness, *id)))
        .collect();
    // One block: every corner moved by the same similarity, so distances
    // between corners all scale by one factor.
    let ratio = after[0].distance(after[7]) / before[0].distance(before[7]);
    for (i, j) in [(1, 30), (5, 40), (12, 25)] {
        let r = after[i].distance(after[j]) / before[i].distance(before[j]).max(1e-6);
        assert!((r - ratio).abs() < 1e-2, "{r} vs {ratio}");
    }
    assert_eq!(overlaps_together(&harness, a, b), 0);
}
