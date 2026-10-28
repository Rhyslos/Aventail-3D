//! The heavy painting check (Phase 11 milestone 10, decision 0188): a 4K
//! texture set of 20 layers on the cube, painted in the real interface,
//! timing what a person does against the targets: the first frame, brush
//! strokes small and large, undo and redo, and how much memory the history
//! keeps.
//!
//! Not part of the normal test run: `tools/heavy/heavy-check.sh` runs it
//! with the heavy-mesh check (release build). `HEAVY_TEXTURE` sets the
//! texture size (4096 as standard) and `HEAVY_LAYERS` the layer count (20).
//! For profiling, `HEAVY_REPEAT=radius:count` paints that many strokes of
//! that radius after the first frame, and nothing else.
//!
//! The targets allow for a two-core machine: a stroke's start mixes the
//! layers under the newly reached tiles and a brush as big as the object
//! reaches millions of texels at once; more cores make both quicker.

use super::brushing_tests::painting;
use super::heavy_check::{H, Report, on_screen, pointer, step};
use egui::Modifiers;
use glam::Vec3;
use paint::{Image, TextureSet};
use std::sync::Arc;

/// A texture set of `layers` paint layers, `size` pixels across, each
/// painted over a quarter of its tiles (a different quarter each, so
/// every tile has paint in some layer), as a well-worked texture would be.
fn worked_set(size: u32, layers: usize) -> TextureSet {
    let mut set = TextureSet::new(size).expect("a texture size");
    for n in 0..layers {
        let path = set
            .add_paint_layer(&format!("Layer {n}"), None)
            .expect("room for the layer");
        let image: &mut Image = set
            .layer_mut(&path)
            .and_then(|l| l.image_mut())
            .expect("a paint layer");
        for index in 0..image.tile_count() {
            if (index + n) % 4 != 0 {
                continue;
            }
            // Half see-through, so every layer shows in the mix.
            let shade = (n * 12 % 256) as u8;
            let tile: Vec<u8> =
                [shade, 255 - shade, (index % 256) as u8, 128].repeat(paint::TILE_BYTES / 4);
            image.set_tile(index, Some(Arc::new(tile)));
        }
    }
    set
}

/// A stroke across the cube's front with a brush of `radius` points,
/// timing its start, each frame of it and its end against `target_ms`.
fn timed_stroke(
    harness: &mut H,
    report: &mut Report,
    radius: f32,
    what: [&'static str; 3],
    targets_ms: [u64; 3],
) {
    let from = on_screen(harness, Vec3::new(-40.0, -50.0, 50.0));
    let to = on_screen(harness, Vec3::new(40.0, -50.0, 50.0));
    harness.state_mut().workbench.attributes.brush.brush.radius = radius;
    pointer(harness, from, None, Modifiers::NONE);
    step(harness);
    report.frames(harness, what[0], targets_ms[0], 1, |h, _| {
        pointer(h, from, Some(true), Modifiers::NONE);
    });
    report.frames(harness, what[1], targets_ms[1], 20, |h, i| {
        let t = (i + 1) as f32 / 20.0;
        pointer(h, from + (to - from) * t, None, Modifiers::NONE);
    });
    report.frames(harness, what[2], targets_ms[2], 1, |h, _| {
        pointer(h, to, Some(false), Modifiers::NONE);
    });
    step(harness);
}

/// The heavy painting check. Prints a table and fails if a target is
/// missed.
#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn heavy_paint_check() {
    let size: u32 = std::env::var("HEAVY_TEXTURE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4096);
    let layers: usize = std::env::var("HEAVY_LAYERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let mut report = Report::default();
    let mut harness = painting();
    let cube = harness.state().workbench.document().scene().walk()[0].0;
    let set = report.once(
        "Make the test texture set (not timed against users)",
        60_000,
        || worked_set(size, layers),
    );
    {
        let doc = harness.state_mut().workbench.document_mut();
        doc.set_selection([cube]);
        let mut texture = doc.scene().texture();
        texture.size = size;
        doc.execute(scene::Command::SetTexture(texture)).unwrap();
        doc.execute(scene::Command::SetTextureSet {
            material: "Default".into(),
            set: Some(set),
        })
        .unwrap();
    }
    report.frames(
        &mut harness,
        "First frame showing it (mixing every layer)",
        3_000,
        1,
        |_, _| {},
    );
    // For profiling: HEAVY_REPEAT=radius:count strokes, and nothing else.
    if let Some((radius, count)) = std::env::var("HEAVY_REPEAT").ok().and_then(|v| {
        let (r, n) = v.split_once(':')?;
        Some((r.parse::<f32>().ok()?, n.parse::<usize>().ok()?))
    }) {
        for _ in 0..count {
            timed_stroke(
                &mut harness,
                &mut report,
                radius,
                ["Start", "Each frame", "End"],
                [100, 50, 50],
            );
        }
        report.print_titled("Repeated strokes (profiling)");
        return;
    }
    // The history from here on: the set itself is one big step.
    let history_before = harness.state().workbench.document().history_memory();
    report.frames(
        &mut harness,
        "A frame where nothing changes",
        16,
        10,
        |_, _| {},
    );

    // A stroke across the cube's front: small, then large.
    timed_stroke(
        &mut harness,
        &mut report,
        20.0,
        [
            "Start a stroke (20-point brush)",
            "Paint with it (each frame)",
            "Finish the stroke",
        ],
        [100, 50, 50],
    );
    timed_stroke(
        &mut harness,
        &mut report,
        200.0,
        [
            "Start a stroke (200 points: the whole cube)",
            "Paint with it (each frame)",
            "Finish the stroke",
        ],
        [500, 150, 50],
    );
    report.frames(&mut harness, "Undo a stroke", 150, 1, |h, _| {
        h.state_mut().workbench.document_mut().undo();
    });
    report.frames(&mut harness, "Redo it", 150, 1, |h, _| {
        h.state_mut().workbench.document_mut().redo();
    });

    // The history after many strokes: only the tiles each touched.
    for i in 0..20 {
        let y = 20.0 + i as f32 * 3.0;
        let a = on_screen(&harness, Vec3::new(-40.0, -50.0, y));
        let b = on_screen(&harness, Vec3::new(40.0, -50.0, y));
        harness.state_mut().workbench.attributes.brush.brush.radius = 10.0;
        pointer(&mut harness, a, None, Modifiers::NONE);
        step(&mut harness);
        pointer(&mut harness, a, Some(true), Modifiers::NONE);
        step(&mut harness);
        pointer(&mut harness, b, None, Modifiers::NONE);
        step(&mut harness);
        pointer(&mut harness, b, Some(false), Modifiers::NONE);
        step(&mut harness);
    }
    let history = harness.state().workbench.document().history_memory() - history_before;
    let layer_bytes = (size as usize).pow(2) * 4;
    let ok = report.print_titled(&format!(
        "Heavy painting check: a {size} px texture set of {layers} layers"
    ));
    println!(
        "The history's growth over 22 strokes: {:.0} MB (one full layer is {:.0} MB)",
        history as f64 / 1e6,
        layer_bytes as f64 / 1e6
    );
    // Each stroke keeps only the tiles it touched: far less than a layer
    // a stroke.
    assert!(
        history < layer_bytes * 2,
        "the history keeps too much: {history} bytes"
    );
    assert!(ok, "some steps missed their targets (marked SLOW)");
}
