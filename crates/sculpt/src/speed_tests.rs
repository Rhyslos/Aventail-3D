//! The live sculpt's speed on the heavy test shape (decision 0313): every
//! brush with about 140 000 points in reach, a symmetric stroke, finishing
//! a stroke, and undoing and redoing it. Ignored in the normal test run:
//! `cargo test --release -p sculpt speed_benchmark -- --ignored --nocapture`
//! (`HEAVY_POLYGONS` sets the shape's size, three million as standard).
//! The heavy-mesh check (`tools/heavy/heavy-check.sh`) times the same in
//! the real interface.

use super::*;
use glam::Vec3;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn dab(kind: BrushKind, centre: Vec3, radius: f32) -> Dab {
    Dab {
        kind,
        centre,
        radius,
        strength: 0.5,
        falloff: Falloff::Smooth,
        invert: false,
    }
}

/// Milliseconds, for printing.
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// Ten dabs of `kind` across the front, after a first that takes hold
/// for the grabbing brushes; the average and slowest dab, and the points
/// the last moved.
fn ten_dabs(live: &mut LiveSculpt, kind: BrushKind, at: Vec3, radius: f32) -> (f64, f64, usize) {
    if kind.grabs() {
        live.dab(&dab(kind, at, radius));
    }
    let (mut total, mut slowest, mut moved) = (Duration::ZERO, Duration::ZERO, 0);
    for i in 1..=10 {
        let way = if kind.grabs() { Vec3::NEG_Y } else { Vec3::X };
        let start = Instant::now();
        moved = live.dab(&dab(kind, at + way * i as f32, radius));
        let took = start.elapsed();
        // Handed out to be drawn, as each frame does: the next dab copies
        // the chunks it shares.
        let _ = live.shape();
        total += took;
        slowest = slowest.max(took);
    }
    (ms(total) / 10.0, ms(slowest), moved)
}

#[test]
#[ignore = "a benchmark"]
fn speed_benchmark() {
    let polygons: usize = std::env::var("HEAVY_POLYGONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000_000);
    let input = Arc::new(mesh::samples::heavy_ball(polygons, 100.0));
    let start = Instant::now();
    let mut live = LiveSculpt::new(Arc::clone(&input), None);
    println!(
        "Live sculpt of {} points ready in {:.0} ms",
        input.vertex_count(),
        ms(start.elapsed())
    );
    let at = live
        .hit(Vec3::new(0.0, -500.0, 100.0), Vec3::Y)
        .expect("the front")
        .point;
    // About 140 000 points in reach on the three-million-polygon shape.
    let radius = 52.0 * (3_000_000.0 / polygons as f32).sqrt();
    println!(
        "{:<14} {:>9} {:>9} {:>8}",
        "brush", "average", "slowest", "points"
    );
    for kind in BrushKind::ALL {
        let (average, slowest, moved) = ten_dabs(&mut live, kind, at, radius);
        println!(
            "{:<14} {average:>7.1}ms {slowest:>7.1}ms {moved:>8}",
            kind.name()
        );
        let start = Instant::now();
        live.finish();
        println!("{:<14} finished in {:.1} ms", "", ms(start.elapsed()));
    }
    // A symmetric stroke: the dab and its mirror copy.
    live.set_symmetry(Symmetry {
        axes: [true, false, false],
        radial: None,
    });
    let (average, slowest, moved) = ten_dabs(&mut live, BrushKind::Draw, at, radius);
    println!(
        "{:<14} {average:>7.1}ms {slowest:>7.1}ms {moved:>8}",
        "Symmetric"
    );
    live.finish();
    live.set_symmetry(Symmetry::NONE);

    // Undo and redo of a big stroke, as the interface does them.
    // The shape stays shown (drawn, and kept by the scene) meanwhile, as
    // in the app, so the points moved share their chunks with it.
    let before = live.sculpt().clone();
    ten_dabs(&mut live, BrushKind::Draw, at, radius);
    let shown = live.shape();
    let start = Instant::now();
    let after = live.finish().expect("a stroke");
    let finish = start.elapsed();
    let shown = (shown, live.shape());
    let start = Instant::now();
    live.follow(&before);
    let undo = start.elapsed();
    let shown = (shown, live.shape());
    let start = Instant::now();
    live.follow(&after);
    let redo = start.elapsed();
    drop(shown);
    println!("Finish a big stroke while it is shown {:.1} ms", ms(finish));
    println!(
        "Undo a big stroke {:.1} ms, redo it {:.1} ms; the step keeps {:.1} MB",
        ms(undo),
        ms(redo),
        (before.memory_apart(&after) + after.memory_apart(&before)) as f64 / 1e6
    );
}
