//! Tests for bakes for the pipeline (decision 0253).

use super::bake_prep::stacked_pieces;
use super::baker::godot_path;
use super::seams_tests::{add, shape, steps};
use super::*;

#[test]
fn every_selected_piece_of_a_kit_is_baked_on_its_own() {
    let mut harness = tests::fresh();
    let a = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let b = add(&mut harness, scene::Primitive::Cylinder(Default::default()));
    let w = &mut harness.state_mut().workbench;
    w.document_mut().set_selection([a, b]);
    let orders = w.bake_orders().unwrap();
    assert_eq!(orders.len(), 2);
    assert_ne!(orders[0].name, "");
}

#[test]
fn a_godot_material_is_written_beside_the_maps() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let dir = std::env::temp_dir().join(format!("bake-godot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("kit")).unwrap();
    std::fs::write(dir.join("project.godot"), "").unwrap();
    let w = &mut harness.state_mut().workbench;
    w.document_mut()
        .execute(scene::Command::SetTexture(scene::TextureSettings {
            size: 256,
            ..Default::default()
        }))
        .unwrap();
    w.baker.maps = [true, false, false, false, false, false, false];
    w.baker.godot = true;
    w.bake_into(dir.join("kit"));
    w.finish_bake();
    let names: Vec<String> = std::fs::read_dir(dir.join("kit"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n.ends_with("_orm.png")), "{names:?}");
    let material = names
        .iter()
        .find(|n| n.ends_with(".tres"))
        .expect("the material");
    let text = std::fs::read_to_string(dir.join("kit").join(material)).unwrap();
    assert!(
        text.contains("res://kit/") && text.contains("orm_texture"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn godot_paths_are_from_the_project_or_beside() {
    let outside = std::env::temp_dir().join("no-godot-here-xyz").join("a.png");
    assert_eq!(godot_path(&outside), "a.png");
}

#[test]
fn stacked_copies_are_left_out_once() {
    // Two cubes in one mesh, their UVs the same: the second lies on the
    // first.
    let cube = scene::Primitive::Cube(Default::default()).mesh();
    let mut two = scene::Mesh::new();
    mesh::parts::append(&mut two, &cube, glam::Mat4::IDENTITY);
    let one = two.face_count();
    mesh::parts::append(
        &mut two,
        &cube,
        glam::Mat4::from_translation(glam::Vec3::X * 300.0),
    );
    let skipped = stacked_pieces(&two);
    assert!(!skipped.is_empty());
    assert!(
        skipped.len() <= one,
        "only the copies, not both: {}",
        skipped.len()
    );
    assert!(stacked_pieces(&cube).len() < cube.face_count());
}

#[test]
fn vertex_colours_are_baked_in_one_step() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [false, true, false, false, false, false, false];
    w.run_bake_vertex_colours();
    assert_eq!(steps(&harness), before + 1);
    assert!(mesh::colours::has_colours(&shape(&harness)));
    // Only maps from the shape itself.
    let w = &mut harness.state_mut().workbench;
    w.baker.maps = [true, false, false, false, false, false, true];
    w.run_bake_vertex_colours();
    assert!(super::modelling_tests::note(&harness).contains("tick occlusion"));
}
