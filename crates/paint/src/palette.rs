//! Colour palettes ([decision 0183](../../../docs/decisions/0183-colour.md)):
//! named lists of colours kept with the project, and read from and
//! written to GIMP palette files (`.gpl`), a plain text format Krita,
//! Inkscape, Aseprite and many paint programs share.

/// The most colours a palette may hold.
pub const MAX_COLOURS: usize = 1024;

/// The most palettes a project may hold.
pub const MAX_PALETTES: usize = 64;

/// The most characters in a palette's name.
pub const MAX_NAME: usize = 200;

/// A named list of colours (sRGB).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    /// Its name.
    pub name: String,
    /// Its colours, in order.
    pub colours: Vec<[u8; 3]>,
}

impl Palette {
    /// An empty palette called `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            colours: Vec::new(),
        }
    }

    /// Whether its name and size are within the limits.
    pub fn is_valid(&self) -> bool {
        self.name.chars().count() <= MAX_NAME && self.colours.len() <= MAX_COLOURS
    }

    /// The palette as a GIMP palette file's text.
    pub fn to_gpl(&self) -> String {
        let mut out = String::from("GIMP Palette\n");
        let name: String = self.name.chars().filter(|c| !c.is_control()).collect();
        out.push_str(&format!("Name: {name}\n#\n"));
        for c in &self.colours {
            out.push_str(&format!(
                "{:3} {:3} {:3}\t#{:02X}{:02X}{:02X}\n",
                c[0], c[1], c[2], c[0], c[1], c[2]
            ));
        }
        out
    }

    /// The palette in a GIMP palette file's `text`, named `fallback` if
    /// the file names none. `None` if it isn't one.
    pub fn from_gpl(text: &str, fallback: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim_start_matches('\u{feff}').trim() != "GIMP Palette" {
            return None;
        }
        let mut palette = Self::new(fallback);
        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix("Name:") {
                palette.name = name.trim().chars().take(MAX_NAME).collect();
                continue;
            }
            if line.starts_with("Columns:") {
                continue;
            }
            let mut numbers = line.split_whitespace().map(|n| n.parse::<u8>().ok());
            let (Some(Some(r)), Some(Some(g)), Some(Some(b))) =
                (numbers.next(), numbers.next(), numbers.next())
            else {
                return None;
            };
            if palette.colours.len() == MAX_COLOURS {
                break;
            }
            palette.colours.push([r, g, b]);
        }
        Some(palette)
    }
}

/// The colour written as `#RRGGBB`.
pub fn hex(colour: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", colour[0], colour[1], colour[2])
}

/// The colour in `text` written as `#RRGGBB`, `RRGGBB` or `#RGB`.
pub fn from_hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().trim_start_matches('#');
    let value = |s: &str| u8::from_str_radix(s, 16).ok();
    match digits.len() {
        6 if digits.is_ascii() => Some([
            value(&digits[0..2])?,
            value(&digits[2..4])?,
            value(&digits[4..6])?,
        ]),
        3 if digits.is_ascii() => {
            let one = |i: usize| value(&digits[i..i + 1]).map(|v| v * 17);
            Some([one(0)?, one(1)?, one(2)?])
        }
        _ => None,
    }
}

/// Hue (0 to 1), saturation and value of an sRGB colour.
pub fn to_hsv(colour: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = colour.map(|v| v as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    [h, s, max]
}

/// The sRGB colour of hue, saturation and value (each 0 to 1).
pub fn from_hsv(hsv: [f32; 3]) -> [u8; 3] {
    let [h, s, v] = hsv.map(|x| if x.is_finite() { x } else { 0.0 });
    let h = h.rem_euclid(1.0) * 6.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r, g, b].map(|u| ((u + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// How light a colour looks, 0 to 1 (the weights paint programs use for
/// a value check).
pub fn lightness(colour: [u8; 3]) -> f32 {
    (0.3 * colour[0] as f32 + 0.59 * colour[1] as f32 + 0.11 * colour[2] as f32) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_palette_round_trips_through_a_gimp_file() {
        let palette = Palette {
            name: "Crate".into(),
            colours: vec![[227, 170, 102], [0, 0, 0], [255, 255, 255]],
        };
        let text = palette.to_gpl();
        assert!(text.starts_with("GIMP Palette\n"));
        assert_eq!(Palette::from_gpl(&text, "x"), Some(palette));
    }

    #[test]
    fn gimp_files_from_other_programs_are_read() {
        let text = "\u{feff}GIMP Palette\nName: Sky\nColumns: 4\n# a comment\n\n 10  20  30\tDeep\n255 255 255 White\n";
        let palette = Palette::from_gpl(text, "fallback").unwrap();
        assert_eq!(palette.name, "Sky");
        assert_eq!(palette.colours, [[10, 20, 30], [255, 255, 255]]);
        let unnamed = Palette::from_gpl("GIMP Palette\n1 2 3\n", "Mine").unwrap();
        assert_eq!(unnamed.name, "Mine");
    }

    #[test]
    fn other_files_and_bad_lines_are_refused() {
        assert_eq!(Palette::from_gpl("", "x"), None);
        assert_eq!(Palette::from_gpl("JASC-PAL\n0100\n", "x"), None);
        assert_eq!(Palette::from_gpl("GIMP Palette\n300 0 0\n", "x"), None);
        assert_eq!(Palette::from_gpl("GIMP Palette\n1 2\n", "x"), None);
        // Too many colours: the first ones are kept.
        let many = format!("GIMP Palette\n{}", "1 2 3\n".repeat(MAX_COLOURS + 10));
        assert_eq!(
            Palette::from_gpl(&many, "x").unwrap().colours.len(),
            MAX_COLOURS
        );
    }

    #[test]
    fn hex_and_hsv_convert_both_ways() {
        assert_eq!(hex([227, 170, 102]), "#E3AA66");
        assert_eq!(from_hex("#e3aa66"), Some([227, 170, 102]));
        assert_eq!(from_hex("E3AA66"), Some([227, 170, 102]));
        assert_eq!(from_hex("#fff"), Some([255, 255, 255]));
        assert_eq!(from_hex("#ggg"), None);
        assert_eq!(from_hex("#éé"), None);
        for colour in [
            [227, 170, 102],
            [0, 0, 0],
            [255, 0, 0],
            [12, 200, 90],
            [128, 128, 128],
        ] {
            assert_eq!(from_hsv(to_hsv(colour)), colour);
        }
        assert_eq!(from_hsv([f32::NAN, 2.0, -1.0]), [0, 0, 0]);
        assert!((lightness([255, 255, 255]) - 1.0).abs() < 1e-6);
    }
}
