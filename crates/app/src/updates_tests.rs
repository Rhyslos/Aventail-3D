//! Tests for updates (decision 0169): reading GitHub's answer, who
//! updates, one look with a fake internet, the looking in the background
//! and the setup program's arguments.

use super::*;
use std::sync::{Arc, Mutex};

/// GitHub's answer for build `build`, its setup program `size` bytes.
fn answer(build: &str, size: u64) -> String {
    format!(
        r#"{{
            "tag_name": "{build}",
            "html_url": "https://github.com/Rhyslos/Aventail-3D/releases/tag/{build}",
            "assets": [
                {{ "name": "notes.txt", "size": 3, "browser_download_url": "https://example.com/notes" }},
                {{ "name": "{}", "size": {size}, "browser_download_url": "https://github.com/x/setup.exe" }}
            ]
        }}"#,
        installer_name()
    )
}

#[test]
fn build_numbers_are_whole_numbers_above_zero() {
    assert_eq!(parse_build("412"), Some(412));
    assert_eq!(parse_build(" 7\n"), Some(7));
    for bad in ["", "0", "-3", "4.1", "build-4", "99999999999"] {
        assert_eq!(parse_build(bad), None, "{bad:?}");
    }
}

#[test]
fn the_setup_programs_name_comes_from_branding() {
    assert_eq!(installer_name(), format!("{}-setup.exe", branding::APP_ID));
}

#[test]
fn githubs_answer_is_read() {
    let release = parse_release(&answer("build-412", 1000)).unwrap();
    assert_eq!(release.build, 412);
    assert_eq!(release.size, 1000);
    assert_eq!(release.installer_url, "https://github.com/x/setup.exe");
    assert!(release.page.ends_with("/build-412"));
}

#[test]
fn a_bad_answer_says_why() {
    assert!(parse_release("not json").is_err());
    assert!(parse_release("{}").unwrap_err().contains("no build number"));
    assert!(
        parse_release(&answer("v1.0", 5))
            .unwrap_err()
            .contains("no build number")
    );
    assert!(
        parse_release(&answer("build-3", 0))
            .unwrap_err()
            .contains("empty")
    );
    let without = r#"{"tag_name": "build-9", "assets": []}"#;
    assert!(
        parse_release(without)
            .unwrap_err()
            .contains("no setup program")
    );
    let plain_http = answer("build-9", 5).replace("https://github.com/x", "http://github.com/x");
    assert!(
        parse_release(&plain_http)
            .unwrap_err()
            .contains("download address")
    );
}

#[test]
fn random_answers_never_panic() {
    // A fuzz test: whatever comes back, reading it gives a release or a
    // reason.
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let pieces = [
        "{",
        "}",
        "[",
        "]",
        "\"tag_name\"",
        ":",
        ",",
        "\"build-",
        "12\"",
        "\"assets\"",
        "null",
        "\"size\"",
        "-1",
        "1e99",
        "\"name\"",
        "\"browser_download_url\"",
        "\"https://x\"",
    ];
    for _ in 0..2000 {
        let mut text = String::new();
        for _ in 0..(seed % 24) {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            text.push_str(pieces[(seed % pieces.len() as u64) as usize]);
        }
        let _ = parse_release(&text);
    }
}

