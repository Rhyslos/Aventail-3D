//! Tests for graph files: a round trip with own boxes inside own boxes,
//! fitting a file's own boxes to a project's, newer and damaged files,
//! and a fuzz run.

use super::*;
use crate::project::v98_tests::{LUMPY, PEBBLES, rich_document_v98 as sample_document};
use matgraph::{Evaluator, Registry, Size};

/// The sample's "Pebbles" graph with its own boxes.
fn pebbles_file() -> GraphFile {
    let doc = sample_document();
    let graphs = doc.scene().graphs();
    let (_, graph) = graphs.iter().find(|(_, g)| g.name() == "Pebbles").unwrap();
    GraphFile::of(graph, |id| graphs.own(id).cloned()).unwrap()
}

fn material(file: &GraphFile) -> Arc<matgraph::MaterialMaps> {
    let registry = matgraph::own::registry_with(&file.own);
    Evaluator::with_threads(1)
        .material(&registry, &file.graph, Size::new(32).unwrap())
        .unwrap()
}

#[test]
fn a_graph_file_holds_the_own_boxes_and_reads_back() {
    let file = pebbles_file();
    let ids: Vec<&str> = file.own.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, [LUMPY, PEBBLES], "the box inside comes first");
    let text = write_graph_file(&file);
    assert!(text.contains("\"format\": 1"));
    let back = read_graph_file(text.as_bytes()).unwrap();
    assert_eq!(back, file);
    assert_eq!(*material(&back), *material(&file));
}

#[test]
fn a_graph_without_own_boxes_reads_back() {
    let mut graph = Graph::new("Plain");
    graph
        .apply(
            Registry::standard(),
            &matgraph::GraphEdit::AddBox {
                kind: "noise".into(),
                at: glam::Vec2::ZERO,
            },
        )
        .unwrap();
    let file = GraphFile::of(&graph, |_| None).unwrap();
    assert!(file.own.is_empty());
    let back = read_graph_file(write_graph_file(&file).as_bytes()).unwrap();
    assert_eq!(back.graph, graph);
}

#[test]
fn a_graph_with_an_own_box_missing_can_t_be_written() {
    let file = pebbles_file();
    assert!(GraphFile::of(&file.graph, |_| None).is_none());
}

#[test]
fn own_boxes_a_file_doesn_t_use_are_left_out() {
    let file = pebbles_file();
    let mut stored: serde_json::Value = serde_json::from_str(&write_graph_file(&file)).unwrap();
    // Another own box, unused: a copy of Lumpy under another id.
    let mut spare = stored["own"][0].clone();
    spare["id"] = "own:00000000000000c3".into();
    stored["own"].as_array_mut().unwrap().push(spare);
    let back = read_graph_file(stored.to_string().as_bytes()).unwrap();
    assert_eq!(back.own.len(), 2);
}

#[test]
fn fitting_keeps_ids_the_project_holds_the_same_and_renames_changed_ones() {
    let file = pebbles_file();
    // A project without them, or with them exactly: kept as they are.
    assert_eq!(file.fitted_to(|_| None).unwrap(), file);
    let same = file.fitted_to(|id| file.own.iter().find(|o| o.id == id).cloned());
    assert_eq!(same.unwrap(), file);
    // A project whose Lumpy was changed: Lumpy is renamed, and Pebbles
    // (holding it) too; the graph places the new ids.
    let mut changed = (*file.own[0]).clone();
    changed.name = "Lumpy, changed".into();
    let changed = Arc::new(changed);
    let fitted = file
        .fitted_to(|id| (id == LUMPY).then(|| changed.clone()))
        .unwrap();
    let ids: BTreeSet<&str> = fitted.own.iter().map(|o| o.id.as_str()).collect();
    assert!(!ids.contains(LUMPY) && !ids.contains(PEBBLES), "{ids:?}");
    assert!(fitted.graph.boxes().all(|(_, b)| b.kind != PEBBLES));
    assert_eq!(fitted.own[0].name, "Lumpy");
    // The same picture.
    assert_eq!(*material(&fitted), *material(&file));
    // A project with another box under Pebbles' id: only Pebbles moves.
    let other = changed.clone();
    let fitted = file
        .fitted_to(|id| (id == PEBBLES).then(|| other.clone()))
        .unwrap();
    assert_eq!(fitted.own[0].id, LUMPY);
    assert_ne!(fitted.own[1].id, PEBBLES);
}

