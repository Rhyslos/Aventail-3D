//! Running part of a script: selected lines and the console line.

use super::*;
use crate::run::tests::{mistakes, names, run_on, said, snapshot};
use crate::{Ending, Limits};

const LUST: &str = "-- name: Two lines
-- api: 1
fetch \"aventail/script\":Script;

Script:say(\"one\");
Script:say(\"two\");
Script:say(\"three\");
end
";

fn file(name: &str, text: &str) -> ScriptFile {
    ScriptFile {
        name: name.into(),
        text: text.into(),
    }
}

fn run(file: &ScriptFile) -> (Vec<String>, Ending) {
    let ran = run_on(&file.name, &file.text, snapshot().0, Limits::default());
    (said(&ran.lines), ran.ending)
}

#[test]
fn only_the_selected_lines_run_with_the_header_and_fetches() {
    let part = only_lines(&file("two.lsc", LUST), 6, 6);
    assert_eq!(
        part.text,
        "-- name: Two lines\n-- api: 1\nfetch \"aventail/script\":Script;\n\n\nScript:say(\"two\");\nend\n"
    );
    assert_eq!(run(&part), (vec!["two".to_string()], Ending::Finished));
    // The whole file chosen is the file (its own `end` kept, no second).
    let all = only_lines(&file("two.lsc", LUST), 1, 8);
    assert_eq!(all.text, LUST);
    assert_eq!(run(&all).0, ["one", "two", "three"]);
}

#[test]
fn a_mistake_in_selected_lines_is_at_its_own_line() {
    let text = "-- api: 1\nprint(\"fine\")\nlocal x = = 1\nprint(\"after\")\n";
    let part = only_lines(&file("bad.lua", text), 3, 3);
    let ran = run_on("bad.lua", &part.text, snapshot().0, Limits::default());
    assert_eq!(ran.ending, Ending::Failed);
    assert_eq!(mistakes(&ran.lines)[0].line, 3, "{:?}", ran.lines);
    assert!(said(&ran.lines).is_empty());
}

#[test]
fn selected_lua_lines_run_alone() {
    let text = "-- api: 1\nprint(\"a\")\nprint(\"b\")\n";
    let part = only_lines(&file("a.lua", text), 3, 3);
    assert_eq!(part.text, "-- api: 1\n\nprint(\"b\")\n");
    assert_eq!(run(&part), (vec!["b".to_string()], Ending::Finished));
    // Lines past the end, or backwards, choose nothing but the header.
    assert_eq!(only_lines(&file("a.lua", text), 9, 12).text, "-- api: 1\n");
    assert_eq!(only_lines(&file("a.lua", text), 3, 2).text, "-- api: 1\n");
    assert_eq!(only_lines(&file("a.lua", ""), 1, 1).text, "\n");
}

#[test]
fn the_console_line_reaches_every_call_in_both_languages() {
    let (lust, before) = console_script(
        "int n = #Scene:shapes(); Script:say(n.text());",
        Language::LustScript,
        &names(),
    );
    assert_eq!(before, 2);
    let ran = run_on(&lust.name, &lust.text, snapshot().0, Limits::default());
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    // Nothing about the scripting version.
    assert!(ran.lines.iter().all(|l| !matches!(l, crate::Line::Note(_))));
    assert_eq!(said(&ran.lines), ["3"]);
    let (lua, before) = console_script("print(#aventail.Scene:shapes())", Language::Lua, &names());
    assert_eq!(before, 1);
    assert_eq!(run(&lua), (vec!["3".to_string()], Ending::Finished));
}

#[test]
fn console_lines_change_the_scene_and_their_mistakes_are_placed() {
    let (lust, _) = console_script("Shape s = Scene:add(Primitive.Cube);", Language::LustScript, &names());
    let ran = run_on(&lust.name, &lust.text, snapshot().0, Limits::default());
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    assert!(!ran.landing.unwrap().changes.is_empty());
    // A mistake on the second typed line is at the console's line 4: 2
    // put before it.
    let (lust, before) = console_script(
        "Script:say(\"a\");\nint n = \"text\";",
        Language::LustScript,
        &names(),
    );
    let ran = run_on(&lust.name, &lust.text, snapshot().0, Limits::default());
    assert_eq!(ran.ending, Ending::Failed);
    assert_eq!(mistakes(&ran.lines)[0].line - before, 2, "{:?}", ran.lines);
    // Typing `end` yourself is fine too.
    let (lust, _) = console_script("Script:say(\"x\");\nend", Language::LustScript, &names());
    assert_eq!(run(&lust), (vec!["x".to_string()], Ending::Finished));
}
