//! The material editor's smoothness check: a big graph (and the biggest
//! starter graph) on the node canvas, panned and zoomed through the real
//! interface, timing each frame and counting the work each pan or zoom
//! causes: graphs worked out, boxes worked out, box textures made, and
//! what is sent to the graphics card (box pictures and the font atlas).
//!
//! Panning and zooming only move what is drawn, so they should cost no
//! graph work and no new textures at all. Not part of the normal test
//! run (pictures are worked out at the app's full size on another
//! thread):
//!
//! `cargo test --release -p ui material_smoothness -- --ignored --nocapture`

use super::heavy_check::H;
use super::tests::info;
use super::*;
use crate::material::look::COLUMN;
use crate::material::pictures::Pictures;
use egui::{Event, Modifiers, MouseWheelUnit, PointerButton};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Frame, Graph, GraphEdit, PlugRef, Registry, ValueKind};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What reached the graphics card.
#[derive(Debug, Default, Clone, Copy)]
struct Uploads {
    /// Box pictures and other images made or changed.
    images: u64,
    image_bytes: u64,
    /// Changes to the font atlas (new glyphs drawn into it), and how many
    /// times it was sent whole (made again from nothing).
    font: u64,
    font_bytes: u64,
    font_whole: u64,
}

/// A stand-in for the graphics card that counts what is sent to it.
#[derive(Clone, Default)]
struct Counting(Arc<Mutex<Uploads>>);

impl egui_kittest::TestRenderer for Counting {
    fn handle_delta(&mut self, delta: &mut egui::TexturesDelta) {
        let mut up = self.0.lock().expect("counts");
        for (id, images) in delta.set.drain() {
            for image in images {
                let bytes = (image.image.width() * image.image.height() * 4) as u64;
                if id == egui::TextureId::default() {
                    up.font += 1;
                    up.font_bytes += bytes;
                    if image.pos.is_none() {
                        up.font_whole += 1;
                    }
                } else {
                    up.images += 1;
                    up.image_bytes += bytes;
                }
            }
        }
        delta.free.clear();
    }

    fn render(
        &mut self,
        _ctx: &egui::Context,
        _output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        Err("nothing is drawn in the smoothness check".into())
    }
}

/// One measured gesture.
struct Row {
    what: String,
    frames: usize,
    slowest: Duration,
    average: Duration,
    /// The slowest frame after the gesture, while its work finishes.
    after: Duration,
    /// Times the preview laid the material on its shape again.
    lays: u64,
    runs: u64,
    boxes_made: u64,
    textures: u64,
    up: Uploads,
}

/// A graph of `lanes` rows of `per_lane` boxes: a pattern then boxes
/// that change a height, joined in a chain, a Blend in each row taking
/// from the row above, each row in a comment frame with a note, and
/// reroute dots on every third wire.
fn stress_graph(lanes: usize, per_lane: usize) -> Graph {
    let registry = Registry::standard();
    let height_out = |info: &matgraph::BoxInfo| {
        info.outputs.first().is_some_and(|o| o.kind == ValueKind::Height)
    };
    let mut sources = Vec::new();
    let mut changers = Vec::new();
    for t in registry.iter() {
        let info = t.info();
        if !height_out(info) || matches!(&*info.id, "blend" | "time" | "pulse") {
            continue;
        }
        let needed = info.inputs.iter().filter(|p| p.needed).count();
        if info.inputs.is_empty() {
            sources.push(info.id.to_string());
        } else if needed <= 1 && info.inputs[0].kind == ValueKind::Height {
            changers.push(info.id.to_string());
        }
    }
    assert!(!sources.is_empty() && !changers.is_empty());
    let mut graph = Graph::new("Stress");
    let apply = |graph: &mut Graph, edit: GraphEdit| graph.apply(registry, &edit);
    let row_height = 300.0;
    let mut rows: Vec<Vec<BoxId>> = Vec::new();
    let mut next = 1u64;
    for lane in 0..lanes {
        let y = lane as f32 * row_height;
        let mut row = Vec::new();
        for col in 0..per_lane {
            let kind = if col == 0 {
                sources[lane % sources.len()].clone()
            } else if col == per_lane / 2 && lane > 0 {
                "blend".to_string()
            } else {
                changers[(lane * 7 + col) % changers.len()].clone()
            };
            let at = Vec2::new(col as f32 * COLUMN, y + 70.0);
            apply(&mut graph, GraphEdit::AddBox { kind, at }).expect("box");
            row.push(BoxId(next));
            next += 1;
        }
        let max = Vec2::new(per_lane as f32 * COLUMN, y + row_height - 20.0);
        let mut frame = Frame::new(format!("Row {}", lane + 1), Vec2::new(-20.0, y), max);
        frame.note = "A row of boxes made for the smoothness check: a pattern, then \
                      boxes that change its height one after another."
            .into();
        apply(&mut graph, GraphEdit::AddFrame(frame)).expect("frame");
        rows.push(row);
    }
    let first_height_input = |graph: &Graph, id: BoxId, skip: usize| {
        let kind = &graph.get(id)?.kind;
        let info = registry.get(kind)?.info();
        info.inputs
            .iter()
            .filter(|p| p.kind == ValueKind::Height)
            .nth(skip)
            .map(|p| PlugRef::new(id, p.key.to_string()))
    };
    let output = |graph: &Graph, id: BoxId| {
        let kind = &graph.get(id)?.kind;
        let info = registry.get(kind)?.info();
        info.outputs
            .first()
            .map(|p| PlugRef::new(id, p.key.to_string()))
    };
    let mut wires = 0;
    for (lane, row) in rows.iter().enumerate() {
        for pair in row.windows(2) {
            let (Some(from), Some(to)) = (output(&graph, pair[0]), first_height_input(&graph, pair[1], 0))
            else {
                continue;
            };
            if apply(&mut graph, GraphEdit::Connect { from, to: to.clone() }).is_ok() {
                wires += 1;
                if wires % 3 == 0 {
                    let a = graph.get(pair[0]).map(|b| b.at).unwrap_or_default();
                    let points = vec![a + Vec2::new(170.0, 120.0), a + Vec2::new(190.0, 40.0)];
                    let _ = apply(&mut graph, GraphEdit::SetBends { to, points });
                }
            }
        }
        // The Blend's other input, from the row above.
        if lane > 0 {
            let blend = row[row.len() / 2];
            let above = rows[lane - 1][row.len() / 2 - 1];
            if let (Some(from), Some(to)) = (output(&graph, above), first_height_input(&graph, blend, 1)) {
                let _ = apply(&mut graph, GraphEdit::Connect { from, to });
            }
        }
    }
    graph
}

