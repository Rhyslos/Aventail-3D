//! The Scripts panel's parts without a window: console history, the
//! lines Ctrl + Enter takes, colouring and underlines, and the editor's
//! saving, checking and reloading.

use super::code_view::{layout_job, underlines};
use super::console::MAX_HISTORY;
use super::editor::{CHECK_PAUSE, LOOK_EVERY, Notice, SAVE_PAUSE, lines_of};
use super::*;
use script::Language;
use std::time::Duration;

fn names() -> Names {
    Names {
        folder: "aventail".into(),
        made_by: "The app 0.0.1".into(),
    }
}

/// A fresh, empty folder for one test.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scripts-panel-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ------------------------------------------------------------ history ---

#[test]
fn up_and_down_go_through_earlier_lines_and_back_to_the_draft() {
    let mut h = History::default();
    assert_eq!(h.back("typing"), None, "nothing yet");
    h.push("one");
    h.push("two");
    h.push("two");
    h.push("   ");
    assert_eq!(h.lines(), ["one", "two"], "no repeats or empty lines");
    assert_eq!(h.back("draft").as_deref(), Some("two"));
    assert_eq!(h.back("two").as_deref(), Some("one"));
    assert_eq!(h.back("one"), None, "the oldest is the end");
    assert_eq!(h.forward().as_deref(), Some("two"));
    assert_eq!(h.forward().as_deref(), Some("draft"), "past the newest");
    assert_eq!(h.forward(), None);
    // Running a line starts again from the newest.
    h.back("");
    h.push("three");
    assert_eq!(h.back("").as_deref(), Some("three"));
}

#[test]
fn history_keeps_the_newest_hundred() {
    let mut h = History::default();
    for n in 0..MAX_HISTORY + 20 {
        h.push(&n.to_string());
    }
    assert_eq!(h.lines().len(), MAX_HISTORY);
    assert_eq!(h.lines()[0], "20");
}

#[test]
fn fuzzed_history_never_breaks() {
    let mut seed: u64 = 0xc0_501e;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut h = History::default();
    for _ in 0..20_000 {
        match next() % 4 {
            0 => h.push(&(next() % 150).to_string()),
            1 => {
                if let Some(line) = h.back("d") {
                    assert!(h.lines().contains(&line));
                }
            }
            2 => {
                if let Some(line) = h.forward() {
                    assert!(line == "d" || line.is_empty() || h.lines().contains(&line));
                }
            }
            _ => h.push(""),
        }
        assert!(h.lines().len() <= MAX_HISTORY);
        assert!(h.lines().windows(2).all(|w| w[0] != w[1]));
    }
}

// ------------------------------------------------ lines Ctrl + Enter takes

#[test]
fn the_selected_lines_are_found_from_the_cursor() {
    let text = "a\nbb\nccc\n";
    assert_eq!(lines_of(text, 3, 3), None, "nothing selected");
    assert_eq!(lines_of(text, 2, 4), Some((2, 2)));
    assert_eq!(lines_of(text, 4, 2), Some((2, 2)), "either way round");
    assert_eq!(lines_of(text, 0, 7), Some((1, 3)));
    // Ending at a line's very start doesn't take that line.
    assert_eq!(lines_of(text, 2, 5), Some((2, 2)));
    assert_eq!(lines_of(text, 0, 100), Some((1, 3)));
    assert_eq!(lines_of("", 0, 1), Some((1, 1)));
}

// ------------------------------------------------- colours and underlines

