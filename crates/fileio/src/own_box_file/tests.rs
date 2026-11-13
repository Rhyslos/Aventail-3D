//! Tests for box files: a round trip with the own boxes inside, newer
//! and damaged files, and a fuzz run.

use super::*;
use crate::project::v98_tests::{LUMPY, PEBBLES, rich_document_v98 as sample_document};

fn pebbles_file() -> BoxFile {
    let doc = sample_document();
    let graphs = doc.scene().graphs();
    let main = graphs.own(PEBBLES).unwrap().clone();
    BoxFile::of(&main, |id| graphs.own(id).cloned()).unwrap()
}

#[test]
fn a_box_file_holds_the_boxes_inside_and_reads_back() {
    let file = pebbles_file();
    assert_eq!(file.boxes.len(), 2);
    assert_eq!(file.boxes[0].id, LUMPY, "the box inside comes first");
    let text = write_box_file(&file);
    assert!(text.contains("\"format\": 1"));
    let back = read_box_file(text.as_bytes()).unwrap();
    assert_eq!(back, file);
}

#[test]
fn a_box_with_one_inside_missing_can_t_be_written() {
    let doc = sample_document();
    let main = doc.scene().graphs().own(PEBBLES).unwrap().clone();
    assert!(BoxFile::of(&main, |_| None).is_none());
}

#[test]
fn newer_large_and_damaged_files_say_why() {
    let text = write_box_file(&pebbles_file());
    let newer = text.replacen("\"format\": 1", "\"format\": 7", 1);
    assert!(
        read_box_file(newer.as_bytes())
            .unwrap_err()
            .contains("newer")
    );
    assert!(read_box_file(b"not json").is_err());
    assert!(read_box_file(b"{\"format\": 1}").is_err());
    let wrong_main = text.replacen(PEBBLES, "own:0000000000000fff", 1);
    assert!(read_box_file(wrong_main.as_bytes()).is_err());
    let huge = vec![b' '; (MAX_BOX_FILE + 1) as usize];
    assert!(read_box_file(&huge).unwrap_err().contains("too large"));
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
fn hostile_box_files_never_crash_the_reader() {
    let text = write_box_file(&pebbles_file()).into_bytes();
    let mut rng = Lcg(5);
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
            // Digits changed (ids, kinds, places).
            _ => {
                for _ in 0..(rng.next() % 6 + 1) {
                    let at = (rng.next() as usize) % bytes.len();
                    if bytes[at].is_ascii_digit() {
                        bytes[at] = b'0' + (rng.next() % 10) as u8;
                    }
                }
            }
        }
        if let Ok(file) = read_box_file(&bytes) {
            read += 1;
            assert!(file.boxes.iter().any(|b| b.id == file.main.id));
        }
    }
    assert!(read > 0);
}

#[test]
fn a_box_with_tags_is_written_as_version_6_and_reads_back() {
    let mut file = pebbles_file();
    let mut main = (*file.main).clone();
    main.tags = vec!["pebbles".into(), "round".into()];
    let main = Arc::new(main);
    file.main = main.clone();
    *file.boxes.last_mut().unwrap() = main;
    let text = write_box_file(&file);
    assert!(text.contains("\"format\": 6"));
    let back = read_box_file(text.as_bytes()).unwrap();
    assert_eq!(back.main.tags, ["pebbles", "round"]);
    assert_eq!(back, file);
}
