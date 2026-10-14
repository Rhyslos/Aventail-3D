//! Drawn keyboards for the Keys window ([decision 0061](../../../docs/decisions/0061-drawn-keyboard.md)):
//! where each key sits, what is printed on it, and which key the app
//! reads from it, for each keyboard layout the window offers.
//!
//! The app reads letters as they are printed (a German keyboard's Z is
//! where an American one has Y), and the number row as numbers whatever
//! is printed on it (a French keyboard's & key is 1), as egui reports
//! them. So each drawn key names the [`KeyId`] it gives, and a shortcut
//! lights the keys that give its key.

use egui::Key;
use serde::{Deserialize, Serialize};

/// A key on a drawn keyboard, as the app reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum KeyId {
    /// A key the app reads (letters, numbers, F keys, Delete...).
    Key(Key),
    /// Shift, Ctrl (⌘ on a Mac) or Alt, held with other keys or while
    /// dragging.
    Shift,
    Ctrl,
    Alt,
}

/// A keyboard layout the window can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum KeyboardLayout {
    /// American (ANSI).
    #[default]
    Us,
    /// British (ISO).
    Uk,
    /// Norwegian (ISO): Æ, Ø and Å.
    Norwegian,
    /// Danish (ISO): Æ, Ø and Å, the other way round.
    Danish,
    /// Swedish and Finnish (ISO): Ö, Ä and Å.
    SwedishFinnish,
    /// German (ISO, QWERTZ).
    German,
    /// French (ISO, AZERTY).
    French,
}

impl KeyboardLayout {
    /// Every layout, in the order offered.
    pub const ALL: [Self; 7] = [
        Self::Us,
        Self::Uk,
        Self::Norwegian,
        Self::Danish,
        Self::SwedishFinnish,
        Self::German,
        Self::French,
    ];

    /// Its name, for the picker.
    pub fn name(self) -> &'static str {
        match self {
            Self::Us => "US (QWERTY)",
            Self::Uk => "UK (QWERTY)",
            Self::Norwegian => "Norwegian",
            Self::Danish => "Danish",
            Self::SwedishFinnish => "Swedish / Finnish",
            Self::German => "German (QWERTZ)",
            Self::French => "French (AZERTY)",
        }
    }

    /// The best guess for a system language such as `nb-NO`, `en_GB` or
    /// `de`: the language and, for English, the country.
    pub fn guess(language: &str) -> Self {
        let lower = language.to_lowercase().replace('_', "-");
        let mut parts = lower.split(['-', '.']);
        let lang = parts.next().unwrap_or_default();
        let country = parts.next().unwrap_or_default();
        match lang {
            "nb" | "nn" | "no" => Self::Norwegian,
            "da" => Self::Danish,
            "sv" | "fi" => Self::SwedishFinnish,
            "de" => Self::German,
            "fr" if country != "ca" => Self::French,
            "en" if matches!(country, "gb" | "uk" | "ie") => Self::Uk,
            _ => Self::Us,
        }
    }

    /// Whether it has the ISO shape: an extra key beside the left Shift
    /// and a tall Enter.
    fn iso(self) -> bool {
        self != Self::Us
    }
}

/// One drawn key: where it is and how big, in key widths (1 is a letter
/// key), what is printed on it, and the key the app reads from it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cap {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) label: String,
    pub(crate) id: Option<KeyId>,
}

/// A row being laid out from the left.
struct Row<'a> {
    caps: &'a mut Vec<Cap>,
    x: f32,
    y: f32,
}

impl Row<'_> {
    fn key(&mut self, label: &str, id: Option<KeyId>, w: f32) -> &mut Self {
        self.caps.push(Cap {
            x: self.x,
            y: self.y,
            w,
            h: 1.0,
            label: label.to_string(),
            id,
        });
        self.x += w;
        self
    }

    fn gap(&mut self, w: f32) -> &mut Self {
        self.x += w;
        self
    }
}

fn k(key: Key) -> Option<KeyId> {
    Some(KeyId::Key(key))
}

