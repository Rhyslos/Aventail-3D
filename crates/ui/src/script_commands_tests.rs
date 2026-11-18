//! What's kept for scripts, and reading the library's commands.

use super::*;

fn key(file: &str, command: Option<&str>) -> CommandKey {
    CommandKey {
        file: file.into(),
        command: command.map(str::to_string),
        project: false,
    }
}

fn names() -> Names {
    Names {
        folder: "aventail".into(),
        made_by: "The app".into(),
    }
}

fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("script-commands-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn f6() -> SavedKey {
    SavedKey::from_press(egui::Key::F6, egui::Modifiers::NONE)
}

#[test]
fn settings_survive_the_trip_through_the_settings_file() {
    let mut settings = ScriptSettings {
        time_seconds: 30,
        memory_mb: 512,
        ..ScriptSettings::default()
    };
    let bevel = key("bevel.lsc", Some("BevelHardEdges"));
    settings.set_on_palette(&bevel, true);
    settings.keys.push((bevel.clone(), f6()));
    settings.keys.push((
        key("tidy.lua", None),
        SavedKey::from_press(
            egui::Key::K,
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        ),
    ));
    settings.off.push("old.lua".into());
    settings.remember(
        &bevel,
        &[
            ("width".into(), SettingValue::Float(0.5)),
            ("segments".into(), SettingValue::Int(3)),
            ("side".into(), SettingValue::Choice("Left".into())),
            ("tint".into(), SettingValue::Colour([1.0, 0.0, 0.5, 1.0])),
            ("at".into(), SettingValue::Vec3([1.0, 2.0, 3.0])),
            ("label".into(), SettingValue::Text("é".into())),
            ("on".into(), SettingValue::Bool(true)),
        ],
    );
    let text = serde_json::to_string(&settings).unwrap();
    assert!(text.contains("\"F6\""), "keys are kept by name: {text}");
    let back: ScriptSettings = serde_json::from_str(&text).unwrap();
    assert_eq!(back.clone().checked(), settings);
    assert_eq!(
        back.values_of(&bevel)[1],
        ("segments".into(), SettingValue::Int(3))
    );
    assert_eq!(back.limits().time, Duration::from_secs(30));
    assert_eq!(back.limits().memory, 512 << 20);
}