/// The biggest starter graph.
fn biggest_starter() -> (String, Graph) {
    fileio::starter_graphs::STARTERS
        .iter()
        .filter_map(|s| Some((s.key.to_string(), s.load().ok()?)))
        .filter(|(_, f)| f.own.is_empty())
        .map(|(k, f)| (k, f.graph))
        .max_by_key(|(_, g)| g.boxes().count())
        .expect("a starter")
}

/// The Material mode showing `graph`, its pictures worked out on a
/// thread at the app's sizes, with the uploads counted.
fn page(graph: Graph) -> (H, Counting) {
    let mut workbench = Workbench::new(info(), None);
    workbench
        .document_mut()
        .execute(scene::Command::AddGraph(graph))
        .expect("graph");
    workbench.mode = Mode::Material;
    workbench.material.pictures = Pictures::on_a_thread();
    let counting = Counting::default();
    let mut harness = super::tests::harness_built_sized(
        egui_kittest::Harness::builder()
            .with_step_dt(1.0 / 60.0)
            .with_max_steps(10_000)
            .renderer(counting.clone()),
        workbench,
        super::tests::TestRenderer::default(),
        [1600.0, 1000.0],
    );
    // As big a font atlas as a graphics card gives the app.
    harness.input_mut().max_texture_side = Some(8192);
    (harness, counting)
}

/// One frame, timed with its tessellation (what the app does before
/// drawing).
fn frame(harness: &mut H) -> Duration {
    let start = Instant::now();
    harness.step();
    let shapes = harness.output().shapes.clone();
    let ppp = harness.output().pixels_per_point;
    let _ = harness.ctx.tessellate(shapes, ppp);
    let took = start.elapsed();
    harness.state_mut().renderer.drawn.clear();
    harness.state_mut().renderer.previews.clear();
    took
}

/// One frame as often as a 60 Hz screen asks for one: the pictures'
/// thread works alongside, as it does in the app.
fn paced(harness: &mut H) -> Duration {
    let took = frame(harness);
    if let Some(rest) = Duration::from_secs_f64(1.0 / 60.0).checked_sub(took) {
        std::thread::sleep(rest);
    }
    took
}

/// Frames until the pictures are all in (at most `seconds`), and a few
/// after; the slowest frame.
fn settle(harness: &mut H, seconds: f32) -> Duration {
    let until = Instant::now() + Duration::from_secs_f32(seconds);
    let mut slowest = Duration::ZERO;
    loop {
        slowest = slowest.max(paced(harness));
        let waiting = harness.state().workbench.material.pictures.waiting();
        if !waiting || Instant::now() > until {
            break;
        }
    }
    for _ in 0..5 {
        slowest = slowest.max(paced(harness));
    }
    slowest
}

