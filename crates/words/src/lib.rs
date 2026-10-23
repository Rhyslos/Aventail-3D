//! The app's words, through one place (decisions 0132 and 0134): English
//! is the app's one language package for now, and other languages can be
//! added later as packages without touching the code that shows text.
//!
//! Every crate that makes text a person reads uses this one: the interface
//! (`ui`), the undo labels, messages and reasons made in `scene`, `mesh`
//! and `fileio`, and the texts of `app`, `tools` and `uv`. It has no
//! dependencies, so it sits below them all (layer 0).
//!
//! The English text stays where it's used, so the code reads plainly:
//!
//! - `tr("Delete objects")` for plain text;
//! - `trf!("{} polygons selected", count)` for text with values in it, in
//!   place of `format!` (`{}` takes the next value, `{:.1}` a number with
//!   one decimal, `{{` and `}}` a brace);
//! - `pub const LABEL: &str = text!("Delete objects");` for text kept in a
//!   constant, shown with `tr(LABEL)`.
//!
//! The English text is the key: a language package lists English texts
//! with their translations, and `tr` gives the translation (or the English,
//! when the package has none). The English package (`lang/en.txt`) lists
//! every text, so translators know what to translate; a test in the `ui`
//! crate keeps it complete.
//!
//! On a Mac, every text says "Cmd" where it says "Ctrl" (decision 0133):
//! the app's keys use Cmd there, and the texts are written once, with
//! Ctrl.

use std::collections::HashMap;
use std::fmt::{Display, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

/// Marks a constant's text as interface text, part of the English
/// package. It stays the plain English: show it with [`tr`].
#[macro_export]
macro_rules! text {
    ($english:literal) => {
        $english
    };
}

/// `format!` through the language package: the template is translated,
/// then the values are put in. Takes `{}` and `{:.N}`, not named values.
#[macro_export]
macro_rules! trf {
    ($template:literal $(, $value:expr)* $(,)?) => {
        $crate::fill(
            $crate::tr($template),
            &[$(&$value as &dyn ::std::fmt::Display),*],
        )
    };
}

/// A language package: English texts and their translations.
#[derive(Debug, Default)]
pub struct Package {
    words: HashMap<String, &'static str>,
}

impl Package {
    /// Reads a package: one text per line, `English = translation`, with
    /// `\n` for a line break, `\=` for an equals sign and `\\` for a
    /// backslash. Blank lines and lines starting with `#` are skipped, as
    /// are lines without a translation.
    pub fn read(text: &str) -> Self {
        let mut words = HashMap::new();
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((english, translated)) = split_line(line) else {
                continue;
            };
            if translated.is_empty() {
                continue;
            }
            // Packages are read once and kept for the app's life.
            let kept: &'static str = Box::leak(translated.into_boxed_str());
            words.insert(english, kept);
        }
        Self { words }
    }

    /// How many texts it translates.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Whether it translates nothing (English).
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

/// A package line's English and translation, unescaped.
fn split_line(line: &str) -> Option<(String, String)> {
    let mut english = String::new();
    let mut chars = line.chars();
    let mut found = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => english.push('\n'),
                Some(other) => english.push(other),
                None => break,
            },
            ' ' if chars.as_str().starts_with("= ") => {
                chars.nth(1);
                found = true;
                break;
            }
            other => english.push(other),
        }
    }
    if !found {
        return None;
    }
    Some((english, unescape(chars.as_str())))
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A text as a package line's English part: line breaks as `\n`, and
/// `\`, `=` escaped.
pub fn escape(english: &str) -> String {
    let mut out = String::new();
    for c in english.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\\' => out.push_str("\\\\"),
            '=' => out.push_str("\\="),
            other => out.push(other),
        }
    }
    out
}

/// The package in use; English (empty) until another is chosen.
fn current() -> &'static RwLock<Package> {
    static CURRENT: OnceLock<RwLock<Package>> = OnceLock::new();
    CURRENT.get_or_init(|| RwLock::new(Package::default()))
}

/// Uses `package` for every text from now on (English: an empty one).
pub fn use_package(package: Package) {
    if let Ok(mut current) = current().write() {
        *current = package;
    }
}

/// Whether texts name Cmd for Ctrl (on a Mac).
static MAC_KEYS: AtomicBool = AtomicBool::new(false);

