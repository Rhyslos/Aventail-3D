//! Tests for the starter graphs: each file matches its recipe and its
//! card, loads through the graph file reader, works out without a
//! problem, tiles seamlessly, is laid out in frames with notes, and shows
//! settings in the painter.
//!
//! `UPDATE_STARTERS=1 cargo test -p fileio starter` writes the files
//! again from the recipes; `STARTER_RENDERS=<folder> cargo test -p fileio
//! --release starter_renders -- --ignored` draws each at 512 pixels to
//! look at.

use super::recipes::materials::recipe;
use super::*;
use crate::graph_file::write_graph_file;
use matgraph::{Evaluator, Graph, Registry, SeamCheck, Size, Value};
use std::path::PathBuf;
use std::sync::Arc;

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("starters")
}

fn worked_out(graph: &Graph, side: u32) -> Arc<MaterialMaps> {
    let registry = Registry::standard();
    let size = Size::new(side).unwrap();
    let mut evaluator = Evaluator::new();
    let evaluation = evaluator.run(registry, graph, size, &[]);
    let problems: Vec<String> = evaluation
        .problems()
        .map(|(id, why)| format!("{id:?}: {}", why.reason))
        .collect();
    assert!(problems.is_empty(), "{}: {problems:?}", graph.name());
    evaluator
        .material(registry, graph, size)
        .unwrap_or_else(|why| panic!("{}: {}", graph.name(), why.reason))
}