/// Measures `frames` frames of a gesture (`input(i)` before frame `i`),
/// then the frames after it until the pictures settle again: all the
/// work the gesture caused.
fn measure(
    harness: &mut H,
    counting: &Counting,
    what: String,
    frames: usize,
    mut input: impl FnMut(&mut H, usize),
) -> Row {
    let before = harness.state().workbench.material.pictures.counts;
    let lays_before = harness.state().workbench.material.preview.versions;
    *counting.0.lock().expect("counts") = Uploads::default();
    let mut slowest = Duration::ZERO;
    let mut total = Duration::ZERO;
    for i in 0..frames {
        // Straight into the frame's input: `Harness::event` would run a
        // frame for each event.
        input(harness, i);
        let took = paced(harness);
        slowest = slowest.max(took);
        total += took;
    }
    let after_slowest = settle(harness, 300.0);
    let after = harness.state().workbench.material.pictures.counts;
    Row {
        what,
        frames,
        slowest,
        average: total / frames.max(1) as u32,
        after: after_slowest,
        lays: harness.state().workbench.material.preview.versions - lays_before,
        runs: after.runs - before.runs,
        boxes_made: after.boxes_made - before.boxes_made,
        textures: after.textures - before.textures,
        up: *counting.0.lock().expect("counts"),
    }
}

/// Pans, zooms in and out, and rests on the canvas of `graph`.
fn gestures(name: &str, graph: Graph) -> Vec<Row> {
    let (mut harness, counting) = page(graph);
    settle(&mut harness, 600.0);
    let canvas = harness
        .get_by_label(crate::material::CANVAS_LABEL)
        .rect();
    let middle = canvas.center();
    let mut rows = Vec::new();
    // Middle drag: 60 frames of 6 points each way.
    rows.push(measure(
        &mut harness,
        &counting,
        format!("{name}: pan (middle drag)"),
        62,
        |h, i| {
            let at = middle + egui::vec2(6.0, 3.0) * i as f32;
            h.input_mut().events.push(Event::PointerMoved(at));
            if i == 0 || i == 61 {
                h.input_mut().events.push(Event::PointerButton {
                    pos: at,
                    button: PointerButton::Middle,
                    pressed: i == 0,
                    modifiers: Modifiers::NONE,
                });
            }
        },
    ));
    // The second time round, every text size has been seen before.
    for (dir, label) in [
        (1.0, "zoom in (wheel)"),
        (-1.0, "zoom out (wheel)"),
        (1.0, "zoom in again"),
        (-1.0, "zoom out again"),
    ] {
        rows.push(measure(
            &mut harness,
            &counting,
            format!("{name}: {label}"),
            60,
            |h, i| {
                if i % 2 == 0 {
                    h.input_mut().events.push(Event::MouseWheel {
                        unit: MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, dir),
                        modifiers: Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    });
                }
                h.input_mut().events.push(Event::PointerMoved(middle + egui::vec2(40.0, 20.0)));
            },
        ));
    }
    rows.push(measure(
        &mut harness,
        &counting,
        format!("{name}: resting"),
        30,
        |_, _| {},
    ));
    let asked = harness
        .output()
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map(|v| v.repaint_delay);
    println!("{name}: resting, the next frame asked for after {asked:?}");
    rows
}

#[test]
#[ignore = "a timing check: run with --ignored --nocapture (release)"]
fn material_smoothness() {
    let (starter, graph) = biggest_starter();
    let mut rows = gestures(&format!("starter {starter}"), graph);
    let stress = stress_graph(10, 15);
    println!("stress graph: {} boxes", stress.boxes().count());
    rows.extend(gestures("stress (150 boxes)", stress));
    println!();
    println!(
        "{:<38} {:>6} {:>9} {:>9} {:>9} {:>5} {:>6} {:>6} {:>5} {:>7} {:>9} {:>6} {:>9} {:>5}",
        "",
        "frames",
        "slowest",
        "average",
        "after",
        "runs",
        "boxes",
        "texts",
        "lays",
        "images",
        "img KB",
        "font",
        "font KB",
        "whole"
    );
    for r in &rows {
        println!(
            "{:<38} {:>6} {:>7.1}ms {:>7.1}ms {:>7.1}ms {:>5} {:>6} {:>6} {:>5} {:>7} {:>9} {:>6} {:>9} {:>5}",
            r.what,
            r.frames,
            r.slowest.as_secs_f64() * 1000.0,
            r.average.as_secs_f64() * 1000.0,
            r.after.as_secs_f64() * 1000.0,
            r.runs,
            r.boxes_made,
            r.textures,
            r.lays,
            r.up.images,
            r.up.image_bytes / 1024,
            r.up.font,
            r.up.font_bytes / 1024,
            r.up.font_whole,
        );
    }
    println!();
    // The preview lays the material at 64 pixels in tests; in the app it
    // is laid at up to 2048, on the interface thread, each time.
    let graph = biggest_starter().1;
    let mut evaluator = matgraph::Evaluator::new();
    let value = evaluator
        .material(Registry::standard(), &graph, crate::material::pictures::full())
        .map(matgraph::Value::Material)
        .expect("the starter's material");
    for size in [512, 2048] {
        let start = Instant::now();
        let _ = crate::material::laying::laid(&value, 4.0, size);
        println!(
            "the preview laying a material at {size} pixels: {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
