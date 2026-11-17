//! Scripts changing the scene (decision 0302): every changing call in
//! both languages, landing as one undo step that ends as the script's
//! copy did, and runs that fail, are stopped or go too far landing
//! nothing.

use crate::Landing;
use crate::output::{Ending, Line};
use crate::run::tests::{Ran, mistakes, names, run_on, said, snapshot};
use crate::{Limits, Run, ScriptFile, Snapshot};
use scene::{Document, Object, ObjectId, ObjectKind, Scene};
use std::time::{Duration, Instant};

/// The real document a snapshot was taken of.
fn real(snapshot: &Snapshot) -> Document {
    let mut document =
        Document::from_scene(snapshot.scene.clone(), snapshot.selection.iter().copied());
    document.select_elements(snapshot.elements.iter().cloned(), "Select", None);
    document
}

/// The objects of `scene` in order with their parents: the scene as a
/// person sees it (undo never takes back the next id).
fn objects(scene: &Scene) -> Vec<(ObjectId, Option<ObjectId>, Object)> {
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| (id, scene.parent(id), scene.object(id).unwrap().clone()))
        .collect()
}

/// Runs `text` as `name` on the test scene and lands what it did on the
/// real document, held while it ran, checking the two end the same and
/// that one undo and redo take it back and bring it again. Gives back
/// the run and the document after landing.
fn run_and_land(name: &str, text: &str) -> (Ran, Document) {
    let (snapshot, _) = snapshot();
    let mut document = real(&snapshot);
    let before = document.clone();
    document.hold();
    let ran = run_on(name, text, snapshot, Limits::default());
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    // Held, nothing else changed it while the script ran.
    assert_eq!(document.scene(), before.scene());
    document.release();
    let Some(landing) = ran.landing.clone() else {
        panic!("{name} changed nothing:\n{text}");
    };
    let steps = document.undo_steps();
    document
        .execute_batch(landing.changes.clone(), "The script")
        .unwrap();
    assert_eq!(document.undo_steps(), steps + 1);
    assert_eq!(document.undo_label(), Some("The script"));
    assert_eq!(
        document.scene(),
        &landing.scene,
        "the copy and the document"
    );
    let after = document.clone();
    document.undo();
    assert_eq!(objects(document.scene()), objects(before.scene()));
    assert_eq!(document.selection(), before.selection());
    document.redo();
    assert_eq!(document.scene(), after.scene());
    assert_eq!(document.selection(), after.selection());
    (ran, document)
}

/// The test scene's objects by name.
fn named(document: &Document, name: &str) -> Option<ObjectId> {
    let scene = document.scene();
    scene
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| scene.object(*id).is_some_and(|o| o.name == name))
}

const EVERY_CHANGE_IN_LUA: &str = r#"-- api: 1
local A = aventail
local S = A.Scene
local cube = S:find("Cube")

-- An edge of a polygon, read fresh: edits change the ids.
local function firstEdges(shape)
  local p = shape:polygons(0, 1)[1]
  return { { a = p.corners[1], b = p.corners[2] } }, p.id
end

local plane = S:add(A.Primitive.Plane)
print(plane:name(), S:selected()[1] == plane)
plane:setName("Floor")
plane:setPosition({ x = 0, y = 0, z = -50 })
plane:setRotation(vector.create(90, 0, 0))
plane:setScale({ x = 2, y = 2, z = 1 })
plane:hide()
print(plane:shown())
plane:show()
local p = plane:rotation()
print(plane:name(), plane:position().z, math.floor(p.x + 0.5), plane:scale().x, plane:shown())

local tri = S:build("Tri", { { x = 0, y = 0, z = 0 }, { x = 100, y = 0, z = 0 }, { x = 0, y = 100, z = 0 } },
  { { corners = { 0, 1, 2 } } })
print(tri:pointCount(), tri:polygonCount())
local group = S:group({ tri })
print(tri:parent() == group, group:name())

