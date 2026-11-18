//! Running scripts end to end: normal, edge, failure and hostile scripts.

use super::*;
use crate::output::Line;
use scene::{Command, Document, ObjectKind, Primitive};
use std::time::{Duration, Instant};

pub(crate) fn names() -> Names {
    Names {
        folder: "aventail".into(),
        made_by: "The app 0.0.1".into(),
    }
}

/// A scene with a cube (selected) and a group holding a sphere, and the
/// cube's id.
pub(crate) fn snapshot() -> (Snapshot, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        Default::default(),
    ))))
    .unwrap();
    let cube = doc.scene().children(None)[0];
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::all()[3])))
        .unwrap();
    // New objects go in at the top.
    let sphere = doc.scene().children(None)[0];
    doc.execute(Command::Group(vec![sphere])).unwrap();
    let mut elements = mesh::ElementSelection::new();
    elements.insert(mesh::Element::Polygon(mesh::FaceId::from_raw(2)));
    let snapshot = Snapshot {
        scene: doc.scene().copy_with_results(),
        selection: vec![cube],
        elements: vec![(cube, elements)],
        mode: Mode::Polygons,
        project_name: "Crates".into(),
        project_folder: String::new(),
        unwrap: Default::default(),
        access: crate::Access::default(),
        moment: None,
        export: None,
    };
    (snapshot, cube)
}

/// What a run said, how it ended, and what it changed.
pub(crate) struct Ran {
    pub(crate) lines: Vec<Line>,
    pub(crate) ending: Ending,
    pub(crate) landing: Option<Landing>,
}

/// Runs `text` as the file `name` against `snapshot` with `limits`.
pub(crate) fn run_on(name: &str, text: &str, snapshot: Snapshot, limits: Limits) -> Ran {
    let file = ScriptFile {
        name: name.into(),
        text: text.into(),
    };
    let events = Run::start(file, snapshot, limits, names()).wait();
    let mut lines = Vec::new();
    let mut ending = None;
    let mut landing = None;
    for event in events {
        match event {
            Event::Line(line) => lines.push(line),
            Event::Ended(e) => ending = Some(e),
            Event::Land(l) => landing = Some(*l),
            Event::Progress(..) | Event::Cancelled(_) => {}
        }
    }
    Ran {
        lines,
        ending: ending.unwrap(),
        landing,
    }
}

/// Runs `text` as the file `name` with `limits`, and gives back what it
/// said and how it ended.
pub(crate) fn run_with(name: &str, text: &str, limits: Limits) -> (Vec<Line>, Ending) {
    let ran = run_on(name, text, snapshot().0, limits);
    (ran.lines, ran.ending)
}

pub(crate) fn run(name: &str, text: &str) -> (Vec<Line>, Ending) {
    run_with(name, text, Limits::default())
}

