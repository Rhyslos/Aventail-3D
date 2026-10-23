//! Runs the architecture checks on the real workspace.
//! If this test fails, its message says which crate broke which rule.

use std::path::Path;

fn workspace_root() -> &'static Path {
    // tools/layer-check -> workspace root
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn workspace_follows_architecture_rules() {
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let violations = layer_check::check(&crates);
    let report: Vec<String> = violations.iter().map(ToString::to_string).collect();
    assert!(
        violations.is_empty(),
        "architecture rules broken:\n{}",
        report.join("\n")
    );
}

#[test]
fn every_planned_crate_exists() {
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let names: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();
    let expected = [
        "app", "booleans", "config", "fileio", "mesh", "render", "scene", "tools", "ui", "uv",
    ];
    assert_eq!(names, expected);
}

#[test]
fn every_crate_is_registered_in_the_workspace() {
    // Each crate in crates/ must be listed (with its path) in the root
    // Cargo.toml's [workspace.dependencies], except the app, which nothing
    // depends on.
    let root = std::fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();
    let internal = layer_check::internal_crate_names(&root).unwrap();
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    for c in crates.iter().filter(|c| c.name != "app") {
        assert!(
            internal.contains(&c.name),
            "{} is missing from [workspace.dependencies] in the root Cargo.toml",
            c.name
        );
    }
}

#[test]
fn only_the_app_is_in_the_top_layer() {
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let top: Vec<&str> = crates
        .iter()
        .filter(|c| c.layer == Some(layer_check::MAX_LAYER))
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(top, ["app"]);
}

#[test]
fn the_mesh_crate_stands_alone() {
    // The foundation depends on no other app crate, so it can be tested
    // and reused on its own.
    let crates = layer_check::load_workspace(workspace_root()).unwrap();
    let mesh = crates.iter().find(|c| c.name == "mesh").unwrap();
    assert_eq!(mesh.layer, Some(0));
    assert!(mesh.internal_deps.is_empty());
}
