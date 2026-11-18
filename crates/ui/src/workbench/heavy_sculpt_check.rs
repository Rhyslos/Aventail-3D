//! The heavy sculpting check (Phase 17a milestone 1, decision 0304): the
//! heavy test shape (three million polygons as standard) sculpted in the
//! real interface, timing what a person does against the plan's targets:
//! making it ready, strokes with about 20 000 and 200 000 points in reach,
//! Smooth, Clay strips, Grab and Snake hook with the big brush (decision
//! 0306), a stamp of a 1024-pixel alpha dragged over about 200 000 points
//! (decision 0308), undo and redo, and saving and opening a project with a sculpt.
//!
//! Not part of the normal test run: `tools/heavy/heavy-check.sh` runs it
//! with the heavy-mesh check (release build). `HEAVY_POLYGONS` sets the
//! shape's size.

use super::heavy_check::{H, Report, on_screen, pointer, step};
use super::tests::{harness_for, info};
use super::*;
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use glam::Vec3;
use std::time::{Duration, Instant};

/// The brush radius on screen, in points, reaching `points` points
/// around the ball's front.
fn radius_for(harness: &H, front: Vec3, total: usize, points: usize) -> f32 {
    let area = 4.0 * std::f32::consts::PI * 100.0 * 100.0;
    let world = (points as f32 / total as f32 * area / std::f32::consts::PI).sqrt();
    let view = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(view.width(), view.height());
    let camera = *harness.state().workbench.viewport().camera();
    world / camera.world_per_pixel(front, size).max(1e-6)
}