#[test]
fn damaged_or_odd_settings_are_put_right() {
    let back: ScriptSettings =
        serde_json::from_str(r#"{"time_seconds": 0, "memory_mb": 99999999}"#).unwrap();
    let back = back.checked();
    assert_eq!(back.time_seconds, 1);
    assert_eq!(back.memory_mb, 2048);
    assert!(serde_json::from_str::<ScriptSettings>(r#"{"keys": [[{"file":"a","command":null},{"key":"NotAKey","ctrl":false,"shift":false,"alt":false}]]}"#).is_err());
    // The same key twice keeps the first.
    let mut two = ScriptSettings::default();
    two.keys.push((key("a.lua", None), f6()));
    two.keys.push((key("b.lua", None), f6()));
    assert_eq!(two.checked().keys.len(), 1);
}

#[test]
fn fuzzed_settings_files_never_panic() {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let good = serde_json::to_string(&ScriptSettings::default()).unwrap();
    let pieces = [
        "{",
        "}",
        "[",
        "]",
        ",",
        ":",
        "\"time_seconds\"",
        "\"palette\"",
        "\"keys\"",
        "\"values\"",
        "\"off\"",
        "\"F6\"",
        "null",
        "-1",
        "1e99",
        "\"Int\"",
        "{\"Float\":1}",
        "true",
    ];
    for _ in 0..3_000 {
        let mut text = good.clone();
        for _ in 0..(next() % 4) {
            let at = (next() as usize) % (text.len() + 1);
            let at = (0..=at)
                .rev()
                .find(|i| text.is_char_boundary(*i))
                .unwrap_or(0);
            text.insert_str(at, pieces[(next() as usize) % pieces.len()]);
        }
        if let Ok(read) = serde_json::from_str::<ScriptSettings>(&text) {
            let read = read.checked();
            assert!(TIME_RANGE.contains(&read.time_seconds));
            assert!(MEMORY_RANGE.contains(&read.memory_mb));
            let _ = read.limits();
        }
    }
}

#[test]
fn renaming_and_removing_a_script_carry_what_was_kept() {
    let mut settings = ScriptSettings::default();
    let a = key("a.lua", Some("A"));
    settings.set_on_palette(&a, true);
    settings.keys.push((a.clone(), f6()));
    settings.remember(&a, &[("n".into(), SettingValue::Int(2))]);
    settings.off.push("a.lua".into());
    settings.renamed("a.lua", "b.lua");
    let b = key("b.lua", Some("A"));
    assert!(settings.on_palette(&b));
    assert_eq!(settings.key_of(&b), Some(f6()));
    assert_eq!(settings.values_of(&b).len(), 1);
    assert!(settings.is_off("b.lua"));
    settings.removed("b.lua");
    assert_eq!(settings, ScriptSettings::default());
}

#[test]
fn the_palette_keeps_one_place_each_within_its_limit() {
    let mut settings = ScriptSettings::default();
    let a = key("a.lua", None);
    settings.set_on_palette(&a, true);
    settings.set_on_palette(&a, true);
    assert_eq!(settings.palette.len(), 1);
    for i in 0..MAX_KEPT + 5 {
        settings.set_on_palette(&key(&format!("{i}.lua"), None), true);
    }
    assert_eq!(settings.palette.len(), MAX_KEPT);
    settings.set_on_palette(&a, false);
    assert!(!settings.on_palette(&a));
}

/// A registry that has read `dir`.
fn read(dir: &Path) -> Registry {
    let library = Library::new(dir.to_path_buf());
    let mut registry = Registry::default();
    registry.refresh(Some(&library), &names(), Limits::default(), Instant::now());
    registry.wait();
    registry
}

#[test]
fn the_library_s_commands_are_read_and_named() {
    let dir = folder("named");
    std::fs::write(
        dir.join("bevel.lua"),
        "-- name: Bevel hard edges\n-- count: segments\nlocal B = command(\"BevelHardEdges\", { width = 0.2, segments = 1 })\nfunction B:run() end\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("two.lua"),
        "-- name: Tidy\nlocal A = command(\"Names\", {})\nfunction A:run() end\nlocal B = command(\"Materials\", {})\nfunction B:run() end\n",
    )
    .unwrap();
    std::fs::write(dir.join("plain.lua"), "print(1)\n").unwrap();
    std::fs::write(dir.join("broken.lua"), "this is not lua\n").unwrap();
    let registry = read(&dir);
    let settings = ScriptSettings::default();
    let names: Vec<String> = registry
        .commands(&settings)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(
        names,
        [
            "Bevel hard edges",
            "plain",
            "Tidy: Materials",
            "Tidy: Names"
        ]
    );
    let bevel = &registry.commands(&settings)[0];
    assert_eq!(bevel.count.as_deref(), Some("segments"));
    assert_eq!(bevel.settings.len(), 2);
    let mut off = settings.clone();
    off.off.push("two.lua".into());
    assert_eq!(registry.commands(&off).len(), 2);
}

#[test]
fn two_commands_with_the_same_name_are_told_apart() {
    let dir = folder("twins");
    for file in ["a.lua", "b.lua"] {
        std::fs::write(
            dir.join(file),
            "local T = command(\"Tidy\", {})\nfunction T:run() end\n",
        )
        .unwrap();
    }
    let registry = read(&dir);
    let names: Vec<String> = registry
        .commands(&ScriptSettings::default())
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["Tidy (a.lua)", "Tidy (b.lua)"]);
}

#[test]
fn a_changed_script_is_read_again_and_a_removed_one_goes() {
    let dir = folder("changed");
    let path = dir.join("a.lua");
    std::fs::write(
        &path,
        "local A = command(\"A\", {})\nfunction A:run() end\n",
    )
    .unwrap();
    let library = Library::new(dir.clone());
    let mut registry = Registry::default();
    let start = Instant::now();
    registry.refresh(Some(&library), &names(), Limits::default(), start);
    registry.wait();
    assert_eq!(registry.commands(&ScriptSettings::default())[0].name, "A");
    std::fs::write(
        &path,
        "local B = command(\"Bee\", { n = 1 })\nfunction B:run() end\n-- longer\n",
    )
    .unwrap();
    registry.refresh(
        Some(&library),
        &names(),
        Limits::default(),
        start + LOOK_EVERY * 2,
    );
    registry.wait();
    assert_eq!(registry.commands(&ScriptSettings::default())[0].name, "Bee");
    std::fs::remove_file(&path).unwrap();
    registry.refresh(
        Some(&library),
        &names(),
        Limits::default(),
        start + LOOK_EVERY * 4,
    );
    assert!(registry.scripts.is_empty());
}

#[test]
fn remembered_values_are_used_only_while_they_fit() {
    let dir = folder("values");
    std::fs::write(
        dir.join("a.lua"),
        "local A = command(\"A\", { n = 1, w = 0.5 })\nfunction A:run() end\n",
    )
    .unwrap();
    let registry = read(&dir);
    let mut settings = ScriptSettings::default();
    let command = registry.commands(&settings)[0].clone();
    settings.remember(
        &command.key,
        &[
            ("n".into(), SettingValue::Int(4)),
            ("w".into(), SettingValue::Text("no".into())),
            ("gone".into(), SettingValue::Int(1)),
        ],
    );
    assert_eq!(
        command.values(&settings),
        [
            ("n".to_string(), SettingValue::Int(4)),
            ("w".to_string(), SettingValue::Float(0.5)),
        ]
    );
}