#[test]
fn mistakes_are_underlined_where_they_point() {
    let text = "first\n    bad word here\nlast";
    let at = |line, column| Problem {
        file: "a.lsc".into(),
        line,
        column,
        message: String::new(),
    };
    let marked = |p: Problem| -> Vec<&str> {
        underlines(text, "a.lsc", &[p])
            .into_iter()
            .map(|r| &text[r])
            .collect()
    };
    assert_eq!(marked(at(2, 9)), ["word"]);
    assert_eq!(
        marked(at(2, 0)),
        ["bad word here"],
        "a whole line, without its indent"
    );
    assert_eq!(marked(at(2, 8)), [" "], "a space is still shown");
    assert_eq!(
        marked(at(2, 99)),
        ["e"],
        "past a line's end: its last character"
    );
    assert_eq!(marked(at(3, 1)), ["last"]);
    let quoted = "x = \"some text\";\ny = \"open";
    let mark = |line, column| -> Vec<&str> {
        underlines(
            quoted,
            "a.lsc",
            &[Problem {
                file: "a.lsc".into(),
                line,
                column,
                message: String::new(),
            }],
        )
        .into_iter()
        .map(|r| &quoted[r])
        .collect()
    };
    assert_eq!(mark(1, 5), ["\"some text\""], "text in quotes, whole");
    assert_eq!(mark(2, 5), ["\"open"], "unfinished, to the line's end");
    assert!(marked(at(9, 1)).is_empty(), "past the text");
    assert!(marked(at(0, 0)).is_empty(), "about no line");
    let mut other = at(1, 1);
    other.file = "aventail/scene.lsc".into();
    assert!(marked(other).is_empty(), "another file's");
    assert!(underlines("", "a.lsc", &[at(1, 1)]).is_empty());
}

#[test]
fn colouring_covers_the_whole_text_in_order() {
    let text = "-- name: x\nloop (Shape s in Scene:selected()) {\n    é = \"t\";\n}\nend\n";
    let underlined = underlines(
        text,
        "a.lsc",
        &[Problem {
            file: "a.lsc".into(),
            line: 2,
            column: 7,
            message: String::new(),
        }],
    );
    let job = layout_job(
        text,
        Language::LustScript,
        &underlined,
        egui::FontId::monospace(12.0),
    );
    let range = |s: &egui::text::LayoutSection| -> std::ops::Range<usize> {
        s.byte_range.start.into()..s.byte_range.end.into()
    };
    let mut at = 0;
    for section in &job.sections {
        assert_eq!(range(section).start, at);
        at = range(section).end;
    }
    assert_eq!(at, text.len());
    let shape = text.find("Shape").unwrap();
    let marked: Vec<_> = job
        .sections
        .iter()
        .filter(|s| s.format.underline.width > 0.0)
        .map(|s| &text[range(s)])
        .collect();
    assert_eq!(marked, ["Shape"]);
    let colour_at = |byte: usize| {
        job.sections
            .iter()
            .find(|s| range(s).contains(&byte))
            .unwrap()
            .format
            .color
    };
    assert_eq!(colour_at(shape), crate::theme::code::TYPE_NAME);
    assert_eq!(colour_at(0), crate::theme::code::COMMENT);
    assert_eq!(
        colour_at(text.find("loop").unwrap()),
        crate::theme::code::KEYWORD
    );
    assert_eq!(
        colour_at(text.find("\"t\"").unwrap()),
        crate::theme::code::TEXT
    );
}

#[test]
fn colouring_a_ten_thousand_line_script_is_quick() {
    let line = "    shape:bevel(shape:hardEdges(), 0.2, 2); -- round \"it\" off\n";
    let text = line.repeat(10_000);
    let problems: Vec<Problem> = (1..=50)
        .map(|n| Problem {
            file: "long.lsc".into(),
            line: n * 100,
            column: 5,
            message: String::new(),
        })
        .collect();
    let start = std::time::Instant::now();
    let underlined = underlines(&text, "long.lsc", &problems);
    let job = layout_job(
        &text,
        Language::LustScript,
        &underlined,
        egui::FontId::monospace(12.0),
    );
    let took = start.elapsed();
    assert_eq!(underlined.len(), 50);
    assert!(job.sections.len() > 100_000);
    // Done once per change of the text, never per frame.
    assert!(took < Duration::from_secs(5), "{took:?}");
}

