use super::*;

#[test]
fn every_call_in_the_table_has_a_body() {
    // A script calling each one with nothing would fail on its values,
    // never on "has no body".
    let mut text = String::from("-- api: 1\nlocal s = aventail.Scene:selected()[1]\n");
    for call in CALLS {
        let on = match crate::describe::owner(call.owner).map(|o| o.kind) {
            Some(crate::describe::OwnerKind::Thing) => "s".to_string(),
            _ => format!("aventail.{}", call.owner),
        };
        let args = match (call.owner, call.name) {
            ("Shape", "points" | "polygons") => "0, 1",
            ("Selection", "mode") => "",
            ("Selection", _) => "s",
            ("Scene", "find") => "\"Cube\"",
            ("Script", "say" | "warn") => "\"x\"",
            ("Script", "progress") => "0.5, \"x\"",
            _ => "",
        };
        text.push_str(&format!("{on}:{}({args})\n", call.name));
    }
    let (lines, ending) = crate::run::tests::run("every.lua", &text);
    assert_eq!(ending, crate::Ending::Finished, "{lines:?}");
}

#[test]
fn polygons_and_points_are_read_in_chunks_that_stop_at_the_end() {
    let (lines, ending) = crate::run::tests::run(
        "chunks.lua",
        "local c = aventail.Scene:find(\"Cube\")\nprint(#c:points(6, 10), #c:points(100, 10), #c:polygons(0, 0), #aventail.Scene:find(\"Group\"):points(0, 5))",
    );
    assert_eq!(ending, crate::Ending::Finished, "{lines:?}");
    assert_eq!(crate::run::tests::said(&lines), ["2\t0\t0\t0"]);
}
