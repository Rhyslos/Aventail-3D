use super::*;

fn names() -> Names {
    crate::run::tests::names()
}

#[test]
fn every_call_belongs_to_an_owner_and_names_are_unique() {
    let mut keys: Vec<String> = CALLS.iter().map(Call::key).collect();
    for call in CALLS {
        assert!(owner(call.owner).is_some(), "{}", call.key());
        assert!(call.since >= 1 && call.since <= API_VERSION);
        assert!(!call.about.is_empty());
    }
    keys.sort();
    let before = keys.len();
    keys.dedup();
    assert_eq!(keys.len(), before, "two calls share a name");
}

#[test]
fn the_description_files_are_named_after_the_folder_and_say_who_made_them() {
    let files = descriptions(&names());
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "aventail/scene.lsc",
            "aventail/selection.lsc",
            "aventail/project.lsc",
            "aventail/script.lsc"
        ]
    );
    for (_, text) in &files {
        let first = text.lines().next().unwrap();
        assert!(first.starts_with("-- Made by The app 0.0.1"));
        assert!(first.contains(DESCRIPTION_FILE));
    }
    let selection = &files[1].1;
    assert!(selection.contains("fetch \"aventail/scene\":[Shape, Edge];"), "{selection}");
    assert!(selection.contains("state Mode = [Model,"), "{selection}");
}

#[test]
fn the_description_files_check_with_lust_on_their_own() {
    let mut files = descriptions(&names());
    files.insert(
        0,
        (
            "main.lsc".to_string(),
            "fetch \"aventail/scene\":[Scene, Shape, Point, Polygon, Edge];\n\
             fetch \"aventail/selection\":[Selection, Mode];\n\
             fetch \"aventail/project\":Project;\n\
             fetch \"aventail/script\":Script;\nend\n"
                .to_string(),
        ),
    );
    let engine = lust::Engine {
        folders: vec!["aventail".into()],
    };
    lust::check_with(&engine, "main.lsc", &files).unwrap();
}

#[test]
fn another_folder_name_changes_every_face() {
    let other = Names {
        folder: "other".into(),
        made_by: "Other 1".into(),
    };
    assert!(descriptions(&other).iter().all(|(p, _)| p.starts_with("other/")));
    let page = reference_page(&other);
    assert!(page.contains("`other.Scene:selected()`"));
    assert!(!page.contains("aventail"));
}

#[test]
fn the_reference_lists_every_call_in_both_languages() {
    let page = reference_page(&names());
    for call in CALLS {
        assert!(page.contains(&format!("`{}`", call.lust_call())), "{}", call.key());
        assert!(page.contains(&format!("`{}`", call.lua_call("aventail"))), "{}", call.key());
    }
    assert!(page.starts_with("# Script reference\n\n> "));
}
