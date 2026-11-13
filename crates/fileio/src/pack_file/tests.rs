//! Tests for packs: a round trip of graphs and own boxes sharing boxes
//! inside, tags kept, refusals in plain words, and a fuzz run.

use super::*;
use crate::project::v98_tests::{LUMPY, PEBBLES, rich_document_v98 as sample_document};
use matgraph::GraphEdit;

/// A pack of the sample's Pebbles graph (with its two own boxes), a
/// starter graph, and the own boxes Pebbles and Lumpy on their own.
fn sample_pack() -> Pack {
    let doc = sample_document();
    let graphs = doc.scene().graphs();
    let (_, graph) = graphs.iter().find(|(_, g)| g.name() == "Pebbles").unwrap();
    let mut graph = (**graph).clone();
    graph
        .apply(
            graphs.registry(),
            &GraphEdit::SetTags(vec!["stone".into(), "round".into()]),
        )
        .unwrap();
    let pebbles = GraphFile::of(&graph, |id| graphs.own(id).cloned()).unwrap();
    let starter = crate::starter_graphs::STARTERS[0].load().unwrap();
    let boxes = [PEBBLES, LUMPY]
        .iter()
        .map(|id| {
            let main = graphs.own(id).unwrap().clone();
            BoxFile::of(&main, |i| graphs.own(i).cloned()).unwrap()
        })
        .collect();
    Pack {
        graphs: vec![pebbles, starter],
        boxes,
    }
}

#[test]
fn a_pack_reads_back_exactly() {
    let pack = sample_pack();
    assert_eq!(pack.len(), 4);
    assert!(!pack.is_empty());
    let text = write_pack(&pack);
    assert!(text.contains("\"format\": 1"));
    let back = read_pack(text.as_bytes()).unwrap();
    assert_eq!(back, pack);
    assert_eq!(back.graphs[0].graph.tags(), ["round", "stone"]);
    // Each entry carries the own boxes it needs, so each reads alone.
    assert_eq!(back.graphs[0].own.len(), 2);
    assert_eq!(back.boxes[0].boxes.len(), 2);
    assert_eq!(back.boxes[1].boxes.len(), 1);
}

#[test]
fn an_empty_pack_reads_back() {
    let text = write_pack(&Pack::default());
    let back = read_pack(text.as_bytes()).unwrap();
    assert!(back.is_empty());
    // Lists left out are empty.
    assert!(read_pack(b"{\"format\": 1}").unwrap().is_empty());
}

#[test]
fn newer_large_damaged_and_crowded_packs_say_why() {
    let text = write_pack(&sample_pack());
    let newer = text.replacen("\"format\": 1", "\"format\": 2", 1);
    assert!(read_pack(newer.as_bytes()).unwrap_err().contains("newer"));
    assert!(read_pack(b"not json").is_err());
    assert!(read_pack(b"[]").is_err());
    let huge = vec![b' '; (MAX_PACK_FILE + 1) as usize];
    assert!(read_pack(&huge).unwrap_err().contains("too large"));
    // A graph placing an own box its entry doesn't hold: the whole pack
    // is refused, naming the entry.
    let missing = text.replacen(
        &format!("\"kind\": \"{PEBBLES}\""),
        "\"kind\": \"own:dd\"",
        1,
    );
    let why = read_pack(missing.as_bytes()).unwrap_err();
    assert!(why.contains("graph number 1"), "{why}");
    // A box entry naming a box it doesn't hold.
    let wrong_main = text.replacen(
        &format!("\"main\": \"{LUMPY}\""),
        "\"main\": \"own:00000000000000ff\"",
        1,
    );
    let why = read_pack(wrong_main.as_bytes()).unwrap_err();
    assert!(why.contains("own box number 2"), "{why}");
    // More entries than a pack holds.
    let one = Pack {
        graphs: Vec::new(),
        boxes: vec![sample_pack().boxes[1].clone()],
    };
    let crowded = Pack {
        graphs: Vec::new(),
        boxes: vec![one.boxes[0].clone(); MAX_PACK_ITEMS + 1],
    };
    let why = read_pack(write_pack(&crowded).as_bytes()).unwrap_err();
    assert!(why.contains(&MAX_PACK_ITEMS.to_string()), "{why}");
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
fn hostile_packs_never_crash_the_reader() {
    let text = write_pack(&sample_pack()).into_bytes();
    let mut rng = Lcg(29);
    let mut read = 0;
    for _ in 0..2000 {
        let mut bytes = text.clone();
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
        if let Ok(pack) = read_pack(&bytes) {
            read += 1;
            assert!(pack.len() <= MAX_PACK_ITEMS);
            // What is read is whole: every own box a graph places is there.
            for file in &pack.graphs {
                assert!(matgraph::tags::are_tidy(file.graph.tags()));
                for (_, b) in file.graph.boxes() {
                    if matgraph::own::is_own(&b.kind) {
                        assert!(file.own.iter().any(|o| o.id == b.kind));
                    }
                }
            }
            for file in &pack.boxes {
                assert!(file.boxes.iter().any(|o| o.id == file.main.id));
            }
        }
    }
    assert!(read > 0);
}