#[test]
fn only_an_installed_numbered_windows_copy_with_updates_on_updates() {
    let dir = std::env::temp_dir().join(format!("updates-who-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let exe = dir.join("app.exe");
    std::fs::write(&exe, b"x").unwrap();
    assert!(why_not(Some(5), &exe, true, false).unwrap().contains("off"));
    assert!(
        why_not(Some(5), &exe, false, true)
            .unwrap()
            .contains("Windows")
    );
    assert!(why_not(None, &exe, true, true).unwrap().contains("by hand"));
    assert!(
        why_not(Some(5), &exe, true, true)
            .unwrap()
            .contains("isn't installed")
    );
    std::fs::write(dir.join("unins000.exe"), b"x").unwrap();
    assert_eq!(why_not(Some(5), &exe, true, true), None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A fake internet: GitHub's answer, and the setup program's bytes.
#[derive(Clone)]
struct Fake {
    answer: Result<String, String>,
    file: Result<Vec<u8>, String>,
    downloads: Arc<Mutex<usize>>,
    /// A newer answer to give from now on, if set.
    newest: Arc<Mutex<Option<String>>>,
}

impl Fake {
    fn new(answer: &str, file: &[u8]) -> Self {
        Self {
            answer: Ok(answer.to_string()),
            file: Ok(file.to_vec()),
            downloads: Arc::default(),
            newest: Arc::default(),
        }
    }
}

impl Fetch for Fake {
    fn text(&self, url: &str) -> Result<String, String> {
        assert_eq!(url, branding::RELEASES_API);
        match self.newest.lock().unwrap().clone() {
            Some(newer) => Ok(newer),
            None => self.answer.clone(),
        }
    }

    fn file(&self, _url: &str, to: &Path) -> Result<(), String> {
        *self.downloads.lock().unwrap() += 1;
        let bytes = self.file.clone()?;
        std::fs::write(to, bytes).map_err(|e| e.to_string())
    }
}

fn fresh_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("updates-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_newer_build_is_downloaded_once_and_older_ones_forgotten() {
    let folder = fresh_folder("newer");
    std::fs::create_dir_all(&folder).unwrap();
    let old = installer_path(&folder, 3);
    std::fs::write(&old, b"old").unwrap();
    let fake = Fake::new(&answer("build-12", 4), b"exe!");
    let found = look(&fake, 10, &folder);
    let Found::Ready { release, installer } = &found else {
        panic!("{found:?}");
    };
    assert_eq!(release.build, 12);
    assert_eq!(std::fs::read(installer).unwrap(), b"exe!");
    assert!(!old.exists(), "older setup programs go");
    // A second look finds it whole and doesn't download it again.
    assert_eq!(look(&fake, 10, &folder), found);
    assert_eq!(*fake.downloads.lock().unwrap(), 1);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_same_or_an_older_build_is_up_to_date() {
    let folder = fresh_folder("same");
    let fake = Fake::new(&answer("build-12", 4), b"exe!");
    assert_eq!(look(&fake, 12, &folder), Found::UpToDate(12));
    assert_eq!(look(&fake, 20, &folder), Found::UpToDate(12));
    assert_eq!(*fake.downloads.lock().unwrap(), 0);
    assert!(!folder.exists(), "nothing written");
}

#[test]
fn a_cut_off_or_failed_download_is_never_kept() {
    let folder = fresh_folder("cut");
    let short = Fake::new(&answer("build-12", 100), b"only part");
    let found = look(&short, 10, &folder);
    assert!(
        matches!(&found, Found::Failed(why) if why.contains("cut off")),
        "{found:?}"
    );
    let failing = Fake {
        file: Err("connection reset".into()),
        ..Fake::new(&answer("build-12", 4), b"")
    };
    assert_eq!(
        look(&failing, 10, &folder),
        Found::Failed("connection reset".into())
    );
    let offline = Fake {
        answer: Err("Could not resolve host".into()),
        ..Fake::new("", b"")
    };
    assert_eq!(
        look(&offline, 10, &folder),
        Found::Failed("Could not resolve host".into())
    );
    let left: Vec<_> = std::fs::read_dir(&folder)
        .map(|d| d.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "nothing left behind: {left:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Polls `updater` until `done` holds for what it has said, or gives up.
fn wait_for(updater: &Updater, done: impl Fn(&[Found]) -> bool) -> Vec<Found> {
    let mut found = Vec::new();
    for _ in 0..400 {
        found.extend(updater.poll());
        if done(&found) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    found
}

fn quick(fake: &Fake, current: u32, folder: &Path) -> Updater {
    Updater::start_with(
        Box::new(fake.clone()),
        current,
        folder.to_path_buf(),
        Duration::from_millis(5),
        Duration::from_millis(5),
    )
}

#[test]
fn the_first_look_asks_before_downloading() {
    // The splash screen asks first (decision 0173): the first look only
    // says a newer build is there, and downloads nothing.
    let folder = fresh_folder("ask");
    let fake = Fake::new(&answer("build-12", 4), b"exe!");
    let updater = Updater::start_with(
        Box::new(fake.clone()),
        10,
        folder.clone(),
        Duration::from_millis(5),
        Duration::from_secs(3600),
    );
    let found = wait_for(&updater, |f| !f.is_empty());
    let [Found::Newer(release)] = found.as_slice() else {
        panic!("{found:?}");
    };
    assert_eq!(release.build, 12);
    assert_eq!(*fake.downloads.lock().unwrap(), 0, "nothing downloaded yet");
    // Update now: it downloads, says how far, then that it's ready.
    updater.download(release.clone());
    let found = wait_for(&updater, |f| {
        f.iter().any(|x| matches!(x, Found::Ready { .. }))
    });
    assert!(
        matches!(
            found.as_slice(),
            [Found::Downloading { .. }, Found::Ready { .. }]
        ),
        "{found:?}"
    );
    // It stops once ready: no more looks.
    std::thread::sleep(Duration::from_millis(50));
    assert!(updater.poll().is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn later_looks_download_at_once() {
    // Declined on the splash screen (quiet up to 0 here, so not that):
    // the first look asks; with no answer, the next look downloads.
    let folder = fresh_folder("later");
    let fake = Fake::new(&answer("build-12", 4), b"exe!");
    let updater = quick(&fake, 10, &folder);
    let found = wait_for(&updater, |f| {
        f.iter().any(|x| matches!(x, Found::Ready { .. }))
    });
    assert!(
        matches!(
            found.as_slice(),
            [
                Found::Newer(_),
                ..,
                Found::Downloading { .. },
                Found::Ready { .. }
            ]
        ),
        "{found:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_skipped_build_is_never_offered_but_a_newer_one_is() {
    let folder = fresh_folder("skip");
    let fake = Fake::new(&answer("build-12", 4), b"exe!");
    let updater = quick(&fake, 10, &folder);
    updater.set_quiet(12);
    let found = wait_for(&updater, |f| f.len() >= 3);
    assert!(
        found.iter().all(|f| matches!(f, Found::Skipped(12))),
        "{found:?}"
    );
    assert_eq!(*fake.downloads.lock().unwrap(), 0);
    // Build 13 comes out: offered, though 12 is skipped.
    *fake.newest.lock().unwrap() = Some(answer("build-13", 4));
    let found = wait_for(&updater, |f| {
        f.iter()
            .any(|x| matches!(x, Found::Ready { release, .. } if release.build == 13))
    });
    assert!(
        found
            .iter()
            .any(|x| matches!(x, Found::Ready { release, .. } if release.build == 13)),
        "{found:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_updater_keeps_looking_after_a_failure() {
    let folder = fresh_folder("again");
    let offline = Fake {
        answer: Err("offline".into()),
        ..Fake::new("", b"")
    };
    let updater = Updater::start_with(
        Box::new(offline),
        10,
        folder,
        Duration::from_millis(5),
        Duration::from_millis(5),
    );
    let mut failures = 0;
    for _ in 0..400 {
        failures += updater
            .poll()
            .iter()
            .filter(|f| matches!(f, Found::Failed(_)))
            .count();
        if failures >= 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(failures >= 2);
}

#[test]
fn the_setup_program_runs_quietly_and_reopens_only_after_restart_now() {
    let now = PendingInstall {
        installer: PathBuf::from("setup.exe"),
        reopen: true,
    };
    assert!(now.arguments().contains(&"/VERYSILENT"));
    assert!(!now.arguments().contains(&"/NOLAUNCH"));
    let later = PendingInstall {
        reopen: false,
        ..now
    };
    assert!(later.arguments().contains(&"/NOLAUNCH"));
}

#[test]
fn common_curl_complaints_are_said_plainly() {
    assert_eq!(
        curl_failure("curl: (22) The requested URL returned error: 404\n"),
        "no version has been published yet"
    );
    assert_eq!(
        curl_failure("curl: (6) Could not resolve host: api.github.com"),
        "no internet connection"
    );
    assert_eq!(
        curl_failure("curl: (22) The requested URL returned error: 403"),
        "GitHub asked to wait a while"
    );
    assert_eq!(
        curl_failure("curl: (28) Operation timed out"),
        "(28) Operation timed out"
    );
}

#[test]
fn curl_really_asks_where_it_can() {
    // With curl on this machine and no releases yet (or one), the real
    // address gives a release or a plain reason, never a panic.
    if std::process::Command::new("curl")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let result = Curl
        .text(branding::RELEASES_API)
        .and_then(|t| parse_release(&t));
    if let Err(why) = result {
        assert!(!why.is_empty());
    }
}

#[test]
fn a_download_says_when_it_starts_and_how_far_it_has_got() {
    let folder = fresh_folder("progress");
    let fake = Fake::new(&answer("build-12", 4), b"exe!");
    let told = std::sync::Mutex::new(Vec::new());
    let found = look_telling(&fake, 10, &folder, &|f| told.lock().unwrap().push(f));
    assert!(matches!(found, Found::Ready { .. }));
    let told = told.into_inner().unwrap();
    let [Found::Downloading { release, part }] = told.as_slice() else {
        panic!("{told:?}");
    };
    assert_eq!(release.build, 12);
    // The part file has become the setup program: nothing left at the
    // part's place, so 0; half a file reads as a half.
    assert_eq!(progress(part, release), 0.0);
    std::fs::write(part, b"ex").unwrap();
    assert!((progress(part, release) - 0.5).abs() < 1e-6);
    // Up to date: nothing downloads, nothing is told.
    let quiet = std::sync::Mutex::new(0);
    look_telling(&fake, 12, &folder, &|_| *quiet.lock().unwrap() += 1);
    assert_eq!(*quiet.lock().unwrap(), 0);
    let _ = std::fs::remove_dir_all(&folder);
}