/// A stroke across the ball's front, holding `modifiers`, timing its
/// start, each frame and its end.
fn timed_stroke(
    harness: &mut H,
    report: &mut Report,
    modifiers: Modifiers,
    what: [&'static str; 3],
    targets_ms: [u64; 3],
) {
    let from = on_screen(harness, Vec3::new(-30.0, -100.0, 100.0));
    let to = on_screen(harness, Vec3::new(30.0, -100.0, 100.0));
    harness.event(egui::Event::ModifiersChanged(modifiers));
    pointer(harness, from, None, modifiers);
    step(harness);
    report.frames(harness, what[0], targets_ms[0], 1, |h, _| {
        pointer(h, from, Some(true), modifiers);
    });
    report.frames(harness, what[1], targets_ms[1], 30, |h, i| {
        let t = (i + 1) as f32 / 30.0;
        pointer(h, from + (to - from) * t, None, modifiers);
    });
    report.frames(harness, what[2], targets_ms[2], 1, |h, _| {
        pointer(h, to, Some(false), modifiers);
    });
    harness.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    step(harness);
}

/// The heavy sculpting check. Prints a table and fails if a target is
/// missed.
#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn heavy_sculpt_check() {
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
    let mut harness = harness_for(workbench);
    harness.run();
    let id = harness.state().workbench.document().selection()[0];
    let total = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(id)
        .unwrap()
        .mesh
        .vertex_count();
    harness.state_mut().workbench.mode = Mode::Sculpt;
    report.frames(
        &mut harness,
        "Enter Sculpt mode (the shape made ready on another thread)",
        100,
        1,
        |_, _| {},
    );
    let start = Instant::now();
    while harness.state().workbench.sculpting.sculpted().is_none() {
        std::thread::sleep(Duration::from_millis(10));
        step(&mut harness);
        assert!(start.elapsed() < Duration::from_secs(120), "never ready");
    }
    let ready = start.elapsed();
    report.record(
        "Ready to sculpt (in the background, frames go on)",
        ready,
        5_000,
    );
    report.frames(
        &mut harness,
        "A frame where nothing changes",
        16,
        10,
        |_, _| {},
    );

    let front = Vec3::new(0.0, -100.0, 100.0);
    let history_before = harness.state().workbench.document().history_memory();
    for (points, what, targets) in [
        (
            20_000,
            [
                "Start a stroke (about 20 000 points in reach)",
                "Sculpt with it (each frame)",
                "Finish the stroke (one undo step)",
            ],
            [33, 16, 150],
        ),
        (
            200_000,
            [
                "Start a stroke (about 200 000 points in reach)",
                "Sculpt with it (each frame)",
                "Finish the stroke (one undo step)",
            ],
            [60, 33, 400],
        ),
    ] {
        let radius = radius_for(&harness, front, total, points);
        for brush in &mut harness.state_mut().workbench.attributes.sculpt.brushes {
            brush.radius = radius;
        }
        timed_stroke(&mut harness, &mut report, Modifiers::NONE, what, targets);
    }
    timed_stroke(
        &mut harness,
        &mut report,
        Modifiers::SHIFT,
        [
            "Start Smooth (about 200 000 points in reach)",
            "Smooth (each frame)",
            "Finish Smooth",
        ],
        [60, 33, 400],
    );
    // The brushes for forms the plan times (decision 0306), with the big
    // brush.
    for (kind, what) in [
        (
            sculpt::BrushKind::ClayStrips,
            [
                "Start Clay strips (about 280 000 points in reach)",
                "Clay strips (each frame)",
                "Finish Clay strips",
            ],
        ),
        (
            sculpt::BrushKind::Grab,
            [
                "Take hold with Grab (about 200 000 points)",
                "Grab (each frame)",
                "Finish Grab",
            ],
        ),
        (
            sculpt::BrushKind::SnakeHook,
            [
                "Start Snake hook (about 200 000 points in reach)",
                "Snake hook (each frame)",
                "Finish Snake hook",
            ],
        ),
    ] {
        harness.state_mut().workbench.attributes.sculpt.brush = kind;
        timed_stroke(
            &mut harness,
            &mut report,
            Modifiers::NONE,
            what,
            [60, 33, 400],
        );
    }
    // A stamp of a 1024-pixel alpha (decision 0308), dragged across the
    // front: laid again each frame, as big as the drag.
    let side = sculpt::alpha::MAX_ALPHA_SIDE;
    let values: Vec<u8> = (0..side * side)
        .map(|i| ((i % side) ^ (i / side)) as u8)
        .collect();
    let big = sculpt::Alpha::new(side, side, values, "big").expect("an alpha");
    {
        let settings = &mut harness.state_mut().workbench.attributes.sculpt;
        settings.brush = sculpt::BrushKind::Draw;
        settings.stroke = crate::sculpt_tab::StrokeKind::Stamp;
        settings.alpha = Some(std::sync::Arc::new(big));
    }
    timed_stroke(
        &mut harness,
        &mut report,
        Modifiers::NONE,
        [
            "Press a stamp (a 1024-pixel alpha)",
            "Drag the stamp, up to about 200 000 points (each frame)",
            "Finish the stamp",
        ],
        [60, 33, 400],
    );
    {
        let settings = &mut harness.state_mut().workbench.attributes.sculpt;
        settings.stroke = crate::sculpt_tab::StrokeKind::Freehand;
        settings.alpha = None;
    }
    let history = harness.state().workbench.document().history_memory() - history_before;
    report.frames(&mut harness, "Undo a stroke", 50, 1, |h, _| {
        h.state_mut().workbench.document_mut().undo();
    });
    report.frames(&mut harness, "Redo it", 50, 1, |h, _| {
        h.state_mut().workbench.document_mut().redo();
    });

    // Saving and opening a project holding the sculpt.
    let doc = harness.state().workbench.document();
    let bytes = report.once(
        "Save the project with its sculpt (to memory)",
        5_000,
        || fileio::project::to_bytes(doc).expect("saved"),
    );
    let back = report.once("Open it again (and work its shape out)", 5_000, || {
        let back = fileio::project::from_bytes(&bytes).expect("opened");
        let _ = back.scene().evaluated(id);
        back
    });
    assert_eq!(
        *back.scene().evaluated(id).unwrap().mesh,
        *doc.scene().evaluated(id).unwrap().mesh
    );
    let ok = report.print_titled(&format!(
        "Heavy sculpting check: the heavy test shape of {polygons} polygons ({total} points)"
    ));
    println!(
        "The history's growth over seven strokes: {:.0} MB; the project file: {:.0} MB",
        history as f64 / 1e6,
        bytes.len() as f64 / 1e6
    );
    assert!(ok, "a target was missed (SLOW above)");
}
