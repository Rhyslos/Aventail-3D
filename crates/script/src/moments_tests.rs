//! Scripts running by themselves at a moment (decision 0309): reacting
//! in LustScript and Lua, settings chosen, what's read while an export
//! starts, stopping it, and what's refused.

use super::*;
use crate::output::{Ending, Event, Line};
use crate::run::tests::{names, snapshot};
use crate::{Choice, Limits, Run, ScriptFile, SettingValue, Snapshot};
use std::time::{Duration, Instant};

/// What a moment's run said, how it ended, what it changed, and why it
/// stopped the export, if it did.
struct Ran {
    lines: Vec<Line>,
    ending: Ending,
    changes: usize,
    cancelled: Vec<String>,
}

fn run_moment(
    name: &str,
    text: &str,
    snapshot: Snapshot,
    moment: Moment,
    settings: Vec<Choice>,
) -> Ran {
    let file = ScriptFile {
        name: name.into(),
        text: text.into(),
    };
    let events = Run::start_moment(
        file,
        snapshot,
        Limits::default(),
        names(),
        moment,
        settings,
    )
    .wait();
    let mut ran = Ran {
        lines: Vec::new(),
        ending: Ending::Failed,
        changes: 0,
        cancelled: Vec::new(),
    };
    for event in events {
        match event {
            Event::Line(line) => ran.lines.push(line),
            Event::Ended(e) => ran.ending = e,
            Event::Land(l) => ran.changes = l.changes.len(),
            Event::Cancelled(why) => ran.cancelled.push(why),
            Event::Progress(..) => {}
        }
    }
    ran
}

