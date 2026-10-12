//! The heavy-mesh check (DESIGN.md §4.8, decisions
//! [0033](../../../../docs/decisions/0033-heavy-mesh-safeguard.md) and
//! [0035](../../../../docs/decisions/0035-heavy-meshes.md)): one object of
//! millions of polygons in the real interface, timing what a person does
//! with it against the targets.
//!
//! Not part of the normal test run. Run it at the end of a phase, or when
//! asked, with `tools/heavy/heavy-check.sh` (release build; the polygon count can
//! be set with `HEAVY_POLYGONS`, three million as standard).

use super::tests::{State, camera, harness_for, info};
use super::*;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use glam::{Vec2, Vec3};
use std::time::{Duration, Instant};

type H = Harness<'static, State>;

/// One measured step: what was done, how long the slowest frame took, and
/// the target for it.
struct Measured {
    what: &'static str,
    slowest: Duration,
    average: Duration,
    target: Duration,
}

#[derive(Default)]
struct Report(Vec<Measured>);

impl Report {
    /// Runs `frames` frames, each after `before(i)` sends its input, and
    /// records the slowest and the average.
    fn frames(
        &mut self,
        harness: &mut H,
        what: &'static str,
        target_ms: u64,
        frames: usize,
        mut before: impl FnMut(&mut H, usize),
    ) {
        let mut slowest = Duration::ZERO;
        let mut total = Duration::ZERO;
        for i in 0..frames {
            before(harness, i);
            let start = Instant::now();
            step(harness);
            let took = start.elapsed();
            slowest = slowest.max(took);
            total += took;
        }
        self.0.push(Measured {
            what,
            slowest,
            average: total / frames.max(1) as u32,
            target: Duration::from_millis(target_ms),
        });
    }

    /// Times one piece of work outside the interface.
    fn once<T>(&mut self, what: &'static str, target_ms: u64, work: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let out = work();
        let took = start.elapsed();
        self.0.push(Measured {
            what,
            slowest: took,
            average: took,
            target: Duration::from_millis(target_ms),
        });
        out
    }

    fn print(&self, polygons: usize) -> bool {
        println!();
        println!("Heavy-mesh check: one object of {polygons} polygons");
        println!(
            "{:<58} {:>10} {:>10} {:>10}  ",
            "", "slowest", "average", "target"
        );
        let mut all_met = true;
        for m in &self.0 {
            let met = m.slowest <= m.target;
            all_met &= met;
            println!(
                "{:<58} {:>8.1}ms {:>8.1}ms {:>8}ms  {}",
                m.what,
                m.slowest.as_secs_f64() * 1000.0,
                m.average.as_secs_f64() * 1000.0,
                m.target.as_millis(),
                if met { "ok" } else { "SLOW" }
            );
        }
        println!();
        all_met
    }
}

/// Runs one frame. The stand-in renderer keeps every view it is asked to
/// draw (for other tests); here that would keep every overlay alive, so
/// they are let go.
fn step(harness: &mut H) {
    harness.step();
    harness.state_mut().renderer.drawn.clear();
}

