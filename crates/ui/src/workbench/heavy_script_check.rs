//! The heavy scripting check (Phase 14's gate, decision 0309): scripts on
//! the heavy test shape (three million polygons as standard), timed in the
//! real interface against the plan's targets: starting a run (the copy of
//! the scene it works on), a script reading every polygon a chunk at a
//! time, one changing the shape, and landing what it changed as one undo
//! step.
//!
//! Not part of the normal test run: `tools/heavy/heavy-check.sh` runs it
//! with the heavy-mesh check (release build). `HEAVY_POLYGONS` sets the
//! shape's size.

use super::heavy_check::{H, Report, step};
use super::tests::{harness_for, info};
use super::*;
use std::time::{Duration, Instant};

/// Reads every polygon of the selected shape, a chunk at a time, and
/// counts those facing up.
const READ_ALL: &str = "-- name: Count polygons facing up
-- api: 1
local up, read = 0, 0
for _, shape in ipairs(aventail.Scene:selected()) do
    local count = shape:polygonCount()
    local from = 0
    while from < count do
        for _, polygon in ipairs(shape:polygons(from, 100000)) do
            read = read + 1
            if polygon.facing.z > 0.9 then up = up + 1 end
        end
        from = from + 100000
    end
end
print(read, up)
";

/// Changes the selected shape: renames it, extrudes one polygon and gives
/// the first 100 000 polygons a material.
const CHANGE: &str = "-- name: Change the heavy shape
-- api: 1
local shape = aventail.Scene:selected()[1]
shape:setName('Heavy, changed')
local ids = {}
for _, polygon in ipairs(shape:polygons(0, 100000)) do ids[#ids + 1] = polygon.id end
shape:extrude({ ids[1] }, 1.0)
shape:setMaterial(ids, 'Steel')
";

/// Starts `text` running, timing the frame that starts it, then waits for
/// it to end, timing the run and the frame that lands it.
fn timed_run(
    harness: &mut H,
    report: &mut Report,
    text: &str,
    what: [&'static str; 3],
    targets_ms: [u64; 3],
) {
    let file = script::ScriptFile {
        name: "heavy.lua".into(),
        text: text.into(),
    };
    let started = Instant::now();
    report.frames(harness, what[0], targets_ms[0], 1, |h, _| {
        h.state_mut().workbench.run_script(file.clone());
    });
    // Frames go on while it runs on its own thread; the slowest is the
    // one that lands what it changed.
    let mut slowest = Duration::ZERO;
    while harness.state().workbench.scripts.is_running() {
        std::thread::sleep(Duration::from_millis(5));
        let frame = Instant::now();
        step(harness);
        slowest = slowest.max(frame.elapsed());
        assert!(started.elapsed() < Duration::from_secs(300), "never ended");
    }
    report.record(what[1], started.elapsed(), targets_ms[1]);
    report.record(what[2], slowest, targets_ms[2]);
    let workbench = &harness.state().workbench;
    assert!(!workbench.scripts.is_running());
    let rows = &workbench.scripts.rows;
    assert!(
        matches!(rows.last(), Some(crate::scripts_panel::Row::Ended(_, true))),
        "{:?}",
        &rows[rows.len().saturating_sub(4)..]
    );
}

/// The heavy scripting check. Prints a table and fails if a target is
/// missed.
#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn heavy_script_check() {
    let polygons: usize = std::env::var("HEAVY_POLYGONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000_000);
    let mut report = Report::default();
    let mut workbench = Workbench::new(info(), None);
    report.once(
        "Make the heavy test shape (not timed against users)",
        60_000,
        || {
            workbench.add_test_shape(polygons);
        },
    );
    // Generous limits: the check times the work, not the standard limit.
    workbench.scripts.settings.time_seconds = 600;
    workbench.scripts.settings.memory_mb = 2048;
    let mut harness = harness_for(workbench);
    harness.run();
    step(&mut harness);
    timed_run(
        &mut harness,
        &mut report,
        READ_ALL,
        [
            "Start a script (the scene copied for it)",
            "Read every polygon, 100 000 at a time",
            "The slowest frame while it runs and ends",
        ],
        [100, 10_000, 100],
    );
    let steps = harness.state().workbench.document().undo_steps();
    timed_run(
        &mut harness,
        &mut report,
        CHANGE,
        [
            "Start a script that changes the shape",
            "Rename, extrude a polygon, material on 100 000",
            "The slowest frame: its changes land as one undo step",
        ],
        [100, 10_000, 3_000],
    );
    assert_eq!(harness.state().workbench.document().undo_steps(), steps + 1);
    report.frames(&mut harness, "Undo the whole run", 3_000, 1, |h, _| {
        h.state_mut().workbench.document_mut().undo();
    });
    let met = report.print_titled(&format!(
        "Heavy scripting check: one object of {polygons} polygons"
    ));
    assert!(met, "a target was missed");
}
