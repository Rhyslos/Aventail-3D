//! The script library on disk: the description files, listing, new
//! scripts and their names, saving, and stamps.

use super::*;
use crate::run::tests::{names, run_on, said, snapshot};
use crate::{Ending, Limits};

/// A fresh, empty folder for one test.
pub(crate) fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("script-library-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn preparing_writes_the_description_files_once_and_again_when_changed() {
    let dir = folder("prepare");
    let library = Library::new(dir.clone());
    library.prepare(&names()).unwrap();
    let scene = dir.join("aventail").join("scene.lsc");
    let written = std::fs::read_to_string(&scene).unwrap();
    assert!(written.contains(crate::describe::DESCRIPTION_FILE));
    for (path, text) in crate::descriptions(&names()) {
        assert_eq!(std::fs::read_to_string(dir.join(path)).unwrap(), text);
    }
    // Unchanged files aren't written again.
    let before = stamp(&scene).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    library.prepare(&names()).unwrap();
    assert_eq!(stamp(&scene), Some(before));
    // Changed by hand, they're put back.
    std::fs::write(&scene, "spoilt").unwrap();
    library.prepare(&names()).unwrap();
    assert_eq!(std::fs::read_to_string(&scene).unwrap(), written);
    // The description files aren't listed as scripts.
    assert!(library.scripts().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn new_scripts_start_from_a_starter_that_runs_in_both_languages() {
    let dir = folder("create");
    let library = Library::new(dir.clone());
    let lust = library
        .create("Say names", Language::LustScript, &names())
        .unwrap();
    let lua = library.create("say names", Language::Lua, &names()).unwrap();
    assert_eq!(lust.file_name().unwrap(), "Say names.lsc");
    assert_eq!(
        library.scripts().unwrap(),
        [lust.clone(), lua.clone()],
        "listed by name"
    );
    for path in [&lust, &lua] {
        let file = ScriptFile::read(path).unwrap();
        assert_eq!(crate::read_header(&file.text).name.as_deref(), Some(path.file_stem().unwrap().to_str().unwrap()));
        assert!(crate::check(&file, &names()).is_empty(), "{}", file.text);
        let ran = run_on(&file.name, &file.text, snapshot().0, Limits::default());
        assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
        // The cube is selected.
        assert_eq!(said(&ran.lines).len(), 1, "{:?}", ran.lines);
    }
    // A name already used is refused, and the script is left alone.
    let again = library.create("Say names", Language::LustScript, &names());
    assert!(again.unwrap_err().contains("already"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn names_a_file_cant_have_are_refused_with_a_reason() {
    for bad in [
        "", "   ", "a/b", "a\\b", "a:b", "what?", "a*", "<x>", "a|b", "\"q\"", "tab\there", ".hidden",
        "end.", "CON", "nul", "com1", "LPT9.x",
    ] {
        let why = file_name(bad, Language::Lua).unwrap_err();
        assert!(!why.is_empty(), "{bad:?}");
    }
    assert!(file_name(&"x".repeat(MAX_NAME_CHARS + 1), Language::Lua).is_err());
    assert_eq!(
        file_name(&"é".repeat(MAX_NAME_CHARS), Language::Lua).unwrap(),
        format!("{}.lua", "é".repeat(MAX_NAME_CHARS))
    );
    assert_eq!(
        file_name("  Bevel v1.2 ", Language::LustScript).unwrap(),
        "Bevel v1.2.lsc"
    );
    assert_eq!(file_name("Console", Language::Lua).unwrap(), "Console.lua");
    assert_eq!(file_name("com10", Language::Lua).unwrap(), "com10.lua");
}

#[test]
fn fuzzed_names_are_safe_file_names_or_refused() {
    let mut seed: u64 = 0xf11e;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3_000 {
        let name: String = (0..next() % 12)
            .map(|_| {
                let pick = next() % 6;
                match pick {
                    0 => char::from_u32((next() % 0x11_0000) as u32).unwrap_or('x'),
                    1 => ['/', '\\', '.', ':', '\0', '\n', ' '][(next() % 7) as usize],
                    _ => (b'a' + (next() % 26) as u8) as char,
                }
            })
            .collect();
        if let Ok(file) = file_name(&name, Language::Lua) {
            assert!(!file.contains(['/', '\\', ':', '\0', '\n']), "{name:?}");
            assert!(!file.starts_with('.'));
            assert_eq!(language_of(Path::new(&file)), Some(Language::Lua));
            assert_eq!(Path::new(&file).components().count(), 1, "{name:?}");
        }
    }
}

#[test]
fn listing_skips_other_files_and_a_missing_folder_says_why() {
    let dir = folder("listing");
    let library = Library::new(dir.clone());
    assert!(library.scripts().unwrap_err().contains("couldn't be read"));
    std::fs::create_dir_all(dir.join("folder.lua")).unwrap();
    std::fs::write(dir.join("notes.txt"), "x").unwrap();
    std::fs::write(dir.join("B.LUA"), "x").unwrap();
    std::fs::write(dir.join("a.lsc"), "x").unwrap();
    let names: Vec<_> = library
        .scripts()
        .unwrap()
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, ["a.lsc", "B.LUA"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn saving_writes_the_whole_text_and_changes_the_stamp() {
    let dir = folder("save");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s.lua");
    assert_eq!(stamp(&path), None);
    save(&path, "print(1)\n").unwrap();
    let first = stamp(&path).unwrap();
    save(&path, "print(12)\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "print(12)\n");
    assert_ne!(stamp(&path).unwrap(), first, "a longer file has a new stamp");
    // A folder that isn't there says so.
    assert!(save(&dir.join("no").join("s.lua"), "x").is_err());
    let _ = std::fs::remove_dir_all(dir);
}