/// Where `world` appears in the viewport, on screen.
fn on_screen(harness: &H, world: Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

fn pointer(harness: &mut H, at: Pos2, pressed: Option<bool>, modifiers: Modifiers) {
    harness.event(Event::PointerMoved(at));
    if let Some(pressed) = pressed {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
    }
}

/// A spot on the ball's surface, facing the camera, turned `turn` radians
/// around the ball.
fn on_ball(harness: &H, radius: f32, turn: f32) -> Vec3 {
    let centre = Vec3::new(0.0, 0.0, radius);
    let towards = (camera(harness).position - centre).normalize();
    let towards = glam::Quat::from_rotation_z(turn) * towards;
    centre + towards * radius * 0.98
}

/// The heavy-mesh check. Prints a table and fails if a target is missed.
#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn heavy_mesh_check() {
    let polygons: usize = std::env::var("HEAVY_POLYGONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000_000);
    let radius = 100.0;
    let mut report = Report::default();

    let ball = report.once(
        "Build the test shape (not timed against users)",
        60_000,
        || mesh::samples::heavy_ball(polygons, radius),
    );
    let mut workbench = Workbench::new(info(), None);
    let doc = workbench.document_mut();
    report.once("Add it to the scene", 2_000, || {
        doc.execute(scene::Command::Create(scene::ObjectKind::Mesh(
            std::sync::Arc::new(ball),
        )))
        .unwrap()
    });
    let heavy = doc.selection()[0];
    doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
        scene::PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    let mut moved = doc.scene().object(cube).unwrap().transform;
    moved.position = Vec3::new(250.0, 0.0, 0.0);
    doc.execute(scene::Command::SetTransforms(vec![(cube, moved)]))
        .unwrap();
    doc.clear_selection();

    let mut harness = harness_for(workbench);
    report.frames(&mut harness, "First frame showing it", 3_000, 1, |_, _| {});
    report.frames(
        &mut harness,
        "A frame where nothing changes",
        16,
        10,
        |_, _| {},
    );
    // The shape is indexed for picking in the background meanwhile; the
    // window keeps working. Wait for it, as a person would take a moment
    // before clicking.
    report.once(
        "Index it in the background (window keeps working)",
        5_000,
        || {
            let start = Instant::now();
            while harness.state().workbench.components.preparing()
                && start.elapsed() < Duration::from_secs(60)
            {
                std::thread::sleep(Duration::from_millis(5));
            }
        },
    );

    // Orbiting: Alt + left drag.
    let middle = on_screen(&harness, Vec3::new(0.0, 0.0, radius));
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    pointer(&mut harness, middle, Some(true), Modifiers::ALT);
    step(&mut harness);
    report.frames(
        &mut harness,
        "Orbit the view (each frame)",
        16,
        10,
        |h, i| {
            pointer(
                h,
                middle + egui::vec2(8.0 * (i + 1) as f32, 0.0),
                None,
                Modifiers::ALT,
            );
        },
    );
    pointer(
        &mut harness,
        middle + egui::vec2(80.0, 0.0),
        Some(false),
        Modifiers::ALT,
    );
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    step(&mut harness);
    step(&mut harness);

    // Working on another object beside it.
    let cube_spot = on_screen(&harness, Vec3::new(250.0, 0.0, 50.0));
    report.frames(
        &mut harness,
        "Click the cube beside it (select)",
        50,
        1,
        |h, _| {
            pointer(h, cube_spot, Some(true), Modifiers::NONE);
            pointer(h, cube_spot, Some(false), Modifiers::NONE);
        },
    );
    step(&mut harness);
    assert_eq!(harness.state().workbench.document().selection(), [cube]);
    harness.key_press(Key::W);
    step(&mut harness);
    step(&mut harness);
    let arrow = gizmo_x_arrow(&harness);
    pointer(&mut harness, arrow, None, Modifiers::NONE);
    step(&mut harness);
    pointer(&mut harness, arrow, Some(true), Modifiers::NONE);
    step(&mut harness);
    report.frames(
        &mut harness,
        "Drag the cube with the gizmo (each frame)",
        33,
        10,
        |h, i| {
            pointer(
                h,
                arrow + egui::vec2(4.0 * (i + 1) as f32, 0.0),
                None,
                Modifiers::NONE,
            );
        },
    );
    pointer(
        &mut harness,
        arrow + egui::vec2(40.0, 0.0),
        Some(false),
        Modifiers::NONE,
    );
    step(&mut harness);
    harness.key_press(Key::Q);
    step(&mut harness);

    // Selecting the heavy object.
    let ball_spot = on_screen(&harness, on_ball(&harness, radius, 0.0));
    report.frames(
        &mut harness,
        "Click the heavy object (select)",
        50,
        1,
        |h, _| {
            pointer(h, ball_spot, Some(true), Modifiers::NONE);
            pointer(h, ball_spot, Some(false), Modifiers::NONE);
        },
    );
    step(&mut harness);
    assert_eq!(harness.state().workbench.document().selection(), [heavy]);

    // Polygons mode on it.
    report.frames(&mut harness, "Switch to Polygons mode", 3_000, 1, |h, _| {
        h.key_press(Key::Num3);
    });
    step(&mut harness);
    report.frames(
        &mut harness,
        "Hover over its polygons (each frame)",
        50,
        10,
        |h, i| {
            let spot = on_screen(h, on_ball(h, radius, 0.02 * i as f32));
            pointer(h, spot, None, Modifiers::NONE);
        },
    );
    let spot = on_screen(&harness, on_ball(&harness, radius, 0.05));
    step(&mut harness);
    report.frames(&mut harness, "Click a polygon (select)", 50, 1, |h, _| {
        pointer(h, spot, Some(true), Modifiers::NONE);
        pointer(h, spot, Some(false), Modifiers::NONE);
    });
    step(&mut harness);
    let selected = harness
        .state()
        .workbench
        .document()
        .elements(heavy)
        .map_or(0, |e| e.polygons().count());
    assert_eq!(selected, 1, "one polygon selected");

    // Moving the polygon.
    harness.key_press(Key::W);
    step(&mut harness);
    step(&mut harness);
    let arrow = gizmo_x_arrow(&harness);
    pointer(&mut harness, arrow, None, Modifiers::NONE);
    step(&mut harness);
    pointer(&mut harness, arrow, Some(true), Modifiers::NONE);
    step(&mut harness);
    report.frames(
        &mut harness,
        "Drag the polygon with the gizmo (each frame)",
        33,
        10,
        |h, i| {
            pointer(
                h,
                arrow + egui::vec2(3.0 * (i + 1) as f32, 0.0),
                None,
                Modifiers::NONE,
            );
        },
    );
    pointer(
        &mut harness,
        arrow + egui::vec2(30.0, 0.0),
        Some(false),
        Modifiers::NONE,
    );
    report.frames(&mut harness, "Let go of the drag", 200, 1, |_, _| {});
    step(&mut harness);
    let ops = harness
        .state()
        .workbench
        .document()
        .scene()
        .object(heavy)
        .unwrap()
        .stack
        .operation_count();
    assert_eq!(ops, 1, "the drag recorded one edit");

    report.frames(&mut harness, "Undo the move (Ctrl + Z)", 200, 1, |h, _| {
        h.event(Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        });
    });
    step(&mut harness);
    harness.key_press(Key::Q);
    step(&mut harness);

    // Points mode, and a box over part of it.
    report.frames(&mut harness, "Switch to Points mode", 3_000, 1, |h, _| {
        h.key_press(Key::Num1);
    });
    step(&mut harness);
    report.frames(
        &mut harness,
        "Hover over its points (each frame)",
        50,
        10,
        |h, i| {
            let spot = on_screen(h, on_ball(h, radius, 0.02 * i as f32));
            pointer(h, spot, None, Modifiers::NONE);
        },
    );
    let from = ball_spot - egui::vec2(20.0, 20.0);
    pointer(&mut harness, from, Some(true), Modifiers::NONE);
    step(&mut harness);
    for i in 1..=4 {
        pointer(
            &mut harness,
            from + egui::vec2(10.0, 10.0) * i as f32,
            None,
            Modifiers::NONE,
        );
        step(&mut harness);
    }
    report.frames(
        &mut harness,
        "Box-select a patch of points (let go)",
        500,
        1,
        |h, _| {
            pointer(
                h,
                from + egui::vec2(40.0, 40.0),
                Some(false),
                Modifiers::NONE,
            );
        },
    );
    step(&mut harness);
    let points = harness
        .state()
        .workbench
        .document()
        .elements(heavy)
        .map_or(0, |e| e.points().count());
    assert!(points > 0, "the box caught some points");

    let ctrl = |h: &mut H, key| {
        h.event(Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        });
    };
    report.frames(
        &mut harness,
        "Select every point (Ctrl + A)",
        3_000,
        1,
        |h, _| {
            ctrl(h, Key::A);
        },
    );
    step(&mut harness);
    let points = harness
        .state()
        .workbench
        .document()
        .elements(heavy)
        .map_or(0, |e| e.points().count());
    assert!(points > polygons * 9 / 10, "{points} points selected");
    report.frames(&mut harness, "Undo that (Ctrl + Z)", 3_000, 1, |h, _| {
        ctrl(h, Key::Z);
    });
    step(&mut harness);

    report.frames(&mut harness, "Back to Model mode", 3_000, 1, |h, _| {
        h.key_press(Key::Num1);
    });
    step(&mut harness);

    let all_met = report.print(polygons);
    if let Some(mb) = memory_mb() {
        println!("Memory in use at the end (without the GPU's copy): about {mb} MB");
    }
    assert!(all_met, "some steps missed their targets (see the table)");
}

/// How much memory the process uses, in MB (Linux only).
fn memory_mb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

/// Where the gizmo's X arrow is, a little in from its tip.
fn gizmo_x_arrow(harness: &H) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = Vec2::new(rect.width(), rect.height());
    let gizmo = harness
        .state()
        .workbench
        .viewport()
        .gizmo()
        .expect("a gizmo");
    let length = gizmo.length(&camera(harness), size);
    on_screen(harness, gizmo.origin + gizmo.axes[0] * length * 0.8)
}
