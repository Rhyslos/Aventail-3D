//! Trust and approvals kept in the settings, and a project's scripts read
//! only once trusted.

use super::*;
use crate::script_commands::ScriptSettings;

fn names() -> Names {
    Names {
        folder: "aventail".into(),
        made_by: "The app".into(),
    }
}

fn library(file: &str) -> ScriptId {
    ScriptId::Library(file.into())
}

#[test]
fn a_script_asking_nothing_needs_no_approval() {
    let settings = ScriptSettings::default();
    assert!(!settings.needs_approval(&library("a.lua"), &[]));
    assert_eq!(
        settings.access_for(Some(&library("a.lua")), &[], Some(Path::new("/p"))),
        Access::default()
    );
}

#[test]
fn an_approval_covers_what_was_asked_and_a_bigger_ask_asks_again() {
    let mut settings = ScriptSettings::default();
    let id = library("a.lua");
    let read = [Ask::ReadProjectFolder];
    assert!(settings.needs_approval(&id, &read));
    settings.approve(&id, &read, None);
    assert!(!settings.needs_approval(&id, &read));
    let access = settings.access_for(Some(&id), &read, Some(Path::new("/project")));
    assert!(access.read && !access.write);
    assert_eq!(access.folder.as_deref(), Some(Path::new("/project")));
    // A later version asks to write too: asked again, and until then it
    // only reads.
    let more = [Ask::ReadProjectFolder, Ask::WriteProjectFolder];
    assert!(settings.needs_approval(&id, &more));
    assert!(
        !settings
            .access_for(Some(&id), &more, Some(Path::new("/p")))
            .write
    );
    // Asking for less than was allowed gets only what it asks.
    settings.approve(&id, &more, None);
    let access = settings.access_for(Some(&id), &read, Some(Path::new("/p")));
    assert!(access.read && !access.write);
    // Another script gets nothing from this one's approval.
    assert!(settings.needs_approval(&library("b.lua"), &read));
}

#[test]
fn a_chosen_folder_needs_its_folder_and_wins_over_the_projects() {
    let mut settings = ScriptSettings::default();
    let id = library("export.lua");
    let asks = [Ask::ChosenFolder, Ask::ReadProjectFolder];
    settings.approve(&id, &asks, None);
    assert!(settings.needs_approval(&id, &asks), "no folder chosen");
    settings.approve(&id, &asks, Some(PathBuf::from("/exports")));
    assert!(!settings.needs_approval(&id, &asks));
    let access = settings.access_for(Some(&id), &asks, Some(Path::new("/project")));
    assert!(access.chosen && access.write);
    assert_eq!(access.folder.as_deref(), Some(Path::new("/exports")));
}

#[test]
fn an_unsaved_project_gives_no_folder_and_the_console_reaches_nothing() {
    let mut settings = ScriptSettings::default();
    let id = library("a.lua");
    let asks = [Ask::WriteProjectFolder];
    settings.approve(&id, &asks, None);
    let access = settings.access_for(Some(&id), &asks, None);
    assert!(access.write && access.folder.is_none());
    assert_eq!(
        settings.access_for(None, &asks, Some(Path::new("/p"))),
        Access::default()
    );
}

#[test]
fn taking_back_renaming_and_removing_follow_the_script() {
    let mut settings = ScriptSettings::default();
    let asks = [Ask::ReadProjectFolder];
    settings.approve(&library("a.lua"), &asks, None);
    settings.renamed("a.lua", "b.lua");
    assert!(!settings.needs_approval(&library("b.lua"), &asks));
    assert!(settings.needs_approval(&library("a.lua"), &asks));
    settings.removed("b.lua");
    assert!(settings.needs_approval(&library("b.lua"), &asks));
    settings.approve(&library("c.lua"), &asks, None);
    settings.take_back(&library("c.lua"));
    assert!(settings.needs_approval(&library("c.lua"), &asks));
}