#[test]
fn newer_large_and_damaged_files_say_why() {
    let text = write_graph_file(&pebbles_file());
    let newer = text.replacen("\"format\": 1", "\"format\": 7", 1);
    assert!(
        read_graph_file(newer.as_bytes())
            .unwrap_err()
            .contains("newer")
    );
    assert!(read_graph_file(b"not json").is_err());
    assert!(read_graph_file(b"{\"format\": 1}").is_err());
    // A graph placing an own box the file doesn't hold.
    let missing = text.replacen(
        &format!("\"kind\": \"{PEBBLES}\""),
        "\"kind\": \"own:dd\"",
        1,
    );
    assert!(read_graph_file(missing.as_bytes()).is_err());
    let huge = vec![b' '; (MAX_GRAPH_FILE + 1) as usize];
    assert!(read_graph_file(&huge).unwrap_err().contains("too large"));
}

/// A simple repeatable random number source.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 20
    }
}

#[test]
fn hostile_graph_files_never_crash_the_reader() {
    // The sample, and a starter graph (frames, notes, gradients).
    let texts = [
        write_graph_file(&pebbles_file()).into_bytes(),
        crate::starter_graphs::STARTERS[1].file.as_bytes().to_vec(),
    ];
    let mut rng = Lcg(11);
    let mut read = 0;
    for i in 0..3000 {
        let mut bytes = texts[i % 2].clone();
        match rng.next() % 3 {
            // Flipped bits.
            0 => {
                for _ in 0..(rng.next() % 4 + 1) {
                    let at = (rng.next() as usize) % bytes.len();
                    bytes[at] ^= 1 << (rng.next() % 8);
                }
            }
            // Cut short.
            1 => bytes.truncate((rng.next() as usize) % bytes.len()),
            // Digits changed (ids, kinds, places, settings).
            _ => {
                for _ in 0..(rng.next() % 6 + 1) {
                    let at = (rng.next() as usize) % bytes.len();
                    if bytes[at].is_ascii_digit() {
                        bytes[at] = b'0' + (rng.next() % 10) as u8;
                    }
                }
            }
        }
        if let Ok(file) = read_graph_file(&bytes) {
            read += 1;
            // What is read is whole: every own box it places is there.
            for (_, b) in file.graph.boxes() {
                if is_own(&b.kind) {
                    assert!(file.own.iter().any(|o| o.id == b.kind));
                }
            }
            let _ = file.fitted_to(|_| None);
        }
    }
    assert!(read > 0);
}

#[test]
fn a_graph_naming_a_palette_is_written_as_version_2_and_reads_back() {
    let mut graph = Graph::new("Painted");
    for edit in [
        matgraph::GraphEdit::AddBox {
            kind: "palette_map".into(),
            at: glam::Vec2::ZERO,
        },
        matgraph::GraphEdit::SetSetting {
            node: matgraph::BoxId(1),
            key: "palette".into(),
            value: matgraph::SettingValue::Palette("Foliage".into()),
        },
    ] {
        graph.apply(Registry::standard(), &edit).unwrap();
    }
    let file = GraphFile::of(&graph, |_| None).unwrap();
    let text = write_graph_file(&file);
    assert!(text.contains("\"format\": 2"), "{text}");
    let back = read_graph_file(text.as_bytes()).unwrap();
    assert_eq!(back.graph, graph);
    assert_eq!(
        back.graph
            .get(matgraph::BoxId(1))
            .unwrap()
            .settings
            .get("palette"),
        Some(matgraph::SettingValue::Palette("Foliage".into()))
    );
    // An older app (format 1 only) refuses it as newer, not misread it;
    // a format past this app's still is.
    let newer = text.replacen("\"format\": 2", "\"format\": 7", 1);
    assert!(
        read_graph_file(newer.as_bytes())
            .unwrap_err()
            .contains("newer")
    );
}

