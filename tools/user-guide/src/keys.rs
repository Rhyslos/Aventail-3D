//! Spotting keys in the manual's words, so the guide can draw them as
//! keyboard keys. The manual writes keys in bold or as code: **Ctrl + Z**,
//! **Esc**, `Alt + V`, "press **2**". A combination with Ctrl, Shift or Alt
//! is always a key; so are the names no button has (Esc, Space, Enter).
//! A single letter, digit or a word such as Delete is a key only where
//! the words around say so ("press", "hold", a Shortcut column), because
//! the manual also writes **X** for an axis and **Delete** for a button.

/// Held keys, which make anything after them a shortcut.
const MODIFIERS: [&str; 6] = ["Ctrl", "Shift", "Alt", "Cmd", "Win", "Option"];

/// Keys that are only ever keys.
const ALWAYS: [&str; 6] = ["Esc", "Escape", "Space", "Enter", "Backspace", "Tab"];

/// Key names that are also ordinary words or buttons.
const WORDS: [&str; 16] = [
    "Delete",
    "Del",
    "Insert",
    "Home",
    "End",
    "Page Up",
    "Page Down",
    "PgUp",
    "PgDn",
    "Up",
    "Down",
    "Left",
    "Right",
    "Return",
    "Numpad",
    "Menu",
];

/// Words just before a key that say it is one ("press **V**").
const CUES: [&str; 11] = [
    "press",
    "pressing",
    "presses",
    "pressed",
    "hold",
    "holding",
    "tap",
    "key",
    "keys",
    "shortcut",
    "shortcuts",
];

/// How sure the words around are that a short name is a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Context {
    /// Nothing says so: only certain keys count.
    Plain,
    /// "press", a Shortcut column or a key just before: any key name counts.
    Keys,
}

/// One part of a combination.
#[derive(Debug, PartialEq, Eq)]
enum Part<'a> {
    Key(&'a str),
    Words(&'a str),
}

/// The keyboard-key HTML for `text`, or `None` when it isn't a key.
pub(crate) fn html(text: &str, context: Context) -> Option<String> {
    let parts = parts(text, context)?;
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            let both_keys = matches!(parts[i - 1], Part::Key(_)) && matches!(part, Part::Key(_));
            out.push_str(if both_keys { "+" } else { " + " });
        }
        match part {
            Part::Key(k) => {
                out.push_str("<kbd>");
                out.push_str(&crate::escape(k));
                out.push_str("</kbd>");
            }
            Part::Words(w) => out.push_str(&crate::escape(w)),
        }
    }
    Some(out)
}

fn parts(text: &str, context: Context) -> Option<Vec<Part<'_>>> {
    let text = text.trim();
    if text.is_empty() || text.len() > 40 {
        return None;
    }
    let pieces: Vec<&str> = if text == "+" {
        vec![text]
    } else {
        text.split(" + ").map(str::trim).collect()
    };
    if pieces.iter().any(|p| p.is_empty()) {
        return None;
    }
    if pieces.len() == 1 {
        let one = pieces[0];
        let sure = MODIFIERS.contains(&one) || ALWAYS.contains(&one) || is_function_key(one);
        let named = sure || (context == Context::Keys && (is_single(one) || WORDS.contains(&one)));
        return named.then(|| vec![Part::Key(one)]);
    }
    if !MODIFIERS.contains(&pieces[0]) {
        return None;
    }
    let mut out = Vec::with_capacity(pieces.len());
    for piece in pieces {
        if is_key(piece) {
            out.push(Part::Key(piece));
        } else if is_mouse_words(piece) {
            out.push(Part::Words(piece));
        } else {
            return None;
        }
    }
    Some(out)
}

fn is_key(s: &str) -> bool {
    MODIFIERS.contains(&s)
        || ALWAYS.contains(&s)
        || WORDS.contains(&s)
        || is_function_key(s)
        || is_single(s)
        || matches!(s, "↑" | "↓" | "←" | "→")
}

/// One printable key: a capital letter, a digit or a sign.
fn is_single(s: &str) -> bool {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            c.is_ascii_uppercase() || c.is_ascii_digit() || (c.is_ascii_punctuation() && c != '*')
        }
        _ => false,
    }
}

fn is_function_key(s: &str) -> bool {
    s.strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n))
}

/// What a mouse does in a combination: "click", "mouse wheel", "left drag".
fn is_mouse_words(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c == ' ' || c == '-')
}

/// Whether the words just before (the current paragraph or cell so far)
/// call what follows a key: "…then press ", "hold ", "the key ".
pub(crate) fn cued(before: &str) -> bool {
    let lower = before.to_lowercase();
    let mut words = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .rev();
    // The text must end right after the cue (ignoring filler words and
    // an opening bracket), so "press E and pick" doesn't count.
    let tail = lower.trim_end();
    if !tail.ends_with(|c: char| c.is_alphanumeric() || c == '(') {
        return false;
    }
    for word in words.by_ref() {
        if matches!(word, "the" | "down" | "again" | "then") {
            continue;
        }
        return CUES.contains(&word);
    }
    false
}

/// Whether the words between two keys only join them: ", ", " or ",
/// " and ", " / ".
pub(crate) fn joins(between: &str) -> bool {
    let rest = between
        .trim()
        .trim_start_matches(',')
        .trim()
        .trim_start_matches(['(', ')'])
        .trim();
    matches!(rest, "" | "or" | "and" | "/" | ", or" | "then")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(s: &str) -> Option<String> {
        html(s, Context::Plain)
    }

    #[test]
    fn combinations_are_keys() {
        assert_eq!(
            plain("Ctrl + Z").as_deref(),
            Some("<kbd>Ctrl</kbd>+<kbd>Z</kbd>")
        );
        assert_eq!(
            plain("Ctrl + Shift + S").as_deref(),
            Some("<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd>")
        );
        assert_eq!(
            plain("Ctrl + mouse wheel").as_deref(),
            Some("<kbd>Ctrl</kbd> + mouse wheel")
        );
        assert_eq!(
            plain("Alt + ,").as_deref(),
            Some("<kbd>Alt</kbd>+<kbd>,</kbd>")
        );
    }

    #[test]
    fn names_only_keys_have_are_keys_anywhere() {
        for k in ["Esc", "Space", "Enter", "Shift", "F1", "F12"] {
            assert!(plain(k).is_some(), "{k}");
        }
    }

    #[test]
    fn short_names_need_the_words_around() {
        for k in ["X", "2", "Delete", "Up"] {
            assert!(plain(k).is_none(), "{k}");
            assert!(html(k, Context::Keys).is_some(), "{k}");
        }
    }

    #[test]
    fn ordinary_words_are_never_keys() {
        for s in [
            "Bevel",
            "100%",
            "Last edit",
            "Ctrl + Something Else",
            "Bake + shape",
            "x",
            "×",
            "F0",
            "F99",
            "",
            "Ctrl + ",
        ] {
            assert!(html(s, Context::Keys).is_none(), "{s:?}");
        }
    }

    #[test]
    fn the_cue_must_be_right_before() {
        assert!(cued("Select a shape and press "));
        assert!(cued("Fix: press "));
        assert!(cued("Hold down "));
        assert!(cued("press ("));
        assert!(!cued("Press E and choose "));
        assert!(!cued("its "));
        assert!(!cued(""));
        assert!(!cued("press, "));
    }

    #[test]
    fn joining_words() {
        assert!(joins(", "));
        assert!(joins(" or "));
        assert!(joins(", or "));
        assert!(!joins(" for the Points mode, "));
    }
}