#[test]
fn console_mistakes_are_placed_among_the_lines_typed() {
    let problem = |file: &str, line| Problem {
        file: file.into(),
        line,
        column: 3,
        message: String::new(),
    };
    let console = Some(("console.lsc".to_string(), 2));
    assert_eq!(placed(problem("console.lsc", 4), &console).line, 2);
    let fetches = placed(problem("console.lsc", 2), &console);
    assert_eq!((fetches.line, fetches.column), (0, 0));
    assert_eq!(placed(problem("other.lsc", 4), &console).line, 4);
    assert_eq!(placed(problem("console.lsc", 4), &None).line, 4);
}

// ------------------------------------------------------------ the editor

/// Ticks `editor` until its check has come back (at most 10 s).
fn checked(editor: &mut Editor, now: Instant) {
    let start = std::time::Instant::now();
    editor.tick(now + CHECK_PAUSE, &names());
    while editor.is_checking() {
        assert!(start.elapsed() < Duration::from_secs(10), "never checked");
        std::thread::sleep(Duration::from_millis(5));
        editor.tick(now + CHECK_PAUSE, &names());
    }
}

#[test]
fn typing_is_checked_after_a_pause_and_saved_after_a_longer_one() {
    let dir = folder("typing");
    let path = dir.join("t.lua");
    std::fs::write(&path, "print(1)\n").unwrap();
    let now = Instant::now();
    let mut editor = Editor::open(&path, now).unwrap();
    checked(&mut editor, now);
    assert!(editor.problems.is_empty());
    editor.text = "print(1\n".into();
    editor.typed(now);
    // Not yet: the pause hasn't passed.
    editor.tick(now + CHECK_PAUSE / 2, &names());
    assert!(!editor.is_checking());
    checked(&mut editor, now);
    assert_eq!(editor.problems.len(), 1);
    assert_eq!(editor.problems[0].file, "t.lua");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "print(1)\n",
        "not saved yet"
    );
    assert!(editor.tick(now + SAVE_PAUSE, &names()).is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "print(1\n");
    assert!(!editor.unsaved());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_check_while_typing_never_runs_the_script() {
    let dir = folder("never-runs");
    let path = dir.join("loop.lua");
    // Run, this would never end, and would print first.
    std::fs::write(&path, "print('ran')\nwhile true do end\n").unwrap();
    let now = Instant::now();
    let mut editor = Editor::open(&path, now).unwrap();
    let start = std::time::Instant::now();
    checked(&mut editor, now);
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(editor.problems.is_empty());
    // The same for LustScript.
    let path = dir.join("loop.lsc");
    std::fs::write(&path, "-- api: 1\nloop {\n}\nend\n").unwrap();
    let mut editor = Editor::open(&path, now).unwrap();
    checked(&mut editor, now);
    assert!(editor.problems.is_empty(), "{:?}", editor.problems);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_file_changed_outside_is_loaded_again_once() {
    let dir = folder("outside");
    let path = dir.join("o.lua");
    std::fs::write(&path, "print(1)\n").unwrap();
    let now = Instant::now();
    let mut editor = Editor::open(&path, now).unwrap();
    std::fs::write(&path, "print(2) -- from another program\n").unwrap();
    let notices = editor.tick(now + LOOK_EVERY, &names());
    assert_eq!(editor.text, "print(2) -- from another program\n");
    assert!(matches!(&notices[..], [Notice::Note(n)] if n.contains("loaded again")));
    // Looked at again, nothing new: no second load.
    assert!(editor.tick(now + LOOK_EVERY * 2, &names()).is_empty());
    assert!(editor.tick(now + LOOK_EVERY * 3, &names()).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_change_outside_while_typing_asks_which_to_keep() {
    let dir = folder("both");
    let path = dir.join("b.lua");
    std::fs::write(&path, "print(1)\n").unwrap();
    let now = Instant::now();
    let mut editor = Editor::open(&path, now).unwrap();
    editor.text = "print('mine')\n".into();
    editor.typed(now);
    std::fs::write(&path, "print('theirs')\n").unwrap();
    // Looked at before the typing is saved.
    editor.tick(now + LOOK_EVERY, &names());
    assert!(editor.changed_outside);
    assert_eq!(editor.text, "print('mine')\n", "the typing is kept");
    // While asking, nothing is written over theirs.
    editor.tick(now + SAVE_PAUSE * 3, &names());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "print('theirs')\n");
    assert_eq!(editor.keep_mine(), None);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "print('mine')\n");
    // And the other way.
    editor.text = "print('mine again')\n".into();
    editor.typed(now);
    std::fs::write(&path, "print('theirs again, longer')\n").unwrap();
    editor.tick(now + LOOK_EVERY * 5, &names());
    assert!(editor.changed_outside);
    editor.take_theirs(now);
    assert_eq!(editor.text, "print('theirs again, longer')\n");
    assert!(!editor.unsaved());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn typing_is_saved_when_the_editor_closes_and_a_removed_file_is_written_again() {
    let dir = folder("closing");
    let path = dir.join("c.lua");
    std::fs::write(&path, "print(1)\n").unwrap();
    let now = Instant::now();
    let mut editor = Editor::open(&path, now).unwrap();
    std::fs::remove_file(&path).unwrap();
    let notices = editor.tick(now + LOOK_EVERY, &names());
    assert!(matches!(&notices[..], [Notice::Note(n)] if n.contains("removed")));
    editor.text = "print(3)\n".into();
    editor.typed(now);
    drop(editor);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "print(3)\n");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_save_that_fails_says_so_once() {
    let dir = folder("unsaveable");
    let path = dir.join("u.lua");
    std::fs::write(&path, "print(1)\n").unwrap();
    let now = Instant::now();
    let mut editor = Editor::open(&path, now).unwrap();
    // Its folder made a file: it can't be written.
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::write(&dir, "in the way").unwrap();
    editor.text = "print(2)\n".into();
    editor.typed(now);
    assert!(matches!(editor.save_now(), Some(Notice::Trouble(_))));
    editor.typed(now);
    assert_eq!(editor.save_now(), None, "said once");
    drop(editor);
    let _ = std::fs::remove_file(&dir);
}

#[test]
fn opening_what_isnt_a_script_says_why() {
    let dir = folder("not-a-script");
    let path = dir.join("notes.txt");
    std::fs::write(&path, "hello").unwrap();
    assert!(Editor::open(&path, Instant::now()).is_err());
    assert!(Editor::open(&dir.join("missing.lua"), Instant::now()).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[allow(clippy::single_range_in_vec_init)]
fn lines_are_split_as_egui_splits_them_and_pieces_never_cross_them() {
    use super::code_view::{paragraph_ranges, pieces};
    let cases: [(&str, &[std::ops::Range<usize>]); 8] = [
        ("", &[0..0]),
        ("a", &[0..1]),
        ("a\n", &[0..2]),
        ("a\nb", &[0..1, 2..3]),
        ("a\nb\n", &[0..1, 2..4]),
        ("\n", &[0..1]),
        ("\n\n", &[0..0, 1..2]),
        ("a\n\nb", &[0..1, 2..2, 3..4]),
    ];
    for (text, want) in cases {
        assert_eq!(paragraph_ranges(text), want, "{text:?}");
    }
    let text = "--[[ x\ny ]] z\n\"s\"\n\né\n😀";
    let all = pieces(text, Language::Lua, &[1..9]);
    let mut at = 0;
    for (range, _, _) in &all {
        assert_eq!(range.start, at);
        at = range.end;
        let inside = &text[range.clone()];
        assert!(inside == "\n" || !inside.contains('\n'), "{inside:?}");
    }
    assert_eq!(at, text.len());
}