local sphere = S:find("Sphere")
local smooth = #sphere:hardEdges()
sphere:markHard((firstEdges(sphere)))
print(#sphere:hardEdges() - smooth)
local edges, poly = firstEdges(cube)
cube:bevel(edges, 5, 2)
print(cube:polygonCount())
edges, poly = firstEdges(cube)
cube:extrude({ poly }, 10)
edges, poly = firstEdges(cube)
cube:inset({ poly }, 2)
edges, poly = firstEdges(cube)
cube:markSeams(edges)
print(#cube:seams() > 0)
cube:unwrap()
cube:setMaterial({ poly }, "Red")
print(cube:materials()[2])
cube:makeEditable()

S:find("Sphere"):delete()
print(S:find("Sphere") == nil)

A.Selection:clear()
print(#S:selected())
A.Selection:set({ cube, plane })
A.Selection:setPolygons(cube, { 0, 1 })
A.Selection:setPoints(cube, { 3 })
A.Selection:setEdges(cube, firstEdges(cube))
print(#S:selected(), #A.Selection:polygons(cube), #A.Selection:points(cube), #A.Selection:edges(cube))
"#;

#[test]
fn every_changing_call_lands_in_lua_and_undoes_in_one_step() {
    let (ran, document) = run_and_land("every.lua", EVERY_CHANGE_IN_LUA);
    let said = said(&ran.lines);
    assert_eq!(said[0], "Plane\ttrue");
    assert_eq!(said[1], "false");
    assert_eq!(said[2], "Floor\t-50\t90\t2\ttrue");
    assert_eq!(said[3], "3\t1");
    assert!(said[4].starts_with("true\tGroup"), "{}", said[4]);
    assert_eq!(said[5], "1");
    assert_ne!(said[6], "6", "the bevel shows at once");
    assert_eq!(said[7], "true");
    assert_eq!(said[8], "Red");
    assert_eq!(said[9], "true");
    assert_eq!(said[10], "0");
    assert_eq!(said[11], "2\t2\t1\t1");

    let scene = document.scene();
    let cube = named(&document, "Cube").unwrap();
    let floor = named(&document, "Floor").unwrap();
    let tri = named(&document, "Tri").unwrap();
    assert!(named(&document, "Sphere").is_none());
    assert!(matches!(
        scene.object(cube).unwrap().kind,
        ObjectKind::Mesh(_)
    ));
    assert_eq!(scene.object(cube).unwrap().materials[1].name, "Red");
    assert_eq!(
        scene
            .parent(tri)
            .and_then(|g| scene.object(g))
            .unwrap()
            .name,
        "Group 2"
    );
    let floor_at = scene.object(floor).unwrap().transform;
    assert_eq!(floor_at.position.z, -50.0);
    assert!((floor_at.rotation.x - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    let mut selected = document.selection();
    selected.sort();
    let mut wanted = vec![cube, floor];
    wanted.sort();
    assert_eq!(selected, wanted);
    assert_eq!(document.elements(cube).unwrap().polygons().count(), 2);
}

#[test]
fn edits_are_recorded_in_edit_layers_until_made_editable() {
    let (_, document) = run_and_land(
        "layers.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nlocal p = c:polygons(0, 1)[1]\nc:extrude({ p.id }, 10)\nc:setMaterial({ p.id }, \"Default\")\n",
    );
    let cube = named(&document, "Cube").unwrap();
    let object = document.scene().object(cube).unwrap();
    assert!(
        matches!(object.kind, ObjectKind::Primitive(_)),
        "still a cube"
    );
    let recorded: usize = object.stack.layers().iter().map(|l| l.entries.len()).sum();
    assert_eq!(recorded, 2);
}

#[test]
fn changing_calls_work_in_lustscript() {
    let (ran, document) = run_and_land(
        "build.lsc",
        "-- api: 1
fetch \"aventail/scene\":[Scene, Shape, Polygon, Edge, Primitive];
fetch \"aventail/selection\":Selection;

vec3 array points = [[0, 0, 0], [100, 0, 0], [100, 100, 0], [0, 100, 0]];
Polygon array polygons = [new Polygon { corners = [0, 1, 2, 3] }];
Shape quad = Scene:build(\"Quad\", points, polygons);
print(quad:pointCount().text() + \" \" + quad:polygonCount().text());
Shape box = Scene:add(Primitive.Cube);
box:setName(\"Box\");
box:setPosition([10, 20, 30]);
Polygon array faces = box:polygons(0, 1);
int a = faces[0].corners[0];
int b = faces[0].corners[1];
Edge array edges = [new Edge { a = a, b = b }];
box:markHard(edges);
box:bevel(edges, 2, 1);
box:setMaterial([faces[0].id], \"Steel\");
Selection:set([quad, box]);
Selection:setPolygons(quad, [0]);
print((#Scene:selected()).text() + \" \" + box:materials()[1]);
end
",
    );
    let said = said(&ran.lines);
    assert_eq!(said, ["4 1", "2 Steel"]);
    let quad = named(&document, "Quad").unwrap();
    let box_id = named(&document, "Box").unwrap();
    assert_eq!(
        document.scene().object(box_id).unwrap().transform.position,
        glam::Vec3::new(10.0, 20.0, 30.0)
    );
    assert_eq!(document.elements(quad).unwrap().polygons().count(), 1);
}

#[test]
fn deleting_everything_lands_and_undo_brings_it_all_back() {
    let (_, document) = run_and_land(
        "empty.lua",
        "-- api: 1\nlocal top = {}\nfor _, s in ipairs(aventail.Scene:shapes()) do\n  if s:parent() == nil then table.insert(top, s) end\nend\nfor _, s in ipairs(top) do s:delete() end\nprint(#aventail.Scene:shapes())\n",
    );
    assert!(document.scene().walk().is_empty());
    assert!(document.selection().is_empty());
}

#[test]
fn a_run_that_reads_only_lands_nothing() {
    let ran = run_on(
        "read.lua",
        "-- api: 1\nprint(#aventail.Scene:shapes())\naventail.Selection:set(aventail.Scene:selected())\n",
        snapshot().0,
        Limits::default(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    // Selecting what's already selected is a change the document skips,
    // so only that one comes back.
    let landing = ran.landing.unwrap();
    assert_eq!(landing.changes.len(), 1);
}

/// Runs `text` and checks it lands nothing, giving back its mistakes'
/// messages.
fn lands_nothing(name: &str, text: &str, ending: Ending) -> Vec<String> {
    let ran = run_on(name, text, snapshot().0, Limits::default());
    assert_eq!(ran.ending, ending, "{:?}", ran.lines);
    assert_eq!(ran.landing, None, "{:?}", ran.lines);
    let mut found: Vec<String> = mistakes(&ran.lines)
        .into_iter()
        .map(|p| p.message)
        .collect();
    found.extend(ran.lines.iter().filter_map(|l| match l {
        Line::Note(n) => Some(n.clone()),
        _ => None,
    }));
    found
}

#[test]
fn a_script_failing_after_changes_lands_nothing() {
    let found = lands_nothing(
        "fails.lua",
        "-- api: 1\naventail.Scene:find(\"Cube\"):setName(\"Gone\")\nlocal t = nil\nprint(t.x)\n",
        Ending::Failed,
    );
    assert!(!found.is_empty());
}

#[test]
fn a_refused_change_stops_the_script_with_the_scenes_reason() {
    let found = lands_nothing(
        "refused.lua",
        "-- api: 1\nlocal g = aventail.Scene:find(\"Group\")\ng:setName(\"   \")\n",
        Ending::Failed,
    );
    assert!(
        found
            .iter()
            .any(|m| m.contains("setName(name) couldn't be done: a name can't be empty")),
        "{found:?}"
    );
    let found = lands_nothing(
        "group.lua",
        "-- api: 1\nlocal g = aventail.Scene:find(\"Group\")\ng:extrude({ 0 }, 1)\n",
        Ending::Failed,
    );
    assert!(found.iter().any(|m| m.contains("no shape")), "{found:?}");
}

#[test]
fn a_refusal_caught_with_pcall_still_lands_nothing() {
    let found = lands_nothing(
        "caught.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nc:setName(\"Kept?\")\n\
         print(pcall(function() c:setName(\"\") end))\n\
         print(pcall(function() c:setName(\"After\") end))\n",
        Ending::Failed,
    );
    assert!(
        found.iter().any(|m| m.contains("nothing it did was kept")),
        "{found:?}"
    );
}

#[test]
fn a_generator_with_bad_polygons_is_refused_with_a_reason() {
    let cases = [
        ("{ { corners = { 0, 1, 7 } } }", "only 3 points"),
        ("{ { corners = { 0, 1 } } }", "at least three corners"),
        ("{ { corners = { 0, 1, 1 } } }", "twice"),
        ("{ { corners = { 0, -1, 2 } } }", "counted from 0"),
        (
            "{ { corners = { 0, 1.5, 2 } } }",
            "Polygon with its corners",
        ),
        ("{ { 0, 1, 2 } }", "Polygon with its corners"),
        ("{ 5 }", "Polygon with its corners"),
        ("7", "a list"),
    ];
    for (polygons, reason) in cases {
        let text = format!(
            "-- api: 1\naventail.Scene:build(\"Bad\", {{ {{ x = 0, y = 0, z = 0 }}, {{ x = 1, y = 0, z = 0 }}, {{ x = 0, y = 1, z = 0 }} }}, {polygons})\n"
        );
        let found = lands_nothing("bad.lua", &text, Ending::Failed);
        assert!(
            found.iter().any(|m| m.contains(reason)),
            "{polygons}: {found:?}"
        );
    }
    let found = lands_nothing(
        "nan.lua",
        "-- api: 1\naventail.Scene:build(\"Bad\", { { x = 0/0, y = 0, z = 0 } }, {})\n",
        Ending::Failed,
    );
    assert!(found.iter().any(|m| m.contains("real number")), "{found:?}");
    let found = lands_nothing(
        "noname.lua",
        "-- api: 1\naventail.Scene:build(\" \", {}, {})\n",
        Ending::Failed,
    );
    assert!(found.iter().any(|m| m.contains("name")), "{found:?}");
}

#[test]
fn a_million_polygons_is_refused_and_lands_nothing() {
    let found = lands_nothing(
        "million.lua",
        "-- api: 1\nlocal p = { corners = { 0, 1, 2 } }\nlocal all = {}\nfor i = 1, 1000000 do all[i] = p end\n\
         aventail.Scene:build(\"Huge\", { vector.create(0, 0, 0), vector.create(1, 0, 0), vector.create(0, 1, 0) }, all)\n",
        Ending::Failed,
    );
    assert!(
        found.iter().any(|m| m.contains("at most 100000")),
        "{found:?}"
    );
}

#[test]
fn a_shape_with_no_polygons_can_be_built() {
    let (ran, document) = run_and_land(
        "points.lua",
        "-- api: 1\nlocal s = aventail.Scene:build(\"Dots\", { vector.create(1, 2, 3) }, {})\nprint(s:pointCount(), s:polygonCount())\n",
    );
    assert_eq!(said(&ran.lines), ["1\t0"]);
    assert!(named(&document, "Dots").is_some());
}

#[test]
fn elements_the_shape_hasnt_got_are_refused() {
    let cases = [
        "c:extrude({ 99 }, 1)",
        "c:bevel({ { a = 0, b = 99 } }, 1, 1)",
        "c:bevel({ { a = 0, b = 1 } }, 1, 0)",
        "aventail.Selection:setPoints(c, { 1000 })",
        "aventail.Selection:setPolygons(c, { -1 })",
        "c:setMaterial({ 0 }, \"\")",
        "aventail.Scene:add(\"Teapot\")",
        "aventail.Scene:group({})",
        "c:setPosition(5)",
    ];
    for case in cases {
        let text = format!("-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\n{case}\n");
        let found = lands_nothing("missing.lua", &text, Ending::Failed);
        assert_eq!(found.len(), 1, "{case}: {found:?}");
    }
}

#[test]
fn empty_lists_change_nothing() {
    let ran = run_on(
        "empty.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nc:bevel({}, 1, 1)\nc:extrude({}, 1)\nc:markHard({})\nc:setMaterial({}, \"Red\")\n",
        snapshot().0,
        Limits::default(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    assert_eq!(ran.landing, None);
}

#[test]
fn a_shape_deleted_earlier_is_refused_in_plain_words() {
    let found = lands_nothing(
        "deleted.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nc:delete()\nc:setName(\"Back?\")\n",
        Ending::Failed,
    );
    assert!(
        found.iter().any(|m| m.contains("isn't in the scene")),
        "{found:?}"
    );
}

#[test]
fn the_changes_a_run_may_make_are_limited() {
    let found = lands_nothing(
        "many.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nfor i = 1, 200000 do c:setName(\"Cube \" .. i) end\n",
        Ending::Failed,
    );
    assert!(
        found.iter().any(|m| m.contains("more than 100000 changes")),
        "{found:?}"
    );
}

#[test]
fn a_script_making_millions_of_shapes_is_stopped_and_lands_nothing() {
    let limits = Limits {
        time: Duration::from_secs(3),
        ..Limits::default()
    };
    let ran = run_on(
        "flood.lua",
        "-- api: 1\nfor i = 1, 5000000 do aventail.Scene:add(aventail.Primitive.Cube) end\n",
        snapshot().0,
        limits,
    );
    assert!(
        matches!(ran.ending, Ending::OutOfTime | Ending::Failed),
        "{:?}",
        ran.ending
    );
    assert_eq!(ran.landing, None);
}

#[test]
fn a_script_stopped_halfway_lands_nothing() {
    let file = ScriptFile {
        name: "halfway.lua".into(),
        text: "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nlocal i = 0\nwhile true do i = i + 1; c:setName(\"Cube \" .. i) end\n".into(),
    };
    let run = Run::start(file, snapshot().0, Limits::default(), names());
    std::thread::sleep(Duration::from_millis(300));
    run.stop();
    let events = run.wait();
    assert!(
        events.contains(&crate::Event::Ended(Ending::Stopped)),
        "{events:?}"
    );
    assert!(!events.iter().any(|e| matches!(e, crate::Event::Land(_))));
}

#[test]
fn landing_ten_thousand_small_changes_is_quick() {
    let (snapshot, _) = snapshot();
    let mut document = real(&snapshot);
    let ran = run_on(
        "renames.lua",
        "-- api: 1\nlocal c = aventail.Scene:find(\"Cube\")\nfor i = 1, 10000 do c:setPosition({ x = i, y = 0, z = 0 }) end\n",
        snapshot,
        Limits::default(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.lines);
    let Landing { changes, scene } = ran.landing.unwrap();
    assert_eq!(changes.len(), 10_000);
    let start = Instant::now();
    document.execute_batch(changes, "Moves").unwrap();
    let took = start.elapsed();
    eprintln!("landing 10,000 changes took {took:?}");
    assert!(took < Duration::from_secs(2), "{took:?}");
    assert_eq!(document.scene(), &scene);
}

#[test]
fn each_changing_call_on_its_own_lands_as_the_copy_ended() {
    let lines = [
        "S:add(A.Primitive.Torus)",
        "S:add(\"Gear\")",
        "S:build(\"T\", { vector.create(0, 0, 0), vector.create(1, 0, 0), vector.create(0, 1, 0) }, { { corners = { 2, 1, 0 } } })",
        "S:group({ c, S:find(\"Group\") })",
        "c:setName(\"Renamed\")",
        "c:setPosition(vector.create(1, 2, 3))",
        "c:setRotation(vector.create(10, 20, 30))",
        "c:setScale(vector.create(1, 2, 3))",
        "c:hide()",
        "c:hide() c:show()",
        "c:delete()",
        "c:bevel(c:hardEdges(), 3, 3)",
        "c:extrude({ 0, 1, 2 }, -5)",
        "c:inset({ 3, 4 }, 1)",
        "S:find(\"Sphere\"):markHard({ { a = 0, b = 1 } })",
        "local s = S:find(\"Sphere\") local p = s:polygons(100, 1)[1] s:markSeams({ { a = p.corners[1], b = p.corners[2] } })",
        "c:unwrap()",
        "c:setMaterial({ 5 }, \"Blue\")",
        "c:makeEditable()",
        "A.Selection:set({ S:find(\"Sphere\") })",
        "A.Selection:setPoints(c, { 0, 7 })",
        "A.Selection:setEdges(c, { { a = 0, b = 1 } })",
        "A.Selection:setPolygons(c, { 5 })",
        "A.Selection:clear()",
    ];
    for line in lines {
        let text = format!(
            "-- api: 1\nlocal A = aventail\nlocal S = A.Scene\nlocal c = S:find(\"Cube\")\n{line}\n"
        );
        run_and_land("one.lua", &text);
    }
}

#[test]
fn fuzzed_changes_never_panic_and_land_as_the_copy_ended() {
    // Fixed seed: the same scripts every run.
    let mut seed: u64 = 0x5eed_c0de;
    let mut next = |below: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % below
    };
    let shapes = ["Cube", "Sphere", "Group", "Nothing"];
    for _ in 0..80 {
        let mut text = String::from("-- api: 1\nlocal A = aventail\nlocal S = A.Scene\n");
        for _ in 0..1 + next(8) {
            let shape = shapes[next(4) as usize];
            let (n, m) = (next(12), next(12));
            let line = match next(14) {
                0 => format!("S:find(\"{shape}\"):setName(\"N{n}\")"),
                1 => format!("S:find(\"{shape}\"):extrude({{ {n}, {m} }}, {n})"),
                2 => format!("S:find(\"{shape}\"):bevel({{ {{ a = {n}, b = {m} }} }}, 1, {n})"),
                3 => format!("S:find(\"{shape}\"):inset({{ {n} }}, 0.5)"),
                4 => format!("S:find(\"{shape}\"):delete()"),
                5 => "S:add(A.Primitive.Cube)".to_string(),
                6 => format!("S:find(\"{shape}\"):setMaterial({{ {n} }}, \"M{m}\")"),
                7 => format!("A.Selection:setPolygons(S:find(\"{shape}\"), {{ {n} }})"),
                8 => format!("S:group({{ S:find(\"{shape}\") }})"),
                9 => format!("S:find(\"{shape}\"):makeEditable()"),
                10 => format!("S:find(\"{shape}\"):setPosition(vector.create({n}, {m}, 0))"),
                11 => format!(
                    "S:build(\"B\", {{ vector.create(0, 0, 0), vector.create({n}, 0, 0), vector.create(0, {m}, 0) }}, {{ {{ corners = {{ 0, {}, 2 }} }} }})",
                    n % 4
                ),
                12 => format!("S:find(\"{shape}\"):markSeams({{ {{ a = {n}, b = {m} }} }})"),
                _ => format!("pcall(function() S:find(\"{shape}\"):setName(\"\") end)"),
            };
            text.push_str(&line);
            text.push('\n');
        }
        let (snapshot, _) = snapshot();
        let mut document = real(&snapshot);
        let ran = run_on("fuzz.lua", &text, snapshot, Limits::default());
        if let Some(landing) = ran.landing {
            assert_eq!(ran.ending, Ending::Finished, "{text}");
            document.execute_batch(landing.changes, "Fuzz").unwrap();
            assert_eq!(document.scene(), &landing.scene, "{text}");
        }
    }
}