/// The key a printed letter gives (A to Z), if it is one.
fn letter(c: char) -> Option<KeyId> {
    Key::from_name(&c.to_ascii_uppercase().to_string())
        .filter(|_| c.is_ascii_alphabetic())
        .map(KeyId::Key)
}

const NUMBERS: [Key; 10] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
    Key::Num0,
];

/// What a layout prints on its rows: the number row (left of 1, the
/// numbers as printed, and the two after 0), and the three letter rows
/// (each key's label, letters reading as themselves).
struct Printed {
    before_one: &'static str,
    numbers: [&'static str; 10],
    after_zero: [&'static str; 2],
    top: [&'static str; 12],
    home: [&'static str; 12],
    bottom: [&'static str; 10],
    /// The extra key beside the left Shift on ISO keyboards.
    extra: &'static str,
}

const DIGITS: [&str; 10] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];

fn printed(layout: KeyboardLayout) -> Printed {
    use KeyboardLayout::*;
    let qwerty_top = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]"];
    let qwerty_home = ["A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "#"];
    let qwerty_bottom = ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"];
    match layout {
        Us => Printed {
            before_one: "`",
            numbers: DIGITS,
            after_zero: ["-", "="],
            top: qwerty_top,
            home: ["A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "\\"],
            bottom: qwerty_bottom,
            extra: "",
        },
        Uk => Printed {
            before_one: "`",
            numbers: DIGITS,
            after_zero: ["-", "="],
            top: qwerty_top,
            home: qwerty_home,
            bottom: qwerty_bottom,
            extra: "\\",
        },
        Norwegian => Printed {
            before_one: "|",
            numbers: DIGITS,
            after_zero: ["+", "\\"],
            top: ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "Å", "¨"],
            home: ["A", "S", "D", "F", "G", "H", "J", "K", "L", "Ø", "Æ", "'"],
            bottom: ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "-"],
            extra: "<",
        },
        Danish => Printed {
            before_one: "½",
            numbers: DIGITS,
            after_zero: ["+", "´"],
            top: ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "Å", "¨"],
            home: ["A", "S", "D", "F", "G", "H", "J", "K", "L", "Æ", "Ø", "'"],
            bottom: ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "-"],
            extra: "<",
        },
        SwedishFinnish => Printed {
            before_one: "§",
            numbers: DIGITS,
            after_zero: ["+", "´"],
            top: ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "Å", "¨"],
            home: ["A", "S", "D", "F", "G", "H", "J", "K", "L", "Ö", "Ä", "'"],
            bottom: ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "-"],
            extra: "<",
        },
        German => Printed {
            before_one: "^",
            numbers: DIGITS,
            after_zero: ["ß", "´"],
            top: ["Q", "W", "E", "R", "T", "Z", "U", "I", "O", "P", "Ü", "+"],
            home: ["A", "S", "D", "F", "G", "H", "J", "K", "L", "Ö", "Ä", "#"],
            bottom: ["Y", "X", "C", "V", "B", "N", "M", ",", ".", "-"],
            extra: "<",
        },
        French => Printed {
            before_one: "²",
            numbers: [
                "& 1", "é 2", "\" 3", "' 4", "( 5", "- 6", "è 7", "_ 8", "ç 9", "à 0",
            ],
            after_zero: [")", "="],
            top: ["A", "Z", "E", "R", "T", "Y", "U", "I", "O", "P", "^", "$"],
            home: ["Q", "S", "D", "F", "G", "H", "J", "K", "L", "M", "ù", "*"],
            bottom: ["W", "X", "C", "V", "B", "N", ",", ";", ":", "!"],
            extra: "<",
        },
    }
}

/// The keys a printed label gives the app, beyond letters: the comma
/// (Ctrl + , opens Settings).
fn punctuation(label: &str) -> Option<KeyId> {
    match label {
        "," => k(Key::Comma),
        _ => None,
    }
}

