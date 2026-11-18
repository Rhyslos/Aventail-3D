//! Reaching files: every path trick refused, permissions kept, writing
//! only when a run ends well.

use super::*;
use crate::run::tests::{mistakes, run_on, snapshot};
use crate::{Ending, Limits};

/// A fresh folder of its own for a test.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("script-access-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn full(root: &Path) -> Access {
    Access {
        folder: Some(root.to_path_buf()),
        read: true,
        write: true,
        chosen: false,
    }
}

fn read_only(root: &Path) -> Access {
    Access {
        write: false,
        ..full(root)
    }
}

/// Runs a Lua script with `access`.
fn run_with_access(text: &str, access: Access) -> crate::run::tests::Ran {
    let (mut snap, _) = snapshot();
    snap.access = access;
    run_on("files.lua", text, snap, Limits::default())
}

#[test]
fn every_path_trick_is_refused() {
    let root = folder("tricks");
    std::fs::create_dir_all(root.join("inside")).unwrap();
    let long_name = "n".repeat(MAX_NAME_BYTES + 1);
    let long_path = "a/".repeat(MAX_PATH_BYTES / 2 + 1);
    let deep = "d/".repeat(MAX_DEPTH + 1) + "f.txt";
    for given in [
        "",
        "   ",
        "..",
        "../out.txt",
        "inside/../../out.txt",
        "inside\\..\\..\\out.txt",
        "/etc/passwd",
        "\\\\server\\share\\x.txt",
        "C:\\Windows\\win.ini",
        "C:out.txt",
        "file.txt:hidden",
        "inside",
        "./",
        long_name.as_str(),
        long_path.as_str(),
        deep.as_str(),
        "bad<name>.txt",
        "what?.txt",
        "tab\there.txt",
        "nul\0.txt",
        "ends.in.dot.",
        "ends in space ",
        "CON",
        "nul.txt",
        "com1.log",
        "LPT9",
    ] {
        assert!(place(&root, given).is_err(), "{given:?} should be refused");
    }
    for given in [
        "notes.txt",
        "inside/notes.txt",
        "./a/b/c.txt",
        "inside//x.txt",
        "con1.txt",
    ] {
        let path = place(&root, given).unwrap();
        assert!(
            path.starts_with(std::fs::canonicalize(&root).unwrap()),
            "{given}"
        );
    }
}

#[cfg(unix)]
#[test]
fn links_out_of_the_folder_are_refused() {
    let root = folder("links");
    let outside = folder("links-outside");
    std::fs::write(outside.join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("away")).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.txt"), root.join("secret.txt")).unwrap();
    // Even a link that stays inside isn't followed.
    std::fs::write(root.join("real.txt"), "here").unwrap();
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).unwrap();
    for given in ["away/secret.txt", "secret.txt", "alias.txt", "away/new.txt"] {
        let why = place(&root, given).unwrap_err();
        assert!(why.contains("link"), "{given}: {why}");
    }
    let ran = run_with_access(
        r#"aventail.Project:writeText("away/planted.txt", "x")"#,
        full(&root),
    );
    assert_eq!(ran.ending, Ending::Failed);
    assert!(!outside.join("planted.txt").exists());
}