#[test]
fn a_graph_with_a_painting_is_written_as_version_3_small_and_reads_back() {
    let mut rgba = vec![0u8; 1024 * 1024 * 4];
    // A stroke of moss across the middle.
    for y in 500..530 {
        for x in 0..1024 {
            let at = (y * 1024 + x) * 4;
            rgba[at..at + 4].copy_from_slice(&[60, 120, 40, 255]);
        }
    }
    let painting = matgraph::Painting::from_rgba(1024, &rgba).unwrap();
    let mut graph = Graph::new("Moss here");
    for edit in [
        matgraph::GraphEdit::AddBox {
            kind: "painted".into(),
            at: glam::Vec2::ZERO,
        },
        matgraph::GraphEdit::SetPainting {
            node: matgraph::BoxId(1),
            painting: Some(Arc::new(painting.clone())),
        },
    ] {
        graph.apply(Registry::standard(), &edit).unwrap();
    }
    let file = GraphFile::of(&graph, |_| None).unwrap();
    let text = write_graph_file(&file);
    assert!(text.contains("\"format\": 3"));
    // Packed: far smaller than the 4 MB of pixels.
    assert!(text.len() < 200_000, "{} bytes", text.len());
    let back = read_graph_file(text.as_bytes()).unwrap();
    assert_eq!(back.graph, graph);
    assert_eq!(
        back.graph
            .get(matgraph::BoxId(1))
            .unwrap()
            .painting
            .as_deref(),
        Some(&painting)
    );
    // A format past this app's is refused as newer.
    let newer = text.replacen("\"format\": 3", "\"format\": 7", 1);
    assert!(
        read_graph_file(newer.as_bytes())
            .unwrap_err()
            .contains("newer")
    );
}

/// A photo of random speckles, `n` pixels square: the hardest kind of
/// picture to pack.
fn speckled_photo(n: u32) -> matgraph::Photo {
    let mut state = 99u32;
    let image = image::RgbImage::from_fn(n, n, |_, _| {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let v = (state >> 16) as u8;
        image::Rgb([v, v.wrapping_mul(3), v / 2])
    });
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    crate::image_file::read_photo(&png, "speckles.png").unwrap()
}

#[test]
fn a_graph_with_a_photo_is_written_as_version_4_and_reads_back() {
    // The hardest photo to pack, at the largest size an Image box keeps,
    // still fits a graph file with room to spare.
    let photo = speckled_photo(matgraph::MAX_PHOTO_SIDE);
    assert!(photo.packed().len() <= 4 << 20, "{}", photo.packed().len());
    let mut graph = Graph::new("From a photo");
    for edit in [
        matgraph::GraphEdit::AddBox {
            kind: "image".into(),
            at: glam::Vec2::ZERO,
        },
        matgraph::GraphEdit::SetPhoto {
            node: matgraph::BoxId(1),
            photo: Some(Arc::new(photo.clone())),
        },
    ] {
        graph.apply(Registry::standard(), &edit).unwrap();
    }
    let file = GraphFile::of(&graph, |_| None).unwrap();
    let text = write_graph_file(&file);
    assert!(text.contains("\"format\": 4"));
    assert!((text.len() as u64) < MAX_GRAPH_FILE, "{} bytes", text.len());
    let back = read_graph_file(text.as_bytes()).unwrap();
    assert_eq!(back.graph, graph);
    let kept = back.graph.get(matgraph::BoxId(1)).unwrap().photo.clone();
    assert_eq!(kept.as_deref(), Some(&photo));
    // A format past this app's is refused as newer.
    let newer = text.replacen("\"format\": 4", "\"format\": 7", 1);
    assert!(
        read_graph_file(newer.as_bytes())
            .unwrap_err()
            .contains("newer")
    );
}

