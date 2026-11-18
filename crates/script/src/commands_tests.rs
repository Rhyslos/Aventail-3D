//! Reading a script's commands and running one with chosen settings.

use super::*;
use crate::output::Ending;
use crate::run::tests::{names, snapshot};
use crate::{Event, Run};

fn file(name: &str, text: &str) -> ScriptFile {
    ScriptFile {
        name: name.into(),
        text: text.into(),
    }
}

fn read(name: &str, text: &str) -> Described {
    commands_of(&file(name, text), &names(), Limits::default())
}

const EVERY_KIND_LUST: &str = "-- name: Every kind
-- api: 1
fetch \"aventail/script\":Script;

state Side = [Left, Right];

component Kinds {
    has {
        bool on = true;
        int count = 3;
        float width = 0.25;
        string label = \"hi\";
        Side side = Side.Right;
        vec3 offset = [1, 2, 3];
        colour tint = [0.5, 0.25, 1.0];
    }

    method run() {
        Script:say(\"{on} {count} {width} {label} {offset.x} {tint.g}\");
        is side == Side.Left? then {
            Script:say(\"left\");
        }
    }
}

pass Kinds;
end
";

const EVERY_KIND_LUA: &str = "-- api: 1
local K = command(\"Kinds\", { on = true, count = 3, width = 0.25, label = \"hi\",
    offset = vector.create(1, 2, 3), tint = { r = 0.5, g = 0.25, b = 1 }, extra = {} })
function K:run()
    print(self.on, self.count, self.width, self.label, self.offset.x, self.tint.g)
end
";

fn kinds(found: &Described) -> Vec<(String, SettingKind, Option<SettingValue>)> {
    found.commands[0]
        .settings
        .iter()
        .map(|s| (s.name.clone(), s.kind.clone(), s.start.clone()))
        .collect()
}

#[test]
fn every_kind_of_setting_is_read_in_lustscript() {
    let found = read("kinds.lsc", EVERY_KIND_LUST);
    assert!(found.problems.is_empty(), "{:?}", found.problems);
    assert_eq!(found.commands.len(), 1);
    assert_eq!(found.commands[0].name, "Kinds");
    let all = kinds(&found);
    assert_eq!(
        all[0],
        (
            "on".into(),
            SettingKind::Bool,
            Some(SettingValue::Bool(true))
        )
    );
    assert_eq!(
        all[1],
        ("count".into(), SettingKind::Int, Some(SettingValue::Int(3)))
    );
    assert_eq!(all[2].1, SettingKind::Float);
    assert_eq!(all[3].2, Some(SettingValue::Text("hi".into())));
    assert_eq!(
        all[4].1,
        SettingKind::Choice(vec!["Left".into(), "Right".into()])
    );
    assert_eq!(all[4].2, Some(SettingValue::Choice("Right".into())));
    assert_eq!(all[5].2, Some(SettingValue::Vec3([1.0, 2.0, 3.0])));
    assert_eq!(all[6].1, SettingKind::Colour);
}

#[test]
fn every_kind_of_setting_is_read_in_lua_by_name() {
    let found = read("kinds.lua", EVERY_KIND_LUA);
    assert_eq!(found.commands.len(), 1);
    let all = kinds(&found);
    let names: Vec<&str> = all.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(
        names,
        ["count", "extra", "label", "offset", "on", "tint", "width"]
    );
    assert_eq!(all[0].1, SettingKind::Int);
    assert!(matches!(all[1].1, SettingKind::Other(_)));
    assert_eq!(all[1].2, None);
    assert_eq!(all[3].1, SettingKind::Vec3);
    assert_eq!(all[5].2, Some(SettingValue::Colour([0.5, 0.25, 1.0, 1.0])));
    assert_eq!(all[6].2, Some(SettingValue::Float(0.25)));
}

/// Runs `command` of `text` with `values`, and gives what it said.
fn run_command(
    name: &str,
    text: &str,
    command: &str,
    values: Vec<(&str, SettingValue)>,
) -> (Vec<String>, Ending) {
    let choice = Choice {
        command: command.into(),
        values: values.into_iter().map(|(n, v)| (n.into(), v)).collect(),
    };
    let events = Run::start_command(
        file(name, text),
        snapshot().0,
        Limits::default(),
        names(),
        choice,
    )
    .wait();
    let mut said = Vec::new();
    let mut ending = None;
    for event in events {
        match event {
            Event::Line(crate::Line::Said(s)) => said.push(s),
            Event::Line(crate::Line::Mistake(p)) => said.push(format!("mistake: {p}")),
            Event::Ended(e) => ending = Some(e),
            _ => {}
        }
    }
    (said, ending.unwrap())
}

#[test]
fn chosen_settings_reach_the_command_in_both_languages() {
    let values = vec![
        ("on", SettingValue::Bool(false)),
        ("count", SettingValue::Int(7)),
        ("width", SettingValue::Float(1.5)),
        ("label", SettingValue::Text("yo".into())),
        ("side", SettingValue::Choice("Left".into())),
        ("offset", SettingValue::Vec3([9.0, 0.0, 0.0])),
        ("tint", SettingValue::Colour([0.0, 0.75, 0.0, 1.0])),
    ];
    let (said, ending) = run_command("kinds.lsc", EVERY_KIND_LUST, "Kinds", values.clone());
    assert_eq!(ending, Ending::Finished, "{said:?}");
    assert_eq!(said, ["false 7 1.5 yo 9.0 0.75", "left"]);
    let (said, ending) = run_command("kinds.lua", EVERY_KIND_LUA, "Kinds", values);
    assert_eq!(ending, Ending::Finished, "{said:?}");
    assert_eq!(said, ["false\t7\t1.5\tyo\t9\t0.75"]);
}

#[test]
fn values_of_the_wrong_kind_or_unknown_settings_keep_the_start() {
    let values = vec![
        ("count", SettingValue::Text("seven".into())),
        ("side", SettingValue::Choice("Up".into())),
        ("nothing", SettingValue::Int(1)),
    ];
    let (said, ending) = run_command("kinds.lsc", EVERY_KIND_LUST, "Kinds", values);
    assert_eq!(ending, Ending::Finished, "{said:?}");
    assert_eq!(said, ["true 3 0.25 hi 1.0 0.25"]);
}

#[test]
fn only_the_chosen_command_runs() {
    let text = "-- api: 1
local A = command(\"A\", {})
function A:run() print(\"a\") end
local B = command(\"B\", {})
function B:run() print(\"b\") end
";
    let (said, _) = run_command("two.lua", text, "B", vec![]);
    assert_eq!(said, ["b"]);
    let (said, ending) = run_command("two.lua", text, "Gone", vec![]);
    assert_eq!(ending, Ending::Failed);
    assert!(said[0].contains("no command called Gone"), "{said:?}");
}

#[test]
fn a_script_with_no_commands_has_none() {
    let found = read("plain.lua", "-- api: 1\nprint(\"hello\")\n");
    assert!(found.commands.is_empty());
    assert!(found.problems.is_empty());
    let found = read(
        "plain.lsc",
        "-- api: 1\nfetch \"aventail/script\":Script;\nScript:say(\"x\");\nend\n",
    );
    assert!(found.commands.is_empty(), "{:?}", found.problems);
    assert!(found.problems.is_empty(), "{:?}", found.problems);
}

#[test]
fn components_without_run_are_not_commands_and_lua_twins_are_noted() {
    let found = read(
        "c.lsc",
        "component NotACommand {\n    has {\n        int n = 0;\n    }\n}\npass NotACommand;\nend\n",
    );
    assert!(found.commands.is_empty(), "{:?}", found.problems);
    let found = read(
        "twins.lua",
        "local A = command(\"Tidy\", {})\nfunction A:run() end\nlocal B = command(\"Tidy\", {})\nfunction B:run() end\ncommand(\"NoRun\")\n",
    );
    assert_eq!(found.commands.len(), 1);
    assert!(found.problems.is_empty());
    assert_eq!(found.notes.len(), 2, "{:?}", found.notes);
}

#[test]
fn settings_named_like_reserved_words_are_handled() {
    let found = read(
        "reserved.lua",
        "local A = command(\"A\", { [\"end\"] = 1, run = 2, __index = 3, ['local'] = true })\nfunction A:run() print(self['end'], self['local']) end\n",
    );
    let names: Vec<&str> = found.commands[0]
        .settings
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, ["end", "local"]);
    assert_eq!(found.notes.len(), 2, "{:?}", found.notes);
    let (said, _) = run_command(
        "reserved.lua",
        "local A = command(\"A\", { [\"end\"] = 1, ['local'] = true })\nfunction A:run() print(self['end'], self['local']) end\n",
        "A",
        vec![
            ("end", SettingValue::Int(4)),
            ("local", SettingValue::Bool(false)),
        ],
    );
    assert_eq!(said, ["4\tfalse"]);
}

#[test]
fn mistakes_newer_scripts_and_slow_starts_give_no_commands() {
    let found = read("bad.lsc", "component {\nend\n");
    assert!(found.commands.is_empty());
    assert!(!found.problems.is_empty());
    let found = read(
        "new.lua",
        "-- api: 99\nlocal A = command(\"A\", {})\nfunction A:run() end\n",
    );
    assert!(found.commands.is_empty());
    assert!(found.problems[0].message.contains("newer version"));
    let found = read("slow.lua", "while true do end\n");
    assert!(found.commands.is_empty());
    assert!(
        found.problems[0].message.contains("took longer"),
        "{:?}",
        found.problems
    );
    let found = read("fails.lua", "error(\"no\")\n");
    assert!(found.problems[0].message.contains("no"));
}

#[test]
fn reading_commands_changes_nothing_and_reaches_no_scene() {
    let found = read(
        "maker.lua",
        "aventail.Scene:add(aventail.Primitive.Cube)\nprint(#aventail.Scene:shapes())\nlocal A = command(\"A\", {})\nfunction A:run() end\n",
    );
    assert_eq!(found.commands.len(), 1, "{:?}", found.problems);
}

#[test]
fn names_read_as_words() {
    assert_eq!(words_of("BevelHardEdges"), "Bevel hard edges");
    assert_eq!(words_of("bevelHardEdges"), "Bevel hard edges");
    assert_eq!(words_of("bevel_hard_edges"), "Bevel hard edges");
    assert_eq!(words_of("width"), "Width");
    assert_eq!(words_of("UV2"), "Uv2");
    assert_eq!(words_of(""), "");
    assert_eq!(words_of("__x__"), "X");
}

#[test]
fn fuzzed_settings_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let pieces = [
        "1",
        "1.5",
        "true",
        "\"x\"",
        "{}",
        "{x=1,y=2,z=3}",
        "{r=1,g=0,b=0}",
        "nil",
        "-0",
        "1e308",
        "0/0",
        "vector.create(1,2,3)",
        "function() end",
    ];
    for _ in 0..60 {
        let mut entries = Vec::new();
        for i in 0..(next() % 6) {
            let value = pieces[(next() % pieces.len() as u64) as usize];
            entries.push(format!("s{i} = {value}"));
        }
        let text = format!(
            "local A = command(\"A\", {{ {} }})\nfunction A:run() end\n",
            entries.join(", ")
        );
        let found = read("fuzz.lua", &text);
        assert!(found.commands.len() <= 1, "{text}");
        for setting in found.commands.iter().flat_map(|c| &c.settings) {
            if let Some(start) = &setting.start {
                assert!(start.fits(&setting.kind), "{text}");
            }
        }
    }
}