fn png_of(rgb: &[u8], side: u32) -> Vec<u8> {
    let image = image::RgbImage::from_raw(side, side, rgb.to_vec()).unwrap();
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

/// The card picture a starter's graph makes now.
fn card_of(graph: &Graph) -> Vec<u8> {
    card_picture(&worked_out(graph, CARD_WORKED_OUT), CARD_SIDE)
}

#[test]
fn starter_files_match_their_recipes_and_cards() {
    let update = std::env::var_os("UPDATE_STARTERS").is_some();
    let mut stale = Vec::new();
    for starter in &STARTERS {
        let graph = recipe(starter.key);
        assert_eq!(graph.name(), starter.name, "the card names the graph");
        let text = write_graph_file(&GraphFile {
            graph: graph.clone(),
            own: Vec::new(),
        });
        let card = card_of(&graph);
        if update {
            let dir = folder();
            std::fs::write(dir.join(format!("{}.matgraph", starter.key)), &text).unwrap();
            std::fs::write(
                dir.join(format!("{}.png", starter.key)),
                png_of(&card, CARD_SIDE),
            )
            .unwrap();
            continue;
        }
        // Line endings aside: a checkout may turn them into CRLF.
        if starter.file.replace("\r\n", "\n") != text {
            stale.push(format!("{} (graph)", starter.key));
        }
        // The card is the picture the graph makes (within a level, for
        // rounding on other machines).
        match starter.card_picture() {
            Ok(read) if read.width == CARD_SIDE && read.height == CARD_SIDE => {
                let differs = read
                    .rgba
                    .chunks(4)
                    .zip(card.chunks(3))
                    .filter(|(a, b)| (0..3).any(|i| a[i].abs_diff(b[i]) > 2))
                    .count();
                if differs > (CARD_SIDE * CARD_SIDE / 100) as usize {
                    stale.push(format!("{} (card: {differs} pixels differ)", starter.key));
                }
            }
            _ => stale.push(format!("{} (card unreadable)", starter.key)),
        }
    }
    assert!(
        stale.is_empty(),
        "starter files differ from their recipes: {stale:?}; run \
         UPDATE_STARTERS=1 cargo test -p fileio starter"
    );
}

#[test]
fn every_starter_loads_and_works_out_seamlessly() {
    for starter in &STARTERS {
        let file = starter
            .load()
            .unwrap_or_else(|why| panic!("{}: {why}", starter.key));
        assert_eq!(file.graph.name(), starter.name);
        // At the size the Material mode's pictures use while dragging;
        // `starter_renders` checks 512 too. (At 128 pixels a mortar line
        // lying on the join is a big step, and Bricks and Metal plate put
        // one there, so the check would see a join that isn't one.)
        let check = SeamCheck::of(&Value::Material(worked_out(&file.graph, 256)));
        assert!(!check.shows(), "{}: {}", starter.key, check.describe());
        let material = worked_out(&file.graph, 128);
        // Nothing is out of range or not a number.
        for (name, grey) in [
            ("height", &material.height),
            ("roughness", &material.roughness),
            ("metal", &material.metal),
        ] {
            assert!(
                grey.pixels().iter().all(|v| (0.0..=1.0).contains(v)),
                "{}: {name}",
                starter.key
            );
        }
        // The same on another number of cores.
        let again = Evaluator::with_threads(1)
            .material(Registry::standard(), &file.graph, Size::new(64).unwrap())
            .unwrap();
        let more = Evaluator::with_threads(4)
            .material(Registry::standard(), &file.graph, Size::new(64).unwrap())
            .unwrap();
        assert_eq!(*again, *more, "{}", starter.key);
    }
}

#[test]
fn every_starter_is_laid_out_in_frames_with_notes() {
    let registry = Registry::standard();
    for starter in &STARTERS {
        let graph = starter.load().unwrap().graph;
        let frames: Vec<_> = graph.frames().collect();
        assert!(frames.len() >= 3, "{}: a frame per step", starter.key);
        for (id, frame) in &frames {
            assert!(!frame.title.is_empty() && !frame.note.is_empty());
            assert!(
                frame.note.len() <= matgraph::graph::MAX_NOTE,
                "{}",
                starter.key
            );
            assert!(
                !graph.boxes_in(*id).is_empty(),
                "{}: {}",
                starter.key,
                frame.title
            );
        }
        // Every box is in exactly one frame, and frames don't overlap.
        for (id, _) in graph.boxes() {
            let holding = frames
                .iter()
                .filter(|(f, _)| graph.boxes_in(*f).contains(&id))
                .count();
            assert_eq!(holding, 1, "{}: box {id:?}", starter.key);
        }
        for (i, (_, a)) in frames.iter().enumerate() {
            for (_, b) in &frames[i + 1..] {
                let apart = a.max.x <= b.min.x
                    || b.max.x <= a.min.x
                    || a.max.y <= b.min.y
                    || b.max.y <= a.min.y;
                assert!(apart, "{}: {} and {}", starter.key, a.title, b.title);
            }
        }
        // Wires run left to right.
        for wire in graph.wires() {
            let x = |id| graph.get(id).unwrap().at.x;
            assert!(x(wire.to.node) > x(wire.from.node), "{}", starter.key);
        }
        // Some settings are shown in the painter, and it has a real size.
        let shown = graph.shown(registry);
        assert!(
            (3..=8).contains(&shown.len()),
            "{}: {} shown",
            starter.key,
            shown.len()
        );
        let out = graph.first_of(matgraph::eval::MATERIAL_OUT).unwrap();
        assert!(
            graph.get(out).unwrap().settings.get("size_cm").is_some(),
            "{}: a real-world size of its own",
            starter.key
        );
    }
}

#[test]
fn starters_have_names_keys_and_cards_apart() {
    let keys: std::collections::BTreeSet<_> = STARTERS.iter().map(|s| s.key).collect();
    let names: std::collections::BTreeSet<_> = STARTERS.iter().map(|s| s.name).collect();
    assert_eq!(keys.len(), STARTERS.len());
    assert_eq!(names.len(), STARTERS.len());
    assert_eq!(Starter::find("bricks").unwrap().name, "Bricks");
    assert_eq!(Starter::find("leaves").unwrap().name, "Leaves");
    let cards: std::collections::BTreeSet<_> = STARTERS.iter().map(|s| s.card).collect();
    assert_eq!(cards.len(), STARTERS.len());
}

#[test]
fn the_card_picture_averages_and_lights_the_material() {
    let graph = recipe("bricks");
    let material = worked_out(&graph, 64);
    let card = card_picture(&material, 32);
    assert_eq!(card.len(), 32 * 32 * 3);
    // Asked for more than the material has: as many as it has.
    assert_eq!(card_picture(&material, 4096).len(), 64 * 64 * 3);
    assert_eq!(card_picture(&material, 0).len(), 3);
}

#[test]
fn the_leaves_starter_is_a_sheet_of_whole_cut_out_leaves() {
    let file = Starter::find("leaves").unwrap().load().unwrap();
    let m = worked_out(&file.graph, 256);
    let opacity = m.opacity.as_ref().expect("an alpha card");
    assert!(m.translucency.is_some());
    // Nothing reaches the sheet's edges: every leaf is whole.
    for i in 0..256 {
        for (x, y) in [(i, 0), (0, i), (i, 255), (255, i)] {
            assert_eq!(opacity.at(x, y), 0.0, "{x}, {y}");
        }
    }
    // Six leaves, each a card of its own, cover a good share of the sheet.
    let drawn = file
        .graph
        .boxes()
        .find_map(|(_, b)| b.drawing.clone())
        .unwrap();
    let sheet = shapes::Sheet::of(&drawn, shapes::SheetOptions::default());
    assert_eq!(sheet.cards().len(), 6);
    let covered = opacity.pixels().iter().sum::<f32>() / (256.0 * 256.0);
    assert!((0.25..0.8).contains(&covered), "{covered}");
    // The card shows the leaves over the dark checker.
    let card = card_picture(&m, 64);
    assert!(card[..3].iter().all(|v| *v < 60), "{:?}", &card[..3]);
}

/// Draws every starter at 512 pixels to look at: the colour, lit as on
/// its card, the height, and the lit picture repeated two by two (to see
/// the joins). Run by hand with
/// `STARTER_RENDERS=<folder> cargo test -p fileio --release starter_renders -- --ignored`.
#[test]
#[ignore = "draws pictures to look at; run by hand"]
fn starter_renders() {
    let Some(dir) = std::env::var_os("STARTER_RENDERS") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let only = std::env::var("STARTER").ok();
    for starter in &STARTERS {
        if only.as_deref().is_some_and(|o| o != starter.key) {
            continue;
        }
        let graph = recipe(starter.key);
        let side = 512u32;
        let m = worked_out(&graph, side);
        let check = SeamCheck::of(&Value::Material(m.clone()));
        assert!(!check.shows(), "{}: {}", starter.key, check.describe());
        let lit = card_picture(&m, side);
        let mut strip = image::RgbImage::new(side * 4, side);
        for y in 0..side {
            for x in 0..side {
                let (xi, yi) = (i64::from(x), i64::from(y));
                let c = m.colour.at(xi, yi);
                let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                strip.put_pixel(x, y, image::Rgb([b(c[0]), b(c[1]), b(c[2])]));
                let i = ((y * side + x) * 3) as usize;
                strip.put_pixel(side + x, y, image::Rgb([lit[i], lit[i + 1], lit[i + 2]]));
                let h = b(m.height.at(xi, yi));
                strip.put_pixel(2 * side + x, y, image::Rgb([h, h, h]));
                // Two by two, at half size.
                let (sx, sy) = ((x * 2) % side, (y * 2) % side);
                let j = ((sy * side + sx) * 3) as usize;
                strip.put_pixel(
                    3 * side + x,
                    y,
                    image::Rgb([lit[j], lit[j + 1], lit[j + 2]]),
                );
            }
        }
        strip
            .save(dir.join(format!("{}.png", starter.key)))
            .unwrap();
    }
}