#[test]
fn approvals_and_trust_survive_a_restart_and_damage_is_put_right() {
    let mut settings = ScriptSettings::default();
    settings.approve(&library("a.lua"), &[Ask::ChosenFolder], Some("/x".into()));
    settings.trust(&script::fingerprint("print(1)"));
    let text = serde_json::to_string(&settings).unwrap();
    let back: ScriptSettings = serde_json::from_str::<ScriptSettings>(&text)
        .unwrap()
        .checked();
    assert_eq!(back, settings);
    let mut damaged = settings.clone();
    damaged.trusted.push("not a fingerprint".into());
    damaged.approvals.push(damaged.approvals[0].clone());
    damaged.keys.push((
        crate::script_commands::CommandKey {
            file: "kept.lua".into(),
            command: None,
            project: true,
        },
        crate::script_commands::SavedKey {
            key: crate::script_commands::SavedKeyName(egui::Key::F7),
            ctrl: false,
            shift: false,
            alt: false,
        },
    ));
    let checked = damaged.checked();
    assert_eq!(checked.trusted.len(), 1);
    assert_eq!(checked.approvals.len(), 1);
    assert!(checked.keys.is_empty(), "project commands keep no keys");
}

#[test]
fn trust_is_kept_once_and_the_oldest_go_first() {
    let mut settings = ScriptSettings::default();
    for n in 0..=MAX_TRUSTED {
        settings.trust(&script::fingerprint(&n.to_string()));
    }
    settings.trust(&script::fingerprint("1"));
    assert_eq!(settings.trusted.len(), MAX_TRUSTED);
    assert!(!settings.is_trusted(&script::fingerprint("0")));
    assert!(settings.is_trusted(&script::fingerprint(&MAX_TRUSTED.to_string())));
}

fn kept() -> Arc<[scene::ProjectScript]> {
    vec![
        scene::ProjectScript {
            name: "Hello.lua".into(),
            text: "-- name: Hello\n-- asks: read the project folder\nlocal C = command(\"Hello\", { times = 2 })\nfunction C:run() end\n".into(),
        },
        scene::ProjectScript {
            name: "Spy.lua".into(),
            // Its start would say something if it were ever run.
            text: "print(\"started\")\n".into(),
        },
    ]
    .into()
}

#[test]
fn an_untrusted_projects_scripts_are_never_read_or_listed_as_commands() {
    let settings = ScriptSettings::default();
    let mut project = ProjectScripts::default();
    assert!(project.refresh(kept(), &settings, &names(), Limits::default()));
    project.wait();
    assert_eq!(project.untrusted(&settings), 2);
    assert!(project.entries.iter().all(|e| e.described.is_none()));
    assert!(project.commands(&settings).is_empty());
    // Their headers are read without running anything.
    assert_eq!(
        project.entry("Hello.lua").unwrap().header.asks,
        [Ask::ReadProjectFolder]
    );
}

#[test]
fn a_trusted_projects_scripts_become_commands_of_the_project() {
    let mut settings = ScriptSettings::default();
    let list = kept();
    let mut project = ProjectScripts::default();
    project.refresh(list.clone(), &settings, &names(), Limits::default());
    for entry in &project.entries {
        settings.trust(&entry.fingerprint);
    }
    project.refresh(list.clone(), &settings, &names(), Limits::default());
    project.wait();
    let commands = project.commands(&settings);
    assert_eq!(commands.len(), 2, "{commands:?}");
    assert!(commands.iter().all(|c| c.key.project));
    assert!(commands.iter().any(|c| c.name == "Hello"));
    assert_eq!(project.untrusted(&settings), 0);
    // The same list again changes nothing.
    assert!(!project.refresh(list, &settings, &names(), Limits::default()));
    // Changing a script's text makes it a different script: off again.
    let changed: Arc<[scene::ProjectScript]> = vec![scene::ProjectScript {
        name: "Hello.lua".into(),
        text: "print(\"different\")\n".into(),
    }]
    .into();
    project.refresh(changed, &settings, &names(), Limits::default());
    assert_eq!(project.untrusted(&settings), 1);
}
