//! The heavy sculpting check (Phase 17a milestone 1, decision 0304): the
//! heavy test shape (three million polygons as standard) sculpted in the
//! real interface, timing what a person does against the plan's targets:
//! making it ready, strokes with about 20 000 and 200 000 points in reach,
//! Smooth, Clay strips, Grab and Snake hook with the big brush (decision
//! 0306), a stamp of a 1024-pixel alpha dragged over about 200 000 points
//! (decision 0308), a symmetric stroke, inverting and growing a mask and
//! the gizmo moving the unmasked half (decision 0310), undo and redo, and
//! saving and opening a project with a sculpt. Decision 0313 (Phase 17a
//! milestone 6) adds every other brush, undoing and redoing a big stroke
//! and the memory it keeps, the mask's other actions and their undo, and
//! remeshing for sculpting against the plan's targets.
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
    let mut big_stroke_memory = 0;
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
        let before = harness.state().workbench.document().history_memory();
        timed_stroke(&mut harness, &mut report, Modifiers::NONE, what, targets);
        if points == 200_000 {
            big_stroke_memory = harness.state().workbench.document().history_memory() - before;
            // Undo and redo of the big stroke (target 50 ms; its places
            // are kept with it, decision 0313).
            report.frames(&mut harness, "Undo the big stroke", 50, 1, |h, _| {
                h.state_mut().workbench.document_mut().undo();
            });
            report.frames(&mut harness, "Redo it", 50, 1, |h, _| {
                h.state_mut().workbench.document_mut().redo();
            });
        }
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
        // The rest (decision 0313: every brush is measured).
        (
            sculpt::BrushKind::Clay,
            ["Start Clay", "Clay (each frame)", "Finish Clay"],
        ),
        (
            sculpt::BrushKind::Inflate,
            ["Start Inflate", "Inflate (each frame)", "Finish Inflate"],
        ),
        (
            sculpt::BrushKind::Crease,
            ["Start Crease", "Crease (each frame)", "Finish Crease"],
        ),
        (
            sculpt::BrushKind::Pinch,
            ["Start Pinch", "Pinch (each frame)", "Finish Pinch"],
        ),
        (
            sculpt::BrushKind::Flatten,
            ["Start Flatten", "Flatten (each frame)", "Finish Flatten"],
        ),
        (
            sculpt::BrushKind::Scrape,
            ["Start Scrape", "Scrape (each frame)", "Finish Scrape"],
        ),
        (
            sculpt::BrushKind::Fill,
            ["Start Fill", "Fill (each frame)", "Finish Fill"],
        ),
        (
            sculpt::BrushKind::Mask,
            [
                "Start the Mask brush",
                "Mask brush (each frame)",
                "Finish the Mask brush",
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
    // The Mask brush's mask goes before the next brushes.
    harness
        .state_mut()
        .workbench
        .run_sculpt_action(crate::sculpt_masks::SculptAction::ClearMask);
    step(&mut harness);
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
    // Symmetry and masks (decision 0310): a symmetric stroke (twice the
    // dabs), inverting and growing a mask, and the gizmo moving the
    // unmasked half.
    harness.state_mut().workbench.attributes.sculpt.symmetry.on = true;
    timed_stroke(
        &mut harness,
        &mut report,
        Modifiers::NONE,
        [
            "Start a symmetric stroke (twice the dabs)",
            "Symmetric stroke (each frame)",
            "Finish the symmetric stroke",
        ],
        [60, 33, 400],
    );
    harness.state_mut().workbench.attributes.sculpt.symmetry.on = false;
    step(&mut harness);
    // Last of the strokes, as it bends the whole shape away from where
    // the others press. Elastic grab reaches four brush sizes (choice
    // 34): with the big brush, the whole shape. Not one of the plan's
    // targets (Smooth, Grab and Snake hook are): timed to show where it
    // stands, against loose limits of our own.
    harness.state_mut().workbench.attributes.sculpt.brush = sculpt::BrushKind::ElasticGrab;
    timed_stroke(
        &mut harness,
        &mut report,
        Modifiers::NONE,
        [
            "Take hold with Elastic grab (the whole shape in reach)",
            "Elastic grab (each frame; not a target)",
            "Finish Elastic grab",
        ],
        [500, 1000, 2000],
    );
    harness.state_mut().workbench.attributes.sculpt.brush = sculpt::BrushKind::Draw;
    // Half the shape masked by a box over the left of the view.
    report.frames(&mut harness, "Mask half the shape (box)", 200, 1, |h, _| {
        let wb = &mut h.state_mut().workbench;
        if let Some(live) = wb.sculpting.live.as_mut() {
            let mesh = live.live.mesh();
            let half = sculpt::mask::set_where(mesh, live.live.mask(), 255, |p| p.x < 0.0);
            if let Some(new) = live.live.set_mask(half) {
                wb.send_sculpt_for_tests(new);
            }
        }
    });
    for (what, action) in [
        (
            "Invert the mask",
            crate::sculpt_masks::SculptAction::InvertMask,
        ),
        ("Grow the mask", crate::sculpt_masks::SculptAction::GrowMask),
        (
            "Shrink the mask",
            crate::sculpt_masks::SculptAction::ShrinkMask,
        ),
        (
            "Sharpen the mask",
            crate::sculpt_masks::SculptAction::SharpenMask,
        ),
        (
            "Soften the mask",
            crate::sculpt_masks::SculptAction::SoftenMask,
        ),
        (
            "Mask from cavity",
            crate::sculpt_masks::SculptAction::MaskFromCavity,
        ),
        (
            "Mask facing up",
            crate::sculpt_masks::SculptAction::MaskFacingUp,
        ),
        (
            "Invert it back",
            crate::sculpt_masks::SculptAction::InvertMask,
        ),
    ] {
        report.frames(&mut harness, what, 50, 1, |h, _| {
            h.state_mut().workbench.run_sculpt_action(action);
        });
    }
    report.frames(
        &mut harness,
        "Take hold of the unmasked half",
        200,
        1,
        |h, _| {
            let wb = &mut h.state_mut().workbench;
            wb.tool = crate::transforming::Tool::Move;
            wb.sculpt_gizmo_events(&[crate::viewport::GizmoEvent::Begin {
                handle: tools::Handle::Axis(2),
                slide: false,
            }]);
        },
    );
    report.frames(
        &mut harness,
        "Move the unmasked half (each frame)",
        33,
        10,
        |h, i| {
            let wb = &mut h.state_mut().workbench;
            let pointer = crate::components::ViewPointer {
                camera: tools::Camera::default(),
                size: glam::Vec2::ONE,
                at: glam::Vec2::ZERO,
            };
            wb.sculpt_gizmo_events(&[crate::viewport::GizmoEvent::Update {
                change: tools::Change::Translate(Vec3::Z * (i + 1) as f32),
                handle: tools::Handle::Axis(2),
                pointer,
            }]);
        },
    );
    report.frames(
        &mut harness,
        "Finish moving the unmasked half",
        400,
        1,
        |h, _| {
            let wb = &mut h.state_mut().workbench;
            wb.sculpt_gizmo_events(&[crate::viewport::GizmoEvent::End]);
            wb.tool = crate::transforming::Tool::Select;
        },
    );
    harness
        .state_mut()
        .workbench
        .run_sculpt_action(crate::sculpt_masks::SculptAction::ClearMask);
    step(&mut harness);
    let history = harness.state().workbench.document().history_memory() - history_before;
    report.frames(&mut harness, "Undo a change to the mask", 50, 1, |h, _| {
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
    // Remesh for sculpting (decision 0312) against the plan's targets:
    // the sculpted heavy shape to about a million polygons (5 s) and to
    // about as many as it has (5 s), and a game-sized sculpt (half a
    // million polygons) to a million (2 s). In the background in the app;
    // here the work itself is timed.
    let sculpted = doc.scene().evaluated(id).unwrap().mesh;
    let game = mesh::samples::heavy_ball(500_000, 100.0);
    for (shape, polygons, what, target) in [
        (
            &*sculpted,
            1_000_000,
            "Remesh it to about 1 million polygons",
            5_000,
        ),
        (
            &*sculpted,
            3_000_000,
            "Remesh it to about 3 million polygons",
            5_000,
        ),
        (
            &game,
            1_000_000,
            "Remesh a 500 000-polygon sculpt to about 1 million",
            2_000,
        ),
    ] {
        let area = mesh::remesh::projected_area(shape);
        let options = mesh::remesh::SculptRemeshOptions {
            voxel_size: mesh::remesh::voxel_for(area, polygons),
            ..Default::default()
        };
        let progress = mesh::remesh::Progress::default();
        let out = report.once(what, target, || {
            mesh::remesh::remesh_for_sculpting(shape, &options, None, &progress)
        });
        assert!(out.is_ok(), "{what}");
    }
    let ok = report.print_titled(&format!(
        "Heavy sculpting check: the heavy test shape of {polygons} polygons ({total} points)"
    ));
    println!(
        "The history's growth over the strokes and masks: {:.0} MB; the big stroke's undo step: \
         {:.0} MB; the project file: {:.0} MB",
        history as f64 / 1e6,
        big_stroke_memory as f64 / 1e6,
        bytes.len() as f64 / 1e6
    );
    assert!(ok, "a target was missed (SLOW above)");
}