/// What the script said, in order.
pub(crate) fn said(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| match l {
            Line::Said(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// The mistakes in the output.
pub(crate) fn mistakes(lines: &[Line]) -> Vec<Problem> {
    lines
        .iter()
        .filter_map(|l| match l {
            Line::Mistake(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}

const LUST_READER: &str = "-- name: Read the scene
-- api: 1
fetch \"aventail/scene\":[Scene, Shape];
fetch \"aventail/script\":Script;

loop (Shape s in Scene:shapes()) {
    Script:say(s:name() + \" \" + s:pointCount().text() + \" \" + s:polygonCount().text());
}
end
";

const LUA_READER: &str = "-- name: Read the scene
-- api: 1
for _, s in ipairs(aventail.Scene:shapes()) do
    print(s:name() .. \" \" .. s:pointCount() .. \" \" .. s:polygonCount())
end
";

#[test]
fn a_script_reads_names_and_counts_in_lustscript_and_lua() {
    let (lines, ending) = run("read.lsc", LUST_READER);
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    let lust = said(&lines);
    assert_eq!(lust, ["Group 0 0", "Sphere 266 288", "Cube 8 6"]);
    let (lines, ending) = run("read.lua", LUA_READER);
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    assert_eq!(said(&lines), lust);
}

#[test]
fn every_reading_call_answers_in_lua() {
    let (lines, ending) = run(
        "all.lua",
        "-- api: 1
local A = aventail
local cube = A.Scene:selected()[1]
assert(cube == A.Scene:find(\"Cube\"), \"the same shape is the same value\")
assert(A.Scene:find(\"Nothing here\") == nil)
print(cube:name(), cube.id >= 0, cube:shown())
local p = cube:position()
print(p.x, p.y, p.z, cube:scale().x, cube:rotation().z)
print(cube:parent() == nil, #cube:children())
local sphere = A.Scene:find(\"Sphere\")
print(sphere:parent():name(), sphere:parent():children()[1] == sphere)
local pts = cube:points(0, 3)
print(#pts, pts.n, pts[1].id, typeof(pts[1].position))
local polys = cube:polygons(5, 10)
print(#polys, #polys[1].corners, polys[1].material, typeof(polys[1].facing))
print(#cube:hardEdges(), #cube:seams() >= 0, cube:materials()[1])
print(A.Selection:mode() == A.Mode.Polygons, A.Selection:mode())
print(#A.Selection:polygons(cube), A.Selection:polygons(cube)[1], #A.Selection:points(cube), #A.Selection:edges(cube))
print(A.Project:name(), A.Project:folder() == \"\", A.Script:api())
A.Script:warn(\"careful\")
A.Script:progress(0.5, \"half\")
",
    );
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    let said = said(&lines);
    assert_eq!(said[0], "Cube\ttrue\ttrue");
    assert_eq!(said[1], "0\t0\t0\t1\t0");
    assert_eq!(said[2], "true\t0");
    assert_eq!(said[3], "Group\ttrue");
    assert_eq!(said[4], "3\t3\t0\tvector");
    assert_eq!(said[5], "1\t4\tDefault\tvector");
    assert!(said[6].ends_with("\tDefault"), "{}", said[6]);
    assert_eq!(said[7], "true\tPolygons");
    assert_eq!(said[8], "1\t2\t0\t0");
    assert_eq!(said[9], "Crates\ttrue\t1");
    assert!(lines.contains(&Line::Warning("careful".into())));
}

/// Points and polygons are packed into bytes and made into tables in
/// Luau (for speed): read in chunks that don't line up with the count,
/// they must say exactly what the mesh holds, in order.
#[test]
fn chunks_of_points_and_polygons_say_what_the_mesh_holds() {
    let (snapshot, _) = snapshot();
    let sphere = snapshot
        .scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            snapshot
                .scene
                .object(*id)
                .is_some_and(|o| o.name == "Sphere")
        })
        .unwrap();
    let mesh = snapshot.scene.evaluated(sphere).unwrap().mesh;
    let materials = snapshot.scene.material_list(sphere);
    let mut expected = Vec::new();
    for (face, corners) in mesh.faces() {
        let ids: Vec<String> = corners.iter().map(|c| c.get().to_string()).collect();
        let f = mesh.face_normal(face);
        let slot = mesh::material::slot_of(&mesh, face) as usize;
        let material = materials
            .get(slot)
            .map_or(scene::Material::standard().name, |m| m.name.clone());
        expected.push(format!(
            "f {} {} {:.5} {:.5} {:.5} {material}",
            face.get(),
            ids.join(","),
            f64::from(f.x),
            f64::from(f.y),
            f64::from(f.z)
        ));
    }
    for (v, at) in mesh.vertices() {
        expected.push(format!(
            "p {} {:.5} {:.5} {:.5}",
            v.get(),
            f64::from(at.x),
            f64::from(at.y),
            f64::from(at.z)
        ));
    }
    let (lines, ending) = run(
        "chunks.lua",
        "-- api: 1
local sphere = aventail.Scene:find(\"Sphere\")
local fmt, concat = string.format, table.concat
local from = 0
while true do
    local chunk = sphere:polygons(from, 37)
    assert(chunk.n == #chunk)
    if chunk.n == 0 then break end
    for _, p in ipairs(chunk) do
        assert(p.corners.n == #p.corners)
        print(fmt(\"f %d %s %.5f %.5f %.5f %s\", p.id, concat(p.corners, \",\"), p.facing.x, p.facing.y, p.facing.z, p.material))
    end
    from += 37
end
from = 0
while from < sphere:pointCount() do
    for _, p in ipairs(sphere:points(from, 50)) do
        print(fmt(\"p %d %.5f %.5f %.5f\", p.id, p.position.x, p.position.y, p.position.z))
    end
    from += 50
end
local past = sphere:polygons(100000, 10)
print(past.n, #sphere:points(0, 0))
",
    );
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    let mut said = said(&lines);
    assert_eq!(said.pop().unwrap(), "0\t0");
    assert_eq!(said, expected);
}

#[test]
fn every_reading_call_answers_in_lustscript() {
    let (lines, ending) = run(
        "all.lsc",
        "-- api: 1
fetch \"aventail/scene\":[Scene, Shape, Point, Polygon, Edge];
fetch \"aventail/selection\":[Selection, Mode];
fetch \"aventail/project\":Project;
fetch \"aventail/script\":Script;

Shape cube = Scene:find(\"Cube\");
print(cube:name() + \" \" + cube:position().text() + \" \" + cube:scale().text());
Point array pts = cube:points(0, 2);
print((#pts).text() + \" \" + pts[1].id.text());
Polygon array polys = cube:polygons(0, 1);
print((#polys[0].corners).text() + \" \" + polys[0].material);
Edge array hard = cube:hardEdges();
is Selection:mode() == Mode.Polygons? then {
    print(\"polygons \" + (#Selection:polygons(cube)).text());
}
is cube:parent() == nothing? then {
    print(\"top\");
}
print(Project:name() + \" \" + Script:api().text());
Script:progress(1, \"done\");
end
",
    );
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    let said = said(&lines);
    assert_eq!(said[0], "Cube [0.0, 0.0, 0.0] [1.0, 1.0, 1.0]");
    assert_eq!(said[1], "2 1");
    assert_eq!(said[2], "4 Default");
    assert_eq!(said[3], "polygons 1");
    assert_eq!(said[4], "top");
    assert_eq!(said[5], "Crates 1");
}

#[test]
fn commands_run_once_with_their_settings_in_both_languages() {
    let (lines, ending) = run(
        "bevel.lsc",
        "-- name: Bevel hard edges
-- api: 1
-- count: segments
fetch \"aventail/scene\":[Scene, Shape];

component BevelHardEdges {
    has {
        float width = 0.2;
        int segments = 1;
    }

    method run() {
        loop (Shape shape in Scene:selected()) {
            print(shape:name() + \" \" + width.text() + \" \" + segments.text());
        }
    }
}

component NotACommand {
    has {
        int n = 0;
    }
}

pass BevelHardEdges, NotACommand;
end
",
    );
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    assert_eq!(said(&lines), ["Cube 0.2 1"]);
    let (lines, ending) = run(
        "bevel.lua",
        "-- api: 1
local Bevel = command(\"BevelHardEdges\", { width = 0.2, segments = 1 })
function Bevel:run()
    for _, shape in ipairs(aventail.Scene:selected()) do
        print(shape:name(), self.width, self.segments)
    end
end
command(\"NoRun\")
",
    );
    assert_eq!(ending, Ending::Finished, "{lines:?}");
    assert_eq!(said(&lines), ["Cube\t0.2\t1"]);
    assert!(
        lines
            .iter()
            .any(|l| matches!(l, Line::Note(n) if n.contains("NoRun"))),
        "{lines:?}"
    );
}

#[test]
fn empty_and_header_only_scripts_run_and_say_nothing() {
    for (name, text) in [
        ("empty.lua", ""),
        ("header.lua", "-- name: Just a header\n-- api: 1\n"),
        ("header.lsc", "-- name: Just a header\n-- api: 1\nend\n"),
    ] {
        let (lines, ending) = run(name, text);
        assert_eq!(ending, Ending::Finished, "{name}: {lines:?}");
        assert!(said(&lines).is_empty());
    }
    // Every Lust file ends with `end`: an empty one is a placed mistake.
    let (lines, ending) = run("empty.lsc", "");
    assert_eq!(ending, Ending::Failed);
    assert!(mistakes(&lines)[0].message.contains("end"), "{lines:?}");
}

#[test]
fn a_lustscript_without_end_and_a_lua_syntax_error_are_placed_mistakes() {
    let (lines, ending) = run("no_end.lsc", "-- api: 1\nprint(\"hi\");\n");
    assert_eq!(ending, Ending::Failed);
    assert!(said(&lines).is_empty(), "nothing ran: {lines:?}");
    assert_eq!(mistakes(&lines)[0].file, "no_end.lsc");
    let (lines, ending) = run(
        "broken.lsc",
        "-- api: 1\nfetch \"aventail/scene\":[Scene];\nint n = \"text\";\nend\n",
    );
    assert_eq!(ending, Ending::Failed);
    let found = mistakes(&lines);
    assert_eq!(
        (found[0].file.as_str(), found[0].line),
        ("broken.lsc", 3),
        "{found:?}"
    );
    let (lines, ending) = run("broken.lua", "-- api: 1\n\nlocal x = = 2\n");
    assert_eq!(ending, Ending::Failed);
    let found = mistakes(&lines);
    assert_eq!(
        (found[0].file.as_str(), found[0].line),
        ("broken.lua", 3),
        "{found:?}"
    );
}

#[test]
fn a_call_lust_doesnt_know_is_a_mistake_before_running() {
    let (lines, ending) = run(
        "typo.lsc",
        "-- api: 1\nfetch \"aventail/scene\":[Scene, Shape];\nprint(\"ran\");\nShape s = Scene:fnd(\"x\");\nend\n",
    );
    assert_eq!(ending, Ending::Failed);
    assert!(said(&lines).is_empty(), "nothing ran: {lines:?}");
    assert_eq!(mistakes(&lines)[0].line, 4);
}

#[test]
fn runtime_failures_point_at_the_scripts_own_line() {
    let (lines, ending) = run(
        "fail.lua",
        "-- api: 1\nprint(1)\n\nlocal t = nil\nprint(t.x)\n",
    );
    assert_eq!(ending, Ending::Failed);
    let found = mistakes(&lines);
    assert_eq!(
        (found[0].file.as_str(), found[0].line),
        ("fail.lua", 5),
        "{found:?}"
    );
    // A call given the wrong thing names the call and the value.
    let (lines, _) = run("wrong.lua", "-- api: 1\n\naventail.Scene.find(\"Cube\")\n");
    let found = mistakes(&lines);
    assert_eq!(found[0].line, 3, "{found:?}");
    assert!(
        found[0]
            .message
            .contains("aventail.Scene:find(name) wants text for name"),
        "{found:?}"
    );
    assert!(found[0].message.contains("colon"), "{found:?}");
    // In LustScript, a failure while running is placed in the LustScript.
    let (lines, ending) = run(
        "fail.lsc",
        "-- api: 1\nfetch \"aventail/scene\":[Scene, Shape];\n\nShape s = Scene:find(\"Nope\");\nprint(s:name());\nend\n",
    );
    assert_eq!(ending, Ending::Failed);
    let found = mistakes(&lines);
    assert_eq!(
        (found[0].file.as_str(), found[0].line),
        ("fail.lsc", 5),
        "{found:?}"
    );
    // A chunk asked for past what one call reads.
    let (lines, _) = run(
        "big.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nc:points(0, 100001)\n",
    );
    assert!(
        mistakes(&lines)[0].message.contains("at most 100000"),
        "{lines:?}"
    );
}

#[test]
fn versions_are_checked_and_a_missing_one_is_noted() {
    let (lines, ending) = run("new.lua", "-- api: 9\nprint(1)\n");
    assert_eq!(ending, Ending::Failed);
    assert!(
        mistakes(&lines)[0]
            .message
            .contains("newer version of the app")
    );
    let (lines, ending) = run("old.lua", "print(1)\n");
    assert_eq!(ending, Ending::Finished);
    assert!(
        lines
            .iter()
            .any(|l| matches!(l, Line::Note(n) if n.contains("taken as version 1")))
    );
    let (lines, ending) = run("tidy.txt", "print(1)\n");
    assert_eq!(ending, Ending::Failed);
    assert!(mistakes(&lines)[0].message.contains("isn't a script"));
}

/// Limits short enough for tests to wait for.
fn quick() -> Limits {
    Limits {
        time: Duration::from_millis(300),
        memory: 64 << 20,
    }
}

#[test]
fn endless_loops_and_recursion_are_stopped_by_the_time_limit() {
    for (name, text) in [
        ("loop.lua", "while true do end"),
        ("loop.lsc", "-- api: 1\nloop {\n}\nend\n"),
        ("calls.lua", "local function f() return f() end f()"),
        (
            "deep.lua",
            "local function f(n) return 1 + f(n + 1) end f(1)",
        ),
    ] {
        let start = Instant::now();
        let (lines, ending) = run_with(name, text, quick());
        assert!(
            matches!(ending, Ending::OutOfTime | Ending::Failed),
            "{name}: {ending:?} {lines:?}"
        );
        assert!(start.elapsed() < Duration::from_secs(5), "{name}");
    }
}

#[test]
fn the_time_limit_holds_through_pcall() {
    // Each time the limit's error is caught, the loop around tries again:
    // once past its time, every check stops it, and the loop itself is
    // stopped.
    for text in [
        "while true do pcall(function() while true do end end) end",
        "local function f() pcall(f) pcall(f) end while true do pcall(f) end",
        "while true do xpcall(function() while true do end end, function() while true do end end) end",
    ] {
        let start = Instant::now();
        let (_, ending) = run_with("stubborn.lua", text, quick());
        assert!(matches!(ending, Ending::OutOfTime), "{text}: {ending:?}");
        assert!(start.elapsed() < Duration::from_secs(5), "{text}");
    }
}

#[test]
fn memory_bombs_are_stopped_by_the_memory_limit() {
    for text in [
        "local s = string.rep(\"x\", 1e9)",
        "local t = {} for i = 1, 1e9 do t[i] = i end",
        "local s = \"x\" while true do s = s .. s end",
        "local t = {} while true do t[#t + 1] = {} end",
    ] {
        let start = Instant::now();
        let (lines, ending) = run_with("bomb.lua", text, quick());
        assert!(
            matches!(
                ending,
                Ending::OutOfMemory | Ending::OutOfTime | Ending::Failed
            ),
            "{text}: {ending:?} {lines:?}"
        );
        assert_ne!(ending, Ending::Finished);
        assert!(start.elapsed() < Duration::from_secs(10), "{text}");
    }
    let (_, ending) = run_with("bomb.lua", "local s = string.rep(\"x\", 1e9)", quick());
    assert_eq!(ending, Ending::OutOfMemory);
}

#[test]
fn deep_nesting_is_a_mistake_never_a_crash() {
    let deep_lua = format!("local x = {}1{}", "(".repeat(5000), ")".repeat(5000));
    let (lines, ending) = run("nest.lua", &deep_lua);
    assert_eq!(ending, Ending::Failed, "{lines:?}");
    let deep_lust = format!("int x = {}1{};\nend\n", "(".repeat(5000), ")".repeat(5000));
    let (lines, ending) = run("nest.lsc", &deep_lust);
    assert_eq!(ending, Ending::Failed, "{lines:?}");
    let tables = format!("local t = {}{}", "{".repeat(5000), "}".repeat(5000));
    let (_, ending) = run("tables.lua", &tables);
    assert_ne!(ending, Ending::Finished);
}

#[test]
fn reaching_outside_the_sandbox_fails_in_plain_words() {
    for text in [
        "io.open(\"x\", \"w\")",
        "os.execute(\"echo hi\")",
        "os.remove(\"x\")",
        "require(\"x\")",
        "loadstring(\"return 1\")()",
        "getfenv(1).x = 1",
        "setfenv(1, {})",
        "debug.getinfo(1)",
        "string.rep = nil",
        "aventail.Scene = nil",
        "aventail.Scene.selected = print",
        "getmetatable(aventail.Scene).__index = nil",
        "setmetatable(aventail.Scene, {})",
        "rawset(aventail, \"Scene\", 1)",
        "local s = aventail.Scene:selected()[1] s.id = 9",
        "getmetatable(\"\").__index = {}",
        "aventail.Scene:selected()[1]:explode()",
    ] {
        let (lines, ending) = run("hostile.lua", text);
        assert_eq!(ending, Ending::Failed, "{text} should fail: {lines:?}");
        assert_eq!(mistakes(&lines).len(), 1, "{text}");
    }
    // Bytecode is never loaded: text starting with a low byte is text.
    let (_, ending) = run("bytes.lua", "\u{1b}Lua\0\0\0");
    assert_eq!(ending, Ending::Failed);
    // Lust's own file calls say they aren't available here.
    let (lines, ending) = run(
        "files.lsc",
        "-- api: 1\nstring s = Files:read(\"x.txt\");\nend\n",
    );
    assert_eq!(ending, Ending::Failed, "{lines:?}");
}

#[test]
fn one_script_leaves_nothing_behind_for_the_next() {
    let (_, ending) = run("one.lua", "leftover = 42 table.insert = nil");
    assert_eq!(ending, Ending::Failed);
    let (_, ending) = run("one.lua", "leftover = 42");
    assert_eq!(ending, Ending::Finished);
    let (lines, ending) = run("two.lua", "print(leftover, table.insert ~= nil)");
    assert_eq!(ending, Ending::Finished);
    assert_eq!(said(&lines), ["nil\ttrue"]);
}

#[test]
fn stop_ends_a_running_script_at_once() {
    let file = ScriptFile {
        name: "spin.lua".into(),
        text: "while true do end".into(),
    };
    let long = Limits {
        time: Duration::from_secs(600),
        ..Limits::default()
    };
    let mut run = Run::start(file, snapshot().0, long, names());
    std::thread::sleep(Duration::from_millis(100));
    assert!(!run.has_ended());
    assert!(run.events().iter().all(|e| !matches!(e, Event::Ended(_))));
    let start = Instant::now();
    run.stop();
    assert!(run.stopping());
    let events = run.wait();
    assert_eq!(events.last(), Some(&Event::Ended(Ending::Stopped)));
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn printing_too_much_is_cut_and_the_run_goes_on() {
    let (lines, ending) = run(
        "chatty.lua",
        "for i = 1, 5000 do print(string.rep(\"x\", 10000)) end print(\"never shown\")",
    );
    assert_eq!(ending, Ending::Finished);
    assert_eq!(said(&lines).len(), crate::MAX_LINES);
    assert!(
        said(&lines)
            .iter()
            .all(|l| l.len() <= crate::MAX_LINE_BYTES + 3)
    );
}

#[test]
fn endings_have_words() {
    let limits = Limits::default();
    assert!(ending_words(&Ending::OutOfTime, "Tidy", limits).contains("10 seconds"));
    assert!(ending_words(&Ending::OutOfMemory, "Tidy", limits).contains("256 MB"));
    assert!(ending_words(&Ending::Finished, "Tidy", limits).starts_with("Tidy"));
}

#[test]
fn fuzzed_scripts_never_panic() {
    // Fixed seed: the same scripts every run. Pieces of LustScript and Lua
    // stuck together at random, checked and run with short limits.
    let mut seed: u64 = 0x1234_5678;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let pieces = [
        "fetch \"aventail/scene\":[Scene, Shape];\n",
        "Scene:shapes()",
        "Shape s = ",
        "loop (Shape s in Scene:selected()) {",
        "}",
        "end\n",
        "print(",
        ")",
        ";",
        "\n",
        "component C {",
        "has {",
        "int n = 1;",
        "method run() {",
        "is ",
        "? then {",
        "aventail.Scene:find(\"Cube\")",
        ":points(0, 10)",
        "local x = ",
        "function f()",
        "for i = 1, 3 do",
        " end",
        "\"",
        "{",
        "[",
        "]",
        "nothing",
        "-- name: x\n",
        "-- api: 1\n",
        "é",
        "\0",
        "!!!",
        "@",
        ".text()",
        "1e309",
        "-",
    ];
    let quick = Limits {
        time: Duration::from_millis(200),
        memory: 32 << 20,
    };
    for i in 0..150 {
        let mut text = String::new();
        for _ in 0..(next() % 24) {
            text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
        }
        let name = if i % 2 == 0 { "fuzz.lsc" } else { "fuzz.lua" };
        let (_, ending) = run_with(name, &text, quick);
        let _ = ending;
    }
}

#[test]
fn compiling_and_starting_a_small_script_is_quick() {
    // The benchmark: check, compile and run a small LustScript and the
    // same in Lua, a few times each; print the times (cargo test -- --nocapture).
    for (name, text) in [("read.lsc", LUST_READER), ("read.lua", LUA_READER)] {
        let start = Instant::now();
        let runs = 5;
        for _ in 0..runs {
            let (_, ending) = run(name, text);
            assert_eq!(ending, Ending::Finished);
        }
        let each = start.elapsed() / runs;
        println!("{name}: checked, compiled and run in {each:?} each");
        // Generous, for busy machines and debug builds: a person waits
        // for this when they run a script.
        assert!(each < Duration::from_secs(3), "{name}: {each:?}");
    }
}

#[test]
fn the_manuals_example_scripts_run() {
    // Every script in docs/manual/scripts.md and your-first-script.md
    // (``` blocks are LustScript, ```lua blocks Lua; ```text blocks are
    // pieces, not whole scripts) runs to its end against the test scene.
    let manual = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual");
    let page = ["scripts.md", "your-first-script.md"]
        .map(|p| std::fs::read_to_string(manual.join(p)).unwrap())
        .join("\n")
        .replace("\r\n", "\n");
    let mut found = 0;
    let mut rest = page.as_str();
    while let Some(at) = rest.find("```") {
        let after = &rest[at + 3..];
        let (kind, body) = after.split_once('\n').unwrap();
        let end = body.find("```").unwrap();
        let code = &body[..end];
        rest = &body[end + 3..];
        let name = match kind {
            "" => "example.lsc",
            "lua" => "example.lua",
            _ => continue,
        };
        found += 1;
        let mut snap = snapshot().0;
        if code.contains("-- asks:") {
            // The permissions section's example: allowed, in a folder of
            // its own holding the file it reads.
            let folder =
                std::env::temp_dir().join(format!("script-manual-example-{}", std::process::id()));
            std::fs::create_dir_all(folder.join("notes")).unwrap();
            std::fs::write(folder.join("notes/names.txt"), "Chair").unwrap();
            snap.access = crate::Access {
                folder: Some(folder),
                read: true,
                write: true,
                chosen: false,
            };
        }
        if code.contains("ProjectSaved") {
            // A script that runs by itself as a project is saved: run at
            // that moment, it ends well (the test scene's names are tidy).
            let file = ScriptFile {
                name: name.into(),
                text: code.into(),
            };
            let events = Run::start_moment(
                file,
                snap,
                Limits::default(),
                names(),
                crate::Moment::ProjectSaved,
                Vec::new(),
            )
            .wait();
            assert!(
                events.contains(&Event::Ended(Ending::Finished)),
                "{code}\n{events:?}"
            );
            continue;
        }
        let ran = run_on(name, code, snap, Limits::default());
        assert_eq!(ran.ending, Ending::Finished, "{code}\n{:?}", ran.lines);
        // Each one prints something or changes something (the selection
        // may be what it works on, and it may have nothing to change).
        assert!(
            !said(&ran.lines).is_empty() || ran.landing.is_some() || code.contains("selected"),
            "{code}"
        );
    }
    assert!(found >= 7, "only {found} examples found");
}

#[test]
fn your_first_scripts_mistakes_are_as_the_manual_says() {
    // docs/manual/your-first-script.md, step 9 and the Lua section.
    let lust = "-- name: Bevel hard edges\n-- about: Bevels every hard edge of the selected shapes.\n-- api: 1\n\nfetch \"aventail/scene\":[Scene, Shape];\n\nloop (Shape shape in Scene:selected()) {\n    shape:bevel(shape:hardEdges(), \"wide\", 1);\n}\nend\n";
    let file = ScriptFile {
        name: "Bevel hard edges.lsc".into(),
        text: lust.into(),
    };
    let problems = crate::check(&file, &names());
    assert_eq!(
        (problems[0].line, problems[0].column),
        (8, 36),
        "{problems:?}"
    );
    let lua = "-- api: 1\nfor _, shape in ipairs(aventail.Scene:selected()) do\n    shape:bevel(shape:hardEdges(), \"wide\", 1)\nend\n";
    let ran = run_on("b.lua", lua, snapshot().0, Limits::default());
    let mistake = &mistakes(&ran.lines)[0];
    assert_eq!(mistake.line, 3);
    assert!(
        mistake.message.contains("shape:bevel") && mistake.message.contains("width"),
        "{mistake:?}"
    );
}
