use super::*;

#[test]
fn every_line_of_a_full_header_is_read() {
    let header = read_header(
        "-- name: Bevel hard edges\n\
         -- about: Bevels every hard edge of the selected shapes.\n\
         -- api: 1\n\
         -- count: segments\n\
         -- asks: read and write the project folder\n\
         \n\
         fetch \"aventail/scene\":[Scene, Shape];\n\
         -- name: not this, it's past the header\n",
    );
    assert_eq!(header.name.as_deref(), Some("Bevel hard edges"));
    assert_eq!(
        header.about.as_deref(),
        Some("Bevels every hard edge of the selected shapes.")
    );
    assert_eq!(header.api, Some(1));
    assert_eq!(header.count.as_deref(), Some("segments"));
    assert_eq!(header.asks, vec![Ask::WriteProjectFolder]);
    assert!(header.notes.is_empty(), "{:?}", header.notes);
    assert_eq!(header.shown_name("bevel.lsc"), "Bevel hard edges");
}

#[test]
fn keys_in_any_case_ordinary_comments_and_a_byte_order_mark() {
    let header = read_header(
        "\u{feff}--!strict\n-- A comment that isn't a header line.\n--NAME:Tidy\n-- Asks: Read the project folder\n-- asks: read the project folder\nprint(1)",
    );
    assert_eq!(header.name.as_deref(), Some("Tidy"));
    assert_eq!(header.asks, vec![Ask::ReadProjectFolder]);
    assert!(header.notes.is_empty(), "{:?}", header.notes);
}

#[test]
fn empty_and_headerless_scripts_have_empty_headers() {
    assert_eq!(read_header(""), Header::default());
    assert_eq!(
        read_header("print(\"hi\")\n-- name: late"),
        Header::default()
    );
    assert_eq!(
        read_header("--[[ name: inside a block ]]"),
        Header::default()
    );
    assert_eq!(Header::default().shown_name("tidy.lua"), "tidy");
    assert_eq!(Header::default().shown_name("noending"), "noending");
}

#[test]
fn lines_that_cant_be_read_are_noted_with_their_line() {
    let header = read_header(
        "-- name: One\n-- name: Two\n-- api: one\n-- api: 0\n-- count: 3 segments\n-- asks: the internet\n-- about:\n",
    );
    assert_eq!(header.name.as_deref(), Some("One"));
    assert_eq!(header.api, None);
    assert_eq!(header.count, None);
    assert!(header.asks.is_empty());
    assert_eq!(header.notes.len(), 6, "{:?}", header.notes);
    assert!(header.notes[0].starts_with("Line 2:"));
    assert!(header.notes[1].contains("whole number"));
    assert!(header.notes[4].contains("read the project folder"));
    assert!(header.notes[5].contains("empty"));
}

#[test]
fn long_values_are_cut_and_only_the_top_is_read() {
    let long = "x".repeat(MAX_NAME_CHARS + 50);
    let header = read_header(&format!("-- name: {long}"));
    assert_eq!(
        header.name.as_ref().map(|n| n.chars().count()),
        Some(MAX_NAME_CHARS + 1)
    );
    // A header past the lines looked at isn't read.
    let mut text = "--\n".repeat(MAX_HEADER_LINES);
    text.push_str("-- name: Too far down\n");
    assert_eq!(read_header(&text).name, None);
}

#[test]
fn fuzzed_headers_never_panic() {
    // Fixed seed: the same texts every run.
    let mut seed: u64 = 0x5eed_1234;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let pieces = [
        "-- ",
        "--",
        "name",
        "about",
        "api",
        "asks",
        "count",
        ":",
        " ",
        "\n",
        "\r\n",
        "é",
        "\u{feff}",
        "1",
        "99999999999",
        "-1",
        "read the project folder",
        "--[[",
        "]]",
        "\0",
        "🙂",
        "x",
    ];
    for _ in 0..3000 {
        let mut text = String::new();
        for _ in 0..(next() % 40) {
            text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
        }
        let header = read_header(&text);
        if let Some(name) = &header.name {
            assert!(name.chars().count() <= MAX_NAME_CHARS + 1);
        }
        let _ = header.shown_name(&text);
    }
}
