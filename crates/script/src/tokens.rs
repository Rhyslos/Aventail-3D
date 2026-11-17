//! The pieces of a script's text, for colouring it in the Scripts panel's
//! editor (plan, milestone 3): a small reader of our own for LustScript
//! and Lua, never the compiler. It never fails and never runs anything:
//! text it doesn't know is a symbol, and an unfinished string or comment
//! runs to where it would have ended (the line's end, or the text's).

use crate::Language;
use std::ops::Range;

/// What a piece of a script is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// One of the language's own words (`loop`, `method`, `function`).
    Keyword,
    /// A name starting with a capital: a kind of thing or an object
    /// (`Scene`, `Shape`, `Primitive`).
    TypeName,
    /// Any other name.
    Name,
    /// A number.
    Number,
    /// Text in quotes, or Lua's long brackets.
    Text,
    /// A comment.
    Comment,
    /// Anything else that isn't space: brackets, operators, a stray
    /// character.
    Symbol,
}

/// A piece of a script: what it is and where, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub kind: TokenKind,
    /// Where it is in the text, in bytes; always on character boundaries
    /// and never empty.
    pub range: Range<usize>,
}

/// LustScript's own words (Lust's `RESERVED`, alpha 73).
const LUST_WORDS: &[&str] = &[
    "class",
    "object",
    "node",
    "assembly",
    "create",
    "has",
    "method",
    "hook",
    "return",
    "like",
    "new",
    "nothing",
    "schema",
    "route",
    "view",
    "style",
    "keep",
    "pass",
    "fetch",
    "get",
    "lua",
    "is",
    "then",
    "or",
    "either",
    "not",
    "true",
    "false",
    "loop",
    "stop",
    "skip",
    "in",
    "try",
    "error",
    "state",
    "save",
    "load",
    "saved",
    "end",
    "database",
    "int",
    "float",
    "string",
    "bool",
    "array",
    "native",
    "cMap",
    "map",
    "component",
    "check",
    "when",
    "lock",
];

/// Lua's own words, with Luau's `continue`.
const LUA_WORDS: &[&str] = &[
    "and", "break", "continue", "do", "else", "elseif", "end", "false", "for", "function", "goto",
    "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// The pieces of `text`, in order, written in `language`. Space between
/// them is left out.
pub fn tokens(text: &str, language: Language) -> Vec<Token> {
    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        at: 0,
        tokens: Vec::new(),
    };
    while reader.at < reader.bytes.len() {
        reader.next(language);
    }
    reader.tokens
}

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    tokens: Vec<Token>,
}