fn said(ran: &Ran) -> Vec<String> {
    ran.lines
        .iter()
        .filter_map(|l| match l {
            Line::Said(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

fn mistakes(ran: &Ran) -> String {
    ran.lines
        .iter()
        .filter_map(|l| match l {
            Line::Mistake(p) => Some(p.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The test scene, an export of its cube in glTF starting.
fn exporting() -> Snapshot {
    let (mut snapshot, cube) = snapshot();
    snapshot.export = Some(ExportInfo {
        shapes: vec![cube],
        format: fileio::export::ExportFormat::Glb,
    });
    snapshot
}

const LUST_ALL: &str = r#"-- name: Moments
-- api: 1
fetch "aventail/scene":[Scene, Shape];
fetch "aventail/script":Script;
fetch "aventail/moments":[ProjectOpened, ProjectSaved, ExportStarting];

component Bravo {
    has {
        int times = 1;
    }
    on ProjectSaved() {
        Script:say("bravo saved {times}");
    }
}

component Alpha {
    on ProjectOpened() {
        Script:say("alpha opened");
    }
    on ProjectSaved() {
        Script:say("alpha saved");
        Scene:shapes()[0]:setName("Tidied");
    }
    on ExportStarting() {
        Script:say("alpha exporting");
    }
    method run() {
        Script:say("alpha ran");
    }
}
end
"#;

#[test]
fn each_component_reacting_runs_once_in_name_order() {
    let ran = run_moment(
        "moments.lsc",
        LUST_ALL,
        snapshot().0,
        Moment::ProjectSaved,
        Vec::new(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{}", mistakes(&ran));
    assert_eq!(said(&ran), ["alpha saved", "bravo saved 1"]);
    assert_eq!(ran.changes, 1);
    let ran = run_moment(
        "moments.lsc",
        LUST_ALL,
        snapshot().0,
        Moment::ProjectOpened,
        Vec::new(),
    );
    // Only what reacts runs: not the command's run, not other moments.
    assert_eq!(said(&ran), ["alpha opened"]);
    assert_eq!(ran.changes, 0);
}

#[test]
fn settings_chosen_reach_the_component_reacting() {
    let ran = run_moment(
        "moments.lsc",
        LUST_ALL,
        snapshot().0,
        Moment::ProjectSaved,
        vec![Choice {
            command: "Bravo".into(),
            values: vec![("times".into(), SettingValue::Int(4))],
        }],
    );
    assert_eq!(said(&ran), ["alpha saved", "bravo saved 4"]);
}

const LUA_ALL: &str = r#"-- name: Moments
-- api: 1
local Tidy = command("Tidy", { times = 1 })
function Tidy:on_ProjectSaved()
    print("tidy saved " .. self.times)
    aventail.Scene:shapes()[1]:setName("Tidied")
end
function Tidy:on_ExportStarting()
    print("tidy exporting")
end
local Check = command("Check", {})
function Check:on_ExportStarting()
    print("check exporting")
end
"#;

#[test]
fn lua_commands_react_with_on_functions_in_the_order_made() {
    let ran = run_moment(
        "moments.lua",
        LUA_ALL,
        exporting(),
        Moment::ExportStarting,
        Vec::new(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{}", mistakes(&ran));
    assert_eq!(said(&ran), ["tidy exporting", "check exporting"]);
    let ran = run_moment(
        "moments.lua",
        LUA_ALL,
        snapshot().0,
        Moment::ProjectSaved,
        vec![Choice {
            command: "Tidy".into(),
            values: vec![("times".into(), SettingValue::Int(3))],
        }],
    );
    assert_eq!(said(&ran), ["tidy saved 3"]);
    assert_eq!(ran.changes, 1);
}

#[test]
fn a_lua_command_reacting_without_a_run_is_no_command_and_says_nothing() {
    let described = crate::commands_of(
        &ScriptFile {
            name: "moments.lua".into(),
            text: LUA_ALL.into(),
        },
        &names(),
        Limits::default(),
    );
    assert!(described.problems.is_empty(), "{:?}", described.problems);
    assert!(described.commands.is_empty());
    assert!(described.notes.is_empty(), "{:?}", described.notes);
    assert_eq!(
        described.moments,
        [Moment::ProjectSaved, Moment::ExportStarting]
    );
    // Run whole (the Run button), it does nothing and notes nothing.
    let ran = crate::run::tests::run_on(
        "moments.lua",
        LUA_ALL,
        snapshot().0,
        Limits::default(),
    );
    assert_eq!(ran.ending, Ending::Finished);
    assert!(ran.lines.is_empty(), "{:?}", ran.lines);
}

#[test]
fn the_moments_a_script_reacts_to_are_read_with_its_commands() {
    let described = crate::commands_of(
        &ScriptFile {
            name: "moments.lsc".into(),
            text: LUST_ALL.into(),
        },
        &names(),
        Limits::default(),
    );
    assert!(described.problems.is_empty(), "{:?}", described.problems);
    assert_eq!(described.moments, Moment::ALL);
    // Alpha has a run; Bravo only reacts, so it's no command.
    let commands: Vec<&str> = described
        .commands
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(commands, ["Alpha"]);
}

#[test]
fn a_script_reacting_to_nothing_does_nothing_at_a_moment() {
    let ran = run_moment(
        "plain.lua",
        "print('top')\nlocal C = command('C', {})\nfunction C:run() print('ran') end",
        snapshot().0,
        Moment::ProjectOpened,
        Vec::new(),
    );
    // Its start runs, as every run's does; its command doesn't.
    assert_eq!(said(&ran), ["top"]);
    assert_eq!(ran.ending, Ending::Finished);
}

#[test]
fn an_export_starting_is_read_and_stopped_with_a_reason() {
    let text = r#"-- name: Check
fetch "aventail/scene":Shape;
fetch "aventail/script":Script;
fetch "aventail/moments":[Export, ExportStarting, Format];
component Check {
    on ExportStarting() {
        loop (Shape shape in Export:shapes()) {
            Script:say(shape:name());
        }
        is Export:format() == Format.Glb? then {
            Script:say("glb");
        }
        Export:cancel("the cube is too plain");
        Export:cancel("a second reason is left out");
        Script:say("carries on");
    }
}
end
"#;
    let ran = run_moment(
        "check.lsc",
        text,
        exporting(),
        Moment::ExportStarting,
        Vec::new(),
    );
    assert_eq!(ran.ending, Ending::Finished, "{}", mistakes(&ran));
    assert_eq!(said(&ran), ["Cube", "glb", "carries on"]);
    assert_eq!(ran.cancelled, ["the cube is too plain"]);
}

#[test]
fn lua_reads_the_export_too() {
    let text = "local C = command('C', {})\nfunction C:on_ExportStarting()\n  for _, s in ipairs(aventail.Export:shapes()) do print(s:name()) end\n  print(aventail.Export:format())\n  aventail.Export:cancel('')\nend";
    let mut snapshot = exporting();
    if let Some(export) = snapshot.export.as_mut() {
        export.format = fileio::export::ExportFormat::Stl;
    }
    let ran = run_moment(
        "check.lua",
        text,
        snapshot,
        Moment::ExportStarting,
        Vec::new(),
    );
    assert_eq!(said(&ran), ["Cube", "Stl"]);
    // An empty reason still stops it, saying it gave none.
    assert_eq!(ran.cancelled, ["it gave no reason"]);
}

#[test]
fn a_script_cant_change_the_scene_while_an_export_starts() {
    let text = "local C = command('C', {})\nfunction C:on_ExportStarting()\n  aventail.Scene:shapes()[1]:setName('New')\nend";
    let ran = run_moment(
        "change.lua",
        text,
        exporting(),
        Moment::ExportStarting,
        Vec::new(),
    );
    assert_eq!(ran.ending, Ending::Failed);
    assert_eq!(ran.changes, 0);
    assert!(
        mistakes(&ran).contains("can't change the scene while an export is starting"),
        "{}",
        mistakes(&ran)
    );
}

#[test]
fn the_export_calls_are_refused_outside_an_export() {
    for (moment, snapshot) in [
        (Moment::ProjectSaved, exporting()),
        (Moment::ExportStarting, snapshot().0),
    ] {
        let text = "local C = command('C', {})\nfunction C:on_ProjectSaved() aventail.Export:shapes() end\nfunction C:on_ExportStarting() aventail.Export:cancel('x') end";
        let ran = run_moment("outside.lua", text, snapshot, moment, Vec::new());
        assert_eq!(ran.ending, Ending::Failed);
        assert!(ran.cancelled.is_empty());
        assert!(
            mistakes(&ran).contains("only works while an export is starting"),
            "{}",
            mistakes(&ran)
        );
    }
    // A plain run can't reach them either.
    let ran = crate::run::tests::run_on(
        "plain.lua",
        "aventail.Export:format()",
        exporting(),
        Limits::default(),
    );
    assert_eq!(ran.ending, Ending::Failed);
}

#[test]
fn a_slow_script_at_a_moment_is_stopped_after_two_seconds() {
    let started = Instant::now();
    let ran = run_moment(
        "slow.lua",
        "local C = command('C', {})\nfunction C:on_ProjectSaved() while true do end end",
        snapshot().0,
        Moment::ProjectSaved,
        Vec::new(),
    );
    assert_eq!(ran.ending, Ending::OutOfTime);
    let took = started.elapsed();
    assert!(took >= MOMENT_LIMIT, "{took:?}");
    assert!(took < Duration::from_secs(8), "{took:?}");
}

#[test]
fn files_a_script_writes_are_kept_only_when_the_export_goes_on() {
    let root = std::env::temp_dir().join(format!("script-moments-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let access = crate::Access {
        folder: Some(root.clone()),
        read: true,
        write: true,
        chosen: false,
    };
    for (cancel, kept) in [(false, true), (true, false)] {
        let text = format!(
            "-- asks: read and write the project folder\nlocal C = command('C', {{}})\nfunction C:on_ExportStarting()\n  aventail.Project:writeText('log.txt', 'checked')\n  {}\nend",
            if cancel {
                "aventail.Export:cancel('no')"
            } else {
                ""
            }
        );
        let mut snapshot = exporting();
        snapshot.access = access.clone();
        let _ = std::fs::remove_file(root.join("log.txt"));
        let ran = run_moment("log.lua", &text, snapshot, Moment::ExportStarting, Vec::new());
        assert_eq!(ran.ending, Ending::Finished, "{}", mistakes(&ran));
        assert_eq!(root.join("log.txt").exists(), kept);
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_mistake_at_a_moment_points_at_the_scripts_line() {
    let ran = run_moment(
        "broken.lua",
        "local C = command('C', {})\nfunction C:on_ProjectOpened()\n  error('nope')\nend",
        snapshot().0,
        Moment::ProjectOpened,
        Vec::new(),
    );
    assert_eq!(ran.ending, Ending::Failed);
    assert!(mistakes(&ran).contains("broken.lua:3"), "{}", mistakes(&ran));
}

#[test]
fn fuzzed_moment_scripts_never_panic() {
    let pieces = [
        "aventail.Export:cancel('x')",
        "aventail.Export:shapes()",
        "aventail.Scene:add(aventail.Primitive.Cube)",
        "print(aventail.Export:format())",
        "error('x')",
        "self.times = 'text'",
        "aventail.Script:warn('w')",
        "",
    ];
    let mut seed: u64 = 7;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as usize
    };
    for _ in 0..40 {
        let moment = Moment::ALL[next() % 3];
        let mut text = String::from("local C = command('C', { times = 1 })\n");
        for m in Moment::ALL {
            text.push_str(&format!(
                "function C:{}()\n  {}\n  {}\nend\n",
                m.lua_handler(),
                pieces[next() % pieces.len()],
                pieces[next() % pieces.len()]
            ));
        }
        let snapshot = if next() % 2 == 0 {
            exporting()
        } else {
            snapshot().0
        };
        let ran = run_moment(
            "fuzz.lua",
            &text,
            snapshot,
            moment,
            vec![Choice {
                command: "C".into(),
                values: vec![("times".into(), SettingValue::Text("x".into()))],
            }],
        );
        if moment == Moment::ExportStarting {
            assert_eq!(ran.changes, 0, "{text}");
        }
    }
}
