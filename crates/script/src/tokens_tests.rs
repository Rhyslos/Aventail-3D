//! The editor's colouring: every kind of piece, unfinished strings and
//! comments, and fuzzed text.

use super::*;
use TokenKind::*;

/// The pieces of `text` as (kind, text).
fn pieces(text: &str, language: Language) -> Vec<(TokenKind, &str)> {
    tokens(text, language)
        .into_iter()
        .map(|t| (t.kind, &text[t.range]))
        .collect()
}

#[test]
fn lustscript_pieces_of_every_kind() {
    let text = "fetch \"aventail/scene\":[Scene, Shape]; -- the calls\nfloat width = 0.25e-1; // more\nloop (Shape s in Scene:selected()) { s:bevel(s:hardEdges(), width, 2); }\n--[[ a\nblock ]] end";
    let got = pieces(text, Language::LustScript);
    assert_eq!(got[0], (Keyword, "fetch"));
    assert_eq!(got[1], (Text, "\"aventail/scene\""));
    assert_eq!(got[2], (Symbol, ":"));
    assert_eq!(got[3], (Symbol, "["));
    assert_eq!(got[4], (TypeName, "Scene"));
    assert!(got.contains(&(Comment, "-- the calls")));
    assert!(got.contains(&(Keyword, "float")));
    assert!(got.contains(&(Name, "width")));
    assert!(got.contains(&(Number, "0.25e-1")));
    assert!(got.contains(&(Comment, "// more")));
    assert!(got.contains(&(Keyword, "loop")));
    assert!(got.contains(&(Name, "hardEdges")));
    assert!(got.contains(&(Number, "2")));
    assert!(got.contains(&(Comment, "--[[ a\nblock ]]")));
    assert_eq!(got.last(), Some(&(Keyword, "end")));
}

#[test]
fn lua_pieces_of_every_kind() {
    let text = "local n = 0x1F + 3.5 -- count\nprint('it''s', \"a \\\"b\\\"\", [==[long\n]] still]==])\n--[[ block\n]] function f() return nil end";
    let got = pieces(text, Language::Lua);
    assert_eq!(got[0], (Keyword, "local"));
    assert_eq!(got[1], (Name, "n"));
    assert!(got.contains(&(Number, "0x1F")));
    assert!(got.contains(&(Number, "3.5")));
    assert!(got.contains(&(Comment, "-- count")));
    assert!(got.contains(&(Text, "'it'")));
    assert!(got.contains(&(Text, "'s'")));
    assert!(got.contains(&(Text, "\"a \\\"b\\\"\"")));
    assert!(got.contains(&(Text, "[==[long\n]] still]==]")));
    assert!(got.contains(&(Comment, "--[[ block\n]]")));
    assert!(got.contains(&(Keyword, "function")));
    assert!(got.contains(&(Keyword, "nil")));
    // Lua's words aren't LustScript's, and the other way round.
    assert_eq!(pieces("local", Language::LustScript), [(Name, "local")]);
    assert_eq!(pieces("loop", Language::Lua), [(Name, "loop")]);
    // `//` is Lua's whole-number division, not a comment.
    assert_eq!(
        pieces("a // b", Language::Lua),
        [(Name, "a"), (Symbol, "/"), (Symbol, "/"), (Name, "b")]
    );
    // Single quotes aren't text in LustScript.
    assert_eq!(pieces("'", Language::LustScript), [(Symbol, "'")]);
}

#[test]
fn unfinished_strings_and_comments_end_where_they_would() {
    // Text stops at the line's end.
    let got = pieces("x = \"open\ny = 1", Language::LustScript);
    assert_eq!(got[2], (Text, "\"open"));
    assert_eq!(got[3], (Name, "y"));
    let got = pieces("x = 'open\ny", Language::Lua);
    assert_eq!(got[2], (Text, "'open"));
    // A block comment or long string never closed runs to the end.
    assert_eq!(
        pieces("--[[ never\nclosed", Language::LustScript),
        [(Comment, "--[[ never\nclosed")]
    );
    assert_eq!(
        pieces("--[==[ never ]]", Language::Lua),
        [(Comment, "--[==[ never ]]")]
    );
    assert_eq!(pieces("[[ open", Language::Lua), [(Text, "[[ open")]);
    // A backslash at the very end.
    assert_eq!(pieces("\"a\\", Language::Lua), [(Text, "\"a\\")]);
    assert_eq!(pieces("\"a\\", Language::LustScript), [(Text, "\"a\\")]);
    // A `[` that opens nothing is a symbol.
    assert_eq!(pieces("[=x", Language::Lua)[0], (Symbol, "["));
    assert!(pieces("", Language::Lua).is_empty());
    assert!(pieces(" \n\t ", Language::Lua).is_empty());
}

#[test]
fn letters_beyond_ascii_and_stray_characters() {
    assert_eq!(
        pieces("café = «x»", Language::Lua),
        [
            (Name, "café"),
            (Symbol, "="),
            (Symbol, "«"),
            (Name, "x"),
            (Symbol, "»")
        ]
    );
    assert_eq!(pieces("Été", Language::LustScript), [(TypeName, "Été")]);
    assert_eq!(pieces("😀", Language::Lua), [(Symbol, "😀")]);
}

/// Every piece is in order, on character boundaries, never empty, and
/// everything that isn't space is in one.
fn assert_sound(text: &str, language: Language) {
    let found = tokens(text, language);
    let mut covered = vec![false; text.len()];
    let mut last = 0;
    for token in &found {
        assert!(token.range.start >= last, "{text:?}: {found:?}");
        assert!(token.range.start < token.range.end, "{text:?}");
        assert!(text.is_char_boundary(token.range.start), "{text:?}");
        assert!(text.is_char_boundary(token.range.end), "{text:?}");
        covered[token.range.clone()]
            .iter_mut()
            .for_each(|c| *c = true);
        last = token.range.end;
    }
    for (at, c) in text.char_indices() {
        assert!(c.is_ascii_whitespace() || covered[at], "{text:?} at {at}");
    }
}

#[test]
fn fuzzed_text_is_read_soundly() {
    // A small fixed random walk over pieces that matter, and any bytes.
    const PIECES: &[&str] = &[
        "--", "[[", "]]", "[=", "=[", "]=", "\"", "'", "\\", "\n", " ", "//", "0x", "1e", "e-",
        ".5", "_", "a", "Z", "é", "😀", "{", "}", "end", "loop", "\u{feff}", "\r", "\t", "-", "[",
        "]", "=",
    ];
    let mut seed: u64 = 0x5eed_c0de;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for round in 0..3_000 {
        let mut text = String::new();
        for _ in 0..(next() % 40) {
            if next() % 5 == 0 {
                text.push(char::from_u32((next() % 0x2_0000) as u32).unwrap_or('?'));
            } else {
                text.push_str(PIECES[(next() % PIECES.len() as u64) as usize]);
            }
        }
        let language = if round % 2 == 0 {
            Language::Lua
        } else {
            Language::LustScript
        };
        assert_sound(&text, language);
    }
}

#[test]
fn ten_thousand_lines_are_read_quickly() {
    let line = "    shape:bevel(shape:hardEdges(), 0.2, 2); -- round it \"off\"\n";
    let text = line.repeat(10_000);
    let start = std::time::Instant::now();
    let found = tokens(&text, Language::LustScript);
    let took = start.elapsed();
    assert!(found.len() > 100_000);
    // Generous for a debug build on a busy machine; about 20 ms there.
    assert!(took < std::time::Duration::from_secs(2), "{took:?}");
}