impl Reader<'_> {
    /// The byte `ahead` places on, or 0 past the end.
    fn peek(&self, ahead: usize) -> u8 {
        self.bytes.get(self.at + ahead).copied().unwrap_or(0)
    }

    /// Reads one piece (or skips one space) from `at`.
    fn next(&mut self, language: Language) {
        let start = self.at;
        let c = self.peek(0);
        if c.is_ascii_whitespace() {
            self.at += 1;
            return;
        }
        let kind = if c == b'-' && self.peek(1) == b'-' {
            self.comment(language);
            TokenKind::Comment
        } else if c == b'/' && self.peek(1) == b'/' && language == Language::LustScript {
            self.skip_line();
            TokenKind::Comment
        } else if c == b'"' || (c == b'\'' && language == Language::Lua) {
            self.quoted(c, language);
            TokenKind::Text
        } else if c == b'[' && language == Language::Lua && self.long_open().is_some() {
            self.long_bracket();
            TokenKind::Text
        } else if c.is_ascii_digit() || (c == b'.' && self.peek(1).is_ascii_digit()) {
            self.number();
            TokenKind::Number
        } else if c == b'_' || c.is_ascii_alphabetic() || c >= 0x80 && self.letter_here() {
            return self.word(language);
        } else {
            self.one_char();
            TokenKind::Symbol
        };
        self.push(kind, start);
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        if self.at > start {
            self.tokens.push(Token {
                kind,
                range: start..self.at,
            });
        }
    }

    /// Steps over one whole character.
    fn one_char(&mut self) {
        let width = self.text[self.at..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);
        self.at += width;
    }

    /// Whether the character here is a letter (beyond ASCII).
    fn letter_here(&self) -> bool {
        self.text[self.at..]
            .chars()
            .next()
            .is_some_and(char::is_alphabetic)
    }

    fn skip_line(&mut self) {
        while self.at < self.bytes.len() && self.bytes[self.at] != b'\n' {
            self.at += 1;
        }
    }

    /// A comment from its `--`: a block (`--[[ … ]]`, and Lua's
    /// `--[==[ … ]==]`) or the rest of the line.
    fn comment(&mut self, language: Language) {
        self.at += 2;
        if self.peek(0) == b'[' {
            match language {
                Language::Lua if self.long_open().is_some() => return self.long_bracket(),
                Language::LustScript if self.peek(1) == b'[' => {
                    self.at += 2;
                    return self.until(b"]]");
                }
                _ => {}
            }
        }
        self.skip_line();
    }

    /// Steps past the next `end`, or to the text's end if there's none.
    fn until(&mut self, end: &[u8]) {
        match find(&self.bytes[self.at..], end) {
            Some(found) => self.at += found + end.len(),
            None => self.at = self.bytes.len(),
        }
    }

    /// How many `=` a Lua long bracket opening here has (`[[` is 0,
    /// `[==[` is 2), if one opens here.
    fn long_open(&self) -> Option<usize> {
        let level = self.bytes[self.at + 1..]
            .iter()
            .take_while(|b| **b == b'=')
            .count();
        (self.peek(1 + level) == b'[').then_some(level)
    }

    /// A Lua long bracket, from its opening to its closing of the same
    /// level, or to the text's end.
    fn long_bracket(&mut self) {
        let level = self.long_open().unwrap_or(0);
        self.at += level + 2;
        let mut end = vec![b']'];
        end.extend(std::iter::repeat_n(b'=', level));
        end.push(b']');
        self.until(&end);
    }

    /// Text in quotes, ending at its closing quote or the line's end
    /// (neither language lets text run past a line). A backslash keeps
    /// the next character in the text.
    fn quoted(&mut self, quote: u8, language: Language) {
        self.at += 1;
        while self.at < self.bytes.len() {
            match self.bytes[self.at] {
                b'\n' => return,
                b'\\' => {
                    self.at += 1;
                    // Lua's `\` before a line's end carries on to the next.
                    if self.peek(0) != b'\n' || language == Language::Lua {
                        if self.at < self.bytes.len() {
                            self.one_char();
                        }
                        continue;
                    }
                    return;
                }
                c if c == quote => {
                    self.at += 1;
                    return;
                }
                _ => self.one_char(),
            }
        }
    }

    /// A number: digits, a point, an exponent (`1e-3`), a hex number
    /// (`0xff`), and `_` between digits as Luau allows.
    fn number(&mut self) {
        let hex = self.peek(0) == b'0' && matches!(self.peek(1), b'x' | b'X');
        if hex {
            self.at += 2;
        }
        while self.at < self.bytes.len() {
            let c = self.bytes[self.at];
            let exponent = !hex
                && matches!(c, b'e' | b'E')
                && (self.peek(1).is_ascii_digit()
                    || matches!(self.peek(1), b'+' | b'-') && self.peek(2).is_ascii_digit());
            if exponent {
                self.at += 2;
            } else if c.is_ascii_alphanumeric()
                || c == b'_'
                || (c == b'.' && self.peek(1).is_ascii_digit())
            {
                self.at += 1;
            } else {
                return;
            }
        }
    }

    /// A name or one of the language's words.
    fn word(&mut self, language: Language) {
        let start = self.at;
        while self.at < self.bytes.len() {
            let c = self.bytes[self.at];
            if c == b'_' || c.is_ascii_alphanumeric() {
                self.at += 1;
            } else if c >= 0x80
                && self.text[self.at..]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphanumeric)
            {
                self.one_char();
            } else {
                break;
            }
        }
        let word = &self.text[start..self.at];
        let words = match language {
            Language::LustScript => LUST_WORDS,
            Language::Lua => LUA_WORDS,
        };
        let kind = if words.contains(&word) {
            TokenKind::Keyword
        } else if word.chars().next().is_some_and(char::is_uppercase) {
            TokenKind::TypeName
        } else {
            TokenKind::Name
        };
        self.push(kind, start);
    }
}

/// Where `needle` first is in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tests;