/// A stamp of random speckles with random see-through, `n` pixels square:
/// the hardest kind of stamp to pack.
fn speckled_stamp(n: u32, seed: u32) -> matgraph::Stamp {
    let mut state = seed;
    let image = image::RgbaImage::from_fn(n, n, |_, _| {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let v = (state >> 16) as u8;
        image::Rgba([v, v.wrapping_mul(3), v / 2, v.wrapping_mul(7) | 1])
    });
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    crate::image_file::read_stamp(&png, "speckles.png").unwrap()
}

#[test]
fn a_graph_with_stamps_is_written_as_version_5_and_reads_back() {
    // As many of the hardest stamps to pack, at the largest size a Stamps
    // box keeps, as one box takes: the file still fits with room to spare.
    let side = matgraph::MAX_STAMP_SIDE;
    let mut stamps = matgraph::Stamps::one(speckled_stamp(side, 1)).unwrap();
    let mut seed = 2;
    while let Ok(more) = stamps.with_added(speckled_stamp(side, seed)) {
        stamps = more;
        seed += 1;
    }
    assert!(stamps.len() >= 4, "{}", stamps.len());
    let stamps = stamps.with_weight(1, 3);
    let mut graph = Graph::new("Rivets");
    for edit in [
        matgraph::GraphEdit::AddBox {
            kind: "stamps".into(),
            at: glam::Vec2::ZERO,
        },
        matgraph::GraphEdit::SetStamps {
            node: matgraph::BoxId(1),
            stamps: Some(Arc::new(stamps.clone())),
        },
    ] {
        graph.apply(Registry::standard(), &edit).unwrap();
    }
    let file = GraphFile::of(&graph, |_| None).unwrap();
    let text = write_graph_file(&file);
    assert!(text.contains("\"format\": 5"));
    assert!((text.len() as u64) < MAX_GRAPH_FILE, "{} bytes", text.len());
    let back = read_graph_file(text.as_bytes()).unwrap();
    assert_eq!(back.graph, graph);
    let kept = back.graph.get(matgraph::BoxId(1)).unwrap().stamps.clone();
    assert_eq!(kept.as_deref(), Some(&stamps));
    assert_eq!(kept.unwrap().entries()[1].weight, 3);
    let newer = text.replacen("\"format\": 5", "\"format\": 7", 1);
    assert!(
        read_graph_file(newer.as_bytes())
            .unwrap_err()
            .contains("newer")
    );
}

#[test]
fn a_graph_or_own_box_with_tags_is_written_as_version_6_and_reads_back() {
    let plain = pebbles_file();
    assert!(write_graph_file(&plain).contains("\"format\": 1"));
    // Tags on the graph.
    let mut tagged = plain.clone();
    let registry = matgraph::own::registry_with(&tagged.own);
    tagged
        .graph
        .apply(
            &registry,
            &matgraph::GraphEdit::SetTags(vec!["Stone".into()]),
        )
        .unwrap();
    let text = write_graph_file(&tagged);
    assert!(text.contains("\"format\": 6"));
    let back = read_graph_file(text.as_bytes()).unwrap();
    assert_eq!(back.graph.tags(), ["stone"]);
    assert_eq!(back, tagged);
    // Tags on an own box inside only.
    let mut boxed = plain.clone();
    let mut own = (*boxed.own[0]).clone();
    own.tags = vec!["lumpy".into()];
    boxed.own[0] = Arc::new(own);
    let text = write_graph_file(&boxed);
    assert!(text.contains("\"format\": 6"));
    assert_eq!(
        read_graph_file(text.as_bytes()).unwrap().own[0].tags,
        ["lumpy"]
    );
}

#[test]
fn every_starter_s_tags_are_tidy() {
    for starter in &crate::starter_graphs::STARTERS {
        let tags: Vec<String> = starter.tags.iter().map(|t| t.to_string()).collect();
        assert!(!tags.is_empty(), "{}", starter.key);
        let tidy = matgraph::tags::tidy_tags(starter.tags.iter().copied());
        assert_eq!(tidy.len(), tags.len(), "{}", starter.key);
        assert!(tags.iter().all(|t| tidy.contains(t)), "{}", starter.key);
    }
}