/// Says whether this computer is a Mac, where texts name Cmd for Ctrl.
/// The workbench sets it from egui's idea of the system.
pub fn use_mac_keys(on: bool) {
    MAC_KEYS.store(on, Ordering::Relaxed);
}

/// `english`, in the language in use.
pub fn tr(english: &'static str) -> &'static str {
    let text = current()
        .read()
        .ok()
        .and_then(|p| p.words.get(english).copied())
        .unwrap_or(english);
    if MAC_KEYS.load(Ordering::Relaxed) {
        with_mac_keys(text)
    } else {
        text
    }
}

/// `text` with "Ctrl" said as "Cmd". Each text is changed once and kept,
/// so there are at most as many as the interface has texts.
fn with_mac_keys(text: &'static str) -> &'static str {
    if !text.contains("Ctrl") {
        return text;
    }
    static CHANGED: OnceLock<RwLock<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let changed = CHANGED.get_or_init(|| RwLock::new(HashMap::new()));
    if let Some(done) = changed.read().ok().and_then(|c| c.get(text).copied()) {
        return done;
    }
    let done: &'static str = Box::leak(text.replace("Ctrl", "Cmd").into_boxed_str());
    if let Ok(mut changed) = changed.write() {
        changed.insert(text, done);
    }
    done
}

/// `template` with `values` put in, in order: `{}` takes the next value's
/// text, `{:.N}` a number with N decimals, `{{` and `}}` a brace. A value
/// missing is left out; spare values are ignored.
pub fn fill(template: &str, values: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = values.iter();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut spec = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    spec.push(c);
                }
                let Some(value) = next.next() else {
                    continue;
                };
                let decimals = spec
                    .strip_prefix(":.")
                    .and_then(|n| n.parse::<usize>().ok());
                let _ = match decimals {
                    Some(n) => write!(out, "{value:.n$}"),
                    None => write!(out, "{value}"),
                };
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_text_itself() {
        assert_eq!(tr("Delete objects"), "Delete objects");
        assert_eq!(trf!("{} polygons selected", 3), "3 polygons selected");
        assert_eq!(text!("Smooth by angle"), "Smooth by angle");
    }

    #[test]
    fn values_go_in_order_with_decimals_and_braces() {
        assert_eq!(
            fill(
                "Stretched up to {:.1}× ({} pieces) {{kept}}",
                &[&1.25f32, &4]
            ),
            "Stretched up to 1.2× (4 pieces) {kept}"
        );
        // A missing value leaves its place out; spare ones are ignored.
        assert_eq!(fill("{} and {}", &[&1]), "1 and ");
        assert_eq!(fill("just {}", &[&1, &2]), "just 1");
    }

    #[test]
    fn a_package_reads_its_lines_and_skips_the_rest() {
        let package = Package::read(
            "# Norwegian\n\
             Delete objects = Slett objekter\n\
             Two\\nlines = To\\nlinjer\n\
             a \\= b = a \\= b\n\
             Not translated = \n\
             no equals sign here\n",
        );
        assert_eq!(package.len(), 3);
        assert_eq!(package.words["Delete objects"], "Slett objekter");
        assert_eq!(package.words["Two\nlines"], "To\nlinjer");
        assert_eq!(package.words["a = b"], "a = b");
        assert_eq!(escape("a = b\nc\\d"), "a \\= b\\nc\\\\d");
    }

    #[test]
    fn a_mac_says_cmd_for_ctrl() {
        // The change itself; the switch is global, so other tests would
        // see it if this test turned it on.
        assert_eq!(
            with_mac_keys("Ctrl + Z takes it back; Ctrl + wheel for more."),
            "Cmd + Z takes it back; Cmd + wheel for more."
        );
        assert_eq!(with_mac_keys("Shift + H"), "Shift + H");
        // Kept, not made again.
        let a = with_mac_keys("Ctrl + A");
        assert!(std::ptr::eq(a, with_mac_keys("Ctrl + A")));
    }

    #[test]
    fn junk_packages_never_panic() {
        // Random bytes, as text, read without panicking.
        let mut seed = 7u64;
        for _ in 0..200 {
            let text: String = (0..80)
                .map(|_| {
                    seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    char::from_u32((seed >> 33) as u32 % 0x250).unwrap_or('?')
                })
                .collect();
            let _ = Package::read(&text);
        }
    }
}
