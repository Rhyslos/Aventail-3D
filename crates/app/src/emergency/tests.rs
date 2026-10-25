//! Tests for the emergency copy: writing it, recovery finding it, and a
//! held or broken lock never holding the hook up.

use super::*;
use fileio::autosave::Session;

/// A fresh, empty folder for one test.
fn root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("app-emergency-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// A document with a cube in it.
fn cube() -> Document {
    let mut doc = Document::new();
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    doc
}

#[test]
fn a_crash_writes_the_snapshot_and_the_next_start_finds_it() {
    let _one = crate::one_at_a_time();
    let root = root("write");
    let session = Session::start(&root).unwrap();
    session.write(b"older backup", 5).unwrap();
    let emergency = Emergency::new();
    emergency.set_folder(Some(session.folder().to_path_buf()));
    emergency.keep(Some(cube()));
    // What the panic hook does, without a real panic.
    let copy = emergency.write_now(HOOK_WAIT).expect("written");
    assert!(copy.starts_with(session.folder()));
    // The app goes: its lock is let go, its folder stays.
    drop(session);
    let found = autosave::leftovers(&root);
    assert_eq!(found.len(), 1);
    let newest = &found[0].copies[0];
    assert_eq!(newest.path, copy);
    assert!(newest.emergency);
    assert_eq!(found[0].copies.len(), 2, "the older backup is kept");
    let reopened = fileio::project::open(&copy).unwrap();
    assert_eq!(
        reopened.scene().children(None).len(),
        1,
        "the cube is in it"
    );
    // The hook may run again later (another panic): it writes again.
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn nothing_unsaved_or_nowhere_to_put_it_writes_nothing() {
    let _one = crate::one_at_a_time();
    let root = root("nothing");
    let session = Session::start(&root).unwrap();
    let emergency = Emergency::new();
    // No folder yet (the app is starting): nothing.
    emergency.keep(Some(cube()));
    assert_eq!(emergency.write_now(HOOK_WAIT), None);
    // A folder, but nothing unsaved: nothing.
    emergency.set_folder(Some(session.folder().to_path_buf()));
    emergency.keep(None);
    assert_eq!(emergency.write_now(HOOK_WAIT), None);
    // After a normal close: nothing.
    emergency.keep(Some(cube()));
    emergency.forget();
    assert_eq!(emergency.write_now(HOOK_WAIT), None);
    // A folder that has gone: nothing, and no crash.
    emergency.set_folder(Some(root.join("gone")));
    emergency.keep(Some(cube()));
    assert_eq!(emergency.write_now(HOOK_WAIT), None);
    session.end().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_held_lock_does_not_hold_the_hook_up() {
    let _one = crate::one_at_a_time();
    let root = root("held");
    let session = Session::start(&root).unwrap();
    let emergency = Emergency::new();
    emergency.set_folder(Some(session.folder().to_path_buf()));
    emergency.keep(Some(cube()));
    let held = emergency.shared.slot.lock().unwrap();
    let start = Instant::now();
    assert_eq!(emergency.write_now(HOOK_WAIT), None);
    assert!(start.elapsed() < Duration::from_secs(2), "gave up quickly");
    drop(held);
    // Once let go, it works, and the failed try left nothing stuck.
    assert!(emergency.write_now(HOOK_WAIT).is_some());
    session.end().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_broken_lock_still_gives_up_its_snapshot() {
    let _one = crate::one_at_a_time();
    let root = root("poisoned");
    let session = Session::start(&root).unwrap();
    let emergency = Emergency::new();
    emergency.set_folder(Some(session.folder().to_path_buf()));
    emergency.keep(Some(cube()));
    // A thread panics while holding the lock, which breaks ("poisons") it.
    let breaker = emergency.clone();
    let _ = std::thread::spawn(move || {
        let _held = breaker.shared.slot.lock().unwrap();
        panic!("broken on purpose");
    })
    .join();
    assert!(emergency.shared.slot.is_poisoned());
    assert!(emergency.write_now(HOOK_WAIT).is_some());
    // Keeping and forgetting work on a broken lock too.
    emergency.keep(None);
    emergency.forget();
    session.end().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn one_copy_at_a_time() {
    let _one = crate::one_at_a_time();
    let root = root("once");
    let session = Session::start(&root).unwrap();
    let emergency = Emergency::new();
    emergency.set_folder(Some(session.folder().to_path_buf()));
    emergency.keep(Some(cube()));
    // A copy being written (a panic in the writing itself runs the hook
    // again): the second doesn't start another.
    emergency.shared.writing.store(true, Ordering::SeqCst);
    assert_eq!(emergency.write_now(HOOK_WAIT), None);
    emergency.shared.writing.store(false, Ordering::SeqCst);
    assert!(emergency.write_now(HOOK_WAIT).is_some());
    assert!(!emergency.shared.writing.load(Ordering::SeqCst), "done");
    // The hook itself (without a panic) writes and says so.
    emergency.on_panic();
    session.end().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn snapshots_follow_changes_at_most_twice_a_second_and_clear_on_save() {
    let _one = crate::one_at_a_time();
    let emergency = Emergency::new();
    let mut snapshots = Snapshots::new(emergency.clone());
    let kept = || lock(&emergency.shared.slot).document.clone();
    let start = Instant::now();
    let mut doc = cube();
    // Unsaved work: kept at once.
    assert_eq!(snapshots.update(&doc, true, start), None);
    let first = kept().expect("kept");
    assert_eq!(first.revision(), 0, "a fresh copy, without the history");
    assert_eq!(first.scene().children(None).len(), 1);
    // Nothing changed: the same snapshot.
    assert_eq!(snapshots.update(&doc, true, start), None);
    assert!(Arc::ptr_eq(&first, &kept().unwrap()));
    // A change soon after: due a little later, and the app is told when.
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let soon = start + Duration::from_millis(100);
    assert_eq!(
        snapshots.update(&doc, true, soon),
        Some(SNAPSHOT_EVERY - Duration::from_millis(100))
    );
    assert_eq!(kept().unwrap().scene().children(None).len(), 1);
    // When it is due: taken.
    assert_eq!(snapshots.update(&doc, true, start + SNAPSHOT_EVERY), None);
    assert_eq!(kept().unwrap().scene().children(None).len(), 2);
    // Saved: nothing to rescue.
    assert_eq!(snapshots.update(&doc, false, start + SNAPSHOT_EVERY), None);
    assert!(kept().is_none());
    // The clock going backwards doesn't upset it.
    let _ = doc.undo();
    assert!(snapshots.update(&doc, true, start).is_some());
    assert!(
        snapshots.emergency().write_now(HOOK_WAIT).is_none(),
        "no folder"
    );
}

/// The child half of [`a_real_panic_saves_the_copy`]: installs the hook,
/// keeps a snapshot and panics. Does nothing unless run by that test.
#[test]
#[ignore = "run as a separate process by a_real_panic_saves_the_copy"]
fn panicking_child() {
    let _one = crate::one_at_a_time();
    let Some(folder) = std::env::var_os("EMERGENCY_CHILD_FOLDER") else {
        return;
    };
    let emergency = Emergency::new();
    emergency.install();
    emergency.set_folder(Some(PathBuf::from(folder)));
    emergency.keep(Some(cube()));
    panic!("a crash, on purpose");
}

#[test]
fn a_real_panic_saves_the_copy() {
    let _one = crate::one_at_a_time();
    // A panic hook is for the whole program, so the real one is tried in
    // a separate process: this test program again, running only the child.
    let root = root("real");
    let session = Session::start(&root).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "emergency::tests::panicking_child",
            "--ignored",
            "--nocapture",
        ])
        .env("EMERGENCY_CHILD_FOLDER", session.folder())
        .output()
        .unwrap();
    assert!(!output.status.success(), "the child panicked");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(
        said.contains("a crash, on purpose"),
        "the usual message: {said}"
    );
    assert!(said.contains("emergency copy"), "where it went: {said}");
    drop(session);
    let found = autosave::leftovers(&root);
    assert_eq!(found.len(), 1);
    assert!(found[0].copies[0].emergency);
    let reopened = fileio::project::open(&found[0].copies[0].path).unwrap();
    assert_eq!(reopened.scene().children(None).len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}