#[test]
fn a_script_reads_and_writes_inside_its_folder() {
    let root = folder("normal");
    std::fs::write(root.join("names.txt"), "Chair\nTable").unwrap();
    let ran = run_with_access(
        r#"
        local names = aventail.Project:readText("names.txt")
        aventail.Project:writeText("out/copy.txt", names .. "\nStool")
        print(aventail.Project:readText("out/copy.txt"))
        "#,
        full(&root),
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", mistakes(&ran.lines));
    assert_eq!(
        std::fs::read_to_string(root.join("out/copy.txt")).unwrap(),
        "Chair\nTable\nStool"
    );
    let said = crate::run::tests::said(&ran.lines);
    assert!(said.iter().any(|l| l.contains("Stool")), "{said:?}");
}

#[test]
fn a_write_without_permission_is_refused_and_nothing_is_written() {
    let root = folder("refused");
    for access in [Access::default(), read_only(&root)] {
        let ran = run_with_access(r#"aventail.Project:writeText("out.txt", "hello")"#, access);
        assert_eq!(ran.ending, Ending::Failed);
        let mistake = mistakes(&ran.lines).remove(0).message;
        assert!(
            mistake.contains("isn't allowed to write files"),
            "{mistake}"
        );
        assert!(mistake.contains("-- asks:"), "{mistake}");
        assert!(!root.join("out.txt").exists());
    }
    let ran = run_with_access(r#"aventail.Project:readText("x.txt")"#, Access::default());
    let mistake = mistakes(&ran.lines).remove(0).message;
    assert!(mistake.contains("isn't allowed to read files"), "{mistake}");
}

#[test]
fn an_unsaved_project_has_no_folder_to_reach() {
    let ran = run_with_access(
        r#"aventail.Project:readText("x.txt")"#,
        Access {
            folder: None,
            read: true,
            write: true,
            chosen: false,
        },
    );
    let mistake = mistakes(&ran.lines).remove(0).message;
    assert!(mistake.contains("hasn't been saved yet"), "{mistake}");
}

#[test]
fn a_run_that_fails_writes_nothing() {
    let root = folder("failing");
    let ran = run_with_access(
        r#"
        aventail.Project:writeText("first.txt", "one")
        error("and then it went wrong")
        "#,
        full(&root),
    );
    assert_eq!(ran.ending, Ending::Failed);
    assert!(!root.join("first.txt").exists());
}

#[test]
fn a_stopped_or_timed_out_run_writes_nothing() {
    let root = folder("timed");
    let (mut snap, _) = snapshot();
    snap.access = full(&root);
    let ran = run_on(
        "slow.lua",
        r#"aventail.Project:writeText("first.txt", "one") while true do end"#,
        snap,
        Limits {
            time: std::time::Duration::from_millis(200),
            ..Limits::default()
        },
    );
    assert_eq!(ran.ending, Ending::OutOfTime);
    assert!(!root.join("first.txt").exists());
}

#[test]
fn reading_is_capped_and_wants_text() {
    let root = folder("caps");
    std::fs::write(
        root.join("big.txt"),
        vec![b'a'; MAX_READ_BYTES as usize + 1],
    )
    .unwrap();
    std::fs::write(root.join("bytes.bin"), [0xff, 0xfe, 0x00]).unwrap();
    for (given, words) in [
        ("big.txt", "bigger than"),
        ("bytes.bin", "isn't text"),
        ("missing.txt", "no such file"),
    ] {
        let ran = run_with_access(
            &format!(r#"aventail.Project:readText("{given}")"#),
            read_only(&root),
        );
        let mistake = mistakes(&ran.lines).remove(0).message;
        assert!(mistake.contains(words), "{given}: {mistake}");
    }
}

#[test]
fn writing_too_much_is_refused() {
    let mut writes = Writes::default();
    let root = folder("much");
    let access = full(&root);
    for n in 0..MAX_WRITES {
        write(&access, &mut writes, "call", &format!("f{n}.txt"), vec![]).unwrap();
    }
    assert!(write(&access, &mut writes, "call", "one-more.txt", vec![]).is_err());
    // Writing a file again replaces it, and doesn't count twice.
    write(&access, &mut writes, "call", "f0.txt", vec![1]).unwrap();
    let mut writes = Writes::default();
    write(
        &access,
        &mut writes,
        "call",
        "a.bin",
        vec![0; MAX_WRITE_BYTES],
    )
    .unwrap();
    assert!(write(&access, &mut writes, "call", "b.bin", vec![0]).is_err());
    write(&access, &mut writes, "call", "a.bin", vec![0; 10]).unwrap();
    write(&access, &mut writes, "call", "b.bin", vec![0]).unwrap();
}

#[test]
fn a_chosen_folder_is_reached_like_the_projects() {
    let root = folder("chosen");
    let ran = run_with_access(
        r#"aventail.Project:writeText("report.txt", "done")"#,
        Access {
            chosen: true,
            ..full(&root)
        },
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", mistakes(&ran.lines));
    assert_eq!(
        std::fs::read_to_string(root.join("report.txt")).unwrap(),
        "done"
    );
}

#[test]
fn shapes_export_with_a_preset_in_both_languages() {
    let root = folder("export");
    let ran = run_with_access(
        r#"
        aventail.Project:exportShapes(aventail.Scene:shapes(), "out/all.glb", aventail.Preset.Godot)
        aventail.Project:exportShapes(aventail.Scene:selected(), "out/cube.obj", "SubstancePainter")
        "#,
        full(&root),
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", mistakes(&ran.lines));
    assert!(
        std::fs::read(root.join("out/all.glb"))
            .unwrap()
            .starts_with(b"glTF")
    );
    assert!(root.join("out/cube.obj").exists());
    assert!(root.join("out/cube.mtl").exists(), "the file beside it too");
    let (mut snap, _) = snapshot();
    snap.access = full(&root);
    let ran = run_on(
        "export.lsc",
        r#"fetch "aventail/scene":Scene;
fetch "aventail/project":[Project, Preset];
Project:exportShapes(Scene:selected(), "lust.stl", Preset.Printing);
end
"#,
        snap,
        Limits::default(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", mistakes(&ran.lines));
    assert!(root.join("lust.stl").exists());
}

#[test]
fn an_export_with_the_wrong_ending_or_no_permission_is_refused() {
    let root = folder("export-refused");
    let ran = run_with_access(
        r#"aventail.Project:exportShapes(aventail.Scene:shapes(), "all.obj", aventail.Preset.Godot)"#,
        full(&root),
    );
    let mistake = mistakes(&ran.lines).remove(0).message;
    assert!(mistake.contains(".glb"), "{mistake}");
    let ran = run_with_access(
        r#"aventail.Project:exportShapes(aventail.Scene:shapes(), "all.glb", aventail.Preset.Godot)"#,
        read_only(&root),
    );
    let mistake = mistakes(&ran.lines).remove(0).message;
    assert!(mistake.contains("isn't allowed to write"), "{mistake}");
    assert!(!root.join("all.glb").exists());
}

#[test]
fn fuzzed_paths_never_panic_or_escape() {
    let root = folder("fuzz");
    let canonical = std::fs::canonicalize(&root).unwrap();
    let pieces = [
        "a", "..", ".", "/", "\\", ":", "C:", "\0", "~", "con", "é", " ", "*", "b.txt", "//",
    ];
    let mut state = 7u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize
    };
    for _ in 0..3_000 {
        let given: String = (0..next() % 8)
            .map(|_| pieces[next() % pieces.len()])
            .collect();
        if let Ok(path) = place(&root, &given) {
            assert!(path.starts_with(&canonical), "{given:?} → {path:?}");
        }
    }
}