/// The drawn keyboard for `layout`: the F row, the main block, and the
/// Delete key's block with the arrows beside it. Positions are in key
/// widths from the top left.
pub(crate) fn caps(layout: KeyboardLayout) -> Vec<Cap> {
    let p = printed(layout);
    let mut caps = Vec::new();
    // The F row.
    {
        let mut row = Row {
            caps: &mut caps,
            x: 0.0,
            y: 0.0,
        };
        row.key("Esc", k(Key::Escape), 1.0).gap(1.0);
        let f = [
            Key::F1,
            Key::F2,
            Key::F3,
            Key::F4,
            Key::F5,
            Key::F6,
            Key::F7,
            Key::F8,
            Key::F9,
            Key::F10,
            Key::F11,
            Key::F12,
        ];
        for (i, key) in f.iter().enumerate() {
            if i > 0 && i % 4 == 0 {
                row.gap(0.5);
            }
            row.key(&format!("F{}", i + 1), k(*key), 1.0);
        }
    }
    let top = 1.25;
    // The number row.
    {
        let mut row = Row {
            caps: &mut caps,
            x: 0.0,
            y: top,
        };
        row.key(p.before_one, None, 1.0);
        for (label, key) in p.numbers.iter().zip(NUMBERS) {
            row.key(label, k(key), 1.0);
        }
        row.key(p.after_zero[0], None, 1.0)
            .key(p.after_zero[1], None, 1.0)
            .key("Backspace", k(Key::Backspace), 2.0);
    }
    // The top letter row.
    {
        let mut row = Row {
            caps: &mut caps,
            x: 0.0,
            y: top + 1.0,
        };
        row.key("Tab", k(Key::Tab), 1.5);
        for label in p.top {
            let id = label
                .chars()
                .next()
                .and_then(letter)
                .filter(|_| label.len() == 1);
            row.key(label, id, 1.0);
        }
        if layout.iso() {
            // The tall Enter, over two rows.
            row.caps.push(Cap {
                x: row.x + 0.25,
                y: top + 1.0,
                w: 1.25,
                h: 2.0,
                label: "Enter".into(),
                id: k(Key::Enter),
            });
        } else {
            row.key("\\", None, 1.5);
        }
    }
    // The home row.
    {
        let mut row = Row {
            caps: &mut caps,
            x: 0.0,
            y: top + 2.0,
        };
        row.key("Caps", None, 1.75);
        let home = if layout.iso() {
            &p.home[..]
        } else {
            &p.home[..11]
        };
        for label in home {
            let id = label
                .chars()
                .next()
                .and_then(letter)
                .filter(|_| label.len() == 1);
            row.key(label, id, 1.0);
        }
        if !layout.iso() {
            row.key("Enter", k(Key::Enter), 2.25);
        }
    }
    // The bottom row.
    {
        let mut row = Row {
            caps: &mut caps,
            x: 0.0,
            y: top + 3.0,
        };
        if layout.iso() {
            row.key("Shift", Some(KeyId::Shift), 1.25)
                .key(p.extra, None, 1.0);
        } else {
            row.key("Shift", Some(KeyId::Shift), 2.25);
        }
        for label in p.bottom {
            let id = label
                .chars()
                .next()
                .and_then(letter)
                .filter(|_| label.len() == 1)
                .or_else(|| punctuation(label));
            row.key(label, id, 1.0);
        }
        row.key("Shift", Some(KeyId::Shift), 2.75);
    }
    // The space row.
    {
        let mut row = Row {
            caps: &mut caps,
            x: 0.0,
            y: top + 4.0,
        };
        row.key("Ctrl", Some(KeyId::Ctrl), 1.5)
            .key("", None, 1.25)
            .key("Alt", Some(KeyId::Alt), 1.25)
            .key("Space", k(Key::Space), 7.0)
            .key("Alt Gr", None, 1.25)
            .key("", None, 1.25)
            .key("Ctrl", Some(KeyId::Ctrl), 1.5);
    }
    // Delete's block and the arrows.
    {
        let x = 15.5;
        let mut row = Row {
            caps: &mut caps,
            x,
            y: top,
        };
        row.key("Ins", None, 1.0)
            .key("Home", None, 1.0)
            .key("PgUp", None, 1.0);
        let mut row = Row {
            caps: &mut caps,
            x,
            y: top + 1.0,
        };
        row.key("Del", k(Key::Delete), 1.0)
            .key("End", None, 1.0)
            .key("PgDn", None, 1.0);
        let mut row = Row {
            caps: &mut caps,
            x: x + 1.0,
            y: top + 3.0,
        };
        row.key("↑", k(Key::ArrowUp), 1.0);
        let mut row = Row {
            caps: &mut caps,
            x,
            y: top + 4.0,
        };
        row.key("←", k(Key::ArrowLeft), 1.0)
            .key("↓", k(Key::ArrowDown), 1.0)
            .key("→", k(Key::ArrowRight), 1.0);
    }
    caps
}

