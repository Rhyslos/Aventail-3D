//! Tests for UV sets (decision 0241).

use super::*;
use crate::{CubeParams, Primitive};

fn cube() -> Mesh {
    Primitive::Cube(CubeParams::default()).mesh()
}

fn add(mesh: &mut Mesh, name: &str) -> Outcome {
    change(mesh, &SetChange::Add(name.into()))
}

#[test]
fn a_shape_starts_with_one_set_and_can_add_more() {
    let mut mesh = cube();
    assert_eq!(names(&mesh), vec![STANDARD_NAME.to_string()]);
    assert_eq!(add(&mut mesh, "detail").matched, 1);
    assert_eq!(
        names(&mesh),
        vec![STANDARD_NAME.to_string(), "detail".into()]
    );
    // The new set starts as a copy of the active one.
    assert_eq!(values(&mesh, "detail"), values(&mesh, STANDARD_NAME));
    // A taken name, or a broken one, is refused.
    assert_eq!(add(&mut mesh, "detail").skipped, 1);
    assert_eq!(add(&mut mesh, "").skipped, 1);
    assert_eq!(add(&mut mesh, "a:b").skipped, 1);
    assert_eq!(add(&mut mesh, &"x".repeat(MAX_NAME + 1)).skipped, 1);
}

#[test]
fn making_a_set_active_swaps_it_with_uv() {
    let mut mesh = cube();
    add(&mut mesh, "second");
    // Change the active set's UVs.
    let moved: Vec<Vec2> = values(&mesh, STANDARD_NAME)
        .unwrap()
        .into_iter()
        .map(|v| v * 0.5)
        .collect();
    crate::uvs::set_uvs(&mut mesh, moved.clone());
    let second = values(&mesh, "second").unwrap();
    assert_eq!(
        change(&mut mesh, &SetChange::MakeActive("second".into())).matched,
        1
    );
    assert_eq!(active_name(&mesh), "second");
    assert_eq!(values(&mesh, "second"), Some(second.clone()));
    assert_eq!(values(&mesh, STANDARD_NAME), Some(moved));
    // The UV tools see the active set as `uv`.
    let uvs = crate::uvs::Uvs::of(&mesh).unwrap();
    assert_eq!(uvs.at(0), second[0]);
    // Making the active set active again changes nothing.
    assert_eq!(
        change(&mut mesh, &SetChange::MakeActive("second".into())).skipped,
        1
    );
}

#[test]
fn copy_rename_and_delete() {
    let mut mesh = cube();
    assert_eq!(
        change(
            &mut mesh,
            &SetChange::Copy {
                from: STANDARD_NAME.into(),
                to: "copy".into()
            }
        )
        .matched,
        1
    );
    assert_eq!(
        change(
            &mut mesh,
            &SetChange::Rename {
                from: "copy".into(),
                to: "kept".into()
            }
        )
        .matched,
        1
    );
    assert!(names(&mesh).contains(&"kept".to_string()));
    // Renaming the active set.
    change(
        &mut mesh,
        &SetChange::Rename {
            from: STANDARD_NAME.into(),
            to: "main".into(),
        },
    );
    assert_eq!(active_name(&mesh), "main");
    // Deleting the active set: the other takes its place.
    let kept = values(&mesh, "kept").unwrap();
    assert_eq!(
        change(&mut mesh, &SetChange::Delete("main".into())).matched,
        1
    );
    assert_eq!(names(&mesh), vec!["kept".to_string()]);
    assert_eq!(values(&mesh, "kept"), Some(kept));
    // The last set can't go.
    assert_eq!(
        change(&mut mesh, &SetChange::Delete("kept".into())).skipped,
        1
    );
    // Unknown names change nothing.
    assert_eq!(
        change(&mut mesh, &SetChange::Delete("nope".into())).skipped,
        1
    );
}

#[test]
fn the_lightmap_is_exported_second_and_there_is_a_limit() {
    let mut mesh = cube();
    add(&mut mesh, "a");
    add(&mut mesh, LIGHTMAP_NAME);
    assert_eq!(second(&mesh), Some(LIGHTMAP_NAME.to_string()));
    for i in 0..MAX_SETS {
        add(&mut mesh, &format!("s{i}"));
    }
    assert_eq!(names(&mesh).len(), MAX_SETS);
    // Without UVs, nothing.
    let mut bare = cube();
    bare.remove_attribute(crate::attributes::UV);
    assert!(names(&bare).is_empty());
    assert_eq!(add(&mut bare, "x").skipped, 1);
}