/// How wide and tall a drawn keyboard is, in key widths.
pub(crate) const SIZE: (f32, f32) = (18.5, 6.25);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_guess_their_keyboard() {
        assert_eq!(KeyboardLayout::guess("nb-NO"), KeyboardLayout::Norwegian);
        assert_eq!(
            KeyboardLayout::guess("nn_NO.UTF-8"),
            KeyboardLayout::Norwegian
        );
        assert_eq!(KeyboardLayout::guess("da-DK"), KeyboardLayout::Danish);
        assert_eq!(
            KeyboardLayout::guess("sv-SE"),
            KeyboardLayout::SwedishFinnish
        );
        assert_eq!(KeyboardLayout::guess("fi"), KeyboardLayout::SwedishFinnish);
        assert_eq!(KeyboardLayout::guess("de-AT"), KeyboardLayout::German);
        assert_eq!(KeyboardLayout::guess("fr-FR"), KeyboardLayout::French);
        assert_eq!(KeyboardLayout::guess("fr-CA"), KeyboardLayout::Us);
        assert_eq!(KeyboardLayout::guess("en-GB"), KeyboardLayout::Uk);
        assert_eq!(KeyboardLayout::guess("en-US"), KeyboardLayout::Us);
        assert_eq!(KeyboardLayout::guess(""), KeyboardLayout::Us);
    }

    #[test]
    fn every_layout_has_every_letter_and_number_once_and_fits() {
        for layout in KeyboardLayout::ALL {
            let caps = caps(layout);
            for c in 'A'..='Z' {
                let key = Key::from_name(&c.to_string()).unwrap();
                let n = caps.iter().filter(|cap| cap.id == k(key)).count();
                assert_eq!(n, 1, "{layout:?} {c}");
            }
            for key in NUMBERS {
                assert_eq!(caps.iter().filter(|cap| cap.id == k(key)).count(), 1);
            }
            for cap in &caps {
                assert!(
                    cap.x >= 0.0 && cap.x + cap.w <= SIZE.0 + 1e-3,
                    "{layout:?} {cap:?}"
                );
                assert!(
                    cap.y >= 0.0 && cap.y + cap.h <= SIZE.1 + 1e-3,
                    "{layout:?} {cap:?}"
                );
            }
            // No two keys on top of each other.
            for (i, a) in caps.iter().enumerate() {
                for b in &caps[i + 1..] {
                    let apart = a.x + a.w <= b.x + 1e-3
                        || b.x + b.w <= a.x + 1e-3
                        || a.y + a.h <= b.y + 1e-3
                        || b.y + b.h <= a.y + 1e-3;
                    assert!(apart, "{layout:?}: {} and {}", a.label, b.label);
                }
            }
        }
    }

    #[test]
    fn letters_follow_what_is_printed() {
        let german = caps(KeyboardLayout::German);
        let z = german.iter().find(|c| c.id == k(Key::Z)).unwrap();
        let y = german.iter().find(|c| c.id == k(Key::Y)).unwrap();
        assert!(z.y < y.y, "Z on the top row, Y at the bottom");
        let french = caps(KeyboardLayout::French);
        let a = french.iter().find(|c| c.id == k(Key::A)).unwrap();
        let q = french.iter().find(|c| c.id == k(Key::Q)).unwrap();
        assert!(a.y < q.y, "A on the top row, Q on the home row");
        assert_eq!(
            french.iter().find(|c| c.id == k(Key::Num1)).unwrap().label,
            "& 1"
        );
        let norwegian = caps(KeyboardLayout::Norwegian);
        for label in ["Æ", "Ø", "Å"] {
            assert!(norwegian.iter().any(|c| c.label == label), "{label}");
        }
    }
}
